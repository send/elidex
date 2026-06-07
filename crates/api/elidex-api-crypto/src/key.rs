//! `CryptoKey` data model (WebCrypto §13) — the engine-independent key
//! representation stored by the VM's `crypto_key_states` side-store.
//!
//! `length` is informational metadata only: an HMAC key is an octet
//! string and the MAC consumes the full [`KeyMaterial`]; there is no
//! trailing-bit masking (WebCrypto defines no masking step and browsers
//! store the full material, so `material` is the source of truth and
//! export round-trips deterministically).

use crate::algorithm::{AesVariant, AlgorithmName, NamedCurve};
use crate::hash::HashAlgorithm;

/// `CryptoKey.type` (WebCrypto §13 `KeyType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyType {
    Secret,
    Public,
    Private,
}

impl KeyType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Secret => "secret",
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

/// A `CryptoKey` usage (WebCrypto §13 `KeyUsage`). The full enum is
/// declared now; HMAC only accepts `Sign` / `Verify`. The variant
/// declaration order is the WebCrypto §13.2 canonical order, so the
/// derived `Ord` drives `normalize_usages`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyUsage {
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    DeriveKey,
    DeriveBits,
    WrapKey,
    UnwrapKey,
}

impl KeyUsage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Encrypt => "encrypt",
            Self::Decrypt => "decrypt",
            Self::Sign => "sign",
            Self::Verify => "verify",
            Self::DeriveKey => "deriveKey",
            Self::DeriveBits => "deriveBits",
            Self::WrapKey => "wrapKey",
            Self::UnwrapKey => "unwrapKey",
        }
    }

    /// Whether HMAC accepts this usage (WebCrypto §31.6.3/.4 step 1:
    /// `sign` / `verify` only).
    pub fn is_hmac_usage(self) -> bool {
        matches!(self, Self::Sign | Self::Verify)
    }

    /// Whether an AES block-cipher mode (CTR / CBC / GCM) accepts this usage
    /// (WebCrypto §27.7.3 / §28.4.3 / §29.4.3 step 1: `encrypt` / `decrypt` /
    /// `wrapKey` / `unwrapKey`).  AES-KW uses the stricter [`Self::is_aes_kw_usage`].
    pub fn is_aes_usage(self) -> bool {
        matches!(
            self,
            Self::Encrypt | Self::Decrypt | Self::WrapKey | Self::UnwrapKey
        )
    }

    /// Whether AES-KW accepts this usage (WebCrypto §30.3.3 / §30.3.4 step 1:
    /// `wrapKey` / `unwrapKey` only — AES-KW has no encrypt/decrypt op).
    pub fn is_aes_kw_usage(self) -> bool {
        matches!(self, Self::WrapKey | Self::UnwrapKey)
    }

    /// Whether HKDF / PBKDF2 accept this usage (WebCrypto §33.4.2 /
    /// §34.4.2 import step: `deriveKey` / `deriveBits` only).
    pub fn is_kdf_usage(self) -> bool {
        matches!(self, Self::DeriveKey | Self::DeriveBits)
    }

    /// Whether ECDSA accepts this usage for a key of `key_type` (WebCrypto
    /// §23.7.3 / §23.7.4): a public key accepts only `verify`; a private key
    /// only `sign`.  Unlike the symmetric predicates this is key-type-
    /// dependent (the usage split across the generated key pair).
    pub fn is_ecdsa_usage(self, key_type: KeyType) -> bool {
        match key_type {
            KeyType::Public => matches!(self, Self::Verify),
            KeyType::Private => matches!(self, Self::Sign),
            KeyType::Secret => false,
        }
    }

    /// Whether ECDH accepts this usage for a key of `key_type` (WebCrypto
    /// §24.4.1 / §24.4.3): a public key accepts **none** (ECDH public keys
    /// have no usages); a private key accepts `deriveKey` / `deriveBits`.
    pub fn is_ecdh_usage(self, key_type: KeyType) -> bool {
        match key_type {
            KeyType::Private => matches!(self, Self::DeriveKey | Self::DeriveBits),
            KeyType::Public | KeyType::Secret => false,
        }
    }

    /// Parse a `KeyUsage` from its IDL identifier, or `None` if unrecognized.
    pub fn from_ident(s: &str) -> Option<Self> {
        Some(match s {
            "encrypt" => Self::Encrypt,
            "decrypt" => Self::Decrypt,
            "sign" => Self::Sign,
            "verify" => Self::Verify,
            "deriveKey" => Self::DeriveKey,
            "deriveBits" => Self::DeriveBits,
            "wrapKey" => Self::WrapKey,
            "unwrapKey" => Self::UnwrapKey,
            _ => return None,
        })
    }
}

/// Normalize a usages list (WebCrypto "normalize usages" — used when
/// setting `CryptoKey.[[usages]]`): deduplicate and return the entries
/// in canonical (`KeyUsage` declaration) order, so `key.usages` and
/// exported JWK `key_ops` never expose duplicates or caller order.
pub fn normalize_usages(mut usages: Vec<KeyUsage>) -> Vec<KeyUsage> {
    usages.sort_unstable();
    usages.dedup();
    usages
}

