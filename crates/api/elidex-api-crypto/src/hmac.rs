//! HMAC sign / verify + key-length resolution (WebCrypto §31 HMAC).
//!
//! Built on the RustCrypto `hmac` crate (`Hmac<D>`, digest-0.11
//! ecosystem) — `verify_slice` is constant-time, so equality checks do
//! not leak via timing.

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};

use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;

type HmacSha1 = Hmac<sha1::Sha1>;
type HmacSha256 = Hmac<sha2::Sha256>;
type HmacSha384 = Hmac<sha2::Sha384>;
type HmacSha512 = Hmac<sha2::Sha512>;

/// Compute `HMAC-hash(key, data)` (WebCrypto §31 HMAC Sign).
pub fn sign(hash: HashAlgorithm, key: &[u8], data: &[u8]) -> Vec<u8> {
    match hash {
        HashAlgorithm::Sha1 => mac_sign::<HmacSha1>(key, data),
        HashAlgorithm::Sha256 => mac_sign::<HmacSha256>(key, data),
        HashAlgorithm::Sha384 => mac_sign::<HmacSha384>(key, data),
        HashAlgorithm::Sha512 => mac_sign::<HmacSha512>(key, data),
    }
}

/// Constant-time verify of `signature` against `HMAC-hash(key, data)`
/// (WebCrypto §31 HMAC Verify).
pub fn verify(hash: HashAlgorithm, key: &[u8], signature: &[u8], data: &[u8]) -> bool {
    match hash {
        HashAlgorithm::Sha1 => mac_verify::<HmacSha1>(key, data, signature),
        HashAlgorithm::Sha256 => mac_verify::<HmacSha256>(key, data, signature),
        HashAlgorithm::Sha384 => mac_verify::<HmacSha384>(key, data, signature),
        HashAlgorithm::Sha512 => mac_verify::<HmacSha512>(key, data, signature),
    }
}

fn mac_sign<M: Mac + KeyInit>(key: &[u8], data: &[u8]) -> Vec<u8> {
    // HMAC accepts a key of any length (hashed if longer than the block
    // size, zero-padded if shorter), so `new_from_slice` never fails.
    let mut mac = <M as KeyInit>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn mac_verify<M: Mac + KeyInit>(key: &[u8], data: &[u8], signature: &[u8]) -> bool {
    let mut mac = <M as KeyInit>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    // `verify_slice` is constant-time and length-checks the tag.
    mac.verify_slice(signature).is_ok()
}

/// Resolve the byte count to generate for an HMAC `generateKey`
/// (WebCrypto §31 HMAC Generate Key): the `length` member if present
/// (non-zero), else the hash block size. A zero `length` is an
/// `OperationError`.
pub fn generate_key_byte_len(
    hash: HashAlgorithm,
    length: Option<u32>,
) -> Result<usize, AlgorithmError> {
    let bits = match length {
        None => hash.block_size_bits(),
        Some(0) => {
            return Err(AlgorithmError::Operation(
                "HMAC key length must be greater than zero".to_string(),
            ));
        }
        Some(l) => l,
    };
    // ceil(bits / 8). `bits` is a u32, so this never overflows usize on
    // any supported target.
    Ok((bits as usize).div_ceil(8))
}

/// The effective bit length recorded on a generated HMAC key: the
/// `length` member, or the hash block size if absent. (Zero is rejected
/// by [`generate_key_byte_len`] before this is reached.)
pub fn generate_key_bit_len(hash: HashAlgorithm, length: Option<u32>) -> u32 {
    length.unwrap_or_else(|| hash.block_size_bits())
}
