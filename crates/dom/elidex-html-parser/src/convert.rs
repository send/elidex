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

fn convert_children(
    rc_handle: &Handle,
    parent: Entity,
    dom: &mut EcsDom,
    opts: ParseFragmentOptions,
) {
    for child in &*rc_handle.children.borrow() {
        if opts.allow_declarative_shadow && try_attach_declarative_shadow(child, parent, dom, opts)
        {
            // Template was consumed as declarative shadow root markup;
            // the host entity already received the new shadow tree.
            continue;
        }
        if let Some(entity) = convert_node(child, dom, opts) {
            let ok = dom.append_child(parent, entity);
            debug_assert!(ok, "append_child failed during RcDom conversion");
        }
    }
}

/// Convert fragment children and return the list of newly created entities.
///
/// Like [`convert_children`] but returns the created entities for tracking.
/// Used by [`crate::parse_html_fragment`] to report which nodes were added.
pub(crate) fn convert_fragment_children(
    rc_handle: &Handle,
    parent: Entity,
    dom: &mut EcsDom,
    opts: ParseFragmentOptions,
) -> Vec<Entity> {
    let mut created = Vec::new();
    for child in &*rc_handle.children.borrow() {
        if opts.allow_declarative_shadow && try_attach_declarative_shadow(child, parent, dom, opts)
        {
            // Template was consumed as a declarative shadow root; no
            // child entity is created in the light tree.
            continue;
        }
        if let Some(entity) = convert_node(child, dom, opts) {
            if dom.append_child(parent, entity) {
                created.push(entity);
            }
        }
    }
    created
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
        convert_children(&contents, shadow_root, dom, opts);
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
/// Derived components (InlineStyle / CustomElementState / IframeData) are
/// attached by `element_init::attach_derived`, invoked per-node from
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

fn convert_node(handle: &Handle, dom: &mut EcsDom, opts: ParseFragmentOptions) -> Option<Entity> {
    match &handle.data {
        NodeData::Element { .. } => {
            let (tag, namespace, attributes) = build_element_data(handle)?;
            // `create_element_ns` attaches a `Namespace` component only for
            // non-HTML namespaces (HTML stays component-free), so the foreign
            // guard in `attach_derived` sees the real namespace.
            let entity = dom.create_element_ns(&tag, namespace, attributes, None);
            // Attach derived components at creation time — BEFORE the element
            // is appended anywhere. The tolerant fragment path
            // (`convert_fragment_children`, e.g. `innerHTML`) builds into a
            // live, dispatcher-bound `dom`, so `append_child` fires
            // `MutationEvent::Insert` synchronously; the `CustomElementState`
            // / `IframeData` / `InlineStyle` must already be present when the
            // CustomElementReactionConsumer reads them. Deriving in a
            // post-build walk would race the insert (and miss declarative-
            // shadow content not tracked in any root list). The strict Tier-1
            // backend instead derives post-build in `parse_progressive` (it
            // is pre-bind / dispatch-suppressed and cannot reach this crate's
            // deps). Both share the one `attach_derived` implementation.
            attach_derived(dom, entity);
            convert_children(handle, entity, dom, opts);
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
    use crate::parse_html;
    use crate::test_helpers::{child_text, find_tag};
    use elidex_ecs::{InlineStyle, TagType};

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
        let style = dom.world().get::<&InlineStyle>(div).unwrap();
        // Canonical `elidex_css::parse_inline_style` form: color keywords
        // round-trip to hex, shorthands expand to longhands.
        assert_eq!(style.get("color"), Some("#ff0000"));
        assert_eq!(style.get("margin"), None);
        assert_eq!(style.get("margin-top"), Some("10px"));
        assert_eq!(style.get("margin-left"), Some("10px"));
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
        // style attribute preserved in Attributes
        let attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(attrs.get("style"), Some("color: red"));
        // Also parsed into InlineStyle (canonical post-parse form).
        let style = dom.world().get::<&InlineStyle>(div).unwrap();
        assert_eq!(style.get("color"), Some("#ff0000"));
    }

    #[test]
    fn inline_style_preserved_on_img() {
        // Inline style attributes are still parsed by the parser.
        // Presentational hint override is handled by cascade priority in elidex-dom-compat.
        let result = parse_html(r#"<img width="100" style="width: 200px">"#);
        let dom = &result.dom;
        let doc = result.document;

        let img = find_tag(dom, doc, "img").expect("img");
        let style = dom.world().get::<&InlineStyle>(img).unwrap();
        assert_eq!(style.get("width"), Some("200px"));
    }
}
