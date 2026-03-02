use super::*;
use elidex_plugin::{CssColor, LengthUnit};

fn parse_decls(css: &str) -> Vec<Declaration> {
    parse_declaration_block(css)
}

fn parse_single(property: &str, value: &str) -> Vec<Declaration> {
    parse_decls(&format!("{property}: {value}"))
}

#[test]
fn parse_display_block() {
    let decls = parse_single("display", "block");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "display");
    assert_eq!(decls[0].value, CssValue::Keyword("block".into()));
}

#[test]
fn parse_color_named() {
    let decls = parse_single("color", "red");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
}

#[test]
fn parse_color_hex() {
    let decls = parse_single("color", "#ff0000");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
}

#[test]
fn parse_background_color() {
    let decls = parse_single("background-color", "blue");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Color(CssColor::BLUE));
}

#[test]
fn parse_font_size_px() {
    let decls = parse_single("font-size", "16px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Length(16.0, LengthUnit::Px));
}

#[test]
fn parse_font_family_list() {
    let decls = parse_single("font-family", "Arial, sans-serif");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "font-family");
    match &decls[0].value {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn parse_font_family_multiword_unquoted() {
    let decls = parse_single("font-family", "Times New Roman, sans-serif");
    assert_eq!(decls.len(), 1);
    match &decls[0].value {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::Keyword("Times New Roman".into()));
            assert_eq!(items[1], CssValue::Keyword("sans-serif".into()));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn parse_width_auto() {
    let decls = parse_single("width", "auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Auto);
}

#[test]
fn parse_important_flag() {
    let decls = parse_decls("color: red");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
    assert!(!decls[0].important);
}

#[test]
fn parse_inline_important() {
    // Browsers support !important in inline styles.
    let decls = parse_decls("color: red !important");
    assert_eq!(decls.len(), 1);
    assert!(decls[0].important);
    assert_eq!(decls[0].value, CssValue::Color(CssColor::RED));
}

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
fn unknown_property_skipped() {
    let decls = parse_single("-webkit-xxx", "value");
    assert!(decls.is_empty());
}

#[test]
fn multiple_declarations() {
    let decls = parse_decls("color: red; display: block; width: 100px");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "color");
    assert_eq!(decls[1].property, "display");
    assert_eq!(decls[2].property, "width");
}

#[test]
fn global_keyword_inherit() {
    let decls = parse_single("color", "inherit");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Inherit);
}

#[test]
fn parse_currentcolor_keyword() {
    let decls = parse_single("color", "currentcolor");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");
    assert_eq!(decls[0].value, CssValue::Keyword("currentcolor".into()));
}

#[test]
fn parse_currentcolor_case_insensitive() {
    let decls = parse_single("background-color", "CurrentColor");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("currentcolor".into()));
}

#[test]
fn parse_border_color_currentcolor() {
    let decls = parse_single("border-top-color", "currentColor");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("currentcolor".into()));
}

// --- Flex property tests ---

#[test]
fn parse_flex_direction() {
    let decls = parse_single("flex-direction", "column");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "flex-direction");
    assert_eq!(decls[0].value, CssValue::Keyword("column".into()));
}

#[test]
fn parse_flex_wrap() {
    let decls = parse_single("flex-wrap", "wrap-reverse");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("wrap-reverse".into()));
}

#[test]
fn parse_justify_content() {
    let decls = parse_single("justify-content", "space-between");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("space-between".into()));
}

#[test]
fn parse_align_items() {
    let decls = parse_single("align-items", "center");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("center".into()));
}

#[test]
fn parse_flex_grow() {
    let decls = parse_single("flex-grow", "2");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "flex-grow");
    assert_eq!(decls[0].value, CssValue::Number(2.0));
}

#[test]
fn parse_flex_basis_auto() {
    let decls = parse_single("flex-basis", "auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Auto);
}

#[test]
fn parse_flex_basis_length() {
    let decls = parse_single("flex-basis", "200px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Length(200.0, LengthUnit::Px));
}

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

#[test]
fn parse_order() {
    let decls = parse_single("order", "-1");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "order");
    assert_eq!(decls[0].value, CssValue::Number(-1.0));
}

#[test]
fn parse_display_inline_flex() {
    let decls = parse_single("display", "inline-flex");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("inline-flex".into()));
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

// --- Custom property tests (M3-0) ---

#[test]
fn parse_custom_property_color() {
    let decls = parse_single("--bg", "#0d1117");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "--bg");
    assert_eq!(decls[0].value, CssValue::RawTokens("#0d1117".into()));
}

