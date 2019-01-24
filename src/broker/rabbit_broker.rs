use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use futures::{
    sync::mpsc::{
        UnboundedSender,
        unbounded
    },
    Async,
    Poll,
    IntoFuture,
    future,
    Future,
    Stream,
};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use lapin_futures::client::ConnectionOptions;
use lapin_futures::channel::{ConfirmSelectOptions, BasicPublishOptions, BasicConsumeOptions, BasicProperties,QueueDeclareOptions};
use lapin_futures::types::FieldTable;

use super::protocol::{
    BrokerRequest,
    BrokerResponse,
};

use colored::*;
use common::Error;

type Client = lapin_futures::client::Client<TcpStream>;
type Channel = lapin_futures::channel::Channel<TcpStream>;

const DEFAULT_QUEUE_EXPIRATION: u32 = 86400000;
const DEFAULT_MESSAGE_EXPIRATION: &str = "86400000";

pub struct Broker {
    address: SocketAddr,
    consumers: Arc<Mutex<HashMap<String, Consumer>>>,
    username: String,
    password: String,
}

impl Broker {
    pub fn new(address: SocketAddr, username: String, password: String) -> Broker {
        Broker {
            address,
            consumers: Arc::new(Mutex::new(HashMap::new())),
            username,
            password,
        }
    }

    pub fn start(&self) -> UnboundedSender<BrokerRequest> {
        let (tx, rx) = unbounded();
        let address = self.address.clone();
        let username = self.username.clone();
        let password = self.password.clone();
        let consumers = self.consumers.clone();
        std::thread::spawn(move|| {
            let program = Broker::connect(address, username, password)
                .and_then(move |client| {
                    info!("connected to rabbitmq broker");
                    let request_loop = rx
                        .for_each(move |request| {
                            match request {
                                BrokerRequest::Subscribe { subject, response_sender } => {
                                    let mut consumer = Consumer::new(subject.clone(), response_sender.clone());
                                    tokio::spawn(consumer.start(&client).map_err(|_|{}));
                                    consumers.lock().unwrap().insert(subject.clone(), consumer);
                                },
                                BrokerRequest::Unsubscribe { subject } => {
                                    let _consumer = consumers.lock().unwrap().remove(&subject);
                                },
                                BrokerRequest::PostMessage { subject, payload, reply_to } => {
                                    let future = handle_post_message(&client, subject, payload, reply_to);
                                    tokio::spawn(future.map_err(|_|{}));
                                },
                            }
                            Ok(())
                        })
                        .map_err(|e| error!("error = {:?}", e));

                    tokio::spawn(request_loop);
                    Ok(())
                }).map_err(|e| {
                error!("{}", e);
            }
            );

            tokio::run(program);
        });
        tx
    }

    pub fn connect(address: SocketAddr, username: String, password: String) -> impl Future<Item = Client, Error = Error> {
        TcpStream::connect(&address)
            .and_then(move|stream| {
                Client::connect(stream, ConnectionOptions {
                    frame_max: 65535,
                    username,
                    password,
                    ..Default::default()
                })
            })
            .map_err(|_| Error::generic("failed connecting!"))
            .and_then(|(client, heartbeat)| {
                tokio::spawn(heartbeat.map_err(|e| error!("heartbeat error: {:?}", e)))
                    .into_future().map(|_| client)
                    .map_err(|_| Error::generic("failed connecting!"))
            })
    }
}

struct Consumer {
    subject: String,
    sender: UnboundedSender<BrokerResponse>,
    completer: UnboundedSender<()>,
    channel: Arc<Mutex<Option<Channel>>>,
}

