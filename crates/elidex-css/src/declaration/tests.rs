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