#[test]
fn parse_custom_property_multi_token() {
    let decls = parse_single("--font-stack", "\"Courier New\", monospace");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "--font-stack");
    assert_eq!(
        decls[0].value,
        CssValue::RawTokens("\"Courier New\", monospace".into())
    );
}

#[test]
fn parse_custom_property_keyword() {
    let decls = parse_single("--display-mode", "flex");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::RawTokens("flex".into()));
}

#[test]
fn parse_custom_property_whitespace_only_value() {
    // A custom property with only whitespace content should store as RawTokens.
    // Note: `--empty: ;` has the semicolon consumed as value by cssparser's
    // tokenizer (not a declaration separator in this context).
    // A truly empty custom property requires `--empty:;` with no space.
    let decls = parse_single("--x", "  spaces  ");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "--x");
    // collect_remaining_tokens trims whitespace.
    assert_eq!(decls[0].value, CssValue::RawTokens("spaces".into()));
}

#[test]
fn parse_var_function_simple() {
    let decls = parse_single("color", "var(--text-color)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");
    assert_eq!(decls[0].value, CssValue::Var("--text-color".into(), None));
}

#[test]
fn parse_var_function_with_color_fallback() {
    let decls = parse_single("color", "var(--text-color, #ff0000)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");
    match &decls[0].value {
        CssValue::Var(name, Some(fb)) => {
            assert_eq!(name, "--text-color");
            assert_eq!(**fb, CssValue::Color(CssColor::RED));
        }
        other => panic!("expected Var with fallback, got {other:?}"),
    }
}

#[test]
fn parse_var_function_with_keyword_fallback() {
    let decls = parse_single("display", "var(--display-mode, block)");
    assert_eq!(decls.len(), 1);
    match &decls[0].value {
        CssValue::Var(name, Some(fb)) => {
            assert_eq!(name, "--display-mode");
            assert_eq!(**fb, CssValue::Keyword("block".into()));
        }
        other => panic!("expected Var with fallback, got {other:?}"),
    }
}

#[test]
fn parse_var_function_nested() {
    let decls = parse_single("color", "var(--a, var(--b, red))");
    assert_eq!(decls.len(), 1);
    match &decls[0].value {
        CssValue::Var(name_a, Some(fb_a)) => {
            assert_eq!(name_a, "--a");
            match fb_a.as_ref() {
                CssValue::Var(name_b, Some(fb_b)) => {
                    assert_eq!(name_b, "--b");
                    assert_eq!(**fb_b, CssValue::Color(CssColor::RED));
                }
                other => panic!("expected nested Var, got {other:?}"),
            }
        }
        other => panic!("expected Var, got {other:?}"),
    }
}

#[test]
fn parse_var_in_background_color() {
    let decls = parse_single("background-color", "var(--bg)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Var("--bg".into(), None));
}

#[test]
fn parse_var_in_width() {
    let decls = parse_single("width", "var(--w)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Var("--w".into(), None));
}

#[test]
fn parse_var_in_font_size() {
    let decls = parse_single("font-size", "var(--fs)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "font-size");
    assert_eq!(decls[0].value, CssValue::Var("--fs".into(), None));
}

#[test]
fn parse_var_in_font_family() {
    let decls = parse_single("font-family", "var(--ff)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "font-family");
    assert_eq!(decls[0].value, CssValue::Var("--ff".into(), None));
}

// --- Font-weight tests (M3-1) ---

#[test]
fn parse_font_weight_normal() {
    let decls = parse_single("font-weight", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "font-weight");
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into()));
}

#[test]
fn parse_font_weight_bold() {
    let decls = parse_single("font-weight", "bold");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("bold".into()));
}

#[test]
fn parse_font_weight_numeric() {
    for weight in [100_u32, 400, 700, 900] {
        let decls = parse_single("font-weight", &weight.to_string());
        assert_eq!(decls.len(), 1, "weight={weight}");
        #[allow(clippy::cast_precision_loss)]
        let expected = weight as f32;
        assert_eq!(
            decls[0].value,
            CssValue::Number(expected),
            "weight={weight}"
        );
    }
}

#[test]
fn parse_font_weight_invalid() {
    let decls = parse_single("font-weight", "heavy");
    assert!(decls.is_empty());
}

// --- Line-height tests (M3-1) ---

#[test]
fn parse_line_height_normal() {
    let decls = parse_single("line-height", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into()));
}

