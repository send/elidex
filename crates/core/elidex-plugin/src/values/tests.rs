use super::*;

#[test]
fn css_value_keyword() {
    let v = CssValue::Keyword("block".into());
    assert_eq!(v, CssValue::Keyword("block".into()));
}

#[test]
fn css_value_length() {
    let v = CssValue::Length(10.0, LengthUnit::Px);
    assert_eq!(v, CssValue::Length(10.0, LengthUnit::Px));
}

#[test]
fn css_value_color() {
    let v = CssValue::Color(CssColor::RED);
    assert_eq!(v, CssValue::Color(CssColor::rgb(255, 0, 0)));
}

#[test]
fn css_value_number() {
    let v = CssValue::Number(1.5);
    assert_eq!(v, CssValue::Number(1.5));
}

#[test]
fn css_value_percentage() {
    let v = CssValue::Percentage(50.0);
    assert_eq!(v, CssValue::Percentage(50.0));
}

#[test]
fn css_value_string() {
    let v = CssValue::String("hello".into());
    assert_eq!(v, CssValue::String("hello".into()));
}

#[test]
fn css_value_global_keywords() {
    assert_ne!(CssValue::Auto, CssValue::Initial);
    assert_ne!(CssValue::Inherit, CssValue::Unset);
}

#[test]
fn css_color_new_and_rgb() {
    let c = CssColor::new(10, 20, 30, 128);
    assert_eq!(c.r, 10);
    assert_eq!(c.a, 128);

    let opaque = CssColor::rgb(10, 20, 30);
    assert_eq!(opaque.a, 255);
}

#[test]
fn css_color_named_constants() {
    assert_eq!(CssColor::BLACK, CssColor::rgb(0, 0, 0));
    assert_eq!(CssColor::WHITE, CssColor::rgb(255, 255, 255));
    assert_eq!(CssColor::RED, CssColor::rgb(255, 0, 0));
    assert_eq!(CssColor::GREEN, CssColor::rgb(0, 128, 0));
    assert_eq!(CssColor::BLUE, CssColor::rgb(0, 0, 255));
    assert_eq!(CssColor::TRANSPARENT, CssColor::new(0, 0, 0, 0));
}

#[test]
fn css_color_display_opaque() {
    assert_eq!(CssColor::RED.to_string(), "#ff0000");
    assert_eq!(CssColor::BLACK.to_string(), "#000000");
}

#[test]
fn css_color_display_alpha() {
    let c = CssColor::new(255, 0, 0, 128);
    let s = c.to_string();
    assert!(s.starts_with("rgba(255, 0, 0, "));
}

#[test]
fn css_color_default_is_transparent() {
    assert_eq!(CssColor::default(), CssColor::new(0, 0, 0, 0));
}

#[test]
fn length_unit_clone_debug() {
    let u = LengthUnit::Em;
    let u2 = u;
    assert_eq!(u, u2);
    assert_eq!(format!("{u:?}"), "Em");
}

#[test]
fn length_unit_fr() {
    let v = CssValue::Length(1.0, LengthUnit::Fr);
    assert_eq!(v.as_length(), Some((1.0, LengthUnit::Fr)));
}

#[test]
fn css_value_list() {
    let v = CssValue::List(vec![
        CssValue::String("Arial".into()),
        CssValue::Keyword("sans-serif".into()),
    ]);
    match &v {
        CssValue::List(items) => assert_eq!(items.len(), 2),
        _ => panic!("expected List"),
    }
}

#[test]
fn css_value_as_keyword() {
    let v = CssValue::Keyword("block".into());
    assert_eq!(v.as_keyword(), Some("block"));
    assert_eq!(CssValue::Auto.as_keyword(), None);
}

#[test]
fn css_value_as_color() {
    let v = CssValue::Color(CssColor::RED);
    assert_eq!(v.as_color(), Some(&CssColor::RED));
    assert_eq!(CssValue::Auto.as_color(), None);
}

#[test]
fn css_value_as_number_accessor() {
    let v = CssValue::Number(1.5);
    assert_eq!(v.as_number(), Some(1.5));
    assert_eq!(CssValue::Auto.as_number(), None);
}

