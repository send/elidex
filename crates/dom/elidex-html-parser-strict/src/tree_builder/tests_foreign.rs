//! Golden + reject tests for inline foreign content (WHATWG HTML §13.2.6.5).
//!
//! Exercises the strict foreign-content path through [`super::TreeBuilder`]:
//! `<math>` / `<svg>` entry (§13.2.6.4.7), the §13.2.6 dispatcher branch,
//! SVG element / attribute and MathML attribute case correction (§13.2.6.1 /
//! §13.2.6.5), the MathML-text / HTML integration points, CDATA sections, and
//! the strict reject of every §13.2.6.5 parse-error branch. Trees are asserted
//! in the html5lib `#document` format (foreign elements carry an `svg ` /
//! `math ` namespace prefix), reusing the serializer from [`super::tests`].

use elidex_ecs::{EcsDom, Entity, Namespace};

use super::tests::serialize_document;
use super::TreeBuilder;

/// Wrap `body` in a minimal conforming document, parse it, and assert the
/// serialized tree's `<body>` children equal `expected_body` (whose lines
/// already carry the depth-2 `|     ` indentation). A leading newline in
/// `expected_body` is trimmed for readable inline literals.
fn assert_body(body: &str, expected_body: &str) {
    let html = format!("<!DOCTYPE html><html><head></head><body>{body}</body></html>");
    let result = TreeBuilder::build(&html).expect("valid HTML5 should parse");
    let got = serialize_document(&result);
    let expected = format!(
        "| <!DOCTYPE html>\n| <html>\n|   <head>\n|   <body>\n{}",
        expected_body.trim_start_matches('\n')
    );
    assert_eq!(got, expected, "\ninput: {html}");
}

/// Wrap `body` in a minimal document and assert strict parsing rejects it.
fn assert_reject(body: &str) {
    let html = format!("<!DOCTYPE html><html><head></head><body>{body}</body></html>");
    assert!(
        TreeBuilder::build(&html).is_err(),
        "expected strict reject for: {html}"
    );
}

/// Recursively find the first element with the exact tag name `tag`.
fn find_tag(dom: &EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
    let mut next = dom.get_first_child(parent);
    while let Some(child) = next {
        if dom.has_tag(child, tag) {
            return Some(child);
        }
        if let Some(found) = find_tag(dom, child, tag) {
            return Some(found);
        }
        next = dom.get_next_sibling(child);
    }
    None
}

// ----- §13.2.6.4.7 entry + namespaces -----

#[test]
fn svg_root_is_svg_namespace() {
    assert_body(
        "<svg></svg>",
        "\
|     <svg svg>
",
    );
}

#[test]
fn math_root_is_mathml_namespace() {
    assert_body(
        "<math></math>",
        "\
|     <math math>
",
    );
}

#[test]
fn svg_nested_children_stay_in_svg_namespace() {
    assert_body(
        "<svg><g><circle></circle></g></svg>",
        "\
|     <svg svg>
|       <svg g>
|         <svg circle>
",
    );
}

// ----- §13.2.6.5 SVG element tag-name case table -----

#[test]
fn svg_element_tag_name_case_corrected() {
    // `foreignobject` / `fegaussianblur` arrive ASCII-lowercased from the
    // tokenizer and are case-corrected per the §13.2.6.5 element table.
    assert_body(
        "<svg><foreignObject></foreignObject><feGaussianBlur></feGaussianBlur></svg>",
        "\
|     <svg svg>
|       <svg foreignObject>
|       <svg feGaussianBlur>
",
    );
}

// ----- §13.2.6.1 attribute case tables -----

#[test]
fn svg_attribute_case_corrected() {
    assert_body(
        "<svg viewBox=\"0 0 1 1\"></svg>",
        "\
|     <svg svg>
|       viewBox=\"0 0 1 1\"
",
    );
}

#[test]
fn mathml_attribute_case_corrected() {
    assert_body(
        "<math definitionURL=\"u\"></math>",
        "\
|     <math math>
|       definitionURL=\"u\"
",
    );
}

