//! HTML parser for elidex.
//!
//! Parses HTML5 documents into an ECS-backed DOM tree using html5ever.
//!
//! Entry points:
//! - [`parse_html`] — UTF-8 string input, tolerant html5ever (§11.3 Tier-2).
//! - [`parse_progressive`] — raw byte input: charset auto-detection + §11.3
//!   strict-first dispatch (Tier-1) with a tolerant fallback (Tier-2).
//! - [`parse_strict`] — strict mode that rejects documents with parse errors.

pub mod charset;
mod convert;

pub use charset::{detect_and_decode, DecodeResult, EncodingConfidence};
// `ParseResult`, `ParseTier`, `ParseFragmentOptions`, `StrictParseError`,
// and `parse_strict` are owned by the engine-independent
// `elidex-html-parser-strict` crate (the strict-mode SoT). They are
// re-exported here so existing `elidex_html_parser::…` import paths keep
// working; the tolerant html5ever entry points below produce the same
// `ParseResult` type (tagged `ParseTier::Recovered`).
pub use elidex_html_parser_strict::{
    parse_strict, ParseFragmentOptions, ParseResult, ParseTier, StrictParseError,
};

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
    opts: ParseFragmentOptions,
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
                return convert::convert_fragment_children(child, parent, dom, opts);
            }
        }
    }
    // No wrapper — use document's children directly.
    convert::convert_fragment_children(&rc_dom.document, parent, dom, opts)
}

