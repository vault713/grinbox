pub mod types;

use std::collections::HashMap;
use std::str::FromStr;

use colored::*;
use uuid::Uuid;
use futures::{
    sync::mpsc::{
        UnboundedReceiver,
        UnboundedSender,
        unbounded
    },
    future::lazy,
    Future,
    Stream,
};

use ws::{
    Handler,
    Sender,
    Message,
    Handshake,
    CloseCode,
    Request,
    Response,
};

use super::broker::protocol::{
    BrokerRequest,
    BrokerResponse,
};

use common::crypto::{Signature, PublicKey, Base58, Hex, verify_signature};

use common::protocol::{
    ProtocolRequest,
    ProtocolResponse,
    ProtocolError
};

use common::Error;
use self::types::GrinboxAddress;

static MAX_SUBSCRIPTIONS: usize = 1;


pub struct BrokerResponseHandler {
    inner: std::sync::Arc<std::sync::Mutex<Server>>,
    response_receiver: UnboundedReceiver<BrokerResponse>,
}

pub struct AsyncServer {
    id: String,
    inner: std::sync::Arc<std::sync::Mutex<Server>>,
    nats_sender: UnboundedSender<BrokerRequest>,
    response_handlers_sender: UnboundedSender<BrokerResponseHandler>,
    subscriptions: HashMap<String, Subscription>,
}

pub struct Server {
    id: String,
    out: Sender,
}

struct Subscription {

}

#[derive(Serialize, Deserialize, Debug)]
struct SignedPayload {
    str: String,
    challenge: String,
    signature: String,
}

impl Drop for AsyncServer {
    fn drop(&mut self) {
        for (subject, _subscription) in &self.subscriptions {
            if self.nats_sender.unbounded_send(BrokerRequest::Unsubscribe {
                subject: subject.clone(),
            }).is_err() {
                error!("failed to unsubscribe while dropping server!");
            };
        }
    }
}

impl AsyncServer {
    pub fn new(out: Sender, nats_sender: UnboundedSender<BrokerRequest>, response_handlers_sender: UnboundedSender<BrokerResponseHandler>) -> AsyncServer
    {
        let id = Uuid::new_v4().to_string();

        let server = Server {
            id: id.clone(),
            out,
        };

        AsyncServer {
            id: id.clone(),
            inner: std::sync::Arc::new(std::sync::Mutex::new(server)),
            nats_sender,
            response_handlers_sender,
            subscriptions: HashMap::new(),
        }
    }

    pub fn init() -> UnboundedSender<BrokerResponseHandler> {
        let (fut_tx, fut_rx) = unbounded::<BrokerResponseHandler>();

        std::thread::spawn(move || {
            info!("broker handler started");
            let fut_loop = fut_rx.for_each(move |handler| {
                let clone = handler.inner.clone();
                let response_loop = handler.response_receiver
                    .for_each(move |m| {
                        match m {
                            BrokerResponse::Message { subject: _, payload, reply_to } => {
                                let signed_payload = serde_json::from_str::<SignedPayload>(&payload);
                                if signed_payload.is_ok() {
                                    let signed_payload = signed_payload.unwrap();
                                    let response = ProtocolResponse::Slate {
                                        from: reply_to,
                                        str: signed_payload.str,
                                        challenge: signed_payload.challenge,
                                        signature: signed_payload.signature,
                                    };
                                    let guard = clone.lock().unwrap();
                                    let ref server = *guard;
                                    info!("[{}] <- {}", server.id.bright_green(), response);
                                    if server.out.send(serde_json::to_string(&response).unwrap()).is_err() {
                                        error!("failed sending slate to client!");
                                    };
                                } else {
                                    error!("invalid payload!");
                                }
                            }
                        }
                        Ok(())
                    });

                std::thread::spawn(move || {
                    tokio::run(lazy(|| {
                        tokio::spawn(response_loop);
                        Ok(())
                    }));
                });
                Ok(())
            }).map_err(|_| {});

            tokio::run(lazy(move || {
                tokio::spawn(fut_loop)
            }));
            debug!("future thread ended...");
        });
        fut_tx
    }

    fn error(kind: ProtocolError) -> ProtocolResponse {
        let description = format!("{}", kind);
        ProtocolResponse::Error { kind, description }
    }

    fn ok() -> ProtocolResponse {
        ProtocolResponse::Ok
    }

    fn get_challenge_raw(&self) -> &str {
        "7WUDtkSaKyGRUnQ22rE3QUXChV8DmA6NnunDYP4vheTpc"
    }

    fn get_challenge(&self) -> ProtocolResponse {
        ProtocolResponse::Challenge { str: String::from(self.get_challenge_raw()) }
    }

    fn verify_signature(&self, public_key: &str, challenge: &str, signature: &str) -> Result<(), Error> {
        let public_key = PublicKey::from_base58_check(public_key, 2)?;
        let signature = Signature::from_hex(signature)?;
        verify_signature(challenge, &signature, &public_key).map_err(|_| {
            Error::ProtocolError { kind: ProtocolError::InvalidSignature }
        })?;
        Ok(())
    }

