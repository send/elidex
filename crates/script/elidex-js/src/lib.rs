//! JavaScript parser for elidex (ES2020+ strict mode, pure Rust).
//!
//! Stage 1 of the elidex JS engine: a full-fidelity parser producing an
//! Arena-allocated AST with byte-offset spans on every node.
//!
//! # Features
//! - Byte-oriented lexer + recursive descent + Pratt expression parser
//! - ES2020+ strict mode baseline
//! - Error recovery: partial AST returned even on syntax errors
//! - Arena allocation with typed `NodeId` references
//! - Post-parse scope analysis
//!
//! # Design Policy: No Annex B
//!
//! elidex intentionally does NOT implement ECMA-262 Annex B ("Additional ECMAScript
//! Features for Web Browsers"). This includes: legacy octal escapes in strings/regexps,
//! `\0` followed by digits in non-unicode regexps, legacy octal integer literals,
//! sloppy mode `with` statements, `__proto__` semantics, HTML-like comments, relaxed
//! `RegExp` identity escapes/control escapes/backreferences in non-unicode mode, and
//! block-scoped function declarations in sloppy mode. See `docs/design` for rationale.

// Source positions are always < 4GB in a JS parser.
#![allow(clippy::cast_possible_truncation)]

pub mod arena;
pub mod ast;
pub mod atom;
pub mod bytecode;
pub mod compiler;
pub mod error;
pub mod regexp;
pub mod span;
pub mod token;
pub mod wtf16;
// Native functions share a fixed `fn(...) -> Result<JsValue, VmError>` pointer
// signature even when infallible, so wraps are inherent to the design.
#[allow(clippy::unnecessary_wraps)]
pub mod vm;

#[cfg(feature = "engine")]
mod engine;
#[cfg(feature = "engine")]
pub use engine::ElidexJsEngine;

#[cfg(test)]
#[cfg(feature = "engine")]
mod tests_call_listener;
#[cfg(test)]
#[cfg(feature = "engine")]
mod tests_dispatch_integration;

mod lexer;
mod parser;
mod scope;

pub use arena::{Arena, NodeId};
pub use ast::{Program, ProgramKind};
pub use atom::{Atom, StringInterner};
pub use error::{JsParseError, JsParseErrorKind, ParseOutput};
pub use scope::{Binding, BindingKind, Scope, ScopeAnalysis, ScopeKind};
pub use span::Span;

/// Parse source as a Script. Always returns a `Program` (possibly with Error nodes).
pub fn parse_script(source: &str) -> ParseOutput {
    let p = parser::Parser::new(source, ProgramKind::Script);
    p.parse()
}

/// Parse source as a Module. Always returns a `Program` (possibly with Error nodes).
pub fn parse_module(source: &str) -> ParseOutput {
    let p = parser::Parser::new(source, ProgramKind::Module);
    p.parse()
}

/// Post-parse scope analysis. Skips Error nodes gracefully.
#[must_use]
pub fn analyze_scopes(program: &Program) -> ScopeAnalysis {
    scope::analyze(program)
}
