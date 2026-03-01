//! CSS color parsing.
//!
//! Supports named colors (CSS Color Level 4, all 148), hex notation
//! (`#RGB`, `#RRGGBB`, `#RGBA`, `#RRGGBBAA`), `rgb()`/`rgba()`,
//! `hsl()`/`hsla()`, and the `transparent` keyword.

use cssparser::Parser;
use elidex_plugin::CssColor;

/// All 148 CSS Color Level 4 named colors, sorted for binary search.
static NAMED_COLORS: &[(&str, CssColor)] = &[
    ("aliceblue", CssColor::rgb(240, 248, 255)),
    ("antiquewhite", CssColor::rgb(250, 235, 215)),
    ("aqua", CssColor::rgb(0, 255, 255)),
    ("aquamarine", CssColor::rgb(127, 255, 212)),
    ("azure", CssColor::rgb(240, 255, 255)),
    ("beige", CssColor::rgb(245, 245, 220)),
    ("bisque", CssColor::rgb(255, 228, 196)),
    ("black", CssColor::rgb(0, 0, 0)),
    ("blanchedalmond", CssColor::rgb(255, 235, 205)),
    ("blue", CssColor::rgb(0, 0, 255)),
    ("blueviolet", CssColor::rgb(138, 43, 226)),
    ("brown", CssColor::rgb(165, 42, 42)),
    ("burlywood", CssColor::rgb(222, 184, 135)),
    ("cadetblue", CssColor::rgb(95, 158, 160)),
    ("chartreuse", CssColor::rgb(127, 255, 0)),
    ("chocolate", CssColor::rgb(210, 105, 30)),
    ("coral", CssColor::rgb(255, 127, 80)),
    ("cornflowerblue", CssColor::rgb(100, 149, 237)),
    ("cornsilk", CssColor::rgb(255, 248, 220)),
    ("crimson", CssColor::rgb(220, 20, 60)),
    ("cyan", CssColor::rgb(0, 255, 255)),
    ("darkblue", CssColor::rgb(0, 0, 139)),
    ("darkcyan", CssColor::rgb(0, 139, 139)),
    ("darkgoldenrod", CssColor::rgb(184, 134, 11)),
    ("darkgray", CssColor::rgb(169, 169, 169)),
    ("darkgreen", CssColor::rgb(0, 100, 0)),
    ("darkgrey", CssColor::rgb(169, 169, 169)),
    ("darkkhaki", CssColor::rgb(189, 183, 107)),
    ("darkmagenta", CssColor::rgb(139, 0, 139)),
    ("darkolivegreen", CssColor::rgb(85, 107, 47)),
    ("darkorange", CssColor::rgb(255, 140, 0)),
    ("darkorchid", CssColor::rgb(153, 50, 204)),
    ("darkred", CssColor::rgb(139, 0, 0)),
    ("darksalmon", CssColor::rgb(233, 150, 122)),
    ("darkseagreen", CssColor::rgb(143, 188, 143)),
    ("darkslateblue", CssColor::rgb(72, 61, 139)),
    ("darkslategray", CssColor::rgb(47, 79, 79)),
    ("darkslategrey", CssColor::rgb(47, 79, 79)),
    ("darkturquoise", CssColor::rgb(0, 206, 209)),
    ("darkviolet", CssColor::rgb(148, 0, 211)),
    ("deeppink", CssColor::rgb(255, 20, 147)),
    ("deepskyblue", CssColor::rgb(0, 191, 255)),
    ("dimgray", CssColor::rgb(105, 105, 105)),
    ("dimgrey", CssColor::rgb(105, 105, 105)),
    ("dodgerblue", CssColor::rgb(30, 144, 255)),
    ("firebrick", CssColor::rgb(178, 34, 34)),
    ("floralwhite", CssColor::rgb(255, 250, 240)),
    ("forestgreen", CssColor::rgb(34, 139, 34)),
    ("fuchsia", CssColor::rgb(255, 0, 255)),
    ("gainsboro", CssColor::rgb(220, 220, 220)),
    ("ghostwhite", CssColor::rgb(248, 248, 255)),
    ("gold", CssColor::rgb(255, 215, 0)),
    ("goldenrod", CssColor::rgb(218, 165, 32)),
    ("gray", CssColor::rgb(128, 128, 128)),
    ("green", CssColor::rgb(0, 128, 0)),
    ("greenyellow", CssColor::rgb(173, 255, 47)),
    ("grey", CssColor::rgb(128, 128, 128)),
    ("honeydew", CssColor::rgb(240, 255, 240)),
    ("hotpink", CssColor::rgb(255, 105, 180)),
    ("indianred", CssColor::rgb(205, 92, 92)),
    ("indigo", CssColor::rgb(75, 0, 130)),
    ("ivory", CssColor::rgb(255, 255, 240)),
    ("khaki", CssColor::rgb(240, 230, 140)),
    ("lavender", CssColor::rgb(230, 230, 250)),
    ("lavenderblush", CssColor::rgb(255, 240, 245)),
    ("lawngreen", CssColor::rgb(124, 252, 0)),
    ("lemonchiffon", CssColor::rgb(255, 250, 205)),
    ("lightblue", CssColor::rgb(173, 216, 230)),
    ("lightcoral", CssColor::rgb(240, 128, 128)),
    ("lightcyan", CssColor::rgb(224, 255, 255)),
    ("lightgoldenrodyellow", CssColor::rgb(250, 250, 210)),
    ("lightgray", CssColor::rgb(211, 211, 211)),
    ("lightgreen", CssColor::rgb(144, 238, 144)),
    ("lightgrey", CssColor::rgb(211, 211, 211)),
    ("lightpink", CssColor::rgb(255, 182, 193)),
    ("lightsalmon", CssColor::rgb(255, 160, 122)),
    ("lightseagreen", CssColor::rgb(32, 178, 170)),
    ("lightskyblue", CssColor::rgb(135, 206, 250)),
    ("lightslategray", CssColor::rgb(119, 136, 153)),
    ("lightslategrey", CssColor::rgb(119, 136, 153)),
    ("lightsteelblue", CssColor::rgb(176, 196, 222)),
    ("lightyellow", CssColor::rgb(255, 255, 224)),
    ("lime", CssColor::rgb(0, 255, 0)),
    ("limegreen", CssColor::rgb(50, 205, 50)),
    ("linen", CssColor::rgb(250, 240, 230)),
    ("magenta", CssColor::rgb(255, 0, 255)),
    ("maroon", CssColor::rgb(128, 0, 0)),
    ("mediumaquamarine", CssColor::rgb(102, 205, 170)),
    ("mediumblue", CssColor::rgb(0, 0, 205)),
    ("mediumorchid", CssColor::rgb(186, 85, 211)),
    ("mediumpurple", CssColor::rgb(147, 112, 219)),
    ("mediumseagreen", CssColor::rgb(60, 179, 113)),
    ("mediumslateblue", CssColor::rgb(123, 104, 238)),
    ("mediumspringgreen", CssColor::rgb(0, 250, 154)),
    ("mediumturquoise", CssColor::rgb(72, 209, 204)),
    ("mediumvioletred", CssColor::rgb(199, 21, 133)),
    ("midnightblue", CssColor::rgb(25, 25, 112)),
    ("mintcream", CssColor::rgb(245, 255, 250)),
    ("mistyrose", CssColor::rgb(255, 228, 225)),
    ("moccasin", CssColor::rgb(255, 228, 181)),
    ("navajowhite", CssColor::rgb(255, 222, 173)),
    ("navy", CssColor::rgb(0, 0, 128)),
    ("oldlace", CssColor::rgb(253, 245, 230)),
    ("olive", CssColor::rgb(128, 128, 0)),
    ("olivedrab", CssColor::rgb(107, 142, 35)),
    ("orange", CssColor::rgb(255, 165, 0)),
    ("orangered", CssColor::rgb(255, 69, 0)),
    ("orchid", CssColor::rgb(218, 112, 214)),
    ("palegoldenrod", CssColor::rgb(238, 232, 170)),
    ("palegreen", CssColor::rgb(152, 251, 152)),
    ("paleturquoise", CssColor::rgb(175, 238, 238)),
    ("palevioletred", CssColor::rgb(219, 112, 147)),
    ("papayawhip", CssColor::rgb(255, 239, 213)),
    ("peachpuff", CssColor::rgb(255, 218, 185)),
    ("peru", CssColor::rgb(205, 133, 63)),
    ("pink", CssColor::rgb(255, 192, 203)),
    ("plum", CssColor::rgb(221, 160, 221)),
    ("powderblue", CssColor::rgb(176, 224, 230)),
    ("purple", CssColor::rgb(128, 0, 128)),
    ("rebeccapurple", CssColor::rgb(102, 51, 153)),
    ("red", CssColor::rgb(255, 0, 0)),
    ("rosybrown", CssColor::rgb(188, 143, 143)),
    ("royalblue", CssColor::rgb(65, 105, 225)),
    ("saddlebrown", CssColor::rgb(139, 69, 19)),
    ("salmon", CssColor::rgb(250, 128, 114)),
    ("sandybrown", CssColor::rgb(244, 164, 96)),
    ("seagreen", CssColor::rgb(46, 139, 87)),
    ("seashell", CssColor::rgb(255, 245, 238)),
    ("sienna", CssColor::rgb(160, 82, 45)),
    ("silver", CssColor::rgb(192, 192, 192)),
    ("skyblue", CssColor::rgb(135, 206, 235)),
    ("slateblue", CssColor::rgb(106, 90, 205)),
    ("slategray", CssColor::rgb(112, 128, 144)),
    ("slategrey", CssColor::rgb(112, 128, 144)),
    ("snow", CssColor::rgb(255, 250, 250)),
    ("springgreen", CssColor::rgb(0, 255, 127)),
    ("steelblue", CssColor::rgb(70, 130, 180)),
    ("tan", CssColor::rgb(210, 180, 140)),
    ("teal", CssColor::rgb(0, 128, 128)),
    ("thistle", CssColor::rgb(216, 191, 216)),
    ("tomato", CssColor::rgb(255, 99, 71)),
    ("turquoise", CssColor::rgb(64, 224, 208)),
    ("violet", CssColor::rgb(238, 130, 238)),
    ("wheat", CssColor::rgb(245, 222, 179)),
    ("white", CssColor::rgb(255, 255, 255)),
    ("whitesmoke", CssColor::rgb(245, 245, 245)),
    ("yellow", CssColor::rgb(255, 255, 0)),
    ("yellowgreen", CssColor::rgb(154, 205, 50)),
];

