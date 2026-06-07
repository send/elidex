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
    AesCtr,
    AesCbc,
    AesGcm,
    /// HKDF (WebCrypto §33) — `importKey` (raw), `deriveBits`, and
    /// `get key length` (§33.4.3 → null, consumed by `deriveKey`).
    Hkdf,
    /// PBKDF2 (WebCrypto §34) — `importKey` (raw), `deriveBits`, and
    /// `get key length` (§34.4.3 → null, consumed by `deriveKey`).
    Pbkdf2,
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
        } else if name.eq_ignore_ascii_case("AES-CTR") {
            Some(Self::AesCtr)
        } else if name.eq_ignore_ascii_case("AES-CBC") {
            Some(Self::AesCbc)
        } else if name.eq_ignore_ascii_case("AES-GCM") {
            Some(Self::AesGcm)
        } else if name.eq_ignore_ascii_case("HKDF") {
            Some(Self::Hkdf)
        } else if name.eq_ignore_ascii_case("PBKDF2") {
            Some(Self::Pbkdf2)
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
            Self::Hmac | Self::AesCtr | Self::AesCbc | Self::AesGcm | Self::Hkdf | Self::Pbkdf2 => {
                None
            }
        }
    }

    fn as_aes(self) -> Option<AesVariant> {
        match self {
            Self::AesCtr => Some(AesVariant::Ctr),
            Self::AesCbc => Some(AesVariant::Cbc),
            Self::AesGcm => Some(AesVariant::Gcm),
            Self::Sha1
            | Self::Sha256
            | Self::Sha384
            | Self::Sha512
            | Self::Hmac
            | Self::Hkdf
            | Self::Pbkdf2 => None,
        }
    }
}

/// The three AES block-cipher modes that support `encrypt` / `decrypt`
/// (WebCrypto §27 AES-CTR / §28 AES-CBC / §29 AES-GCM).  The discriminator
/// is shared by the normalized generate/import forms and the key's
/// [`KeyAlgorithm`][crate::key::KeyAlgorithm], so dispatch stays typed
/// rather than stringly.  (AES-KW, §30, supports only `wrapKey` /
/// `unwrapKey` and lands with the `#11-crypto-subtle-full` PR-3 wrap
/// surface — it is not a variant
/// here.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AesVariant {
    Ctr,
    Cbc,
    Gcm,
}

impl AesVariant {
    /// The canonical WebCrypto algorithm name (`"AES-GCM"` etc.) for the
    /// key's `[[algorithm]]` `name` attribute and the JWK `alg` mapping.
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Ctr => "AES-CTR",
            Self::Cbc => "AES-CBC",
            Self::Gcm => "AES-GCM",
        }
    }

    pub(crate) fn algorithm_name(self) -> AlgorithmName {
        match self {
            Self::Ctr => AlgorithmName::AesCtr,
            Self::Cbc => AlgorithmName::AesCbc,
            Self::Gcm => AlgorithmName::AesGcm,
        }
    }

    /// The JWK `alg` value for an AES key of `length_bits` bits in this mode:
    /// the `alg` set by the AES import algorithms (WebCrypto §27.7.4 /
    /// §28.4.4 / §29.4.4) and emitted by the export algorithms (§27.7.5 /
    /// §28.4.5 / §29.4.5) — `A128GCM` / `A192CBC` / `A256CTR` …, or `None` for
    /// a non-AES key length.
    pub fn jwk_alg(self, length_bits: u32) -> Option<&'static str> {
        Some(match (length_bits, self) {
            (128, Self::Ctr) => "A128CTR",
            (128, Self::Cbc) => "A128CBC",
            (128, Self::Gcm) => "A128GCM",
            (192, Self::Ctr) => "A192CTR",
            (192, Self::Cbc) => "A192CBC",
            (192, Self::Gcm) => "A192GCM",
            (256, Self::Ctr) => "A256CTR",
            (256, Self::Cbc) => "A256CBC",
            (256, Self::Gcm) => "A256GCM",
            _ => return None,
        })
    }
}

