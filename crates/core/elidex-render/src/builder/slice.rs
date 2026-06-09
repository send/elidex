//! `box-decoration-break: slice` per-fragment paint geometry (css-break-3 §5.4 /
//! §5.4.1) for the render fragment-walk (terminal-Z C-1).
//!
//! These pure helpers turn a fragment index + the fragmentation-context writing
//! mode into the per-fragment slice adjustments the paint walk applies: which border
//! edges are *at a break* (omitted), the per-fragment [`SlicedBox`] geometry (broken
//! edges' border + padding zeroed), the square-the-broken-corners radius transform,
//! and the cumulative background-position offset that keeps a sliced background's
//! tiling phase continuous across column breaks. The single paint walk
//! ([`super::walk`]) calls them; keeping them here keeps the central walk under the
//! 1000-line file budget.

use elidex_plugin::{BoxModel, EdgeSizes, Rect, Vector, WritingMode};

/// Physical `[top, right, bottom, left]` "this edge is at a fragmentation break"
/// flags for fragment `i` of `n` along the writing-mode block axis (css-break-3
/// §5.4: no border/padding is inserted *at a break*). A continuation fragment
/// (`i > 0`) breaks at its block-start; a fragment that continues (`i < n - 1`)
/// breaks at its block-end. The block axis maps to physical edges by writing mode;
/// the inline-axis edges are never "at a break" (they paint on every fragment).
/// `n <= 1` ⇒ no breaks (single box).
pub(super) fn break_edges(i: usize, n: usize, wm: WritingMode) -> [bool; 4] {
    if n <= 1 {
        return [false; 4];
    }
    let at_block_start = i > 0;
    let at_block_end = i < n - 1;
    // (block-start, block-end) physical indices into [top, right, bottom, left].
    let (start_idx, end_idx) = match wm {
        WritingMode::HorizontalTb => (0, 2), // top / bottom
        WritingMode::VerticalRl | WritingMode::SidewaysRl => (1, 3), // right / left
        WritingMode::VerticalLr | WritingMode::SidewaysLr => (3, 1), // left / right
    };
    let mut e = [false; 4];
    e[start_idx] = at_block_start;
    e[end_idx] = at_block_end;
    e
}

/// A box with the fragmentation-break edges' padding **and** border zeroed
/// (css-break-3 §5.4: no border or padding is inserted at a break, so adjacent
/// fragments' content abuts). The border is also painted-omitted via the `emit_*`
/// `omit_edges`; this zeroing makes the geometry (border/padding boxes used for the
/// bg fill area and the overflow clip) match — and gives the rounded-border ring a
/// zero-width edge at the break, so the ring naturally omits the break border while
/// keeping the unbroken corners rounded. `omit` is the physical
/// `[top, right, bottom, left]` break-edge mask.
pub(super) struct SlicedBox {
    content: Rect,
    padding: EdgeSizes,
    border: EdgeSizes,
    margin: EdgeSizes,
}

