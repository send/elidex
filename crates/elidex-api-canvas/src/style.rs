//! CSS color string parsing for Canvas 2D `fillStyle`/`strokeStyle`.

use cssparser::{Parser, ParserInput};
use elidex_plugin::CssColor;

/// Parse a CSS color string (e.g., `"red"`, `"#ff0000"`, `"rgb(255,0,0)"`).
///
/// Returns `None` if the string is not a valid CSS color.
pub(crate) fn parse_color_string(input: &str) -> Option<CssColor> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut pi = ParserInput::new(trimmed);
    let mut parser = Parser::new(&mut pi);
    let color = elidex_css::parse_color(&mut parser).ok()?;
    if parser.is_exhausted() {
        Some(color)
    } else {
        None
    }
}

/// Serialize a `CssColor` for Canvas 2D `fillStyle`/`strokeStyle` getter.
///
/// Per WHATWG HTML §4.12.5.1.1:
/// - Opaque colors → lowercase `#rrggbb`
/// - Non-opaque colors → `rgba(r, g, b, alpha)` where alpha is 0–1
pub fn serialize_canvas_color(color: CssColor) -> String {
    if color.a == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    } else {
        let alpha = f64::from(color.a) / 255.0;
        // Use enough precision (3 decimal places) to distinguish all u8 alpha values.
        let formatted = format!("{alpha:.3}");
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        format!("rgba({}, {}, {}, {trimmed})", color.r, color.g, color.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_string_variants() {
        let cases: &[(&str, Option<CssColor>)] = &[
            // Named colors.
            ("red", Some(CssColor::rgb(255, 0, 0))),
            ("blue", Some(CssColor::rgb(0, 0, 255))),
            ("transparent", Some(CssColor::TRANSPARENT)),
            // Hex colors.
            ("#ff8000", Some(CssColor::rgb(255, 128, 0))),
            ("#f80", Some(CssColor::rgb(255, 136, 0))),
            // Functional notation.
            ("rgb(10, 20, 30)", Some(CssColor::rgb(10, 20, 30))),
            // Invalid inputs.
            ("notacolor", None),
            ("", None),
        ];

        for (input, expected) in cases {
            assert_eq!(parse_color_string(input), *expected, "input: {input:?}");
        }
    }

    #[test]
    fn serialize_canvas_color_variants() {
        let cases: &[(CssColor, &str)] = &[
            // Opaque colors → #rrggbb.
            (CssColor::RED, "#ff0000"),
            (CssColor::rgb(0, 128, 255), "#0080ff"),
            (CssColor::BLACK, "#000000"),
            (CssColor::WHITE, "#ffffff"),
            // Fully transparent → rgba(..., 0).
            (CssColor::TRANSPARENT, "rgba(0, 0, 0, 0)"),
        ];

        for (color, expected) in cases {
            assert_eq!(
                serialize_canvas_color(*color),
                *expected,
                "color: {color:?}"
            );
        }
    }

    #[test]
    fn serialize_non_opaque_color() {
        let semi = CssColor::new(255, 0, 0, 128);
        let s = serialize_canvas_color(semi);
        assert!(s.starts_with("rgba(255, 0, 0, "));
        assert!(!s.contains("255)")); // alpha is not 255
    }
}
