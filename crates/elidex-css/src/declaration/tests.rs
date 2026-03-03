use super::*;
use elidex_plugin::{CssColor, LengthUnit};

fn parse_decls(css: &str) -> Vec<Declaration> {
    parse_declaration_block(css)
}

fn parse_single(property: &str, value: &str) -> Vec<Declaration> {
    parse_decls(&format!("{property}: {value}"))
}

// =============================================================================
// 1a: Keyword property parsing (table-driven)
// =============================================================================

#[test]
#[allow(clippy::too_many_lines)]
fn parse_keyword_properties() {
    // (input_property, input_value, expected_property, expected_keyword)
    for (prop, input, expected_prop, expected_kw) in [
        // display
        ("display", "block", "display", "block"),
        ("display", "inline-block", "display", "inline-block"),
        ("display", "none", "display", "none"),
        ("display", "flex", "display", "flex"),
        ("display", "inline-flex", "display", "inline-flex"),
        ("display", "list-item", "display", "list-item"),
        ("display", "grid", "display", "grid"),
        ("display", "inline-grid", "display", "inline-grid"),
        // position
        ("position", "absolute", "position", "absolute"),
        ("position", "relative", "position", "relative"),
        ("position", "fixed", "position", "fixed"),
        ("position", "static", "position", "static"),
        // flex
        ("flex-direction", "column", "flex-direction", "column"),
        ("flex-wrap", "wrap-reverse", "flex-wrap", "wrap-reverse"),
        (
            "justify-content",
            "space-between",
            "justify-content",
            "space-between",
        ),
        ("align-items", "center", "align-items", "center"),
        ("align-self", "auto", "align-self", "auto"),
        (
            "align-content",
            "space-around",
            "align-content",
            "space-around",
        ),
        // text
        ("text-transform", "none", "text-transform", "none"),
        ("text-transform", "uppercase", "text-transform", "uppercase"),
        ("text-transform", "lowercase", "text-transform", "lowercase"),
        (
            "text-transform",
            "capitalize",
            "text-transform",
            "capitalize",
        ),
        ("text-align", "left", "text-align", "left"),
        ("text-align", "center", "text-align", "center"),
        ("text-align", "right", "text-align", "right"),
        // text-align mappings
        ("text-align", "start", "text-align", "left"),
        ("text-align", "end", "text-align", "right"),
        ("text-align", "justify", "text-align", "left"),
        // white-space
        ("white-space", "normal", "white-space", "normal"),
        ("white-space", "pre", "white-space", "pre"),
        ("white-space", "nowrap", "white-space", "nowrap"),
        ("white-space", "pre-wrap", "white-space", "pre-wrap"),
        ("white-space", "pre-line", "white-space", "pre-line"),
        // overflow (scroll/auto map to hidden)
        ("overflow", "visible", "overflow", "visible"),
        ("overflow", "hidden", "overflow", "hidden"),
        ("overflow", "scroll", "overflow", "hidden"),
        ("overflow", "auto", "overflow", "hidden"),
        // list-style-type
        ("list-style-type", "disc", "list-style-type", "disc"),
        ("list-style-type", "decimal", "list-style-type", "decimal"),
        ("list-style-type", "none", "list-style-type", "none"),
        // box model
        ("box-sizing", "content-box", "box-sizing", "content-box"),
        ("box-sizing", "border-box", "box-sizing", "border-box"),
        ("border-top-style", "solid", "border-top-style", "solid"),
        // font
        ("font-weight", "normal", "font-weight", "normal"),
        ("font-weight", "bold", "font-weight", "bold"),
        ("font-style", "normal", "font-style", "normal"),
        ("font-style", "italic", "font-style", "italic"),
        ("font-style", "oblique", "font-style", "oblique"),
        // line-height
        ("line-height", "normal", "line-height", "normal"),
        // grid
        ("grid-auto-flow", "column", "grid-auto-flow", "column"),
        ("grid-auto-flow", "row dense", "grid-auto-flow", "row dense"),
        (
            "grid-auto-flow",
            "column dense",
            "grid-auto-flow",
            "column dense",
        ),
        (
            "grid-template-columns",
            "none",
            "grid-template-columns",
            "none",
        ),
        // table
        ("border-collapse", "separate", "border-collapse", "separate"),
        ("border-collapse", "collapse", "border-collapse", "collapse"),
        ("table-layout", "auto", "table-layout", "auto"),
        ("table-layout", "fixed", "table-layout", "fixed"),
        ("caption-side", "top", "caption-side", "top"),
        ("caption-side", "bottom", "caption-side", "bottom"),
        // currentcolor
        ("color", "currentcolor", "color", "currentcolor"),
        ("color", "CurrentColor", "color", "currentcolor"),
        (
            "background-color",
            "currentcolor",
            "background-color",
            "currentcolor",
        ),
        (
            "border-top-color",
            "currentColor",
            "border-top-color",
            "currentcolor",
        ),
        // text-decoration → text-decoration-line
        ("text-decoration", "none", "text-decoration-line", "none"),
        (
            "text-decoration",
            "underline",
            "text-decoration-line",
            "underline",
        ),
        // text-decoration-line longhand
        (
            "text-decoration-line",
            "line-through",
            "text-decoration-line",
            "line-through",
        ),
        // content keywords
        ("content", "none", "content", "none"),
        ("content", "normal", "content", "normal"),
    ] {
        let decls = parse_single(prop, input);
        assert_eq!(decls.len(), 1, "{prop}: {input}");
        assert_eq!(decls[0].property, expected_prop, "{prop}: {input}");
        assert_eq!(
            decls[0].value,
            CssValue::Keyword(expected_kw.into()),
            "{prop}: {input}"
        );
    }
}

