//! [`VmError`] / [`VmErrorKind`] — VM execution errors.
//!
//! Extracted from `vm/value.rs` to keep that file under the
//! 1000-line convention.  Re-exported from `value` so downstream
//! code that uses `value::VmError` keeps compiling unchanged.

use std::fmt;

use super::value::{JsValue, StringId};

/// An error raised during VM execution.
#[derive(Debug)]
pub struct VmError {
    pub kind: VmErrorKind,
    pub message: String,
}

/// The kind of VM error.
#[derive(Debug)]
pub enum VmErrorKind {
    TypeError,
    ReferenceError,
    RangeError,
    SyntaxError,
    /// A URI encoding/decoding error.
    UriError,
    /// A user `throw` value — the thrown JS value is preserved.
    ThrowValue(JsValue),
    /// Internal VM error (should not occur in correct programs).
    InternalError,
    /// Compilation error.
    CompileError,
    /// A DOMException (WHATWG WebIDL §3.14).  The `name` field holds
    /// the interned spec name (e.g. `"SyntaxError"`,
    /// `"HierarchyRequestError"`) — distinct from JS `SyntaxError`
    /// even when they share the literal string.  `message` lives on
    /// the parent `VmError.message` field.
    ///
    /// `vm_error_to_thrown` materialises the variant into a proper
    /// `DOMException` instance with `DOMException.prototype` in its
    /// chain and `name` / `message` / `code` resolvable through
    /// prototype accessors (side-table `dom_exception_states`).
    DomException {
        name: StringId,
    },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix: &str = match &self.kind {
            VmErrorKind::TypeError => "TypeError",
            VmErrorKind::ReferenceError => "ReferenceError",
            VmErrorKind::RangeError => "RangeError",
            VmErrorKind::SyntaxError => "SyntaxError",
            VmErrorKind::UriError => "URIError",
            VmErrorKind::ThrowValue(_) => "Uncaught",
            VmErrorKind::InternalError => "InternalError",
            VmErrorKind::CompileError => "CompileError",
            // Display uses a fixed label — the actual interned name is
            // consumed when `vm_error_to_thrown` builds the JS-visible
            // instance.  Rust-side logging (swallowed-callback traces)
            // just sees `"DOMException: <message>"`.
            VmErrorKind::DomException { .. } => "DOMException",
        };
        write!(f, "{prefix}: {}", self.message)
    }
}

impl std::error::Error for VmError {}

impl VmError {
    /// Build a VmError carrying a user-thrown JS value.  Used to propagate
    /// `throw expr` and reject-forwarded reasons through the call stack
    /// without coercing them back to strings.
    ///
    /// A generic `message` is attached for diagnostic paths that log via
    /// the `Display` impl (timer / microtask callback swallow paths) —
    /// otherwise `"Uncaught: "` with an empty tail would hit stderr.
    /// Callers that want a richer message (e.g. the value's display form)
    /// can build a `VmError` directly.
    pub fn throw(value: JsValue) -> Self {
        Self {
            kind: VmErrorKind::ThrowValue(value),
            message: "JavaScript value thrown".to_string(),
        }
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::TypeError,
            message: message.into(),
        }
    }

    pub fn reference_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::ReferenceError,
            message: message.into(),
        }
    }

    pub fn syntax_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::SyntaxError,
            message: message.into(),
        }
    }

    pub fn range_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::RangeError,
            message: message.into(),
        }
    }

    pub fn uri_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::UriError,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::InternalError,
            message: message.into(),
        }
    }

    /// Build a DOMException-flavoured `VmError`.  `name` is a
    /// pre-interned `StringId` (lookup on `WellKnownStrings` —
    /// `dom_exc_*` fields) — the call site stays allocation-free on
    /// the hot throw path.  `vm_error_to_thrown` consumes the variant
    /// and hands the caller a `DOMException` instance whose
    /// `.name` / `.message` / `.code` accessors read from
    /// `VmInner::dom_exception_states` (WebIDL §3.6.8).
    pub fn dom_exception(name: StringId, message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::DomException { name },
            message: message.into(),
        }
    }
}
