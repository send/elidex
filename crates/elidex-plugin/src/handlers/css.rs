//! Built-in [`CssPropertyHandler`] implementations for representative properties.

use crate::{
    CssColor, CssPropertyHandler, CssValue, LengthUnit, ParseError, PluginRegistry, StyleContext,
};

/// Parse a non-negative, finite f32 from a string.
///
/// Rejects NaN, Infinity, and negative values — these are invalid in CSS
/// length/percentage contexts (CSS Values Level 4 §4.1).
fn parse_non_negative_f32(s: &str) -> Option<f32> {
    let n: f32 = s.parse().ok()?;
    if n.is_finite() && n >= 0.0 {
        Some(n)
    } else {
        None
    }
}

/// Parse a finite f32 from a string (negative values allowed).
fn parse_finite_f32(s: &str) -> Option<f32> {
    let n: f32 = s.parse().ok()?;
    n.is_finite().then_some(n)
}

fn css_parse_error(property: &str, input: &str, message: &str) -> ParseError {
    ParseError {
        property: property.into(),
        input: input.into(),
        message: message.into(),
    }
}

/// Parse a CSS keyword property value, returning `CssValue::Keyword` with the
/// canonical (lowercase) form.
///
/// CSS Values Level 4 §2: all CSS keywords are ASCII case-insensitive.
fn parse_css_keyword(
    property: &str,
    value: &str,
    allowed: &[&str],
    error_msg: &str,
) -> Result<CssValue, ParseError> {
    let lower = value.trim().to_ascii_lowercase();
    if allowed.contains(&lower.as_str()) {
        Ok(CssValue::Keyword(lower))
    } else {
        Err(css_parse_error(property, value, error_msg))
    }
}

// ---------------------------------------------------------------------------
// DisplayHandler
// ---------------------------------------------------------------------------

struct DisplayHandler;

impl CssPropertyHandler for DisplayHandler {
    fn property_name(&self) -> &'static str {
        "display"
    }

    fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
        parse_css_keyword(
            "display",
            value,
            &[
                "block",
                "inline",
                "flex",
                "grid",
                "none",
                "inline-block",
                "inline-flex",
                "inline-grid",
                "contents",
                "table",
            ],
            "unsupported display value",
        )
    }

    fn resolve(&self, value: &CssValue, _ctx: &StyleContext) -> CssValue {
        value.clone()
    }

    fn affects_layout(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// ColorHandler
// ---------------------------------------------------------------------------

struct ColorHandler;

impl CssPropertyHandler for ColorHandler {
    fn property_name(&self) -> &'static str {
        "color"
    }

    fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
        let trimmed = value.trim();
        // CSS Color Level 4 §6.1: color keywords are case-insensitive.
        let lower = trimmed.to_ascii_lowercase();
        match lower.as_str() {
            "red" => return Ok(CssValue::Color(CssColor::RED)),
            "green" => return Ok(CssValue::Color(CssColor::GREEN)),
            "blue" => return Ok(CssValue::Color(CssColor::BLUE)),
            "black" => return Ok(CssValue::Color(CssColor::BLACK)),
            "white" => return Ok(CssValue::Color(CssColor::WHITE)),
            "transparent" => return Ok(CssValue::Color(CssColor::TRANSPARENT)),
            _ => {}
        }
        if let Some(hex) = trimmed.strip_prefix('#') {
            if let Some(color) = parse_hex_color(hex) {
                return Ok(CssValue::Color(color));
            }
        }
        Err(css_parse_error("color", value, "unsupported color value"))
    }

    fn resolve(&self, value: &CssValue, _ctx: &StyleContext) -> CssValue {
        value.clone()
    }
}

