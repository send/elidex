//! Logical coordinate types for writing-mode-aware layout.
//!
//! Provides types that abstract over physical coordinates (x/y, width/height)
//! using flow-relative terms (inline/block). These types support conversion
//! to/from physical coordinates via [`WritingModeContext`].

use crate::computed_style::{Direction, WritingMode};
use crate::layout_types::{EdgeSizes, Rect, Size};

/// Context for converting between logical and physical coordinates.
///
/// Combines `writing-mode` and `direction` to determine axis mapping:
/// - `horizontal-tb`: inline=horizontal, block=vertical (default)
/// - `vertical-rl`: inline=vertical, block=horizontal (right-to-left)
/// - `vertical-lr`: inline=vertical, block=horizontal (left-to-right)
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WritingModeContext {
    /// The writing mode.
    pub writing_mode: WritingMode,
    /// The inline base direction.
    pub direction: Direction,
}

impl WritingModeContext {
    /// Create a new writing mode context.
    #[must_use]
    pub fn new(writing_mode: WritingMode, direction: Direction) -> Self {
        Self {
            writing_mode,
            direction,
        }
    }

    /// Returns `true` if the inline axis is horizontal.
    #[must_use]
    pub fn is_horizontal(&self) -> bool {
        matches!(self.writing_mode, WritingMode::HorizontalTb)
    }

    /// Returns `true` if the inline direction is reversed (RTL in horizontal,
    /// bottom-to-top never occurs in CSS).
    #[must_use]
    pub fn is_inline_reversed(&self) -> bool {
        matches!(self.direction, Direction::Rtl)
    }

    /// Returns `true` if the block direction is reversed
    /// (`vertical-rl` and `sideways-rl` have block progression right-to-left).
    #[must_use]
    pub fn is_block_reversed(&self) -> bool {
        matches!(
            self.writing_mode,
            WritingMode::VerticalRl | WritingMode::SidewaysRl
        )
    }
}

/// A size in flow-relative dimensions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalSize {
    /// Size along the inline axis.
    pub inline: f32,
    /// Size along the block axis.
    pub block: f32,
}

impl LogicalSize {
    /// Convert a physical `Size` to logical using the given writing context.
    #[must_use]
    pub fn from_physical(size: Size, ctx: WritingModeContext) -> Self {
        if ctx.is_horizontal() {
            Self {
                inline: size.width,
                block: size.height,
            }
        } else {
            Self {
                inline: size.height,
                block: size.width,
            }
        }
    }

    /// Convert to a physical `Size`.
    #[must_use]
    pub fn to_physical(self, ctx: WritingModeContext) -> Size {
        if ctx.is_horizontal() {
            Size {
                width: self.inline,
                height: self.block,
            }
        } else {
            Size {
                width: self.block,
                height: self.inline,
            }
        }
    }
}

/// A rectangle in flow-relative coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalRect {
    /// Start position along the inline axis.
    pub inline_start: f32,
    /// Start position along the block axis.
    pub block_start: f32,
    /// Size along the inline axis.
    pub inline_size: f32,
    /// Size along the block axis.
    pub block_size: f32,
}

impl LogicalRect {
    /// Convert a physical `Rect` to logical coordinates.
    ///
    /// For RTL direction, `inline_start` is the inline-end physical edge
    /// (right edge in horizontal, bottom edge in vertical) so that it
    /// represents the true inline-start position.
    #[must_use]
    pub fn from_physical(rect: Rect, ctx: WritingModeContext) -> Self {
        if ctx.is_horizontal() {
            let inline_start = if ctx.is_inline_reversed() {
                rect.x + rect.width // RTL: inline-start is right edge
            } else {
                rect.x
            };
            Self {
                inline_start,
                block_start: rect.y,
                inline_size: rect.width,
                block_size: rect.height,
            }
        } else {
            let inline_start = if ctx.is_inline_reversed() {
                rect.y + rect.height // vertical RTL: inline-start is bottom edge
            } else {
                rect.y
            };
            Self {
                inline_start,
                block_start: rect.x,
                inline_size: rect.height,
                block_size: rect.width,
            }
        }
    }

