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
    /// (`vertical-rl` has block progression right-to-left).
    #[must_use]
    pub fn is_block_reversed(&self) -> bool {
        matches!(self.writing_mode, WritingMode::VerticalRl)
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
    // TODO(Phase 4): For RTL direction, `inline_start` should be computed as
    // `rect.x + rect.width` (the right edge) in horizontal mode, or
    // `rect.y + rect.height` (the bottom edge) in vertical mode, so that
    // `inline_start` truly represents the inline-start position.
    // Currently this only swaps axes but does not flip for RTL.
    #[must_use]
    pub fn from_physical(rect: Rect, ctx: WritingModeContext) -> Self {
        if ctx.is_horizontal() {
            Self {
                inline_start: rect.x,
                block_start: rect.y,
                inline_size: rect.width,
                block_size: rect.height,
            }
        } else {
            Self {
                inline_start: rect.y,
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
            Rect {
                x: self.inline_start,
                y: self.block_start,
                width: self.inline_size,
                height: self.block_size,
            }
        } else {
            Rect {
                x: self.block_start,
                y: self.inline_start,
                width: self.block_size,
                height: self.inline_size,
            }
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
            // TODO(Phase 4): For vertical + RTL, inline_start should be
            // edges.bottom (bottom-to-top inline direction). Currently
            // inline_start is always edges.top regardless of direction.
            // The roundtrip test passes because from_physical/to_physical
            // have matching bugs that cancel out.
            let (block_start, block_end) = if ctx.is_block_reversed() {
                (edges.right, edges.left) // vertical-rl: block starts at right
            } else {
                (edges.left, edges.right) // vertical-lr: block starts at left
            };
            Self {
                inline_start: edges.top,
                inline_end: edges.bottom,
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
            EdgeSizes {
                top: self.inline_start,
                right,
                bottom: self.inline_end,
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
        let phys = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
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
        for ctx in [vertical_rl(), vertical_lr()] {
            let logical = LogicalEdges::from_physical(edges, ctx);
            assert_eq!(
                logical.to_physical(ctx),
                edges,
                "roundtrip failed for {ctx:?}"
            );
        }
    }
}
