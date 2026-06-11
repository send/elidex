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
    /// A time value (e.g. `0.3s`, `200ms`). Stored in seconds.
    Time(f32),
    /// A CSS `url()` value. Stores the URL string as-is (relative or absolute).
    Url(String),
    /// An angle value in degrees (e.g. `45deg`, `0.5turn`).
    Angle(f32),
    /// A gradient value (linear, radial, or conic).
    Gradient(Box<GradientValue>),
    /// A list of CSS transform functions (CSS Transforms L1 §5 / L2 §6).
    TransformList(Vec<TransformFunction>),
}

/// A parsed CSS gradient value before resolution.
#[derive(Clone, Debug, PartialEq)]
pub enum GradientValue {
    /// `linear-gradient()` or `repeating-linear-gradient()`.
    Linear {
        /// Direction: angle or `to <side-or-corner>`.
        direction: AngleOrDirection,
        /// Color stops with optional positions.
        stops: Vec<CssColorStop>,
        /// Whether this is a `repeating-linear-gradient()`.
        repeating: bool,
    },
    /// `radial-gradient()` or `repeating-radial-gradient()`.
    Radial {
        /// Shape: `circle` or `ellipse`.
        shape: Option<String>,
        /// Size keyword or explicit lengths.
        size: Option<String>,
        /// Center position.
        position: Option<Vec<CssValue>>,
        /// Color stops with optional positions.
        stops: Vec<CssColorStop>,
        /// Whether this is a `repeating-radial-gradient()`.
        repeating: bool,
    },
    /// `conic-gradient()` or `repeating-conic-gradient()`.
    Conic {
        /// Start angle in degrees.
        from_angle: Option<f32>,
        /// Center position.
        position: Option<Vec<CssValue>>,
        /// Angular color stops.
        stops: Vec<CssColorStop>,
        /// Whether this is a `repeating-conic-gradient()`.
        repeating: bool,
    },
}

/// A color stop in a CSS gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct CssColorStop {
    /// The color value.
    pub color: CssValue,
    /// Optional position (length or percentage).
    pub position: Option<CssValue>,
}

/// Direction for a linear gradient.
#[derive(Clone, Debug, PartialEq)]
pub enum AngleOrDirection {
    /// An explicit angle in degrees.
    Angle(f32),
    /// A `to <side-or-corner>` direction (e.g. `["top"]`, `["top", "right"]`).
    To(Vec<String>),
}