// =============================================================================
// 1b: Rejected values (table-driven)
// =============================================================================

#[test]
fn parse_rejected_values() {
    for (prop, value) in [
        ("font-weight", "heavy"),
        ("box-sizing", "padding-box"),
        ("border-radius", "50%"),
        ("border-radius", "-5px"),
        ("white-space", "break-spaces"),
        ("overflow", "clip"),
        ("text-align", "middle"),
        ("min-width", "-10px"),
        ("max-width", "-5px"),
        ("min-height", "-20%"),
        ("max-height", "-10px"),
        ("row-gap", "-10px"),
        ("gap", "-5px"),
        ("row-gap", "-50%"),
        ("border-spacing", "10%"),
        ("-webkit-xxx", "value"),
        ("list-style", "disc foo"),
        ("list-style", "unknown-value"),
    ] {
        let decls = parse_single(prop, value);
        assert!(decls.is_empty(), "{prop}: {value} should be rejected");
    }
}

// =============================================================================
// 1c: Length property parsing (table-driven)
// =============================================================================

#[test]
fn parse_length_properties() {
    for (prop, input, num, unit) in [
        ("font-size", "16px", 16.0, LengthUnit::Px),
        ("font-size", "2em", 2.0, LengthUnit::Em),
        ("font-size", "1.5rem", 1.5, LengthUnit::Rem),
        ("width", "100px", 100.0, LengthUnit::Px),
        ("margin-top", "10px", 10.0, LengthUnit::Px),
        ("padding-bottom", "5px", 5.0, LengthUnit::Px),
        ("border-top-width", "2px", 2.0, LengthUnit::Px),
        ("border-radius", "5px", 5.0, LengthUnit::Px),
        ("border-radius", "0", 0.0, LengthUnit::Px),
        ("row-gap", "10px", 10.0, LengthUnit::Px),
        ("column-gap", "20px", 20.0, LengthUnit::Px),
        ("grid-auto-columns", "50px", 50.0, LengthUnit::Px),
        ("min-width", "100px", 100.0, LengthUnit::Px),
        ("max-width", "500px", 500.0, LengthUnit::Px),
        ("min-width", "0", 0.0, LengthUnit::Px),
        ("max-width", "0", 0.0, LengthUnit::Px),
        ("line-height", "20px", 20.0, LengthUnit::Px),
        ("flex-basis", "200px", 200.0, LengthUnit::Px),
    ] {
        let decls = parse_single(prop, input);
        assert_eq!(decls.len(), 1, "{prop}: {input}");
        assert_eq!(
            decls[0].value,
            CssValue::Length(num, unit),
            "{prop}: {input}"
        );
    }
}

// =============================================================================
// 1d: Number property parsing (table-driven)
// =============================================================================

