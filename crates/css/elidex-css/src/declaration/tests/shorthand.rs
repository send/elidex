use super::*;

// --- Margin / padding shorthand ---

#[test]
fn expand_margin_one() {
    let decls = parse_single("margin", "10px");
    assert_eq!(decls.len(), 4);
    for d in &decls {
        assert_eq!(d.value, CssValue::Length(10.0, LengthUnit::Px));
    }
    assert_eq!(decls[0].property, "margin-top");
    assert_eq!(decls[1].property, "margin-right");
    assert_eq!(decls[2].property, "margin-bottom");
    assert_eq!(decls[3].property, "margin-left");
}

#[test]
fn expand_margin_two() {
    let decls = parse_single("margin", "10px 20px");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px)); // top
    assert_eq!(decls[1].value, CssValue::Length(20.0, LengthUnit::Px)); // right
    assert_eq!(decls[2].value, CssValue::Length(10.0, LengthUnit::Px)); // bottom
    assert_eq!(decls[3].value, CssValue::Length(20.0, LengthUnit::Px)); // left
}

#[test]
fn expand_margin_four() {
    let decls = parse_single("margin", "1px 2px 3px 4px");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));
    assert_eq!(decls[1].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[2].value, CssValue::Length(3.0, LengthUnit::Px));
    assert_eq!(decls[3].value, CssValue::Length(4.0, LengthUnit::Px));
}

#[test]
fn expand_padding() {
    let decls = parse_single("padding", "5px 10px");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].property, "padding-top");
    assert_eq!(decls[0].value, CssValue::Length(5.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "padding-right");
    assert_eq!(decls[1].value, CssValue::Length(10.0, LengthUnit::Px));
}

// --- Global keyword expansion ---

#[test]
fn global_keyword_expands_margin_shorthand() {
    let decls = parse_single("margin", "inherit");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].property, "margin-top");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "margin-right");
    assert_eq!(decls[2].property, "margin-bottom");
    assert_eq!(decls[3].property, "margin-left");
}

#[test]
fn global_keyword_expands_border_shorthand() {
    let decls = parse_single("border", "initial");
    assert_eq!(decls.len(), 12);
    assert_eq!(decls[0].property, "border-top-width");
    assert_eq!(decls[0].value, CssValue::Initial);
}

#[test]
fn global_keyword_longhand_unchanged() {
    let decls = parse_single("color", "inherit");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");
    assert_eq!(decls[0].value, CssValue::Inherit);
}

#[test]
fn global_keyword_expands_flex_shorthand() {
    let decls = parse_single("flex", "inherit");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "flex-grow");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "flex-shrink");
    assert_eq!(decls[2].property, "flex-basis");
}

#[test]
fn global_keyword_expands_text_decoration_shorthand() {
    let decls = parse_single("text-decoration", "inherit");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "text-decoration-style");
    assert_eq!(decls[1].value, CssValue::Inherit);
    assert_eq!(decls[2].property, "text-decoration-color");
    assert_eq!(decls[2].value, CssValue::Inherit);
}

#[test]
fn global_keyword_expands_background_shorthand() {
    let decls = parse_single("background", "inherit");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "background-color");
    assert_eq!(decls[0].value, CssValue::Inherit);
}

#[test]
fn global_keyword_expands_border_bottom_shorthand() {
    let decls = parse_single("border-bottom", "inherit");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "border-bottom-width");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "border-bottom-style");
    assert_eq!(decls[1].value, CssValue::Inherit);
    assert_eq!(decls[2].property, "border-bottom-color");
    assert_eq!(decls[2].value, CssValue::Inherit);
}

// --- Border shorthand ---

#[test]
fn expand_border() {
    let decls = parse_single("border", "1px solid black");
    assert_eq!(decls.len(), 12);
    // Check first side (top).
    assert_eq!(decls[0].property, "border-top-width");
    assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-top-style");
    assert_eq!(decls[1].value, CssValue::Keyword("solid".into()));
    assert_eq!(decls[2].property, "border-top-color");
    assert_eq!(decls[2].value, CssValue::Color(CssColor::BLACK));
}

#[test]
fn expand_border_bottom_full() {
    let decls = parse_single("border-bottom", "1px solid red");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "border-bottom-width");
    assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-bottom-style");
    assert_eq!(decls[1].value, CssValue::Keyword("solid".into()));
    assert_eq!(decls[2].property, "border-bottom-color");
    assert_eq!(decls[2].value, CssValue::Color(CssColor::RED));
}

#[test]
fn expand_border_top_full() {
    let decls = parse_single("border-top", "2px dashed blue");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "border-top-width");
    assert_eq!(decls[0].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-top-style");
    assert_eq!(decls[1].value, CssValue::Keyword("dashed".into()));
    assert_eq!(decls[2].property, "border-top-color");
    assert_eq!(decls[2].value, CssValue::Color(CssColor::BLUE));
}

