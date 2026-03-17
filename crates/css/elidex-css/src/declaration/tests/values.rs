use super::*;

// =============================================================================
// 1a: Keyword property parsing (table-driven)
// =============================================================================

#[test]
#[allow(clippy::too_many_lines)]
// Test setup with multiple assertions.
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
        ("text-align", "start", "text-align", "start"),
        ("text-align", "end", "text-align", "end"),
        ("text-align", "justify", "text-align", "justify"),
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
        ("overflow", "clip", "overflow", "hidden"),
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
        ("overflow", "overlay"),
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

#[test]
fn parse_border_radius_shorthand() {
    // 1 value → all 4 corners
    let decls = parse_single("border-radius", "5px");
    assert_eq!(decls.len(), 4);
    for d in &decls {
        assert_eq!(d.value, CssValue::Length(5.0, LengthUnit::Px));
    }
    assert_eq!(decls[0].property, "border-top-left-radius");
    assert_eq!(decls[1].property, "border-top-right-radius");
    assert_eq!(decls[2].property, "border-bottom-right-radius");
    assert_eq!(decls[3].property, "border-bottom-left-radius");

    // 2 values → TL+BR / TR+BL
    let decls = parse_single("border-radius", "10px 20px");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px)); // TL
    assert_eq!(decls[1].value, CssValue::Length(20.0, LengthUnit::Px)); // TR
    assert_eq!(decls[2].value, CssValue::Length(10.0, LengthUnit::Px)); // BR
    assert_eq!(decls[3].value, CssValue::Length(20.0, LengthUnit::Px)); // BL

    // 4 values → TL / TR / BR / BL
    let decls = parse_single("border-radius", "1px 2px 3px 4px");
    assert_eq!(decls.len(), 4);
    assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));
    assert_eq!(decls[1].value, CssValue::Length(2.0, LengthUnit::Px));
    assert_eq!(decls[2].value, CssValue::Length(3.0, LengthUnit::Px));
    assert_eq!(decls[3].value, CssValue::Length(4.0, LengthUnit::Px));

    // 0 → all corners zero
    let decls = parse_single("border-radius", "0");
    assert_eq!(decls.len(), 4);
    for d in &decls {
        assert_eq!(d.value, CssValue::Length(0.0, LengthUnit::Px));
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
// 1e: Auto property parsing (table-driven)
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
// 1f: Color property parsing (table-driven)
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
// 1g: Percentage property parsing (table-driven)
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

// --- var() references ---

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

// --- Text decoration: multiple values ---

#[test]
fn parse_text_decoration_multiple() {
    let decls = parse_single("text-decoration", "underline line-through");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "text-decoration-line");
    match &decls[0].value {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::Keyword("underline".into()));
            assert_eq!(items[1], CssValue::Keyword("line-through".into()));
        }
        other => panic!("expected List, got {other:?}"),
    }
    assert_eq!(decls[1].property, "text-decoration-style");
    assert_eq!(decls[1].value, CssValue::Keyword("solid".into()));
    assert_eq!(decls[2].property, "text-decoration-color");
    assert_eq!(decls[2].value, CssValue::Keyword("currentcolor".into()));
}

// --- M4-1: text-decoration shorthand with style ---

#[test]
fn parse_text_decoration_shorthand_with_style() {
    let decls = parse_single("text-decoration", "underline dashed");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Keyword("underline".into()));
    assert_eq!(decls[1].property, "text-decoration-style");
    assert_eq!(decls[1].value, CssValue::Keyword("dashed".into()));
}

#[test]
fn parse_text_decoration_shorthand_none() {
    let decls = parse_single("text-decoration", "none");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
    assert_eq!(decls[1].property, "text-decoration-style");
    assert_eq!(decls[1].value, CssValue::Keyword("solid".into()));
}

#[test]
fn parse_text_decoration_none_with_style_and_color() {
    // "none dashed red" — "none" is the line value, style and color should still parse.
    let decls = parse_single("text-decoration", "none dashed red");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Keyword("none".into()));
    assert_eq!(decls[1].property, "text-decoration-style");
    assert_eq!(decls[1].value, CssValue::Keyword("dashed".into()));
    assert_eq!(decls[2].property, "text-decoration-color");
    // "red" parsed as Color.
    assert!(matches!(decls[2].value, CssValue::Color(_)));
}

#[test]
fn parse_text_decoration_none_exclusive_with_line_keywords() {
    // "none underline" — CSS Text Decoration Level 3: none is exclusive.
    // Trailing unparsed tokens make the shorthand invalid → empty result.
    let decls = parse_single("text-decoration", "none underline");
    assert!(
        decls.is_empty(),
        "none + line keyword should be invalid (trailing tokens), got: {decls:?}"
    );

    // "underline none" — reverse order, also invalid due to trailing "none" token.
    let decls2 = parse_single("text-decoration", "underline none");
    assert!(
        decls2.is_empty(),
        "line keyword + none should be invalid (trailing tokens), got: {decls2:?}"
    );
}

