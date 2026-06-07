//! Engine-independent WebCrypto algorithms for elidex.
//!
//! Provides the pure algorithm + spec-validation layer behind the
//! `SubtleCrypto` VM thin binding (CLAUDE.md "Layering mandate"): the VM
//! marshals JS values to/from these `&[u8]` / typed-error APIs and never
//! performs crypto math or §14.3.x validation itself.
//!
//! # Layout
//!
//! - [`algorithm`] — §18.4 "normalize an algorithm" registry.
//! - `aes` (crate-internal) — AES-GCM / AES-CBC / AES-CTR encrypt /
//!   decrypt (§27-§29); reached only through [`ops`] (which validates the
//!   key length), so it is `pub(crate)` — the raw `&[u8]`-key cipher fns are
//!   not a public surface.
//! - [`hash`] — SHA-1/256/384/512 digest driver.
//! - [`hmac`] — HMAC sign / verify / key-length resolution (§31).
//! - [`jwk`] — `oct` JSON Web Key parse / serialize (§15).
//! - [`key`] — the [`key::CryptoKeyData`] model (§13).
//! - [`ops`] — operation-level entry points owning all spec validation.
//! - [`error`] — the [`error::AlgorithmError`] → DOMException taxonomy.

pub(crate) mod aes;
pub mod algorithm;
pub mod error;
pub mod hash;
pub mod hmac;
pub mod jwk;
pub mod key;
pub mod ops;

#[cfg(test)]
mod tests;

pub use algorithm::{
    is_supported, normalize, params_shape, AesVariant, AlgorithmName, AlgorithmParams,
    NormalizedAlgorithm, Operation, RawAlgorithm,
};
pub use error::AlgorithmError;
pub use hash::HashAlgorithm;
pub use jwk::JsonWebKey;
pub use key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
pub use ops::{ExportedKey, KeyData, KeyFormat};
