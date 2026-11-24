//! Trim all inline geometry to the committed lines of the current fragment.
use super::LinePacker;

impl LinePacker {
    /// The resumed suffix was packed at this fragment's origin (CSS Break 3 §5.1).
    /// Every recorded block coordinate comes from the same line-height accumulator.
    pub fn truncate_fragment(&mut self, effective_line_count: usize) {
        let break_offset: f32 = self
            .line_boxes
            .iter()
            .take(effective_line_count)
            .map(|lb| lb.block_size)
            .sum();
        let keep = |block_start: f32| block_start < break_offset;
        for lines in self.flow_lines.values_mut() {
            lines.retain(|line| keep(line.block_start));
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
            for r in &b.line_rects {
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

        self.static_positions.retain(|_, position| keep(position.y));
        self.relpos_atomic_placements
            .retain(|(_, _, block_local)| keep(*block_local));
    }
}