#[test]
fn expand_border_left_style_only() {
    let decls = parse_single("border-left", "none");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "border-left-width");
    assert_eq!(decls[0].value, CssValue::Length(3.0, LengthUnit::Px)); // default: medium
    assert_eq!(decls[1].property, "border-left-style");
    assert_eq!(decls[1].value, CssValue::Keyword("none".into()));
    assert_eq!(decls[2].property, "border-left-color");
    assert_eq!(decls[2].value, CssValue::Keyword("currentcolor".into()));
}

// --- Multiple declarations ---

#[test]
fn multiple_declarations() {
    let decls = parse_decls("color: red; display: block; width: 100px");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "color");
    assert_eq!(decls[1].property, "display");
    assert_eq!(decls[2].property, "width");
}

// --- Flex shorthands ---

#[test]
fn parse_flex_shorthand_none() {
    let decls = parse_single("flex", "none");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "flex-grow");
    assert_eq!(decls[0].value, CssValue::Number(0.0));
    assert_eq!(decls[1].property, "flex-shrink");
    assert_eq!(decls[1].value, CssValue::Number(0.0));
    assert_eq!(decls[2].property, "flex-basis");
    assert_eq!(decls[2].value, CssValue::Auto);
}

#[test]
fn parse_flex_shorthand_auto() {
    let decls = parse_single("flex", "auto");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].value, CssValue::Number(1.0));
    assert_eq!(decls[1].value, CssValue::Number(1.0));
    assert_eq!(decls[2].value, CssValue::Auto);
}

#[test]
fn parse_flex_shorthand_single_number() {
    // flex: 2 → 2 1 0
    let decls = parse_single("flex", "2");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].value, CssValue::Number(2.0));
    assert_eq!(decls[1].value, CssValue::Number(1.0));
    assert_eq!(decls[2].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_flex_shorthand_full() {
    let decls = parse_single("flex", "2 3 100px");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].value, CssValue::Number(2.0));
    assert_eq!(decls[1].value, CssValue::Number(3.0));
    assert_eq!(decls[2].value, CssValue::Length(100.0, LengthUnit::Px));
}

#[test]
fn parse_flex_flow_shorthand() {
    let decls = parse_single("flex-flow", "column wrap");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "flex-direction");
    assert_eq!(decls[0].value, CssValue::Keyword("column".into()));
    assert_eq!(decls[1].property, "flex-wrap");
    assert_eq!(decls[1].value, CssValue::Keyword("wrap".into()));
}

#[test]
fn parse_flex_flow_direction_only() {
    let decls = parse_single("flex-flow", "row-reverse");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].value, CssValue::Keyword("row-reverse".into()));
    assert_eq!(decls[1].value, CssValue::Keyword("nowrap".into()));
}

// --- Gap shorthand ---

#[test]
fn parse_gap_shorthand_one_value() {
    let decls = parse_single("gap", "15px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(15.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(15.0, LengthUnit::Px));
}

#[test]
fn parse_gap_shorthand_two_values() {
    let decls = parse_single("gap", "10px 20px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(20.0, LengthUnit::Px));
}

#[test]
fn parse_gap_normal_keyword() {
    let decls = parse_single("gap", "normal");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_row_gap_normal_keyword() {
    let decls = parse_single("row-gap", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_gap_shorthand_normal_and_length() {
    let decls = parse_single("gap", "normal 10px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(10.0, LengthUnit::Px));
}

#[test]
fn parse_gap_shorthand_length_and_normal() {
    let decls = parse_single("gap", "10px normal");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_gap_zero_shorthand() {
    let decls = parse_single("gap", "0");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(0.0, LengthUnit::Px));
}

// --- List-style shorthand ---

#[test]
fn parse_list_style_shorthand() {
    let decls = parse_single("list-style", "square");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "list-style-type");
    assert_eq!(decls[0].value, CssValue::Keyword("square".into()));
}

#[test]
fn parse_list_style_shorthand_with_important() {
    let decls = parse_decls("list-style: disc !important");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "list-style-type");
    assert_eq!(decls[0].value, CssValue::Keyword("disc".into()));
    assert!(decls[0].important);
}

// --- Background shorthand ---

#[test]
fn parse_background_hex_color() {
    let decls = parse_single("background", "#ff0000");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "background-color");
    assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
}

#[test]
fn parse_background_named_color() {
    let decls = parse_single("background", "red");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "background-color");
    assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
}

// --- Content property ---

#[test]
fn parse_content_string() {
    let decls = parse_single("content", r#""hello""#);
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "content");
    assert_eq!(decls[0].value, CssValue::String("hello".to_string()));
}

#[test]
fn parse_content_multiple_strings() {
    let decls = parse_single("content", r#""a" "b""#);
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::String("a".to_string()),
            CssValue::String("b".to_string()),
        ])
    );
}

#[test]
fn parse_content_attr() {
    let decls = parse_single("content", "attr(title)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("attr:title".to_string()));
}
