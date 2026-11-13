//! WebCrypto §20.5 "RSA key generation parameter validation".
//!
//! Generate Key step 2 of RSASSA-PKCS1-v1_5 (§20.9.3), RSA-PSS (§21.4.3) and
//! RSA-OAEP (§22.4.3) runs this algorithm before any key generation.  Without
//! it an even `publicExponent` never finds a prime `p` with
//! `gcd(e, p - 1) = 1`, so the rsa crate's prime search does not return (a
//! script-reachable hang on the VM thread).

use crate::error::AlgorithmError;

use rsa::RsaPublicKey;

use super::operation;

/// Smallest modulusLength `generate` attempts.  Below it the rsa crate's prime
/// search can fail to terminate for an odd exponent:
/// - with e <= 2^33 - 1, e has at most 9 distinct odd prime factors
///   (3*5*...*29 < 2^33 < 3*5*...*31);
/// - a b-bit prime p with p - 1 = 2q (q prime, q > 2^33) gives
///   gcd(e, p - 1) = 1 for every such e;
/// - such primes exist among the candidates when b >= 35, so the crate's loop
///   ends with probability 1;
/// - each prime is modulusLength / 2 bits, so modulusLength >= 2 * 35 = 70.
pub(crate) const MIN_RSA_GENERATE_MODULUS_BITS: u32 = 70;

/// §16: the minimal big-endian encoding (leading zero bytes stripped; empty is zero).
fn minimal_be(bytes: &[u8]) -> &[u8] {
    let skip = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    &bytes[skip..]
}

/// Generate Key's own capability bounds (step 3 "generation fails" ->
/// OperationError): modulusLength below [`MIN_RSA_GENERATE_MODULUS_BITS`], or
/// an exponent above the rsa crate's `MAX_PUB_EXPONENT`.  Compared on the
/// bytes; no `BigUint` is built before the check.
pub(crate) fn validate_rsa_generate_capability(
    modulus_length: u32,
    public_exponent: &[u8],
) -> Result<(), AlgorithmError> {
    if modulus_length < MIN_RSA_GENERATE_MODULUS_BITS {
        return Err(operation(
            "RSA modulusLength is below the supported minimum for key generation",
        ));
    }
    let e = minimal_be(public_exponent);
    // MAX_PUB_EXPONENT < 2^64, so an exponent longer than 8 bytes is above it.
    let too_big = e.len() > 8
        || e.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
            > RsaPublicKey::MAX_PUB_EXPONENT;
    if too_big {
        return Err(operation(
            "RSA publicExponent exceeds the supported maximum for key generation",
        ));
    }
    Ok(())
}

/// §20.5 steps 1-3: convert `public_exponent` to a non-negative integer (§16
/// BigInteger: unsigned big-endian, any number of leading zero bytes, the empty
/// array is zero) and fail with an OperationError if `modulus_length` is less
/// than 4, or the exponent is less than 3, is even, or is greater than or equal
/// to `2^modulus_length - 1`.
pub(crate) fn validate_rsa_keygen_params(
    modulus_length: u32,
    public_exponent: &[u8],
) -> Result<(), AlgorithmError> {
    if modulus_length < 4 {
        return Err(operation(
            "RSA modulusLength must be at least 4 (WebCrypto §20.5)",
        ));
    }
    let e = minimal_be(public_exponent);
    if e.is_empty() || (e.len() == 1 && e[0] < 3) {
        return Err(operation(
            "RSA publicExponent must be at least 3 (WebCrypto §20.5)",
        ));
    }
    if e[e.len() - 1] & 1 == 0 {
        return Err(operation(
            "RSA publicExponent must be odd (WebCrypto §20.5)",
        ));
    }
    // `e` is non-empty with a non-zero first byte.
    let lead_zeros = e[0].leading_zeros();
    let bits = 8 * e.len() as u64 - u64::from(lead_zeros);
    let ml = u64::from(modulus_length);
    // e >= 2^ml - 1 is `ml` one-bits: compare by bit length, never materializing
    // either number (ml is script-controlled up to `u32::MAX`).
    let all_ones = e[0] == 0xFF >> lead_zeros && e[1..].iter().all(|&b| b == 0xFF);
    if bits > ml || (bits == ml && all_ones) {
        return Err(operation(
            "RSA publicExponent must be less than 2^modulusLength - 1 (WebCrypto §20.5)",
        ));
    }
    Ok(())
}