#[test]
fn parse_line_height_number() {
    let decls = parse_single("line-height", "1.5");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(1.5));
}

#[test]
fn parse_line_height_px() {
    let decls = parse_single("line-height", "20px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Length(20.0, LengthUnit::Px));
}

#[test]
fn parse_line_height_percent() {
    let decls = parse_single("line-height", "150%");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Percentage(150.0));
}

// --- Text-transform tests (M3-1) ---

#[test]
fn parse_text_transform_none() {
    let decls = parse_single("text-transform", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
}

#[test]
fn parse_text_transform_uppercase() {
    let decls = parse_single("text-transform", "uppercase");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("uppercase".into()));
}

#[test]
fn parse_text_transform_lowercase() {
    let decls = parse_single("text-transform", "lowercase");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("lowercase".into()));
}

#[test]
fn parse_text_transform_capitalize() {
    let decls = parse_single("text-transform", "capitalize");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("capitalize".into()));
}

// --- Text-decoration tests (M3-1) ---

#[test]
fn parse_text_decoration_none() {
    let decls = parse_single("text-decoration", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
}

#[test]
fn parse_text_decoration_underline() {
    let decls = parse_single("text-decoration", "underline");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Keyword("underline".into()));
}

#[test]
fn parse_text_decoration_line_through() {
    let decls = parse_single("text-decoration-line", "line-through");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("line-through".into()));
}

#[test]
fn parse_text_decoration_multiple() {
    let decls = parse_single("text-decoration", "underline line-through");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-line");
    match &decls[0].value {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::Keyword("underline".into()));
            assert_eq!(items[1], CssValue::Keyword("line-through".into()));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn global_keyword_expands_text_decoration_shorthand() {
    let decls = parse_single("text-decoration", "inherit");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Inherit);
}

#[test]
fn parse_var_in_font_weight() {
    let decls = parse_single("font-weight", "var(--fw)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Var("--fw".into(), None));
}

#[test]
fn parse_var_in_line_height() {
    let decls = parse_single("line-height", "var(--lh)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Var("--lh".into(), None));
}

// --- M3-2: Box model property tests ---

#[test]
fn parse_box_sizing_content_box() {
    let decls = parse_single("box-sizing", "content-box");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "box-sizing");
    assert_eq!(decls[0].value, CssValue::Keyword("content-box".into()));
}

#[test]
fn parse_box_sizing_border_box() {
    let decls = parse_single("box-sizing", "border-box");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("border-box".into()));
}

#[test]
fn parse_box_sizing_invalid() {
    let decls = parse_single("box-sizing", "padding-box");
    assert!(decls.is_empty());
}

#[test]
fn parse_border_radius_zero() {
    let decls = parse_single("border-radius", "0");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "border-radius");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_border_radius_px() {
    let decls = parse_single("border-radius", "5px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Length(5.0, LengthUnit::Px));
}

#[test]
fn parse_border_radius_percentage_rejected() {
    // Percentages are rejected in Phase 3 — resolution requires box dimensions.
    let decls = parse_single("border-radius", "50%");
    assert!(decls.is_empty());
}

#[test]
fn parse_border_radius_negative_rejected() {
    let decls = parse_single("border-radius", "-5px");
    assert!(decls.is_empty());
}

#[test]
fn parse_opacity_zero() {
    let decls = parse_single("opacity", "0");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "opacity");
    assert_eq!(decls[0].value, CssValue::Number(0.0));
}

#[test]
fn parse_opacity_half() {
    let decls = parse_single("opacity", "0.5");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(0.5));
}

#[test]
fn parse_opacity_one() {
    let decls = parse_single("opacity", "1");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(1.0));
}

#[test]
fn parse_opacity_clamp_negative() {
    let decls = parse_single("opacity", "-0.5");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(0.0));
}

#[test]
fn parse_opacity_clamp_above_one() {
    let decls = parse_single("opacity", "1.5");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(1.0));
}

// --- M3-5: gap + text-align parsing ---

#[test]
fn parse_row_gap() {
    let decls = parse_single("row-gap", "10px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px));
}

#[test]
fn parse_column_gap() {
    let decls = parse_single("column-gap", "20px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "column-gap");
    assert_eq!(decls[0].value, CssValue::Length(20.0, LengthUnit::Px));
}

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
fn parse_text_align_left() {
    let decls = parse_single("text-align", "left");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("left".into()));
}

#[test]
fn parse_text_align_center() {
    let decls = parse_single("text-align", "center");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("center".into()));
}

