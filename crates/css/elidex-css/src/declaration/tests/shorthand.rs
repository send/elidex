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
    assert_eq!(decls.len(), 8);
    assert_eq!(decls[0].property, "background-color");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "background-image");
    assert_eq!(decls[1].value, CssValue::Inherit);
    assert_eq!(decls[7].property, "background-attachment");
    assert_eq!(decls[7].value, CssValue::Inherit);
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

// --- Font shorthand ---

#[test]
fn font_shorthand_size_and_family() {
    let decls = parse_single("font", "16px Arial");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[0].property, "font-style");
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into()));
    assert_eq!(decls[1].property, "font-weight");
    assert_eq!(decls[1].value, CssValue::Keyword("normal".into()));
    assert_eq!(decls[2].property, "font-size");
    assert_eq!(decls[2].value, CssValue::Length(16.0, LengthUnit::Px));
    assert_eq!(decls[3].property, "line-height");
    assert_eq!(decls[3].value, CssValue::Keyword("normal".into()));
    assert_eq!(decls[4].property, "font-family");
    assert_eq!(
        decls[4].value,
        CssValue::List(vec![CssValue::Keyword("Arial".into())])
    );
}

#[test]
fn font_shorthand_full() {
    let decls = parse_single("font", r#"italic bold 16px/1.5 "Times New Roman", serif"#);
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[0].property, "font-style");
    assert_eq!(decls[0].value, CssValue::Keyword("italic".into()));
    assert_eq!(decls[1].property, "font-weight");
    assert_eq!(decls[1].value, CssValue::Keyword("bold".into()));
    assert_eq!(decls[2].property, "font-size");
    assert_eq!(decls[2].value, CssValue::Length(16.0, LengthUnit::Px));
    assert_eq!(decls[3].property, "line-height");
    assert_eq!(decls[3].value, CssValue::Number(1.5));
    assert_eq!(decls[4].property, "font-family");
    assert_eq!(
        decls[4].value,
        CssValue::List(vec![
            CssValue::String("Times New Roman".into()),
            CssValue::Keyword("serif".into()),
        ])
    );
}

#[test]
fn font_shorthand_weight_only() {
    let decls = parse_single("font", "bold 14px sans-serif");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into())); // style
    assert_eq!(decls[1].value, CssValue::Keyword("bold".into())); // weight
    assert_eq!(decls[2].value, CssValue::Length(14.0, LengthUnit::Px));
}

#[test]
fn font_shorthand_numeric_weight() {
    let decls = parse_single("font", "300 12px monospace");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into())); // style
    assert_eq!(decls[1].value, CssValue::Number(300.0)); // weight
    assert_eq!(decls[2].value, CssValue::Length(12.0, LengthUnit::Px));
    assert_eq!(
        decls[4].value,
        CssValue::List(vec![CssValue::Keyword("monospace".into())])
    );
}

#[test]
fn font_shorthand_style_and_numeric_weight() {
    let decls = parse_single("font", "italic 700 20px Georgia");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[0].value, CssValue::Keyword("italic".into()));
    assert_eq!(decls[1].value, CssValue::Number(700.0));
    assert_eq!(decls[2].value, CssValue::Length(20.0, LengthUnit::Px));
}

#[test]
fn font_shorthand_line_height_length() {
    let decls = parse_single("font", "16px/24px Arial");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[2].value, CssValue::Length(16.0, LengthUnit::Px));
    assert_eq!(decls[3].value, CssValue::Length(24.0, LengthUnit::Px));
}

#[test]
fn font_shorthand_keyword_size() {
    let decls = parse_single("font", "small serif");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[2].value, CssValue::Keyword("small".into()));
    assert_eq!(
        decls[4].value,
        CssValue::List(vec![CssValue::Keyword("serif".into())])
    );
}

#[test]
fn font_shorthand_multi_word_family() {
    let decls = parse_single("font", "16px Times New Roman");
    assert_eq!(decls.len(), 5);
    assert_eq!(
        decls[4].value,
        CssValue::List(vec![CssValue::Keyword("Times New Roman".into())])
    );
}

#[test]
fn font_shorthand_multiple_families() {
    let decls = parse_single("font", "16px Arial, Helvetica, sans-serif");
    assert_eq!(decls.len(), 5);
    assert_eq!(
        decls[4].value,
        CssValue::List(vec![
            CssValue::Keyword("Arial".into()),
            CssValue::Keyword("Helvetica".into()),
            CssValue::Keyword("sans-serif".into()),
        ])
    );
}

#[test]
fn font_shorthand_global_keyword_expand() {
    let decls = parse_single("font", "inherit");
    assert_eq!(decls.len(), 5);
    assert_eq!(decls[0].property, "font-style");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[4].property, "font-family");
    assert_eq!(decls[4].value, CssValue::Inherit);
}

#[test]
fn font_shorthand_invalid_missing_family() {
    // font-size without font-family is invalid.
    let decls = parse_single("font", "16px");
    assert!(decls.is_empty());
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

// --- column-rule shorthand ---

#[test]
fn column_rule_all_three() {
    let decls = parse_single("column-rule", "2px solid red");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "column-rule-width");
    assert_eq!(decls[0].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-rule-style");
    assert_eq!(decls[1].value, CssValue::Keyword("solid".to_string()));
    assert_eq!(decls[2].property, "column-rule-color");
    assert_eq!(
        decls[2].value,
        CssValue::Color(CssColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        })
    );
}

#[test]
fn column_rule_style_only() {
    // Omitted components reset to initial values.
    let decls = parse_single("column-rule", "dashed");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "column-rule-width");
    assert_eq!(decls[0].value, CssValue::Length(3.0, LengthUnit::Px)); // medium
    assert_eq!(decls[1].property, "column-rule-style");
    assert_eq!(decls[1].value, CssValue::Keyword("dashed".to_string()));
    assert_eq!(decls[2].property, "column-rule-color");
    assert_eq!(
        decls[2].value,
        CssValue::Keyword("currentcolor".to_string())
    );
}

#[test]
fn column_rule_any_order() {
    // Input order differs from output — output always: width, style, color.
    let decls = parse_single("column-rule", "red thick dotted");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "column-rule-width");
    assert_eq!(decls[0].value, CssValue::Length(5.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-rule-style");
    assert_eq!(decls[1].value, CssValue::Keyword("dotted".to_string()));
    assert_eq!(decls[2].property, "column-rule-color");
}

// --- columns shorthand ---

#[test]
fn columns_both() {
    let decls = parse_single("columns", "200px 3");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "column-width");
    assert_eq!(decls[0].value, CssValue::Length(200.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-count");
    assert_eq!(decls[1].value, CssValue::Number(3.0));
}

#[test]
fn columns_auto() {
    // "auto" matches column-count; omitted column-width resets to auto.
    let decls = parse_single("columns", "auto");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "column-width");
    assert_eq!(decls[0].value, CssValue::Auto);
    assert_eq!(decls[1].property, "column-count");
    assert_eq!(decls[1].value, CssValue::Auto);
}

#[test]
fn columns_count_only() {
    // Omitted column-width resets to initial (auto).
    let decls = parse_single("columns", "3");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "column-width");
    assert_eq!(decls[0].value, CssValue::Auto);
    assert_eq!(decls[1].property, "column-count");
    assert_eq!(decls[1].value, CssValue::Number(3.0));
}
