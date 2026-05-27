//! WebAssembly error classification.
//!
//! Maps wasmtime errors to the three WebAssembly JS API error classes:
//! `CompileError`, `LinkError`, `RuntimeError`.
//!
//! Per WASM JS API §5.10 "Error Objects" the spec defines three error
//! constructor classes; JS engines use `WasmErrorKind` to construct the
//! correct JS error type, avoiding engine-specific classification logic.
//!
//! ## Tier E — documented `wasmtime::Error` exception
//!
//! `WasmError::source` exposes `Option<wasmtime::Error>` as a public
//! field; this is the only `wasmtime::*` token allowed on the crate's
//! public surface (see `lib.rs` tier table). Justification: the
//! engine-bridge layer owns `wasmtime` as a direct dependency, and
//! preserving the source error lets D-16 host code chain-inspect
//! (downcast to `wasmtime::Trap`, etc.) when surfacing JS errors.
//! `Option<_>` (rather than the plan's literal non-Option shape) is a
//! deviation noted in the landing memo: native-only error paths
//! (overflow on `u32::try_from(grow_result_u64)`) have no wasmtime
//! cause, and synthesizing a placeholder would be a reactive
//! anti-pattern.

use std::fmt;

/// Classification of WebAssembly errors per WASM JS API §5.10
/// "Error Objects". `#[non_exhaustive]` carries additive room for
/// future proposals (e.g. Exception Handling's `WebAssembly.Exception`
/// would surface as a new kind once host machinery lands).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WasmErrorKind {
    /// Module validation / compilation failed (§5.10 `CompileError`).
    Compile,
    /// Module instantiation failed — e.g. missing imports (§5.10
    /// `LinkError`).
    Link,
    /// Runtime execution error — trap, fuel exhaustion, stack overflow
    /// (§5.10 `RuntimeError` + §7.1 stack-overflow + §7.2 out-of-memory).
    Runtime,
}

impl fmt::Display for WasmErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile => f.write_str("CompileError"),
            Self::Link => f.write_str("LinkError"),
            Self::Runtime => f.write_str("RuntimeError"),
        }
    }
}

/// A WebAssembly error with classification, message, and (when the
/// error originated inside wasmtime) the original `wasmtime::Error` for
/// chain inspection. See module-level docs for the tier-E exception
/// rationale on `source` — that is the ONLY pub field per plan §4.2
/// trip-wire #1; `kind` / `message` are private to keep construction
/// confined to `new()` / `with_source()` and reads to the accessors.
/// `#[non_exhaustive]` future-proofs adding new fields (e.g. proposal-
/// specific cause kinds) without a semver break.
#[derive(Debug)]
#[non_exhaustive]
pub struct WasmError {
    pub(crate) kind: WasmErrorKind,
    pub source: Option<wasmtime::Error>,
    pub(crate) message: String,
}

impl WasmError {
    /// Construct an error with no wasmtime source — used by native-only
    /// validation paths (e.g. overflow checks after `wasmtime::Memory::grow`
    /// returns a `u64` that exceeds `u32::MAX`).
    pub fn new(kind: WasmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            source: None,
            message: message.into(),
        }
    }

    /// Construct an error preserving the wasmtime cause. The message is
    /// derived from the source if not supplied explicitly. Used by the
    /// `engine_conv::wasm_error_from_wasmtime` classifier.
    ///
    /// `pub(crate)` to keep `wasmtime::Error` off the engine-bridge public
    /// surface — external callers should receive `WasmError` already
    /// constructed by `engine_conv` and inspect the cause through
    /// `source_err()` rather than wrap raw `wasmtime::Error` themselves.
    /// Plan §4.2 tier-E enumerates exactly `pub source` + `pub fn source_err`
    /// as the intentional engine-bridge pub-surface that mentions
    /// `wasmtime::Error`.
    pub(crate) fn with_source(kind: WasmErrorKind, source: wasmtime::Error) -> Self {
        let message = source.to_string();
        Self {
            kind,
            source: Some(source),
            message,
        }
    }

    pub fn kind(&self) -> &WasmErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the underlying wasmtime cause for error-chain inspection,
    /// `None` when the error originated from a native-only validation
    /// path.
    pub fn source_err(&self) -> Option<&wasmtime::Error> {
        self.source.as_ref()
    }
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for WasmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(AsRef::as_ref)
    }
}
