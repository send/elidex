//! Layout types for the box model and layout algorithms.

use crate::computed_style::Dimension;

/// An axis-aligned rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// X coordinate of the top-left corner.
    pub x: f32,
    /// Y coordinate of the top-left corner.
    pub y: f32,
    /// Width of the rectangle.
    pub width: f32,
    /// Height of the rectangle.
    pub height: f32,
}

/// A 2D size (width and height).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

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

impl Rect {
    /// Create a rectangle from position and size.
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns a new rectangle expanded outward by the given edge sizes.
    #[must_use]
    pub fn expand(self, edges: EdgeSizes) -> Self {
        Self {
            x: self.x - edges.left,
            y: self.y - edges.top,
            width: self.width + edges.left + edges.right,
            height: self.height + edges.top + edges.bottom,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_default() {
        let r = Rect::default();
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.width, 0.0);
        assert_eq!(r.height, 0.0);
    }

    #[test]
    fn size_default() {
        let s = Size::default();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn edge_sizes_default() {
        let e = EdgeSizes::<f32>::default();
        assert_eq!(e.top, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.bottom, 0.0);
        assert_eq!(e.left, 0.0);
    }

    #[test]
    fn layout_box_padding_box() {
        let b = LayoutBox {
            content: Rect::new(20.0, 20.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            ..Default::default()
        };
        let pb = b.padding_box();
        assert_eq!(pb.x, 10.0);
        assert_eq!(pb.y, 10.0);
        assert_eq!(pb.width, 120.0);
        assert_eq!(pb.height, 70.0);
    }

    #[test]
    fn layout_box_border_box() {
        let b = LayoutBox {
            content: Rect::new(25.0, 25.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            ..Default::default()
        };
        let bb = b.border_box();
        assert_eq!(bb.x, 10.0);
        assert_eq!(bb.y, 10.0);
        assert_eq!(bb.width, 130.0);
        assert_eq!(bb.height, 80.0);
    }

    #[test]
    fn layout_box_margin_box() {
        let b = LayoutBox {
            content: Rect::new(30.0, 30.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            margin: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
        };
        let mb = b.margin_box();
        assert_eq!(mb.x, 10.0); // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.y, 10.0); // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.width, 140.0);
        assert_eq!(mb.height, 90.0);
    }

    #[test]
    fn layout_box_default_all_zero() {
        let b = LayoutBox::default();
        let mb = b.margin_box();
        assert_eq!(mb, Rect::default());
    }

    #[test]
    fn layout_box_asymmetric_edges() {
        let b = LayoutBox {
            content: Rect::new(20.0, 10.0, 200.0, 100.0),
            padding: EdgeSizes {
                top: 5.0,
                right: 15.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            },
            margin: EdgeSizes {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
        };
        let bb = b.border_box();
        assert_eq!(bb.x, 6.0); // 20 - 10 (pad.left) - 4 (border.left)
        assert_eq!(bb.y, 4.0); // 10 - 5 (pad.top) - 1 (border.top)
        assert_eq!(bb.width, 231.0); // 200 + 10 + 15 + 4 + 2
        assert_eq!(bb.height, 119.0); // 100 + 5 + 10 + 1 + 3
    }

    #[test]
    fn layout_context_default() {
        let ctx = LayoutContext::default();
        assert_eq!(ctx.viewport, Size::default());
        assert_eq!(ctx.containing_block, Size::default());
    }

    #[test]
    fn layout_result_default() {
        let r = LayoutResult::default();
        assert_eq!(r.bounds, Rect::default());
        assert_eq!(r.margin, EdgeSizes::default());
    }
}
