use std::thread;
use ws::{
    connect, CloseCode, Error as WsError, ErrorKind as WsErrorKind, Handler, Handshake, Message,
    Result as WsResult, Sender, util::Token,
};

use crate::client::{CloseReason, GrinboxPublisher, GrinboxSubscriber, GrinboxSubscriptionHandler};
use crate::error::{ErrorKind, Result};
use crate::types::{Arc, Mutex, GrinboxAddress, GrinboxMessage, GrinboxRequest, GrinboxResponse, Slate, TxProof, TxProofErrorKind};
use crate::utils::crypto::{Hex, sign_challenge};
use crate::utils::secp::SecretKey;

const KEEPALIVE_TOKEN: Token = Token(1);
const KEEPALIVE_INTERVAL_MS: u64 = 30_000;

#[derive(Clone)]
pub struct GrinboxClient {
    address: GrinboxAddress,
    broker: GrinboxBroker,
    protocol_unsecure: bool,
    secret_key: SecretKey,
}

impl GrinboxClient {
    pub fn new(
        address: &GrinboxAddress,
        secert_key: &SecretKey,
        protocol_unsecure: bool,
    ) -> Result<Self> {
        Ok(Self {
            address: address.clone(),
            broker: GrinboxBroker::new(protocol_unsecure)?,
            protocol_unsecure,
            secret_key: secert_key.clone(),
        })
    }

    fn generate_signature(challenge: &str, secret_key: &SecretKey) -> String {
        let signature = sign_challenge(challenge, secret_key).expect("could not sign challenge!");
        signature.to_hex()
    }
}

impl GrinboxPublisher for GrinboxClient {
    fn post_slate(&self, slate: &Slate, to: &GrinboxAddress) -> Result<()> {
        let broker = GrinboxBroker::new(self.protocol_unsecure)?;
        broker.post_slate(slate, &to, &self.address, &self.secret_key)?;
        Ok(())
    }
}

impl GrinboxSubscriber for GrinboxClient {
    fn subscribe(&mut self, handler: Box<GrinboxSubscriptionHandler + Send>) -> Result<()> {
        self.broker
            .start(&self.address, &self.secret_key, handler)?;
        Ok(())
    }

    fn unsubscribe(&self) {
        self.broker.stop();
    }

    fn is_running(&self) -> bool {
        self.broker.is_running()
    }
}

#[derive(Clone)]
struct GrinboxBroker {
    inner: Arc<Mutex<Option<Sender>>>,
    protocol_unsecure: bool,
}

struct ConnectionMetadata {
    retries: u32,
    connected_at_least_once: bool,
}

impl ConnectionMetadata {
    pub fn new() -> Self {
        Self {
            retries: 0,
            connected_at_least_once: false,
        }
    }
}