/// The canonical algorithm descriptor stored on a `CryptoKey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAlgorithm {
    /// HMAC: the hash + the (informational) bit length.
    Hmac { hash: HashAlgorithm, length: u32 },
    /// AES (CTR / CBC / GCM): the mode + the key bit length (128/192/256).
    /// The algorithm object's `name` is `variant.canonical_name()`; an AES
    /// key has no `hash` member.
    Aes { variant: AesVariant, length: u32 },
    /// HKDF (WebCrypto §33) — a name-only `KeyAlgorithm`: the key's
    /// `[[algorithm]]` is `{ name: "HKDF" }` (the call-time `hash` / `salt`
    /// / `info` live on the `deriveBits` algorithm, not the key).
    Hkdf,
    /// PBKDF2 (WebCrypto §34) — a name-only `KeyAlgorithm`:
    /// `{ name: "PBKDF2" }`.
    Pbkdf2,
    /// ECDSA (WebCrypto §23) — the named curve.  The key's `[[algorithm]]`
    /// is the `EcKeyAlgorithm` `{ name: "ECDSA", namedCurve: "P-256"… }`
    /// (§23.5); the call-time signature `hash` lives on `EcdsaParams`
    /// (§23.3), not the key — mirroring the HMAC / HKDF hash split.
    Ecdsa { curve: NamedCurve },
    /// ECDH (WebCrypto §24) — the named curve.  The key's `[[algorithm]]`
    /// is the `EcKeyAlgorithm` `{ name: "ECDH", namedCurve: "P-256"… }`.
    Ecdh { curve: NamedCurve },
}

impl KeyAlgorithm {
    /// The canonical algorithm name for `[[algorithm]]` name comparison
    /// (WebCrypto sign/verify/encrypt/decrypt/deriveBits/deriveKey "name
    /// member equality" check).
    pub fn name(self) -> AlgorithmName {
        match self {
            Self::Hmac { .. } => AlgorithmName::Hmac,
            Self::Aes { variant, .. } => variant.algorithm_name(),
            Self::Hkdf => AlgorithmName::Hkdf,
            Self::Pbkdf2 => AlgorithmName::Pbkdf2,
            Self::Ecdsa { .. } => AlgorithmName::Ecdsa,
            Self::Ecdh { .. } => AlgorithmName::Ecdh,
        }
    }

    /// The EC named curve for an ECDSA / ECDH key, or `None` for a
    /// symmetric key (WebCrypto §23.5 / §24 `EcKeyAlgorithm.namedCurve`).
    pub fn named_curve(self) -> Option<NamedCurve> {
        match self {
            Self::Ecdsa { curve } | Self::Ecdh { curve } => Some(curve),
            Self::Hmac { .. } | Self::Aes { .. } | Self::Hkdf | Self::Pbkdf2 => None,
        }
    }
}

/// The engine-independent key material.  Symmetric algorithms (HMAC / AES)
/// and KDF input keying material use [`Self::Raw`]; elliptic-curve keys
/// (WebCrypto §23 ECDSA / §24 ECDH) use [`Self::Ec`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyMaterial {
    /// Symmetric / KDF key bytes (the full octet string).
    Raw(Vec<u8>),
    /// Elliptic-curve key material.  `public_point` is the SEC1 §2.3.3
    /// uncompressed encoding `0x04‖x‖y` (always present — derived from the
    /// scalar for a private key at import / generate time); `private_scalar`
    /// is `Some` iff this is a private key (the big-endian secret scalar,
    /// `NamedCurve::coordinate_len` bytes).  The typed curve key is
    /// reconstructed in the `ec` backend at op time (the asymmetric analogue
    /// of `Raw(bytes)` → cipher).
    Ec {
        public_point: Vec<u8>,
        private_scalar: Option<Vec<u8>>,
    },
}

impl KeyMaterial {
    /// The flat octet form of a symmetric (`Raw`) key.  EC key material has
    /// no single flat form (it carries a public point plus an optional
    /// scalar), and every symmetric op gates on an algorithm-name match
    /// before reading the material, so an EC key never reaches this arm —
    /// EC ops use [`Self::ec_public_point`] / [`Self::ec_private_scalar`].
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Raw(b) => b,
            Self::Ec { .. } => {
                unreachable!("EC key material has no flat byte form; use the ec_* accessors")
            }
        }
    }

    /// The SEC1 uncompressed public point of an EC key, or `None` for a
    /// symmetric key.
    pub fn ec_public_point(&self) -> Option<&[u8]> {
        match self {
            Self::Ec { public_point, .. } => Some(public_point),
            Self::Raw(_) => None,
        }
    }

    /// The big-endian private scalar of an EC **private** key, or `None`
    /// for a public or symmetric key.
    pub fn ec_private_scalar(&self) -> Option<&[u8]> {
        match self {
            Self::Ec { private_scalar, .. } => private_scalar.as_deref(),
            Self::Raw(_) => None,
        }
    }
}

/// The engine-independent `CryptoKey` payload (WebCrypto §13).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CryptoKeyData {
    pub key_type: KeyType,
    pub extractable: bool,
    pub algorithm: KeyAlgorithm,
    pub usages: Vec<KeyUsage>,
    pub material: KeyMaterial,
}

impl CryptoKeyData {
    /// Whether the key's usages include `usage`.
    pub fn has_usage(&self, usage: KeyUsage) -> bool {
        self.usages.contains(&usage)
    }
}