impl BoxModel for SlicedBox {
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

/// Build the [`SlicedBox`] for a fragment: zero the padding + border on each broken
/// edge (css-break-3 §5.4). The non-broken (outer) edges keep their real values.
///
/// Because the source `content` rect is positioned **inside** the box's border +
/// padding, zeroing a break edge's decoration without compensating would shrink the
/// box to the content — leaving a decoration-sized gap and mis-clipping a
/// continuation column. So the content is **expanded** to absorb the removed
/// decoration: the box edge stays at the break edge (§5.4: the content abuts the
/// cut, no border/padding there) instead of moving inward by the decoration extent.
pub(super) fn sliced_box(frag: &dyn BoxModel, omit: [bool; 4]) -> SlicedBox {
    let z = |v: f32, o: bool| if o { 0.0 } else { v };
    let (p, b) = (frag.padding(), frag.border());
    let c = frag.content();
    // Decoration (border + padding) removed on each edge that is at a break.
    let rt = if omit[0] { b.top + p.top } else { 0.0 };
    let rr = if omit[1] { b.right + p.right } else { 0.0 };
    let rb = if omit[2] { b.bottom + p.bottom } else { 0.0 };
    let rl = if omit[3] { b.left + p.left } else { 0.0 };
    // Grow the content into the removed decoration; start edges (top/left) also move
    // the origin out, so the box's border box stays at the original break-edge line.
    let content = Rect::new(
        c.origin.x - rl,
        c.origin.y - rt,
        c.size.width + rl + rr,
        c.size.height + rt + rb,
    );
    SlicedBox {
        content,
        padding: EdgeSizes::new(
            z(p.top, omit[0]),
            z(p.right, omit[1]),
            z(p.bottom, omit[2]),
            z(p.left, omit[3]),
        ),
        border: EdgeSizes::new(
            z(b.top, omit[0]),
            z(b.right, omit[1]),
            z(b.bottom, omit[2]),
            z(b.left, omit[3]),
        ),
        margin: frag.margin(),
    }
}

/// Square the `border-radius` corners adjacent to a fragmentation break (css-break-3
/// §5.4: `slice` applies border-radius to the unbroken whole box, so only its real
/// outer corners are rounded — internal break corners are square). `radii` is
/// `[top-left, top-right, bottom-right, bottom-left]`; `omit` is `[top, right,
/// bottom, left]`. A corner is squared iff either of its two adjacent edges broke.
pub(super) fn square_broken_corners(radii: [f32; 4], omit: [bool; 4]) -> [f32; 4] {
    let [top, right, bottom, left] = omit;
    [
        if top || left { 0.0 } else { radii[0] },     // top-left
        if top || right { 0.0 } else { radii[1] },    // top-right
        if bottom || right { 0.0 } else { radii[2] }, // bottom-right
        if bottom || left { 0.0 } else { radii[3] },  // bottom-left
    ]
}

/// Block-axis extent of a box under writing mode `wm` (height for a horizontal
/// mode, width for a vertical mode) — accumulated across fragments (over their
/// padding boxes, the bg painting area) to offset the slice background-position so
/// its tiling phase stays continuous.
pub(super) fn block_axis_extent(box_rect: Rect, wm: WritingMode) -> f32 {
    if wm.is_horizontal() {
        box_rect.size.height
    } else {
        box_rect.size.width
    }
}

/// Background-position shift for fragment `i`'s `box-decoration-break: slice`
/// painting (css-break-3 §5.4.1): `-cumulative_block_extent` projected onto the
/// physical block-flow direction, so the image is painted as if on the unbroken
/// composite box. Horizontal-tb flows down (−y); vertical-rl/sideways-rl flow left
/// (+x toward the composite origin on the right); vertical-lr/sideways-lr flow right
/// (−x).
pub(super) fn slice_bg_offset(wm: WritingMode, cum_block: f32) -> Vector {
    match wm {
        WritingMode::HorizontalTb => Vector::new(0.0, -cum_block),
        WritingMode::VerticalRl | WritingMode::SidewaysRl => Vector::new(cum_block, 0.0),
        WritingMode::VerticalLr | WritingMode::SidewaysLr => Vector::new(-cum_block, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        block_axis_extent, break_edges, slice_bg_offset, sliced_box, square_broken_corners,
    };
    use elidex_plugin::{BoxModel, EdgeSizes, Rect, Vector, WritingMode};

    #[test]
    fn break_edges_single_box_has_no_breaks() {
        assert_eq!(break_edges(0, 1, WritingMode::HorizontalTb), [false; 4]);
    }

    #[test]
    fn sliced_box_zeros_padding_and_border_on_broken_edges_only() {
        // A middle fragment (top+bottom broken, horizontal-tb). The block-axis
        // (top/bottom) padding + border are removed; the inline (left/right) keep theirs.
        let base = TestBox {
            content: Rect::new(0.0, 0.0, 100.0, 40.0),
            padding: EdgeSizes::new(5.0, 6.0, 7.0, 8.0),
            border: EdgeSizes::new(1.0, 2.0, 3.0, 4.0),
            margin: EdgeSizes::default(),
        };
        // The ORIGINAL break-edge lines (border-box top/bottom) before slicing.
        let orig_top = base.border_box().origin.y; // 0 - 5 - 1 = -6
        let orig_bottom = base.border_box().origin.y + base.border_box().size.height; // +50
        let s = sliced_box(&base, [true, false, true, false]);
        assert_eq!(
            (s.padding().top, s.padding().bottom),
            (0.0, 0.0),
            "broken block-axis padding removed (§5.4: no padding at a break)"
        );
        assert_eq!(
            (s.padding().left, s.padding().right),
            (8.0, 6.0),
            "inline padding preserved (not at a break)"
        );
        assert_eq!(
            (s.border().top, s.border().bottom),
            (0.0, 0.0),
            "broken block-axis border removed"
        );
        assert_eq!((s.border().left, s.border().right), (4.0, 2.0));
        // R3-F1: the content is EXPANDED into the removed decoration so the box edge
        // stays at the original break-edge line (no decoration-sized gap / mis-clip).
        let sbb = s.border_box();
        assert_eq!(
            (sbb.origin.y, sbb.origin.y + sbb.size.height),
            (orig_top, orig_bottom),
            "the sliced border box still meets the break edges (content absorbed the \
             removed border+padding) — not shrunk inward to the content"
        );
        // The inline (unbroken) edges keep their decoration ⇒ box width unchanged.
        assert_eq!(sbb.origin.x, base.border_box().origin.x);
        assert_eq!(sbb.size.width, base.border_box().size.width);
    }

    #[test]
    fn square_broken_corners_squares_corners_touching_a_break() {
        // top broken ⇒ both top corners square; bottom unbroken ⇒ stay rounded.
        assert_eq!(
            square_broken_corners([5.0, 6.0, 7.0, 8.0], [true, false, false, false]),
            [0.0, 0.0, 7.0, 8.0]
        );
        // top+bottom broken (a middle fragment) ⇒ all four corners square.
        assert_eq!(
            square_broken_corners([5.0, 6.0, 7.0, 8.0], [true, false, true, false]),
            [0.0; 4]
        );
        // no break ⇒ radii unchanged.
        assert_eq!(
            square_broken_corners([5.0, 6.0, 7.0, 8.0], [false; 4]),
            [5.0, 6.0, 7.0, 8.0]
        );
    }

    /// A minimal [`BoxModel`] for the sliced-geometry unit tests.
    struct TestBox {
        content: Rect,
        padding: EdgeSizes,
        border: EdgeSizes,
        margin: EdgeSizes,
    }
    impl BoxModel for TestBox {
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

    #[test]
    fn break_edges_horizontal_omits_block_axis_at_breaks() {
        // [top, right, bottom, left]. horizontal-tb: block axis = top/bottom.
        // First fragment of 3: breaks at its block-END only (bottom).
        assert_eq!(
            break_edges(0, 3, WritingMode::HorizontalTb),
            [false, false, true, false]
        );
        // Middle: breaks at both block-START (top) and block-END (bottom).
        assert_eq!(
            break_edges(1, 3, WritingMode::HorizontalTb),
            [true, false, true, false]
        );
        // Last: breaks at its block-START only (top). Inline edges never break.
        assert_eq!(
            break_edges(2, 3, WritingMode::HorizontalTb),
            [true, false, false, false]
        );
    }

    #[test]
    fn break_edges_vertical_maps_block_axis_to_inline_physical_edges() {
        // vertical-rl: block-start = right, block-end = left.
        // First of 2 breaks at block-END (left).
        assert_eq!(
            break_edges(0, 2, WritingMode::VerticalRl),
            [false, false, false, true]
        );
        // Last of 2 breaks at block-START (right).
        assert_eq!(
            break_edges(1, 2, WritingMode::VerticalRl),
            [false, true, false, false]
        );
        // vertical-lr: block-start = left, block-end = right (mirror of -rl).
        assert_eq!(
            break_edges(0, 2, WritingMode::VerticalLr),
            [false, true, false, false]
        );
    }

    #[test]
    fn slice_bg_offset_projects_negative_block_flow_per_mode() {
        // horizontal flows down ⇒ shift up (−y).
        assert_eq!(
            slice_bg_offset(WritingMode::HorizontalTb, 40.0),
            Vector::new(0.0, -40.0)
        );
        // vertical-rl flows left, composite origin on the right ⇒ +x.
        assert_eq!(
            slice_bg_offset(WritingMode::VerticalRl, 40.0),
            Vector::new(40.0, 0.0)
        );
        // vertical-lr flows right ⇒ −x.
        assert_eq!(
            slice_bg_offset(WritingMode::VerticalLr, 40.0),
            Vector::new(-40.0, 0.0)
        );
    }

    #[test]
    fn block_axis_extent_is_height_horizontal_width_vertical() {
        let bb = Rect::new(0.0, 0.0, 100.0, 40.0);
        assert_eq!(block_axis_extent(bb, WritingMode::HorizontalTb), 40.0);
        assert_eq!(block_axis_extent(bb, WritingMode::VerticalRl), 100.0);
    }
}
