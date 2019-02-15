use failure::Fail;
use crate::types::GrinboxError;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "\x1b[31;1merror:\x1b[0m {}", 0)]
    GenericError(String),
    #[fail(display = "\x1b[31;1merror:\x1b[0m secp error")]
    SecpError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid character!")]
    InvalidBase58Character(char, usize),
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid length!")]
    InvalidBase58Length,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid checksum!")]
    InvalidBase58Checksum,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid network!")]
    InvalidBase58Version,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid key!")]
    InvalidBase58Key,
    #[fail(display = "\x1b[31;1merror:\x1b[0m could not parse number from string!")]
    NumberParsingError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m could not parse `{}` to a grinbox address!", 0)]
    GrinboxAddressParsingError(String),
    #[fail(display = "\x1b[31;1merror:\x1b[0m unable to encrypt message")]
    Encryption,
    #[fail(display = "\x1b[31;1merror:\x1b[0m unable to decrypt message")]
    Decryption,
    #[fail(display = "\x1b[31;1merror:\x1b[0m unable to verify proof")]
    VerifyProof,
    #[fail(display = "\x1b[31;1merror:\x1b[0m grinbox websocket terminated unexpectedly!")]
    GrinboxWebsocketAbnormalTermination,
    #[fail(display = "\x1b[31;1merror:\x1b[0m grinbox protocol error `{}`", 0)]
    GrinboxProtocolError(GrinboxError),
}
