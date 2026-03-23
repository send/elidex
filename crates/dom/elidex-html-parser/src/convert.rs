//! `RcDom` → `EcsDom` conversion.
//!
//! Two-pass approach: html5ever parses into `RcDom`, then this module
//! walks the tree and builds an ECS DOM.

use std::fmt;

use elidex_ecs::{Attributes, EcsDom, Entity, InlineStyle};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// Result of parsing an HTML document.
///
/// `EcsDom` does not implement `Debug`, so this type provides a manual
/// implementation that prints the document entity and error list.
pub struct ParseResult {
    /// The populated DOM tree.
    pub dom: EcsDom,
    /// The document root entity (parent of `<html>`).
    pub document: Entity,
    /// Parse warnings collected from html5ever.
    pub errors: Vec<String>,
    /// Detected encoding name (set by `parse_tolerant`, `None` for `parse_html`).
    ///
    /// Always a canonical name from `encoding_rs` (e.g. `"UTF-8"`, `"Shift_JIS"`).
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

pub(crate) fn convert_document(rc_dom: RcDom) -> ParseResult {
    let mut dom = EcsDom::new();
    let document = dom.create_document_root();
    convert_children(&rc_dom.document, document, &mut dom);
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
    }
}

fn convert_children(rc_handle: &Handle, parent: Entity, dom: &mut EcsDom) {
    for child in &*rc_handle.children.borrow() {
        if let Some(entity) = convert_node(child, dom) {
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
) -> Vec<Entity> {
    let mut created = Vec::new();
    for child in &*rc_handle.children.borrow() {
        if let Some(entity) = convert_node(child, dom) {
            if dom.append_child(parent, entity) {
                created.push(entity);
            }
        }
    }
    created
}

/// Build element attributes and extract inline style from an element handle.
fn build_element_data(handle: &Handle) -> Option<(String, Attributes, Option<InlineStyle>)> {
    let NodeData::Element { name, attrs, .. } = &handle.data else {
        return None;
    };
    let tag = name.local.as_ref().to_string();
    let mut attributes = Attributes::default();
    let mut inline_style = None;
    for attr in attrs.borrow().iter() {
        let name = attr.name.local.as_ref();
        let value: &str = &attr.value;
        if name == "style" {
            inline_style = Some(parse_inline_style(value));
        }
        attributes.set(name, value);
    }
    Some((tag, attributes, inline_style))
}

fn convert_node(handle: &Handle, dom: &mut EcsDom) -> Option<Entity> {
    match &handle.data {
        NodeData::Element { .. } => {
            let (tag, attributes, inline_style) = build_element_data(handle)?;
            let entity = dom.create_element(&tag, attributes);
            if let Some(style) = inline_style {
                let ok = dom.world_mut().insert_one(entity, style).is_ok();
                debug_assert!(ok, "insert_one failed for InlineStyle");
            }
            // Mark custom elements for later upgrade (WHATWG HTML §4.13.2).
            if elidex_custom_elements::is_valid_custom_element_name(&tag) {
                let ce_state = elidex_custom_elements::CustomElementState::undefined(&tag);
                let _ = dom.world_mut().insert_one(entity, ce_state);
            }
            convert_children(handle, entity, dom);
            Some(entity)
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().to_string();
            if text.is_empty() || text.trim().is_empty() {
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

/// Parse a `style` attribute value into an [`InlineStyle`].
///
/// Uses simple `;` and `:` splitting. Full CSS value parsing is handled by
/// elidex-css and elidex-style.
fn parse_inline_style(style: &str) -> InlineStyle {
    let mut inline = InlineStyle::default();
    for decl in style.split(';') {
        let decl = decl.trim();
        if let Some((prop, val)) = decl.split_once(':') {
            let prop = prop.trim();
            let val = val.trim();
            if !prop.is_empty() && !val.is_empty() {
                inline.set(prop, val);
            }
        }
    }
    inline
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
        assert_eq!(style.get("color"), Some("red"));
        assert_eq!(style.get("margin"), Some("10px"));
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
        // Also parsed into InlineStyle
        let style = dom.world().get::<&InlineStyle>(div).unwrap();
        assert_eq!(style.get("color"), Some("red"));
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