#[test]
fn parse_text_decoration_style_longhand() {
    for kw in &["solid", "double", "dotted", "dashed", "wavy"] {
        let decls = parse_single("text-decoration-style", kw);
        assert_eq!(decls.len(), 1, "text-decoration-style: {kw}");
        assert_eq!(decls[0].property, "text-decoration-style");
        assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
    }
}

#[test]
fn parse_text_decoration_color_longhand() {
    let decls = parse_single("text-decoration-color", "red");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-color");
    assert!(matches!(decls[0].value, CssValue::Color(_)));
}

#[test]
fn parse_text_decoration_color_currentcolor() {
    let decls = parse_single("text-decoration-color", "currentcolor");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-color");
    assert_eq!(decls[0].value, CssValue::Keyword("currentcolor".into()));
}

// --- M4-1: letter-spacing / word-spacing ---

#[test]
fn parse_letter_spacing_normal() {
    let decls = parse_single("letter-spacing", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "letter-spacing");
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into()));
}

#[test]
fn parse_letter_spacing_length() {
    let decls = parse_single("letter-spacing", "2px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "letter-spacing");
    assert_eq!(
        decls[0].value,
        CssValue::Length(2.0, elidex_plugin::LengthUnit::Px)
    );
}

#[test]
fn parse_word_spacing_normal() {
    let decls = parse_single("word-spacing", "normal");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "word-spacing");
    assert_eq!(decls[0].value, CssValue::Keyword("normal".into()));
}

#[test]
fn parse_word_spacing_length() {
    let decls = parse_single("word-spacing", "4px");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "word-spacing");
    assert_eq!(
        decls[0].value,
        CssValue::Length(4.0, elidex_plugin::LengthUnit::Px)
    );
}

#[test]
fn parse_letter_spacing_rejects_percentage() {
    let decls = parse_single("letter-spacing", "50%");
    assert!(
        decls.is_empty(),
        "letter-spacing must not accept percentages"
    );
}

#[test]
fn parse_word_spacing_rejects_percentage() {
    let decls = parse_single("word-spacing", "10%");
    assert!(decls.is_empty(), "word-spacing must not accept percentages");
}

#[test]
fn parse_letter_spacing_rejects_calc_with_percentage() {
    let decls = parse_single("letter-spacing", "calc(10px + 5%)");
    assert!(
        decls.is_empty(),
        "letter-spacing must not accept calc() with percentages"
    );
}

#[test]
fn parse_letter_spacing_accepts_calc_without_percentage() {
    let decls = parse_single("letter-spacing", "calc(10px + 2em)");
    assert_eq!(
        decls.len(),
        1,
        "letter-spacing should accept calc() with lengths only"
    );
    assert_eq!(decls[0].property, "letter-spacing");
    assert!(matches!(decls[0].value, CssValue::Calc(_)));
}

#[test]
fn parse_word_spacing_accepts_calc_without_percentage() {
    let decls = parse_single("word-spacing", "calc(5px + 1em)");
    assert_eq!(
        decls.len(),
        1,
        "word-spacing should accept calc() with lengths only"
    );
    assert_eq!(decls[0].property, "word-spacing");
    assert!(matches!(decls[0].value, CssValue::Calc(_)));
}

#[test]
fn parse_text_decoration_overline() {
    let decls = parse_single("text-decoration-line", "overline");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "text-decoration-line");
    assert_eq!(decls[0].value, CssValue::Keyword("overline".into()));
}

// --- Compound var() in multi-token values (CSS Variables Level 1 §3) ---

#[test]
fn compound_var_stored_as_raw_tokens() {
    // A compound value containing var() should be stored as RawTokens.
    let decls = parse_single("border", "var(--bw) solid var(--bc)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "border");
    assert!(
        matches!(&decls[0].value, CssValue::RawTokens(raw) if raw.contains("var(")),
        "compound var() should be stored as RawTokens, got {:?}",
        decls[0].value
    );
}

#[test]
fn compound_var_margin_stored_as_raw_tokens() {
    let decls = parse_single("margin", "0 var(--x)");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "margin");
    assert!(matches!(&decls[0].value, CssValue::RawTokens(_)));
}

#[test]
fn whole_value_var_still_parsed_as_var() {
    // A single var() as the entire value should still use CssValue::Var.
    let decls = parse_single("color", "var(--x)");
    assert_eq!(decls.len(), 1);
    assert!(
        matches!(&decls[0].value, CssValue::Var(name, _) if name == "--x"),
        "whole-value var() should use Var variant, got {:?}",
        decls[0].value
    );
}

#[test]
fn var_with_trailing_tokens_uses_raw_tokens() {
    // var() followed by extra tokens — not exhaustive, so compound path.
    let decls = parse_single("border", "var(--bw) solid red");
    assert_eq!(decls.len(), 1);
    assert!(
        matches!(&decls[0].value, CssValue::RawTokens(raw) if raw.contains("var(")),
        "var() with trailing tokens: {:?}",
        decls[0].value
    );
}
