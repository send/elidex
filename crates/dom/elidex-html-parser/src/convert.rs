//! `RcDom` → `EcsDom` conversion.
//!
//! Two-pass approach: html5ever parses into `RcDom`, then this module
//! walks the tree and builds an ECS DOM.

use elidex_ecs::{
    Attributes, EcsDom, Entity, Namespace, ShadowInit, ShadowRootMode, SlotAssignmentMode,
};
use html5ever::ns;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use elidex_html_parser_strict::{ParseFragmentOptions, ParseResult, ParseTier};

use crate::element_init::attach_derived;

pub(crate) fn convert_document(rc_dom: RcDom) -> ParseResult {
    let mut dom = EcsDom::new();
    let document = dom.create_document_root();
    convert_children(
        &rc_dom.document,
        document,
        &mut dom,
        document,
        ParseFragmentOptions::default(),
    );
    let errors = rc_dom
        .errors
        .into_inner()
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    ParseResult {
        dom,
        document,
        errors,
        encoding: None,
        // The tolerant html5ever backend is the §11.3 Tier-2 rule-based
        // recovery handler, so every tree it produces is `Recovered` —
        // independent of whether recovery rules actually fired (see
        // `ParseTier`). `parse_progressive` returns this verbatim on the
        // strict-parse-error fallback path.
        tier: ParseTier::Recovered,
    }
}

/// Convert an `RcDom` handle's children into the ECS DOM under `parent`,
/// attaching each child to `parent` (the real element node).
///
/// Used by whole-document conversion ([`convert_document`], parent = the
/// element being filled) and, for **nested** fragment children, by
/// [`convert_node`] (parent = the converted element). A `<template
/// shadowrootmode>` child attaches a declarative shadow to `parent` when
/// `opts.allow_declarative_shadow` is set and `parent` is a valid shadow host
/// (§13.2.6.4.4 step 9–10) — for nested content `parent` is the real
/// (non-topmost) host, so it attaches there. The fragment **top level** is
/// handled separately by [`convert_fragment_top_level`], which routes the
/// declarative-shadow host to the context element per the §13.4
/// adjusted-current-node rule.
pub(crate) fn convert_children(
    rc_handle: &Handle,
    parent: Entity,
    dom: &mut EcsDom,
    owner_document: Entity,
    opts: ParseFragmentOptions,
) {
    for child in &*rc_handle.children.borrow() {
        if opts.allow_declarative_shadow
            && try_attach_declarative_shadow(child, parent, dom, owner_document, opts)
        {
            // Template was consumed as declarative shadow root markup;
            // the host entity already received the new shadow tree.
            continue;
        }
        if let Some(entity) = convert_node(child, dom, owner_document, opts) {
            let ok = dom.append_child(parent, entity);
            debug_assert!(ok, "append_child failed during RcDom conversion");
        }
    }
}

/// Convert the **top-level** fragment children under the synthetic `<html>`
/// `root`, with the §13.4 fragment-case declarative-shadow host routing.
///
/// Per the WHATWG "adjusted current node" definition, while the stack of open
/// elements holds only the synthetic root (the fragment case, true for every
/// *top-level* node), the adjusted current node is the **context element**, not
/// the root. So a top-level `<template shadowrootmode>` attaches its
/// declarative shadow to `context` (DSD-on-context, §13.2.6.4.4 step 9–10) —
/// e.g. `el.setHTMLUnsafe('<template shadowrootmode=open>…')` shadows `el`. The
/// light (non-shadow) top-level nodes are still built under `root` and returned
/// detached. Nested templates recurse through [`convert_node`] →
/// [`convert_children`], where the host is the real (non-topmost) parent
/// element, so they attach there as usual.
pub(crate) fn convert_fragment_top_level(
    rc_handle: &Handle,
    root: Entity,
    context: Entity,
    dom: &mut EcsDom,
    owner_document: Entity,
    opts: ParseFragmentOptions,
) {
    for child in &*rc_handle.children.borrow() {
        // Top-level declarative shadow attaches to the CONTEXT (the fragment
        // adjusted current node), not the synthetic root.
        if opts.allow_declarative_shadow
            && try_attach_declarative_shadow(child, context, dom, owner_document, opts)
        {
            continue;
        }
        if let Some(entity) = convert_node(child, dom, owner_document, opts) {
            let ok = dom.append_child(root, entity);
            debug_assert!(ok, "append_child failed during RcDom conversion");
        }
    }
}

