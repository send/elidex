//! Box model keyword enums and dimension types.

use std::fmt;

use crate::CssColor;

keyword_enum! {
    /// The CSS `box-sizing` property.
    BoxSizing {
        ContentBox => "content-box",
        BorderBox => "border-box",
    }
}

keyword_enum! {
    /// The CSS `border-*-style` property (CSS Backgrounds and Borders Level 3 §4.1).
    BorderStyle {
        None => "none",
        Hidden => "hidden",
        Solid => "solid",
        Dashed => "dashed",
        Dotted => "dotted",
        Double => "double",
        Groove => "groove",
        Ridge => "ridge",
        Inset => "inset",
        Outset => "outset",
    }
}

/// A single item in a `content` property value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ContentItem {
    /// A literal string (e.g. `content: ">>"`).
    String(String),
    /// An `attr()` function reference (e.g. `content: attr(title)`).
    Attr(String),
}

/// The computed value of the CSS `content` property.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum ContentValue {
    /// `content: normal` — no generated content for regular elements.
    #[default]
    Normal,
    /// `content: none` — suppress generated content.
    None,
    /// One or more content items.
    Items(Vec<ContentItem>),
}

/// A single border side with width, style, and color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSide {
    /// Border width in pixels.
    pub width: f32,
    /// Border line style.
    pub style: BorderStyle,
    /// Border color.
    pub color: CssColor,
}

impl BorderSide {
    /// A border side with no visible border (width 0, style none).
    pub const NONE: Self = Self {
        width: 0.0,
        style: BorderStyle::None,
        color: CssColor::BLACK,
    };
}

impl Default for BorderSide {
    fn default() -> Self {
        Self::NONE
    }
}

/// A resolved dimension value: lengths are always in px, percentages in `0..100` range.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    Length(f32),
    Percentage(f32),
    #[default]
    Auto,
}

impl Dimension {
    /// Zero-length constant (`Length(0.0)`), used as the CSS initial value
    /// for margins, `min-width`, `min-height`, etc.
    pub const ZERO: Self = Self::Length(0.0);
}
