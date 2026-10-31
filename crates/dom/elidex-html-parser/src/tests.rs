//! Unit tests for the `elidex-html-parser` crate root (extracted from the
//! inline `#[cfg(test)] mod tests` to keep `lib.rs` under the 1000-line
//! convention).

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
        let strict =
            parse_strict(html).unwrap_or_else(|e| panic!("case {i} should be valid HTML5: {e}"));
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

const DSD_OPEN: ParseFragmentOptions = ParseFragmentOptions {
    allow_declarative_shadow: true,
    scripting_disabled: false,
};

/// Canonical text dump of a node for backend-parity comparison: tag, the
/// shadow root (mode + children) and the template contents (children) are
/// included, which `node_sig` does not see.
fn dump_with_shadow_and_content(dom: &EcsDom, e: Entity) -> String {
    let w = dom.world();
    if let Ok(t) = w.get::<&TextContent>(e) {
        return format!("{:?}", t.0);
    }
    let tag = w
        .get::<&TagType>(e)
        .map_or_else(|_| "?".to_string(), |t| t.0.clone());
    let mut out = format!("<{tag} {:?}", attr_map(dom, e));
    if let Some(sr) = dom.get_shadow_root(e) {
        let mode = w
            .get::<&elidex_ecs::ShadowRoot>(sr)
            .map_or("?", |s| match s.mode {
                elidex_ecs::ShadowRootMode::Open => "open",
                elidex_ecs::ShadowRootMode::Closed => "closed",
            });
        out.push_str(" #shadow(");
        out.push_str(mode);
        out.push_str(")[");
        for c in dom.children(sr) {
            out += &dump_with_shadow_and_content(dom, c);
        }
        out += "]";
    }
    if let Some(content) = dom.template_contents_fragment(e) {
        out += " #content[";
        for c in dom.children(content) {
            out += &dump_with_shadow_and_content(dom, c);
        }
        out += "]";
    }
    out += ">(";
    for c in dom.children(e) {
        out += &dump_with_shadow_and_content(dom, c);
    }
    out += ")";
    out
}

/// Parse `html` into a fresh `<div>` context with both backends and return
/// `(strict_dump, tolerant_dump)` of the returned roots. Panics if strict
/// rejects (the inputs are conforming) or the context gains a shadow root.
fn dsd_both_backends(html: &str) -> (String, String) {
    let run = |strict: bool| {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let context = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, context);
        let roots = if strict {
            parse_fragment_strict(html, context, &mut dom, DSD_OPEN).expect("strict parses")
        } else {
            parse_html_fragment(html, context, &mut dom, DSD_OPEN)
        };
        assert!(
            dom.get_shadow_root(context).is_none(),
            "the context never gains a shadow root (strict={strict})"
        );
        roots
            .iter()
            .map(|&r| dump_with_shadow_and_content(&dom, r))
            .collect::<String>()
    };
    (run(true), run(false))
}

#[test]
fn fragment_top_level_declarative_shadow_is_plain_template() {
    // The template start tag (§13.2.6.4.4): declarativeShadowHostElement is
    // the current node, not the adjusted current node ("so that in the
    // fragment case a top-level template element never attaches a shadow root
    // to the parser's fragment context element"), and hostIsRoot (the
    // fragment root also has a redirection-map entry) makes it a regular
    // template. So a *top-level* `<template shadowrootmode>` is returned as a
    // plain template whose content holds the <p>, and the context is never
    // mutated.
    let mut dom = EcsDom::new();
    let context = dom.create_element("div", Attributes::default());
    let _ = dom.create_document_root();
    let added = parse_html_fragment(
        r#"<template shadowrootmode="open"><p>x</p></template>"#,
        context,
        &mut dom,
        DSD_OPEN,
    );
    // Discriminator: the context has no shadow root.
    assert!(
        dom.get_shadow_root(context).is_none(),
        "a top-level declarative template never attaches a shadow to the context"
    );
    // Non-discriminating (true before and after): the context's light
    // children are untouched.
    assert!(dom.children(context).is_empty());
    assert_eq!(added.len(), 1, "one plain template is returned");
    let template = added[0];
    assert!(dom
        .world()
        .get::<&TagType>(template)
        .is_ok_and(|t| t.0 == "template"));
    let tpl_content = dom
        .template_contents_fragment(template)
        .expect("a plain template has a content fragment");
    assert!(
        dom.children(tpl_content)
            .iter()
            .any(|c| dom.world().get::<&TagType>(*c).is_ok_and(|t| t.0 == "p")),
        "the template content holds the parsed <p>"
    );
}

