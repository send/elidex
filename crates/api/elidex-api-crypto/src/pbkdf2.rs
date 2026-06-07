//! PBKDF2 derive-bits (WebCrypto §34.4.1) over RFC 8018 §5.2, with the PRF
//! the HMAC MAC-generation function of FIPS-198-1 §4 keyed by the chosen
//! hash.
//!
//! Built on the RustCrypto `pbkdf2` crate's `pbkdf2_hmac` (the digest-0.11
//! ecosystem shared with `hmac` / `sha2`).  Reached only through the
//! crate-internal derive-bits seam shared by [`crate::ops::derive_bits`] and
//! [`crate::ops::derive_key`] (which enforce the §34.4.1 step-1 `length`
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
/// is enforced upstream in the shared derive-bits seam reached by both
/// [`crate::ops::derive_bits`] and [`crate::ops::derive_key`]).  Per §34.4.1:
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
    // No pre-allocation cap (unlike `hkdf::derive_bits`): PBKDF2 has no
    // spec-defined output-length bound — RFC 8018 allows any `dkLen` up to
    // `(2^32 − 1) × hLen`, so there is no invalid-length region to reject
    // early.  The `dk` buffer IS the caller's requested output, and a large
    // `length` (like a large `iterations`) is a caller-chosen synchronous cost
    // honored per the subtle vertical's deliberate synchronous-settle design;
    // a non-spec `dkLen` cap would be the pragmatic deviation the program
    // declined (the cross-cutting ideal is off-main-thread async-settle for the
    // whole vertical, not a per-op cap).  HKDF differs only because its
    // §33.4.1 step-4 / RFC 5869 §2.3 `255 × HashLen` ceiling makes an
    // over-length request *invalid* (OperationError), worth rejecting before
    // the wasted allocation.
    let mut dk = vec![0u8; len];
    match hash {
        HashAlgorithm::Sha1 => pbkdf2_hmac::<sha1::Sha1>(password, salt, iterations, &mut dk),
        HashAlgorithm::Sha256 => pbkdf2_hmac::<sha2::Sha256>(password, salt, iterations, &mut dk),
        HashAlgorithm::Sha384 => pbkdf2_hmac::<sha2::Sha384>(password, salt, iterations, &mut dk),
        HashAlgorithm::Sha512 => pbkdf2_hmac::<sha2::Sha512>(password, salt, iterations, &mut dk),
    }
    Ok(dk)
}
