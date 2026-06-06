//! Operation-level entry points (WebCrypto §14.3.x) — the layering
//! boundary. Every spec-validation step (usages subset / empty usages /
//! extractable gate / length range / JWK shape / algorithm-name match)
//! lives here; the VM host only marshals JS ↔ these plain-Rust inputs
//! and settles the returned Promise.

use crate::algorithm::NormalizedAlgorithm;
use crate::error::AlgorithmError;
use crate::hmac;
use crate::jwk::{self, JsonWebKey};
use crate::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};

/// The `KeyFormat` enum (WebCrypto §14.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyFormat {
    Raw,
    Pkcs8,
    Spki,
    Jwk,
}

/// `importKey` key material, already marshalled from JS by the VM:
/// `Raw` for the `raw` format (BufferSource bytes), `Jwk` for the `jwk`
/// format (the live JS object's members).
#[derive(Clone, Debug)]
pub enum KeyData {
    Raw(Vec<u8>),
    Jwk(JsonWebKey),
}

/// `exportKey` result — plain-Rust shapes the VM turns into an
/// `ArrayBuffer` or a JS object.
#[derive(Clone, Debug)]
pub enum ExportedKey {
    Raw(Vec<u8>),
    Jwk(JsonWebKey),
}

/// `generateKey` for HMAC (WebCrypto §14.3.6 + §31 Generate Key).
///
/// `rng_bytes` must be exactly [`hmac::generate_key_byte_len`] bytes,
/// filled by the VM from the OS CSPRNG. The bytes are stored verbatim;
/// `length` is recorded as metadata (no masking).
pub fn generate_key(
    algorithm: NormalizedAlgorithm,
    extractable: bool,
    usages: Vec<KeyUsage>,
    rng_bytes: &[u8],
) -> Result<CryptoKeyData, AlgorithmError> {
    let NormalizedAlgorithm::HmacKeyParams { hash, length } = algorithm else {
        return Err(not_supported_op("generateKey"));
    };
    validate_hmac_usages(&usages)?;
    // `ops` owns argument validation (the layering boundary): re-derive
    // the required byte count (this also runs the `length == 0` →
    // OperationError check) and defensively confirm the caller filled
    // exactly that many CSPRNG bytes, so a mis-sized `rng_bytes` cannot
    // silently produce a wrong-length key.
    let expected = hmac::generate_key_byte_len(hash, length)?;
    if rng_bytes.len() != expected {
        return Err(AlgorithmError::Operation(format!(
            "HMAC key generation requires {expected} random byte(s), got {}",
            rng_bytes.len()
        )));
    }
    let bit_len = hmac::generate_key_bit_len(hash, length);
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm: KeyAlgorithm::Hmac {
            hash,
            length: bit_len,
        },
        usages,
        material: KeyMaterial::Raw(rng_bytes.to_vec()),
    })
}

/// `importKey` for HMAC (WebCrypto §14.3.9 + §31 Import Key).
pub fn import_key(
    format: KeyFormat,
    algorithm: NormalizedAlgorithm,
    extractable: bool,
    usages: Vec<KeyUsage>,
    key_data: KeyData,
) -> Result<CryptoKeyData, AlgorithmError> {
    let NormalizedAlgorithm::HmacKeyParams { hash, length } = algorithm else {
        return Err(not_supported_op("importKey"));
    };
    validate_hmac_usages(&usages)?;

    let material = match (format, key_data) {
        (KeyFormat::Raw, KeyData::Raw(bytes)) => bytes,
        (KeyFormat::Jwk, KeyData::Jwk(jwk)) => {
            jwk::import_oct_hmac(&jwk, hash, extractable, &usages)?
        }
        (KeyFormat::Pkcs8 | KeyFormat::Spki, _) => {
            return Err(AlgorithmError::NotSupported(
                "HMAC import supports only the 'raw' and 'jwk' formats".to_string(),
            ));
        }
        // Format / data shape mismatch — the VM marshals them
        // consistently, so this is a defensive guard.
        _ => {
            return Err(AlgorithmError::Type(
                "keyData does not match the requested format".to_string(),
            ));
        }
    };

    // WebCrypto §31.6.4 HMAC Import Key (shared raw + jwk step): "Let
    // length be the length in bits of data. If length is zero then throw
    // a DataError." This fires regardless of the `length` member.
    if material.is_empty() {
        return Err(AlgorithmError::Data(
            "HMAC key material must not be empty".to_string(),
        ));
    }
    let length = resolve_import_length(material.len(), length)?;
    Ok(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable,
        algorithm: KeyAlgorithm::Hmac { hash, length },
        usages,
        material: KeyMaterial::Raw(material),
    })
}