impl Consumer {
    pub fn new(subject: String, sender: UnboundedSender<BrokerResponse>) -> Consumer {
        let (completer, _) = unbounded::<()>();
        Consumer {
            subject,
            sender,
            completer,
            channel: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&mut self, client: &Client) -> impl Future<Item = (), Error = Error> {
        let subject = self.subject.clone();
        let sender = self.sender.clone();
        let (completer, sink) = unbounded::<()>();
        self.completer = completer.clone();
        let self_channel = self.channel.clone();
        client.create_confirm_channel(ConfirmSelectOptions::default()).and_then(move |channel| {
            let mut arguments = FieldTable::new();
            arguments.insert("x-expires".to_string(), lapin_futures::types::AMQPValue::LongUInt(DEFAULT_QUEUE_EXPIRATION));
            let mut queue_declare_options = QueueDeclareOptions::default();
            queue_declare_options.durable = true;
            channel.queue_declare(&subject[..], queue_declare_options, arguments).map(move |queue| (channel, queue))
                .and_then(move |(channel, queue)| {
                    let mut basic_consumer_options = BasicConsumeOptions::default();
                    basic_consumer_options.no_ack = true;
                    channel.basic_consume(&queue, "", basic_consumer_options, FieldTable::new()).map(move |stream| (channel, stream))
                }).and_then(move |(channel, stream)| {
                    let mut guard = self_channel.lock().unwrap();
                    *guard = Some(channel);
                    let completion_pact = CompletionPact::new(stream, sink);
                    completion_pact.for_each(move |message| {
                        let payload = std::str::from_utf8(&message.data);
                        if payload.is_ok() {
                            let payload = payload.unwrap().to_string();
                            let response = BrokerResponse::Message {
                                subject: subject.clone(),
                                payload,
                                reply_to: message.properties.reply_to().clone().unwrap_or("".to_string()),
                            };
                            if sender.unbounded_send(response).is_err() {
                                error!("failed sending broker message to channel!");
                            };
                        } else {
                            error!("invalid payload for message!");
                        }
                        //channel.basic_ack(message.delivery_tag, false)
                        future::ok(())
                    })
            })
        }).map_err(|_| Error::generic("failed connecting!"))
    }
}

impl Drop for Consumer {
    fn drop(&mut self) {
        debug!("dropping consumer for [{}]", self.subject.bright_green());
        let guard = self.channel.lock().unwrap();
        if let Some(ref channel) = *guard {
            tokio::spawn(channel.close(0, "").map_err(|_| {}));
        }
        if self.completer.unbounded_send(()).is_err() {
            error!("failed sending completion signal!");
        };
    }
}

pub fn handle_post_message<T: AsyncRead + AsyncWrite + Sync + Send + 'static>(client: &lapin_futures::client::Client<T>, subject: String, payload: String, reply_to: String) -> impl Future<Item = (), Error = Error> {
    client.create_confirm_channel(ConfirmSelectOptions::default()).and_then(move |channel| {
        let mut arguments = FieldTable::new();
        arguments.insert("x-expires".to_string(), lapin_futures::types::AMQPValue::LongUInt(DEFAULT_QUEUE_EXPIRATION));
        let mut queue_declare_options = QueueDeclareOptions::default();
        queue_declare_options.durable = true;
        channel.queue_declare(&subject[..], queue_declare_options, arguments)
            .and_then(move |_| {
                channel.basic_publish("", &subject[..], payload.into_bytes(),
                    BasicPublishOptions::default(),
                    BasicProperties::default().with_reply_to(reply_to).with_expiration(DEFAULT_MESSAGE_EXPIRATION.to_string()))
                .and_then(move |_| {
                    channel.close(0, "")
                })
            })
    }).map(|_| ()).map_err(|_| Error::generic("failed connecting!"))
}

struct CompletionPact<S, C>
    where S: Stream,
          C: Stream,
{
    stream: S,
    completer: C,
}

impl<S, C> CompletionPact<S, C> where S: Stream, C: Stream {
    fn new(s: S, c: C) -> CompletionPact<S, C>
    {
        CompletionPact {
            stream: s,
            completer: c,
        }
    }
}

impl<S, C> Stream for CompletionPact<S, C> where S: Stream, C: Stream {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
        match self.completer.poll() {
            Ok(Async::Ready(None)) |
            Err(_) |
            Ok(Async::Ready(Some(_))) => {
                Ok(Async::Ready(None))
            },
            Ok(Async::NotReady) => {
                self.stream.poll()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use common::crypto::{PublicKey, generate_keypair, Base58};
    use secp256k1::Secp256k1;
    use rand::OsRng;
    use common::base58::*;

    #[test]
    fn base58check() {
        let secp = Secp256k1::new();
        let mut rng = OsRng::new().expect("OsRng");
        let (secret_key, public_key) = secp.generate_keypair(&mut rng);
        let base58_check = public_key.to_base58_check(vec![1,120]);
        let base58 = public_key.to_base58();

        println!("b58 : {}\nb58c: {}", base58, base58_check);

        let public_key_base58 = PublicKey::from_base58(&base58).unwrap();
        let public_key_base58_check = PublicKey::from_base58_check(&base58_check, 2).unwrap();

        println!("{:?}\n{:?}\n{:?}\n", public_key, public_key_base58, public_key_base58_check);
    }

    fn vanity() {
        let secp = Secp256k1::new();
        let mut rng = OsRng::new().expect("OsRng");
        for i in 0..10000000 {
            let (secret_key, public_key) = secp.generate_keypair(&mut rng);
            let public_key = public_key.to_base58();
            if public_key.starts_with("box") || public_key.starts_with("grin") ||
                public_key.starts_with("713") {
                println!("{}\n{}", public_key, secret_key);
            }
            if i % 1000000 == 0 {
                println!("processed {} keys so far...", i);
            }
        }
    }
}
