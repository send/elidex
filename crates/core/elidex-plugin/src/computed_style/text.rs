//! Text-related keyword enums and types.

use std::fmt;

keyword_enum! {
    /// The CSS `text-align` property (CSS Text Level 3 §7.1).
    ///
    /// Per spec, initial value is `start` (direction-dependent).
    TextAlign {
        Start => "start",
        End => "end",
        Left => "left",
        Center => "center",
        Right => "right",
    }
}

keyword_enum! {
    /// The CSS `text-transform` property.
    TextTransform {
        None => "none",
        Uppercase => "uppercase",
        Lowercase => "lowercase",
        Capitalize => "capitalize",
    }
}

keyword_enum! {
    /// The CSS `white-space` property.
    WhiteSpace {
        Normal => "normal",
        Pre => "pre",
        NoWrap => "nowrap",
        PreWrap => "pre-wrap",
        PreLine => "pre-line",
    }
}

keyword_enum! {
    /// The CSS `list-style-type` property.
    ListStyleType {
        Disc => "disc",
        Circle => "circle",
        Square => "square",
        Decimal => "decimal",
        None => "none",
    }
}

keyword_enum! {
    /// The CSS `font-style` property.
    FontStyle {
        Normal => "normal",
        Italic => "italic",
        Oblique => "oblique",
    }
}

/// The CSS `text-decoration-line` property.
///
/// Not inherited. Multiple values possible (e.g. `underline overline line-through`).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextDecorationLine {
    /// Whether `underline` is set.
    pub underline: bool,
    /// Whether `overline` is set.
    pub overline: bool,
    /// Whether `line-through` is set.
    pub line_through: bool,
}

impl fmt::Display for TextDecorationLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.underline && !self.overline && !self.line_through {
            return f.write_str("none");
        }
        let mut first = true;
        if self.underline {
            f.write_str("underline")?;
            first = false;
        }
        if self.overline {
            if !first {
                f.write_str(" ")?;
            }
            f.write_str("overline")?;
            first = false;
        }
        if self.line_through {
            if !first {
                f.write_str(" ")?;
            }
            f.write_str("line-through")?;
        }
        Ok(())
    }
}

keyword_enum! {
    /// The CSS `text-decoration-style` property (CSS Text Decoration 3 §2.2).
    TextDecorationStyle {
        Solid => "solid",
        Double => "double",
        Dotted => "dotted",
        Dashed => "dashed",
        Wavy => "wavy",
    }
}

/// The CSS `line-height` property, preserving keyword/number semantics.
///
/// CSS Variables Level 1 requires `normal` and unitless `<number>` to be
/// inherited as-is and recomputed relative to each element's `font-size`.
/// Storing the resolved px value at computed time would lose this semantic.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LineHeight {
    /// `line-height: normal` — typically 1.2 × font-size.
    #[default]
    Normal,
    /// Unitless number multiplier (e.g. `line-height: 1.5`).
    Number(f32),
    /// Absolute length in pixels (e.g. `line-height: 24px` or resolved from `%`).
    Px(f32),
}

impl LineHeight {
    /// Resolve to an absolute pixel value given the element's font size.
    #[must_use]
    pub fn resolve_px(self, font_size: f32) -> f32 {
        match self {
            Self::Normal => font_size * 1.2,
            Self::Number(n) => font_size * n,
            Self::Px(px) => px,
        }
    }
}

impl fmt::Display for LineHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => f.write_str("normal"),
            Self::Number(n) => write!(f, "{n}"),
            Self::Px(px) => write!(f, "{px}px"),
        }
    }
}
