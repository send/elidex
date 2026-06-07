//! JSON Web Key (WebCrypto §15 `JsonWebKey`, RFC 7517) — the `oct`
//! symmetric-key subset used by HMAC import/export.
//!
//! The VM marshals the live JS object into [`JsonWebKey`] fields (there
//! is no JSON parse step — `keyData` arrives as a JS object, so member
//! keys are inherently unique); this module validates the `oct` shape
//! and decodes `k` (base64url, no padding, per RFC 7515 §2).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use crate::algorithm::AesVariant;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::{CryptoKeyData, KeyUsage};

/// A JSON Web Key (the members relevant to symmetric `oct` keys).
/// `None` means the member was absent in the source object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JsonWebKey {
    pub kty: Option<String>,
    pub k: Option<String>,
    pub alg: Option<String>,
    pub use_: Option<String>,
    pub key_ops: Option<Vec<String>>,
    pub ext: Option<bool>,
}

/// Validate an `oct` JWK for HMAC import and return the decoded key
/// material (WebCrypto §31 HMAC "Import Key", `jwk` branch).
///
/// All failures map to `DataError` per the HMAC jwk-import "throw a
/// DataError" branches.
pub fn import_oct_hmac(
    jwk: &JsonWebKey,
    hash: HashAlgorithm,
    extractable: bool,
    usages: &[KeyUsage],
) -> Result<Vec<u8>, AlgorithmError> {
    // kty must be "oct".
    if jwk.kty.as_deref() != Some("oct") {
        return Err(data("JWK 'kty' member must be 'oct' for HMAC"));
    }

    // k is required and must be valid base64url (no padding). A
    // zero-length decode is rejected by the caller (`ops::import_key`)
    // per the WebCrypto §31.6.4 shared "if length is zero, throw a
    // DataError" step, so it is not special-cased here.
    let Some(k) = jwk.k.as_deref() else {
        return Err(data("JWK 'k' member is missing"));
    };
    let material = URL_SAFE_NO_PAD
        .decode(k)
        .map_err(|_| data("JWK 'k' member is not valid base64url"))?;

    // alg, if present, must match the requested hash.
    if let Some(alg) = jwk.alg.as_deref() {
        if alg != hash.jwk_hmac_alg() {
            return Err(data("JWK 'alg' member does not match the requested hash"));
        }
    }

    // use, if present, must be "sig" for a signing/verifying key — but
    // only when usages is non-empty (WebCrypto §31.6.4 step 7).  With
    // empty usages the later generic empty-secret-usages SyntaxError
    // (§14.3.9) is the correct rejection, so this DataError must not
    // pre-empt it.
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            if use_ != "sig" {
                return Err(data("JWK 'use' member must be 'sig'"));
            }
        }
    }

    // key_ops, if present, must be a valid superset of the requested
    // usages with no duplicate entries.
    if let Some(key_ops) = &jwk.key_ops {
        validate_key_ops(key_ops, usages)?;
    }

    // ext false cannot satisfy an extractable=true import.
    if let Some(false) = jwk.ext {
        if extractable {
            return Err(data(
                "JWK 'ext' member is false but an extractable key was requested",
            ));
        }
    }

    Ok(material)
}

/// Serialize an HMAC `CryptoKey` to an `oct` JWK (WebCrypto §31 HMAC
/// "Export Key", `jwk` branch).
pub fn export_oct_hmac(key: &CryptoKeyData, hash: HashAlgorithm) -> JsonWebKey {
    JsonWebKey {
        kty: Some("oct".to_string()),
        k: Some(URL_SAFE_NO_PAD.encode(key.material.as_bytes())),
        alg: Some(hash.jwk_hmac_alg().to_string()),
        use_: None,
        key_ops: Some(key.usages.iter().map(|u| u.as_str().to_string()).collect()),
        ext: Some(key.extractable),
    }
}

