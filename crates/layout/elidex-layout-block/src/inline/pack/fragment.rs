//! Fragmentation slice + continuation-rebase for [`LinePacker`](super::LinePacker).
//!
//! `slice_and_rebase_fragment` trims every recorded geometry to a paged/multicol
//! fragment's kept lines and rebases the block axis so the first kept line sits at the
//! fragmentainer's block-start (CSS Fragmentation L3 §2; slice 4 / I-paged). Split out
//! of `pack.rs` to keep the line packer under the repo's ~1000-line convention.

use super::LinePacker;

impl LinePacker {
    /// Slice every recorded geometry to this fragment's kept content lines
    /// `[skip_lines, effective_line_count)` and **continuation-rebase** the block
    /// axis so the first kept line sits at the fragmentainer's block-start (CSS
    /// Fragmentation L3 §2). Slice 4 / I-paged.
    ///
    /// **Single source of the rebase (F2).** Every block coordinate the packer
    /// records — flow-line `block_start`, per-entity rect `block_start` (→
    /// `getClientRects` + the inline `LayoutBox` via `assign_inline_layout_boxes`),
    /// abspos placeholder static positions, positioned-atomic placements — is a
    /// snapshot of the *same* `current_block_offset` accumulator, taken on the same
    /// content line. That accumulator IS the single SoT: the cumulative line-box
    /// offsets (`line_boxes` partial sums) are bit-identical to those snapshots
    /// (the same left-fold over the same per-line heights), so the block offset
    /// alone classifies a recording's line and the rebase delta is subtracted
    /// **once, here, before the caller's `content_origin` fold** (which adds the
    /// fragmentainer cursor exactly once). No consumer re-derives or re-applies —
    /// avoiding the cross-consumer desync four independent rebases would risk.
    ///
    /// `first_baseline` is intentionally *not* rebased: it is the parent-block
    /// baseline scalar (cross-block alignment), captured from the first content
    /// line — correct for the first fragment (delta 0) and an orthogonal
    /// approximation for continuations, outside this line-geometry rebase's scope.
    pub fn slice_and_rebase_fragment(&mut self, skip_lines: usize, effective_line_count: usize) {
        // Block-offset boundaries: `rebase_delta` = offset of the first kept line
        // (== that line's recorded snapshot, exactly); `break_offset` = offset of
        // the first dropped line (== `total` when nothing breaks).
        let rebase_delta: f32 = self
            .line_boxes
            .iter()
            .take(skip_lines)
            .map(|lb| lb.block_size)
            .sum();
        let break_offset: f32 = self
            .line_boxes
            .iter()
            .take(effective_line_count)
            .map(|lb| lb.block_size)
            .sum();
        // A recording on block offset `b` is kept iff its line is in the fragment.
        // Boundaries are exact (same accumulation), so the half-open range needs no
        // epsilon: the first kept line's `b == rebase_delta`, the first dropped
        // line's `b == break_offset`.
        let keep = |b: f32| b >= rebase_delta && b < break_offset;

        // Flow lines (the persisted InlineFragment lines): retain kept, rebase,
        // drop now-empty group buckets.
        for lines in self.flow_lines.values_mut() {
            lines.retain(|l| keep(l.block_start));
            for l in lines.iter_mut() {
                l.block_start -= rebase_delta;
            }
        }
        self.flow_lines.retain(|_, lines| !lines.is_empty());

        // Per-entity bounds (→ inline LayoutBox + InlineClientRects): retain kept
        // per-line rects, rebase them, recompute the union from what's left; drop
        // entities with no rect on a kept line.
        self.entity_bounds.retain(|_, b| {
            b.line_rects.retain(|r| keep(r.block_start));
            if b.line_rects.is_empty() {
                return false;
            }
            // Rebase the kept rects and recompute the union bounding box in one pass.
            let (mut inline_start, mut inline_end) = (f32::INFINITY, f32::NEG_INFINITY);
            let (mut block_start, mut block_end) = (f32::INFINITY, f32::NEG_INFINITY);
            for r in &mut b.line_rects {
                r.block_start -= rebase_delta;
                inline_start = inline_start.min(r.inline_start);
                inline_end = inline_end.max(r.inline_end);
                block_start = block_start.min(r.block_start);
                block_end = block_end.max(r.block_start + r.block_size);
            }
            b.inline_start = inline_start;
            b.inline_end = inline_end;
            b.block_start = block_start;
            b.block_end = block_end;
            true
        });

        // Abspos placeholder static positions (`Point(inline, block)`): rebase kept,
        // drop those whose line went to another fragment.
        self.static_positions.retain(|_, p| {
            if keep(p.y) {
                p.y -= rebase_delta;
                true
            } else {
                false
            }
        });

        // Positioned-atomic placements `(entity, inline_local, block_local)`: same.
        self.relpos_atomic_placements
            .retain_mut(|(_, _, block_local)| {
                if keep(*block_local) {
                    *block_local -= rebase_delta;
                    true
                } else {
                    false
                }
            });
    }
}
