//! AES Key Wrap (WebCrypto §30.3.1 Wrap Key / §30.3.2 Unwrap Key) over
//! RFC 3394 §2.2.1 / §2.2.2 with the default Initial Value (§2.2.3.1).
//!
//! Built on the RustCrypto `aes-kw` crate (`AesKw<Aes*>`, the cipher-0.5 /
//! `aes` 0.9 ecosystem already shared with the `aes` block cipher used by the
//! AES-GCM / CBC / CTR modes in [`crate::aes`]).  Reached only through
//! [`crate::ops::wrap_key`] / [`crate::ops::unwrap_key`] (which gate the
//! name-match + `wrapKey` / `unwrapKey` usage), so the `&[u8]`-keyed primitive
//! is not a public surface.
//!
//! Unlike the AES block-cipher modes, AES-KW takes **no** IV / nonce / counter
//! input: the algorithm uses the fixed RFC 3394 default IV, so there is no
//! per-call parameter beyond the key-encryption key and the payload — which is
//! why the normalized wrap algorithm ([`crate::algorithm::NormalizedAlgorithm::AesKwWrap`])
//! is name-only.

use aes_kw::{KeyInit, KwAes128, KwAes192, KwAes256, IV_LEN};

use crate::error::AlgorithmError;

/// AES-KW wrap (WebCrypto §30.3.1): wrap `plaintext` under the key-encryption
/// key `kek` per RFC 3394 §2.2.1, returning `ciphertext` (one 64-bit semiblock
/// longer than `plaintext`).
///
/// §30.3.1 step 1: a `plaintext` whose length is not a multiple of 64 bits
/// (8 bytes) is an `OperationError` (RFC 3394 wraps whole semiblocks).  The
/// `kek` length is validated to 16/24/32 bytes upstream by
/// [`crate::ops::generate_key`] / [`crate::ops::import_key`].
pub fn wrap(kek: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    // RFC 3394 §2.2.1: wrapping n semiblocks yields n + 1, i.e. + one IV_LEN
    // block.  (`wrap_key` rejects a non-multiple-of-8 `plaintext` before it
    // touches `out`, so the buffer is correctly sized for every accepted case.)
    let mut out = vec![0u8; plaintext.len() + IV_LEN];
    match kek.len() {
        16 => KwAes128::new_from_slice(kek)
            .expect("AES-KW KEK length validated to 16 bytes by ops")
            .wrap_key(plaintext, &mut out),
        24 => KwAes192::new_from_slice(kek)
            .expect("AES-KW KEK length validated to 24 bytes by ops")
            .wrap_key(plaintext, &mut out),
        32 => KwAes256::new_from_slice(kek)
            .expect("AES-KW KEK length validated to 32 bytes by ops")
            .wrap_key(plaintext, &mut out),
        _ => unreachable!("AES-KW KEK length validated to 16/24/32 by ops"),
    }
    .map_err(|_| {
        // §30.3.1 step 1: the only failure for a valid KEK is a payload whose
        // length is not a multiple of 64 bits.
        AlgorithmError::Operation(
            "AES-KW wrap requires the data length to be a multiple of 64 bits".to_string(),
        )
    })?;
    Ok(out)
}

/// AES-KW unwrap (WebCrypto §30.3.2): unwrap `ciphertext` under `kek` per
/// RFC 3394 §2.2.2, returning the recovered `plaintext` (one semiblock shorter
/// than `ciphertext`).
///
/// §30.3.2 step 2: an invalid length (not a multiple of 64 bits, or fewer than
/// two semiblocks) or an integrity-check failure is an `OperationError`.  The
/// `kek` length is validated to 16/24/32 bytes upstream.
pub fn unwrap(kek: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    // The recovered plaintext is one semiblock shorter than the wrapped input.
    // `unwrap_key` rejects a too-short / non-multiple-of-8 `ciphertext` before
    // it touches `out`, so the buffer is correctly sized for every accepted
    // case (`saturating_sub` keeps it non-negative for the rejected ones).
    let mut out = vec![0u8; ciphertext.len().saturating_sub(IV_LEN)];
    match kek.len() {
        16 => KwAes128::new_from_slice(kek)
            .expect("AES-KW KEK length validated to 16 bytes by ops")
            .unwrap_key(ciphertext, &mut out),
        24 => KwAes192::new_from_slice(kek)
            .expect("AES-KW KEK length validated to 24 bytes by ops")
            .unwrap_key(ciphertext, &mut out),
        32 => KwAes256::new_from_slice(kek)
            .expect("AES-KW KEK length validated to 32 bytes by ops")
            .unwrap_key(ciphertext, &mut out),
        _ => unreachable!("AES-KW KEK length validated to 16/24/32 by ops"),
    }
    .map_err(|_| {
        // §30.3.2 step 2: a bad length or a failed integrity check (both
        // surface as an `aes_kw::Error`) is an OperationError.
        AlgorithmError::Operation(
            "AES-KW unwrap failed: invalid length or integrity check".to_string(),
        )
    })?;
    Ok(out)
}