    /// Convert to a physical `Rect`.
    #[must_use]
    pub fn to_physical(self, ctx: WritingModeContext) -> Rect {
        if ctx.is_horizontal() {
            let x = if ctx.is_inline_reversed() {
                self.inline_start - self.inline_size // RTL: left edge = right edge - width
            } else {
                self.inline_start
            };
            Rect::new(x, self.block_start, self.inline_size, self.block_size)
        } else {
            let y = if ctx.is_inline_reversed() {
                self.inline_start - self.inline_size // vertical RTL: top = bottom - height
            } else {
                self.inline_start
            };
            Rect::new(self.block_start, y, self.block_size, self.inline_size)
        }
    }
}

/// Edge sizes in flow-relative terms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalEdges {
    /// Edge at the inline-start side.
    pub inline_start: f32,
    /// Edge at the inline-end side.
    pub inline_end: f32,
    /// Edge at the block-start side.
    pub block_start: f32,
    /// Edge at the block-end side.
    pub block_end: f32,
}

impl LogicalEdges {
    /// Convert physical `EdgeSizes` to logical edges.
    #[must_use]
    pub fn from_physical(edges: EdgeSizes, ctx: WritingModeContext) -> Self {
        if ctx.is_horizontal() {
            // horizontal-tb: inline = left/right, block = top/bottom
            if ctx.is_inline_reversed() {
                Self {
                    inline_start: edges.right,
                    inline_end: edges.left,
                    block_start: edges.top,
                    block_end: edges.bottom,
                }
            } else {
                Self {
                    inline_start: edges.left,
                    inline_end: edges.right,
                    block_start: edges.top,
                    block_end: edges.bottom,
                }
            }
        } else {
            // vertical: inline = top/bottom, block = left/right
            let (inline_start, inline_end) = if ctx.is_inline_reversed() {
                (edges.bottom, edges.top) // RTL: inline-start is bottom
            } else {
                (edges.top, edges.bottom)
            };
            let (block_start, block_end) = if ctx.is_block_reversed() {
                (edges.right, edges.left) // vertical-rl: block starts at right
            } else {
                (edges.left, edges.right) // vertical-lr: block starts at left
            };
            Self {
                inline_start,
                inline_end,
                block_start,
                block_end,
            }
        }
    }

