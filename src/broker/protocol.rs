use futures::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum BrokerRequest {
    Subscribe { subject: String, response_sender: UnboundedSender<BrokerResponse> },
    Unsubscribe { subject: String },
    PostMessage { subject: String, payload: String, reply_to: String },
}

#[derive(Debug)]
pub enum BrokerResponse {
    Message { subject: String, payload: String, reply_to: String },
}