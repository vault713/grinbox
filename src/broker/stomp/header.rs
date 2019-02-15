// Non-camel case types are used for Stomp Protocol version enum variants
#![macro_use]
use std;
use std::slice::Iter;
use unicode_segmentation::UnicodeSegmentation;

// Ideally this would be a simple typedef. However:
// See Rust bug #11047: https://github.com/mozilla/rust/issues/11047
// Cannot call static methods (`with_capacity`) on type aliases (`HeaderList`)
#[derive(Clone, Debug)]
pub struct HeaderList {
    pub headers: Vec<Header>,
}

impl HeaderList {
    pub fn new() -> HeaderList {
        HeaderList::with_capacity(0)
    }
    pub fn with_capacity(capacity: usize) -> HeaderList {
        HeaderList {
            headers: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, header: Header) {
        self.headers.push(header);
    }

    pub fn pop(&mut self) -> Option<Header> {
        self.headers.pop()
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, Header> {
        self.headers.iter()
    }

    pub fn drain<F>(&mut self, mut sink: F)
        where
            F: FnMut(Header),
    {
        while let Some(header) = self.headers.pop() {
            sink(header);
        }
    }

    pub fn concat(&mut self, other_list: &mut HeaderList) {
        other_list.headers.reverse();
        while let Some(header) = other_list.pop() {
            self.headers.push(header);
        }
    }

    pub fn retain<F>(&mut self, test: F)
        where
            F: Fn(&Header) -> bool,
    {
        self.headers.retain(test)
    }
}

pub struct SuppressedHeader<'a>(pub &'a str);
#[derive(Clone, Debug)]
pub struct Header(pub HeaderName, pub String);

impl Header {
    pub fn new(key: HeaderName, value: &str) -> Header {
        Header(key, value.to_string())
    }

    pub fn get_raw(&self) -> String {
        format!("{}:{}", self.0.as_str(), self.1)
    }

    pub fn encode_value(value: &str) -> String {
        let mut encoded = String::new(); //self.strings.detached();
        for grapheme in UnicodeSegmentation::graphemes(value, true) {
            match grapheme {
                "\\" => encoded.push_str(r"\\"), // Order is significant
                "\r" => encoded.push_str(r"\r"),
                "\n" => encoded.push_str(r"\n"),
                ":" => encoded.push_str(r"\c"),
                g => encoded.push_str(g),
            }
        }
        encoded
    }

    pub fn decode_value(value: &str) -> String {
        let decoded = value.to_string().replace(r"\c", ":");
        decoded
    }

    pub fn get_key<'a>(&'a self) -> HeaderName {
        self.0.clone()
    }

    pub fn get_value<'a>(&'a self) -> &'a str {
        &self.1
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct HeaderName {
    inner: Repr<Custom>,
}
impl HeaderName {
    pub fn from_str(src: &str) -> Self {
        let encoded = Header::encode_value(src);
        let inner = match encoded.parse::<StandardHeader>() {
            Ok(h) => Repr::Standard(h),
            Err(_e) => Repr::Custom(Custom(encoded)),
        };
        Self { inner }
    }
    pub fn as_str(&self) -> &str {
        match self.inner {
            Repr::Standard(v) => v.as_str(),
            Repr::Custom(ref v) => v.0.as_str(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Repr<T> {
    Standard(StandardHeader),
    Custom(T),
}

// Used to hijack the Hash impl
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Custom(String);

macro_rules! standard_headers {
    (
        $(
            $(#[$docs:meta])*
            ($konst:ident, $upcase:ident, $name:expr);
        )+
    ) => {
        #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
        enum StandardHeader {
            $(
                $konst,
            )+
        }

        $(
            $(#[$docs])*
            pub const $upcase: HeaderName = HeaderName {
                inner: Repr::Standard(StandardHeader::$konst),
            };
        )+

        impl std::str::FromStr for StandardHeader {
            type Err = ();
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                    $name => Ok(StandardHeader::$konst ),
                    )+
                    _ => Err(())
                }
            }
        }

        impl StandardHeader {
            #[inline]
            fn as_str(&self) -> &'static str {
                match *self {
                    $(
                    StandardHeader::$konst => $name,
                    )+
                }
            }
        }
    }
}

standard_headers! {
    (ContentType, CONTENT_TYPE, "content-type");
    (AcceptVerion, ACCEPT_VERSION, "accept-version");
    (Ack, ACK, "ack");
    (ContentLength, CONTENT_LENGTH, "content-length");
    (Destination, DESTINATION, "destination");
    (HeartBeat, HEART_BEAT, "heart-beat");
    (Host, HOST, "host");
    (Id, ID, "id");
    (Login, LOGIN, "login");
    (MessageId, MESSAGE_ID, "message-id");
    (Passcode, PASSCODE, "passcode");
    (Receipt, RECEIPT, "receipt");
    (ReceiptID, RECEIPT_ID, "receipt-id");
    (Server, SERVER, "server");
    (Session, SESSION, "session");
    (Subscription, SUBSCRIPTION, "subscription");
    (Transaction, TRANSACTION, "transaction");
    (Version, VERSION, "version");
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum StompVersion {
    Stomp_v1_0,
    Stomp_v1_1,
    Stomp_v1_2,
}
impl std::str::FromStr for StompVersion {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1.0" => Ok(StompVersion::Stomp_v1_0),
            "1.1" => Ok(StompVersion::Stomp_v1_1),
            "1.2" => Ok(StompVersion::Stomp_v1_2),
            _ => Err(()),
        }
    }
}

impl HeaderList {
    pub fn get<'a>(&'a self, key: HeaderName) -> Option<&'a str> {
        self.headers
            .iter()
            .find(|header| header.get_key() == key)
            .map(|v| v.get_value())
    }

    pub fn get_accept_version(&self) -> Option<Vec<StompVersion>> {
        let versions: &str = self.get(ACCEPT_VERSION)?;
        let versions: Vec<StompVersion> = versions
            .split(',')
            .filter_map(|v| v.trim().parse::<StompVersion>().ok())
            .collect();
        Some(versions)
    }

    pub fn get_heart_beat(&self) -> Option<(u32, u32)> {
        let spec = self.get(HEART_BEAT)?;
        trace!("hb: {}", spec);
        let spec_list: Vec<u32> = spec
            .split(',')
            .filter_map(|str_val| str_val.parse::<u32>().ok())
            .collect();

        if spec_list.len() != 2 {
            return None;
        }
        Some((spec_list[0], spec_list[1]))
    }
}

#[macro_export]
macro_rules! header_list [
  ($($header: expr), *) => ({
    let header_list = HeaderList::new();
    $(header_list.push($header);)*
    header_list
  });
  ($($key:expr => $value: expr), *) => ({
    let mut header_list = HeaderList::new();
    $(header_list.push(Header::new($key, $value));)*
    header_list
  })
];

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn encode_return_carriage() {
        let unencoded = "Hello\rWorld";
        let encoded = r"Hello\rWorld";
        assert!(encoded == Header::encode_value(unencoded));
    }

    #[test]
    fn encode_newline() {
        let unencoded = "Hello\nWorld";
        let encoded = r"Hello\nWorld";
        assert!(encoded == Header::encode_value(unencoded));
    }

    #[test]
    fn encode_colon() {
        let unencoded = "Hello:World";
        let encoded = r"Hello\cWorld";
        assert!(encoded == Header::encode_value(unencoded));
    }

    #[test]
    fn encode_slash() {
        let unencoded = r"Hello\World";
        let encoded = r"Hello\\World";
        assert!(encoded == Header::encode_value(unencoded));
    }
}
