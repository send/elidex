//! HTML parser for elidex.
//!
//! Parses HTML5 documents into an ECS-backed DOM tree. Conforming documents
//! take the strict parser (design doc §11.3 Tier-1); html5ever is the tolerant
//! fallback (Tier-2) and the backend for the bare string/fragment entry points.
//!
//! Entry points:
//! - [`parse_html`] — UTF-8 string input, tolerant html5ever (§11.3 Tier-2).
//! - [`parse_progressive`] — raw byte input: charset auto-detection + §11.3
//!   strict-first dispatch (Tier-1) with a tolerant fallback (Tier-2).
//! - [`parse_strict`] — strict mode that rejects documents with parse errors.

pub mod charset;
mod convert;
mod element_init;

pub use charset::{detect_and_decode, DecodeResult, EncodingConfidence};
// `ParseResult`, `ParseTier`, `ParseFragmentOptions`, `StrictParseError`,
// and `parse_fragment_strict` are owned by the engine-independent
// `elidex-html-parser-strict` crate (the strict-mode SoT). They are
// re-exported here so existing `elidex_html_parser::…` import paths keep
// working; the tolerant html5ever entry points below produce the same
// `ParseResult` type (tagged `ParseTier::Recovered`).
//
// `parse_strict` is intentionally NOT re-exported: this crate defines its
// own [`parse_strict`] wrapper (below) that runs the derived-component pass,
// so every public document entry point (`parse_html` / `parse_progressive` /
// `parse_strict`) yields a complete `ParseResult`. (`parse_fragment_strict`
// stays the raw re-export pending the slice-2b fragment-caller unification —
// slot `#11-strict-parser-created-element-ecs-state` fragment wiring.)
pub use elidex_html_parser_strict::{
    parse_fragment_strict, ParseFragmentOptions, ParseResult, ParseTier, StrictParseError,
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

/// Parse a conforming HTML5 document with the strict (Tier-1) backend.
///
/// Wrapper over [`elidex_html_parser_strict::parse_strict`] that runs the
/// canonical derived-component pass (`element_init`) on success, so the
/// public strict entry point produces the same complete [`ParseResult`] —
/// `CustomElementState` (HTML §4.13.3) / `IframeData`
/// (§4.8.5) attached — as [`parse_html`] and [`parse_progressive`]. The
/// strict tree-builder crate itself stays DOM-semantics-free (no
/// `elidex-custom-elements` dep); the derivation lives one layer up here.
///
/// # Errors
/// Propagates the [`StrictParseError`] from the strict backend when the
/// input is not conforming HTML5 (callers fall back to [`parse_html`]).
pub fn parse_strict(html: &str) -> Result<ParseResult, StrictParseError> {
    let mut result = elidex_html_parser_strict::parse_strict(html)?;
    element_init::derive_element_components(&mut result.dom, result.document);
    Ok(result)
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
    // Tier-1 strict: the local `parse_strict` wrapper runs the derived-
    // component pass on success; the tolerant fallback derives inside
    // `convert_document`. Either way the returned tree carries the
    // CustomElementState / IframeData components.
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
    fn strict_and_tolerant_agree_on_conforming_html5() {
        // Both backends preserve inter-element whitespace per WHATWG HTML
        // §13.2.6: the strict parser inserts every character, and since the
        // §11.3 unify the tolerant html5ever path keeps html5ever's whitespace
        // text nodes too (convert.rs no longer over-strips). So strict
        // reproduces the tolerant tree — tags / text (whitespace included) /
        // comments / doctype / attributes / child order — for conforming
        // HTML5, the safety property behind routing valid docs to strict. The
        // corpus mixes whitespace-free and indented documents; the focused
        // whitespace-preservation pin is
        // `tolerant_preserves_inter_element_whitespace_matching_strict`.
        let cases: &[&str] = &[
            "<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>",
            "<!DOCTYPE html><html><head></head><body><div><p>A</p><p>B</p></div></body></html>",
            r#"<!DOCTYPE html><html><head></head><body><a href="https://example.com" class="link">x</a></body></html>"#,
            "<!DOCTYPE html><html><head></head><body><ul><li>one</li><li>two</li></ul></body></html>",
            "<!DOCTYPE html><html><head><title>T</title></head><body><!-- note -->text</body></html>",
            // Indented / whitespace-laden conforming documents — the §11.3 unify
            // proof: with the tolerant strip gone, these agree byte-for-byte too.
            "<!DOCTYPE html><html><head></head><body>\n  <p>A</p>\n  <p>B</p>\n</body></html>",
            "<!DOCTYPE html><html><head></head><body><div>\n  <span>x</span>\n  <span>y</span>\n</div></body></html>",
            // Whitespace in framing / head positions: §13.2.6 drops it before
            // <head> ("initial"/"before html"/"before head" modes ignore the
            // character) and retains it inside <head>/<body> ("in head"/"in body"
            // modes insert it). Both backends apply the same §13.2.6 rules, so
            // they agree on which framing whitespace survives.
            "<!DOCTYPE html>\n<html>\n<head>\n<title>T</title>\n</head>\n<body><p>x</p></body>\n</html>",
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
    fn tolerant_preserves_inter_element_whitespace_matching_strict() {
        // §11.3 whitespace unify (was `strict_keeps_..._that_tolerant_strips`,
        // which pinned the now-eliminated divergence): the tolerant html5ever
        // path keeps the whitespace-only text nodes html5ever places between
        // elements per WHATWG HTML §13.2.6 (convert.rs no longer over-strips
        // them), so on a conforming *indented* document it agrees with the
        // spec-faithful strict backend. This is the navigation(strict) ↔
        // innerHTML(tolerant) DOM consistency the unify program targets.
        let html =
            "<!DOCTYPE html><html><head></head><body>\n  <p>Hi</p>\n  <p>Bye</p>\n</body></html>";
        let strict = parse_strict(html).expect("valid HTML5");
        let tolerant = parse_html(html);
        let sbody = find_tag(&strict.dom, strict.document, "body").expect("strict body");
        let tbody = find_tag(&tolerant.dom, tolerant.document, "body").expect("tolerant body");
        let s_kids = strict.dom.children_iter(sbody).count();
        let t_kids = tolerant.dom.children_iter(tbody).count();
        // body = ws("\n  ") <p>Hi</p> ws("\n  ") <p>Bye</p> ws("\n") = 5 children.
        assert_eq!(
            t_kids, 5,
            "tolerant keeps inter-element whitespace text nodes (§13.2.6)"
        );
        assert_eq!(
            s_kids, t_kids,
            "strict and tolerant agree on the whitespace-inclusive child set"
        );
        // Full-tree equivalence (tags / text incl. whitespace / order).
        assert_subtree_eq(
            &strict.dom,
            strict.document,
            &tolerant.dom,
            tolerant.document,
            "indented-conforming",
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

    #[test]
    fn parse_html_fragment_preserves_inter_element_whitespace() {
        // §11.3 whitespace unify, innerHTML path: `parse_html_fragment` shares
        // the `convert_node` text arm with document parsing, so removing the
        // tolerant over-strip preserves inter-element whitespace in fragments
        // too — the navigation(strict) ↔ innerHTML(tolerant) DOM consistency the
        // unify program targets. (innerHTML uses this fragment entry point.)
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let _ = dom.create_document_root();
        let added = parse_html_fragment(
            "<p>A</p>\n  <p>B</p>",
            "div",
            parent,
            &mut dom,
            ParseFragmentOptions {
                allow_declarative_shadow: false,
            },
        );
        // p ("A"), whitespace text ("\n  "), p ("B").
        assert_eq!(
            added.len(),
            3,
            "fragment keeps the inter-element whitespace text node"
        );
        let ws = dom
            .world()
            .get::<&TextContent>(added[1])
            .expect("middle node is the preserved whitespace text node");
        assert_eq!(ws.0, "\n  ", "the whitespace run is preserved verbatim");
    }

    #[test]
    fn fragment_custom_element_has_ce_state_when_insert_fires() {
        // Codex #329 R1 (P1) regression: the tolerant fragment path
        // (`innerHTML`) builds into a live, dispatcher-bound dom, so
        // `append_child` fires `MutationEvent::Insert` synchronously. The
        // CustomElementReactionConsumer reads `CustomElementState` at that
        // moment to enqueue upgrades; deriving components in a post-build
        // walk raced the insert and the marker was absent. `attach_derived`
        // now runs at element-creation time (before append), so the marker
        // is present when the insert fires.
        use elidex_ecs::{Attributes, EcsDom, MutationDispatcher, MutationEvent, TagType};
        use std::sync::{Arc, Mutex};

        struct InsertProbe(Arc<Mutex<Vec<(String, bool)>>>);
        impl MutationDispatcher for InsertProbe {
            fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
                if let MutationEvent::Insert { node, .. } = *event {
                    let tag = dom
                        .world()
                        .get::<&TagType>(node)
                        .map(|t| t.0.clone())
                        .unwrap_or_default();
                    let has_ce = dom
                        .world()
                        .get::<&elidex_custom_elements::CustomElementState>(node)
                        .is_ok();
                    self.0.lock().unwrap().push((tag, has_ce));
                }
            }
        }

        let mut dom = EcsDom::new();
        let _ = dom.create_document_root();
        let host = dom.create_element("div", Attributes::default());
        let log = Arc::new(Mutex::new(Vec::new()));
        let _ = dom.set_mutation_dispatcher(Box::new(InsertProbe(Arc::clone(&log))));
        let _ = parse_html_fragment(
            "<my-widget></my-widget>",
            "div",
            host,
            &mut dom,
            ParseFragmentOptions::default(),
        );
        let entries = log.lock().unwrap();
        let widget = entries
            .iter()
            .find(|(t, _)| t == "my-widget")
            .expect("an Insert event fired for <my-widget>");
        assert!(
            widget.1,
            "CustomElementState must be attached when the fragment insert fires (else upgrade is never enqueued)"
        );
    }

    #[test]
    fn fragment_declarative_shadow_content_gets_derived_components() {
        // Codex #329 R1 (P2) regression: declarative-shadow content consumed
        // by `try_attach_declarative_shadow` is not tracked in any root list,
        // so a post-build walk over fragment roots never visited it and the
        // shadow-tree elements lost CustomElementState / IframeData. Per-node
        // `attach_derived` in `convert_node` covers them (shadow content flows
        // through `convert_node` like any other node). `InlineStyle` is no
        // longer parser-derived — it materializes lazily on CSSOM access; the
        // `style` attribute is preserved here.
        use elidex_ecs::{Attributes, EcsDom, InlineStyle, TagType};

        let mut dom = EcsDom::new();
        let _ = dom.create_document_root();
        let host = dom.create_element("div", Attributes::default());
        let _ = parse_html_fragment(
            r#"<template shadowrootmode="open"><my-el style="color: red"></my-el></template>"#,
            "div",
            host,
            &mut dom,
            ParseFragmentOptions {
                allow_declarative_shadow: true,
            },
        );
        let sr = dom.get_shadow_root(host).expect("shadow root attached");
        let my_el = dom
            .children(sr)
            .into_iter()
            .find(|c| {
                dom.world()
                    .get::<&TagType>(*c)
                    .is_ok_and(|t| t.0 == "my-el")
            })
            .expect("<my-el> present in shadow tree");
        assert!(
            dom.world()
                .get::<&elidex_custom_elements::CustomElementState>(my_el)
                .is_ok(),
            "declarative-shadow custom element must carry CustomElementState"
        );
        assert!(
            dom.world().get::<&InlineStyle>(my_el).is_err(),
            "InlineStyle is not parser-derived (lazy CSSOM materialization)"
        );
        let attrs = dom.world().get::<&Attributes>(my_el).unwrap();
        assert_eq!(attrs.get("style"), Some("color: red"));
    }

    #[test]
    fn parse_strict_public_entry_derives_components() {
        // Codex #329 R3 (P2): the public `parse_strict` entry point must
        // derive components on its own (no caller-side derive), so a direct
        // strict parse of a custom element carries CustomElementState.
        let result = parse_strict(
            "<!DOCTYPE html><html><head></head><body><my-widget></my-widget></body></html>",
        )
        .expect("conforming HTML5");
        let widget = find_tag(&result.dom, result.document, "my-widget").expect("my-widget");
        assert!(
            result
                .dom
                .world()
                .get::<&elidex_custom_elements::CustomElementState>(widget)
                .is_ok(),
            "public parse_strict must attach CustomElementState without a caller-side derive"
        );
    }

    #[test]
    fn tolerant_foreign_content_not_marked_custom() {
        // Codex #329 R3 (P2): the tolerant backend must preserve foreign
        // namespaces so the HTML-namespace guard holds. An SVG-namespaced
        // <my-foo> parsed via `parse_html` must NOT receive CustomElementState
        // (custom element names are HTML-namespace-scoped, HTML §4.13.3).
        use elidex_ecs::Namespace;
        let result = parse_html("<svg><my-foo></my-foo></svg>");
        let my_foo = find_tag(&result.dom, result.document, "my-foo").expect("my-foo in svg");
        assert_eq!(
            result.dom.namespace_of(my_foo),
            Namespace::Svg,
            "tolerant path must preserve the SVG namespace"
        );
        assert!(
            result
                .dom
                .world()
                .get::<&elidex_custom_elements::CustomElementState>(my_foo)
                .is_err(),
            "foreign-namespace element must not be marked custom on the tolerant path"
        );
    }
}