/// Parse an HTML5 document from raw bytes with `§11.3` progressive
/// degradation (browser mode).
///
/// Detects the character encoding once, up front, then tries the strict
/// (Tier-1) parser: conforming HTML5 takes the fast strict path
/// ([`ParseTier::Clean`]). On the first strict parse error (a WHATWG HTML
/// `§13.2.2` parse error) it falls back to the tolerant html5ever backend
/// (Tier-2 rule-based recovery, [`ParseTier::Recovered`]) over the *same*
/// decoded text — no re-decode. `charset_hint` is the HTTP `Content-Type`
/// header's `charset` parameter, if available.
///
/// The fallback is correctness-safe: the strict parser rejects
/// conservatively (no error recovery), so the worst case is the same tree
/// the tolerant backend would have produced on its own. The resulting
/// [`ParseResult::tier`] records which tier ran, making the strict-vs-
/// fallback gradient observable (`§11.3`).
#[must_use]
pub fn parse_progressive(bytes: &[u8], charset_hint: Option<&str>) -> ParseResult {
    let decoded = detect_and_decode(bytes, charset_hint);
    // Tier-1: the strict parser accepts conforming HTML5 — `into_result`
    // already tagged the result `ParseTier::Clean`. On the first strict parse
    // error, fall back to the tolerant html5ever backend over the *same*
    // decoded text (no re-decode), which `convert_document` tags
    // `ParseTier::Recovered`. Either way, stamp the detected encoding (both
    // `&str` entry points leave it `None`).
    let mut result = parse_strict(&decoded.text).unwrap_or_else(|_| parse_html(&decoded.text));
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
    use elidex_ecs::{Attributes, CommentData, DocTypeData, EcsDom, Entity, TagType, TextContent};
    use std::collections::BTreeMap;

    /// Collect an element's attributes as an order-independent name→value map.
    fn attr_map(dom: &EcsDom, e: Entity) -> BTreeMap<String, String> {
        dom.world()
            .get::<&Attributes>(e)
            .map(|a| {
                a.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A node's identity for differential comparison: kind-specific payload
    /// (element tag / text / comment / doctype) plus attributes. Children are
    /// compared separately by the recursive walk.
    type NodeSig = (
        Option<String>,      // TagType
        Option<String>,      // TextContent
        Option<String>,      // CommentData
        Option<DocTypeData>, // DocTypeData
        BTreeMap<String, String>,
    );

    fn node_sig(dom: &EcsDom, e: Entity) -> NodeSig {
        let w = dom.world();
        (
            w.get::<&TagType>(e).ok().map(|t| t.0.clone()),
            w.get::<&TextContent>(e).ok().map(|t| t.0.clone()),
            w.get::<&CommentData>(e).ok().map(|c| c.0.clone()),
            w.get::<&DocTypeData>(e).ok().map(|d| (*d).clone()),
            attr_map(dom, e),
        )
    }

    /// Assert two parsed subtrees are structurally identical: node signature
    /// (tag / text / comment / doctype + attributes) and child order, walked
    /// recursively. Proves the strict (Tier-1) parser yields the same tree as
    /// the tolerant html5ever (Tier-2) backend for conforming HTML5.
    fn assert_subtree_eq(a: &EcsDom, an: Entity, b: &EcsDom, bn: Entity, path: &str) {
        assert_eq!(node_sig(a, an), node_sig(b, bn), "node mismatch at {path}");
        let ac: Vec<Entity> = a.children_iter(an).collect();
        let bc: Vec<Entity> = b.children_iter(bn).collect();
        assert_eq!(
            ac.len(),
            bc.len(),
            "child-count mismatch at {path} (sig {:?})",
            node_sig(a, an)
        );
        for (i, (ca, cb)) in ac.iter().zip(bc.iter()).enumerate() {
            assert_subtree_eq(a, *ca, b, *cb, &format!("{path}/{i}"));
        }
    }

    // --- §11.3 progressive dispatch: byte input + charset + tier ---

    #[test]
    fn progressive_charset_utf8_bytes() {
        // No DOCTYPE ⇒ strict rejects ⇒ Tier-2 tolerant fallback.
        let html = b"<html><body><p>Hello</p></body></html>";
        let result = parse_progressive(html, None);
        assert_eq!(result.encoding, Some("UTF-8"));
        assert_eq!(result.tier, ParseTier::Recovered);
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "Hello");
    }

    #[test]
    fn progressive_charset_shift_jis() {
        // "日本語" in Shift_JIS: 0x93FA 0x967B 0x8CEA
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"<html><body><p>");
        bytes.extend_from_slice(&[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]);
        bytes.extend_from_slice(b"</p></body></html>");
        let result = parse_progressive(&bytes, Some("Shift_JIS"));
        assert_eq!(result.encoding, Some("Shift_JIS"));
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "日本語");
    }

    #[test]
    fn progressive_charset_with_hint() {
        let html = b"<html><body>OK</body></html>";
        let result = parse_progressive(html, Some("UTF-8"));
        assert_eq!(result.encoding, Some("UTF-8"));
        let body = find_tag(&result.dom, result.document, "body").expect("body");
        assert_eq!(child_text(&result.dom, body), "OK");
    }

    #[test]
    fn progressive_clean_tier_for_conforming_html5() {
        // Conforming HTML5 (leading DOCTYPE, explicit head/body) takes the
        // strict Tier-1 fast path.
        let html = b"<!DOCTYPE html><html><head></head><body><p>Hi</p></body></html>";
        let result = parse_progressive(html, None);
        assert_eq!(result.tier, ParseTier::Clean);
        assert_eq!(result.encoding, Some("UTF-8"));
        assert!(result.errors.is_empty());
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "Hi");
    }

    #[test]
    fn progressive_broken_html_falls_back_to_recovered() {
        let html = b"<div><span></div>";
        let result = parse_progressive(html, None);
        assert_eq!(result.tier, ParseTier::Recovered);
        // html5ever recovered and still built a tree.
        assert!(!result.errors.is_empty());
        assert!(find_tag(&result.dom, result.document, "html").is_some());
    }

    #[test]
    fn progressive_fallback_matches_direct_tolerant() {
        // On the fallback path, parse_progressive must equal a direct
        // parse_html over the same decoded text (errors + tree).
        let bytes = b"<div><span></div>";
        let prog = parse_progressive(bytes, None);
        let decoded = detect_and_decode(bytes, None);
        let direct = parse_html(&decoded.text);
        assert_eq!(prog.tier, ParseTier::Recovered);
        assert_eq!(prog.errors, direct.errors);
        assert_subtree_eq(
            &prog.dom,
            prog.document,
            &direct.dom,
            direct.document,
            "root",
        );
    }

    // --- tier is intrinsic to the producing backend ---

    #[test]
    fn tier_is_clean_from_strict_and_recovered_from_tolerant() {
        let valid = "<!DOCTYPE html><html><head></head><body><p>x</p></body></html>";
        assert_eq!(parse_strict(valid).expect("valid").tier, ParseTier::Clean);
        // parse_html (the tolerant backend) always reports Recovered, even on
        // already-valid markup — the label names the producing tier, not
        // whether recovery rules fired.
        assert_eq!(parse_html(valid).tier, ParseTier::Recovered);
    }

    // --- differential correctness: strict tree ≅ tolerant tree on valid HTML5 ---

    #[test]
    fn strict_and_tolerant_agree_on_whitespace_free_conforming_html5() {
        // Scope note: the corpus is intentionally **whitespace-free** between
        // elements. The strict parser is spec-faithful and keeps inter-element
        // whitespace text nodes, whereas the tolerant html5ever path strips
        // them (convert.rs), so the two backends only produce identical trees
        // when there is no inter-element whitespace to disagree about. That
        // divergence is pinned separately by
        // `strict_keeps_inter_element_whitespace_that_tolerant_strips`; here we
        // prove that, modulo whitespace, strict reproduces the tolerant tree
        // (tags / text / comments / doctype / attributes / child order) for
        // conforming HTML5 — the safety property behind routing valid docs to
        // strict.
        let cases: &[&str] = &[
            "<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>",
            "<!DOCTYPE html><html><head></head><body><div><p>A</p><p>B</p></div></body></html>",
            r#"<!DOCTYPE html><html><head></head><body><a href="https://example.com" class="link">x</a></body></html>"#,
            "<!DOCTYPE html><html><head></head><body><ul><li>one</li><li>two</li></ul></body></html>",
            "<!DOCTYPE html><html><head><title>T</title></head><body><!-- note -->text</body></html>",
        ];
        for (i, html) in cases.iter().enumerate() {
            let strict = parse_strict(html)
                .unwrap_or_else(|e| panic!("case {i} should be valid HTML5: {e}"));
            assert_eq!(strict.tier, ParseTier::Clean, "case {i}");
            let tolerant = parse_html(html);
            assert_subtree_eq(
                &strict.dom,
                strict.document,
                &tolerant.dom,
                tolerant.document,
                &format!("case{i}"),
            );
        }
    }

    #[test]
    fn strict_keeps_inter_element_whitespace_that_tolerant_strips() {
        // Known, accepted §11.3 divergence (pinned so it stays visible rather
        // than silently masked): the strict parser inserts every character
        // (WHATWG HTML §13.2.5/§13.2.6), so a conforming *indented* document
        // keeps whitespace-only text nodes between elements — the spec-correct
        // DOM. The tolerant html5ever path drops them (convert.rs treats an
        // all-whitespace text run as no node). Routing valid docs to strict
        // therefore yields extra inter-element whitespace text nodes vs. the
        // previous tolerant-only behaviour; downstream handling of those nodes
        // is tracked as a follow-up.
        let html =
            "<!DOCTYPE html><html><head></head><body>\n  <p>Hi</p>\n  <p>Bye</p>\n</body></html>";
        let strict = parse_strict(html).expect("valid HTML5");
        let tolerant = parse_html(html);
        let sbody = find_tag(&strict.dom, strict.document, "body").expect("strict body");
        let tbody = find_tag(&tolerant.dom, tolerant.document, "body").expect("tolerant body");
        let s_kids = strict.dom.children_iter(sbody).count();
        let t_kids = tolerant.dom.children_iter(tbody).count();
        // Tolerant: only the two <p> elements (whitespace stripped).
        assert_eq!(t_kids, 2, "tolerant strips inter-element whitespace");
        // Strict: the two <p> elements plus surrounding whitespace text nodes.
        assert!(
            s_kids > t_kids,
            "strict should keep inter-element whitespace text nodes \
             (strict body children={s_kids}, tolerant={t_kids})"
        );
    }

    #[test]
    fn parse_html_encoding_is_none() {
        let result = parse_html("<p>Hello</p>");
        assert!(result.encoding.is_none());
    }

    #[test]
    fn parse_html_fragment_declarative_shadow_attaches_open_root() {
        use elidex_ecs::{Attributes, EcsDom};
        let mut dom = EcsDom::new();
        let host = dom.create_element("div", Attributes::default());
        let _ = dom.create_document_root(); // detached but enough for tag lookups
        let added = parse_html_fragment(
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            "div",
            host,
            &mut dom,
            ParseFragmentOptions {
                allow_declarative_shadow: true,
            },
        );
        // The template is consumed: no light-tree child entities added.
        assert!(
            added.is_empty(),
            "declarative shadow consumes the template; no light-tree adds expected"
        );
        let sr = dom
            .get_shadow_root(host)
            .expect("shadow root must be attached");
        // The shadow tree holds the parsed <p>.
        let shadow_kids = dom.children(sr);
        let p_present = shadow_kids.iter().any(|c| {
            dom.world()
                .get::<&elidex_ecs::TagType>(*c)
                .is_ok_and(|t| t.0 == "p")
        });
        assert!(
            p_present,
            "shadow root should hold the <p>; got tags {:?}",
            shadow_kids
                .iter()
                .map(|c| dom
                    .world()
                    .get::<&elidex_ecs::TagType>(*c)
                    .map(|t| t.0.clone())
                    .ok())
                .collect::<Vec<_>>()
        );
    }
}
