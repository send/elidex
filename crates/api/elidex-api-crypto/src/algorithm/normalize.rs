//! The §18.4.4 "normalize an algorithm" procedure: validate a
//! [`RawAlgorithm`] for an [`Operation`] against the [`super::registry`]
//! and build the [`NormalizedAlgorithm`], plus the required-member /
//! unrecognized-name error helpers.

use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;

use super::model::{NormalizedAlgorithm, RawAlgorithm};
use super::names::{AesVariant, AlgorithmName, NamedCurve};
use super::registry::{resolve_registry, DesiredType, KdfKind};
use super::Operation;

/// Maximum bytes echoed from an attacker-supplied algorithm name into a
/// `NotSupportedError` message (bounds the per-call allocation against a
/// `crypto.subtle.digest('A'.repeat(N), …)` attack).
const MAX_ECHOED_ALGO_NAME_LEN: usize = 64;

/// Normalize an algorithm for `op` (WebCrypto §18.4.4).
///
/// Returns `NotSupported` for an unregistered `(op, name)` pair, and
/// `Type` for a missing required member (e.g. HMAC `hash`, AES key-gen
/// `length`, AES `iv` / `counter`).  Per-mode *operational* validation
/// (iv / counter byte length, `tagLength` validity, key length 128/192/256)
/// lives in the crate-internal `aes` module + [`crate::ops`] at the op step
/// where the spec throws `OperationError`, not here.
/// Takes the freshly-marshalled `RawAlgorithm` **by value** so the AES
/// `iv` / `counter` / `additionalData` byte buffers move straight into the
/// [`NormalizedAlgorithm`] (and thence to the cipher) without a second copy
/// beyond the VM's marshal-time snapshot.
pub fn normalize(op: Operation, raw: RawAlgorithm) -> Result<NormalizedAlgorithm, AlgorithmError> {
    match resolve_registry(op, &raw.name) {
        None => Err(unrecognized(&raw.name)),
        Some(DesiredType::Digest(hash)) => Ok(NormalizedAlgorithm::Digest(hash)),
        Some(DesiredType::HmacSignVerify) => Ok(NormalizedAlgorithm::Hmac),
        Some(DesiredType::HmacKeyParams) => {
            let hash = normalize_required_hash(&raw, "Algorithm")?;
            Ok(NormalizedAlgorithm::HmacKeyParams {
                hash,
                length: raw.length,
            })
        }
        Some(DesiredType::AesImport(variant)) => Ok(NormalizedAlgorithm::AesImport { variant }),
        Some(DesiredType::AesKeyGen(variant)) => {
            // `AesKeyGenParams.length` / `AesDerivedKeyParams.length` is a
            // `required` member: its absence is a WebIDL `TypeError` (the VM
            // also enforces this at marshal time; this is the crate-side spec
            // guard).  Its 128/192/256 validity is an `OperationError` checked
            // in `ops::generate_key` / `ops::get_key_length`.
            let length = raw
                .length
                .ok_or_else(|| required_member("length", "AesKeyGenParams"))?;
            Ok(NormalizedAlgorithm::AesKeyGen { variant, length })
        }
        Some(DesiredType::AesEncryptDecrypt(variant)) => normalize_aes_params(variant, raw),
        // AES-KW wrapKey / unwrapKey: name-only (§30.3.1 / §30.3.2 default IV).
        Some(DesiredType::AesKwWrap) => Ok(NormalizedAlgorithm::AesKwWrap),
        Some(DesiredType::KdfNameOnly(KdfKind::Hkdf)) => Ok(NormalizedAlgorithm::Hkdf),
        Some(DesiredType::KdfNameOnly(KdfKind::Pbkdf2)) => Ok(NormalizedAlgorithm::Pbkdf2),
        Some(DesiredType::HkdfDeriveBits) => {
            // `HkdfParams` — `hash` / `salt` / `info` all `required` (their
            // absence is a `TypeError`, enforced at the VM marshal too).
            let hash = normalize_required_hash(&raw, "HkdfParams")?;
            let salt = raw
                .salt
                .ok_or_else(|| required_member("salt", "HkdfParams"))?;
            let info = raw
                .info
                .ok_or_else(|| required_member("info", "HkdfParams"))?;
            Ok(NormalizedAlgorithm::HkdfParams { hash, salt, info })
        }
        Some(DesiredType::Pbkdf2DeriveBits) => {
            // `Pbkdf2Params` — `hash` / `iterations` / `salt` all `required`.
            let hash = normalize_required_hash(&raw, "Pbkdf2Params")?;
            let salt = raw
                .salt
                .ok_or_else(|| required_member("salt", "Pbkdf2Params"))?;
            let iterations = raw
                .iterations
                .ok_or_else(|| required_member("iterations", "Pbkdf2Params"))?;
            Ok(NormalizedAlgorithm::Pbkdf2Params {
                salt,
                iterations,
                hash,
            })
        }
        Some(DesiredType::EcKeyGen(algorithm)) => {
            let curve = normalize_required_curve(&raw, "EcKeyGenParams")?;
            Ok(NormalizedAlgorithm::EcKeyGen { algorithm, curve })
        }
        Some(DesiredType::EcImport(algorithm)) => {
            let curve = normalize_required_curve(&raw, "EcKeyImportParams")?;
            Ok(NormalizedAlgorithm::EcImport { algorithm, curve })
        }
        Some(DesiredType::EcdsaParams) => {
            let hash = normalize_required_hash(&raw, "EcdsaParams")?;
            Ok(NormalizedAlgorithm::EcdsaParams { hash })
        }
        Some(DesiredType::EcdhDerive) => {
            // §24.3 `public` is a required CryptoKey member; the VM brand-checks
            // it (a non-CryptoKey → TypeError at marshal) and conveys its
            // metadata + SEC1 point as the `peer`.  Its absence is the
            // required-member TypeError (the VM enforces this too).
            let peer = raw
                .peer
                .ok_or_else(|| required_member("public", "EcdhKeyDeriveParams"))?;
            Ok(NormalizedAlgorithm::EcdhDerive { peer })
        }
        Some(DesiredType::RsaKeyGen(variant)) => {
            // `RsaHashedKeyGenParams` — `modulusLength` / `publicExponent`
            // (inherited from `RsaKeyGenParams`) then `hash` (the derived
            // member) are all `required` (absence is a `TypeError`).  Validate
            // presence in that Web IDL inherited-first order so a malformed
            // `RawAlgorithm` reports the same missing member as the spec + the
            // VM marshaller (which fires getters in that order); the owned
            // `publicExponent` is *extracted* last to keep the `&raw` hash read
            // borrow-legal, but the precedence-defining checks run in order.
            // modulusLength validity + the exponent value are the rsa-crate's
            // OperationError at generate (§20.8.3 step 3), honored as-is.
            let modulus_length = raw
                .modulus_length
                .ok_or_else(|| required_member("modulusLength", "RsaHashedKeyGenParams"))?;
            if raw.public_exponent.is_none() {
                return Err(required_member("publicExponent", "RsaHashedKeyGenParams"));
            }
            let hash = normalize_required_hash(&raw, "RsaHashedKeyGenParams")?;
            let public_exponent = raw
                .public_exponent
                .expect("publicExponent presence checked above");
            Ok(NormalizedAlgorithm::RsaKeyGen {
                variant,
                modulus_length,
                public_exponent,
                hash,
            })
        }
        Some(DesiredType::RsaImport(variant)) => {
            let hash = normalize_required_hash(&raw, "RsaHashedImportParams")?;
            Ok(NormalizedAlgorithm::RsaImport { variant, hash })
        }
        Some(DesiredType::RsassaParams) => Ok(NormalizedAlgorithm::RsassaParams),
        Some(DesiredType::RsaPssParams) => {
            // `RsaPssParams` — `saltLength` (required `[EnforceRange] unsigned
            // long`; its absence is a `TypeError`, enforced at the VM marshal
            // too).
            let salt_length = raw
                .salt_length
                .ok_or_else(|| required_member("saltLength", "RsaPssParams"))?;
            Ok(NormalizedAlgorithm::RsaPssParams { salt_length })
        }
        Some(DesiredType::RsaOaepParams) => {
            // `RsaOaepParams` — `label` is OPTIONAL (§22.3), so there is no
            // required-member gate; it moves by-value into the normalized
            // algorithm.  The `[[type]]` (public for encrypt/wrapKey, private
            // for decrypt/unwrapKey) gate lives in the op layer (the `rsa`
            // backend), so all four entry points inherit it.
            Ok(NormalizedAlgorithm::RsaOaep { label: raw.label })
        }
    }
}

