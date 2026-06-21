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
// `parse_strict`) yields a complete `ParseResult`. `parse_fragment_strict`
// stays the raw re-export — the fragment derived-component pass lives in the
// [`parse_fragment_progressive`] dispatcher (§11.3 slice 2b), which every
// fragment caller routes through, so the raw strict entry never reaches a
// caller un-derived.
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

/// Parse an HTML fragment string into **detached** child nodes — the tolerant
/// (§11.3 Tier-2) backend for the WHATWG HTML §13.4 fragment-parsing algorithm.
///
/// `context` is the fragment-context element; its tag and namespace select
/// html5ever's parsing rules (§13.4 step 10/15). Returns the fragment's
/// top-level nodes **detached** (parentless) in tree order (§13.4 step 20),
/// with their node document set to `context`'s owner document. `context` is
/// **not** mutated; the caller places the returned nodes.
///
/// Structurally mirrors the strict backend's `parse_fragment_strict`: build
/// under a synthetic throwaway document + `<html>` root, then return root's
/// children via the shared [`elidex_ecs::EcsDom::finish_detached_fragment`]
/// teardown — so a strict-first dispatcher ([`parse_fragment_progressive`])
/// routes either backend through one detached-return contract
/// (One-issue-one-way).
pub fn parse_html_fragment(
    html: &str,
    context: elidex_ecs::Entity,
    dom: &mut elidex_ecs::EcsDom,
    opts: ParseFragmentOptions,
) -> Vec<elidex_ecs::Entity> {
    use html5ever::{ns, LocalName, QualName};

    // §13.4 step 10/15: derive the context QualName (local name + namespace)
    // from the context entity. The unified detached-return signature carries
    // the entity, so the namespace is *derived*, not hardcoded to HTML — this
    // eliminates the old HTML-ns fork (One-issue-one-way). A context with no
    // `TagType` falls back to "div" (HTML §13.4 "Any other element" generic
    // context, the safe default).
    let local = dom.with_tag_name(context, |t| t.unwrap_or("div").to_owned());
    let context_ns = match dom.namespace_of(context) {
        elidex_ecs::Namespace::Svg => ns!(svg),
        elidex_ecs::Namespace::MathMl => ns!(mathml),
        elidex_ecs::Namespace::Html => ns!(html),
    };
    let context_name = QualName::new(None, context_ns, LocalName::from(local.as_str()));
    let rc_dom = html5ever::parse_fragment(
        RcDom::default(),
        ParseOpts::default(),
        context_name,
        Vec::new(),
        true, // scripting enabled (scriptingMode ≠ Disabled)
    )
    .one(html);

    // html5ever wraps the fragment children under a single synthetic `<html>`
    // element; unwrap to the handle whose children are the real fragment roots
    // (else use the document handle directly).
    let children_owner = {
        let roots = rc_dom.document.children.borrow();
        match roots.as_slice() {
            [only] => match &only.data {
                markup5ever_rcdom::NodeData::Element { name, .. }
                    if name.local.as_ref() == "html" =>
                {
                    only.clone()
                }
                _ => rc_dom.document.clone(),
            },
            _ => rc_dom.document.clone(),
        }
    };

    // Build the converted subtree under a synthetic throwaway document +
    // `<html>` root (the shared prologue both backends use; mutation dispatch is
    // suppressed for the throwaway build, restored after teardown), then return
    // root's children detached via the shared teardown.
    let (document, root, saved) = dom.begin_detached_fragment();
    // §13.4 fragment case (adjusted current node): while the stack holds only
    // the synthetic root — true for every *top-level* node — the adjusted
    // current node is the CONTEXT element, so a top-level
    // `<template shadowrootmode>` attaches its declarative shadow to `context`
    // (DSD-on-context, §13.2.6.4.4 step 9–10), mutating it as a side effect.
    // `convert_fragment_top_level` routes that host correctly; nested templates
    // attach to their real parent. (Strict rejects DSD-on-context and falls
    // back here — the slot `#11-strict-fragment-declarative-shadow-on-context`
    // is the strict-native handling, deferred; correctness is via this path.)
    convert::convert_fragment_top_level(&children_owner, root, context, dom, document, opts);
    let detached = dom.finish_detached_fragment(root, document, context);
    if let Some(dispatcher) = saved {
        dom.set_mutation_dispatcher(dispatcher);
    }
    detached
}