/// `exportKey` (WebCrypto §14.3.10 + §31 Export Key). `extractable=false`
/// gates every format with `InvalidAccessError`.
pub fn export_key(format: KeyFormat, key: &CryptoKeyData) -> Result<ExportedKey, AlgorithmError> {
    if !key.extractable {
        return Err(AlgorithmError::InvalidAccess(
            "key is not extractable".to_string(),
        ));
    }
    match format {
        KeyFormat::Raw => Ok(ExportedKey::Raw(key.material.as_bytes().to_vec())),
        KeyFormat::Jwk => Ok(ExportedKey::Jwk(jwk::export_oct_hmac(
            key,
            key.algorithm.hash(),
        ))),
        KeyFormat::Pkcs8 | KeyFormat::Spki => Err(AlgorithmError::NotSupported(
            "HMAC export supports only the 'raw' and 'jwk' formats".to_string(),
        )),
    }
}

/// `sign` (WebCrypto §14.3.3 + §31 Sign).
pub fn sign(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    require_usage(key, KeyUsage::Sign)?;
    require_name_match(algorithm, key)?;
    Ok(hmac::sign(
        key.algorithm.hash(),
        key.material.as_bytes(),
        data,
    ))
}

/// `verify` (WebCrypto §14.3.4 + §31 Verify).
pub fn verify(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
    signature: &[u8],
    data: &[u8],
) -> Result<bool, AlgorithmError> {
    require_usage(key, KeyUsage::Verify)?;
    require_name_match(algorithm, key)?;
    Ok(hmac::verify(
        key.algorithm.hash(),
        key.material.as_bytes(),
        signature,
        data,
    ))
}

/// HMAC accepts only the `sign` / `verify` usages and rejects an empty
/// usages set (WebCrypto §31 Generate/Import Key, `SyntaxError`).
fn validate_hmac_usages(usages: &[KeyUsage]) -> Result<(), AlgorithmError> {
    if usages.is_empty() {
        return Err(AlgorithmError::Syntax("usages cannot be empty".to_string()));
    }
    for usage in usages {
        if !matches!(usage, KeyUsage::Sign | KeyUsage::Verify) {
            return Err(AlgorithmError::Syntax(
                "HMAC keys support only the 'sign' and 'verify' usages".to_string(),
            ));
        }
    }
    Ok(())
}

/// Resolve + range-check the HMAC import `length` member against the
/// `data` octet length (WebCrypto §31 Import Key): accept
/// `8·len − 8 < length ≤ 8·len`, else `DataError`. `length` is metadata
/// only — the full `material` is the key.
fn resolve_import_length(material_len: usize, length: Option<u32>) -> Result<u32, AlgorithmError> {
    // Callers reject empty material first (§31.6.4 zero-length DataError),
    // so `material_len >= 1` and `data_bits >= 8` here.
    let data_bits = u32::try_from(material_len)
        .ok()
        .and_then(|n| n.checked_mul(8))
        .ok_or_else(|| AlgorithmError::Data("HMAC key material is too large".to_string()))?;
    match length {
        None => Ok(data_bits),
        Some(l) => {
            if l > data_bits || l <= data_bits - 8 {
                return Err(AlgorithmError::Data(
                    "HMAC import 'length' is out of range for the supplied key material"
                        .to_string(),
                ));
            }
            Ok(l)
        }
    }
}

fn require_usage(key: &CryptoKeyData, usage: KeyUsage) -> Result<(), AlgorithmError> {
    if key.has_usage(usage) {
        Ok(())
    } else {
        Err(AlgorithmError::InvalidAccess(format!(
            "key does not support the '{}' operation",
            usage.as_str()
        )))
    }
}

fn require_name_match(
    algorithm: NormalizedAlgorithm,
    key: &CryptoKeyData,
) -> Result<(), AlgorithmError> {
    if algorithm.name() == key.algorithm.name() {
        Ok(())
    } else {
        Err(AlgorithmError::InvalidAccess(
            "algorithm does not match the key's algorithm".to_string(),
        ))
    }
}

fn not_supported_op(op: &str) -> AlgorithmError {
    AlgorithmError::NotSupported(format!("algorithm is not supported for {op}"))
}
