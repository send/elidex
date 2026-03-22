//! Background layer types for CSS Backgrounds Level 3.
//!
//! These types represent fully resolved (computed) background layer values.
//! Parse-stage gradient values live in [`CssValue::Gradient`](crate::CssValue).

use crate::{CssColor, Point, Size};

/// A resolved background image for a single layer.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum BackgroundImage {
    /// No background image.
    #[default]
    None,
    /// A URL reference to an external image.
    Url(String),
    /// A resolved linear gradient.
    LinearGradient(LinearGradient),
    /// A resolved radial gradient.
    RadialGradient(RadialGradient),
    /// A resolved conic gradient.
    ConicGradient(ConicGradient),
}

/// A resolved linear gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    /// Gradient line angle in degrees (0 = to top, 90 = to right).
    pub angle: f32,
    /// Resolved color stops with normalized positions (0.0–1.0).
    pub stops: Vec<ColorStop>,
    /// Whether this is a repeating gradient.
    pub repeating: bool,
}

/// A resolved radial gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    /// Center position in pixels relative to the painting area.
    pub center: Point,
    /// Horizontal and vertical radii in pixels.
    pub radii: Size,
    /// Resolved color stops with normalized positions (0.0–1.0).
    pub stops: Vec<ColorStop>,
    /// Whether this is a repeating gradient.
    pub repeating: bool,
}

/// A resolved conic gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct ConicGradient {
    /// Center position in pixels relative to the painting area.
    pub center: Point,
    /// Start angle in degrees.
    pub start_angle: f32,
    /// End angle in degrees (typically `start_angle + 360.0`).
    pub end_angle: f32,
    /// Resolved angular color stops with positions in degrees.
    pub stops: Vec<ColorStop>,
    /// Whether this is a repeating gradient.
    pub repeating: bool,
}

/// A resolved color stop in a gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorStop {
    /// The stop color.
    pub color: CssColor,
    /// Position along the gradient line (0.0–1.0 for linear/radial, degrees for conic).
    pub position: f32,
}

/// Background position for a single layer.
#[derive(Clone, Debug, PartialEq)]
pub struct BgPosition {
    /// Horizontal position.
    pub x: BgPositionAxis,
    /// Vertical position.
    pub y: BgPositionAxis,
}

impl Default for BgPosition {
    fn default() -> Self {
        Self {
            x: BgPositionAxis::Percentage(0.0),
            y: BgPositionAxis::Percentage(0.0),
        }
    }
}

/// A single axis of a background position.
#[derive(Clone, Debug, PartialEq)]
pub enum BgPositionAxis {
    /// Length in pixels.
    Length(f32),
    /// Percentage (0.0 = start, 100.0 = end).
    Percentage(f32),
    /// Edge offset: `right 10px` → `Edge(Right, 10.0)`.
    Edge(PositionEdge, f32),
}

/// Named edge for background-position offsets.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PositionEdge {
    /// Left edge (default for x-axis).
    Left,
    /// Right edge.
    Right,
    /// Top edge (default for y-axis).
    Top,
    /// Bottom edge.
    Bottom,
}

/// Background size for a single layer.
#[derive(Clone, Debug, PartialEq)]
pub enum BgSize {
    /// Scale to cover the entire painting area.
    Cover,
    /// Scale to fit within the painting area.
    Contain,
    /// Explicit size `(width, height)`. `None` = `auto`.
    Explicit(Option<BgSizeDimension>, Option<BgSizeDimension>),
}

impl Default for BgSize {
    fn default() -> Self {
        Self::Explicit(None, None) // auto auto
    }
}

/// A dimension value for background-size.
#[derive(Clone, Debug, PartialEq)]
pub enum BgSizeDimension {
    /// Length in pixels.
    Length(f32),
    /// Percentage of the painting area.
    Percentage(f32),
}

/// Background repeat for a single layer.
#[derive(Clone, Debug, PartialEq)]
pub struct BgRepeat {
    /// Horizontal repeat mode.
    pub x: BgRepeatAxis,
    /// Vertical repeat mode.
    pub y: BgRepeatAxis,
}

impl Default for BgRepeat {
    fn default() -> Self {
        Self {
            x: BgRepeatAxis::Repeat,
            y: BgRepeatAxis::Repeat,
        }
    }
}

/// Repeat mode for a single axis.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BgRepeatAxis {
    /// Tile the image (default).
    #[default]
    Repeat,
    /// Do not tile.
    NoRepeat,
    /// Space tiles evenly (no clipping).
    Space,
    /// Round tile count and resize to fill.
    Round,
}

/// Box area for background-origin and background-clip.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BoxArea {
    /// Border box.
    BorderBox,
    /// Padding box (default for origin).
    #[default]
    PaddingBox,
    /// Content box.
    ContentBox,
}

/// Background attachment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BgAttachment {
    /// Scrolls with the element (default).
    #[default]
    Scroll,
    /// Fixed relative to the viewport.
    Fixed,
    /// Scrolls with the element's content.
    Local,
}

/// A single resolved background layer.
#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundLayer {
    /// The background image for this layer.
    pub image: BackgroundImage,
    /// Position within the positioning area.
    pub position: BgPosition,
    /// Size of the background image.
    pub size: BgSize,
    /// Repeat mode.
    pub repeat: BgRepeat,
    /// Positioning area (where position is calculated against).
    pub origin: BoxArea,
    /// Painting area (where the image is clipped to).
    pub clip: BoxArea,
    /// Attachment mode.
    pub attachment: BgAttachment,
}

impl Default for BackgroundLayer {
    fn default() -> Self {
        Self {
            image: BackgroundImage::None,
            position: BgPosition::default(),
            size: BgSize::default(),
            repeat: BgRepeat::default(),
            origin: BoxArea::PaddingBox,
            clip: BoxArea::BorderBox,
            attachment: BgAttachment::Scroll,
        }
    }
}
