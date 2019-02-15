use super::frame::Frame;
use super::frame::ToFrameBody;
use super::message_builder::MessageBuilder;
use super::header::*;
use super::session::Session;

pub struct Transaction<'tx, T: 'static> {
    pub id: String,
    pub session: &'tx mut Session<T>,
}

impl<'tx, T: 'static> Transaction<'tx, T>
    where
        T: tokio_io::AsyncWrite + tokio_io::AsyncRead + Send + 'static,
{
    pub fn new(session: &'tx mut Session<T>) -> Transaction<'tx, T> {
        Transaction {
            id: format!("tx/{}", session.generate_transaction_id()),
            session,
        }
    }

    pub fn message<'builder, B: ToFrameBody>(
        &'builder mut self,
        destination: &str,
        body_convertible: B,
    ) -> MessageBuilder<'builder, T> {
        let mut send_frame = Frame::send(destination, body_convertible.to_frame_body());
        send_frame
            .headers
            .push(Header::new(TRANSACTION, self.id.as_ref()));
        MessageBuilder::new(self.session, send_frame)
    }

    // TODO: See if it's feasible to do this via command_sender

    pub fn begin(&mut self) {
        let begin_frame = Frame::begin(self.id.as_ref());
        self.session.send_frame(begin_frame)
    }

    pub fn commit(self) {
        let commit_frame = Frame::commit(self.id.as_ref());
        self.session.send_frame(commit_frame)
    }

    pub fn abort(self) {
        let abort_frame = Frame::abort(self.id.as_ref());
        self.session.send_frame(abort_frame)
    }
}
