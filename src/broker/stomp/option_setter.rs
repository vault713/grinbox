use super::message_builder::MessageBuilder;
use super::session_builder::SessionBuilder;
use super::subscription_builder::SubscriptionBuilder;
use super::header::*;
use super::connection::{HeartBeat, Credentials, OwnedCredentials};
use super::subscription::AckMode;
use super::session::{ReceiptRequest, GenerateReceipt};

pub trait OptionSetter<T> {
    fn set_option(self, T) -> T;
}

impl<'a, T> OptionSetter<MessageBuilder<'a, T>> for Header {
    fn set_option(self, mut builder: MessageBuilder<'a, T>) -> MessageBuilder<'a, T> {
        builder.frame.headers.push(self);
        builder
    }
}

impl<'a, 'b, T> OptionSetter<MessageBuilder<'b, T>> for SuppressedHeader<'a> {
    fn set_option(self, mut builder: MessageBuilder<'b, T>) -> MessageBuilder<'b, T> {
        let SuppressedHeader(key) = self;
        builder
            .frame
            .headers
            .retain(|header| header.get_key().as_str() != key);
        builder
    }
}

impl OptionSetter<SessionBuilder> for Header {
    fn set_option(self, mut builder: SessionBuilder) -> SessionBuilder {
        builder.config.headers.push(self);
        builder
    }
}

impl OptionSetter<SessionBuilder> for HeartBeat {
    fn set_option(self, mut builder: SessionBuilder) -> SessionBuilder {
        builder.config.heartbeat = self;
        builder
    }
}

impl<'b> OptionSetter<SessionBuilder> for Credentials<'b> {
    fn set_option(self, mut builder: SessionBuilder) -> SessionBuilder {
        builder.config.credentials = Some(OwnedCredentials::from(self));
        builder
    }
}

impl<'b> OptionSetter<SessionBuilder> for SuppressedHeader<'b> {
    fn set_option(self, mut builder: SessionBuilder) -> SessionBuilder {
        let SuppressedHeader(key) = self;
        builder
            .config
            .headers
            .retain(|header| header.get_key().as_str() != key);
        builder
    }
}

impl<'a, T> OptionSetter<SubscriptionBuilder<'a, T>> for Header {
    fn set_option(self, mut builder: SubscriptionBuilder<'a, T>) -> SubscriptionBuilder<'a, T> {
        builder.headers.push(self);
        builder
    }
}

impl<'a, 'b, T> OptionSetter<SubscriptionBuilder<'b, T>> for SuppressedHeader<'a> {
    fn set_option(self, mut builder: SubscriptionBuilder<'b, T>) -> SubscriptionBuilder<'b, T> {
        let SuppressedHeader(key) = self;
        builder
            .headers
            .retain(|header| header.get_key().as_str() != key);
        builder
    }
}

impl<'a, T> OptionSetter<SubscriptionBuilder<'a, T>> for AckMode {
    fn set_option(self, mut builder: SubscriptionBuilder<'a, T>) -> SubscriptionBuilder<'a, T> {
        builder.ack_mode = self;
        builder
    }
}

impl<'a, T> OptionSetter<MessageBuilder<'a, T>> for GenerateReceipt
    where
        T: tokio_io::AsyncWrite + tokio_io::AsyncRead + Send + 'static,
{
    fn set_option(self, mut builder: MessageBuilder<'a, T>) -> MessageBuilder<'a, T> {
        let next_id = builder.session.generate_receipt_id();
        let receipt_id = format!("message/{}", next_id);
        builder.receipt_request = Some(ReceiptRequest::new(receipt_id.clone()));
        builder
            .frame
            .headers
            .push(Header::new(RECEIPT, receipt_id.as_ref()));
        builder
    }
}

impl<'a, T> OptionSetter<SubscriptionBuilder<'a, T>> for GenerateReceipt
    where
        T: tokio_io::AsyncWrite + tokio_io::AsyncRead + Send + 'static,
{
    fn set_option(self, mut builder: SubscriptionBuilder<'a, T>) -> SubscriptionBuilder<'a, T> {
        let next_id = builder.session.generate_receipt_id();
        let receipt_id = format!("message/{}", next_id);
        builder.receipt_request = Some(ReceiptRequest::new(receipt_id.clone()));
        builder
            .headers
            .push(Header::new(RECEIPT, receipt_id.as_ref()));
        builder
    }
}