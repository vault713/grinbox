# [713.grinbox](http://grinbox.io) Relay Service

[![Join the chat at https://gitter.im/vault713/grinbox](https://badges.gitter.im/vault713/grinbox.svg)](https://gitter.im/vault713/grinbox?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

## Introduction
[Grin](https://github.com/mimblewimble/grin) is a blockchain-powered cryptocurrency that is an implementation of the MimbleWimble protocol, with a focus on privacy and scalability. In MimbleWimble, transactions are interactive, requiring the Sender and Recipient to interact over a single round trip in order to build the transaction.

### What's 713.grinbox?

713.grinbox provides a simple way for parties to exchange transaction slates as part of the interactive process of building a valid Grin transaction.

In order to communicate over the relay, each party has to be able to get pending slates from the relay from a dedicated address, and to post new slates to the relay to the other party's dedicated address. **The address is identified by each party's public key.**

## Transaction flow overview
Assuming Alice wants to send Bob 50 grins using the relay: 
1. Alice creates a public/private key pair and an access signature to use as her dedicated address
2. Bob creates a public/private key pair and an access signature to use as his dedicated address
3. Bob sends Alice his public key
4. Alice creates a slate for sending 50 grins to Bob and posts it to Bob's address, identified by the public key in the previous step
5. Bob gets the slate from his address using his signature
6. Bob processes the slate and posts the response into Alice's address
7. Alice gets the slate from her address using her signature
8. Alice finalises the transaction and broadcasts it to the Grin blockchain

## Functionality

* Written in Rust, 713.grinbox utilises websockets to communicate with relay users.
* Relay server federation is supported, allowing you to run your own 713.grinbox server and be accessible as `publickey@yourdomain.com`.

## Getting started

* 713.grinbox is integrated with [wallet713](https://github.com/vault713/wallet713), build the latest version of the wallet and try it out.

## Integration instructions

Coming soon. If you want to integrate with 713.grinbox, please reach out to us at [hello@713.mw](mailto:hello@713.mw) or on  [Gitter](https://gitter.im/vault713/grinbox) and we'll help with this.

## Privacy considerations

* **The relay does not store data.** 713.grinbox does not store any data on completed transactions by design, but it would be possible for the relay to do so and as a result build a graph of activity between addresses.

* **Your IP is your responsibility.** When you communicate to the 713.grinbox relay service, you are exposing your IP to the relay. You can obfuscate your real IP address using services such as a VPN and/or TOR or i2p.

## License

Apache License v2.0. 