/// Look up a CSS named color (case-insensitive).
#[cfg(test)]
pub fn named_color(name: &str) -> Option<CssColor> {
    let lower = name.to_ascii_lowercase();
    named_color_lower(&lower)
}

/// Look up a CSS named color from an already-lowercased name.
fn named_color_lower(lower: &str) -> Option<CssColor> {
    NAMED_COLORS
        .binary_search_by_key(&lower, |(n, _)| n)
        .ok()
        .map(|i| NAMED_COLORS[i].1)
}

/// Parse a CSS color value from a cssparser token stream.
///
/// Supports: named colors, hex (`#RGB`, `#RRGGBB`, `#RGBA`, `#RRGGBBAA`),
/// `rgb()`, `rgba()`, and `transparent`.
#[allow(clippy::result_unit_err)]
pub fn parse_color(input: &mut Parser) -> Result<CssColor, ()> {
    let token = input.next().map_err(|_| ())?;
    match token {
        cssparser::Token::Ident(ref name) => {
            let lower = name.to_ascii_lowercase();
            if lower == "transparent" {
                return Ok(CssColor::TRANSPARENT);
            }
            named_color_lower(&lower).ok_or(())
        }
        cssparser::Token::IDHash(ref hash) | cssparser::Token::Hash(ref hash) => {
            parse_hex_color(hash.as_ref())
        }
        cssparser::Token::Function(ref name) => {
            let lower = name.to_ascii_lowercase();
            match lower.as_str() {
                "rgb" | "rgba" => input
                    .parse_nested_block(|i| {
                        parse_rgb_function(i).map_err(|()| i.new_custom_error(()))
                    })
                    .map_err(|_: cssparser::ParseError<'_, ()>| ()),
                "hsl" | "hsla" => input
                    .parse_nested_block(|i| {
                        parse_hsl_function(i).map_err(|()| i.new_custom_error(()))
                    })
                    .map_err(|_: cssparser::ParseError<'_, ()>| ()),
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

/// Parse a hex color string (without the `#` prefix, which cssparser strips).
fn parse_hex_color(hex: &str) -> Result<CssColor, ()> {
    let chars = hex.as_bytes();
    match chars.len() {
        3 => {
            let r = hex_digit(chars[0])? * 17;
            let g = hex_digit(chars[1])? * 17;
            let b = hex_digit(chars[2])? * 17;
            Ok(CssColor::rgb(r, g, b))
        }
        4 => {
            let r = hex_digit(chars[0])? * 17;
            let g = hex_digit(chars[1])? * 17;
            let b = hex_digit(chars[2])? * 17;
            let a = hex_digit(chars[3])? * 17;
            Ok(CssColor::new(r, g, b, a))
        }
        6 => {
            let r = hex_byte(chars[0], chars[1])?;
            let g = hex_byte(chars[2], chars[3])?;
            let b = hex_byte(chars[4], chars[5])?;
            Ok(CssColor::rgb(r, g, b))
        }
        8 => {
            let r = hex_byte(chars[0], chars[1])?;
            let g = hex_byte(chars[2], chars[3])?;
            let b = hex_byte(chars[4], chars[5])?;
            let a = hex_byte(chars[6], chars[7])?;
            Ok(CssColor::new(r, g, b, a))
        }
        _ => Err(()),
    }
}

fn hex_digit(c: u8) -> Result<u8, ()> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(()),
    }
}

fn hex_byte(hi: u8, lo: u8) -> Result<u8, ()> {
    Ok(hex_digit(hi)? * 16 + hex_digit(lo)?)
}

/// Parse the contents of `rgb(r, g, b)` or `rgba(r, g, b, a)`.
fn parse_rgb_function(input: &mut Parser) -> Result<CssColor, ()> {
    let r = parse_color_component(input)?;
    input.expect_comma().map_err(|_| ())?;
    let g = parse_color_component(input)?;
    input.expect_comma().map_err(|_| ())?;
    let b = parse_color_component(input)?;
    let a = if input.try_parse(Parser::expect_comma).is_ok() {
        parse_alpha_component(input)?
    } else {
        255
    };
    Ok(CssColor::new(r, g, b, a))
}

fn parse_component(input: &mut Parser, scale_number: f32) -> Result<u8, ()> {
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Number { value, .. } => Ok(clamp_u8(value * scale_number)),
        cssparser::Token::Percentage { unit_value, .. } => Ok(clamp_u8(unit_value * 255.0)),
        _ => Err(()),
    }
}

/// Parse a single color component (0–255 integer or percentage).
fn parse_color_component(input: &mut Parser) -> Result<u8, ()> {
    parse_component(input, 1.0)
}

/// Parse an alpha component.
///
/// Per CSS Color Level 4, alpha is a `<number>` in `[0, 1]` or a `<percentage>`.
/// Out-of-range values are clamped.
fn parse_alpha_component(input: &mut Parser) -> Result<u8, ()> {
    parse_component(input, 255.0)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn clamp_u8(v: f32) -> u8 {
    // Clamping to [0, 255] guarantees the cast is safe.
    v.round().clamp(0.0, 255.0) as u8
}

/// Convert HSL to RGB (CSS Color Level 4 algorithm).
///
/// - `h`: hue in degrees (0–360, wraps around)
/// - `s`: saturation (0.0–1.0)
/// - `l`: lightness (0.0–1.0)
#[allow(clippy::many_single_char_names)]
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    // Guard against infinity/NaN — treat as 0 (CSS Color Level 4 §12).
    let h = if h.is_finite() { h } else { 0.0 };
    // Normalize hue to [0, 360).
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());

    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let m = l - c / 2.0;
    (
        clamp_u8((r1 + m) * 255.0),
        clamp_u8((g1 + m) * 255.0),
        clamp_u8((b1 + m) * 255.0),
    )
}

/// Parse a hue value: `<number>` or `<angle>` (deg/grad/rad/turn).
fn parse_hue(input: &mut Parser) -> Result<f32, ()> {
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Number { value, .. } => Ok(value),
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let lower = unit.to_ascii_lowercase();
            match lower.as_str() {
                "deg" => Ok(value),
                "grad" => Ok(value * 360.0 / 400.0),
                "rad" => Ok(value * 180.0 / std::f32::consts::PI),
                "turn" => Ok(value * 360.0),
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

/// Parse a percentage value, clamp to 0.0–1.0, and return.
///
/// CSS Color Level 4 §4.2.4: saturation and lightness are clamped to [0%, 100%].
fn parse_percentage_unit_value(input: &mut Parser) -> Result<f32, ()> {
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Percentage { unit_value, .. } => Ok(unit_value.clamp(0.0, 1.0)),
        _ => Err(()),
    }
}

/// Parse the contents of `hsl(h, s%, l%)` or `hsla(h, s%, l%, a)`.
///
/// Supports both comma-separated and space-separated syntax (CSS Color Level 4).
#[allow(clippy::many_single_char_names)]
fn parse_hsl_function(input: &mut Parser) -> Result<CssColor, ()> {
    let h = parse_hue(input)?;

    // Detect separator: comma or space.
    let is_comma = input.try_parse(Parser::expect_comma).is_ok();

    let s = parse_percentage_unit_value(input)?;
    if is_comma {
        input.expect_comma().map_err(|_| ())?;
    }
    let l = parse_percentage_unit_value(input)?;

    let (r, g, b) = hsl_to_rgb(h, s, l);

    // Optional alpha.
    let a = if is_comma {
        if input.try_parse(Parser::expect_comma).is_ok() {
            parse_alpha_component(input)?
        } else {
            255
        }
    } else if input
        .try_parse(|i| i.expect_delim('/').map_err(|_| ()))
        .is_ok()
    {
        parse_alpha_component(input)?
    } else {
        255
    };

    Ok(CssColor::new(r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    fn parse(css: &str) -> Result<CssColor, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_color(&mut parser)
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
}