/// Parse a hex color string (without the leading `#`).
///
/// Accepts 3-digit (`rgb`), 4-digit (`rgba`), 6-digit (`rrggbb`), and
/// 8-digit (`rrggbbaa`) forms per CSS Color Level 4 §4.2.
fn parse_hex_color(hex: &str) -> Option<CssColor> {
    // Validate that all characters are hex digits.
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        // #rgb → #rrggbb
        3 => {
            let r = expand_short_hex(hex.as_bytes()[0]);
            let g = expand_short_hex(hex.as_bytes()[1]);
            let b = expand_short_hex(hex.as_bytes()[2]);
            Some(CssColor::rgb(r, g, b))
        }
        // #rgba → #rrggbbaa
        4 => {
            let r = expand_short_hex(hex.as_bytes()[0]);
            let g = expand_short_hex(hex.as_bytes()[1]);
            let b = expand_short_hex(hex.as_bytes()[2]);
            let a = expand_short_hex(hex.as_bytes()[3]);
            Some(CssColor::new(r, g, b, a))
        }
        // #rrggbb
        6 => {
            let r = parse_hex_pair(&hex[0..2])?;
            let g = parse_hex_pair(&hex[2..4])?;
            let b = parse_hex_pair(&hex[4..6])?;
            Some(CssColor::rgb(r, g, b))
        }
        // #rrggbbaa
        8 => {
            let r = parse_hex_pair(&hex[0..2])?;
            let g = parse_hex_pair(&hex[2..4])?;
            let b = parse_hex_pair(&hex[4..6])?;
            let a = parse_hex_pair(&hex[6..8])?;
            Some(CssColor::new(r, g, b, a))
        }
        _ => None,
    }
}

/// Expand a single hex digit to a doubled byte (e.g. `0xA` → `0xAA`).
fn expand_short_hex(byte: u8) -> u8 {
    let nibble = match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0,
    };
    nibble << 4 | nibble
}

fn parse_hex_pair(s: &str) -> Option<u8> {
    u8::from_str_radix(s, 16).ok()
}

// ---------------------------------------------------------------------------
// WidthHandler
// ---------------------------------------------------------------------------

struct WidthHandler;

