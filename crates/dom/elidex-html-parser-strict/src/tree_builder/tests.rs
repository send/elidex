//! Crate-internal golden tests for the strict tree builder.
//!
//! [`super::TreeBuilder`] is `pub(crate)`, so it is unreachable from an
//! integration test; the tree builder is exercised here instead. Trees are
//! asserted by serializing the built [`EcsDom`] into the html5lib
//! tree-construction `#document` text format (also reused by the corpus
//! harness in `tests_html5lib_tree`).

use std::fmt::Write as _;

use elidex_ecs::{
    Attributes, CommentData, DocTypeData, EcsDom, Entity, Namespace, NodeKind, TextContent,
};

use super::TreeBuilder;
use crate::result::ParseResult;

/// Collect an entity's children in tree order.
fn children(dom: &EcsDom, entity: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut next = dom.get_first_child(entity);
    while let Some(child) = next {
        out.push(child);
        next = dom.get_next_sibling(child);
    }
    out
}

/// Find the first light-tree descendant of `root` with tag `tag` (depth-first).
fn find_descendant_tag(dom: &EcsDom, root: Entity, tag: &str) -> Option<Entity> {
    for child in children(dom, root) {
        if dom.has_tag(child, tag) {
            return Some(child);
        }
        if let Some(found) = find_descendant_tag(dom, child, tag) {
            return Some(found);
        }
    }
    None
}

/// Serialize a parsed document into the html5lib tree-construction
/// `#document` format: one `| `-prefixed line per node, two spaces of indent
/// per depth level, attributes sorted by name, and `<template>` content under
/// a `content` pseudo-node.
pub(super) fn serialize_document(result: &ParseResult) -> String {
    let mut out = String::new();
    for child in children(&result.dom, result.document) {
        serialize_node(&result.dom, child, 0, &mut out);
    }
    out
}

/// Serialize a §13.4 fragment parse — the detached root nodes — into the same
/// html5lib `#document` text format (roots at depth 0). The html5lib fragment
/// corpus lists the fragment's nodes directly, with no enclosing context
/// element, so the roots map one-to-one onto the expected lines.
pub(super) fn serialize_fragment(dom: &EcsDom, roots: &[Entity]) -> String {
    let mut out = String::new();
    for &root in roots {
        serialize_node(dom, root, 0, &mut out);
    }
    out
}

pub(super) fn serialize_node(dom: &EcsDom, entity: Entity, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    match dom.node_kind(entity) {
        Some(NodeKind::Element) => {
            let tag = dom.get_tag_name(entity).unwrap_or_default();
            // html5lib prefixes foreign elements with their namespace
            // (`<svg svg>`, `<math math>`); HTML elements have no prefix.
            let ns_prefix = match dom.namespace_of(entity) {
                Namespace::Html => "",
                Namespace::Svg => "svg ",
                Namespace::MathMl => "math ",
            };
            let _ = writeln!(out, "| {indent}<{ns_prefix}{tag}>");
            if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
                let mut pairs: Vec<(&str, &str)> = attrs.iter().collect();
                pairs.sort_by(|a, b| a.0.cmp(b.0));
                let attr_indent = "  ".repeat(depth + 1);
                for (name, value) in pairs {
                    let _ = writeln!(out, "| {attr_indent}{name}=\"{value}\"");
                }
            }
            if tag == "template" {
                // html5lib serializes template contents under a `content`
                // pseudo-node; elidex holds them in the template's detached
                // `TemplateContents` fragment (HTML §4.12.3).
                let _ = writeln!(out, "| {}content", "  ".repeat(depth + 1));
                if let Some(fragment) = dom.template_contents_fragment(entity) {
                    for child in children(dom, fragment) {
                        serialize_node(dom, child, depth + 2, out);
                    }
                }
            } else {
                for child in children(dom, entity) {
                    serialize_node(dom, child, depth + 1, out);
                }
            }
        }
        Some(NodeKind::Text) => {
            let text = dom
                .world()
                .get::<&TextContent>(entity)
                .map(|t| t.0.clone())
                .unwrap_or_default();
            let _ = writeln!(out, "| {indent}\"{text}\"");
        }
        Some(NodeKind::Comment) => {
            let data = dom
                .world()
                .get::<&CommentData>(entity)
                .map(|c| c.0.clone())
                .unwrap_or_default();
            let _ = writeln!(out, "| {indent}<!-- {data} -->");
        }
        Some(NodeKind::DocumentType) => {
            let doctype = dom.world().get::<&DocTypeData>(entity);
            let line = match doctype {
                Ok(dt) if dt.public_id.is_empty() && dt.system_id.is_empty() => {
                    format!("<!DOCTYPE {}>", dt.name)
                }
                Ok(dt) => format!(
                    "<!DOCTYPE {} \"{}\" \"{}\">",
                    dt.name, dt.public_id, dt.system_id
                ),
                Err(_) => "<!DOCTYPE >".to_string(),
            };
            let _ = writeln!(out, "| {indent}{line}");
        }
        _ => {}
    }
}

