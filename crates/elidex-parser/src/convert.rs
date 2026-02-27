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
}

impl fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseResult")
            .field("document", &self.document)
            .field("errors", &self.errors)
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
    // Presentational hints: <img width/height>
    if tag == "img" {
        inline_style = apply_presentational_hints(&attributes, inline_style);
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
        // Comment, Doctype, ProcessingInstruction — skip for Phase 1
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

/// Apply HTML presentational hints (e.g., `<img width="100">`) as inline styles.
///
/// Only applies if the corresponding CSS property is not already set
/// in an explicit `style` attribute.
fn apply_presentational_hints(
    attrs: &Attributes,
    existing: Option<InlineStyle>,
) -> Option<InlineStyle> {
    let mut style = None;
    for &prop in &["width", "height"] {
        if let Some(val) = attrs.get(prop) {
            let s = style.get_or_insert_with(|| existing.clone().unwrap_or_default());
            if !s.contains(prop) {
                s.set(prop, format_dimension(val));
            }
        }
    }
    style.or(existing)
}

/// Append "px" if the value is a bare non-negative integer (ASCII digits only).
///
/// Per HTML spec, presentational attributes like `width` and `height` use
/// non-negative integers. Values with signs, decimals, or units are returned
/// unchanged.
fn format_dimension(value: &str) -> String {
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        format!("{value}px")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_html;
    use elidex_ecs::{InlineStyle, TagType, TextContent};

    /// Walk children and find the first element with the given tag.
    fn find_tag(dom: &EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
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
    fn child_text(dom: &EcsDom, parent: Entity) -> String {
        let mut result = String::new();
        for child in dom.children_iter(parent) {
            if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                result.push_str(&tc.0);
            }
        }
        result
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
    fn img_presentational_hints() {
        let result = parse_html(r#"<img width="100" height="50">"#);
        let dom = &result.dom;
        let doc = result.document;

        let img = find_tag(dom, doc, "img").expect("img");
        let style = dom.world().get::<&InlineStyle>(img).unwrap();
        assert_eq!(style.get("width"), Some("100px"));
        assert_eq!(style.get("height"), Some("50px"));
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
    fn comments_skipped() {
        let result = parse_html("<div><!-- comment -->text</div>");
        let dom = &result.dom;
        let doc = result.document;

        let div = find_tag(dom, doc, "div").expect("div");
        let children = dom.children(div);
        // Only the text node, no comment
        assert_eq!(children.len(), 1);
        assert_eq!(child_text(dom, div), "text");
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
    fn presentational_hint_no_override() {
        let result = parse_html(r#"<img width="100" style="width: 200px">"#);
        let dom = &result.dom;
        let doc = result.document;

        let img = find_tag(dom, doc, "img").expect("img");
        let style = dom.world().get::<&InlineStyle>(img).unwrap();
        // style attribute takes precedence over presentational hint
        assert_eq!(style.get("width"), Some("200px"));
    }
}
