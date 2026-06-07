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

/// The AES-KW semiblock size in bytes (64 bits, [`IV_LEN`]).  The minimum wrap
/// input is two semiblocks (NIST SP 800-38F §5.3.1: AES-KW wraps n ≥ 2
/// semiblocks); the minimum unwrap input is the IV semiblock plus two, i.e.
/// three.
const MIN_WRAP_BYTES: usize = 2 * IV_LEN; // 16
const MIN_UNWRAP_BYTES: usize = 3 * IV_LEN; // 24

/// AES-KW wrap (WebCrypto §30.3.1): wrap `plaintext` under the key-encryption
/// key `kek` per RFC 3394 §2.2.1, returning `ciphertext` (one 64-bit semiblock
/// longer than `plaintext`).
///
/// §30.3.1 step 1: a `plaintext` that is not a multiple of 64 bits (8 bytes) is
/// an `OperationError`.  Additionally NIST SP 800-38F §5.3.1 requires at least
/// **two** semiblocks (≥ 16 bytes) — the `aes-kw` crate would otherwise accept a
/// single 8-byte semiblock (e.g. an exported 64-bit HMAC key) and emit a
/// nonstandard 16-byte wrap that browsers reject, so the minimum is guarded
/// here.  The `kek` length is validated to 16/24/32 bytes upstream by
/// [`crate::ops::generate_key`] / [`crate::ops::import_key`].
pub fn wrap(kek: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    // §30.3.1 step 1 + NIST SP 800-38F §5.3.1: multiple of 64 bits AND ≥ 2
    // semiblocks (128 bits).
    if plaintext.len() < MIN_WRAP_BYTES || !plaintext.len().is_multiple_of(IV_LEN) {
        return Err(AlgorithmError::Operation(
            "AES-KW wrap requires the data length to be a multiple of 64 bits and at least 128 bits"
                .to_string(),
        ));
    }
    // RFC 3394 §2.2.1: wrapping n semiblocks yields n + 1, i.e. + one IV_LEN
    // block.  The length is already validated above, so `wrap_key` cannot fail.
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
    // The length is pre-validated, so this is a defensive guard only.
    .map_err(|_| AlgorithmError::Operation("AES-KW wrap failed".to_string()))?;
    Ok(out)
}

/// AES-KW unwrap (WebCrypto §30.3.2): unwrap `ciphertext` under `kek` per
/// RFC 3394 §2.2.2, returning the recovered `plaintext` (one semiblock shorter
/// than `ciphertext`).
///
/// §30.3.2 step 2: an invalid length or an integrity-check failure is an
/// `OperationError`.  RFC 3394 / NIST SP 800-38F: the wrapped ciphertext is the
/// IV semiblock plus n ≥ 2 plaintext semiblocks, so it must be a multiple of 64
/// bits AND at least 24 bytes (three semiblocks); the `aes-kw` crate would
/// otherwise accept an 8-byte input (decrypting to empty) or its own
/// nonstandard 16-byte wrap, so the minimum is guarded here.  The `kek` length
/// is validated to 16/24/32 bytes upstream.
pub fn unwrap(kek: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    // §30.3.2 + NIST SP 800-38F: multiple of 64 bits AND ≥ 3 semiblocks
    // (IV + ≥ 2 plaintext semiblocks).
    if ciphertext.len() < MIN_UNWRAP_BYTES || !ciphertext.len().is_multiple_of(IV_LEN) {
        return Err(AlgorithmError::Operation(
            "AES-KW unwrap requires the data length to be a multiple of 64 bits and at least \
             192 bits"
                .to_string(),
        ));
    }
    // The recovered plaintext is one semiblock shorter than the (validated)
    // wrapped input.
    let mut out = vec![0u8; ciphertext.len() - IV_LEN];
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
        // §30.3.2 step 2: the length is pre-validated, so a valid KEK fails only
        // the RFC 3394 integrity check → OperationError.
        AlgorithmError::Operation("AES-KW unwrap failed: integrity check".to_string())
    })?;
    Ok(out)
}
