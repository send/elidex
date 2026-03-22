//! Box model types: `EdgeSizes`, `LayoutBox`, `LayoutContext`, `LayoutResult`.

use super::rect::{Rect, Size};
use crate::computed_style::Dimension;

/// Edge sizes for padding, border, and margin.
///
/// The default type parameter `f32` is used for used/layout values.
/// `EdgeSizes<Dimension>` stores computed values that may contain percentages.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSizes<T = f32> {
    /// Top edge size.
    pub top: T,
    /// Right edge size.
    pub right: T,
    /// Bottom edge size.
    pub bottom: T,
    /// Left edge size.
    pub left: T,
}

impl Default for EdgeSizes<f32> {
    fn default() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl Default for EdgeSizes<Dimension> {
    fn default() -> Self {
        Self {
            top: Dimension::ZERO,
            right: Dimension::ZERO,
            bottom: Dimension::ZERO,
            left: Dimension::ZERO,
        }
    }
}

impl EdgeSizes {
    /// Create edge sizes with individual values for each side.
    #[must_use]
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Create edge sizes with the same value on all sides.
    #[must_use]
    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Sum of left and right edges.
    #[must_use]
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Sum of top and bottom edges.
    #[must_use]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// A box in the layout tree.
///
/// Represents the CSS box model with content, padding, border, and margin areas.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutBox {
    /// The content area.
    pub content: Rect,
    /// Padding between content and border.
    pub padding: EdgeSizes,
    /// Border widths.
    pub border: EdgeSizes,
    /// Margin outside the border.
    pub margin: EdgeSizes,
    /// First baseline offset from content box top edge (`None` if no baseline).
    ///
    /// CSS 2.1 §10.8.1: the first baseline of a box is the first baseline
    /// of its first in-flow line box or block child that has a baseline.
    pub first_baseline: Option<f32>,
}

impl LayoutBox {
    /// Returns the padding box (content + padding).
    #[must_use]
    pub fn padding_box(&self) -> Rect {
        self.content.expand(self.padding)
    }

    /// Returns the border box (content + padding + border).
    #[must_use]
    pub fn border_box(&self) -> Rect {
        self.padding_box().expand(self.border)
    }

    /// Returns the margin box (content + padding + border + margin).
    ///
    /// Note: negative margins can produce a `Rect` with negative width or height.
    #[must_use]
    pub fn margin_box(&self) -> Rect {
        self.border_box().expand(self.margin)
    }
}

/// Context available to a layout algorithm.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutContext {
    /// The viewport size.
    pub viewport: Size,
    /// The containing block size.
    pub containing_block: Size,
}

/// The result of a layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutResult {
    /// The positioned bounding rectangle.
    pub bounds: Rect,
    /// The computed margins.
    pub margin: EdgeSizes,
    /// The computed padding.
    pub padding: EdgeSizes,
    /// The computed border widths.
    pub border: EdgeSizes,
}