/// HTML §4.12.3 `<template shadowrootmode>` declarative shadow DOM hook.
///
/// When the parser encounters a `<template shadowrootmode="open|closed">`
/// child of `parent`, attach a shadow root to `parent` and route the
/// template's content into the new shadow tree. Returns `true` when the
/// template was consumed (caller must skip the standard element-creation
/// path); returns `false` when the child is not a declarative shadow
/// template, when the attach was rejected (host tag not allowed, host
/// already has a shadow root — silent fallback per spec), or when the
/// `shadowrootmode` value is unrecognised.
fn try_attach_declarative_shadow(
    rc_handle: &Handle,
    parent: Entity,
    dom: &mut EcsDom,
    owner_document: Entity,
    opts: ParseFragmentOptions,
) -> bool {
    let NodeData::Element {
        name,
        attrs,
        template_contents,
        ..
    } = &rc_handle.data
    else {
        return false;
    };
    if name.local.as_ref() != "template" {
        return false;
    }
    let attrs = attrs.borrow();
    let mode_attr = attrs
        .iter()
        .find(|a| a.name.local.as_ref() == "shadowrootmode")
        .map(|a| a.value.as_ref());
    let Some(mode_str) = mode_attr else {
        return false;
    };
    // HTML §2.3.9: enumerated attribute values are ASCII-case-insensitive.
    let mode = if mode_str.eq_ignore_ascii_case("open") {
        ShadowRootMode::Open
    } else if mode_str.eq_ignore_ascii_case("closed") {
        ShadowRootMode::Closed
    } else {
        return false;
    };
    let delegates_focus = attrs
        .iter()
        .any(|a| a.name.local.as_ref() == "shadowrootdelegatesfocus");
    let clonable = attrs
        .iter()
        .any(|a| a.name.local.as_ref() == "shadowrootclonable");
    let serializable = attrs
        .iter()
        .any(|a| a.name.local.as_ref() == "shadowrootserializable");
    let slot_assignment = attrs
        .iter()
        .find(|a| a.name.local.as_ref() == "shadowrootslotassignment")
        .and_then(|a| {
            if a.value.eq_ignore_ascii_case("manual") {
                Some(SlotAssignmentMode::Manual)
            } else if a.value.eq_ignore_ascii_case("named") {
                Some(SlotAssignmentMode::Named)
            } else {
                None
            }
        })
        .unwrap_or_default();
    drop(attrs);
    let init = ShadowInit {
        mode,
        delegates_focus,
        slot_assignment,
        clonable,
        serializable,
        // Declarative shadow roots have no customElementRegistry
        // channel — always the document registry.
        null_registry: false,
    };
    // Spec §4.12.3 silently leaves the template as a normal element when
    // attach fails (e.g. parent tag not allowlisted, or parent already has
    // a shadow root from an earlier declarative template). Returning false
    // routes the caller into the standard element-creation path.
    let Ok(shadow_root) = dom.attach_shadow_with_init(parent, init) else {
        return false;
    };
    // The template's content lives in `template_contents` (a
    // DocumentFragment-like handle), not its direct `children`, per
    // html5ever's RcDom representation.
    if let Some(contents) = template_contents.borrow().clone() {
        convert_children(&contents, shadow_root, dom, owner_document, opts);
    }
    true
}

