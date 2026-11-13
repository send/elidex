//! WebCrypto §20.5 "RSA key generation parameter validation".
//!
//! Generate Key step 2 of RSASSA-PKCS1-v1_5 (§20.9.3), RSA-PSS (§21.4.3) and
//! RSA-OAEP (§22.4.3) runs this algorithm before any key generation.  Without
//! it an even `publicExponent` never finds a prime `p` with
//! `gcd(e, p - 1) = 1`, so the rsa crate's prime search does not return (a
//! script-reachable hang on the VM thread).

use crate::error::AlgorithmError;

use super::operation;

/// §20.5 steps 1-3: convert `public_exponent` to a non-negative integer (§16
/// BigInteger: unsigned big-endian, any number of leading zero bytes, the empty
/// array is zero) and fail with an OperationError if `modulus_length` is less
/// than 4, or the exponent is less than 3, is even, or is greater than or equal
/// to `2^modulus_length - 1`.
///
/// The comparison with `2^modulus_length - 1` is by bit length: that number is
/// exactly `modulus_length` one-bits, so neither it nor the exponent is ever
/// materialized (`modulus_length` is script-controlled up to `u32::MAX`).  The
/// cost is O(input length) with no allocation.
pub(crate) fn validate_rsa_keygen_params(
    modulus_length: u32,
    public_exponent: &[u8],
) -> Result<(), AlgorithmError> {
    if modulus_length < 4 {
        return Err(operation(
            "RSA modulusLength must be at least 4 (WebCrypto §20.5)",
        ));
    }
    // §16: the minimal big-endian encoding; empty means zero.
    let skip = public_exponent
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(public_exponent.len());
    let e = &public_exponent[skip..];
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
    // e >= 2^ml - 1 (ml one-bits): more bits, or exactly ml bits all set.
    let all_ones = e[0] == 0xFF >> lead_zeros && e[1..].iter().all(|&b| b == 0xFF);
    if bits > ml || (bits == ml && all_ones) {
        return Err(operation(
            "RSA publicExponent must be less than 2^modulusLength - 1 (WebCrypto §20.5)",
        ));
    }
    Ok(())
}
