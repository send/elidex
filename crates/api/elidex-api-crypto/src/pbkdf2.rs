//! PBKDF2 derive-bits (WebCrypto §34.4.1) over RFC 8018 §5.2, with the PRF
//! the HMAC MAC-generation function of FIPS-198-1 §4 keyed by the chosen
//! hash.
//!
//! Built on the RustCrypto `pbkdf2` crate's `pbkdf2_hmac` (the digest-0.11
//! ecosystem shared with `hmac` / `sha2`).  Reached only through
//! [`crate::ops::derive_bits`] (which enforces the §34.4.1 step-1 `length`
//! constraint), so the `&[u8]`-keyed primitive is not a public surface.

use pbkdf2::pbkdf2_hmac;

use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;

/// Derive `length_bits / 8` bytes via PBKDF2-HMAC-`hash` (RFC 8018 §5.2)
/// from `password`, with `salt` and `iterations` (WebCrypto §34.4.1 Derive
/// Bits step 5: `password` as P, `salt` as S, `iterations` as c, and
/// `length_bits / 8` as dkLen).
///
/// `length_bits` is a non-null multiple of 8 (the §34.4.1 step-1 constraint
/// is enforced by the caller, [`crate::ops::derive_bits`]).  Per §34.4.1:
/// `iterations == 0` is an `OperationError` (step 2) and `length_bits == 0`
/// returns the empty byte sequence (step 3).
pub fn derive_bits(
    hash: HashAlgorithm,
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    length_bits: u32,
) -> Result<Vec<u8>, AlgorithmError> {
    // §34.4.1 step 2: a zero iteration count is an OperationError.
    if iterations == 0 {
        return Err(AlgorithmError::Operation(
            "PBKDF2 iterations must be greater than zero".to_string(),
        ));
    }
    // §34.4.1 step 3: a zero-length derivation returns the empty sequence
    // (RFC 8018 requires dkLen > 0, so this is handled before the PRF runs).
    let len = (length_bits / 8) as usize;
    if len == 0 {
        return Ok(Vec::new());
    }
    let mut dk = vec![0u8; len];
    match hash {
        HashAlgorithm::Sha1 => pbkdf2_hmac::<sha1::Sha1>(password, salt, iterations, &mut dk),
        HashAlgorithm::Sha256 => pbkdf2_hmac::<sha2::Sha256>(password, salt, iterations, &mut dk),
        HashAlgorithm::Sha384 => pbkdf2_hmac::<sha2::Sha384>(password, salt, iterations, &mut dk),
        HashAlgorithm::Sha512 => pbkdf2_hmac::<sha2::Sha512>(password, salt, iterations, &mut dk),
    }
    Ok(dk)
}