impl GradientValue {
    /// Serialize this gradient back to its CSS function form
    /// (`linear-gradient(...)` etc.), re-parseable by the gradient
    /// parser. Backs [`CssValue::to_css_string`].
    #[must_use]
    pub fn to_css_string(&self) -> String {
        fn stops_text(stops: &[CssColorStop]) -> String {
            stops
                .iter()
                .map(|s| match &s.position {
                    Some(p) => format!("{} {}", s.color.to_css_string(), p.to_css_string()),
                    None => s.color.to_css_string(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        fn position_text(position: &[CssValue]) -> String {
            position
                .iter()
                .map(CssValue::to_css_string)
                .collect::<Vec<_>>()
                .join(" ")
        }
        fn with_prelude(name: &str, prelude: &[String], stops: &str) -> String {
            if prelude.is_empty() {
                format!("{name}({stops})")
            } else {
                format!("{name}({}, {stops})", prelude.join(" "))
            }
        }
        match self {
            Self::Linear {
                direction,
                stops,
                repeating,
            } => {
                let name = if *repeating {
                    "repeating-linear-gradient"
                } else {
                    "linear-gradient"
                };
                let dir = match direction {
                    AngleOrDirection::Angle(a) => format!("{a}deg"),
                    AngleOrDirection::To(parts) => format!("to {}", parts.join(" ")),
                };
                format!("{name}({dir}, {})", stops_text(stops))
            }
            Self::Radial {
                shape,
                size,
                position,
                stops,
                repeating,
            } => {
                let name = if *repeating {
                    "repeating-radial-gradient"
                } else {
                    "radial-gradient"
                };
                let mut prelude: Vec<String> = Vec::new();
                if let Some(s) = shape {
                    prelude.push(s.clone());
                }
                if let Some(s) = size {
                    prelude.push(s.clone());
                }
                if let Some(p) = position {
                    prelude.push(format!("at {}", position_text(p)));
                }
                with_prelude(name, &prelude, &stops_text(stops))
            }
            Self::Conic {
                from_angle,
                position,
                stops,
                repeating,
            } => {
                let name = if *repeating {
                    "repeating-conic-gradient"
                } else {
                    "conic-gradient"
                };
                let mut prelude: Vec<String> = Vec::new();
                if let Some(a) = from_angle {
                    prelude.push(format!("from {a}deg"));
                }
                if let Some(p) = position {
                    prelude.push(format!("at {}", position_text(p)));
                }
                with_prelude(name, &prelude, &stops_text(stops))
            }
        }
    }
}

/// A single CSS transform function (CSS Transforms L1 §5, L2 §6).
#[derive(Clone, Debug, PartialEq)]
pub enum TransformFunction {
    // --- Level 1 ---
    /// `translate(<length-percentage>, <length-percentage>?)`
    Translate(CssValue, CssValue),
    /// `translateX(<length-percentage>)`
    TranslateX(CssValue),
    /// `translateY(<length-percentage>)`
    TranslateY(CssValue),
    /// `rotate(<angle>)` — degrees
    Rotate(f32),
    /// `scale(<number>, <number>?)`
    Scale(f32, f32),
    /// `scaleX(<number>)`
    ScaleX(f32),
    /// `scaleY(<number>)`
    ScaleY(f32),
    /// `skew(<angle>, <angle>?)` — degrees
    Skew(f32, f32),
    /// `skewX(<angle>)` — degrees
    SkewX(f32),
    /// `skewY(<angle>)` — degrees
    SkewY(f32),
    /// `matrix(a, b, c, d, e, f)`
    Matrix([f64; 6]),
    // --- Level 2 (3D) ---
    /// `translate3d(<length-percentage>, <length-percentage>, <length>)`
    Translate3d(CssValue, CssValue, CssValue),
    /// `translateZ(<length>)`
    TranslateZ(CssValue),
    /// `rotate3d(<number>, <number>, <number>, <angle>)`
    Rotate3d(f64, f64, f64, f32),
    /// `rotateX(<angle>)` — degrees
    RotateX(f32),
    /// `rotateY(<angle>)` — degrees
    RotateY(f32),
    /// `rotateZ(<angle>)` — degrees (alias for `rotate()`)
    RotateZ(f32),
    /// `scale3d(<number>, <number>, <number>)`
    Scale3d(f32, f32, f32),
    /// `scaleZ(<number>)`
    ScaleZ(f32),
    /// `matrix3d(16 values)`
    Matrix3d([f64; 16]),
    /// `perspective(<length>)` — transform function (not property)
    PerspectiveFunc(f32),
}

impl TransformFunction {
    /// Serialize this transform function to its CSS string representation.
    ///
    /// Non-finite values (NaN, Infinity) are replaced with 0 to ensure
    /// valid CSS output.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn to_css_string(&self) -> String {
        /// Replace NaN/Infinity with 0 to produce valid CSS.
        fn sf(v: f32) -> f32 {
            if v.is_finite() {
                v
            } else {
                0.0
            }
        }
        fn sf64(v: f64) -> f64 {
            if v.is_finite() {
                v
            } else {
                0.0
            }
        }

        fn fmt_val(v: &CssValue) -> String {
            match v {
                CssValue::Length(n, unit) => {
                    let n = sf(*n);
                    format!("{n}{}", unit.as_str())
                }
                CssValue::Percentage(n) => {
                    let n = sf(*n);
                    format!("{n}%")
                }
                CssValue::Number(n) => {
                    let n = sf(*n);
                    format!("{n}")
                }
                _ => "0px".to_string(),
            }
        }

        match self {
            Self::Translate(x, y) => format!("translate({}, {})", fmt_val(x), fmt_val(y)),
            Self::TranslateX(x) => format!("translateX({})", fmt_val(x)),
            Self::TranslateY(y) => format!("translateY({})", fmt_val(y)),
            Self::Rotate(deg) => {
                let deg = sf(*deg);
                format!("rotate({deg}deg)")
            }
            Self::Scale(sx, sy) => {
                let (sx, sy) = (sf(*sx), sf(*sy));
                if (sx - sy).abs() < f32::EPSILON {
                    format!("scale({sx})")
                } else {
                    format!("scale({sx}, {sy})")
                }
            }
            Self::ScaleX(s) => {
                let s = sf(*s);
                format!("scaleX({s})")
            }
            Self::ScaleY(s) => {
                let s = sf(*s);
                format!("scaleY({s})")
            }
            Self::Skew(ax, ay) => {
                let (ax, ay) = (sf(*ax), sf(*ay));
                format!("skew({ax}deg, {ay}deg)")
            }
            Self::SkewX(a) => {
                let a = sf(*a);
                format!("skewX({a}deg)")
            }
            Self::SkewY(a) => {
                let a = sf(*a);
                format!("skewY({a}deg)")
            }
            Self::Matrix(m) => {
                let m: Vec<f64> = m.iter().map(|v| sf64(*v)).collect();
                format!(
                    "matrix({}, {}, {}, {}, {}, {})",
                    m[0], m[1], m[2], m[3], m[4], m[5]
                )
            }
            Self::Translate3d(x, y, z) => {
                format!(
                    "translate3d({}, {}, {})",
                    fmt_val(x),
                    fmt_val(y),
                    fmt_val(z)
                )
            }
            Self::TranslateZ(z) => format!("translateZ({})", fmt_val(z)),
            Self::Rotate3d(x, y, z, deg) => {
                let (x, y, z, deg) = (sf64(*x), sf64(*y), sf64(*z), sf(*deg));
                format!("rotate3d({x}, {y}, {z}, {deg}deg)")
            }
            Self::RotateX(deg) => {
                let deg = sf(*deg);
                format!("rotateX({deg}deg)")
            }
            Self::RotateY(deg) => {
                let deg = sf(*deg);
                format!("rotateY({deg}deg)")
            }
            Self::RotateZ(deg) => {
                let deg = sf(*deg);
                format!("rotateZ({deg}deg)")
            }
            Self::Scale3d(sx, sy, sz) => {
                let (sx, sy, sz) = (sf(*sx), sf(*sy), sf(*sz));
                format!("scale3d({sx}, {sy}, {sz})")
            }
            Self::ScaleZ(s) => {
                let s = sf(*s);
                format!("scaleZ({s})")
            }
            Self::Matrix3d(m) => {
                let vals: Vec<String> = m.iter().map(|v| format!("{}", sf64(*v))).collect();
                format!("matrix3d({})", vals.join(", "))
            }
            Self::PerspectiveFunc(d) => {
                let d = sf(*d);
                if d == 0.0 {
                    "perspective(none)".to_string()
                } else {
                    format!("perspective({d}px)")
                }
            }
        }
    }
}

/// CSS `transform-style` property (CSS Transforms L2 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransformStyle {
    #[default]
    Flat,
    Preserve3d,
}

/// CSS `backface-visibility` property (CSS Transforms L2 §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackfaceVisibility {
    #[default]
    Visible,
    Hidden,
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

    /// Serialize this expression as the body of a `calc()` function (no
    /// outer `calc(...)` wrapper — [`CssValue::to_css_string`] adds it).
    ///
    /// Compound operands are parenthesized so the re-parsed tree
    /// preserves grouping (`a - (b + c)` must not flatten to
    /// `a - b + c`); the `calc()` parser accepts nested parentheses.
    #[must_use]
    pub fn to_css_string(&self) -> String {
        fn operand(e: &CalcExpr) -> String {
            match e {
                CalcExpr::Length(..) | CalcExpr::Percentage(_) | CalcExpr::Number(_) => {
                    e.to_css_string()
                }
                _ => format!("({})", e.to_css_string()),
            }
        }
        match self {
            Self::Length(v, unit) => format!("{v}{}", unit.as_str()),
            Self::Percentage(p) => format!("{p}%"),
            Self::Number(n) => format!("{n}"),
            Self::Add(a, b) => format!("{} + {}", operand(a), operand(b)),
            Self::Sub(a, b) => format!("{} - {}", operand(a), operand(b)),
            Self::Mul(a, b) => format!("{} * {}", operand(a), operand(b)),
            Self::Div(a, b) => format!("{} / {}", operand(a), operand(b)),
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

    /// Convert this value to its CSS string representation (CSSOM
    /// "serialize a CSS value").
    ///
    /// This is the canonical value→text form backing `InlineStyle`
    /// storage and CSSOM `cssText` round-trips: the serialized text is
    /// written back into the `style` content attribute and re-parsed by
    /// the cascade, so every arm must produce re-parseable CSS. The
    /// match is exhaustive within the defining crate — adding a
    /// `CssValue` variant forces a serializer arm at compile time
    /// rather than silently corrupting the attribute round-trip.
    #[must_use]
    pub fn to_css_string(&self) -> String {
        match self {
            Self::Keyword(s) | Self::String(s) | Self::RawTokens(s) => s.clone(),
            Self::Length(n, unit) => format!("{n}{}", unit.as_str()),
            Self::Color(c) => c.to_string(),
            Self::Number(n) => format!("{n}"),
            Self::Percentage(n) => format!("{n}%"),
            Self::Auto => "auto".into(),
            Self::Initial => "initial".into(),
            Self::Inherit => "inherit".into(),
            Self::Unset => "unset".into(),
            Self::List(items) => items
                .iter()
                .map(Self::to_css_string)
                .collect::<Vec<_>>()
                .join(", "),
            Self::Var(name, fallback) => match fallback {
                Some(fb) => format!("var({name}, {})", fb.to_css_string()),
                None => format!("var({name})"),
            },
            Self::Calc(expr) => format!("calc({})", expr.to_css_string()),
            Self::Time(secs) => {
                // CSS time values are stored in seconds; serialize to ms if
                // it's a clean millisecond value, otherwise use seconds.
                let ms = secs * 1000.0;
                #[allow(clippy::cast_possible_truncation)]
                if (ms - ms.round()).abs() < f32::EPSILON && ms >= 0.0 {
                    format!("{}ms", ms.round() as i32)
                } else {
                    format!("{secs}s")
                }
            }
            Self::Url(url) => format!(
                "url(\"{}\")",
                url.replace('\\', "\\\\").replace('"', "\\\"")
            ),
            Self::Angle(deg) => format!("{deg}deg"),
            Self::Gradient(g) => g.to_css_string(),
            Self::TransformList(funcs) => funcs
                .iter()
                .map(TransformFunction::to_css_string)
                .collect::<Vec<_>>()
                .join(" "),
        }
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

impl LengthUnit {
    /// The canonical CSS unit suffix (`px`, `em`, …).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Px => "px",
            Self::Em => "em",
            Self::Rem => "rem",
            Self::Vw => "vw",
            Self::Vh => "vh",
            Self::Vmin => "vmin",
            Self::Vmax => "vmax",
            Self::Fr => "fr",
        }
    }
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
    fn length_unit_as_str_exhaustive() {
        assert_eq!(LengthUnit::Px.as_str(), "px");
        assert_eq!(LengthUnit::Vmin.as_str(), "vmin");
        assert_eq!(LengthUnit::Fr.as_str(), "fr");
    }
}