impl GrinboxBroker {
    fn new(protocol_unsecure: bool) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(None)),
            protocol_unsecure,
        })
    }

    fn post_slate(
        &self,
        slate: &Slate,
        to: &GrinboxAddress,
        from: &GrinboxAddress,
        secret_key: &SecretKey,
    ) -> Result<()> {
        let url = {
            match self.protocol_unsecure {
                true => format!(
                    "ws://{}:{}",
                    from.domain,
                    from.port
                ),
                false => format!(
                    "wss://{}:{}",
                    from.domain,
                    from.port
                ),
            }
        };
        let pkey = to.public_key()?;
        let skey = secret_key.clone();
        connect(url, move |sender| {
            move |msg: Message| {
                let response = serde_json::from_str::<GrinboxResponse>(&msg.to_string())
                    .expect("could not parse response!");
                match response {
                    GrinboxResponse::Challenge { str: _ } => {
                        let message = GrinboxMessage::new(
                            serde_json::to_string(&slate).unwrap(),
                            &to,
                            &pkey,
                            &skey,
                        )
                            .map_err(|_| {
                                WsError::new(WsErrorKind::Protocol, "could not encrypt slate!")
                            })?;
                        let slate_str = serde_json::to_string(&message).unwrap();

                        let mut challenge = String::new();
                        challenge.push_str(&slate_str);

                        let signature = GrinboxClient::generate_signature(&challenge, secret_key);
                        let request = GrinboxRequest::PostSlate {
                            from: from.stripped(),
                            to: to.stripped(),
                            str: slate_str,
                            signature,
                        };
                        sender
                            .send(serde_json::to_string(&request).unwrap())
                            .unwrap();
                    }
                    GrinboxResponse::Error {
                        kind: _,
                        description: _,
                    } => {
                        debug!("{}", response);
                        sender.close(CloseCode::Normal).is_ok();
                    }
                    GrinboxResponse::Ok => {
                        sender.close(CloseCode::Normal).is_ok();
                    }
                    _ => {}
                }
                Ok(())
            }
        })?;
        Ok(())
    }

    fn start(
        &mut self,
        address: &GrinboxAddress,
        secret_key: &SecretKey,
        handler: Box<GrinboxSubscriptionHandler + Send>,
    ) -> Result<()> {
        let handler = Arc::new(Mutex::new(handler));
        let url = {
            let cloned_address = address.clone();
            match self.protocol_unsecure {
                true => format!(
                    "ws://{}:{}",
                    cloned_address.domain,
                    cloned_address.port
                ),
                false => format!(
                    "wss://{}:{}",
                    cloned_address.domain,
                    cloned_address.port
                ),
            }
        };
        let secret_key = secret_key.clone();
        let cloned_address = address.clone();
        let cloned_inner = self.inner.clone();
        let cloned_handler = handler.clone();
        thread::spawn(move || {
            let connection_meta_data = Arc::new(Mutex::new(ConnectionMetadata::new()));
            loop {
                let cloned_address = cloned_address.clone();
                let cloned_handler = cloned_handler.clone();
                let cloned_cloned_inner = cloned_inner.clone();
                let cloned_connection_meta_data = connection_meta_data.clone();
                let result = connect(url.clone(), move |sender| {
                    {
                        let mut guard = cloned_cloned_inner.lock();
                        *guard = Some(sender.clone());
                    }

                    let client = GrinboxWebsocketClient {
                        sender,
                        handler: cloned_handler.clone(),
                        challenge: None,
                        address: cloned_address.clone(),
                        secret_key,
                        connection_meta_data: cloned_connection_meta_data.clone(),
                    };
                    client
                });

                let is_stopped = cloned_inner.lock().is_none();

                if is_stopped {
                    match result {
                        Err(_) => handler.lock().on_close(CloseReason::Abnormal(
                            ErrorKind::GrinboxWebsocketAbnormalTermination.into(),
                        )),
                        _ => handler.lock().on_close(CloseReason::Normal),
                    }
                    break;
                } else {
                    let mut guard = connection_meta_data.lock();
                    if guard.retries == 0 && guard.connected_at_least_once {
                        handler.lock().on_dropped();
                    }
                    let secs = std::cmp::min(32, 2u64.pow(guard.retries));
                    let duration = std::time::Duration::from_secs(secs);
                    std::thread::sleep(duration);
                    guard.retries += 1;
                }
            }
            let mut guard = cloned_inner.lock();
            *guard = None;
        });
        Ok(())
    }

    fn stop(&self) {
        let mut guard = self.inner.lock();
        if let Some(ref sender) = *guard {
            sender.close(CloseCode::Normal).is_ok();
        }
        *guard = None;
    }

    fn is_running(&self) -> bool {
        let guard = self.inner.lock();
        guard.is_some()
    }
}

struct GrinboxWebsocketClient {
    sender: Sender,
    handler: Arc<Mutex<Box<GrinboxSubscriptionHandler + Send>>>,
    challenge: Option<String>,
    address: GrinboxAddress,
    secret_key: SecretKey,
    connection_meta_data: Arc<Mutex<ConnectionMetadata>>,
}