/// Parse `html` and assert the serialized tree equals `expected` (with a
/// leading newline trimmed for readable inline literals).
fn assert_tree(html: &str, expected: &str) {
    let result = TreeBuilder::build(html).expect("valid HTML5 should parse");
    let got = serialize_document(&result);
    assert_eq!(got, expected.trim_start_matches('\n'), "\ninput: {html}");
}

#[test]
fn minimal_document() {
    assert_tree(
        "<!DOCTYPE html><html><head></head><body></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
",
    );
}

#[test]
fn implied_head_and_body() {
    // No explicit head/body: both are inserted implicitly.
    assert_tree(
        "<!DOCTYPE html><html><p>Hi</p></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <p>
|       \"Hi\"
",
    );
}

#[test]
fn nested_elements_and_text() {
    assert_tree(
        "<!DOCTYPE html><html><body><div><span>x</span> y</div></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <div>
|       <span>
|         \"x\"
|       \" y\"
",
    );
}

#[test]
fn attributes_in_source_order_serialized_sorted() {
    // Attributes are stored in source order; the serializer sorts by name.
    assert_tree(
        "<!DOCTYPE html><html><body><div id=\"a\" class=\"b\"></div></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <div>
|       class=\"b\"
|       id=\"a\"
",
    );
}

#[test]
fn paragraph_auto_closes_on_block() {
    // A <p> is implicitly closed by a following block-level <div>.
    assert_tree(
        "<!DOCTYPE html><html><body><p>one<div>two</div></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <p>
|       \"one\"
|     <div>
|       \"two\"
",
    );
}

#[test]
fn list_items_auto_close() {
    assert_tree(
        "<!DOCTYPE html><html><body><ul><li>a<li>b</ul></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <ul>
|       <li>
|         \"a\"
|       <li>
|         \"b\"
",
    );
}

#[test]
fn title_rcdata_text() {
    // Title content is RCDATA: the `<b>` is literal text, not an element.
    assert_tree(
        "<!DOCTYPE html><html><head><title>a<b>c</title></head><body></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|     <title>
|       \"a<b>c\"
|   <body>
",
    );
}

#[test]
fn style_rawtext_text() {
    assert_tree(
        "<!DOCTYPE html><html><head><style>.x{color:red}</style></head><body></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|     <style>
|       \".x{color:red}\"
|   <body>
",
    );
}

#[test]
fn comment_in_body() {
    assert_tree(
        "<!DOCTYPE html><html><body><!-- hi --></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <!--  hi  -->
",
    );
}

#[test]
fn table_with_implied_tbody() {
    assert_tree(
        "<!DOCTYPE html><html><body><table><tr><td>x</td></tr></table></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <table>
|       <tbody>
|         <tr>
|           <td>
|             \"x\"
",
    );
}

