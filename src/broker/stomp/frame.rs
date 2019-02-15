use std::str::from_utf8;
use std::fmt;
use std::fmt::Formatter;
use bytes::BytesMut;

use super::header::*;
use super::subscription::AckMode;

#[derive(Copy, Clone, Debug)]
pub enum Command {
    Send,
    Subscribe,
    Unsubscribe,
    Begin,
    Commit,
    Abort,
    Ack,
    Nack,
    Disconnect,
    Connect,
    Stomp,
    Connected,
    Message,
    Receipt,
    Error,
}

impl Command {
    pub fn as_str(&self) -> &'static str {
        use self::Command::*;

        match *self {
            Send => "SEND",
            Subscribe => "SUBSCRIBE",
            Unsubscribe => "UNSUBSCRIBE",
            Begin => "BEGIN",
            Commit => "COMMIT",
            Abort => "ABORT",
            Ack => "ACK",
            Nack => "NACK",
            Disconnect => "DISCONNECT",
            Connect => "CONNECT",
            Stomp => "STOMP",
            Connected => "CONNECTED",
            Message => "MESSAGE",
            Receipt => "RECEIPT",
            Error => "ERROR",
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub trait ToFrameBody {
    fn to_frame_body<'a>(&'a self) -> &'a [u8];
}

impl<'b> ToFrameBody for &'b [u8] {
    fn to_frame_body<'a>(&'a self) -> &'a [u8] {
        self
    }
}

impl<'b> ToFrameBody for &'b str {
    fn to_frame_body<'a>(&'a self) -> &'a [u8] {
        self.as_bytes()
    }
}

impl ToFrameBody for String {
    fn to_frame_body<'a>(&'a self) -> &'a [u8] {
        self.as_str().as_bytes()
    }
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub command: Command,
    pub headers: HeaderList,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub enum Transmission {
    HeartBeat,
    CompleteFrame(Frame),
}

impl Transmission {
    pub fn write(&self, out: &mut BytesMut) {
        match *self {
            Transmission::HeartBeat => out.extend("\n".as_bytes()),
            Transmission::CompleteFrame(ref frame) => frame.write(out),
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let space_required = self.count_bytes();
        let mut frame_string = String::with_capacity(space_required); // Faster to just allocate?
        frame_string.push_str(self.command.as_str());
        frame_string.push_str("\n");
        for header in self.headers.iter() {
            frame_string.push_str(&header.get_raw());
            frame_string.push_str("\n");
        }
        frame_string.push_str("\n");
        let body_string: &str = match from_utf8(self.body.as_ref()) {
            Ok(ref s) => *s,
            Err(_) => "<Binary content>", // Space is wasted in this case. Could shrink to fit?
        };
        frame_string.push_str(body_string);

        write!(f, "{}", frame_string)
    }
}

impl Frame {
    fn empty(command: Command, headers: HeaderList) -> Self {
        Self {
            command,
            headers,
            body: Vec::new(),
        }
    }

    fn count_bytes(&self) -> usize {
        let mut space_required: usize = 0;
        // Add one to space calculations to make room for '\n'
        space_required += self.command.as_str().len() + 1;
        space_required += self
            .headers
            .iter()
            .fold(0, |length, header| length + header.get_raw().len() + 1);
        space_required += 1; // Newline at end of headers
        space_required += self.body.len();
        space_required
    }

    pub fn write(&self, out: &mut BytesMut) {
        debug!("Sending frame:\n{}", self.to_string());
        out.extend(self.command.as_str().as_bytes());
        out.extend("\n".as_bytes());

        for header in self.headers.iter() {
            out.extend(header.get_raw().as_bytes());
            out.extend("\n".as_bytes());
        }

        out.extend("\n".as_bytes());
        out.extend(&self.body);

        out.extend(&[0]);
    }

    pub fn connect(tx_heartbeat_ms: u32, rx_heartbeat_ms: u32) -> Self {
        let heart_beat = format!("{},{}", tx_heartbeat_ms, rx_heartbeat_ms);

        Self::empty(
            Command::Connect,
            header_list![
                ACCEPT_VERSION => "1.2",
                HEART_BEAT => heart_beat.as_ref(),
                CONTENT_LENGTH => "0"
            ],
        )
    }

    pub fn disconnect() -> Self {
        Self::empty(
            Command::Disconnect,
            header_list![
                RECEIPT => "msg/disconnect"
            ],
        )
    }

    pub fn subscribe(subscription_id: &str, destination: &str, ack_mode: AckMode) -> Self {
        Self::empty(
            Command::Subscribe,
            header_list![
                DESTINATION => destination,
                ID => subscription_id,
                ACK => ack_mode.as_str()
            ],
        )
    }

    pub fn unsubscribe(subscription_id: &str) -> Self {
        Self::empty(
            Command::Unsubscribe,
            header_list![
                ID => subscription_id
            ],
        )
    }

    pub fn ack(ack_id: &str) -> Self {
        Self::empty(
            Command::Ack,
            header_list![
                ID => ack_id
            ],
        )
    }

    pub fn nack(message_id: &str) -> Self {
        Self::empty(
            Command::Nack,
            header_list![
                ID => message_id
            ],
        )
    }

    pub fn send(destination: &str, body: &[u8]) -> Self {
        Self {
            command: Command::Send,
            headers: header_list![
                DESTINATION => destination,
                CONTENT_LENGTH => body.len().to_string().as_ref()
            ],
            body: body.into(),
        }
    }

    pub fn begin(transaction_id: &str) -> Self {
        Self::empty(
            Command::Begin,
            header_list![
                TRANSACTION => transaction_id
            ],
        )
    }

    pub fn abort(transaction_id: &str) -> Self {
        Self::empty(
            Command::Abort,
            header_list![
                TRANSACTION => transaction_id
            ],
        )
    }

    pub fn commit(transaction_id: &str) -> Self {
        Self::empty(
            Command::Commit,
            header_list![
                TRANSACTION => transaction_id
            ],
        )
    }
}