#[test]
fn css_value_as_percentage_accessor() {
    let v = CssValue::Percentage(50.0);
    assert_eq!(v.as_percentage(), Some(50.0));
    assert_eq!(CssValue::Auto.as_percentage(), None);
}

#[test]
fn css_value_is_auto() {
    assert!(CssValue::Auto.is_auto());
    assert!(!CssValue::Initial.is_auto());
    assert!(!CssValue::Number(0.0).is_auto());
}

#[test]
fn css_value_is_global_keyword() {
    assert!(CssValue::Initial.is_global_keyword());
    assert!(CssValue::Inherit.is_global_keyword());
    assert!(CssValue::Unset.is_global_keyword());
    assert!(!CssValue::Auto.is_global_keyword());
    assert!(!CssValue::Keyword("block".into()).is_global_keyword());
}

#[test]
fn css_value_as_length() {
    let v = CssValue::Length(10.0, LengthUnit::Px);
    assert_eq!(v.as_length(), Some((10.0, LengthUnit::Px)));
    assert_eq!(CssValue::Auto.as_length(), None);
}

#[test]
fn calc_expr_add() {
    let expr = CalcExpr::Add(
        Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
        Box::new(CalcExpr::Length(20.0, LengthUnit::Px)),
    );
    let val = CssValue::Calc(Box::new(expr));
    assert!(matches!(val, CssValue::Calc(_)));
}

#[test]
fn calc_expr_nested() {
    // (10px + 5px) * 2
    let sum = CalcExpr::Add(
        Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
        Box::new(CalcExpr::Length(5.0, LengthUnit::Px)),
    );
    let expr = CalcExpr::Mul(Box::new(sum), Box::new(CalcExpr::Number(2.0)));
    let val = CssValue::Calc(Box::new(expr));
    assert!(matches!(val, CssValue::Calc(_)));
}

#[test]
fn to_css_string_raw_tokens() {
    let val = CssValue::RawTokens("#0d1117".into());
    assert_eq!(val.to_css_string(), "#0d1117");
}

#[test]
fn to_css_string_var() {
    let val = CssValue::Var("--bg".into(), None);
    assert_eq!(val.to_css_string(), "var(--bg)");

    let val_fb = CssValue::Var(
        "--bg".into(),
        Some(Box::new(CssValue::Keyword("red".into()))),
    );
    assert_eq!(val_fb.to_css_string(), "var(--bg, red)");
}

#[test]
fn to_css_string_transform_list() {
    let val = CssValue::TransformList(vec![
        TransformFunction::Rotate(45.0),
        TransformFunction::Translate(
            CssValue::Length(10.0, LengthUnit::Px),
            CssValue::Length(20.0, LengthUnit::Px),
        ),
    ]);
    assert_eq!(val.to_css_string(), "rotate(45deg) translate(10px, 20px)");
}

#[test]
fn to_css_string_transform_none() {
    let val = CssValue::TransformList(vec![]);
    assert_eq!(val.to_css_string(), "");
}

#[test]
fn to_css_string_perspective_func_none() {
    let val = CssValue::TransformList(vec![TransformFunction::PerspectiveFunc(0.0)]);
    assert_eq!(val.to_css_string(), "perspective(none)");
}

#[test]
fn to_css_string_matrix3d() {
    let m = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 0.0, 1.0,
    ];
    let val = CssValue::TransformList(vec![TransformFunction::Matrix3d(m)]);
    let result = val.to_css_string();
    assert!(result.starts_with("matrix3d("));
    assert!(result.contains("10"));
}

#[test]
fn to_css_string_calc_simple() {
    let expr = CalcExpr::Sub(
        Box::new(CalcExpr::Percentage(100.0)),
        Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
    );
    let val = CssValue::Calc(Box::new(expr));
    assert_eq!(val.to_css_string(), "calc(100% - 10px)");
}

#[test]
fn to_css_string_calc_nested_grouping_parenthesized() {
    // (10px + 5px) * 2 — the compound left operand of `*` must keep
    // its parentheses or the re-parse would bind `5px * 2` first.
    let sum = CalcExpr::Add(
        Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
        Box::new(CalcExpr::Length(5.0, LengthUnit::Px)),
    );
    let expr = CalcExpr::Mul(Box::new(sum), Box::new(CalcExpr::Number(2.0)));
    let val = CssValue::Calc(Box::new(expr));
    assert_eq!(val.to_css_string(), "calc((10px + 5px) * 2)");
}

