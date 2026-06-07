//! HKDF derive-bits (WebCrypto §33.4.1) over RFC 5869 §2 (HMAC-based
//! Extract-and-Expand Key Derivation Function).
//!
//! Built on the RustCrypto `hkdf` crate (`Hkdf<D>` = HKDF instantiated with
//! `Hmac<D>`, the digest-0.11 ecosystem shared with `hmac` / `sha2`).
//! Reached only through the crate-internal derive-bits seam shared by
//! [`crate::ops::derive_bits`] and [`crate::ops::derive_key`] (which enforce
//! the §33.4.1 step-1 `length` constraint), so the `&[u8]`-keyed primitive is
//! not a public surface.

use hkdf::Hkdf;

use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;

/// Derive `length_bits / 8` bytes via HKDF-`hash` (RFC 5869 §2 Extract then
/// Expand) from input keying material `ikm`, with `salt` and `info`
/// (WebCrypto §33.4.1 Derive Bits step 3: `hash` as Hash, `ikm` as IKM,
/// `salt`, `info`, and `length_bits / 8` as L).
///
/// `length_bits` is a non-null multiple of 8 (the §33.4.1 step-1 constraint
/// is enforced upstream in the shared derive-bits seam reached by both
/// [`crate::ops::derive_bits`] and [`crate::ops::derive_key`]).  A derivation
/// failure — only an output longer than RFC 5869's `255 × HashLen` cap — is
/// an `OperationError` (§33.4.1 step 4).
pub fn derive_bits(
    hash: HashAlgorithm,
    ikm: &[u8],
    salt: &[u8],
    info: &[u8],
    length_bits: u32,
) -> Result<Vec<u8>, AlgorithmError> {
    let len = (length_bits / 8) as usize;
    let mut okm = vec![0u8; len];
    // `salt` is the §33.3 required member (possibly empty); RFC 5869 §2.2
    // treats an empty salt identically to the all-zero default, so passing
    // `Some(salt)` is faithful for every salt value the VM marshals.
    let result = match hash {
        HashAlgorithm::Sha1 => Hkdf::<sha1::Sha1>::new(Some(salt), ikm).expand(info, &mut okm),
        HashAlgorithm::Sha256 => Hkdf::<sha2::Sha256>::new(Some(salt), ikm).expand(info, &mut okm),
        HashAlgorithm::Sha384 => Hkdf::<sha2::Sha384>::new(Some(salt), ikm).expand(info, &mut okm),
        HashAlgorithm::Sha512 => Hkdf::<sha2::Sha512>::new(Some(salt), ikm).expand(info, &mut okm),
    };
    result.map_err(|_| {
        AlgorithmError::Operation(
            "HKDF derived length exceeds the maximum (255 × hash output)".to_string(),
        )
    })?;
    Ok(okm)
}