impl GrinboxWebsocketClient {
    fn subscribe(&self, challenge: &str) -> Result<()> {
        let signature = GrinboxClient::generate_signature(challenge, &self.secret_key);
        let request = GrinboxRequest::Subscribe {
            address: self.address.public_key.to_string(),
            signature,
        };
        self.send(&request)
            .expect("could not send subscribe request!");
        Ok(())
    }

    fn send(&self, request: &GrinboxRequest) -> Result<()> {
        let request = serde_json::to_string(&request).unwrap();
        self.sender.send(request)?;
        Ok(())
    }
}

impl Handler for GrinboxWebsocketClient {
    fn on_open(&mut self, _shake: Handshake) -> WsResult<()> {
        let mut guard = self.connection_meta_data.lock();

        if guard.connected_at_least_once {
            self.handler.lock().on_reestablished();
        } else {
            self.handler.lock().on_open();
            guard.connected_at_least_once = true;
        }

        guard.retries = 0;

        self.sender.timeout(KEEPALIVE_INTERVAL_MS, KEEPALIVE_TOKEN)?;
        Ok(())
    }

    fn on_timeout(&mut self, event: Token) -> WsResult<()> {
        match event {
            KEEPALIVE_TOKEN => {
                self.sender.ping(vec![])?;
                self.sender.timeout(KEEPALIVE_INTERVAL_MS, KEEPALIVE_TOKEN)
            }
            _ => Err(WsError::new(
                WsErrorKind::Internal,
                "Invalid timeout token encountered!",
            )),
        }
    }

    fn on_message(&mut self, msg: Message) -> WsResult<()> {
        let response = match serde_json::from_str::<GrinboxResponse>(&msg.to_string()) {
            Ok(x) => x,
            Err(_) => {
                error!("could not parse response");
                return Ok(());
            }
        };

        match response {
            GrinboxResponse::Challenge { str } => {
                self.challenge = Some(str.clone());
                self.subscribe(&str).map_err(|_| {
                    WsError::new(WsErrorKind::Protocol, "error attempting to subscribe!")
                })?;
            }
            GrinboxResponse::Slate {
                from,
                str,
                challenge,
                signature,
            } => {
                let (mut slate, mut tx_proof) = match TxProof::from_response(
                    from,
                    str,
                    challenge,
                    signature,
                    &self.secret_key,
                    Some(&self.address),
                ) {
                    Ok(x) => x,
                    Err(TxProofErrorKind::ParseAddress) => {
                        error!("could not parse address!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::ParsePublicKey) => {
                        error!("could not parse public key!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::ParseSignature) => {
                        error!("could not parse signature!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::VerifySignature) => {
                        error!("invalid slate signature!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::ParseGrinboxMessage) => {
                        error!("could not parse encrypted slate!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::VerifyDestination) => {
                        error!("could not verify destination!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::DecryptionKey) => {
                        error!("could not determine decryption key!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::DecryptMessage) => {
                        error!("could not decrypt slate!");
                        return Ok(());
                    }
                    Err(TxProofErrorKind::ParseSlate) => {
                        error!("could not parse decrypted slate!");
                        return Ok(());
                    }
                };

                let address = tx_proof.address.clone();
                self.handler
                    .lock()
                    .on_slate(&address, &mut slate, Some(&mut tx_proof));
            }
            GrinboxResponse::Error {
                kind: _,
                description: _,
            } => {
                debug!("{}", response);
            }
            _ => {}
        }
        Ok(())
    }

    fn on_error(&mut self, err: WsError) {
        // Ignore connection reset errors by default
        if let WsErrorKind::Io(ref err) = err.kind {
            if let Some(104) = err.raw_os_error() {
                return;
            }
        }

        error!("{:?}", err);
    }
}