/// Validate an `oct` JWK for AES import and return the decoded key
/// material (WebCrypto §27.7.4 / §28.4.4 / §29.4.4 `jwk` branch — the
/// three AES modes share the step shape; only the `alg` prefix differs by
/// `variant` + key length).  All failures map to `DataError`.
pub fn import_oct_aes(
    jwk: &JsonWebKey,
    variant: AesVariant,
    extractable: bool,
    usages: &[KeyUsage],
) -> Result<Vec<u8>, AlgorithmError> {
    // jwk substep 2: kty must be "oct".
    if jwk.kty.as_deref() != Some("oct") {
        return Err(data("JWK 'kty' member must be 'oct' for AES"));
    }
    // jwk substep 4: decode the k field (base64url, no padding).
    let Some(k) = jwk.k.as_deref() else {
        return Err(data("JWK 'k' member is missing"));
    };
    let material = URL_SAFE_NO_PAD
        .decode(k)
        .map_err(|_| data("JWK 'k' member is not valid base64url"))?;

    // jwk substep 5: the key length in bits must be 128/192/256, and a
    // present `alg` must match the variant's value for that length (e.g.
    // 256-bit AES-GCM → "A256GCM"); any other length is a DataError.
    let bits = bit_len(material.len())?;
    let Some(expected_alg) = variant.jwk_alg(bits) else {
        return Err(data("JWK 'k' must decode to a 128, 192 or 256-bit AES key"));
    };
    if let Some(alg) = jwk.alg.as_deref() {
        if alg != expected_alg {
            return Err(data(
                "JWK 'alg' member does not match the AES key length / mode",
            ));
        }
    }

    // jwk substep 6: a present `use` must be "enc" (AES is an encryption
    // key) — but only when usages is non-empty, so the later generic
    // empty-secret-usages SyntaxError (§14.3.9) is not pre-empted.
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            if use_ != "enc" {
                return Err(data("JWK 'use' member must be 'enc'"));
            }
        }
    }

    // jwk substep 7: a present `key_ops` must be a valid superset of the
    // requested usages.
    if let Some(key_ops) = &jwk.key_ops {
        validate_key_ops(key_ops, usages)?;
    }

    // jwk substep 8: ext=false cannot satisfy an extractable=true import.
    if let Some(false) = jwk.ext {
        if extractable {
            return Err(data(
                "JWK 'ext' member is false but an extractable key was requested",
            ));
        }
    }

    Ok(material)
}

/// Serialize an AES `CryptoKey` to an `oct` JWK (WebCrypto §27.7.5 /
/// §28.4.5 / §29.4.5 `jwk` branch).
pub fn export_oct_aes(key: &CryptoKeyData, variant: AesVariant, length_bits: u32) -> JsonWebKey {
    JsonWebKey {
        kty: Some("oct".to_string()),
        k: Some(URL_SAFE_NO_PAD.encode(key.material.as_bytes())),
        // length is 128/192/256 for any stored AES key, so `jwk_alg` is Some.
        alg: variant.jwk_alg(length_bits).map(str::to_string),
        use_: None,
        key_ops: Some(key.usages.iter().map(|u| u.as_str().to_string()).collect()),
        ext: Some(key.extractable),
    }
}

/// The bit length of a byte sequence, or `DataError` if it would overflow
/// `u32` (defensive — AES material is always ≤ 32 bytes).
fn bit_len(byte_len: usize) -> Result<u32, AlgorithmError> {
    u32::try_from(byte_len)
        .ok()
        .and_then(|n| n.checked_mul(8))
        .ok_or_else(|| data("JWK 'k' member is too large"))
}

/// `key_ops` must be a valid JWK key-operations array containing every
/// requested usage (WebCrypto §31.6.4 step 8).
///
/// Validity is per JWK [RFC 7517 §4.3]: entries are arbitrary strings
/// ("Other values MAY be used"), but duplicate values MUST NOT be present.
/// So this checks duplicates + the requested-usage superset at the
/// **string** level — extension / private operations (e.g. a custom
/// `"derive-foo"` alongside `"sign"`) are ignored, not rejected.
fn validate_key_ops(key_ops: &[String], usages: &[KeyUsage]) -> Result<(), AlgorithmError> {
    // RFC 7517 §4.3: no duplicate key operation values.
    for (i, op) in key_ops.iter().enumerate() {
        if key_ops[i + 1..].iter().any(|other| other == op) {
            return Err(data("JWK 'key_ops' member contains a duplicate entry"));
        }
    }
    // §31.6.4 step 8: key_ops must contain all requested usages.
    for usage in usages {
        if !key_ops.iter().any(|op| op == usage.as_str()) {
            return Err(data(
                "JWK 'key_ops' member is not a superset of the requested usages",
            ));
        }
    }
    Ok(())
}

fn data(msg: &str) -> AlgorithmError {
    AlgorithmError::Data(msg.to_string())
}