#[test]
fn to_css_string_url_quoted_and_escaped() {
    let val = CssValue::Url("data:image/png;base64,iVBO".into());
    assert_eq!(val.to_css_string(), "url(\"data:image/png;base64,iVBO\")");

    let quoted = CssValue::Url("a\"b".into());
    assert_eq!(quoted.to_css_string(), "url(\"a\\\"b\")");
}

#[test]
fn to_css_string_linear_gradient() {
    let val = CssValue::Gradient(Box::new(GradientValue::Linear {
        direction: AngleOrDirection::To(vec!["top".into(), "right".into()]),
        stops: vec![
            CssColorStop {
                color: CssValue::Color(CssColor::RED),
                position: None,
            },
            CssColorStop {
                color: CssValue::Color(CssColor::BLUE),
                position: Some(CssValue::Percentage(80.0)),
            },
        ],
        repeating: false,
    }));
    assert_eq!(
        val.to_css_string(),
        "linear-gradient(to top right, #ff0000, #0000ff 80%)"
    );
}

#[test]
fn to_css_string_radial_gradient_prelude() {
    let val = CssValue::Gradient(Box::new(GradientValue::Radial {
        shape: Some("circle".into()),
        size: None,
        position: Some(vec![CssValue::Percentage(50.0), CssValue::Percentage(50.0)]),
        stops: vec![
            CssColorStop {
                color: CssValue::Color(CssColor::RED),
                position: None,
            },
            CssColorStop {
                color: CssValue::Color(CssColor::BLUE),
                position: None,
            },
        ],
        repeating: true,
    }));
    assert_eq!(
        val.to_css_string(),
        "repeating-radial-gradient(circle at 50% 50%, #ff0000, #0000ff)"
    );
}

#[test]
fn length_unit_as_str_spot_checks() {
    // Exhaustiveness is compile-enforced by the no-fallback match in
    // `as_str`; these are representative output spot-checks only.
    assert_eq!(LengthUnit::Px.as_str(), "px");
    assert_eq!(LengthUnit::Vmin.as_str(), "vmin");
    assert_eq!(LengthUnit::Fr.as_str(), "fr");
}

#[test]
fn to_css_string_string_quoted_and_escaped() {
    // CSSOM "serialize a string": quoted, so CSS-significant chars
    // inside the value can't shred the declaration block on re-parse.
    let val = CssValue::String("a; b".into());
    assert_eq!(val.to_css_string(), "\"a; b\"");

    let quoted = CssValue::String("say \"hi\"".into());
    assert_eq!(quoted.to_css_string(), "\"say \\\"hi\\\"\"");

    let control = CssValue::String("a\nb".into());
    assert_eq!(control.to_css_string(), "\"a\\a b\"");
}

#[test]
fn to_css_string_url_control_chars_escaped() {
    let val = CssValue::Url("a\nb".into());
    assert_eq!(val.to_css_string(), "url(\"a\\a b\")");
}

#[test]
fn escape_css_string_spec_edges() {
    // CSSOM "serialize a string": U+0000 → U+FFFD (the only
    // non-escape substitution), U+007F (DEL) → hex escape, and the
    // hex escape always emits its trailing space so a following
    // hex digit can't be absorbed into the escape on re-parse.
    assert_eq!(escape_css_string("a\u{0}b"), "a\u{FFFD}b");
    assert_eq!(escape_css_string("a\u{7f}b"), "a\\7f b");
    assert_eq!(escape_css_string("\u{1}2"), "\\1 2");
}

#[test]
fn to_css_string_translate_calc_argument() {
    // A calc() transform argument must round-trip, not collapse to a
    // literal.
    let expr = CalcExpr::Sub(
        Box::new(CalcExpr::Percentage(100.0)),
        Box::new(CalcExpr::Length(10.0, LengthUnit::Px)),
    );
    let val = CssValue::TransformList(vec![TransformFunction::TranslateX(CssValue::Calc(
        Box::new(expr),
    ))]);
    assert_eq!(val.to_css_string(), "translateX(calc(100% - 10px))");
}