/// The VM-marshalled raw algorithm identifier: `name` plus the members
/// the current operation may consult.  `hash` is itself a nested
/// `AlgorithmIdentifier`; `length` is the HMAC `unsigned long` /
/// AES-CTR `octet` / AES key-gen `unsigned short`; `iv` / `counter` /
/// `additional_data` are the AES `BufferSource` members (already snapshot-
/// copied by the VM); `tag_length` is the AES-GCM `octet`; `salt` / `info`
/// are the HKDF `BufferSource` members (`salt` shared with PBKDF2) and
/// `iterations` is the PBKDF2 `unsigned long` (WebCrypto §33.3 `HkdfParams`
/// / §34.3 `Pbkdf2Params`, snapshot-copied by the VM).  Which members the
/// VM populates is decided by [`params_shape`] for the `(op, name)` pair
/// (the registry-driven §18.4.4 step-5 recognition gate), so getters never
/// fire for an unregistered name.
#[derive(Clone, Debug, Default)]
pub struct RawAlgorithm {
    pub name: String,
    pub hash: Option<Box<RawAlgorithm>>,
    pub length: Option<u32>,
    pub iv: Option<Vec<u8>>,
    pub counter: Option<Vec<u8>>,
    pub additional_data: Option<Vec<u8>>,
    pub tag_length: Option<u32>,
    pub salt: Option<Vec<u8>>,
    pub info: Option<Vec<u8>>,
    pub iterations: Option<u32>,
}

impl RawAlgorithm {
    /// Construct from a bare name (the string form of an
    /// `AlgorithmIdentifier`); all params-dictionary members absent.
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
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
/// - `AesKeyGen` (generateKey) carries the variant + required key length.
/// - `AesImport` (importKey) carries only the variant — the key length
///   derives from the imported material.
/// - `AesKeyGen` (generateKey **and** AES get-key-length, §27.7.6 / §28.4.6
///   / §29.4.6) carries the variant + key `length`; the get-key-length op
///   reads only `length` (`AesDerivedKeyParams`, structurally identical to
///   `AesKeyGenParams`) — the variant is carried so [`Self::name`] stays
///   total without a sentinel.
/// - `AesGcm` / `AesCbc` / `AesCtr` (encrypt + decrypt share one variant
///   each — the op direction is the `ops::encrypt` vs `ops::decrypt`
///   entry, not the params) carry the mode-specific params.
/// - `Hkdf` / `Pbkdf2` (importKey + get-key-length) carry only the name —
///   the KDF key's `[[algorithm]]` is name-only (§33.4.2 / §34.4.2) and the
///   KDF get-key-length is null (§33.4.3 / §34.4.3).
/// - `HkdfParams` / `Pbkdf2Params` (deriveBits) carry the §33.3 / §34.3
///   derive params (the call-time `hash` lives here, not on the key).
///
/// Not `Copy`: the AES + KDF param variants own the marshalled `iv` /
/// `counter` / `additionalData` / `salt` / `info` byte sequences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizedAlgorithm {
    Digest(HashAlgorithm),
    Hmac,
    HmacKeyParams {
        hash: HashAlgorithm,
        length: Option<u32>,
    },
    AesKeyGen {
        variant: AesVariant,
        length: u32,
    },
    AesImport {
        variant: AesVariant,
    },
    AesGcm {
        iv: Vec<u8>,
        additional_data: Option<Vec<u8>>,
        tag_length: u32,
    },
    AesCbc {
        iv: Vec<u8>,
    },
    AesCtr {
        counter: Vec<u8>,
        length: u32,
    },
    /// HKDF / PBKDF2 name-only form (importKey + get-key-length).
    Hkdf,
    Pbkdf2,
    /// HKDF deriveBits params (WebCrypto §33.3 `HkdfParams`): the `hash`
    /// driving the HMAC, plus `salt` + `info` (both required `BufferSource`).
    HkdfParams {
        hash: HashAlgorithm,
        salt: Vec<u8>,
        info: Vec<u8>,
    },
    /// PBKDF2 deriveBits params (WebCrypto §34.3 `Pbkdf2Params`): `salt`
    /// (required `BufferSource`), `iterations`, and the PRF `hash`.
    Pbkdf2Params {
        salt: Vec<u8>,
        iterations: u32,
        hash: HashAlgorithm,
    },
}

