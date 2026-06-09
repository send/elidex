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
    /// Layout generation counter for per-fragment paged media rendering.
    ///
    /// Each page fragment is laid out with a unique generation value.
    /// During the render walk, entities whose generation doesn't match the
    /// current page are skipped. Default 0 (non-paged path — always visible).
    pub layout_generation: u32,
}

/// Per-line client rects for inline elements (CSSOM View §5 `getClientRects()`).
///
/// Stored as an ECS component on inline elements that span multiple lines.
/// Block elements and single-line inlines use `LayoutBox.border_box()` as a single rect.
#[derive(Clone, Debug)]
pub struct InlineClientRects(pub Vec<Rect>);

/// A box-model carrier: the four nested rectangles (content / padding / border /
/// margin) a paint pass reads to emit chrome and clips.
///
/// The unification linchpin for the render fragment-walk (terminal-Z C-1): both
/// [`LayoutBox`] (the per-entity, G11 last-column box) and `BoxFragment`
/// (`elidex-ecs`'s per-column fragment-store entry) implement it, so
/// the single chrome+clip emission loop is geometry-source-agnostic — the common
/// entity yields one [`LayoutBox`] item (N=1, byte-identical to the pre-C-1 path),
/// a multicol mid-break entity yields its N standalone-store `BoxFragment`s. The
/// box-derivation maths (`content.expand(...)`) lives here once as trait defaults;
/// implementors supply only the four raw edge sets.
pub trait BoxModel {
    /// Content area (absolute/document coords).
    fn content(&self) -> Rect;
    /// Padding widths.
    fn padding(&self) -> EdgeSizes;
    /// Border widths.
    fn border(&self) -> EdgeSizes;
    /// Margin widths.
    fn margin(&self) -> EdgeSizes;

    /// Padding box (content + padding).
    fn padding_box(&self) -> Rect {
        self.content().expand(self.padding())
    }

    /// Border box (content + padding + border).
    fn border_box(&self) -> Rect {
        self.padding_box().expand(self.border())
    }

    /// Margin box (content + padding + border + margin).
    ///
    /// Note: negative margins can produce a `Rect` with negative width or height.
    fn margin_box(&self) -> Rect {
        self.border_box().expand(self.margin())
    }
}

impl BoxModel for LayoutBox {
    fn content(&self) -> Rect {
        self.content
    }
    fn padding(&self) -> EdgeSizes {
        self.padding
    }
    fn border(&self) -> EdgeSizes {
        self.border
    }
    fn margin(&self) -> EdgeSizes {
        self.margin
    }
}

impl LayoutBox {
    /// Returns the padding box (content + padding).
    ///
    /// Inherent forwarder to [`BoxModel::padding_box`] so existing callers need not
    /// import the trait; the maths lives once in the trait default.
    #[must_use]
    pub fn padding_box(&self) -> Rect {
        BoxModel::padding_box(self)
    }

    /// Returns the border box (content + padding + border).
    ///
    /// Inherent forwarder to [`BoxModel::border_box`].
    #[must_use]
    pub fn border_box(&self) -> Rect {
        BoxModel::border_box(self)
    }

    /// Returns the margin box (content + padding + border + margin).
    ///
    /// Inherent forwarder to [`BoxModel::margin_box`]. Note: negative margins can
    /// produce a `Rect` with negative width or height.
    #[must_use]
    pub fn margin_box(&self) -> Rect {
        BoxModel::margin_box(self)
    }

    /// Returns the content rect in **element-local coordinates** —
    /// the coordinate system's origin is the border-box top-left; the
    /// content box starts at `(padding.left, padding.top)` (just
    /// inside the border, at the padding-box top-left) with size =
    /// content size.  Distinct from the public `content` field which
    /// is in document coordinates.
    ///
    /// Used by `ResizeObserverEntry.contentRect` (W3C Resize Observer §2.3)
    /// — the legacy field is defined relative to the element's own box.
    #[must_use]
    pub fn content_rect_local(&self) -> Rect {
        Rect::new(
            self.padding.left,
            self.padding.top,
            self.content.size.width,
            self.content.size.height,
        )
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
