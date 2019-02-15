use super::session::{Session, ReceiptRequest, OutstandingReceipt};
use super::frame::Frame;
use super::option_setter::OptionSetter;

pub struct MessageBuilder<'a, T: 'static> {
    pub session: &'a mut Session<T>,
    pub frame: Frame,
    pub receipt_request: Option<ReceiptRequest>,
}

impl<'a, T> MessageBuilder<'a, T>
    where
        T: tokio_io::AsyncWrite + tokio_io::AsyncRead + Send + 'static,
{
    pub fn new(session: &'a mut Session<T>, frame: Frame) -> Self {
        MessageBuilder {
            session,
            frame,
            receipt_request: None,
        }
    }

    pub fn send(self) {
        if self.receipt_request.is_some() {
            let request = self.receipt_request.unwrap();
            self.session
                .state
                .outstanding_receipts
                .insert(request.id, OutstandingReceipt::new(self.frame.clone()));
        }
        self.session.send_frame(self.frame)
    }

    pub fn with<O>(self, option_setter: O) -> MessageBuilder<'a, T>
        where
            O: OptionSetter<MessageBuilder<'a, T>>,
    {
        option_setter.set_option(self)
    }
}