impl NormalizedAlgorithm {
    /// The canonical algorithm name, for the operation "normalized
    /// algorithm `name` equals the key's `[[algorithm]]` name" check
    /// (sign / verify / encrypt / decrypt).
    pub fn name(&self) -> AlgorithmName {
        match self {
            Self::Digest(h) => match h {
                HashAlgorithm::Sha1 => AlgorithmName::Sha1,
                HashAlgorithm::Sha256 => AlgorithmName::Sha256,
                HashAlgorithm::Sha384 => AlgorithmName::Sha384,
                HashAlgorithm::Sha512 => AlgorithmName::Sha512,
            },
            Self::Hmac | Self::HmacKeyParams { .. } => AlgorithmName::Hmac,
            Self::AesKeyGen { variant, .. } | Self::AesImport { variant } => {
                variant.algorithm_name()
            }
            Self::AesGcm { .. } => AlgorithmName::AesGcm,
            Self::AesCbc { .. } => AlgorithmName::AesCbc,
            Self::AesCtr { .. } => AlgorithmName::AesCtr,
            Self::Hkdf | Self::HkdfParams { .. } => AlgorithmName::Hkdf,
            Self::Pbkdf2 | Self::Pbkdf2Params { .. } => AlgorithmName::Pbkdf2,
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
    /// AES `generateKey` (`AesKeyGenParams`) or AES `get key length`
    /// (`AesDerivedKeyParams`) — both name a `length` (required
    /// `[EnforceRange] unsigned short`) read by step 6; the op (generate vs
    /// get-key-length) is the [`crate::ops`] entry, not the params shape.
    AesKeyGen(AesVariant),
    /// AES `importKey`: a name-only `Algorithm` (registration params =
    /// `None`); the key length derives from the imported material.
    AesImport(AesVariant),
    /// AES `encrypt` / `decrypt`: the mode's params dictionary
    /// (`AesGcmParams` / `AesCbcParams` / `AesCtrParams`).
    AesEncryptDecrypt(AesVariant),
    /// HKDF / PBKDF2 name-only form (`importKey` + `get key length`): a
    /// name-only `Algorithm` (§33.4.2 / §34.4.2 import raw, §33.4.3 /
    /// §34.4.3 get-key-length null).
    KdfNameOnly(KdfKind),
    /// HKDF `deriveBits`: an `HkdfParams` (`hash` + `salt` + `info`, all
    /// required) read by step 6.
    HkdfDeriveBits,
    /// PBKDF2 `deriveBits`: a `Pbkdf2Params` (`salt` + `iterations` +
    /// `hash`, all required) read by step 6.
    Pbkdf2DeriveBits,
}

/// Which KDF a [`DesiredType::KdfNameOnly`] resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KdfKind {
    Hkdf,
    Pbkdf2,
}

/// §18.4.4 step 5: does `supportedAlgorithms[op]` contain a
/// case-insensitive match for `name`, and if so, which IDL dictionary
/// type does it resolve to? `None` ⇒ the spec returns `NotSupportedError`
/// at step 5, *before* the step-6 WebIDL conversion reads any
/// params-dictionary member.
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
        // HKDF / PBKDF2 (WebCrypto §33 / §34): import (raw) + deriveBits +
        // get-key-length (null).  No generateKey / encrypt / export — those
        // fall through to the AES catch-alls below, where `as_aes()` returns
        // `None` for a KDF name, so they resolve to `None` (NotSupported).
        (Operation::ImportKey | Operation::GetKeyLength, AlgorithmName::Hkdf) => {
            Some(DesiredType::KdfNameOnly(KdfKind::Hkdf))
        }
        (Operation::ImportKey | Operation::GetKeyLength, AlgorithmName::Pbkdf2) => {
            Some(DesiredType::KdfNameOnly(KdfKind::Pbkdf2))
        }
        (Operation::DeriveBits, AlgorithmName::Hkdf) => Some(DesiredType::HkdfDeriveBits),
        (Operation::DeriveBits, AlgorithmName::Pbkdf2) => Some(DesiredType::Pbkdf2DeriveBits),
        // AES generateKey / get-key-length both read a `length`-only dict
        // (`AesKeyGenParams` / `AesDerivedKeyParams`); `as_aes()` filters the
        // non-AES names (HMAC handled above, KDF handled above, SHA → None).
        (Operation::GenerateKey | Operation::GetKeyLength, _) => {
            name.as_aes().map(DesiredType::AesKeyGen)
        }
        (Operation::ImportKey, _) => name.as_aes().map(DesiredType::AesImport),
        (Operation::Encrypt | Operation::Decrypt, _) => {
            name.as_aes().map(DesiredType::AesEncryptDecrypt)
        }
        _ => None,
    }
}

