# Grinbox Integration Guide

## Introduction

Grinbox is a relay service and protocol for slate exchange between anonymous parties for building grin transactions.

### Data & Privacy

## Setup

### Requirements

* rust 1.31+ (use rustup- i.e. curl https://sh.rustup.rs -sSf | sh; source $HOME/.cargo/env)
if rust is already installed, you can simply update version with rustup update

* A running instance of [rabbitmq](https://www.rabbitmq.com/)

### Environment Variables

* `BROKER_URI`: The rabbitmq broker URI in the form of (i.e. domain:port). defaults to 127.0.0.1:5672
* `RABBITMQ_DEFAULT_USER`: The username with which grinbox would establish connection to the rabbit broker.
* `RABBITMQ_DEFAULT_PASS`: The associated password to use
* `BIND_ADDRESS`: The http listener bind address (defaults to 0.0.0.0:3420)

### Installation

```
$ git clone https://github.com/vault713/grinbox
$ cd grinbox
$ cargo build --release
```
And then to run:
```
$ cd target/release
$ ./grinbox
``` 

Once grinbox is running it should establish connection with the rabbitmq broker and start to listen to incoming connection on the bind address.

## Integration

The following section covers integration requirements for client that want to communicate with a grinbox relay.

### Create Grinbox addresses

In order to be able to use grinbox, client must have a valid `grinbox address`.

#### Grinbox address format

A grinbox address is composed of 4 components:
1. Scheme = "grinbox" [optional, can be omitted]
2. A base58-check encoded secp256k1 public key (with 2 version bytes)
3. Relay Domain [optional, defaults to `grinbox.io`]
4. Relay Port number [optional, defaults to 443]

```
Note that mainnet and testnet use different version bytes to clearly differentiate real vs. test address:
Mainnet version bytes: [1, 11] which generate addresses starting with a `g` in the public key component
Testnet version bytes: [1, 120] which generate addresses starting with an `x` in the public key component
```

Examples of valid addresses for testnet:

```
xd8dyGUuP89t2TT9N9yU7pZsr6VyJAH6wZauESBQe67q741bVoaN
grinbox://xd95u2toAVHE85BCHTi2tqddL6po3g4JVv8fFXVJGUTuMYKn6Bhp
grinbox://xd9XfKTUCGr6iwzuDKyfN8N3EXd19z4kinCWTJyK5LMzdvoY9AZs@example.com
grinbox://xd8EBsMXfYKyDJUiXYURNXJZ5e66hTJWweeYKgjPxwqXYEgkt8SE@example.com:13420
```
Examples of valid addresses for mainnet:

```
gVuQ7cspvtjKZNBuoxyjrLbNTXhqKt7Hd3MnjfMBr3kSE6z3XkCp
grinbox://gVv3oFZofJYFtRQKcab1xwhu3dAZ2EfqfuoxnwYoXL7VU7Tr8Ch2
grinbox://gVwJ8hXkKyeGMsXgK38URgn5vYeuWRXrBzHGrgM7YcjifJEj52J6@example.com
grinbox://gVuyUw615UYnrUrMc19r8KfaECkrp1UzTT1HUxnXpaUebAGb9Y3s@example.com:13420
```

### Connect to grinbox

Client communication with the grinbox service is done via websockets, utilizing a json-based protocol, where the client issues json-encoded requests to the server via the websocket, and gets json-encoded responses. Connection to grinbox can be done by any client supportive of RFC6455 websocket protocol, while actual communication with the server involves a custom set of json-encoded messages.

Each message is a json object with a `type` attribute that designates the type of message it is, and optional additional attributes depending on the message type.

#### Grinbox Protocol

##### Challenge

Grinbox uses a rolling challenge provided by the server for authenticating ownership of a grinbox address. When clients interact with grinbox to post slates and to get pending slates, they have to assert ownership of their address. They do this by signing the challenge with the private key associated with the address in question.

Upon successful connection to grinbox, the server sends the current challenge to the user in the context of a `Challenge` message.

```
{
	"type": "Challenge",
	"str": "<the current challenge>"
}
```

The client is expect to hold on to the challenge, and use it to sign subsequent requests as appropriate.

Additionally, the client should expect to occasionally receive new challenge messages.

##### Post a Slate

`PostSlate` message is used by a client to send a slate to a receiver. It includes the (encrypted) slate, a destination address as well as a from address and a signature to validate and prove ownership of the from address by the sender. The `from` address will later be used by the receiver in order to reply to the sender as part of the tx building interaction.

To generate the signature, the slate sender has to sign a the challenge composed of both the (encrypted) slate and the current challenge that was retrieved earlier from the server. The signature will be validated by ther server as part of post slate, and in case the signature does not match, the server will reject the slate.

###### Request:

```
{ 
	"type": "PostSlate", 
	"from": "<grinbox address of slate sender>", 
	"to": "<grinbox address of slate receiver>", 
	"str": "<slate encrypted using public key of receiver>",
	"signature": "<signature for str + current challenge using the from address private key>"
}
```

###### Response:

Successful Response: `{ "type": "Ok" }`

Error Response: `{ "type": "Error", "kind": "<error kind>", "description": "<description of the error>"}`

##### Subscribe to an Address

`Subscribe` message is used by a client to get all incoming slates to a specific address. In order to subscribe a user must be able to prove ownership of the address, by signing a challenge using the private key associated with the address.

Once subscription is established successfully, the server would send any pending slates, and all future incoming slates, to the client's websocket.

###### Request:

```
{
	"type": "Subscribe",
	"address": "<the grinbox address>",
	"signature": "<the current challenge signed with the private key of the `address`>"
}
```

###### Response:

Successful Response: `{ "type": "Ok" }`

Error Response: `{ "type": "Error", "kind": "<error kind>", "description": "<description of the error>"}`

##### Unsubscribe from an Address

`Unsubscribe` message is used remove open subscription. Once done, the client will stop receiving slates from the given address.

###### Request:

```
{
	"type": "Unsubscribe",
	"address": "<the grinbox address>",
}
```

###### Response:

Successful Response: `{ "type": "Ok" }`

Error Response: `{ "type": "Error", "kind": "<error kind>", "description": "<description of the error>"}`

#### Encrypting slates

#### Decrypting slates

## Changelog

## Tests

## Troubleshooting

## Reference client implementations

Language | Client | Comment
|---|---|---|
Rust | [Wallet713](https://github.com/vault713/wallet713) | Original implementation, developed by grinbox developers.

## Contributing

## Contact