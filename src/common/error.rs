use super::protocol::ProtocolError;

#[derive(Debug)]
pub enum Error {
    Generic { description: &'static str },
    ProtocolError { kind: ProtocolError },
    Secp { e: secp256k1::Error },
    InvalidBase58Character(char, usize),
    InvalidBase58Length,
    InvalidBase58Checksum
}

impl Error {
    pub fn generic(description: &'static str) -> Self {
        Error::Generic { description }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Generic { description } => write!(f, "{}", description),
            Error::Secp { e } => write!(f, "{}", e),
            _ => write!(f, "{:?}", self)
        }
    }
}

impl From<secp256k1::Error> for Error {
    fn from(e: secp256k1::Error) -> Self {
        Error::Secp { e }
    }
}