#[test]
fn table_whitespace_preserved() {
    // Whitespace between table-structure elements is kept as text, attached to
    // whichever element is current when it is seen (the `\n` before `<tbody>`
    // lands in the table, the `\n` before `<tr>` lands in the tbody);
    // non-whitespace character data would be rejected.
    assert_tree(
        "<!DOCTYPE html><html><body><table>\n<tbody>\n<tr></tr></tbody></table></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <table>
|       \"\n\"
|       <tbody>
|         \"\n\"
|         <tr>
",
    );
}

#[test]
fn template_in_head_holds_content() {
    assert_tree(
        "<!DOCTYPE html><html><head><template><div>x</div></template></head><body></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|     <template>
|       content
|         <div>
|           \"x\"
|   <body>
",
    );
}

#[test]
fn template_content_in_detached_fragment_not_light_children() {
    // HTML §4.12.3: the parser routes `<template>` children into the detached
    // content fragment (the appropriate-place redirect), not the template
    // element's light children.
    let result = TreeBuilder::build(
        "<!DOCTYPE html><html><head><template><div>x</div></template></head></html>",
    )
    .expect("valid template parses");
    let dom = &result.dom;
    let template = find_descendant_tag(dom, result.document, "template").expect("template");
    assert!(
        children(dom, template).is_empty(),
        "template content must not be light children"
    );
    let fragment = dom
        .template_contents_fragment(template)
        .expect("content fragment");
    let frag_children = children(dom, fragment);
    assert_eq!(frag_children.len(), 1);
    assert!(dom.has_tag(frag_children[0], "div"));
}

#[test]
fn hr_in_select_is_sibling_of_option() {
    // `<hr>` is valid `<select>` content: with a select in scope the spec
    // generates implied end tags first, popping the open option, so the `<hr>`
    // is a sibling of the option (not nested inside it).
    assert_tree(
        "<!DOCTYPE html><html><body><select><option>a<hr></select></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <select>
|       <option>
|         \"a\"
|       <hr>
",
    );
}

#[test]
fn pre_drops_leading_newline() {
    assert_tree(
        "<!DOCTYPE html><html><body><pre>\nkept</pre></body></html>",
        "\
| <!DOCTYPE html>
| <html>
|   <head>
|   <body>
|     <pre>
|       \"kept\"
",
    );
}

#[test]
fn declarative_shadow_attaches_to_host() {
    // `<div><template shadowrootmode=open>…` attaches a shadow root to the
    // div; the template itself is not in the light DOM, and its children live
    // in the shadow root.
    let html = "<!DOCTYPE html><html><body><div><template shadowrootmode=\"open\"><span>s</span></template></div></body></html>";
    let result = TreeBuilder::build(html).expect("valid declarative shadow should parse");
    let dom = &result.dom;
    // Locate body > div.
    let html_el = children(dom, result.document)
        .into_iter()
        .find(|&e| dom.has_tag(e, "html"))
        .expect("html");
    let body = children(dom, html_el)
        .into_iter()
        .find(|&e| dom.has_tag(e, "body"))
        .expect("body");
    let div = children(dom, body)
        .into_iter()
        .find(|&e| dom.has_tag(e, "div"))
        .expect("div");
    // The div has no light-DOM template child.
    assert!(
        children(dom, div)
            .iter()
            .all(|&e| !dom.has_tag(e, "template")),
        "declarative shadow template must not remain in the light DOM"
    );
    // The div is a shadow host; the shadow root holds the <span>.
    let shadow = dom
        .get_shadow_root(div)
        .expect("div should have an attached shadow root");
    let span = children(dom, shadow)
        .into_iter()
        .find(|&e| dom.has_tag(e, "span"))
        .expect("shadow root should contain the span");
    assert_eq!(
        children(dom, span)
            .iter()
            .filter_map(|&e| dom.world().get::<&TextContent>(e).ok().map(|t| t.0.clone()))
            .collect::<String>(),
        "s"
    );
}

