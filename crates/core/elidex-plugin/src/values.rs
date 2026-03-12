//! CSS value types.

use std::fmt;

/// A parsed CSS value before resolution.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CssValue {
    /// A CSS keyword (e.g. `block`, `inline`, `none`).
    Keyword(String),
    /// A length with a unit (e.g. `10px`, `2em`).
    Length(f32, LengthUnit),
    /// An RGBA color value.
    Color(CssColor),
    /// A unitless number (e.g. `line-height: 1.5`).
    Number(f32),
    /// A percentage value (e.g. `50%`). Stored as `50.0`, not `0.5`.
    Percentage(f32),
    /// A quoted or unquoted string value.
    String(String),
    /// The `auto` keyword.
    Auto,
    /// The `initial` keyword.
    Initial,
    /// The `inherit` keyword.
    Inherit,
    /// The `unset` keyword.
    Unset,
    /// A list of CSS values (e.g. `font-family: Arial, sans-serif`).
    List(Vec<CssValue>),
    /// A `var()` function reference.
    ///
    /// First argument is the custom property name (e.g. `--bg`),
    /// second is an optional fallback value (e.g. `var(--bg, #000)`).
    Var(String, Option<Box<CssValue>>),
    /// Raw token string for custom property values.
    ///
    /// Custom properties (CSS Variables Level 1) accept arbitrary token
    /// sequences that are not type-checked at parse time.
    RawTokens(String),
    /// A `calc()` expression (CSS Values Level 3 §8).
    Calc(Box<CalcExpr>),
}

/// A node in a `calc()` expression tree.
///
/// Supports `+`, `-`, `*`, `/` with length, percentage, and number operands.
#[derive(Clone, Debug, PartialEq)]
pub enum CalcExpr {
    /// A length value (resolved to a specific unit).
    Length(f32, LengthUnit),
    /// A percentage value.
    Percentage(f32),
    /// A unitless number.
    Number(f32),
    /// Addition: `left + right`.
    Add(Box<CalcExpr>, Box<CalcExpr>),
    /// Subtraction: `left - right`.
    Sub(Box<CalcExpr>, Box<CalcExpr>),
    /// Multiplication: `left * right` (one operand must be a number).
    Mul(Box<CalcExpr>, Box<CalcExpr>),
    /// Division: `left / right` (right must be a number).
    Div(Box<CalcExpr>, Box<CalcExpr>),
}

impl CalcExpr {
    /// Returns `true` if this expression tree contains any percentage terms.
    ///
    /// Used to reject `calc()` with percentages in length-only properties
    /// such as `letter-spacing` and `word-spacing`.
    #[must_use]
    pub fn contains_percentage(&self) -> bool {
        match self {
            Self::Percentage(_) => true,
            Self::Length(..) | Self::Number(_) => false,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) | Self::Div(a, b) => {
                a.contains_percentage() || b.contains_percentage()
            }
        }
    }
}

impl CssValue {
    /// Extract the keyword string if this is a `Keyword` variant.
    #[must_use]
    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Self::Keyword(s) => Some(s),
            _ => None,
        }
    }

    /// Extract the length and unit if this is a `Length` variant.
    #[must_use]
    pub fn as_length(&self) -> Option<(f32, LengthUnit)> {
        match self {
            Self::Length(v, u) => Some((*v, *u)),
            _ => None,
        }
    }

    /// Extract the color if this is a `Color` variant.
    #[must_use]
    pub fn as_color(&self) -> Option<&CssColor> {
        match self {
            Self::Color(c) => Some(c),
            _ => None,
        }
    }

    /// Extract the number if this is a `Number` variant.
    #[must_use]
    pub fn as_number(&self) -> Option<f32> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Extract the percentage if this is a `Percentage` variant.
    #[must_use]
    pub fn as_percentage(&self) -> Option<f32> {
        match self {
            Self::Percentage(p) => Some(*p),
            _ => None,
        }
    }

    /// Returns `true` if this is the `Auto` variant.
    #[must_use]
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Returns `true` if this is a global keyword (`initial`, `inherit`, or `unset`).
    #[must_use]
    pub fn is_global_keyword(&self) -> bool {
        matches!(self, Self::Initial | Self::Inherit | Self::Unset)
    }
}

/// CSS length units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LengthUnit {
    /// Pixels (`px`).
    Px,
    /// Relative to parent font size (`em`).
    Em,
    /// Relative to root font size (`rem`).
    Rem,
    /// Viewport width percentage (`vw`).
    Vw,
    /// Viewport height percentage (`vh`).
    Vh,
    /// Smaller of `vw` and `vh` (`vmin`).
    Vmin,
    /// Larger of `vw` and `vh` (`vmax`).
    Vmax,
    /// Flexible fraction (`fr`) for CSS Grid.
    Fr,
}

/// An RGBA color value.
///
/// Each component is an 8-bit unsigned integer (`0..=255`).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CssColor {
    /// Red component.
    pub r: u8,
    /// Green component.
    pub g: u8,
    /// Blue component.
    pub b: u8,
    /// Alpha component (`255` = fully opaque).
    pub a: u8,
}

impl CssColor {
    /// Create a new color with the given RGBA components.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a fully opaque color.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Transparent (all zeros).
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    /// Black (`#000000`).
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White (`#ffffff`).
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Red (`#ff0000`).
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// Green (`#008000`).
    pub const GREEN: Self = Self::rgb(0, 128, 0);
    /// Blue (`#0000ff`).
    pub const BLUE: Self = Self::rgb(0, 0, 255);
}

impl fmt::Display for CssColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "rgba({}, {}, {}, {:.2})",
                self.r,
                self.g,
                self.b,
                f64::from(self.a) / 255.0
            )
        }
    }
}

#[cfg(test)]
mod tests {
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
}
