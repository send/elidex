//! JS-value → `elidex-api-crypto` input marshalling for the
//! `SubtleCrypto` operations.
//!
//! Per CLAUDE.md "Layering mandate", these helpers only convert Web
//! IDL argument values into the engine-independent crate's input
//! types (algorithm-identifier conversion + normalization inputs,
//! `sequence<KeyUsage>` / `KeyFormat` / `JsonWebKey` conversion, the
//! `[EnforceRange]` length coercion) and build the exported `oct` JWK
//! object — all spec-validation lives in `elidex-api-crypto`.
//!
//! ## Submodules
//!
//! The combined surface exceeds the 1000-line file convention, so it is
//! split into a directory module by marshalling domain (the entry points
//! are re-exported here, so callers keep using `marshal::<fn>`):
//!
//! - `params` — algorithm-identifier conversion + the §18.4.4 params
//!   dictionary read (`convert_algorithm_identifier` / `marshal_algorithm`).
//! - `jwk` — the `JsonWebKey` dictionary read + the exported JWK object
//!   builder (`marshal_jwk` / `build_jwk_object`).
//! - `key` — the `CryptoKey`-arg brand check, `sequence<KeyUsage>` /
//!   `KeyFormat` conversion, and the `CryptoKeyPair` builder
//!   (`require_crypto_key_arg` / `marshal_usages` / `marshal_format` /
//!   `build_crypto_key_pair`).

mod jwk;
mod key;
mod params;

pub(super) use jwk::{build_jwk_object, marshal_jwk};
pub(super) use key::{
    build_crypto_key_pair, marshal_format, marshal_usages, require_crypto_key_arg,
};
pub(super) use params::{convert_algorithm_identifier, marshal_algorithm};