#[test]
fn parse_text_align_right() {
    let decls = parse_single("text-align", "right");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("right".into()));
}

#[test]
fn parse_text_align_start_maps_to_left() {
    let decls = parse_single("text-align", "start");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("left".into()));
}

#[test]
fn parse_text_align_justify_maps_to_left() {
    let decls = parse_single("text-align", "justify");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("left".into()));
}

// L1: text-align: end → right
#[test]
fn parse_text_align_end_maps_to_right() {
    let decls = parse_single("text-align", "end");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("right".into()));
}

// L3: gap: normal → 0px
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

// M5: negative gap rejected
#[test]
fn parse_gap_negative_rejected() {
    let decls = parse_single("row-gap", "-10px");
    assert!(decls.is_empty(), "negative gap should be rejected");
}

#[test]
fn parse_gap_shorthand_negative_rejected() {
    let decls = parse_single("gap", "-5px");
    assert!(
        decls.is_empty(),
        "negative gap shorthand should be rejected"
    );
}

// L4: gap: normal + length mixed values
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

// L5: negative percentage gap rejected
#[test]
fn parse_gap_negative_percentage_rejected() {
    let decls = parse_single("row-gap", "-50%");
    assert!(
        decls.is_empty(),
        "negative percentage gap should be rejected"
    );
}

// L6: gap: 0 shorthand
#[test]
fn parse_gap_zero_shorthand() {
    let decls = parse_single("gap", "0");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "column-gap");
    assert_eq!(decls[1].value, CssValue::Length(0.0, LengthUnit::Px));
}

// L7: invalid text-align value rejected
#[test]
fn parse_text_align_invalid_rejected() {
    let decls = parse_single("text-align", "middle");
    assert!(decls.is_empty(), "invalid text-align should be rejected");
}

// --- M3-6: white-space ---

#[test]
fn parse_white_space_normal() {
    let decls = parse_single("white-space", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into()));
}

#[test]
fn parse_white_space_pre() {
    let decls = parse_single("white-space", "pre");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("pre".into()));
}

#[test]
fn parse_white_space_nowrap() {
    let decls = parse_single("white-space", "nowrap");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("nowrap".into()));
}

#[test]
fn parse_white_space_pre_wrap() {
    let decls = parse_single("white-space", "pre-wrap");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("pre-wrap".into()));
}

#[test]
fn parse_white_space_pre_line() {
    let decls = parse_single("white-space", "pre-line");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("pre-line".into()));
}

#[test]
fn parse_white_space_invalid_rejected() {
    let decls = parse_single("white-space", "break-spaces");
    assert!(decls.is_empty());
}

// --- M3-6: overflow ---

#[test]
fn parse_overflow_visible() {
    let decls = parse_single("overflow", "visible");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("visible".into()));
}

#[test]
fn parse_overflow_hidden() {
    let decls = parse_single("overflow", "hidden");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("hidden".into()));
}

#[test]
fn parse_overflow_scroll_maps_to_hidden() {
    let decls = parse_single("overflow", "scroll");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("hidden".into()));
}

#[test]
fn parse_overflow_auto_maps_to_hidden() {
    let decls = parse_single("overflow", "auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("hidden".into()));
}

// --- M3-6: min/max width/height ---

#[test]
fn parse_min_width_px() {
    let decls = parse_single("min-width", "100px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "min-width");
    assert_eq!(decls[0].value, CssValue::Length(100.0, LengthUnit::Px));
}

#[test]
fn parse_max_width_px() {
    let decls = parse_single("max-width", "500px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "max-width");
    assert_eq!(decls[0].value, CssValue::Length(500.0, LengthUnit::Px));
}

#[test]
fn parse_max_width_none() {
    let decls = parse_single("max-width", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Auto);
}

#[test]
fn parse_min_height_percentage() {
    let decls = parse_single("min-height", "50%");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "min-height");
    assert_eq!(decls[0].value, CssValue::Percentage(50.0));
}

#[test]
fn parse_max_height_none() {
    let decls = parse_single("max-height", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Auto);
}

#[test]
fn parse_min_width_negative_rejected() {
    let decls = parse_single("min-width", "-10px");
    assert!(decls.is_empty(), "negative min-width should be rejected");
}

#[test]
fn parse_max_width_negative_rejected() {
    let decls = parse_single("max-width", "-5px");
    assert!(decls.is_empty(), "negative max-width should be rejected");
}

#[test]
fn parse_min_height_negative_rejected() {
    let decls = parse_single("min-height", "-20%");
    assert!(decls.is_empty(), "negative min-height should be rejected");
}

#[test]
fn parse_min_width_zero() {
    let decls = parse_single("min-width", "0");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "min-width");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_max_width_zero() {
    let decls = parse_single("max-width", "0");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "max-width");
    assert_eq!(decls[0].value, CssValue::Length(0.0, LengthUnit::Px));
}

#[test]
fn parse_gap_positive_percentage() {
    let decls = parse_single("row-gap", "25%");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "row-gap");
    assert_eq!(decls[0].value, CssValue::Percentage(25.0));
}

