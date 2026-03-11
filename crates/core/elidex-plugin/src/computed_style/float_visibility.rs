//! Float, clear, and visibility keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `float` property.
    Float {
        None => "none",
        Left => "left",
        Right => "right",
    }
}

keyword_enum! {
    /// The CSS `clear` property.
    Clear {
        None => "none",
        Left => "left",
        Right => "right",
        Both => "both",
    }
}

keyword_enum! {
    /// The CSS `visibility` property. **Inherited.**
    Visibility {
        Visible => "visible",
        Hidden => "hidden",
        Collapse => "collapse",
    }
}

/// The CSS `vertical-align` property.
///
/// Applies to inline-level and table-cell elements.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum VerticalAlign {
    /// Align baseline of element with baseline of parent.
    #[default]
    Baseline,
    /// Lower baseline of element.
    Sub,
    /// Raise baseline of element.
    Super,
    /// Align top of element with top of parent's font.
    TextTop,
    /// Align bottom of element with bottom of parent's font.
    TextBottom,
    /// Center element vertically.
    Middle,
    /// Align top of element with top of line box.
    Top,
    /// Align bottom of element with bottom of line box.
    Bottom,
    /// Offset by a fixed length in pixels (resolved).
    Length(f32),
    /// Offset by a percentage of line-height.
    Percentage(f32),
}

impl VerticalAlign {
    /// Parse a CSS keyword string into a `VerticalAlign` variant.
    #[must_use]
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "baseline" => Some(Self::Baseline),
            "sub" => Some(Self::Sub),
            "super" => Some(Self::Super),
            "text-top" => Some(Self::TextTop),
            "text-bottom" => Some(Self::TextBottom),
            "middle" => Some(Self::Middle),
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

impl fmt::Display for VerticalAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Baseline => f.write_str("baseline"),
            Self::Sub => f.write_str("sub"),
            Self::Super => f.write_str("super"),
            Self::TextTop => f.write_str("text-top"),
            Self::TextBottom => f.write_str("text-bottom"),
            Self::Middle => f.write_str("middle"),
            Self::Top => f.write_str("top"),
            Self::Bottom => f.write_str("bottom"),
            Self::Length(px) => write!(f, "{px}px"),
            Self::Percentage(pct) => write!(f, "{pct}%"),
        }
    }
}
