//! Algorithm normalization registry (WebCrypto §18.4 "Algorithm
//! Normalization", procedure §18.4.4 "Normalizing an algorithm").
//!
//! The VM marshals a JS `AlgorithmIdentifier` (a string, or an object
//! with `name` + op-relevant members) into a [`RawAlgorithm`]; this
//! module validates the `(op, name)` pair against the registry and the
//! required params, returning a [`NormalizedAlgorithm`]. Later PRs
//! extend the surface by adding registry rows, not by special-casing
//! call sites.

use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;

/// A WebCrypto operation (the `op` argument of §18.4.4). The full set is
/// declared now; only the PR-1 subset (`Digest`, `Sign`, `Verify`,
/// `GenerateKey`, `ImportKey`) is populated in the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Digest,
    Sign,
    Verify,
    GenerateKey,
    ImportKey,
    GetKeyLength,
    Encrypt,
    Decrypt,
    DeriveKey,
    DeriveBits,
    WrapKey,
    UnwrapKey,
}

/// A canonical recognized algorithm name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgorithmName {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Hmac,
}

impl AlgorithmName {
    /// Recognize a name ASCII case-insensitively (§18.4.4 step:
    /// case-insensitive match against registered names).
    fn recognize(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("SHA-1") {
            Some(Self::Sha1)
        } else if name.eq_ignore_ascii_case("SHA-256") {
            Some(Self::Sha256)
        } else if name.eq_ignore_ascii_case("SHA-384") {
            Some(Self::Sha384)
        } else if name.eq_ignore_ascii_case("SHA-512") {
            Some(Self::Sha512)
        } else if name.eq_ignore_ascii_case("HMAC") {
            Some(Self::Hmac)
        } else {
            None
        }
    }

    fn as_hash(self) -> Option<HashAlgorithm> {
        match self {
            Self::Sha1 => Some(HashAlgorithm::Sha1),
            Self::Sha256 => Some(HashAlgorithm::Sha256),
            Self::Sha384 => Some(HashAlgorithm::Sha384),
            Self::Sha512 => Some(HashAlgorithm::Sha512),
            Self::Hmac => None,
        }
    }
}

/// The VM-marshalled raw algorithm identifier: `name` plus the members
/// the current operation may consult (`hash` is itself a nested
/// `AlgorithmIdentifier`, `length` an `unsigned long`).
#[derive(Clone, Debug, Default)]
pub struct RawAlgorithm {
    pub name: String,
    pub hash: Option<Box<RawAlgorithm>>,
    pub length: Option<u32>,
}

impl RawAlgorithm {
    /// Construct from a bare name (the string form of an
    /// `AlgorithmIdentifier`).
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hash: None,
            length: None,
        }
    }
}

/// A validated, normalized algorithm. Variants carry exactly the params
/// the operation needs:
/// - `Digest` carries the hash to compute.
/// - `Hmac` (sign/verify) carries only the name — the hash comes from
///   the key's `[[algorithm]]`.
/// - `HmacKeyParams` (generateKey/importKey) carries the required nested
///   hash + optional length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizedAlgorithm {
    Digest(HashAlgorithm),
    Hmac,
    HmacKeyParams {
        hash: HashAlgorithm,
        length: Option<u32>,
    },
}

impl NormalizedAlgorithm {
    /// The canonical algorithm name, for the sign/verify "name member
    /// equals the key's `[[algorithm]]` name" check.
    pub fn name(self) -> AlgorithmName {
        match self {
            Self::Digest(h) => match h {
                HashAlgorithm::Sha1 => AlgorithmName::Sha1,
                HashAlgorithm::Sha256 => AlgorithmName::Sha256,
                HashAlgorithm::Sha384 => AlgorithmName::Sha384,
                HashAlgorithm::Sha512 => AlgorithmName::Sha512,
            },
            Self::Hmac | Self::HmacKeyParams { .. } => AlgorithmName::Hmac,
        }
    }
}

/// Maximum bytes echoed from an attacker-supplied algorithm name into a
/// `NotSupportedError` message (bounds the per-call allocation against a
/// `crypto.subtle.digest('A'.repeat(N), …)` attack).
const MAX_ECHOED_ALGO_NAME_LEN: usize = 64;

