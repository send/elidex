//! HTML parser for elidex.
//!
//! Parses HTML5 documents into an ECS-backed DOM tree using html5ever.
//!
//! Three entry points:
//! - [`parse_html`] — UTF-8 string input (existing API).
//! - [`parse_tolerant`] — raw byte input with charset auto-detection.
//! - [`parse_strict`] — strict mode that rejects documents with parse errors.

pub mod charset;
mod convert;
mod strict;

pub use charset::{detect_and_decode, DecodeResult, EncodingConfidence};
pub use convert::ParseResult;
pub use strict::{parse_strict, StrictParseError};

use convert::convert_document;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::ParseOpts;
use markup5ever_rcdom::RcDom;

/// Parse an HTML5 document from a UTF-8 string.
///
/// Uses html5ever for spec-compliant parsing with full error recovery.
/// Parse warnings are collected in [`ParseResult::errors`].
#[must_use]
pub fn parse_html(html: &str) -> ParseResult {
    let rc_dom = parse_document(RcDom::default(), ParseOpts::default()).one(html);
    convert_document(rc_dom)
}

/// Parse an HTML fragment string into child nodes.
///
/// Per WHATWG HTML §2.6.4: Fragment parsing uses a context element to
/// determine the parsing mode (e.g., `<table>` context enables table parsing).
/// The resulting nodes are appended as children of `parent` in the given DOM.
///
/// Returns the list of newly created root-level entities.
pub fn parse_html_fragment(
    html: &str,
    context_tag: &str,
    parent: elidex_ecs::Entity,
    dom: &mut elidex_ecs::EcsDom,
) -> Vec<elidex_ecs::Entity> {
    use html5ever::{ns, QualName};

    let context_name = QualName::new(None, ns!(html), context_tag.into());
    let rc_dom = html5ever::parse_fragment(
        RcDom::default(),
        ParseOpts::default(),
        context_name,
        Vec::new(),
        true, // scripting enabled
    )
    .one(html);

    // html5ever's parse_fragment may produce a single <html> wrapper element
    // containing the actual fragment children. We need to unwrap it.
    let children = rc_dom.document.children.borrow();
    if children.len() == 1 {
        // Single child — check if it's an <html> wrapper.
        let child = &children[0];
        if let markup5ever_rcdom::NodeData::Element { ref name, .. } = child.data {
            if name.local.as_ref() == "html" {
                // Unwrap: use the <html> element's children as fragment children.
                return convert::convert_fragment_children(child, parent, dom);
            }
        }
    }
    // No wrapper — use document's children directly.
    convert::convert_fragment_children(&rc_dom.document, parent, dom)
}

/// Parse an HTML5 document from raw bytes (browser mode).
///
/// Detects character encoding automatically, decodes to UTF-8, then parses
/// with html5ever's full error recovery. `charset_hint` is the HTTP
/// `Content-Type` header's `charset` parameter, if available.
#[must_use]
pub fn parse_tolerant(bytes: &[u8], charset_hint: Option<&str>) -> ParseResult {
    let decoded = detect_and_decode(bytes, charset_hint);
    let rc_dom = parse_document(RcDom::default(), ParseOpts::default()).one(decoded.text.as_str());
    let mut result = convert_document(rc_dom);
    result.encoding = Some(decoded.encoding);
    result
}

/// Shared test helpers for DOM tree inspection.
///
/// Used by tests in both `lib.rs` and `convert.rs` to avoid duplication.
#[cfg(test)]
pub(crate) mod test_helpers {
    use elidex_ecs::{EcsDom, Entity, TagType, TextContent};

    /// Walk children recursively and find the first element with the given tag.
    pub fn find_tag(dom: &EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
        for child in dom.children_iter(parent) {
            if let Ok(t) = dom.world().get::<&TagType>(child) {
                if t.0 == tag {
                    return Some(child);
                }
                if let Some(found) = find_tag(dom, child, tag) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Collect text content from direct children.
    pub fn child_text(dom: &EcsDom, parent: Entity) -> String {
        let mut result = String::new();
        for child in dom.children_iter(parent) {
            if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                result.push_str(&tc.0);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{child_text, find_tag};

    #[test]
    fn tolerant_utf8_bytes() {
        let html = b"<html><body><p>Hello</p></body></html>";
        let result = parse_tolerant(html, None);
        assert_eq!(result.encoding, Some("UTF-8"));
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "Hello");
    }

    #[test]
    fn tolerant_shift_jis() {
        // Build a Shift_JIS HTML document.
        // "日本語" in Shift_JIS: 0x93FA 0x967B 0x8CEA
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"<html><body><p>");
        bytes.extend_from_slice(&[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]);
        bytes.extend_from_slice(b"</p></body></html>");
        let result = parse_tolerant(&bytes, Some("Shift_JIS"));
        assert_eq!(result.encoding, Some("Shift_JIS"));
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "日本語");
    }

    #[test]
    fn tolerant_with_hint() {
        let html = b"<html><body>OK</body></html>";
        let result = parse_tolerant(html, Some("UTF-8"));
        assert_eq!(result.encoding, Some("UTF-8"));
        let body = find_tag(&result.dom, result.document, "body").expect("body");
        assert_eq!(child_text(&result.dom, body), "OK");
    }

    #[test]
    fn tolerant_broken_html() {
        let html = b"<div><span></div>";
        let result = parse_tolerant(html, None);
        // html5ever recovers from errors.
        assert!(!result.errors.is_empty());
        // DOM is still built.
        assert!(find_tag(&result.dom, result.document, "html").is_some());
    }

    #[test]
    fn parse_html_encoding_is_none() {
        let result = parse_html("<p>Hello</p>");
        assert!(result.encoding.is_none());
    }
}
