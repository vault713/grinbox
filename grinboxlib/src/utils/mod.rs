use std::fmt::Write;
use crate::error::{Result, ErrorKind};

pub use grin_core::global::is_mainnet;

pub mod base58;
pub mod crypto;
pub mod secp;

/// Encode the provided bytes into a hex string
pub fn to_hex(bytes: Vec<u8>) -> String {
    let mut s = String::new();
    for byte in bytes {
        write!(&mut s, "{:02x}", byte).expect("Unable to write");
    }
    s
}

/// Decode a hex string into bytes.
pub fn from_hex(hex_str: String) -> Result<Vec<u8>> {
    if hex_str.len() % 2 == 1 {
        Err(ErrorKind::NumberParsingError)?;
    }
    let hex_trim = if &hex_str[..2] == "0x" {
        hex_str[2..].to_owned()
    } else {
        hex_str.clone()
    };
    let vec = split_n(&hex_trim.trim()[..], 2)
        .iter()
        .map(|b| u8::from_str_radix(b, 16).map_err(|_| ErrorKind::NumberParsingError.into()))
        .collect::<Result<Vec<u8>>>()?;
    Ok(vec)
}

fn split_n(s: &str, n: usize) -> Vec<&str> {
    (0..(s.len() - n + 1) / 2 + 1)
        .map(|i| &s[2 * i..2 * i + n])
        .collect()
}