#[test]
fn fragment_nested_declarative_shadow_attaches_to_inner_host() {
    // The template start tag (§13.2.6.4.4): a *nested* `<template
    // shadowrootmode>` has a real (non-topmost) current node as its
    // declarativeShadowHostElement, so hostIsRoot is false and the shadow
    // attaches there. Here the inner `<div>` hosts the shadow holding <p>.
    let mut dom = EcsDom::new();
    let context = dom.create_element("body", Attributes::default());
    let _ = dom.create_document_root();
    let added = parse_html_fragment(
        r#"<div><template shadowrootmode="open"><p>x</p></template></div>"#,
        context,
        &mut dom,
        DSD_OPEN,
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
fn fragment_progressive_top_level_declarative_shadow_strict_native() {
    // Strict handles a top-level declarative template natively (no decline):
    // `parse_fragment_strict` returns `Ok` and the same tree as the tolerant
    // backend (a plain template, context unmutated).
    let html = r#"<template shadowrootmode="open"><p>x</p></template>"#;
    let (strict, tolerant) = dsd_both_backends(html);
    assert_eq!(strict, tolerant);
    assert!(strict.starts_with("<template"), "got {strict}");
    assert!(strict.contains("#content[<p"), "got {strict}");
    assert!(!strict.contains("#shadow"), "got {strict}");
}

#[test]
fn fragment_dsd_parity_template_in_template() {
    // Outer template is plain (hostIsRoot); the inner template's current node
    // is the outer `template` (not a valid shadow host name), so the attach
    // fails and the inner one is a plain template inside the outer content.
    let html = r#"<template><template shadowrootmode="open"><p>x</p></template></template>"#;
    let (strict, tolerant) = dsd_both_backends(html);
    assert_eq!(strict, tolerant);
    assert!(!strict.contains("#shadow"), "got {strict}");
    assert_eq!(strict.matches("#content[").count(), 2, "got {strict}");
}

#[test]
fn fragment_dsd_parity_nested_host_in_top_level_template() {
    // The outer top-level template is plain; inside its content the `<div>`
    // is a real host, so both backends give it an open shadow holding <p>.
    let html =
        r#"<template><div><template shadowrootmode="open"><p>x</p></template></div></template>"#;
    let (strict, tolerant) = dsd_both_backends(html);
    assert_eq!(strict, tolerant);
    assert!(strict.contains("#shadow(open)[<p"), "got {strict}");
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
            scripting_disabled: false,
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
fn template_content_custom_element_derived_strict_and_tolerant() {
    // Codex PR380 (P1): a custom element inside `<template>` content must
    // receive `CustomElementState` in BOTH tiers. The strict derivation
    // post-pass (`derive_element_components`) must follow the detached
    // `TemplateContents` fragment to match the tolerant inline derivation —
    // else strict-parsed template content loses CE state (parity break;
    // later breaks clone/upgrade for those nodes).
    let html = "<!DOCTYPE html><template><my-widget></my-widget></template>";
    for (tier, result) in [
        ("tolerant", parse_html(html)),
        ("strict", parse_strict(html).expect("strict parse")),
    ] {
        let dom = &result.dom;
        let template = find_tag(dom, result.document, "template").expect("template");
        let fragment = dom
            .template_contents_fragment(template)
            .expect("content fragment");
        let widget = dom
            .children(fragment)
            .into_iter()
            .find(|&e| {
                dom.world()
                    .get::<&TagType>(e)
                    .is_ok_and(|t| t.0 == "my-widget")
            })
            .unwrap_or_else(|| panic!("{tier}: <my-widget> present in template content"));
        assert!(
            dom.world()
                .get::<&elidex_custom_elements::CustomElementState>(widget)
                .is_ok(),
            "{tier}: custom element in <template> content carries CustomElementState"
        );
    }
}

#[test]
fn template_content_descendant_owner_document() {
    // Codex PR380 (P2): a parsed `<template>` content descendant inherits
    // the fragment's node document (owner_document falls back through the
    // DocumentFragment root), so `content.firstChild.ownerDocument` is the
    // document, matching `content.ownerDocument` — not null.
    let result = parse_html("<template><p>x</p></template>");
    let dom = &result.dom;
    let template = find_tag(dom, result.document, "template").expect("template");
    let fragment = dom
        .template_contents_fragment(template)
        .expect("content fragment");
    let p = dom
        .children(fragment)
        .into_iter()
        .find(|&e| dom.world().get::<&TagType>(e).is_ok_and(|t| t.0 == "p"))
        .expect("<p> present in template content");
    assert_eq!(
        dom.owner_document(p),
        Some(result.document),
        "template content descendant ownerDocument resolves to the document, not null"
    );
    assert_eq!(
        dom.owner_document(fragment),
        Some(result.document),
        "the content fragment's own ownerDocument is the document (C2 approximation)"
    );
}

#[test]
fn fragment_parsed_template_content_owner_is_context_document() {
    // Codex PR380 R2 (P2): fragment-parsed `<template>` content fragments
    // are stamped with the *context's* node document, not the throwaway
    // fragment document (which is destroyed by `finish_detached_fragment`) —
    // otherwise the content's ownerDocument dangles.
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let context = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, context);
    let added = parse_html_fragment(
        "<template><p>x</p></template>",
        context,
        &mut dom,
        ParseFragmentOptions::default(),
    );
    let template = *added
        .iter()
        .find(|&&e| {
            dom.world()
                .get::<&TagType>(e)
                .is_ok_and(|t| t.0 == "template")
        })
        .expect("the <template> is returned");
    let fragment = dom
        .template_contents_fragment(template)
        .expect("content fragment");
    assert_eq!(
        dom.owner_document(fragment),
        Some(doc),
        "fragment-parsed template content fragment ownerDocument is the context document"
    );
    let p = dom
        .children(fragment)
        .into_iter()
        .find(|&e| dom.world().get::<&TagType>(e).is_ok_and(|t| t.0 == "p"))
        .expect("<p> in content");
    assert_eq!(
        dom.owner_document(p),
        Some(doc),
        "content descendant inherits the context document, not a dangling throwaway doc"
    );
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
        DSD_OPEN,
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