#[test]
fn parse_number_properties() {
    for (prop, input, expected) in [
        ("opacity", "0", 0.0),
        ("opacity", "0.5", 0.5),
        ("opacity", "1", 1.0),
        ("flex-grow", "2", 2.0),
        ("flex-shrink", "0", 0.0),
        ("order", "-1", -1.0),
        ("line-height", "1.5", 1.5),
        ("grid-column-start", "2", 2.0),
        ("grid-row-end", "3", 3.0),
    ] {
        let decls = parse_single(prop, input);
        assert_eq!(decls.len(), 1, "{prop}: {input}");
        assert_eq!(
            decls[0].value,
            CssValue::Number(expected),
            "{prop}: {input}"
        );
    }
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
fn parse_opacity_clamp() {
    // Negative clamped to 0
    let decls = parse_single("opacity", "-0.5");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(0.0));

    // Above 1 clamped to 1
    let decls = parse_single("opacity", "1.5");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::Number(1.0));
}

// =============================================================================
// 1e: var() reference parsing (table-driven for simple cases)
// =============================================================================

#[test]
fn parse_var_references() {
    for (prop, var_name) in [
        ("color", "--text-color"),
        ("background-color", "--bg"),
        ("width", "--w"),
        ("font-size", "--fs"),
        ("font-family", "--ff"),
        ("font-weight", "--fw"),
        ("line-height", "--lh"),
        ("display", "--d"),
    ] {
        let input = format!("var({var_name})");
        let decls = parse_single(prop, &input);
        assert_eq!(decls.len(), 1, "{prop}: {input}");
        assert_eq!(decls[0].property, prop, "{prop}: {input}");
        assert_eq!(
            decls[0].value,
            CssValue::Var(var_name.into(), None),
            "{prop}: {input}"
        );
    }
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

// =============================================================================
// 1f: Auto property parsing (table-driven)
// =============================================================================

#[test]
fn parse_auto_properties() {
    for prop in ["width", "height", "margin-top", "margin-left", "flex-basis"] {
        let decls = parse_single(prop, "auto");
        assert_eq!(decls.len(), 1, "{prop}: auto");
        assert_eq!(decls[0].value, CssValue::Auto, "{prop}: auto");
    }
}

#[test]
fn parse_none_maps_to_auto() {
    // max-width/max-height: none → Auto
    for prop in ["max-width", "max-height"] {
        let decls = parse_single(prop, "none");
        assert_eq!(decls.len(), 1, "{prop}: none");
        assert_eq!(decls[0].value, CssValue::Auto, "{prop}: none");
    }
}

// =============================================================================
// 1g: Color property parsing (table-driven)
// =============================================================================

#[test]
fn parse_color_properties() {
    for (prop, input, expected) in [
        ("color", "red", CssColor::RED),
        ("color", "#ff0000", CssColor::RED),
        ("background-color", "blue", CssColor::BLUE),
    ] {
        let decls = parse_single(prop, input);
        assert_eq!(decls.len(), 1, "{prop}: {input}");
        assert_eq!(decls[0].value, CssValue::Color(expected), "{prop}: {input}");
    }
}

// =============================================================================
// 1h: Percentage property parsing (table-driven)
// =============================================================================

#[test]
fn parse_percentage_properties() {
    for (prop, input, expected) in [
        ("height", "50%", 50.0),
        ("width", "100%", 100.0),
        ("min-height", "50%", 50.0),
        ("line-height", "150%", 150.0),
        ("row-gap", "25%", 25.0),
    ] {
        let decls = parse_single(prop, input);
        assert_eq!(decls.len(), 1, "{prop}: {input}");
        assert_eq!(
            decls[0].value,
            CssValue::Percentage(expected),
            "{prop}: {input}"
        );
    }
}

// =============================================================================
// Complex / unique tests (kept as individual functions)
// =============================================================================

// --- Font family ---

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

// --- !important ---

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
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Inherit);
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

// --- Custom properties ---

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

// --- Text decoration: multiple values ---

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

// --- Grid ---

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
fn parse_grid_column_inherit() {
    let decls = parse_single("grid-column", "inherit");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "grid-column-start");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "grid-column-end");
    assert_eq!(decls[1].value, CssValue::Inherit);
}

// --- Table: display variants (already table-driven) ---

#[test]
fn parse_display_table_variants() {
    for kw in [
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
        assert_eq!(decls.len(), 1, "display: {kw}");
        assert_eq!(
            decls[0].value,
            CssValue::Keyword(kw.into()),
            "display: {kw}"
        );
    }
}

// --- Table: border-spacing ---

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
fn parse_border_spacing_inherit() {
    let decls = parse_single("border-spacing", "inherit");
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].property, "border-spacing-h");
    assert_eq!(decls[0].value, CssValue::Inherit);
    assert_eq!(decls[1].property, "border-spacing-v");
    assert_eq!(decls[1].value, CssValue::Inherit);
}