#[test]
fn declarative_shadow_disallowed_leaves_template() {
    // With declarative shadow roots disabled, the template stays an ordinary
    // element in the light DOM.
    let html = "<!DOCTYPE html><html><body><div><template shadowrootmode=\"open\"><span>s</span></template></div></body></html>";
    let result = TreeBuilder::build_with_declarative_shadow(html, false)
        .expect("valid template should parse");
    let dom = &result.dom;
    let html_el = children(dom, result.document)
        .into_iter()
        .find(|&e| dom.has_tag(e, "html"))
        .expect("html");
    let body = children(dom, html_el)
        .into_iter()
        .find(|&e| dom.has_tag(e, "body"))
        .expect("body");
    let div = children(dom, body)
        .into_iter()
        .find(|&e| dom.has_tag(e, "div"))
        .expect("div");
    assert!(
        children(dom, div)
            .iter()
            .any(|&e| dom.has_tag(e, "template")),
        "without declarative shadow the template stays in the light DOM"
    );
    assert!(
        dom.get_shadow_root(div).is_none(),
        "no shadow root should be attached when declarative shadow is disabled"
    );
}

#[test]
fn failed_declarative_shadow_template_falls_back_to_ordinary_with_content_fragment() {
    // A second `<template shadowrootmode>` on a host that already has a shadow
    // root fails to attach and gracefully becomes an *ordinary* template
    // (§4.12.3). It must then get a content fragment like any ordinary
    // template — its `<span>` lands in the fragment, not its light children.
    let html = "<!DOCTYPE html><html><body><div>\
        <template shadowrootmode=\"open\"></template>\
        <template shadowrootmode=\"open\"><span>x</span></template>\
        </div></body></html>";
    let result = TreeBuilder::build_with_declarative_shadow(html, true)
        .expect("valid templates should parse");
    let dom = &result.dom;
    // The div is a shadow host (first declarative template succeeded) and has
    // exactly one light child: the fallback ordinary template.
    let div = find_descendant_tag(dom, result.document, "div").expect("div");
    assert!(
        dom.get_shadow_root(div).is_some(),
        "first template shadowed div"
    );
    let fallback = children(dom, div)
        .into_iter()
        .find(|&e| dom.has_tag(e, "template"))
        .expect("fallback ordinary template in light DOM");
    assert!(
        children(dom, fallback).is_empty(),
        "fallback template content must not be light children"
    );
    let fragment = dom
        .template_contents_fragment(fallback)
        .expect("fallback template has a content fragment");
    let frag_children = children(dom, fragment);
    assert_eq!(frag_children.len(), 1);
    assert!(dom.has_tag(frag_children[0], "span"));
}

// ----- strict-reject cases (no error recovery) -----

#[track_caller]
fn assert_rejected(html: &str) {
    let result = TreeBuilder::build(html);
    assert!(
        result.is_err(),
        "expected strict reject for {html:?}, got Ok"
    );
}

#[test]
fn rejects_missing_doctype() {
    assert_rejected("<html><head></head><body></body></html>");
}

#[test]
fn rejects_non_conforming_doctype() {
    assert_rejected("<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\"><html></html>");
}

#[test]
fn rejects_misnested_tags() {
    assert_rejected("<!DOCTYPE html><html><body><div><span></div></span></body></html>");
}

#[test]
fn rejects_self_closing_non_void() {
    assert_rejected("<!DOCTYPE html><html><body><div/></body></html>");
}

#[test]
fn rejects_unclosed_block_at_eof() {
    assert_rejected("<!DOCTYPE html><html><body><div>x</body></html>");
}

#[test]
fn rejects_stray_end_tag() {
    assert_rejected("<!DOCTYPE html><html><body></div></body></html>");
}

#[test]
fn rejects_content_after_body() {
    assert_rejected("<!DOCTYPE html><html><body></body>trailing</html>");
}
