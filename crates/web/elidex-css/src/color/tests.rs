use super::*;
use cssparser::ParserInput;

fn parse(css: &str) -> Result<CssColor, ()> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_color(&mut parser)
}

/// Look up a CSS named color (case-insensitive).
pub fn named_color(name: &str) -> Option<CssColor> {
    let lower = name.to_ascii_lowercase();
    named_color_lower(&lower)
}

#[test]
fn named_color_red() {
    assert_eq!(parse("red"), Ok(CssColor::RED));
}

#[test]
fn named_color_case_insensitive() {
    assert_eq!(parse("ReD"), Ok(CssColor::RED));
}

#[test]
fn named_color_unknown() {
    assert!(parse("notacolor").is_err());
}

#[test]
fn hex_3_digit() {
    assert_eq!(parse("#f00"), Ok(CssColor::RED));
}

#[test]
fn hex_6_digit() {
    assert_eq!(parse("#ff0000"), Ok(CssColor::RED));
}

#[test]
fn hex_4_digit_alpha() {
    let c = parse("#f00a").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 170); // 0xa * 17 = 170
}

#[test]
fn hex_8_digit_alpha() {
    let c = parse("#ff0000aa").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 170);
}

#[test]
fn hex_invalid_length() {
    assert!(parse("#f0").is_err());
}

#[test]
fn rgb_function() {
    assert_eq!(parse("rgb(255, 0, 0)"), Ok(CssColor::RED));
}

#[test]
fn rgba_function() {
    let c = parse("rgba(255, 0, 0, 0.5)").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

#[test]
fn rgba_alpha_clamped() {
    // Out-of-range alpha values are clamped to [0, 1] per CSS spec.
    let c = parse("rgba(255, 0, 0, 2.0)").unwrap();
    assert_eq!(c.a, 255); // clamped to 1.0
    let c2 = parse("rgba(255, 0, 0, -0.5)").unwrap();
    assert_eq!(c2.a, 0); // clamped to 0.0
}

#[test]
fn rgb_space_syntax() {
    let c = parse("rgb(255 0 0)").unwrap();
    assert_eq!(c, CssColor::RED);
}

#[test]
fn rgb_space_with_slash_alpha() {
    let c = parse("rgb(255 0 0 / 0.5)").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

#[test]
fn rgb_space_with_percentage_alpha() {
    let c = parse("rgb(255 0 0 / 50%)").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.a, 128);
}

#[test]
fn transparent_keyword() {
    assert_eq!(parse("transparent"), Ok(CssColor::TRANSPARENT));
}

#[test]
fn named_color_spot_check() {
    assert_eq!(named_color("aqua"), Some(CssColor::rgb(0, 255, 255)));
    assert_eq!(named_color("fuchsia"), Some(CssColor::rgb(255, 0, 255)));
    assert_eq!(named_color("lime"), Some(CssColor::rgb(0, 255, 0)));
    assert_eq!(named_color("navy"), Some(CssColor::rgb(0, 0, 128)));
    assert_eq!(
        named_color("rebeccapurple"),
        Some(CssColor::rgb(102, 51, 153))
    );
}

#[test]
fn named_color_table_is_sorted() {
    for w in NAMED_COLORS.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "NAMED_COLORS not sorted: {:?} >= {:?}",
            w[0].0,
            w[1].0
        );
    }
}

#[test]
fn named_color_table_has_148_entries() {
    assert_eq!(NAMED_COLORS.len(), 148);
}

// --- hsl()/hsla() tests (M3-6) ---