/// Build the tag name and attribute set from an element handle.
///
/// Returns `(tag, namespace, attributes)`. The namespace is read from the
/// html5ever [`QualName`](html5ever::QualName) so foreign (SVG / MathML)
/// content is created with the correct [`Namespace`] component — without it
/// `EcsDom::namespace_of` would default every node to HTML, defeating the
/// HTML-namespace guard in `element_init::attach_derived` (a `<svg><my-foo>`
/// would wrongly receive `CustomElementState`).
///
/// Derived components (CustomElementState / IframeData) are attached by
/// `element_init::attach_derived`, invoked per-node from
/// [`convert_node`] at creation time (shared with the strict Tier-1 backend's
/// derivation logic).
fn build_element_data(handle: &Handle) -> Option<(String, Namespace, Attributes)> {
    let NodeData::Element { name, attrs, .. } = &handle.data else {
        return None;
    };
    let tag = name.local.as_ref().to_string();
    let namespace = if name.ns == ns!(svg) {
        Namespace::Svg
    } else if name.ns == ns!(mathml) {
        Namespace::MathMl
    } else {
        // ns!(html) and any unexpected namespace map to HTML.
        Namespace::Html
    };
    let mut attributes = Attributes::default();
    for attr in attrs.borrow().iter() {
        attributes.set(attr.name.local.as_ref(), &*attr.value);
    }
    Some((tag, namespace, attributes))
}