/// The params-dictionary members the VM must read for a recognized
/// `(op, name)` pair (WebCrypto §18.4.4 step 6 "convert `alg` to the IDL
/// dictionary"), so the registry — not the VM marshalling layer — owns
/// which members each operation consults.  Returned by [`params_shape`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgorithmParams {
    /// No params-dictionary members beyond `name` (digest / sign / verify /
    /// AES importKey).
    NameOnly,
    /// HMAC generateKey / importKey: `hash` (required) + `length` (optional
    /// `unsigned long`).
    HmacKeyParams,
    /// AES generateKey: `length` (required `[EnforceRange] unsigned short`).
    AesKeyGen,
    /// AES-GCM encrypt / decrypt: `iv` (required), `additionalData`
    /// (optional), `tagLength` (optional `[EnforceRange] octet`).
    AesGcmParams,
    /// AES-CBC encrypt / decrypt: `iv` (required `BufferSource`).
    AesCbcParams,
    /// AES-CTR encrypt / decrypt: `counter` (required `BufferSource`) +
    /// `length` (required `[EnforceRange] octet`).
    AesCtrParams,
    /// HKDF deriveBits (`HkdfParams`): `hash` (required), `info` (required
    /// `BufferSource`), `salt` (required `BufferSource`).
    HkdfParams,
    /// PBKDF2 deriveBits (`Pbkdf2Params`): `hash` (required), `iterations`
    /// (required `[EnforceRange] unsigned long`), `salt` (required
    /// `BufferSource`).
    Pbkdf2Params,
}

/// §18.4.4 step 5 + step-6 member plan: for a registered `(op, name)` pair
/// return which params-dictionary members the VM should read; `None` ⇒ the
/// pair is unregistered, so the spec returns `NotSupportedError` at step 5
/// *before* any getter fires.  The VM marshalling layer routes through this
/// (rather than re-deriving the dictionary shape from the name string), so
/// the registry stays the single source of truth and an unregistered name
/// never triggers a user-defined member getter.
pub fn params_shape(op: Operation, name: &str) -> Option<AlgorithmParams> {
    resolve_registry(op, name).map(|d| match d {
        DesiredType::Digest(_)
        | DesiredType::HmacSignVerify
        | DesiredType::AesImport(_)
        | DesiredType::KdfNameOnly(_) => AlgorithmParams::NameOnly,
        DesiredType::HmacKeyParams => AlgorithmParams::HmacKeyParams,
        DesiredType::AesKeyGen(_) => AlgorithmParams::AesKeyGen,
        DesiredType::AesEncryptDecrypt(variant) => match variant {
            AesVariant::Gcm => AlgorithmParams::AesGcmParams,
            AesVariant::Cbc => AlgorithmParams::AesCbcParams,
            AesVariant::Ctr => AlgorithmParams::AesCtrParams,
        },
        DesiredType::HkdfDeriveBits => AlgorithmParams::HkdfParams,
        DesiredType::Pbkdf2DeriveBits => AlgorithmParams::Pbkdf2Params,
    })
}

/// §18.4.4 step 5 as a predicate: is `(op, name)` a registered pair?
/// (`params_shape(op, name).is_some()`.)
pub fn is_supported(op: Operation, name: &str) -> bool {
    params_shape(op, name).is_some()
}

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
    }
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
