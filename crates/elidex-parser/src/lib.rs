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

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{TagType, TextContent};

    fn find_tag(
        dom: &elidex_ecs::EcsDom,
        parent: elidex_ecs::Entity,
        tag: &str,
    ) -> Option<elidex_ecs::Entity> {
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

    fn child_text(dom: &elidex_ecs::EcsDom, parent: elidex_ecs::Entity) -> String {
        let mut result = String::new();
        for child in dom.children_iter(parent) {
            if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                result.push_str(&tc.0);
            }
        }
        result
    }

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