/// Recognize the required `namedCurve` member of an EC params dictionary
/// (`EcKeyGenParams` §23.4 / `EcKeyImportParams` §23.6).  Its absence is a
/// `TypeError` (IDL-`required`, enforced at the VM marshal too); an
/// unrecognized curve is a `NotSupportedError` (the §23.7.3 / §24.4.1 /
/// §23.7.4 "Otherwise: throw a NotSupportedError" curve step — `NamedCurve`
/// is a typedef, NOT a WebIDL `enum`, so it is prose-validated here, not at
/// the WebIDL conversion).
fn normalize_required_curve(raw: &RawAlgorithm, dict: &str) -> Result<NamedCurve, AlgorithmError> {
    let Some(name) = raw.named_curve.as_deref() else {
        return Err(required_member("namedCurve", dict));
    };
    NamedCurve::from_name(name).ok_or_else(|| {
        AlgorithmError::NotSupported(format!(
            "Unrecognized named curve: '{}'",
            truncate_at_char_boundary(name, MAX_ECHOED_ALGO_NAME_LEN)
        ))
    })
}

/// Structure the per-mode AES encrypt/decrypt params from the marshalled
/// `RawAlgorithm` (WebCrypto §27.3 / §28.3 / §29.3 dictionaries), moving the
/// byte buffers out of `raw`.  Required `BufferSource` members (`iv` /
/// `counter`) and the required AES-CTR `length` are `TypeError` if absent
/// (the VM enforces this too); byte-length / value validity is deferred to
/// the op (`OperationError`).
fn normalize_aes_params(
    variant: AesVariant,
    raw: RawAlgorithm,
) -> Result<NormalizedAlgorithm, AlgorithmError> {
    match variant {
        AesVariant::Gcm => {
            let iv = raw
                .iv
                .ok_or_else(|| required_member("iv", "AesGcmParams"))?;
            Ok(NormalizedAlgorithm::AesGcm {
                iv,
                additional_data: raw.additional_data,
                // §29.4.1/.2 step "tagLength not present → 128"; a *present*
                // out-of-set value is an `OperationError` in `aes`.
                tag_length: raw.tag_length.unwrap_or(128),
            })
        }
        AesVariant::Cbc => {
            let iv = raw
                .iv
                .ok_or_else(|| required_member("iv", "AesCbcParams"))?;
            Ok(NormalizedAlgorithm::AesCbc { iv })
        }
        AesVariant::Ctr => {
            let counter = raw
                .counter
                .ok_or_else(|| required_member("counter", "AesCtrParams"))?;
            let length = raw
                .length
                .ok_or_else(|| required_member("length", "AesCtrParams"))?;
            Ok(NormalizedAlgorithm::AesCtr { counter, length })
        }
        // AES-KW never reaches here: it normalizes via `DesiredType::AesKwWrap`
        // (name-only), not `AesEncryptDecrypt`.
        AesVariant::Kw => unreachable!("AES-KW has no encrypt/decrypt params dictionary"),
    }
}