// ----- integration points (HTML re-entry) -----

#[test]
fn svg_foreign_object_is_html_integration_point() {
    // Inside an SVG `foreignObject` the dispatcher routes start tags back to
    // HTML content, so the `<div>` is an HTML-namespace element.
    assert_body(
        "<svg><foreignObject><div>hi</div></foreignObject></svg>",
        "\
|     <svg svg>
|       <svg foreignObject>
|         <div>
|           \"hi\"
",
    );
}

#[test]
fn mathml_mtext_is_text_integration_point() {
    // `mtext` is a MathML text integration point: the `<b>` start tag is
    // processed as HTML content.
    assert_body(
        "<math><mtext><b>x</b></mtext></math>",
        "\
|     <math math>
|       <math mtext>
|         <b>
|           \"x\"
",
    );
}

#[test]
fn annotation_xml_html_encoding_is_integration_point() {
    // A MathML `annotation-xml` element with `encoding=text/html` is an HTML
    // integration point.
    assert_body(
        "<math><annotation-xml encoding=\"text/html\"><div></div></annotation-xml></math>",
        "\
|     <math math>
|       <math annotation-xml>
|         encoding=\"text/html\"
|         <div>
",
    );
}

#[test]
fn annotation_xml_xhtml_encoding_is_integration_point() {
    // The `application/xhtml+xml` encoding (matched ASCII case-insensitively)
    // is the second HTML-integration-point trigger for `annotation-xml`.
    assert_body(
        "<math><annotation-xml encoding=\"APPLICATION/XHTML+XML\"><div></div></annotation-xml></math>",
        "\
|     <math math>
|       <math annotation-xml>
|         encoding=\"APPLICATION/XHTML+XML\"
|         <div>
",
    );
}

#[test]
fn mglyph_at_mathml_text_integration_point_stays_mathml() {
    // §13.2.6 dispatcher condition 3 excludes `mglyph`/`malignmark`: at a
    // MathML text integration point they are NOT routed to HTML content, so
    // they remain MathML foreign elements (contrast `<b>`, which becomes HTML).
    assert_body(
        "<math><mtext><mglyph></mglyph><malignmark></malignmark></mtext></math>",
        "\
|     <math math>
|       <math mtext>
|         <math mglyph>
|         <math malignmark>
",
    );
}

// ----- self-closing, CDATA, comments -----

#[test]
fn self_closing_svg_element_is_popped() {
    assert_body(
        "<svg><circle/><rect/></svg>",
        "\
|     <svg svg>
|       <svg circle>
|       <svg rect>
",
    );
}

#[test]
fn cdata_section_in_foreign_content_becomes_text() {
    // `<![CDATA[ … ]]>` is only valid in foreign content; its characters
    // (including a literal `<`) become a Text node.
    assert_body(
        "<svg><![CDATA[x<y]]></svg>",
        "\
|     <svg svg>
|       \"x<y\"
",
    );
}

#[test]
fn comment_in_foreign_content_is_inserted() {
    assert_body(
        "<svg><!--c--></svg>",
        "\
|     <svg svg>
|       <!-- c -->
",
    );
}

#[test]
fn svg_title_is_foreign_not_rawtext() {
    // An SVG `<title>` is an ordinary foreign element whose content is parsed
    // as foreign content (contrast HTML `<title>`, which is RAWTEXT).
    assert_body(
        "<svg><title>t</title></svg>",
        "\
|     <svg svg>
|       <svg title>
|         \"t\"
",
    );
}

// ----- namespace component assertions (public-API contract) -----

