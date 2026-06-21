//! CSS value types.

use std::fmt;

/// Escape a string's contents per CSSOM "serialize a string" (the
/// caller supplies the surrounding quotes): `"` and `\` get a backslash
/// escape, U+0000 becomes U+FFFD, and other control characters become a
/// hex escape. Without this, a string value containing CSS-significant
/// characters would corrupt the declaration block on re-parse.
#[must_use]
pub fn escape_css_string(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\0' => out.push('\u{FFFD}'),
            '\u{1}'..='\u{1f}' | '\u{7f}' => {
                let _ = write!(out, "\\{:x} ", ch as u32);
            }
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

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
    /// A list of CSS transform functions (CSS Transforms L1 §7 / L2 §12,
    /// "The Transform Functions").
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
    /// (`linear-gradient(...)` etc., CSS Images 3 §3 "Gradients"),
    /// re-parseable by the gradient parser. Backs
    /// [`CssValue::to_css_string`].
    ///
    /// This is the *specified-value* serializer (round-trips the parsed
    /// AST). The *resolved-value* gradient serializers for
    /// `getComputedStyle` live in `elidex-css-background` and consume the
    /// resolved `background::*Gradient` types (normalized stop positions,
    /// `rgb()` colors) — a genuinely separate spec concern, not a
    /// duplicate of this method.
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
                with_prelude(name, &[dir], &stops_text(stops))
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

/// A single CSS transform function (CSS Transforms L1 §7 / L2 §12,
/// "The Transform Functions").
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
    /// Serialize this transform function to its CSS string
    /// representation (CSS Transforms L1 §7 / L2 §12 function forms).
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
                // Non-numeric argument (e.g. `translate(calc(100% - 10px))`
                // stores a `CssValue::Calc`): delegate to the canonical
                // serializer — collapsing to a literal would corrupt the
                // value on the attribute round-trip.
                _ => v.to_css_string(),
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
    /// This is a grouping-preserving round-trip serializer, not the
    /// canonicalizing css-values-4 "serialize a math function" (which
    /// would simplify the tree) — accepted divergence: the output
    /// re-parses to the same tree, which is all the declaration
    /// round-trip needs.
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
            Self::Keyword(s) | Self::RawTokens(s) => s.clone(),
            // CSSOM "serialize a string": quoted + escaped. Unquoted
            // output would let CSS-significant characters inside the
            // string (`;`, `:`) shred the declaration block when the
            // serialized text is re-parsed (style-attribute write-back).
            Self::String(s) => format!("\"{}\"", escape_css_string(s)),
            Self::Length(n, unit) => format!("{n}{}", unit.as_str()),
            Self::Color(c) => c.to_string(),
            Self::Number(n) => format!("{n}"),
            Self::Percentage(n) => format!("{n}%"),
            Self::Auto => "auto".into(),
            Self::Initial => "initial".into(),
            Self::Inherit => "inherit".into(),
            Self::Unset => "unset".into(),
            // KNOWN GAP: `List` does not record its separator, so this
            // comma-join is wrong for space-separated lists (grid track
            // lists, `text-decoration-line`, `counter-reset`) — those
            // values don't re-parse. Needs separator semantics on the
            // type: slot `#11-cssvalue-list-separator-fidelity`.
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
            Self::Url(url) => format!("url(\"{}\")", escape_css_string(url)),
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

    /// CSSOM **resolved/used-value** serialization of this sRGB color
    /// (CSS Color 4 §16.2.2 "CSS serialization of sRGB values"): the
    /// legacy `rgb()` / `rgba()` form — comma separators, exactly one
    /// ASCII space after each comma, base-10 components in `[0, 255]`,
    /// `rgb()` when alpha is `255` (implicit opaque) else `rgba()` with an
    /// explicit alpha serialized per §16.1.
    ///
    /// This is the form `getComputedStyle` returns (CSSOM-1 §9: a color
    /// longhand's resolved value is its *used* value). It is deliberately
    /// **distinct from [`fmt::Display`]** (`#rrggbb`), which is the
    /// *declared* value / inline-style round-trip form backing `cssText`
    /// and the `<input type=color>` sanitizer — those two serialization
    /// contexts are different per spec and must not be unified.
    #[must_use]
    pub fn to_resolved_value_string(&self) -> String {
        if self.a == 255 {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({}, {}, {}, {})",
                self.r,
                self.g,
                self.b,
                serialize_alpha_u8(self.a)
            )
        }
    }
}

/// Serialize an 8-bit alpha component per CSS Color 4 §16.1
/// ("Serializing alpha values").
///
/// - **Integer-percentage preimage (step 2)**: if some integer `n` in
///   `0..=100` satisfies `round(n * 2.55) == a` (ties rounding up), the
///   alpha is `n / 100` — the common case (e.g. `a = 128` → `n = 50` →
///   `"0.5"`). Exact integer arithmetic.
/// - **Otherwise (step 3)**: `a / 255` serialized to at most six decimal
///   places, trailing zeros trimmed. This reproduces §16.1's worked
///   example (`a = 236` → `"0.92549"`) — the form `getComputedStyle` and
///   WPT color serialization checks expect. (§16.1's step-3 prose rounds
///   `a / 0.255` to an integer before dividing by 1000, which would yield
///   `"0.925"`, but the spec's own example serializes the un-rounded
///   `a / 255` value; we follow the example, which is what implementations
///   and the WPT corpus assert.) Six decimal places is far finer than the
///   8-bit resolution, so every value round-trips.
///
/// Leading zero kept, trailing zeros trimmed.
fn serialize_alpha_u8(a: u8) -> String {
    let ai = u32::from(a);

    // Step 2: integer-percentage preimage. round(n * 2.55) = round(n*255/100), ties up.
    for n in 0u32..=100 {
        if (n * 255 + 50) / 100 == ai {
            return format_decimal_ratio(n, 100);
        }
    }

    // Step 3 (§16.1 worked example): a/255 to <=6 decimal places, trailing
    // zeros trimmed. `f64::from(a)` is lossless (u8 → f64); the division and
    // 6-decimal rounding are display-only (a/255 is well-conditioned in
    // [0, 1] — no cancellation), so floating point is appropriate here.
    let s = format!("{:.6}", f64::from(a) / 255.0);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

/// Format `num / den` (a value in `[0, 1]`, `den` a power of ten) as a CSS
/// `<number>`: leading zero kept, trailing zeros trimmed (e.g. `50/100` →
/// `"0.5"`, `926/1000` → `"0.926"`, `0/100` → `"0"`).
fn format_decimal_ratio(num: u32, den: u32) -> String {
    let int_part = num / den;
    let frac = num % den;
    if frac == 0 {
        return int_part.to_string();
    }
    // Fractional width = digits in `den` (a power of ten) minus 1.
    let width = den.to_string().len() - 1;
    let frac_str = format!("{frac:0width$}");
    let frac_str = frac_str.trim_end_matches('0');
    format!("{int_part}.{frac_str}")
}

/// Serializes to the canonical hex form (`#rrggbb`, or legacy `rgba()`
/// when translucent).
///
/// Load-bearing for the re-parseable declaration round-trip: this is the
/// `CssValue::Color` arm of [`CssValue::to_css_string`], which backs
/// `InlineStyle` storage and the `style`-attribute write-back. Changing
/// the format here changes the canonical inline-style form across
/// parser, CSSOM, and cascade paths at once.
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
mod tests;
