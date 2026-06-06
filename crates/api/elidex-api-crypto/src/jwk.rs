//! JSON Web Key (WebCrypto §15 `JsonWebKey`, RFC 7517) — the `oct`
//! symmetric-key subset used by HMAC import/export.
//!
//! The VM marshals the live JS object into [`JsonWebKey`] fields (there
//! is no JSON parse step — `keyData` arrives as a JS object, so member
//! keys are inherently unique); this module validates the `oct` shape
//! and decodes `k` (base64url, no padding, per RFC 7515 §2).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

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

    // use, if present, must be "sig" for a signing/verifying key.
    if let Some(use_) = jwk.use_.as_deref() {
        if use_ != "sig" {
            return Err(data("JWK 'use' member must be 'sig'"));
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

/// Each `key_ops` entry must be a recognized usage, with no duplicates,
/// and the set must contain every requested usage (superset).
fn validate_key_ops(key_ops: &[String], usages: &[KeyUsage]) -> Result<(), AlgorithmError> {
    let mut parsed: Vec<KeyUsage> = Vec::with_capacity(key_ops.len());
    for op in key_ops {
        let Some(usage) = KeyUsage::from_ident(op) else {
            return Err(data("JWK 'key_ops' member contains an invalid usage"));
        };
        if parsed.contains(&usage) {
            return Err(data("JWK 'key_ops' member contains a duplicate entry"));
        }
        parsed.push(usage);
    }
    for usage in usages {
        if !parsed.contains(usage) {
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