// --- M3-6: list-style-type & display: list-item ---

#[test]
fn parse_display_list_item() {
    let decls = parse_single("display", "list-item");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("list-item".into()));
}

#[test]
fn parse_list_style_type_disc() {
    let decls = parse_single("list-style-type", "disc");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("disc".into()));
}

#[test]
fn parse_list_style_type_decimal() {
    let decls = parse_single("list-style-type", "decimal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("decimal".into()));
}

#[test]
fn parse_list_style_type_none() {
    let decls = parse_single("list-style-type", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
}

#[test]
fn parse_list_style_shorthand() {
    let decls = parse_single("list-style", "square");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "list-style-type");
    assert_eq!(decls[0].value, CssValue::Keyword("square".into()));
}

#[test]
fn parse_list_style_shorthand_rejects_extra_tokens() {
    // "disc foo" should be rejected entirely — no partial parse.
    let decls = parse_single("list-style", "disc foo");
    assert!(
        decls.is_empty(),
        "list-style with extra tokens should be rejected"
    );
}

#[test]
fn parse_list_style_shorthand_rejects_unknown() {
    let decls = parse_single("list-style", "unknown-value");
    assert!(
        decls.is_empty(),
        "unknown list-style-type should be rejected"
    );
}

#[test]
fn parse_list_style_shorthand_with_important() {
    let decls = parse_decls("list-style: disc !important");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "list-style-type");
    assert_eq!(decls[0].value, CssValue::Keyword("disc".into()));
    assert!(decls[0].important);
}

#[test]
fn parse_max_height_negative_rejected() {
    let decls = parse_single("max-height", "-10px");
    assert!(decls.is_empty(), "negative max-height should be rejected");
}

#[test]
fn parse_overflow_invalid_rejected() {
    let decls = parse_single("overflow", "clip");
    assert!(
        decls.is_empty(),
        "unsupported overflow value should be rejected"
    );
}

// --- background shorthand expansion ---

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

#[test]
fn global_keyword_expands_background_shorthand() {
    let decls = parse_single("background", "inherit");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "background-color");
    assert_eq!(decls[0].value, CssValue::Inherit);
}

// --- border-side shorthand expansion ---

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

// --- M3.5-0: content property ---

#[test]
fn parse_content_string() {
    let decls = parse_single("content", r#""hello""#);
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "content");
    assert_eq!(decls[0].value, CssValue::String("hello".to_string()));
}

#[test]
fn parse_content_none() {
    let decls = parse_single("content", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("none".to_string()));
}

#[test]
fn parse_content_normal() {
    let decls = parse_single("content", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("normal".to_string()));
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

// --- M3.5-1: Grid property parsing ---

#[test]
fn parse_display_grid() {
    let decls = parse_single("display", "grid");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("grid".into()));

    let decls = parse_single("display", "inline-grid");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("inline-grid".into()));
}

#[test]
fn parse_grid_template_columns_px_fr() {
    let decls = parse_single("grid-template-columns", "100px 200px 1fr");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "grid-template-columns");
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(200.0, LengthUnit::Px),
            CssValue::Length(1.0, LengthUnit::Fr),
        ])
    );
}

#[test]
fn parse_grid_template_columns_none() {
    let decls = parse_single("grid-template-columns", "none");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
}

#[test]
fn parse_grid_template_rows_minmax_auto() {
    let decls = parse_single("grid-template-rows", "minmax(100px, 1fr) auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::List(vec![
                CssValue::Keyword("minmax".into()),
                CssValue::Length(100.0, LengthUnit::Px),
                CssValue::Length(1.0, LengthUnit::Fr),
            ]),
            CssValue::Auto,
        ])
    );
}