#[test]
fn foreign_elements_carry_namespace_and_html_reentry() {
    let html = "<!DOCTYPE html><html><head></head><body>\
        <svg><foreignObject><div></div></foreignObject></svg><math></math>\
        </body></html>";
    let result = TreeBuilder::build(html).expect("valid HTML5 should parse");
    let dom = &result.dom;

    let svg = find_tag(dom, result.document, "svg").expect("svg element");
    assert_eq!(dom.namespace_of(svg), Namespace::Svg);

    let foreign_object = find_tag(dom, result.document, "foreignObject").expect("foreignObject");
    assert_eq!(dom.namespace_of(foreign_object), Namespace::Svg);

    // Re-entered HTML content at the integration point is HTML-namespace.
    let div = find_tag(dom, result.document, "div").expect("div element");
    assert_eq!(dom.namespace_of(div), Namespace::Html);

    let math = find_tag(dom, result.document, "math").expect("math element");
    assert_eq!(dom.namespace_of(math), Namespace::MathMl);
}

#[test]
fn svg_start_tag_inside_annotation_xml_switches_to_svg() {
    // §13.2.6 dispatcher condition 5: an `<svg>` start tag whose adjusted
    // current node is a MathML `annotation-xml` element is processed as HTML
    // content (the in-body `<svg>` entry), nesting SVG inside MathML.
    assert_body(
        "<math><annotation-xml><svg><circle></circle></svg></annotation-xml></math>",
        "\
|     <math math>
|       <math annotation-xml>
|         <svg svg>
|           <svg circle>
",
    );
}

#[test]
fn foreign_content_inside_template() {
    // `<svg>` reaches the in-body rules through the "in template" mode; the
    // foreign subtree is held in the template's contents.
    assert_body(
        "<template><svg><circle></circle></svg></template>",
        "\
|     <template>
|       content
|         <svg svg>
|           <svg circle>
",
    );
}

#[test]
fn foreign_content_inside_table_cell() {
    // `<svg>` reaches foreign content through a non-"in body" insertion mode
    // (here "in cell", which delegates to the in-body rules), and the
    // dispatcher keeps routing the subtree to foreign content until `</svg>`.
    let html = "<!DOCTYPE html><html><head></head><body>\
        <table><tbody><tr><td><svg><circle></circle></svg></td></tr></tbody></table>\
        </body></html>";
    let result = TreeBuilder::build(html).expect("valid HTML5 should parse");
    let circle = find_tag(&result.dom, result.document, "circle").expect("circle in cell");
    assert_eq!(result.dom.namespace_of(circle), Namespace::Svg);
}

// ----- §13.2.6.5 strict reject (no error recovery) -----

#[test]
fn breakout_start_tag_in_foreign_content_rejected() {
    // A breakout HTML start tag directly in foreign content (no integration
    // point) is the spec's pop-out-and-reprocess recovery; strict rejects.
    assert_reject("<svg><div></div></svg>");
    assert_reject("<math><p></p></math>");
}

#[test]
fn font_with_layout_attr_in_foreign_content_rejected() {
    // `<font>` carrying color/face/size is a breakout start tag.
    assert_reject("<svg><font color=\"red\"></font></svg>");
}

#[test]
fn plain_font_in_foreign_content_is_a_foreign_element() {
    // `<font>` with no color/face/size is an ordinary foreign element.
    assert_body(
        "<svg><font></font></svg>",
        "\
|     <svg svg>
|       <svg font>
",
    );
}

#[test]
fn breakout_end_tag_in_foreign_content_rejected() {
    assert_reject("<svg></p></svg>");
    assert_reject("<svg></br></svg>");
}

#[test]
fn doctype_in_foreign_content_rejected() {
    assert_reject("<svg><!DOCTYPE html></svg>");
}

#[test]
fn misnested_foreign_end_tag_rejected() {
    // `</svg>` while an unclosed `<g>` is the current node is misnesting.
    assert_reject("<svg><g></svg>");
}

#[test]
fn null_character_in_cdata_section_rejected() {
    // A U+0000 NULL reaches foreign content only via a CDATA section; strict
    // rejects rather than substituting U+FFFD.
    assert_reject("<svg><![CDATA[\u{0000}]]></svg>");
}

#[test]
fn cdata_section_in_html_content_rejected() {
    // Outside foreign content `<![CDATA[` is the `cdata-in-html-content`
    // parse error (the foreign-content flag is clear).
    assert_reject("<div><![CDATA[x]]></div>");
}
