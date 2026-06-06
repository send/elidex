//! `CryptoKey` data model (WebCrypto §13) — the engine-independent key
//! representation stored by the VM's `crypto_key_states` side-store.
//!
//! `length` is informational metadata only: an HMAC key is an octet
//! string and the MAC consumes the full [`KeyMaterial`]; there is no
//! trailing-bit masking (WebCrypto defines no masking step and browsers
//! store the full material, so `material` is the source of truth and
//! export round-trips deterministically).

use crate::algorithm::AlgorithmName;
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
/// declared now; HMAC only accepts `Sign` / `Verify`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// The canonical algorithm descriptor stored on a `CryptoKey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAlgorithm {
    /// HMAC: the hash + the (informational) bit length.
    Hmac { hash: HashAlgorithm, length: u32 },
}

impl KeyAlgorithm {
    /// The hash backing this key's algorithm.
    pub fn hash(self) -> HashAlgorithm {
        match self {
            Self::Hmac { hash, .. } => hash,
        }
    }

    /// The canonical algorithm name for `[[algorithm]]` name comparison
    /// (WebCrypto sign/verify "name member equality" check).
    pub fn name(self) -> AlgorithmName {
        match self {
            Self::Hmac { .. } => AlgorithmName::Hmac,
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
