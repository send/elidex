//! The algorithm value types that flow through normalization: the
//! VM-marshalled [`RawAlgorithm`] input (plus its [`EcdhPeer`] member) and
//! the validated [`NormalizedAlgorithm`] output. The [`super::registry`]
//! resolver decides which `RawAlgorithm` members are populated and the
//! [`super::normalize`] procedure builds the `NormalizedAlgorithm`.

use crate::hash::HashAlgorithm;
use crate::key::KeyType;

use super::names::{AesVariant, AlgorithmName, EcAlgorithm, NamedCurve, RsaVariant};

/// The ECDH peer public key conveyed from the VM into `deriveBits`
/// (WebCrypto §24.3 `EcdhKeyDeriveParams.public`).  `public` is a
/// `CryptoKey` (a VM object), so per the Layering mandate the VM extracts
/// its spec-relevant metadata + the SEC1 public-point **bytes** here — the
/// engine-independent crate never holds a VM handle.  [`crate::ops::derive_bits`]
/// validates the §24.4.2 `InvalidAccessError` precedence (peer `[[type]]`
/// is "public"; peer `[[algorithm]]` name equals the base key's; peer curve
/// equals the base key's) against the base key, so the conveyed fields cover
/// every check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcdhPeer {
    /// The peer key's `[[type]]` (§24.4.2: must be "public").
    pub key_type: KeyType,
    /// The peer key's `[[algorithm]]` name (§24.4.2: must equal base name).
    pub algorithm: AlgorithmName,
    /// The peer key's curve — `Some` iff it is an EC key (the §24.4.2 curve
    /// check runs only once the name check confirms the same ECDH family).
    pub curve: Option<NamedCurve>,
    /// The peer key's SEC1 uncompressed public point — `Some` iff it is an
    /// EC public key (the ECDH input, used once validation passes).
    pub public_point: Option<Vec<u8>>,
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
/// VM populates is decided by [`crate::params_shape`] for the `(op, name)`
/// pair (the registry-driven §18.4.4 step-5 recognition gate), so getters
/// never fire for an unregistered name.
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
    /// EC `namedCurve` (WebCrypto §23.4 `EcKeyGenParams` / §23.6
    /// `EcKeyImportParams`) — the raw DOMString, validated to a
    /// [`NamedCurve`] by [`crate::normalize`].
    pub named_curve: Option<String>,
    /// ECDH peer public key (WebCrypto §24.3 `EcdhKeyDeriveParams.public`)
    /// — the VM-extracted metadata + SEC1 point ([`EcdhPeer`]; the marshalling
    /// boundary: the crate gets bytes + metadata, never a VM handle).
    pub peer: Option<EcdhPeer>,
    /// RSA `modulusLength` (WebCrypto §20.4 `RsaHashedKeyGenParams` —
    /// `[EnforceRange] unsigned long`), read on generateKey.
    pub modulus_length: Option<u32>,
    /// RSA `publicExponent` (WebCrypto §20.3 `RsaKeyGenParams` — the only
    /// WebCrypto `BigInteger` = big-endian `Uint8Array` octets, snapshot-
    /// copied by the VM), read on generateKey; moved by-value into the key
    /// like `iv`.
    pub public_exponent: Option<Vec<u8>>,
    /// RSA-PSS `saltLength` (WebCrypto §21.3 `RsaPssParams` —
    /// `[EnforceRange] unsigned long`), read on sign / verify.
    pub salt_length: Option<u32>,
    /// RSA-OAEP `label` (WebCrypto §22.3 `RsaOaepParams` — an **optional**
    /// `BufferSource`, snapshot-copied by the VM), read on encrypt / decrypt /
    /// wrapKey / unwrapKey; moved by-value into the normalized algorithm like
    /// the AES `iv`.
    pub label: Option<Vec<u8>>,
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
/// - `AesKwWrap` (wrapKey / unwrapKey, §30.3.1 / §30.3.2) carries only the
///   name — AES-KW uses the fixed RFC 3394 default IV, so there is no
///   per-call param. (AES-KW generate/import reuse `AesKeyGen{Kw,..}` /
///   `AesImport{Kw}`.)
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
    /// AES-KW wrapKey / unwrapKey (WebCrypto §30.3.1 / §30.3.2) — name-only
    /// (the RFC 3394 default IV is fixed, so there is no per-call param).
    AesKwWrap,
    /// EC generateKey (WebCrypto §23.4 `EcKeyGenParams` / §24.4.1) — the EC
    /// family (ECDSA / ECDH) + the curve.  The single requested `usages`
    /// list is split across the produced key pair by the op (§23.7.3 /
    /// §24.4.1), so it is not carried here.
    EcKeyGen {
        algorithm: EcAlgorithm,
        curve: NamedCurve,
    },
    /// EC importKey (WebCrypto §23.6 `EcKeyImportParams` / §24.4.3) — the EC
    /// family + the curve the imported material must match.
    EcImport {
        algorithm: EcAlgorithm,
        curve: NamedCurve,
    },
    /// ECDSA sign / verify params (WebCrypto §23.3 `EcdsaParams`): the
    /// signature `hash` (the curve comes from the key's `[[algorithm]]`).
    EcdsaParams {
        hash: HashAlgorithm,
    },
    /// ECDH deriveBits params (WebCrypto §24.3 `EcdhKeyDeriveParams`): the
    /// VM-extracted peer public key ([`EcdhPeer`]).  `derive_bits` validates
    /// the §24.4.2 peer checks against the base key.
    EcdhDerive {
        peer: EcdhPeer,
    },
    /// RSA generateKey (WebCrypto §20.4 `RsaHashedKeyGenParams` / §21.4.3) —
    /// the RSA family, the modulus bit length, the public exponent octets,
    /// and the message hash (carried on the key, §20.6).  The single
    /// requested `usages` list is split across the produced key pair by the
    /// op (§20.8.3 / §21.4.3), so it is not carried here.
    RsaKeyGen {
        variant: RsaVariant,
        modulus_length: u32,
        public_exponent: Vec<u8>,
        hash: HashAlgorithm,
    },
    /// RSA importKey (WebCrypto §20.7 `RsaHashedImportParams` / §21.4.4) —
    /// the RSA family + the message hash the imported key carries.
    RsaImport {
        variant: RsaVariant,
        hash: HashAlgorithm,
    },
    /// RSASSA-PKCS1-v1_5 sign / verify params (WebCrypto §20.8.1 / §20.8.2) —
    /// name-only: the dictionary is the bare `Algorithm` (the signature hash
    /// comes from the key's `[[algorithm]]`, §20.6).
    RsassaParams,
    /// RSA-PSS sign / verify params (WebCrypto §21.3 `RsaPssParams`): the
    /// `saltLength` (the hash still comes from the key's `[[algorithm]]`).
    RsaPssParams {
        salt_length: u32,
    },
    /// RSA-OAEP encrypt / decrypt params (WebCrypto §22.3 `RsaOaepParams`): the
    /// optional `label` (the OAEP + MGF1 hash comes from the key's
    /// `[[algorithm]]`, the §20.6 `RsaHashedKeyAlgorithm` reused by §22).
    /// §22.2 registers RSA-OAEP for `encrypt` / `decrypt` only, so this is
    /// reached directly from those two — and from `wrapKey` / `unwrapKey` only
    /// via the generic §14.3.11 / §14.3.12 encrypt / decrypt fallback (RSA-OAEP
    /// registers no own wrap op).
    RsaOaep {
        label: Option<Vec<u8>>,
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
            Self::AesKwWrap => AlgorithmName::AesKw,
            Self::EcKeyGen { algorithm, .. } | Self::EcImport { algorithm, .. } => {
                algorithm.algorithm_name()
            }
            Self::EcdsaParams { .. } => AlgorithmName::Ecdsa,
            Self::EcdhDerive { .. } => AlgorithmName::Ecdh,
            Self::RsaKeyGen { variant, .. } | Self::RsaImport { variant, .. } => {
                variant.algorithm_name()
            }
            Self::RsassaParams => AlgorithmName::RsassaPkcs1V15,
            Self::RsaPssParams { .. } => AlgorithmName::RsaPss,
            Self::RsaOaep { .. } => AlgorithmName::RsaOaep,
        }
    }
}
