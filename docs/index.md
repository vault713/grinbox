# Grinbox
A transaction building service for [Grin](https://grin-tech.org), currently under development.
Grin is a blockchain-powered cryptocurrency that is an implementation of the MimbleWimble protocol, with a focus on privacy and scalability.

In MimbleWimble, transactions are interactive, requiring the Sender and Recipient to interact over a single round trip in order to build the transaction. The purpose of Grinbox is to facilitate this interaction by acting as a non-trusted transaction relaying service that routes messages back and forth between senders and recipients. In its first incarnation, Grinbox will run as a terminal service on the Recipient's machine, fetching transactions from the Grinbox server and relaying them to the Grin wallet.

Our aim is to provide this service to the community for free, starting on Testnet2.

## Benefits with using Grinbox
- **Convenient.** You no longer need to be online in order to receive a transaction. Transactions are sent to your Grinbox, waiting for you to fetch them the next time you come online.
- **Private.** You no longer need to expose your IP address as part of the transaction building process. Senders can send grins to your grinbox.io/username. 
- **Secure.** Grinbox cannot read or sign transactions on your behalf. Grinbox does not have access to your wallet or your grins. 

## Steps
1. Download and build the Grinbox client.
2. Configure your environment, register a Grinbox URL.
3. Run Grinbox. A sender can now send a transaction to your Grinbox URL and Grinbox will relay it to your wallet, and communicate back to the sender.

{% include subscription-form.html %}
