//! Layout types used by [`LayoutModel`](crate::LayoutModel).

/// An axis-aligned rectangle.
#[derive(Clone, Debug, Default, PartialEq)]
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Size {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

/// Edge sizes for padding, border, and margin.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EdgeSizes {
    /// Top edge size in pixels.
    pub top: f32,
    /// Right edge size in pixels.
    pub right: f32,
    /// Bottom edge size in pixels.
    pub bottom: f32,
    /// Left edge size in pixels.
    pub left: f32,
}

/// A box in the layout tree.
///
/// Input to [`LayoutModel::layout()`](crate::LayoutModel::layout).
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
        Rect {
            x: self.content.x - self.padding.left,
            y: self.content.y - self.padding.top,
            width: self.content.width + self.padding.left + self.padding.right,
            height: self.content.height + self.padding.top + self.padding.bottom,
        }
    }

    /// Returns the border box (content + padding + border).
    #[must_use]
    pub fn border_box(&self) -> Rect {
        let pb = self.padding_box();
        Rect {
            x: pb.x - self.border.left,
            y: pb.y - self.border.top,
            width: pb.width + self.border.left + self.border.right,
            height: pb.height + self.border.top + self.border.bottom,
        }
    }

    /// Returns the margin box (content + padding + border + margin).
    #[must_use]
    pub fn margin_box(&self) -> Rect {
        let bb = self.border_box();
        Rect {
            x: bb.x - self.margin.left,
            y: bb.y - self.margin.top,
            width: bb.width + self.margin.left + self.margin.right,
            height: bb.height + self.margin.top + self.margin.bottom,
        }
    }
}

/// Context available to a layout algorithm.
///
/// Passed to [`LayoutModel::layout()`](crate::LayoutModel::layout).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutContext {
    /// The viewport size.
    pub viewport: Size,
    /// The containing block size.
    pub containing_block: Size,
}

/// The result of a layout pass.
///
/// Returned by [`LayoutModel::layout()`](crate::LayoutModel::layout).
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
        let e = EdgeSizes::default();
        assert_eq!(e.top, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.bottom, 0.0);
        assert_eq!(e.left, 0.0);
    }

    #[test]
    fn layout_box_padding_box() {
        let b = LayoutBox {
            content: Rect {
                x: 20.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            },
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
            content: Rect {
                x: 25.0,
                y: 25.0,
                width: 100.0,
                height: 50.0,
            },
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
            content: Rect {
                x: 30.0,
                y: 30.0,
                width: 100.0,
                height: 50.0,
            },
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
        assert_eq!(mb.x, 10.0);  // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.y, 10.0);  // 30 - 10(pad) - 5(border) - 5(margin)
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
            content: Rect {
                x: 20.0,
                y: 10.0,
                width: 200.0,
                height: 100.0,
            },
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
