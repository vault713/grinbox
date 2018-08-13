## [Grinbox](http://grinbox.io) Relay Service

[![Join the chat at https://gitter.im/vault713/grinbox](https://badges.gitter.im/vault713/grinbox.svg)](https://gitter.im/vault713/grinbox?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

Grinbox relay service provides a simple way for 2 parties to exchange slates as part of the interactive building of a grin transaction.

In order to communicate over the relay, each party has to be able to get pending slates from the relay from a dedicated *relay port*, and to post new slates to the relay to the other party's dedicated *relay port*. The *relay port* is identified by each party's public key.

Assuming Alice and Bob want to use the relay to build a transaction of 50 grins from Alice to Bob:
1. Alice creates a public/private key pair and an access signature to her dedicated *relay port*
2. Bob creates a public/private key pair and an access signature to his dedicated *relay port*
3. Bob sends Alice his public key
4. Alice creates a slate for sending 50 grins to Bob and posts it into Bob's *relay port*
5. Bob gets the slate from his *relay port* using his signature
6. Bob processes the slate and posts the response into Alice's *relay port*
7. Alice gets the slate using her signature
8. Alice processes the slates and commits the transaction to grin

## Private/Public Key Generation

Key pair generation is done on the client side using SECP256K1

Sample code for key generation in Node.JS with SECP256K1 lib
```
let privateKey: Buffer;
do {
  privateKey = randomBytes(32);
} while (!secp256k1.privateKeyVerify(privateKey));
const publicKey = secp256k1.publicKeyCreate(privateKey, false);
```

Sample Private Key (hex)
```
7d84f0c35abbdc64f5438ae3f0179e4768dd444505dcfcf0f3a8842b89beeef0`
```

Sample Public Key (hex), which doubles as the *relay port*
```
046c3c115c3d1e61d3500e04c86059244b2fa80cd2111be72e225a7f21d8f65dca41429b5d0d37035081b98136bf3346560f48ba84bf26d5453660e2efc4fe0eb3`
```

## Signing

In order to get slates from the *relay port* for a given public key, a signature hash to be provided. The signature is a SECP256K1 ECDSA using the correspomding private key, where the message is a hash value (32byte) that can be obtained from the relay.

The hash value may rotate from time to time, forcing users to re-create a valid signature.

To obtain the current hash from the relay service, use the following REST API:

```bash
$ curl http://grinbox.io:3000/hash
a48afabdb456cf1c85f107eb1e62f9f70fcc0520ca296e7d0167174d2b522af6
```

Once the has is obtained signing is done by:

```bash
secp256k1.sign(hash, privateKey)
```

See below the generated signature (hex) for the private key and hash given above:

```bash
1a44ef050f92a13d8297201ee8f8199ef0f86ded647f059bcc519f89b1804e04169d2a583fa405dd92a95bfaf7ee5b4e3f0b25c3cd806b2c9372f861117aead7
```

Sending invalid signatures to the relay GET/POST slate REST APIs (see below) will result in a 401 error.

## Post Slate

Back to the example of Alice and Bob. Alice and Bob both have generated keys and signatures:

Alice:
```
{
    "publicKey": "0485508b11097452c61a2647851d5f7a3c19d10261ee973aee8ef587ee31f70b20a343dfe37c8f03eda3abcd440139c94c5c72ba7b81a3bc28fc699075ebac1338",
    "privateKey": "4ef416c41d8c8c3807bd43cc8d0c908872a14c8307a23bf386ca621927c9fd53",
    "signature": "f477950d9f06e5bf3557b7970f3a6256d90f4d55b62c50c1585fe4731766f38c4db9646b3a117afad74007ec6c0015ca03d579a61bd263642e135cf5fa856d8d"
}
```

Bob:
```
{
    "publicKey": "0412b3c4c615fcfee9789c2753353e1b75aad876f5462eb46d48d0470bdb09d1e0f46e42172c0d40c341fd2c14075540f72e575dd38eb8d728db3da7867e70be85",
    "privateKey": "5b4516d03695cde642fbbef2d8e6c56dea181f356740a9ae180f6043a31fe632",
    "signature": "20de7691c9c805401312f125cd3a8ab8573e64893090efa6836dd41be9f8b3bd1d3a0341893ef68e55d0bc8ea311839c8c7bf7898fdaf4579b2db4ef6b0c19c8"
}
```

Alice created a slate to send 50 grins to Bob and can now post the slate into the relay using POST slate REST API.

```
curl -i -X POST \
    -H "Content-Type: application/json" \ 
    -H "Grinbox-Port-From: 0485508b11097452c61a2647851d5f7a3c19d10261ee973aee8ef587ee31f70b20a343dfe37c8f03eda3abcd440139c94c5c72ba7b81a3bc28fc699075ebac1338" \
    -H "Grinbox-Signature: f477950d9f06e5bf3557b7970f3a6256d90f4d55b62c50c1585fe4731766f38c4db9646b3a117afad74007ec6c0015ca03d579a61bd263642e135cf5fa856d8d" \
    -H "Grinbox-Port-To: 0412b3c4c615fcfee9789c2753353e1b75aad876f5462eb46d48d0470bdb09d1e0f46e42172c0d40c341fd2c14075540f72e575dd38eb8d728db3da7867e70be85" \
    -H "Grinbox-Slate-TTL: 120" \
    -d @LOCAL_SLATE_FILE http://grinbox.io:3000/slate
```

Note that Alice needs to provide a number of grinbox headers to the post command:
1. Grinbox-Port-From - the public port originating the slate, this is where Bob can post a slate back to Alice
2. Grinbox-Signature - the proof that Alice indeed has access to read from port specified in 1
3. Grinbox-Port-To - Bob's public port where the slate would be posted
4. Grinbox-Slate-TTL - the slate TTL in seconds, afterwhich the slate is not guaranteed to persist in the relay 

As well as to provide the slate json in the body of the message (i.e. @LOCAL_SLATE_FILE should point to a local slate file previously generated with `./grin wallet send -d LOCAL_FILE`)

## Get Slate

Bob can now get the slate posted in his *relay port* by executing the following command:

```
curl -i \
    -H "Content-Type: application/json" \
    -H "Grinbox-Port-From: 0412b3c4c615fcfee9789c2753353e1b75aad876f5462eb46d48d0470bdb09d1e0f46e42172c0d40c341fd2c14075540f72e575dd38eb8d728db3da7867e70be85" \
    -H "Grinbox-Signature: 20de7691c9c805401312f125cd3a8ab8573e64893090efa6836dd41be9f8b3bd1d3a0341893ef68e55d0bc8ea311839c8c7bf7898fdaf4579b2db4ef6b0c19c8" \
    http://grinbox.io:3000/slate
```

Note that Bob needs to provide a number of grinbox headers to the GET command:
1. Grinbox-Port-From - the public port from which to read the slate
2. Grinbox-Signature - the proof that Bob indeed has access to read from port specified in 1

Note the GET command will return the first available slate in Bob's queue. If the queue is empty a 404 is returned.

In reply Bob gets the data he requires to process the slate:

```
HTTP/1.1 200 OK
X-Powered-By: Express
Grinbox-Port-From: 0485508b11097452c61a2647851d5f7a3c19d10261ee973aee8ef587ee31f70b20a343dfe37c8f03eda3abcd440139c94c5c72ba7b81a3bc28fc699075ebac1338
Content-Type: application/json; charset=utf-8
Content-Length: 2
ETag: W/"2-vyGp6PvFo4RvsFtPoIWeCReyIC8"
Date: Mon, 13 Aug 2018 09:07:24 GMT
Connection: keep-alive

{}
```

Note the slate is returned, along with a Grinbox-Port-From header which specify where Bob needs to post back the reply to, in this case, Alice's *relay port*.

Bob will need to put the slate in a file, and run it through his grin wallet receive command to generate the response slate.

Once the slate response file is generated, Bob uses the POST slate command with the correct headers to POST the response back to Alice:

```
curl -i 
    -X POST \
    -H "Content-Type: application/json" \
    -H "Grinbox-Port-From: 0412b3c4c615fcfee9789c2753353e1b75aad876f5462eb46d48d0470bdb09d1e0f46e42172c0d40c341fd2c14075540f72e575dd38eb8d728db3da7867e70be85" \
    -H "Grinbox-Signature: 20de7691c9c805401312f125cd3a8ab8573e64893090efa6836dd41be9f8b3bd1d3a0341893ef68e55d0bc8ea311839c8c7bf7898fdaf4579b2db4ef6b0c19c8" \
    -H "Grinbox-Port-To: 0485508b11097452c61a2647851d5f7a3c19d10261ee973aee8ef587ee31f70b20a343dfe37c8f03eda3abcd440139c94c5c72ba7b81a3bc28fc699075ebac1338" \
    -H "Grinbox-Slate-TTL: 120" \
    -d @LOCAL_SLATE_RESPONSE_FILE \
    http://grinbox.io:3000/slate
```

## Helper REST API for key pair & signature generation

The following helper REST API can be used to generate the necessary keys and signatures to access a *relay port*:

```
curl http://grinbox.io:3000/port
```

The response is a json containing the resulting new pair and signature:

```
{
  "publicKey": "04ef86b0711823dfe77b732f60fffe282d9f4a7ba1ea71dfc2459ed83d18a33d0d2e81c8d055acdf5f91a9533ae8efb3951fd8bf905cd075be564eb2fc5145b05a",
  "privateKey": "d0107fd154ecf485d9318e6a1c869695a10374a19fedb6b02e6419674bf176ba",
  "signature": "6d77f423f84c4f7edea8cf1b1db9944406de7cb49193f03345c47cb02af76b5d500ec8d0ad60017e42c9a81df7bcfcbccb2698d41b82ea1488d358d72a5f27cf"
}
```

## License

Grinbox is [MIT licensed](LICENSE).
