//! Unit tests for the engine-independent WebCrypto algorithms.
//!
//! HMAC vectors are from RFC 4231 (SHA-256/384/512) and RFC 2202
//! (SHA-1); digest vectors from FIPS 180 examples.

mod aes;
mod aes_kw;
mod derive;
mod digest;
mod ec;
mod hmac;
mod hmac_ops;
mod normalize;
mod rsa;

/// Deterministic key material for the AES / AES-KW ops tests, matching the
/// `fill_random` closure contract of [`crate::ops::generate_key`].  The `Result`
/// shape is required by that contract and the truncating index cast is sound
/// (a generated key is at most 32 bytes, so the index never exceeds a `u8`).
#[allow(clippy::unnecessary_wraps, clippy::cast_possible_truncation)]
pub(super) fn fill_seq(buf: &mut [u8]) -> Result<(), crate::error::AlgorithmError> {
    for (i, b) in buf.iter_mut().enumerate() {
        *b = i as u8;
    }
    Ok(())
}

/// A `fill_random` closure for [`crate::ops::sign`] on the deterministic
/// algorithms (HMAC / ECDSA / RSASSA-PKCS1-v1_5) — they draw no entropy, so
/// this is never invoked (only RSA-PSS consumes the seam).
#[allow(clippy::unnecessary_wraps)]
pub(super) fn no_rng(_buf: &mut [u8]) -> Result<(), crate::error::AlgorithmError> {
    Ok(())
}

/// Unwrap a symmetric (HMAC / AES) `generateKey` result, which is always a
/// [`crate::ops::GeneratedKey::Single`] (only EC keygen yields a `Pair`).
pub(super) fn expect_single(
    result: Result<crate::ops::GeneratedKey, crate::error::AlgorithmError>,
) -> crate::key::CryptoKeyData {
    match result.expect("symmetric generateKey succeeds") {
        crate::ops::GeneratedKey::Single(key) => key,
        crate::ops::GeneratedKey::Pair { .. } => {
            panic!("symmetric generateKey yields a single key, not a pair")
        }
    }
}

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