#[test]
fn hsl_green() {
    let c = parse("hsl(120, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 255);
}

#[test]
fn hsl_red() {
    let c = parse("hsl(0, 100%, 50%)").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
}

#[test]
fn hsl_blue() {
    let c = parse("hsl(240, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 255);
}

#[test]
fn hsla_semi_transparent_red() {
    let c = parse("hsla(0, 100%, 50%, 0.5)").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

#[test]
fn hsl_360_equals_0() {
    let c0 = parse("hsl(0, 100%, 50%)").unwrap();
    let c360 = parse("hsl(360, 100%, 50%)").unwrap();
    assert_eq!(c0, c360);
}

#[test]
fn hsl_to_rgb_grey_when_s_zero() {
    // saturation=0 → grey at lightness level
    let c = parse("hsl(0, 0%, 50%)").unwrap();
    assert_eq!(c.r, 128);
    assert_eq!(c.g, 128);
    assert_eq!(c.b, 128);
}

#[test]
fn hsl_to_rgb_black_when_l_zero() {
    let c = parse("hsl(120, 100%, 0%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
}

#[test]
fn hsl_to_rgb_white_when_l_one() {
    let c = parse("hsl(120, 100%, 100%)").unwrap();
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 255);
}

#[test]
fn hsl_negative_hue_wraparound() {
    // -120 deg should wrap to 240 deg (blue)
    let c = parse("hsl(-120, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 255);
}

#[test]
fn hsla_is_hsl_alias() {
    let hsl = parse("hsl(120, 100%, 50%)").unwrap();
    let hsla = parse("hsla(120, 100%, 50%)").unwrap();
    assert_eq!(hsl, hsla);
}

// --- Angle unit tests ---

#[test]
fn hsl_hue_deg_unit() {
    // 120deg = green
    let c = parse("hsl(120deg, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
}

#[test]
fn hsl_hue_grad_unit() {
    // 200grad = 180deg = cyan
    let c = parse("hsl(200grad, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 255);
}

#[test]
fn hsl_hue_rad_unit() {
    // π rad ≈ 180deg = cyan
    let c = parse("hsl(3.14159265rad, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert!(c.g >= 254); // slight floating-point variance
    assert!(c.b >= 254);
}

#[test]
fn hsl_hue_turn_unit() {
    // 0.5turn = 180deg = cyan
    let c = parse("hsl(0.5turn, 100%, 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 255);
}

#[test]
fn hsl_infinity_hue_treated_as_zero() {
    // Infinity hue should be treated as 0 (red)
    let normal = parse("hsl(0, 100%, 50%)").unwrap();
    let (r, g, b) = super::hsl_to_rgb(f32::INFINITY, 1.0, 0.5);
    assert_eq!(r, normal.r);
    assert_eq!(g, normal.g);
    assert_eq!(b, normal.b);
}

#[test]
fn hsl_nan_hue_treated_as_zero() {
    let (r, g, b) = super::hsl_to_rgb(f32::NAN, 1.0, 0.5);
    assert_eq!(r, 255); // hue=0 → red
    assert_eq!(g, 0);
    assert_eq!(b, 0);
}

#[test]
fn hsla_space_syntax() {
    // hsl(120 100% 50%) space-separated syntax
    let c = parse("hsl(120 100% 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 255);
}

#[test]
fn hsla_space_with_slash_alpha() {
    let c = parse("hsl(120 100% 50% / 0.5)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

#[test]
fn hsla_space_with_percentage_alpha() {
    let c = parse("hsl(120 100% 50% / 50%)").unwrap();
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

// --- HSL rejection tests ---

#[test]
fn hsl_missing_components_rejected() {
    assert!(parse("hsl(120)").is_err());
}

#[test]
fn hsl_bare_numbers_for_sl_rejected() {
    // s and l must be percentages, not bare numbers.
    assert!(parse("hsl(120, 50, 50)").is_err());
}

#[test]
fn hsl_mixed_separators_rejected() {
    // Cannot mix comma and space syntax.
    assert!(parse("hsl(120 100%, 50%)").is_err());
}

#[test]
fn hsl_out_of_range_saturation_clamped() {
    // 200% saturation is clamped to 100%.
    let c = parse("hsl(120, 200%, 50%)").unwrap();
    assert_eq!(c, CssColor::new(0, 255, 0, 255));
}

#[test]
fn hsl_out_of_range_lightness_clamped() {
    // -50% lightness is clamped to 0% (black).
    let c = parse("hsl(0, 100%, -50%)").unwrap();
    assert_eq!(c, CssColor::new(0, 0, 0, 255));
}

#[test]
fn hsl_hue_above_360_wraps() {
    // 480 == 120 (green).
    let c = parse("hsl(480, 100%, 50%)").unwrap();
    assert_eq!(c, CssColor::new(0, 255, 0, 255));
}