/// Parse an already-decoded HTML5 string with `§11.3` progressive
/// degradation — the `&str` twin of [`parse_progressive`].
///
/// Tries the strict (Tier-1) parser first: conforming HTML5 takes the fast
/// strict path ([`ParseTier::Clean`]). On the first strict parse error (a
/// WHATWG HTML `§13.2.2` parse error) it falls back to the tolerant
/// html5ever backend (Tier-2 rule-based recovery, [`ParseTier::Recovered`])
/// over the *same* text. This is the single try-strict-or-tolerant decision
/// site; [`parse_progressive`] (byte input) wraps it with charset detection.
///
/// Use this for whole-document input that is **already a `&str`** (no charset
/// step to run): the in-process shell pipeline (blank-tab markup, iframe
/// `srcdoc`, `about:blank`, tests). `encoding` stays `None` — the bare-`&str`
/// contract, matching [`parse_strict`] / [`parse_html`] (see
/// [`ParseResult::encoding`]).
///
/// The fallback is correctness-safe: the strict parser rejects
/// conservatively (no error recovery), so the worst case is the same tree the
/// tolerant backend would have produced on its own. Both arms attach the
/// parser-owned derived components (the local [`parse_strict`] wrapper runs
/// `derive_element_components` on success; the tolerant fallback derives inline
/// via `convert_node`), so the returned tree carries the CustomElementState /
/// IframeData components either way.
#[must_use]
pub fn parse_progressive_str(html: &str) -> ParseResult {
    parse_strict(html).unwrap_or_else(|_| parse_html(html))
}

/// Parse an HTML5 document from raw bytes with `§11.3` progressive
/// degradation (browser mode).
///
/// Detects the character encoding once, up front, then runs the str-input
/// strict-first dispatch [`parse_progressive_str`] over the decoded text (no
/// re-decode) and stamps the detected encoding. `charset_hint` is the HTTP
/// `Content-Type` header's `charset` parameter, if available. The resulting
/// [`ParseResult::tier`] records which tier ran, making the strict-vs-fallback
/// gradient observable (`§11.3`).
#[must_use]
pub fn parse_progressive(bytes: &[u8], charset_hint: Option<&str>) -> ParseResult {
    let decoded = detect_and_decode(bytes, charset_hint);
    let mut result = parse_progressive_str(&decoded.text);
    result.encoding = Some(decoded.encoding);
    result
}

