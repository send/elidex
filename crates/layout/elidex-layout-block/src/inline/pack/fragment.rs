//! Trim all inline geometry to the committed lines of the current fragment.
use super::LinePacker;

impl LinePacker {
    /// The resumed suffix was packed at this fragment's origin (CSS Break 3 §5.1).
    /// Select by line ordinal: a zero-height line and its successor can share an offset.
    pub fn truncate_fragment(&mut self, effective_line_count: usize) {
        for lines in self.flow_lines.values_mut() {
            lines.truncate(lines.partition_point(|(ordinal, _)| *ordinal < effective_line_count));
        }
        self.flow_lines.retain(|_, lines| !lines.is_empty());

        // Per-entity bounds (→ inline LayoutBox + InlineClientRects): retain kept
        // per-line rects, recompute their union and drop
        // entities with no rect on a kept line.
        self.entity_bounds.retain(|_, b| {
            b.line_rects.retain(|r| r.ordinal < effective_line_count);
            if b.line_rects.is_empty() {
                return false;
            }
            // Recompute the union bounding box of the committed rects.
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

        self.static_positions
            .retain(|_, (ordinal, _)| *ordinal < effective_line_count);
        self.relpos_atomic_placements
            .retain(|(ordinal, ..)| *ordinal < effective_line_count);
    }
}