fn required_member(member: &str, dict: &str) -> AlgorithmError {
    AlgorithmError::Type(format!("{dict}: member {member} is required"))
}

/// Normalize the nested required `hash` member of a params dictionary that
/// carries one (`HmacKeyGenParams` / `HmacImportParams` §31, `HkdfParams`
/// §33.3, `Pbkdf2Params` §34.3). The member is IDL-`required`, so its
/// absence is a `TypeError` raised during normalization (NOT a `DataError`
/// from a downstream path); an unrecognized hash name is a
/// `NotSupportedError`. `dict` names the dictionary for the error message.
fn normalize_required_hash(
    raw: &RawAlgorithm,
    dict: &str,
) -> Result<HashAlgorithm, AlgorithmError> {
    let Some(hash_raw) = raw.hash.as_ref() else {
        return Err(required_member("hash", dict));
    };
    match AlgorithmName::recognize(&hash_raw.name).and_then(AlgorithmName::as_hash) {
        Some(hash) => Ok(hash),
        None => Err(unrecognized(&hash_raw.name)),
    }
}

fn unrecognized(name: &str) -> AlgorithmError {
    AlgorithmError::NotSupported(format!(
        "Unrecognized algorithm name: '{}'",
        truncate_at_char_boundary(name, MAX_ECHOED_ALGO_NAME_LEN)
    ))
}

fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