/// The IDL dictionary type a recognized `(op, name)` pair resolves to
/// (§18.4.4 step 5 `desiredType`), plus the bits `normalize` needs to
/// build the result. This is the registry-membership oracle: a `Some`
/// means the pair is in `supportedAlgorithms[op]` (step 5 found a key),
/// a `None` means step 5 returns `NotSupportedError` before any
/// params-dictionary member is read.
///
/// Both [`normalize`] and [`is_supported`] route through
/// [`resolve_registry`] so the two cannot drift: there is one place that
/// decides whether `(op, name)` is registered.
enum DesiredType {
    /// `digest`: name-only `Algorithm` — the name fully determines the
    /// hash to compute.
    Digest(HashAlgorithm),
    /// `sign` / `verify` HMAC: name-only `Algorithm` (the hash comes from
    /// the key's `[[algorithm]]`).
    HmacSignVerify,
    /// `generateKey` / `importKey` / `getKeyLength` HMAC: an
    /// `HmacKeyGenParams` / `HmacImportParams` whose `hash` (required) and
    /// `length` (optional) members are read by step 6.
    HmacKeyParams,
}

/// §18.4.4 step 5: does `supportedAlgorithms[op]` contain a
/// case-insensitive match for `name`, and if so, which IDL dictionary
/// type does it resolve to? `None` ⇒ the spec returns `NotSupportedError`
/// at step 5, *before* the step-6 WebIDL conversion reads any
/// params-dictionary member (`hash` / `length`).
fn resolve_registry(op: Operation, name: &str) -> Option<DesiredType> {
    let name = AlgorithmName::recognize(name)?;
    match (op, name) {
        (Operation::Digest, _) => name.as_hash().map(DesiredType::Digest),
        (Operation::Sign | Operation::Verify, AlgorithmName::Hmac) => {
            Some(DesiredType::HmacSignVerify)
        }
        (
            Operation::GenerateKey | Operation::ImportKey | Operation::GetKeyLength,
            AlgorithmName::Hmac,
        ) => Some(DesiredType::HmacKeyParams),
        _ => None,
    }
}

/// §18.4.4 step 5 as a predicate: is `(op, name)` a registered pair? The
/// VM marshalling layer calls this to decide whether to read the
/// params-dictionary getters (`hash` / `length`) at all — the spec only
/// converts `alg` to the params dictionary (step 6, which fires those
/// getters) *after* the name is recognized, so an unregistered name must
/// never trigger a user-defined `hash` / `length` getter.
pub fn is_supported(op: Operation, name: &str) -> bool {
    resolve_registry(op, name).is_some()
}

/// Normalize an algorithm for `op` (WebCrypto §18.4.4).
///
/// Returns `NotSupported` for an unregistered `(op, name)` pair, and
/// `Type` for a missing required member (e.g. HMAC `hash`).
pub fn normalize(op: Operation, raw: &RawAlgorithm) -> Result<NormalizedAlgorithm, AlgorithmError> {
    match resolve_registry(op, &raw.name) {
        None => Err(unrecognized(&raw.name)),
        Some(DesiredType::Digest(hash)) => Ok(NormalizedAlgorithm::Digest(hash)),
        Some(DesiredType::HmacSignVerify) => Ok(NormalizedAlgorithm::Hmac),
        Some(DesiredType::HmacKeyParams) => {
            let hash = normalize_hmac_hash(raw)?;
            Ok(NormalizedAlgorithm::HmacKeyParams {
                hash,
                length: raw.length,
            })
        }
    }
}

/// Normalize the nested `hash` member of an `HmacKeyGenParams` /
/// `HmacImportParams`. The member is IDL-`required`, so its absence is a
/// `TypeError` raised during normalization (NOT a `DataError` from the
/// downstream import path).
fn normalize_hmac_hash(raw: &RawAlgorithm) -> Result<HashAlgorithm, AlgorithmError> {
    let Some(hash_raw) = raw.hash.as_ref() else {
        return Err(AlgorithmError::Type(
            "Algorithm: member hash is required".to_string(),
        ));
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