    /// Convert to physical `EdgeSizes`.
    #[must_use]
    pub fn to_physical(self, ctx: WritingModeContext) -> EdgeSizes {
        if ctx.is_horizontal() {
            if ctx.is_inline_reversed() {
                EdgeSizes {
                    top: self.block_start,
                    right: self.inline_start,
                    bottom: self.block_end,
                    left: self.inline_end,
                }
            } else {
                EdgeSizes {
                    top: self.block_start,
                    right: self.inline_end,
                    bottom: self.block_end,
                    left: self.inline_start,
                }
            }
        } else {
            let (left, right) = if ctx.is_block_reversed() {
                (self.block_end, self.block_start)
            } else {
                (self.block_start, self.block_end)
            };
            let (top, bottom) = if ctx.is_inline_reversed() {
                (self.inline_end, self.inline_start) // RTL: top = inline-end
            } else {
                (self.inline_start, self.inline_end)
            };
            EdgeSizes {
                top,
                right,
                bottom,
                left,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ltr_horizontal() -> WritingModeContext {
        WritingModeContext::default()
    }

    fn rtl_horizontal() -> WritingModeContext {
        WritingModeContext {
            writing_mode: WritingMode::HorizontalTb,
            direction: Direction::Rtl,
        }
    }

    fn vertical_rl() -> WritingModeContext {
        WritingModeContext {
            writing_mode: WritingMode::VerticalRl,
            direction: Direction::Ltr,
        }
    }

    fn vertical_lr() -> WritingModeContext {
        WritingModeContext {
            writing_mode: WritingMode::VerticalLr,
            direction: Direction::Ltr,
        }
    }

    fn vertical_rl_rtl() -> WritingModeContext {
        WritingModeContext {
            writing_mode: WritingMode::VerticalRl,
            direction: Direction::Rtl,
        }
    }

    #[test]
    fn writing_mode_context_flags() {
        assert!(ltr_horizontal().is_horizontal());
        assert!(!ltr_horizontal().is_inline_reversed());
        assert!(!ltr_horizontal().is_block_reversed());

        assert!(rtl_horizontal().is_horizontal());
        assert!(rtl_horizontal().is_inline_reversed());

        assert!(!vertical_rl().is_horizontal());
        assert!(vertical_rl().is_block_reversed());

        assert!(!vertical_lr().is_horizontal());
        assert!(!vertical_lr().is_block_reversed());
    }

    #[test]
    fn logical_size_roundtrip_horizontal() {
        let phys = Size {
            width: 100.0,
            height: 50.0,
        };
        let ctx = ltr_horizontal();
        let logical = LogicalSize::from_physical(phys, ctx);
        assert_eq!(logical.inline, 100.0);
        assert_eq!(logical.block, 50.0);
        assert_eq!(logical.to_physical(ctx), phys);
    }

    #[test]
    fn logical_size_roundtrip_vertical() {
        let phys = Size {
            width: 100.0,
            height: 50.0,
        };
        let ctx = vertical_rl();
        let logical = LogicalSize::from_physical(phys, ctx);
        assert_eq!(logical.inline, 50.0); // height → inline
        assert_eq!(logical.block, 100.0); // width → block
        assert_eq!(logical.to_physical(ctx), phys);
    }

    #[test]
    fn logical_rect_roundtrip() {
        let phys = Rect::new(10.0, 20.0, 100.0, 50.0);
        for ctx in [
            ltr_horizontal(),
            rtl_horizontal(),
            vertical_rl(),
            vertical_lr(),
        ] {
            let logical = LogicalRect::from_physical(phys, ctx);
            assert_eq!(
                logical.to_physical(ctx),
                phys,
                "roundtrip failed for {ctx:?}"
            );
        }
    }

    #[test]
    fn logical_edges_ltr_horizontal() {
        let edges = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);
        let ctx = ltr_horizontal();
        let logical = LogicalEdges::from_physical(edges, ctx);
        assert_eq!(logical.inline_start, 4.0); // left
        assert_eq!(logical.inline_end, 2.0); // right
        assert_eq!(logical.block_start, 1.0); // top
        assert_eq!(logical.block_end, 3.0); // bottom
        assert_eq!(logical.to_physical(ctx), edges);
    }

    #[test]
    fn logical_edges_rtl_horizontal() {
        let edges = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);
        let ctx = rtl_horizontal();
        let logical = LogicalEdges::from_physical(edges, ctx);
        // RTL: inline-start = right, inline-end = left
        assert_eq!(logical.inline_start, 2.0);
        assert_eq!(logical.inline_end, 4.0);
        assert_eq!(logical.to_physical(ctx), edges);
    }

    #[test]
    fn logical_edges_vertical_roundtrip() {
        let edges = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);
        for ctx in [vertical_rl(), vertical_lr(), vertical_rl_rtl()] {
            let logical = LogicalEdges::from_physical(edges, ctx);
            assert_eq!(
                logical.to_physical(ctx),
                edges,
                "roundtrip failed for {ctx:?}"
            );
        }
    }

    #[test]
    fn logical_rect_rtl_horizontal_inline_start() {
        let phys = Rect::new(10.0, 20.0, 100.0, 50.0);
        let ctx = rtl_horizontal();
        let logical = LogicalRect::from_physical(phys, ctx);
        // RTL: inline_start = right edge = 10 + 100 = 110
        assert_eq!(logical.inline_start, 110.0);
        assert_eq!(logical.inline_size, 100.0);
        assert_eq!(logical.to_physical(ctx), phys);
    }

    #[test]
    fn logical_rect_vertical_rtl_inline_start() {
        let phys = Rect::new(10.0, 20.0, 100.0, 50.0);
        let ctx = vertical_rl_rtl();
        let logical = LogicalRect::from_physical(phys, ctx);
        // Vertical RTL: inline_start = bottom edge = 20 + 50 = 70
        assert_eq!(logical.inline_start, 70.0);
        assert_eq!(logical.inline_size, 50.0);
        assert_eq!(logical.to_physical(ctx), phys);
    }

    #[test]
    fn logical_edges_vertical_rtl_inline_swap() {
        let edges = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);
        let ctx = vertical_rl_rtl();
        let logical = LogicalEdges::from_physical(edges, ctx);
        // Vertical RTL: inline_start = bottom (3.0), inline_end = top (1.0)
        assert_eq!(logical.inline_start, 3.0);
        assert_eq!(logical.inline_end, 1.0);
        assert_eq!(logical.to_physical(ctx), edges);
    }
}
