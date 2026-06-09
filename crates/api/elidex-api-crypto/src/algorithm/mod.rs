//! Algorithm normalization registry (WebCrypto §18.4 "Algorithm
//! Normalization", procedure §18.4.4 "Normalizing an algorithm").
//!
//! The VM marshals a JS `AlgorithmIdentifier` (a string, or an object
//! with `name` + op-relevant members) into a [`RawAlgorithm`]; this
//! module validates the `(op, name)` pair against the registry and the
//! required params, returning a [`NormalizedAlgorithm`]. Later PRs
//! extend the surface by adding registry rows, not by special-casing
//! call sites.
//!
//! ## Submodules
//!
//! The combined surface exceeds the 1000-line file convention, so it is
//! split into a directory module along the §18.4.4 data flow:
//!
//! - `names` — the recognized-algorithm + variant vocabulary
//!   ([`AlgorithmName`] and the [`AesVariant`] / [`NamedCurve`] /
//!   [`EcAlgorithm`] / [`RsaVariant`] family discriminators).
//! - `model` — the algorithm value types: the VM-marshalled
//!   [`RawAlgorithm`] input (+ [`EcdhPeer`]) and the validated
//!   [`NormalizedAlgorithm`] output.
//! - `registry` — the §18.4.4 step-5 `(op, name)` → desired-type oracle
//!   (`resolve_registry`) and its public [`params_shape`] / [`is_supported`]
//!   views.
//! - `normalize` — the §18.4.4 procedure itself ([`normalize()`]) plus the
//!   required-member / unrecognized-name error helpers.

mod model;
mod names;
mod normalize;
mod registry;

pub use model::{EcdhPeer, NormalizedAlgorithm, RawAlgorithm};
pub use names::{AesVariant, AlgorithmName, EcAlgorithm, NamedCurve, RsaVariant};
pub use normalize::normalize;
pub use registry::{is_supported, params_shape, AlgorithmParams};

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