/// Parse an HTML fragment with `§11.3` progressive degradation — the fragment
/// twin of [`parse_progressive`], and the single entry point every fragment
/// caller (innerHTML / outerHTML / insertAdjacentHTML) routes through.
///
/// Tries the strict (Tier-1) fragment parser first; conforming fragments take
/// the fast strict path. On the first WHATWG HTML `§13.2.2` parse error it
/// falls back to the tolerant html5ever backend ([`parse_html_fragment`]) over
/// an **uncontaminated** `dom` (the strict parser rolls back its partial
/// subtree on reject, leaving `dom` pristine — `parse_fragment_strict`'s
/// contract). Both backends honour the identical detached-return contract: the
/// fragment's top-level nodes come back parentless, adopted into `context`'s
/// owner document, with `context` itself unmodified — the caller places them.
///
/// `opts.allow_declarative_shadow` selects `innerHTML` (false) vs
/// `setHTMLUnsafe` (true) semantics.
#[must_use]
pub fn parse_fragment_progressive(
    html: &str,
    context: elidex_ecs::Entity,
    dom: &mut elidex_ecs::EcsDom,
    opts: ParseFragmentOptions,
) -> Vec<elidex_ecs::Entity> {
    match parse_fragment_strict(html, context, dom, opts) {
        Ok(roots) => {
            // The strict tree-builder crate is DOM-semantics-free, so the
            // parser-owned derived components (CustomElementState §4.13.3 /
            // IframeData §4.8.5) are attached here — the same
            // `derive_element_components` pass `parse_strict` runs for whole
            // documents (the tolerant backend derives inline via
            // `convert_node`). Each returned root is its own detached subtree,
            // so derive per root (the walker is root-inclusive).
            for &root in &roots {
                element_init::derive_element_components(dom, root);
            }
            roots
        }
        // Tier-2 fallback over the pristine dom; the tolerant path derives
        // inline during conversion.
        Err(_) => parse_html_fragment(html, context, dom, opts),
    }
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

    // --- str-input progressive dispatch (parse_progressive_str) ---

    #[test]
    fn progressive_str_clean_tier_for_conforming_html5() {
        // Conforming HTML5 takes the strict Tier-1 path; encoding stays None
        // on the bare-&str entry (no charset step runs).
        let html = "<!DOCTYPE html><html><head></head><body><p>Hi</p></body></html>";
        let result = parse_progressive_str(html);
        assert_eq!(result.tier, ParseTier::Clean);
        assert_eq!(result.encoding, None);
        assert!(result.errors.is_empty());
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "Hi");
    }

    #[test]
    fn progressive_str_broken_html_falls_back_to_recovered() {
        let result = parse_progressive_str("<div><span></div>");
        assert_eq!(result.tier, ParseTier::Recovered);
        assert!(!result.errors.is_empty());
        assert!(find_tag(&result.dom, result.document, "html").is_some());
    }

    #[test]
    fn parse_progressive_wraps_str_core_and_stamps_encoding() {
        // The byte entry routes through the str core: it must agree on
        // tier/tree/errors with `parse_progressive_str(&decoded.text)` (so a
        // regression that bypassed the strict-first dispatch — e.g. calling
        // `parse_html` directly — would diverge on tier for this conforming
        // input) and add ONLY the encoding stamp on top. The deep
        // strict-vs-tolerant tree equivalence itself is pinned by the separate
        // `strict_and_tolerant_agree_*` differential tests, not here.
        let bytes = b"<!DOCTYPE html><html><head></head><body><p>x</p></body></html>";
        let prog = parse_progressive(bytes, None);
        let decoded = detect_and_decode(bytes, None);
        let core = parse_progressive_str(&decoded.text);
        assert_eq!(prog.tier, core.tier);
        assert_eq!(prog.errors, core.errors);
        assert_eq!(prog.encoding, Some("UTF-8"));
        assert_eq!(core.encoding, None);
        assert_subtree_eq(&prog.dom, prog.document, &core.dom, core.document, "root");
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
    fn fragment_top_level_declarative_shadow_attaches_to_context() {
        // §13.4 fragment case + §13.2.6.4.4 step 9–10: while the stack holds
        // only the synthetic root, the adjusted current node is the CONTEXT
        // element — so a *top-level* `<template shadowrootmode>` attaches its
        // declarative shadow to the context (DSD-on-context). The template is
        // consumed (no light-tree node returned); the context gains a shadow
        // root holding the parsed <p>. (`el.setHTMLUnsafe('<template
        // shadowrootmode=open>…')` shadows `el`.)
        let mut dom = EcsDom::new();
        let context = dom.create_element("div", Attributes::default());
        let _ = dom.create_document_root();
        let added = parse_html_fragment(
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            context,
            &mut dom,
            ParseFragmentOptions {
                allow_declarative_shadow: true,
            },
        );
        assert!(
            added.is_empty(),
            "the declarative-shadow template is consumed; no light-tree node returned, got tags {:?}",
            added
                .iter()
                .map(|&e| dom.world().get::<&TagType>(e).map(|t| t.0.clone()).ok())
                .collect::<Vec<_>>()
        );
        let sr = dom
            .get_shadow_root(context)
            .expect("a top-level declarative-shadow template attaches a shadow to the context");
        assert!(
            dom.children(sr)
                .iter()
                .any(|c| dom.world().get::<&TagType>(*c).is_ok_and(|t| t.0 == "p")),
            "the context's shadow tree holds the parsed <p>"
        );
    }

    #[test]
    fn fragment_nested_declarative_shadow_attaches_to_inner_host() {
        // §13.2.6.4.4 step 10: a *nested* `<template shadowrootmode>` has a
        // real (non-topmost) host as its adjusted current node, so the shadow
        // attaches there. Here the inner `<div>` hosts the shadow holding <p>.
        let mut dom = EcsDom::new();
        let context = dom.create_element("body", Attributes::default());
        let _ = dom.create_document_root();
        let added = parse_html_fragment(
            r#"<div><template shadowrootmode="open"><p>x</p></template></div>"#,
            context,
            &mut dom,
            ParseFragmentOptions {
                allow_declarative_shadow: true,
            },
        );
        let outer_div = *added
            .iter()
            .find(|&&e| dom.world().get::<&TagType>(e).is_ok_and(|t| t.0 == "div"))
            .expect("the outer <div> is returned");
        let sr = dom
            .get_shadow_root(outer_div)
            .expect("shadow root must attach to the inner div host");
        let p_present = dom.children(sr).iter().any(|c| {
            dom.world()
                .get::<&elidex_ecs::TagType>(*c)
                .is_ok_and(|t| t.0 == "p")
        });
        assert!(p_present, "the inner div's shadow tree holds the <p>");
    }

    #[test]
    fn parse_html_fragment_preserves_inter_element_whitespace() {
        // §11.3 whitespace unify, innerHTML path: `parse_html_fragment` shares
        // the `convert_node` text arm with document parsing, so removing the
        // tolerant over-strip preserves inter-element whitespace in fragments
        // too — the navigation(strict) ↔ innerHTML(tolerant) DOM consistency the
        // unify program targets. (innerHTML uses this fragment entry point.)
        let mut dom = EcsDom::new();
        let context = dom.create_element("div", Attributes::default());
        let _ = dom.create_document_root();
        let added = parse_html_fragment(
            "<p>A</p>\n  <p>B</p>",
            context,
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
    fn fragment_returned_custom_element_carries_ce_state() {
        // Codex #329 R1 (P1), re-homed for §11.3 slice 2b: the fragment parser
        // now builds in isolation (mutation dispatch suppressed) and returns
        // DETACHED nodes — the real `MutationEvent::Insert` fires when the
        // caller places them, not during the parse. The #329 invariant ("CE
        // state present when the insert fires") therefore holds structurally:
        // derivation (tolerant: inline in `convert_node`) completes before the
        // nodes are returned, so a returned custom element already carries
        // `CustomElementState` and any later placement insert sees it. (The
        // end-to-end caller-fires-insert path is covered in
        // `elidex-script-session` `apply_set_inner_html` tests.)
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let context = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, context);
        let added = parse_html_fragment(
            "<my-widget></my-widget>",
            context,
            &mut dom,
            ParseFragmentOptions::default(),
        );
        let widget = *added
            .iter()
            .find(|&&e| {
                dom.world()
                    .get::<&TagType>(e)
                    .is_ok_and(|t| t.0 == "my-widget")
            })
            .expect("the <my-widget> custom element is returned");
        assert!(
            dom.world()
                .get::<&elidex_custom_elements::CustomElementState>(widget)
                .is_ok(),
            "returned custom element carries CustomElementState (so a placement insert enqueues its upgrade)"
        );
    }

    #[test]
    fn fragment_declarative_shadow_content_gets_derived_components() {
        // Codex #329 R1 (P2) regression: declarative-shadow content consumed
        // by `try_attach_declarative_shadow` is not tracked in any root list,
        // so a post-build walk over fragment roots never visited it and the
        // shadow-tree elements lost CustomElementState / IframeData. Per-node
        // `attach_derived` in `convert_node` covers them (shadow content flows
        // through `convert_node` like any other node). Uses a *nested*
        // declarative shadow (slice 2b: top-level templates no longer attach),
        // so the shadow lives on the inner `<div>`. `InlineStyle` is no longer
        // parser-derived — it materializes lazily on CSSOM access; the `style`
        // attribute is preserved here.
        use elidex_ecs::{Attributes, EcsDom, InlineStyle, TagType};

        let mut dom = EcsDom::new();
        let _ = dom.create_document_root();
        let context = dom.create_element("body", Attributes::default());
        let added = parse_html_fragment(
            r#"<div><template shadowrootmode="open"><my-el style="color: red"></my-el></template></div>"#,
            context,
            &mut dom,
            ParseFragmentOptions {
                allow_declarative_shadow: true,
            },
        );
        let outer_div = *added
            .iter()
            .find(|&&e| dom.world().get::<&TagType>(e).is_ok_and(|t| t.0 == "div"))
            .expect("outer <div> returned");
        let sr = dom
            .get_shadow_root(outer_div)
            .expect("shadow root attached to inner div");
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
    fn fragment_progressive_strict_first_then_tolerant_fallback() {
        // §11.3 slice 2b dispatcher: a conforming fragment takes the strict
        // (Tier-1) path; a malformed one (misnested close) makes strict reject
        // and falls back to the tolerant (Tier-2) backend. Both return detached
        // nodes through the single `parse_fragment_progressive` entry.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let context = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, context);

        let clean = parse_fragment_progressive(
            "<p>ok</p>",
            context,
            &mut dom,
            ParseFragmentOptions::default(),
        );
        assert_eq!(clean.len(), 1, "strict path returns the <p>");

        let recovered = parse_fragment_progressive(
            "<div><span></div>",
            context,
            &mut dom,
            ParseFragmentOptions::default(),
        );
        assert!(
            !recovered.is_empty(),
            "tolerant fallback still builds a tree from malformed markup"
        );
        assert!(
            recovered.iter().all(|&n| dom.get_parent(n).is_none()),
            "fallback nodes are detached too"
        );
    }

    #[test]
    fn fragment_progressive_returns_detached_nodes_owned_by_context_document() {
        // Detached-return contract: returned nodes are parentless, their node
        // document is the context's owner document (via the shared adopt), and
        // the context itself is unmutated.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let context = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, context);
        let nodes = parse_fragment_progressive(
            "<p>x</p><span>y</span>",
            context,
            &mut dom,
            ParseFragmentOptions::default(),
        );
        assert_eq!(nodes.len(), 2);
        for &n in &nodes {
            assert!(dom.get_parent(n).is_none(), "returned node is detached");
            assert_eq!(
                dom.owner_document(n),
                Some(doc),
                "returned node adopts the context's owner document"
            );
        }
        assert!(
            dom.children(context).is_empty(),
            "context is not mutated by the parse"
        );
    }

    #[test]
    fn fragment_tolerant_derives_foreign_context_namespace() {
        // §11.3 slice 2b (F4 determination): the tolerant backend derives the
        // html5ever context `QualName` namespace from the context entity (no
        // hardcoded HTML namespace). An SVG context parses its children as SVG
        // foreign content. If html5ever could not parse a foreign fragment
        // context this assertion would fail and the HTML-ns contingency slot
        // `#11-tolerant-fragment-foreign-context-namespace` would be carved.
        use elidex_ecs::Namespace;
        let mut dom = EcsDom::new();
        let _ = dom.create_document_root();
        let context = dom.create_element("svg", Attributes::default());
        let _ = dom.world_mut().insert_one(context, Namespace::Svg);
        let nodes = parse_html_fragment(
            "<rect></rect>",
            context,
            &mut dom,
            ParseFragmentOptions::default(),
        );
        let rect = nodes
            .iter()
            .copied()
            .find(|&e| dom.world().get::<&TagType>(e).is_ok_and(|t| t.0 == "rect"));
        assert!(rect.is_some(), "svg-context fragment produces a <rect>");
        assert_eq!(
            dom.namespace_of(rect.unwrap()),
            Namespace::Svg,
            "rect parses as SVG foreign content under the derived context namespace"
        );
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
