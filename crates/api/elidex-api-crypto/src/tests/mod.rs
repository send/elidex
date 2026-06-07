//! Unit tests for the engine-independent WebCrypto algorithms.
//!
//! HMAC vectors are from RFC 4231 (SHA-256/384/512) and RFC 2202
//! (SHA-1); digest vectors from FIPS 180 examples.

mod aes;
mod derive;
mod digest;
mod hmac;
mod hmac_ops;
mod normalize;

pub(super) fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

pub(super) fn from_hex(s: &str) -> Vec<u8> {
    assert!(
        s.len().is_multiple_of(2),
        "hex string must have even length"
    );
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex"))
        .collect()
}