#[test]
fn parse_grid_template_columns_repeat() {
    let decls = parse_single("grid-template-columns", "repeat(3, 1fr)");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Length(1.0, LengthUnit::Fr),
            CssValue::Length(1.0, LengthUnit::Fr),
            CssValue::Length(1.0, LengthUnit::Fr),
        ])
    );
}

#[test]
fn parse_grid_auto_flow_row_dense() {
    let decls = parse_single("grid-auto-flow", "row dense");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("row dense".into()));

    let decls = parse_single("grid-auto-flow", "column");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("column".into()));
}

#[test]
fn parse_grid_auto_columns() {
    let decls = parse_single("grid-auto-columns", "50px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Length(50.0, LengthUnit::Px));
}

#[test]
fn parse_grid_column_start_span() {
    let decls = parse_single("grid-column-start", "span 2");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Keyword("span".into()),
            CssValue::Number(2.0),
        ])
    );

    let decls = parse_single("grid-column-start", "2");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(2.0));
}

#[test]
fn parse_grid_column_shorthand() {
    let decls = parse_single("grid-column", "1 / 3");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "grid-column-start");
    assert_eq!(decls[0].value, CssValue::Number(1.0));
    assert_eq!(decls[1].property, "grid-column-end");
    assert_eq!(decls[1].value, CssValue::Number(3.0));
}

#[test]
fn parse_grid_area_shorthand() {
    let decls = parse_single("grid-area", "1 / 2 / 3 / 4");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].property, "grid-row-start");
    assert_eq!(decls[0].value, CssValue::Number(1.0));
    assert_eq!(decls[1].property, "grid-column-start");
    assert_eq!(decls[1].value, CssValue::Number(2.0));
    assert_eq!(decls[2].property, "grid-row-end");
    assert_eq!(decls[2].value, CssValue::Number(3.0));
    assert_eq!(decls[3].property, "grid-column-end");
    assert_eq!(decls[3].value, CssValue::Number(4.0));
}

#[test]
fn parse_grid_template_columns_fr_units() {
    let decls = parse_single("grid-template-columns", "1fr 2fr");
    assert_eq!(decls.len(), 1);
    assert_eq!(
        decls[0].value,
        CssValue::List(vec![
            CssValue::Length(1.0, LengthUnit::Fr),
            CssValue::Length(2.0, LengthUnit::Fr),
        ])
    );
}

#[test]
fn parse_grid_column_inherit() {
    let decls = parse_single("grid-column", "inherit");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "grid-column-start");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "grid-column-end");
    assert_eq!(decls[1].value, CssValue::Inherit);
}

// --- M3.5-2: Table CSS parsing ---

#[test]
fn parse_display_table_variants() {
    for kw in &[
        "table",
        "inline-table",
        "table-caption",
        "table-row",
        "table-cell",
        "table-row-group",
        "table-header-group",
        "table-footer-group",
        "table-column",
        "table-column-group",
    ] {
        let decls = parse_single("display", kw);
        assert_eq!(decls.len(), 1, "failed for {kw}");
        assert_eq!(decls[0].value, CssValue::Keyword((*kw).to_string()));
    }
}

#[test]
fn parse_border_collapse() {
    let decls = parse_single("border-collapse", "separate");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("separate".into()));

    let decls = parse_single("border-collapse", "collapse");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("collapse".into()));
}

#[test]
fn parse_border_spacing_one_value() {
    let decls = parse_single("border-spacing", "2px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Length(2.0, LengthUnit::Px));
}

#[test]
fn parse_border_spacing_two_values() {
    let decls = parse_single("border-spacing", "2px 4px");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Length(4.0, LengthUnit::Px));
}

#[test]
fn parse_table_layout() {
    let decls = parse_single("table-layout", "auto");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("auto".into()));

    let decls = parse_single("table-layout", "fixed");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("fixed".into()));
}

#[test]
fn parse_caption_side() {
    let decls = parse_single("caption-side", "top");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("top".into()));

    let decls = parse_single("caption-side", "bottom");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Keyword("bottom".into()));
}

#[test]
fn parse_border_spacing_percentage_rejected() {
    // CSS 2.1 §17.6.1: border-spacing does not accept percentages.
    let decls = parse_single("border-spacing", "10%");
    assert!(
        decls.is_empty(),
        "border-spacing should reject percentage values"
    );
}

#[test]
fn parse_border_spacing_inherit() {
    let decls = parse_single("border-spacing", "inherit");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Inherit);
}