impl CssPropertyHandler for WidthHandler {
    fn property_name(&self) -> &'static str {
        "width"
    }

    fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
        let trimmed = value.trim();
        // CSS Values Level 4 §2: keywords are ASCII case-insensitive.
        if trimmed.eq_ignore_ascii_case("auto") {
            return Ok(CssValue::Auto);
        }
        if let Some(px) = trimmed.strip_suffix("px") {
            if let Some(n) = parse_non_negative_f32(px) {
                return Ok(CssValue::Length(n, LengthUnit::Px));
            }
        }
        if let Some(pct) = trimmed.strip_suffix('%') {
            if let Some(n) = parse_non_negative_f32(pct) {
                return Ok(CssValue::Percentage(n));
            }
        }
        if let Some(em) = trimmed.strip_suffix("em") {
            if let Some(n) = parse_non_negative_f32(em) {
                return Ok(CssValue::Length(n, LengthUnit::Em));
            }
        }
        Err(css_parse_error(
            "width",
            value,
            "expected auto, length, or percentage",
        ))
    }

    fn resolve(&self, value: &CssValue, ctx: &StyleContext) -> CssValue {
        match value {
            CssValue::Length(n, LengthUnit::Em) => {
                CssValue::Length(n * ctx.parent_font_size_px, LengthUnit::Px)
            }
            // CSS Box Sizing Level 3: width % resolves against containing block width.
            // StyleContext lacks containing_block_width; viewport_width used as fallback.
            CssValue::Percentage(pct) => {
                CssValue::Length(pct / 100.0 * ctx.viewport_width_px, LengthUnit::Px)
            }
            other => other.clone(),
        }
    }

    fn affects_layout(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// OpacityHandler
// ---------------------------------------------------------------------------

struct OpacityHandler;

impl CssPropertyHandler for OpacityHandler {
    fn property_name(&self) -> &'static str {
        "opacity"
    }

    fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
        parse_finite_f32(value.trim())
            .map(CssValue::Number)
            .ok_or_else(|| css_parse_error("opacity", value, "expected a finite number"))
    }

    fn resolve(&self, value: &CssValue, _ctx: &StyleContext) -> CssValue {
        match value {
            CssValue::Number(n) => CssValue::Number(n.clamp(0.0, 1.0)),
            other => other.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// OverflowHandler
// ---------------------------------------------------------------------------

struct OverflowHandler;

impl CssPropertyHandler for OverflowHandler {
    fn property_name(&self) -> &'static str {
        "overflow"
    }

    fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
        parse_css_keyword(
            "overflow",
            value,
            &["visible", "hidden", "scroll", "auto"],
            "expected visible, hidden, scroll, or auto",
        )
    }

    fn resolve(&self, value: &CssValue, _ctx: &StyleContext) -> CssValue {
        value.clone()
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Creates a [`PluginRegistry`] pre-populated with built-in CSS property handlers.
///
/// Registers handlers for: `display`, `color`, `width`, `opacity`, `overflow`.
#[must_use]
pub fn create_css_property_registry() -> PluginRegistry<dyn CssPropertyHandler> {
    let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
    registry.register_static("display", Box::new(DisplayHandler));
    registry.register_static("color", Box::new(ColorHandler));
    registry.register_static("width", Box::new(WidthHandler));
    registry.register_static("opacity", Box::new(OpacityHandler));
    registry.register_static("overflow", Box::new(OverflowHandler));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CssSpecLevel;

    #[test]
    fn display_handler_parse_and_resolve() {
        let h = DisplayHandler;
        assert_eq!(h.property_name(), "display");
        assert!(h.affects_layout());
        assert_eq!(h.spec_level(), CssSpecLevel::Standard);
        let v = h.parse("flex").unwrap();
        assert_eq!(v, CssValue::Keyword("flex".into()));
        assert_eq!(h.resolve(&v, &StyleContext::default()), v);
        assert!(h.parse("invalid").is_err());
        // CSS Values Level 4 §2: keywords are ASCII case-insensitive.
        assert_eq!(h.parse("BLOCK").unwrap(), CssValue::Keyword("block".into()));
        assert_eq!(h.parse("Flex").unwrap(), CssValue::Keyword("flex".into()));
    }

    #[test]
    fn color_handler_parse_named_and_hex() {
        let h = ColorHandler;
        assert_eq!(h.property_name(), "color");
        assert!(!h.affects_layout());
        // Named colors (case-insensitive per CSS Color Level 4 §6.1).
        assert_eq!(h.parse("red").unwrap(), CssValue::Color(CssColor::RED));
        assert_eq!(h.parse("RED").unwrap(), CssValue::Color(CssColor::RED));
        assert_eq!(h.parse("Red").unwrap(), CssValue::Color(CssColor::RED));
        // 6-digit hex
        assert_eq!(
            h.parse("#00ff00").unwrap(),
            CssValue::Color(CssColor::rgb(0, 255, 0))
        );
        // 3-digit hex shorthand (#rgb → #rrggbb)
        assert_eq!(
            h.parse("#0f0").unwrap(),
            CssValue::Color(CssColor::rgb(0, 255, 0))
        );
        // 4-digit hex shorthand (#rgba → #rrggbbaa)
        assert_eq!(
            h.parse("#f008").unwrap(),
            CssValue::Color(CssColor::new(255, 0, 0, 136))
        );
        // 8-digit hex
        assert_eq!(
            h.parse("#ff000080").unwrap(),
            CssValue::Color(CssColor::new(255, 0, 0, 128))
        );
        // Invalid lengths rejected
        assert!(h.parse("#12345").is_err());
        assert!(h.parse("#1234567").is_err());
        assert!(h.parse("not-a-color").is_err());
    }

    #[test]
    fn width_handler_parse_and_resolve() {
        let h = WidthHandler;
        assert_eq!(h.property_name(), "width");
        assert!(h.affects_layout());
        assert_eq!(h.parse("auto").unwrap(), CssValue::Auto);
        assert_eq!(h.parse("AUTO").unwrap(), CssValue::Auto);
        assert_eq!(h.parse("Auto").unwrap(), CssValue::Auto);
        assert_eq!(
            h.parse("100px").unwrap(),
            CssValue::Length(100.0, LengthUnit::Px)
        );
        assert_eq!(h.parse("50%").unwrap(), CssValue::Percentage(50.0));

        // em resolution
        let ctx = StyleContext {
            parent_font_size_px: 20.0,
            ..StyleContext::default()
        };
        let em_val = h.parse("2em").unwrap();
        assert_eq!(
            h.resolve(&em_val, &ctx),
            CssValue::Length(40.0, LengthUnit::Px)
        );

        // percentage resolution
        let pct_val = h.parse("50%").unwrap();
        let resolved = h.resolve(&pct_val, &StyleContext::default());
        assert_eq!(resolved, CssValue::Length(640.0, LengthUnit::Px));

        // Rejects NaN, Infinity, negative (CSS width is non-negative).
        assert!(h.parse("NaNpx").is_err());
        assert!(h.parse("infpx").is_err());
        assert!(h.parse("-100px").is_err());
        assert!(h.parse("-50%").is_err());
        // Zero is valid.
        assert_eq!(
            h.parse("0px").unwrap(),
            CssValue::Length(0.0, LengthUnit::Px)
        );
    }

    #[test]
    fn opacity_handler_parse_and_clamp() {
        let h = OpacityHandler;
        assert_eq!(h.property_name(), "opacity");
        assert_eq!(h.parse("0.5").unwrap(), CssValue::Number(0.5));
        assert!(h.parse("abc").is_err());
        // Rejects NaN/Infinity.
        assert!(h.parse("NaN").is_err());
        assert!(h.parse("inf").is_err());

        let ctx = StyleContext::default();
        assert_eq!(
            h.resolve(&CssValue::Number(1.5), &ctx),
            CssValue::Number(1.0)
        );
        assert_eq!(
            h.resolve(&CssValue::Number(-0.5), &ctx),
            CssValue::Number(0.0)
        );
    }

    #[test]
    fn overflow_handler_parse() {
        let h = OverflowHandler;
        assert_eq!(h.property_name(), "overflow");
        assert_eq!(
            h.parse("hidden").unwrap(),
            CssValue::Keyword("hidden".into())
        );
        assert!(h.parse("invalid").is_err());
        // CSS Values Level 4 §2: keywords are ASCII case-insensitive.
        assert_eq!(
            h.parse("HIDDEN").unwrap(),
            CssValue::Keyword("hidden".into())
        );
        assert_eq!(
            h.parse("Scroll").unwrap(),
            CssValue::Keyword("scroll".into())
        );
    }

    #[test]
    fn css_registry_factory() {
        let registry = create_css_property_registry();
        assert_eq!(registry.len(), 5);
        assert!(registry.resolve("display").is_some());
        assert!(registry.resolve("color").is_some());
        assert!(registry.resolve("width").is_some());
        assert!(registry.resolve("opacity").is_some());
        assert!(registry.resolve("overflow").is_some());
        assert!(registry.resolve("unknown").is_none());
    }

    #[test]
    fn css_registry_dynamic_override() {
        let mut registry = create_css_property_registry();
        // Register a dynamic handler for a custom property.
        struct CustomHandler;
        impl CssPropertyHandler for CustomHandler {
            fn property_name(&self) -> &str {
                "accent-color"
            }
            fn parse(&self, _value: &str) -> Result<CssValue, ParseError> {
                Ok(CssValue::Auto)
            }
            fn resolve(&self, value: &CssValue, _ctx: &StyleContext) -> CssValue {
                value.clone()
            }
        }
        registry.register_dynamic("accent-color".into(), Box::new(CustomHandler));
        assert_eq!(registry.len(), 6);
        assert!(registry.resolve("accent-color").is_some());
    }
}
