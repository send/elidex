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

/// Which degradation tier produced a parsed tree.
///
/// elidex's progressive-degradation model (design doc
/// `docs/design/ja/11-parser-design.md` `§11.3`) routes a document through
/// the fastest parser that accepts it: the strict (Tier-1) parser for
/// conforming HTML5, falling back to the tolerant rule-based-recovery
/// (Tier-2) backend otherwise. `ParseTier` records which tier produced a
/// given [`ParseResult`], so the strict-vs-fallback gradient is observable
/// (navigation logging today; gradient benchmarking later).
///
/// The tier is intrinsic to the backend that built the tree, not a separate
/// decision: [`crate::parse_strict`] only returns `Ok` for conforming input,
/// so its result is always [`ParseTier::Clean`]; the tolerant backend is the
/// rule-based-recovery handler, so its result is always
/// [`ParseTier::Recovered`]. A direct tolerant parse of already-valid markup
/// still reports `Recovered` — the label names the *tier that produced the
/// tree* (the `§11.3` Tier-1/Tier-2 boundary), not whether recovery rules
/// actually fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseTier {
    /// Tier-1: the strict parser accepted conforming HTML5 — the µs happy
    /// path. Produced by [`crate::parse_strict`] on success.
    Clean,
    /// Tier-2: the tolerant rule-based-recovery backend (html5ever, in the
    /// compat crate) produced the tree. Reached by a direct tolerant parse,
    /// or as the progressive-dispatch fallback after a strict parse error.
    Recovered,
}

/// Result of parsing an HTML document.
///
/// `EcsDom` does not implement `Debug`, so this type provides a manual
/// implementation that prints the document entity and error list.
///
/// This is the SoT type; the compat crate (`elidex-html-parser`)
/// re-exports it, so its tolerant html5ever path produces the same
/// `ParseResult` and all caller import paths stay compatible.
pub struct ParseResult {
    /// The populated DOM tree.
    pub dom: EcsDom,
    /// The document root entity (parent of `<html>`).
    pub document: Entity,
    /// Parse warnings.
    ///
    /// Always empty for strict mode: `parse_strict` reports errors out of
    /// band as `Err(StrictParseError)` and only ever returns a
    /// `ParseResult` on the success path. Tolerant mode (compat crate)
    /// collects html5ever recovery warnings here.
    pub errors: Vec<String>,
    /// Detected encoding name.
    ///
    /// Always `None` for the bare `&str` entry points (`parse_strict` /
    /// `parse_html`): no charset detection runs. The byte-input dispatch
    /// (`parse_progressive` in the compat crate) detects the encoding and
    /// populates this with a canonical `encoding_rs` name (e.g. `"UTF-8"`,
    /// `"Shift_JIS"`).
    pub encoding: Option<&'static str>,
    /// Which `§11.3` degradation tier produced this tree.
    ///
    /// [`ParseTier::Clean`] for the strict path ([`crate::parse_strict`]),
    /// [`ParseTier::Recovered`] for the tolerant backend. See [`ParseTier`]
    /// for the backend-intrinsic semantics (a direct tolerant parse of valid
    /// markup is still `Recovered` — the label names the producing tier).
    pub tier: ParseTier,
}

impl fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseResult")
            .field("document", &self.document)
            .field("errors", &self.errors)
            .field("encoding", &self.encoding)
            .field("tier", &self.tier)
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
/// The compat crate (`elidex-html-parser`) re-exports this type via the
/// facade, so caller import paths stay compatible.
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
            tier: ParseTier::Recovered,
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("ParseResult"));
        assert!(debug.contains("test-error"));
        assert!(debug.contains("UTF-8"));
        assert!(debug.contains("Recovered"));
    }
}