fn convert_node(
    handle: &Handle,
    dom: &mut EcsDom,
    owner_document: Entity,
    opts: ParseFragmentOptions,
) -> Option<Entity> {
    match &handle.data {
        NodeData::Element {
            template_contents, ..
        } => {
            let (tag, namespace, attributes) = build_element_data(handle)?;
            // `create_element_ns` attaches a `Namespace` component only for
            // non-HTML namespaces (HTML stays component-free), so the foreign
            // guard in `attach_derived` sees the real namespace.
            let entity = dom.create_element_ns(&tag, namespace, attributes, None);
            // Attach derived components at creation time — BEFORE the element
            // is appended anywhere — so the tolerant backend's output always
            // carries `CustomElementState` / `IframeData` whether it feeds
            // whole-document conversion (`convert_document`) or the fragment
            // path (`convert_children` under a synthetic root, e.g.
            // `innerHTML`). The fragment build is dispatch-suppressed and
            // returns DETACHED nodes, so deriving here guarantees the
            // components are present when the *caller* later places those nodes
            // and the placement `MutationEvent::Insert` fires (the
            // CustomElementReactionConsumer reads them then). Per-node creation
            // also covers declarative-shadow content, which no root-list walk
            // would reach. The strict Tier-1 backend instead derives per
            // returned root in `parse_fragment_progressive` / per document in
            // `parse_strict` (it is DOM-semantics-free and cannot reach this
            // crate's deps). All paths share the one `attach_derived` impl.
            attach_derived(dom, entity);
            // HTML §4.12.3: an HTML `<template>` holds its children in a
            // detached content `DocumentFragment`, not as light children.
            // html5ever stores those children in `template_contents` (its RcDom
            // DocumentFragment-like handle), separate from `children` (empty for
            // a template); recurse them into the content fragment built by the
            // shared `attach_template_contents` helper (the same one the strict
            // tier + createElement use — cross-tier One-issue-one-way). A
            // declarative-shadow `<template>` was already consumed by
            // `try_attach_declarative_shadow` in the caller, so reaching here
            // means an ordinary template — including a declarative one whose
            // attach was rejected, which becomes ordinary per §4.12.3 and whose
            // stored `template_contents` is its template content.
            if namespace == Namespace::Html && tag == "template" {
                let fragment = dom.attach_template_contents(entity, Some(owner_document));
                if let Some(contents) = template_contents.borrow().clone() {
                    convert_children(&contents, fragment, dom, owner_document, opts);
                }
            } else {
                convert_children(handle, entity, dom, owner_document, opts);
            }
            Some(entity)
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().to_string();
            // Preserve whitespace-only text nodes: html5ever already places inter-element
            // whitespace per WHATWG HTML §13.2.6 (framing whitespace it drops itself).
            // Dropping whitespace-only nodes here was an elidex-specific over-deletion that
            // diverged the tolerant path from the strict backend (§11.3 whitespace unify).
            // The residual empty-string ("") drop keeps the two backends aligned: the strict
            // parser inserts characters one at a time and so never creates a zero-length text
            // node either, so dropping one here matches it (this is not a whitespace strip).
            if text.is_empty() {
                return None;
            }
            Some(dom.create_text(text))
        }
        NodeData::Comment { contents } => {
            let data = contents.to_string();
            Some(dom.create_comment(data))
        }
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => {
            let entity = dom.create_document_type(
                name.to_string(),
                public_id.to_string(),
                system_id.to_string(),
            );
            Some(entity)
        }
        // ProcessingInstruction, Document — skip
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{child_text, find_tag};
    use crate::{parse_html, parse_strict};
    use elidex_ecs::{InlineStyle, TagType};

    /// Serialize a subtree (tag + sorted attrs + text, recursively) into a
    /// tier-independent string for the cross-tier parity AC.
    fn serialize_subtree(dom: &EcsDom, entity: Entity, out: &mut String) {
        use std::fmt::Write as _;
        if let Ok(t) = dom.world().get::<&TagType>(entity) {
            out.push('<');
            out.push_str(&t.0);
            let mut attrs: Vec<(String, String)> = dom
                .world()
                .get::<&elidex_ecs::Attributes>(entity)
                .map(|a| {
                    a.iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            attrs.sort();
            for (k, v) in attrs {
                let _ = write!(out, " {k}=\"{v}\"");
            }
            out.push('>');
            for child in dom.children(entity) {
                serialize_subtree(dom, child, out);
            }
            out.push_str("</>");
        } else if let Ok(tc) = dom.world().get::<&elidex_ecs::TextContent>(entity) {
            let _ = write!(out, "\"{}\"", tc.0);
        }
    }

    #[test]
    fn basic_div_with_text() {
        let result = parse_html("<div>Hello</div>");
        let dom = &result.dom;
        let doc = result.document;

        // document → html → body → div → text("Hello")
        let html = find_tag(dom, doc, "html").expect("html");
        let body = find_tag(dom, html, "body").expect("body");
        let div = find_tag(dom, body, "div").expect("div");
        assert_eq!(child_text(dom, div), "Hello");
    }

    #[test]
    fn nested_elements() {
        let result = parse_html("<div><p><span>text</span></p></div>");
        let dom = &result.dom;
        let doc = result.document;

        let html = find_tag(dom, doc, "html").expect("html");
        let body = find_tag(dom, html, "body").expect("body");
        let div = find_tag(dom, body, "div").expect("div");
        let p = find_tag(dom, div, "p").expect("p");
        let span = find_tag(dom, p, "span").expect("span");
        assert_eq!(child_text(dom, span), "text");
    }

    #[test]
    fn attributes_preserved() {
        let result = parse_html(r#"<a href="https://example.com" class="link">text</a>"#);
        let dom = &result.dom;
        let doc = result.document;

        let a = find_tag(dom, doc, "a").expect("a");
        let attrs = dom.world().get::<&Attributes>(a).unwrap();
        assert_eq!(attrs.get("href"), Some("https://example.com"));
        assert_eq!(attrs.get("class"), Some("link"));
    }

    #[test]
    fn inline_style_parsed() {
        let result = parse_html(r#"<div style="color: red; margin: 10px"></div>"#);
        let dom = &result.dom;
        let doc = result.document;

        let div = find_tag(dom, doc, "div").expect("div");
        // The parser does NOT attach `InlineStyle` — the style attribute
        // is preserved verbatim (the cascade reads it directly; the CSSOM
        // `InlineStyle` component materializes lazily in elidex-dom-api on
        // first `el.style.*` access).
        assert!(
            dom.world().get::<&InlineStyle>(div).is_err(),
            "InlineStyle must not be attached at parse time"
        );
        let attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(attrs.get("style"), Some("color: red; margin: 10px"));
    }

    #[test]
    fn implicit_elements_created() {
        // html5ever auto-generates html/head/body
        let result = parse_html("<p>Hello</p>");
        let dom = &result.dom;
        let doc = result.document;

        let html = find_tag(dom, doc, "html").expect("html");
        let head = find_tag(dom, html, "head").expect("head");
        let body = find_tag(dom, html, "body").expect("body");
        assert!(dom.contains(head));
        assert!(dom.contains(body));
        let p = find_tag(dom, body, "p").expect("p");
        assert_eq!(child_text(dom, p), "Hello");
    }

    #[test]
    fn text_only_document() {
        let result = parse_html("Hello World");
        let dom = &result.dom;
        let doc = result.document;

        let html = find_tag(dom, doc, "html").expect("html");
        let body = find_tag(dom, html, "body").expect("body");
        assert_eq!(child_text(dom, body), "Hello World");
    }

    #[test]
    fn img_no_parser_presentational_hints() {
        // Presentational hints are now handled by elidex-dom-compat, not the parser.
        let result = parse_html(r#"<img width="100" height="50">"#);
        let dom = &result.dom;
        let doc = result.document;

        let img = find_tag(dom, doc, "img").expect("img");
        // No InlineStyle from parser — hints are generated by compat layer during cascade.
        assert!(dom.world().get::<&InlineStyle>(img).is_err());
        // Attributes are still preserved.
        let attrs = dom.world().get::<&Attributes>(img).unwrap();
        assert_eq!(attrs.get("width"), Some("100"));
        assert_eq!(attrs.get("height"), Some("50"));
    }

    #[test]
    fn empty_document() {
        let result = parse_html("");
        let dom = &result.dom;
        let doc = result.document;

        // html5ever generates html/head/body even for empty input
        let html = find_tag(dom, doc, "html").expect("html");
        assert!(find_tag(dom, html, "head").is_some());
        assert!(find_tag(dom, html, "body").is_some());
    }

    #[test]
    fn parse_errors_collected() {
        let result = parse_html("<div><span></div>");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn multiple_children() {
        let result = parse_html("<div><p>A</p><p>B</p><p>C</p></div>");
        let dom = &result.dom;
        let doc = result.document;

        let div = find_tag(dom, doc, "div").expect("div");
        let children = dom.children(div);
        // All 3 children should be <p> elements
        assert_eq!(children.len(), 3);
        for child in &children {
            let tag = dom.world().get::<&TagType>(*child).unwrap();
            assert_eq!(tag.0, "p");
        }
        assert_eq!(child_text(dom, children[0]), "A");
        assert_eq!(child_text(dom, children[1]), "B");
        assert_eq!(child_text(dom, children[2]), "C");
    }

    #[test]
    fn comments_preserved() {
        let result = parse_html("<div><!-- comment -->text</div>");
        let dom = &result.dom;
        let doc = result.document;

        let div = find_tag(dom, doc, "div").expect("div");
        let children = dom.children(div);
        // Comment node + text node
        assert_eq!(children.len(), 2);
        // First child is the comment
        let comment = children[0];
        let comment_data = dom
            .world()
            .get::<&elidex_ecs::CommentData>(comment)
            .unwrap();
        assert_eq!(comment_data.0, " comment ");
        // Second child is text
        let tc = dom
            .world()
            .get::<&elidex_ecs::TextContent>(children[1])
            .unwrap();
        assert_eq!(tc.0, "text");
    }

    #[test]
    fn doctype_preserved() {
        let result = parse_html("<!DOCTYPE html><html><body>test</body></html>");
        let dom = &result.dom;
        let doc = result.document;

        // First child of document should be the doctype
        let children = dom.children(doc);
        let doctype = children
            .iter()
            .find(|&&e| dom.node_kind(e) == Some(elidex_ecs::NodeKind::DocumentType))
            .expect("should have doctype");
        let dt = dom
            .world()
            .get::<&elidex_ecs::DocTypeData>(*doctype)
            .unwrap();
        assert_eq!(dt.name, "html");
    }

    #[test]
    fn doctype_before_html() {
        let result = parse_html("<!DOCTYPE html><html><body></body></html>");
        let dom = &result.dom;
        let doc = result.document;

        let children = dom.children(doc);
        // Doctype should come before html element
        assert!(children.len() >= 2);
        let first_kind = dom.node_kind(children[0]);
        assert_eq!(first_kind, Some(elidex_ecs::NodeKind::DocumentType));
    }

    #[test]
    fn style_attribute_also_in_attributes() {
        let result = parse_html(r#"<div style="color: red"></div>"#);
        let dom = &result.dom;
        let doc = result.document;

        let div = find_tag(dom, doc, "div").expect("div");
        // style attribute preserved verbatim in Attributes; no eager
        // `InlineStyle` component (lazy CSSOM materialization).
        let attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(attrs.get("style"), Some("color: red"));
        assert!(dom.world().get::<&InlineStyle>(div).is_err());
    }

    #[test]
    fn inline_style_preserved_on_img() {
        // The inline style attribute is preserved verbatim; the cascade
        // reads it (presentational-hint override is handled by cascade
        // priority in elidex-dom-compat). No eager `InlineStyle`.
        let result = parse_html(r#"<img width="100" style="width: 200px">"#);
        let dom = &result.dom;
        let doc = result.document;

        let img = find_tag(dom, doc, "img").expect("img");
        let attrs = dom.world().get::<&Attributes>(img).unwrap();
        assert_eq!(attrs.get("style"), Some("width: 200px"));
        assert!(dom.world().get::<&InlineStyle>(img).is_err());
    }

    // HTML §4.12.3: `<template>` content routes into the detached content
    // fragment, not the template's light children (previously the tolerant
    // tier dropped it entirely — html5ever stores template children in
    // `template_contents`, which `convert_children` never read for ordinary
    // templates).
    #[test]
    fn template_content_routed_to_fragment() {
        let result = parse_html("<template><div>x</div></template>");
        let dom = &result.dom;
        let doc = result.document;

        let template = find_tag(dom, doc, "template").expect("template");
        // The template element has NO light children — content is detached.
        assert_eq!(
            dom.children(template).len(),
            0,
            "template content must not be light children of the template element"
        );
        // The content lives in the associated fragment.
        let fragment = dom
            .template_contents_fragment(template)
            .expect("template must have a content fragment");
        let frag_children = dom.children(fragment);
        assert_eq!(frag_children.len(), 1, "one child <div> in the fragment");
        let div = frag_children[0];
        assert!(dom.has_tag(div, "div"));
        assert_eq!(child_text(dom, div), "x");
    }

    #[test]
    fn nested_template_content_routed_to_inner_fragment() {
        // A `<template>` inside another template's content gets its own
        // fragment; the deepest `<span>` lives two fragments down.
        let result = parse_html("<template><template><span>y</span></template></template>");
        let dom = &result.dom;
        let doc = result.document;

        let outer = find_tag(dom, doc, "template").expect("outer template");
        let outer_frag = dom
            .template_contents_fragment(outer)
            .expect("outer content fragment");
        let inner = dom.children(outer_frag);
        assert_eq!(inner.len(), 1);
        let inner = inner[0];
        assert!(dom.has_tag(inner, "template"), "inner is a <template>");
        let inner_frag = dom
            .template_contents_fragment(inner)
            .expect("inner content fragment");
        let span = dom.children(inner_frag);
        assert_eq!(span.len(), 1);
        assert!(dom.has_tag(span[0], "span"));
        assert_eq!(child_text(dom, span[0]), "y");
    }

    // §10.8 cross-tier parity AC (the load-bearing One-issue-one-way
    // invariant): the SAME `<template>` markup yields structurally identical
    // content-fragment subtrees under the strict (Tier-1) and tolerant
    // (Tier-2) backends — both route through the one `attach_template_contents`
    // helper, differing only in redirect plumbing.
    #[test]
    fn template_content_strict_tolerant_parity() {
        // Strict (Tier-1) requires a doctype; tolerant accepts it too, so the
        // same input drives both backends.
        let html = "<!DOCTYPE html><template><div class=\"a\" id=\"d\">x</div><p>y</p></template>";

        let tolerant = parse_html(html);
        let strict = parse_strict(html).expect("strict parse");

        let frag_serialized = |result: &ParseResult| {
            let dom = &result.dom;
            let template = find_tag(dom, result.document, "template").expect("template present");
            let fragment = dom
                .template_contents_fragment(template)
                .expect("content fragment present");
            let mut out = String::new();
            for child in dom.children(fragment) {
                serialize_subtree(dom, child, &mut out);
            }
            out
        };

        let tolerant_tree = frag_serialized(&tolerant);
        let strict_tree = frag_serialized(&strict);
        assert_eq!(
            tolerant_tree, strict_tree,
            "strict and tolerant template content fragments must be structurally identical"
        );
        assert_eq!(strict_tree, "<div class=\"a\" id=\"d\">\"x\"</><p>\"y\"</>");
    }
}