// ---------------------------------------------------------------------------
// CSSOM resolved-value color serialization (CSS Color 4 §16.2.2 + §16.1)
// ---------------------------------------------------------------------------

#[test]
fn resolved_value_opaque_uses_rgb() {
    // §16.2.2: opaque sRGB → rgb(), comma + single space, no alpha.
    assert_eq!(CssColor::RED.to_resolved_value_string(), "rgb(255, 0, 0)");
    assert_eq!(CssColor::BLACK.to_resolved_value_string(), "rgb(0, 0, 0)");
    assert_eq!(
        CssColor::rgb(29, 164, 192).to_resolved_value_string(),
        "rgb(29, 164, 192)"
    );
}

#[test]
fn resolved_value_translucent_uses_rgba() {
    // §16.2.2: alpha != 1 → rgba() with explicit alpha (§16.1).
    assert_eq!(
        CssColor::new(0, 0, 0, 128).to_resolved_value_string(),
        "rgba(0, 0, 0, 0.5)"
    );
    // Fully transparent: rgba(..., 0).
    assert_eq!(
        CssColor::new(0, 0, 0, 0).to_resolved_value_string(),
        "rgba(0, 0, 0, 0)"
    );
}

#[test]
fn resolved_value_distinct_from_display() {
    // The declared-value Display form (#rrggbb / rgba with {:.2}) must stay
    // separate from the resolved-value form — they are different spec
    // serialization contexts and must not be unified.
    assert_eq!(CssColor::RED.to_string(), "#ff0000");
    assert_eq!(CssColor::RED.to_resolved_value_string(), "rgb(255, 0, 0)");
    assert_eq!(
        CssColor::new(0, 0, 0, 128).to_string(),
        "rgba(0, 0, 0, 0.50)"
    );
    assert_eq!(
        CssColor::new(0, 0, 0, 128).to_resolved_value_string(),
        "rgba(0, 0, 0, 0.5)"
    );
}

#[test]
fn serialize_alpha_integer_percentage_preimage() {
    // §16.1 step 2: integer-percentage preimage → n/100.
    assert_eq!(serialize_alpha_u8(0), "0"); // n=0
    assert_eq!(serialize_alpha_u8(26), "0.1"); // n=10: round(25.5)=26 (ties up)
    assert_eq!(serialize_alpha_u8(128), "0.5"); // n=50: round(127.5)=128
    assert_eq!(serialize_alpha_u8(237), "0.93"); // n=93: round(237.15)=237
    assert_eq!(serialize_alpha_u8(255), "1"); // n=100 (only via to_resolved... rgb path)
}

#[test]
fn serialize_alpha_no_preimage_minimal_roundtrip() {
    // §16.1 step 3: 236 has no integer-% preimage (n=92→235, n=94→240).
    // elidex emits the minimal toward-+∞ form that round-trips: "0.926".
    assert_eq!(serialize_alpha_u8(236), "0.926");
    // Round-trip: round(0.926 * 255) == 236.
    assert_eq!((0.926_f64 * 255.0).round() as u8, 236);
}

#[test]
fn serialize_alpha_roundtrips_all_u8() {
    // Every 8-bit alpha must round-trip through its serialization
    // (§16.1: precision "must at least be sufficient to round-trip").
    for a in 0u16..=255 {
        let a = a as u8;
        let s = serialize_alpha_u8(a);
        let parsed: f64 = s.parse().unwrap();
        // Re-parse model: 8-bit alpha = round(v * 255).
        let back = (parsed * 255.0).round() as u8;
        assert_eq!(back, a, "alpha {a} serialized as {s:?} did not round-trip");
        // Leading zero kept, no trailing-zero fraction.
        assert!(
            !s.ends_with('0') || !s.contains('.'),
            "trailing zero in {s:?}"
        );
        if s.starts_with('.') {
            panic!("missing leading zero in {s:?}");
        }
    }
}

#[test]
fn format_decimal_ratio_trims_and_keeps_leading_zero() {
    assert_eq!(format_decimal_ratio(50, 100), "0.5");
    assert_eq!(format_decimal_ratio(926, 1000), "0.926");
    assert_eq!(format_decimal_ratio(0, 100), "0");
    assert_eq!(format_decimal_ratio(100, 100), "1");
    assert_eq!(format_decimal_ratio(5, 100), "0.05");
}
