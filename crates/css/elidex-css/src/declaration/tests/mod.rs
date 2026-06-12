use super::*;
use elidex_plugin::{CssColor, LengthUnit};

mod grid_table;
mod shorthand;
mod values;

fn parse_decls(css: &str) -> Vec<Declaration> {
    parse_declaration_block(css)
}

fn parse_single(property: &str, value: &str) -> Vec<Declaration> {
    parse_decls(&format!("{property}: {value}"))
}

// --- parse_inline_style (canonical attribute→InlineStyle derivation) ---

#[test]
fn inline_style_basic_properties() {
    let style = parse_inline_style("display: block; width: 200px");
    assert_eq!(style.get("display"), Some("block"));
    assert_eq!(style.get("width"), Some("200px"));
}

#[test]
fn inline_style_color_keyword_canonicalizes_to_hex() {
    // Values store in post-parse canonical form: `red` → `#ff0000`
    // (same accepted divergence as the CSSOM cssText round-trip).
    let style = parse_inline_style("color: red");
    assert_eq!(style.get("color"), Some("#ff0000"));
}

#[test]
fn inline_style_shorthand_expands_to_longhands() {
    let style = parse_inline_style("margin: 10px");
    assert_eq!(style.get("margin"), None);
    assert_eq!(style.get("margin-top"), Some("10px"));
    assert_eq!(style.get("margin-bottom"), Some("10px"));
}

#[test]
fn inline_style_value_with_semicolon_inside_function_not_split() {
    // The motivating divergence for the canonical fn: a naive `;`/`:`
    // splitter shreds function values containing `;` or `:` (data URLs).
    // The real parser must not fabricate properties out of the URL body.
    let style = parse_inline_style("background: url(data:image/png;base64,iVBO); color: blue");
    assert_eq!(style.get("color"), Some("#0000ff"));
    // No garbage keys leaked from inside the url() token.
    assert!(style.get("base64,iVBO)").is_none());
    for i in 0..style.len() {
        let prop = style.property_at(i).unwrap();
        assert!(
            !prop.contains("base64") && !prop.contains("url"),
            "naive-split artifact leaked into InlineStyle: {prop}"
        );
    }
}

#[test]
fn inline_style_unknown_property_dropped() {
    let style = parse_inline_style("not-a-real-property: 12px; display: flex");
    assert_eq!(style.get("not-a-real-property"), None);
    assert_eq!(style.get("display"), Some("flex"));
}

#[test]
fn inline_style_calc_round_trips() {
    let style = parse_inline_style("width: calc(100% - 10px)");
    assert_eq!(style.get("width"), Some("calc(100% - 10px)"));
    // The serialized form must re-parse to the same value.
    let reparsed = parse_inline_style(&style.css_text());
    assert_eq!(reparsed.get("width"), Some("calc(100% - 10px)"));
}

#[test]
fn inline_style_custom_property_case_preserved() {
    let style = parse_inline_style("--MyVar: 10px");
    assert_eq!(style.get("--MyVar"), Some("10px"));
}

#[test]
fn inline_style_empty_and_garbage_input() {
    assert!(parse_inline_style("").is_empty());
    assert!(parse_inline_style("garbage").is_empty());
    assert!(parse_inline_style(";;;").is_empty());
}

#[test]
fn inline_style_important_preserved_and_reemitted() {
    // The cascade reads importance from the style attribute, which
    // `sync_to_attribute` rewrites from `css_text()` — a
    // priority-stripping derivation would silently demote `!important`
    // on the first unrelated style write.
    let style = parse_inline_style("color: red !important; width: 10px");
    assert!(style.is_important("color"));
    assert!(!style.is_important("width"));
    assert_eq!(style.get("color"), Some("#ff0000"));

    let text = style.css_text();
    assert_eq!(text, "color: #ff0000 !important; width: 10px");

    // Full round-trip: the re-parse sees the same priority.
    let reparsed = parse_inline_style(&text);
    assert!(reparsed.is_important("color"));
    assert!(!reparsed.is_important("width"));
}

#[test]
fn inline_style_important_on_shorthand_expands_to_longhands() {
    let style = parse_inline_style("margin: 10px !important");
    assert!(style.is_important("margin-top"));
    assert!(style.is_important("margin-left"));
}
