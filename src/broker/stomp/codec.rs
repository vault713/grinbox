use std::io::Error as IoError;
use std::str;
use bytes::BytesMut;
use tokio_io::codec::{Encoder, Decoder};
use futures::prelude::*;

use super::header::{Header, HeaderName, HeaderList, CONTENT_LENGTH};
use super::frame::{Command, Frame, Transmission};

macro_rules! opt_nr {
    ($opt: expr) => {
        match $opt {
            Some(v) => v,
            None => return Ok(Async::NotReady),
        }
    };
}

#[derive(Debug)]
pub enum ParseError {
    Utf8,
    ContentLength,
    UnknownCommand(String),
    Invalid,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self)
    }
}
impl std::error::Error for ParseError {}

fn parse_transmission(src0: &[u8]) -> Poll<(Transmission, usize), ParseError> {
    let (command, mut src) = try_ready!(get_line(src0));
    if command.is_empty() {
        return Ok(Async::Ready((
            Transmission::HeartBeat,
            src0.len() - src.len(),
        )));
    }

    let command = parse_command(command)?;

    let mut headers = HeaderList::new();

    loop {
        let (line, src1) = try_ready!(get_line(src));
        src = src1;
        if line.is_empty() {
            break;
        }
        let header = try_ready!(parse_header(line));
        headers.push(header);
    }

    let (src1, body) = match headers.get(CONTENT_LENGTH) {
        Some(len) => {
            let len = len.parse().map_err(|_e| ParseError::ContentLength)?;
            if src.len() <= len {
                return Ok(Async::NotReady);
            }
            if src[len] != b'\0' {
                return Err(ParseError::Invalid);
            }

            (&src[(len + 1)..], Vec::from(&src[..len]))
        }
        None => {
            let mut split = src.splitn(2, |b| *b == b'\0');
            let body = opt_nr!(split.next());
            let src = opt_nr!(split.next());
            (src, Vec::from(body))
        }
    };
    src = src1;

    let frame = Frame {
        command,
        headers,
        body,
    };

    Ok(Async::Ready((
        Transmission::CompleteFrame(frame),
        src0.len() - src.len(),
    )))
}

fn parse_header(src: &[u8]) -> Poll<Header, ParseError> {
    let src = str::from_utf8(src).map_err(|_e| ParseError::Utf8)?;
    let mut parts = src.split(':');

    let key = opt_nr!(parts.next());
    let value = opt_nr!(parts.next());

    Ok(Async::Ready(Header::new(HeaderName::from_str(key), &Header::decode_value(value))))
}

fn parse_command(src: &[u8]) -> Result<Command, ParseError> {
    let command = match src {
        b"CONNECTED" => Command::Connected,
        b"MESSAGE" => Command::Message,
        b"RECEIPT" => Command::Receipt,
        b"ERROR" => Command::Error,
        unknown => {
            return Err(ParseError::UnknownCommand(
                str::from_utf8(unknown).unwrap().to_owned(),
            ))
        }
    };
    Ok(command)
}

fn get_line<'a>(src: &'a [u8]) -> Poll<(&'a [u8], &'a [u8]), ParseError> {
    let mut split = src.splitn(2, |b| *b == b'\n');

    let mut line = opt_nr!(split.next());
    let remain = opt_nr!(split.next());

    if !line.is_empty() && line[line.len() - 1] == b'\r' {
        line = &line[..(line.len() - 1)];
    }
    Ok(Async::Ready((line, remain)))
}

pub struct Codec;

impl Encoder for Codec {
    type Item = Transmission;
    type Error = IoError;
    fn encode(&mut self, item: Transmission, buffer: &mut BytesMut) -> Result<(), IoError> {
        item.write(buffer);
        Ok(())
    }
}

impl Decoder for Codec {
    type Item = Transmission;
    type Error = IoError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Transmission>, IoError> {
        match parse_transmission(&src) {
            Ok(Async::NotReady) => Ok(None),
            Ok(Async::Ready((t, len))) => {
                src.split_to(len);
                Ok(Some(t))
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}