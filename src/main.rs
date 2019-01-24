#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
use std::net::ToSocketAddrs;

extern crate env_logger;
extern crate uuid;
extern crate serde_json;
extern crate ws;
extern crate lapin_futures;
extern crate nitox;
extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate secp256k1;
extern crate rand;
extern crate sha2;
extern crate digest;
extern crate colored;
extern crate regex;

mod common;

mod broker;
use broker::Broker;

mod server;
use server::AsyncServer;

fn main() {
    env_logger::init();

    info!("hello, world!");

    let broker_uri = std::env::var("BROKER_URI")
        .unwrap_or_else(|_| "127.0.0.1:5672".to_string())
        .to_socket_addrs().unwrap().next();

    let username = std::env::var("RABBITMQ_DEFAULT_USER").unwrap_or("".to_string());
    let password = std::env::var("RABBITMQ_DEFAULT_PASS").unwrap_or("".to_string());

    if broker_uri.is_none() {
        error!("could not resolve broker uri!");
        panic!();
    }

    let broker_uri = broker_uri.unwrap();

    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:13420".to_string());

    info!("Broker URI: {}", broker_uri);
    info!("Bind address: {}", bind_address);

    let broker = Broker::new(broker_uri, username, password);

    let sender = broker.start();
    let response_handlers_sender = AsyncServer::init();
    ws::Builder::new()
        .build(|out| {
            AsyncServer::new(out, sender.clone(), response_handlers_sender.clone())
        })
        .unwrap()
        .listen(&bind_address[..])
        .unwrap();
}
