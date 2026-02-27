//! Computed style representation for resolved CSS property values.
//!
//! [`ComputedStyle`] is an ECS component attached to every element after
//! style resolution. It contains fully resolved values for all Phase 1
//! CSS properties.

use std::fmt;

use crate::CssColor;

/// The CSS `display` property.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Display {
    Block,
    #[default]
    Inline,
    InlineBlock,
    None,
    Flex,
}

impl AsRef<str> for Display {
    fn as_ref(&self) -> &str {
        match self {
            Self::Block => "block",
            Self::Inline => "inline",
            Self::InlineBlock => "inline-block",
            Self::None => "none",
            Self::Flex => "flex",
        }
    }
}

/// The CSS `position` property.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
}

impl AsRef<str> for Position {
    fn as_ref(&self) -> &str {
        match self {
            Self::Static => "static",
            Self::Relative => "relative",
            Self::Absolute => "absolute",
            Self::Fixed => "fixed",
        }
    }
}

/// The CSS `border-*-style` property.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BorderStyle {
    #[default]
    None,
    Solid,
    Dashed,
    Dotted,
}

impl AsRef<str> for BorderStyle {
    fn as_ref(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Solid => "solid",
            Self::Dashed => "dashed",
            Self::Dotted => "dotted",
        }
    }
}

/// Implement `fmt::Display` by delegating to `AsRef<str>`.
macro_rules! display_via_as_ref {
    ($($ty:ty),+ $(,)?) => {
        $(impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_ref())
            }
        })+
    };
}

display_via_as_ref!(Display, Position, BorderStyle);

/// A resolved dimension value: lengths are always in px, percentages in `0..100` range.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    Length(f32),
    Percentage(f32),
    #[default]
    Auto,
}

/// Fully resolved CSS property values for an element.
///
/// Attached as an ECS component by `elidex_style::resolve_styles()`.
/// All relative units have been resolved to absolute pixel values.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedStyle {
    // --- Inherited properties ---
    /// Foreground color. Initial: black.
    pub color: CssColor,
    /// Font size in pixels. Initial: 16.0.
    pub font_size: f32,
    /// Font family list. Initial: `["serif"]`.
    pub font_family: Vec<String>,

    // --- Non-inherited properties ---
    /// Display type. Initial: Inline.
    pub display: Display,
    /// Positioning scheme. Initial: Static.
    pub position: Position,
    /// Background color. Initial: transparent.
    pub background_color: CssColor,

    /// Content width. Initial: Auto.
    pub width: Dimension,
    /// Content height. Initial: Auto.
    pub height: Dimension,

    /// Margin top. Initial: Length(0.0).
    pub margin_top: Dimension,
    /// Margin right. Initial: Length(0.0).
    pub margin_right: Dimension,
    /// Margin bottom. Initial: Length(0.0).
    pub margin_bottom: Dimension,
    /// Margin left. Initial: Length(0.0).
    pub margin_left: Dimension,

    // TODO(Phase 2): replace padding_{top,right,bottom,left} with EdgeSizes
    /// Padding top in pixels. Initial: 0.0.
    pub padding_top: f32,
    /// Padding right in pixels. Initial: 0.0.
    pub padding_right: f32,
    /// Padding bottom in pixels. Initial: 0.0.
    pub padding_bottom: f32,
    /// Padding left in pixels. Initial: 0.0.
    pub padding_left: f32,

    // TODO(Phase 2): replace border_{top,right,bottom,left}_{width,style,color} with BorderSide struct
    /// Border top width in pixels. Computed initial: 0.0 (medium=3px, but 0 when style=none).
    pub border_top_width: f32,
    /// Border right width in pixels. Computed initial: 0.0.
    pub border_right_width: f32,
    /// Border bottom width in pixels. Computed initial: 0.0.
    pub border_bottom_width: f32,
    /// Border left width in pixels. Computed initial: 0.0.
    pub border_left_width: f32,

    /// Border top style. Initial: None.
    pub border_top_style: BorderStyle,
    /// Border right style. Initial: None.
    pub border_right_style: BorderStyle,
    /// Border bottom style. Initial: None.
    pub border_bottom_style: BorderStyle,
    /// Border left style. Initial: None.
    pub border_left_style: BorderStyle,

    /// Border top color. Initial: currentcolor (resolved to `color`).
    pub border_top_color: CssColor,
    /// Border right color. Initial: currentcolor (resolved to `color`).
    pub border_right_color: CssColor,
    /// Border bottom color. Initial: currentcolor (resolved to `color`).
    pub border_bottom_color: CssColor,
    /// Border left color. Initial: currentcolor (resolved to `color`).
    pub border_left_color: CssColor,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        let color = CssColor::BLACK;
        Self {
            // Inherited
            color,
            font_size: 16.0,
            font_family: vec!["serif".to_string()],

            // Non-inherited
            display: Display::default(),
            position: Position::default(),
            background_color: CssColor::TRANSPARENT,

            width: Dimension::Auto,
            height: Dimension::Auto,

            margin_top: Dimension::Length(0.0),
            margin_right: Dimension::Length(0.0),
            margin_bottom: Dimension::Length(0.0),
            margin_left: Dimension::Length(0.0),

            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,

            // CSS initial value is `medium` (3px), but computed value is 0
            // when border-style is `none` (the default).
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,

            border_top_style: BorderStyle::default(),
            border_right_style: BorderStyle::default(),
            border_bottom_style: BorderStyle::default(),
            border_left_style: BorderStyle::default(),

            // currentcolor → resolved to `color` field value
            border_top_color: color,
            border_right_color: color,
            border_bottom_color: color,
            border_left_color: color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_initial_values() {
        let s = ComputedStyle::default();
        assert_eq!(s.color, CssColor::BLACK);
        assert_eq!(s.font_size, 16.0);
        assert_eq!(s.font_family, vec!["serif".to_string()]);
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.position, Position::Static);
        assert_eq!(s.background_color, CssColor::TRANSPARENT);
        assert_eq!(s.width, Dimension::Auto);
        assert_eq!(s.height, Dimension::Auto);
        assert_eq!(s.margin_top, Dimension::Length(0.0));
        assert_eq!(s.padding_top, 0.0);
        assert_eq!(s.border_top_width, 0.0);
        assert_eq!(s.border_top_style, BorderStyle::None);
        // currentcolor → color (BLACK)
        assert_eq!(s.border_top_color, CssColor::BLACK);
    }

    #[test]
    fn enum_defaults() {
        assert_eq!(Display::default(), Display::Inline);
        assert_eq!(Position::default(), Position::Static);
        assert_eq!(BorderStyle::default(), BorderStyle::None);
        assert_eq!(Dimension::default(), Dimension::Auto);
    }

    #[test]
    fn clone_and_partial_eq() {
        let a = ComputedStyle::default();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn dimension_variants() {
        let l = Dimension::Length(42.0);
        let p = Dimension::Percentage(50.0);
        let a = Dimension::Auto;
        assert_ne!(l, p);
        assert_ne!(p, a);
        assert_ne!(l, a);
    }
}
