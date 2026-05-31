//! Parse result types and fragment-parse options.
//!
//! These types are the SoT contract for HTML parsing in elidex. The
//! companion compat crate (`elidex-html-parser`) re-exports `ParseResult`
//! and `ParseFragmentOptions` via `pub use elidex_html_parser_strict::*`,
//! preserving caller import paths (`use elidex_html_parser::ParseResult`
//! etc.). [`crate::parse_strict`] returns a populated [`ParseResult`]
//! straight from the tree builder.

use std::fmt;

use elidex_ecs::{EcsDom, Entity};

/// Result of parsing an HTML document.
///
/// `EcsDom` does not implement `Debug`, so this type provides a manual
/// implementation that prints the document entity and error list.
///
/// Field shape is preserved from the existing `elidex-html-parser` type
/// (`crates/dom/elidex-html-parser/src/convert.rs`) so that A4 facade
/// re-export keeps caller code compatible.
pub struct ParseResult {
    /// The populated DOM tree.
    pub dom: EcsDom,
    /// The document root entity (parent of `<html>`).
    pub document: Entity,
    /// Parse warnings collected during parsing.
    ///
    /// Strict mode populates this only on `Err(StrictParseError)` paths;
    /// successful strict parses always return an empty `errors` vec.
    /// Tolerant mode (compat crate) collects html5ever recovery warnings.
    pub errors: Vec<String>,
    /// Detected encoding name.
    ///
    /// Always `None` for strict mode (`parse_strict` takes `&str` input,
    /// no charset detection). Tolerant mode (`parse_tolerant` in compat
    /// crate) populates with a canonical `encoding_rs` name (e.g.
    /// `"UTF-8"`, `"Shift_JIS"`).
    pub encoding: Option<&'static str>,
}

impl fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseResult")
            .field("document", &self.document)
            .field("errors", &self.errors)
            .field("encoding", &self.encoding)
            .finish_non_exhaustive()
    }
}

/// Options controlling fragment parsing semantics.
///
/// The `allow_declarative_shadow` flag selects between plain `innerHTML`
/// (no shadow attach) and HTML §4.12.3 `<template shadowrootmode>` /
/// DOM §4.9 attach-a-shadow-root semantics (where the template becomes
/// a shadow root attached to the parent host).
///
/// Shape preserved from existing `elidex-html-parser::ParseFragmentOptions`
/// for A4 facade re-export compatibility.
#[derive(Default, Clone, Copy, Debug)]
pub struct ParseFragmentOptions {
    /// When true, `<template shadowrootmode="open|closed">` children
    /// are interpreted as declarative shadow root markup. The parent
    /// receives a freshly-attached shadow root whose children come
    /// from the template's content, and the `<template>` element
    /// itself is discarded.
    ///
    /// Spec references: HTML `§4.12.3` (`shadowrootmode` attribute
    /// trigger) and DOM `§4.9` "attach a shadow root" algorithm.
    ///
    /// Per spec, a failed attach (for example because the host tag is
    /// not allowed, or the host already has a shadow root) silently
    /// leaves the `<template>` as an ordinary element.
    pub allow_declarative_shadow: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fragment_options_default() {
        let opts = ParseFragmentOptions::default();
        assert!(!opts.allow_declarative_shadow);
    }

    #[test]
    fn parse_fragment_options_copy() {
        let opts = ParseFragmentOptions {
            allow_declarative_shadow: true,
        };
        // `Copy` makes this implicit; reading `opts` after the binding
        // confirms it was not moved. (`Clone` is delegated to `Copy`,
        // so calling `.clone()` would just trip `clippy::clone_on_copy`.)
        let copied = opts;
        assert!(copied.allow_declarative_shadow);
        assert!(opts.allow_declarative_shadow);
    }

    #[test]
    fn parse_result_debug_shape() {
        let mut dom = EcsDom::new();
        let document = dom.create_document_root();
        let result = ParseResult {
            dom,
            document,
            errors: vec!["test-error".to_string()],
            encoding: Some("UTF-8"),
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("ParseResult"));
        assert!(debug.contains("test-error"));
        assert!(debug.contains("UTF-8"));
    }
}