    fn subscribe(&mut self, address: String, signature: String) -> ProtocolResponse {
        let result = self.verify_signature(&address, self.get_challenge_raw(), &signature);
        match result {
            Ok(()) => {
                if self.subscriptions.len() == MAX_SUBSCRIPTIONS {
                    AsyncServer::error(ProtocolError::TooManySubscriptions)
                } else {
                    let (res_tx, res_rx) = unbounded::<BrokerResponse>();
                    if self.nats_sender.unbounded_send(BrokerRequest::Subscribe {
                        subject: address.clone(),
                        response_sender: res_tx,
                    }).is_err() {
                        error!("could not issue subscribe request!");
                        return AsyncServer::error(ProtocolError::UnknownError)
                    };

                    if self.response_handlers_sender.unbounded_send(BrokerResponseHandler {
                        inner: self.inner.clone(),
                        response_receiver: res_rx,
                    }).is_err() {
                        error!("could not register subscription handler!");
                        return AsyncServer::error(ProtocolError::UnknownError)
                    };

                    self.subscriptions.insert(
                        address.clone(),
                        Subscription {}
                    );

                    AsyncServer::ok()
                }
            },
            Err(e) => {
                match e {
                    Error::ProtocolError { kind } => AsyncServer::error(kind),
                    _ => AsyncServer::error(ProtocolError::UnknownError),
                }
            }
        }
    }

    fn unsubscribe(&mut self, address: String) -> ProtocolResponse {
        let result = self.subscriptions.remove(&address);
        match result {
            Some(_subscription) => {
                if self.nats_sender.unbounded_send(BrokerRequest::Unsubscribe {
                    subject: address.clone(),
                }).is_err() {
                    error!("could not unsubscribe!");
                    return AsyncServer::error(ProtocolError::UnknownError)
                };

                AsyncServer::ok()
            },
            None => AsyncServer::error(ProtocolError::InvalidRequest)
        }
    }

    fn post_slate(&self, from: String, to: String, str: String, signature: String) -> ProtocolResponse {
        let from_address = GrinboxAddress::from_str(&from);
        if from_address.is_err() {
            return AsyncServer::error(ProtocolError::InvalidRequest);
        }
        let from_address = from_address.unwrap();

        let to_address = GrinboxAddress::from_str(&to);
        if to_address.is_err() {
            return AsyncServer::error(ProtocolError::InvalidRequest);
        }
        let to_address = to_address.unwrap();

        if let Err(_) = PublicKey::from_base58_check(&to_address.public_key, 2) {
            AsyncServer::error(ProtocolError::InvalidRequest)
        } else {
            let mut challenge = String::new();
            challenge.push_str(&str);

            let mut result = self.verify_signature(&from_address.public_key, &challenge, &signature);
            let mut challenge_raw = "";
            if result.is_err() {
                challenge.push_str(self.get_challenge_raw());
                challenge_raw = self.get_challenge_raw();
                result = self.verify_signature(&from_address.public_key, &challenge, &signature);
            }

            match result {
                Ok(()) => {
                    let signed_payload = SignedPayload { str, challenge: challenge_raw.to_string(), signature };
                    let signed_payload = serde_json::to_string(&signed_payload).unwrap();
                    if self.nats_sender.unbounded_send(BrokerRequest::PostMessage {
                        subject: to_address.public_key,
                        payload: signed_payload,
                        reply_to: from_address.to_string(),
                    }).is_err() {
                        error!("could not post message to broker!");
                        return AsyncServer::error(ProtocolError::UnknownError)
                    };
                    AsyncServer::ok()
                },
                Err(e) => {
                    match e {
                        Error::ProtocolError { kind } => AsyncServer::error(kind),
                        _ => AsyncServer::error(ProtocolError::UnknownError),
                    }
                }
            }
        }
    }
}

impl Handler for AsyncServer {
    fn on_request(&mut self, req: &Request) -> Result<Response, ws::Error> {
        let res = Response::from_request(req);
        if let Err(_) = res {
            let response = Response::new(200, "", vec![]);
            Ok(response)
        } else {
            Ok(res.unwrap())
        }
    }

    fn on_open(&mut self, _: Handshake) -> Result<(), ws::Error> {
        info!("[{}] {}", self.id.bright_green(), "connection established".bright_purple());

        let response = self.get_challenge();
        debug!("[{}] <- {}", self.id.bright_green(), response);
        let server = self.inner.lock().unwrap();
        if server.out.send(serde_json::to_string(&response).unwrap()).is_err() {
            error!("could not send challenge to client!");
        };
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<(), ws::Error> {
        let request = serde_json::from_str(&msg.to_string());

        let response =
            if request.is_ok() {
                let request = request.unwrap();
                info!("[{}] -> {}", self.id.bright_green(), request);
                match request {
                    ProtocolRequest::Challenge => self.get_challenge(),
                    ProtocolRequest::Subscribe { address, signature } => self.subscribe(address, signature),
                    ProtocolRequest::PostSlate { from, to, str, signature } => self.post_slate(from, to, str, signature),
                    ProtocolRequest::Unsubscribe { address } => self.unsubscribe(address),
                }
            } else {
                debug!("[{}] -> {}", self.id.bright_green(), "invalid request!".bright_red());
                AsyncServer::error(ProtocolError::InvalidRequest)
            };

        info!("[{}] <- {}", self.id.bright_green(), response);
        let server = self.inner.lock().unwrap();
        server.out.send(serde_json::to_string(&response).unwrap())
    }

    fn on_close(&mut self, code: CloseCode, _reason: &str) {
        let code = format!("{:?}", code);
        info!("[{}] {} [{}]", self.id.bright_green(), "connection dropped".bright_purple(), code.bright_green());
    }

    fn on_error(&mut self, err: ws::Error) {
        error!("the server encountered an error: {:?}", err);
    }
}