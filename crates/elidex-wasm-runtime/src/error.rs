//! WebAssembly error classification.
//!
//! Maps wasmtime errors to the three WebAssembly JS API error classes:
//! `CompileError`, `LinkError`, `RuntimeError`.
//!
//! JS engines use `WasmErrorKind` to construct the correct JS error type,
//! avoiding engine-specific classification logic.

use std::fmt;

/// Classification of WebAssembly errors per JS API spec §3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmErrorKind {
    /// Module validation/compilation failed (§3.1 `CompileError`).
    Compile,
    /// Module instantiation failed — e.g. missing imports (§3.2 `LinkError`).
    Link,
    /// Runtime execution error — trap, fuel exhaustion (§3.3 `RuntimeError`).
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

/// A WebAssembly error with classification and message.
#[derive(Debug)]
pub struct WasmError {
    pub kind: WasmErrorKind,
    message: String,
}

impl WasmError {
    pub fn new(kind: WasmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for WasmError {}

/// Classify a wasmtime error into a `WasmErrorKind`.
///
/// Heuristic: if the error chain contains a `wasmtime::Trap`, it's a runtime
/// error. Otherwise, we rely on the call site to pass the correct kind.
pub(crate) fn classify_wasmtime_error(
    error: &wasmtime::Error,
    default_kind: WasmErrorKind,
) -> WasmError {
    // Check for Trap in the error chain (runtime error).
    if error.downcast_ref::<wasmtime::Trap>().is_some() {
        return WasmError::new(WasmErrorKind::Runtime, error.to_string());
    }
    WasmError::new(default_kind, error.to_string())
}
