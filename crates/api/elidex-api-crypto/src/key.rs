//! `CryptoKey` data model (WebCrypto §13) — the engine-independent key
//! representation stored by the VM's `crypto_key_states` side-store.
//!
//! `length` is informational metadata only: an HMAC key is an octet
//! string and the MAC consumes the full [`KeyMaterial`]; there is no
//! trailing-bit masking (WebCrypto defines no masking step and browsers
//! store the full material, so `material` is the source of truth and
//! export round-trips deterministically).

use crate::algorithm::{AesVariant, AlgorithmName};
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
        }
    }
}

/// The raw key bytes. (PR-1 ships only symmetric `Raw` material;
/// asymmetric DER/PEM variants land in later PRs.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyMaterial {
    Raw(Vec<u8>),
}

impl KeyMaterial {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Raw(b) => b,
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
