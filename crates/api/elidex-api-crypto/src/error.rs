//! Error taxonomy for WebCrypto operations.
//!
//! Each [`AlgorithmError`] variant maps to a specific JS-observable
//! exception when surfaced by the VM thin binding:
//!
//! | Variant | JS exception |
//! |---|---|
//! | `NotSupported` | `NotSupportedError` (DOMException) |
//! | `Data` | `DataError` (DOMException) |
//! | `Syntax` | `SyntaxError` (DOMException) |
//! | `InvalidAccess` | `InvalidAccessError` (DOMException) |
//! | `Operation` | `OperationError` (DOMException) |
//! | `Type` | `TypeError` (plain JS TypeError) |
//!
//! The crate is engine-independent, so it carries only the canonical
//! name + message; the VM host maps the variant to the concrete
//! DOMException constructor / `VmError::type_error`.

use core::fmt;

/// A WebCrypto operation failure, carrying the exception name + message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlgorithmError {
    /// Unrecognized / unregistered algorithm or format → `NotSupportedError`.
    NotSupported(String),
    /// Malformed key material / JWK shape / out-of-range length → `DataError`.
    Data(String),
    /// Invalid key usages (empty or unsupported) → `SyntaxError`.
    Syntax(String),
    /// Operation not permitted for the key (missing usage, non-extractable,
    /// algorithm mismatch) → `InvalidAccessError`.
    InvalidAccess(String),
    /// Operation-specific failure (e.g. zero-length key generation)
    /// → `OperationError`.
    Operation(String),
    /// WebIDL conversion failure (missing required member) → `TypeError`.
    Type(String),
}

impl AlgorithmError {
    /// The JS exception name this error maps to (`"TypeError"` for
    /// [`Self::Type`], otherwise the DOMException name).
    pub fn exception_name(&self) -> &'static str {
        match self {
            Self::NotSupported(_) => "NotSupportedError",
            Self::Data(_) => "DataError",
            Self::Syntax(_) => "SyntaxError",
            Self::InvalidAccess(_) => "InvalidAccessError",
            Self::Operation(_) => "OperationError",
            Self::Type(_) => "TypeError",
        }
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        match self {
            Self::NotSupported(m)
            | Self::Data(m)
            | Self::Syntax(m)
            | Self::InvalidAccess(m)
            | Self::Operation(m)
            | Self::Type(m) => m,
        }
    }
}

impl fmt::Display for AlgorithmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.exception_name(), self.message())
    }
}

impl std::error::Error for AlgorithmError {}
