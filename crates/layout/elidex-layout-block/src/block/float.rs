//! CSS float layout context (CSS 2.1 §9.5).
//!
//! Tracks placed floats (left/right) and provides placement, clearance,
//! and available-width queries for content flowing around floats.

use elidex_plugin::{Clear, Float};

/// A placed float with its position and dimensions.
#[derive(Clone, Debug)]
struct PlacedFloat {
    /// Left edge of the float's margin box.
    x: f32,
    /// Top edge of the float's margin box.
    y: f32,
    /// Width of the float's margin box.
    width: f32,
    /// Height of the float's margin box.
    height: f32,
}

impl PlacedFloat {
    /// Bottom edge of the float's margin box.
    fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Returns `true` if this float overlaps the vertical span `[y, y+height)`.
    fn overlaps_span(&self, y: f32, height: f32) -> bool {
        self.bottom() > y && self.y < y + height
    }
}

/// Tracks placed floats within a block formatting context.
#[derive(Clone, Debug, Default)]
pub struct FloatContext {
    /// Left-floated elements.
    left_floats: Vec<PlacedFloat>,
    /// Right-floated elements.
    right_floats: Vec<PlacedFloat>,
    /// Width of the containing block.
    containing_width: f32,
}

impl FloatContext {
    /// Create a new float context for the given containing block width.
    pub fn new(containing_width: f32) -> Self {
        Self {
            left_floats: Vec::new(),
            right_floats: Vec::new(),
            containing_width,
        }
    }

    /// Returns `true` if there are any active floats.
    #[cfg(test)]
    fn has_floats(&self) -> bool {
        !self.left_floats.is_empty() || !self.right_floats.is_empty()
    }

    /// Place a float and return its margin-box (x, y) position.
    ///
    /// The float is placed at or below `cursor_y`, on the left or right edge,
    /// avoiding overlap with existing floats. Coordinates are relative to the
    /// containing block's content edge.
    pub fn place_float(
        &mut self,
        float_side: Float,
        margin_box_width: f32,
        margin_box_height: f32,
        cursor_y: f32,
    ) -> (f32, f32) {
        // Find the lowest Y at which the float can fit without overlapping
        // existing floats on the same side.
        let mut y = cursor_y;

        // Drop below floats until the new float fits horizontally.
        loop {
            let (left_used, right_used) = self.used_width_at(y, margin_box_height);
            let available = self.containing_width - left_used - right_used;

            if margin_box_width <= available {
                break;
            }

            // Float doesn't fit — drop below the shallowest float at this Y.
            let next_y = self.next_clear_y(y, margin_box_height);
            if next_y <= y {
                break; // No more floats to clear
            }
            y = next_y;
        }

        let (left_used, right_used) = self.used_width_at(y, margin_box_height);

        let x = match float_side {
            Float::Left => left_used,
            // CSS 2.1 §9.5.1 rule 3: right float's right edge must not be to
            // the right of the containing block, and must not overlap existing
            // right floats. Also must not be to the left of any left float.
            Float::Right => (self.containing_width - right_used - margin_box_width).max(left_used),
            Float::None => return (0.0, y), // shouldn't happen
        };

        let placed = PlacedFloat {
            x,
            y,
            width: margin_box_width,
            height: margin_box_height,
        };

        match float_side {
            Float::Left => self.left_floats.push(placed),
            Float::Right => self.right_floats.push(placed),
            Float::None => {}
        }

        (x, y)
    }

    /// Compute the Y position needed to clear past floats.
    ///
    /// Returns the Y below the relevant floats based on the `clear` value.
    pub fn clear_y(&self, clear: Clear, cursor_y: f32) -> f32 {
        let max_bottom = |floats: &[PlacedFloat]| {
            floats
                .iter()
                .map(PlacedFloat::bottom)
                .fold(cursor_y, f32::max)
        };
        match clear {
            Clear::None => cursor_y,
            Clear::Left => max_bottom(&self.left_floats),
            Clear::Right => max_bottom(&self.right_floats),
            Clear::Both => max_bottom(&self.left_floats).max(max_bottom(&self.right_floats)),
        }
    }

    /// Get the total horizontal space used by floats at vertical position `y`
    /// over a span of `height`.
    ///
    /// Returns `(left_used, right_used)`.
    fn used_width_at(&self, y: f32, height: f32) -> (f32, f32) {
        let left_used = self
            .left_floats
            .iter()
            .filter(|f| f.overlaps_span(y, height))
            .map(|f| f.x + f.width)
            .fold(0.0_f32, f32::max);

        let right_used = self
            .right_floats
            .iter()
            .filter(|f| f.overlaps_span(y, height))
            .map(|f| self.containing_width - f.x)
            .fold(0.0_f32, f32::max);

        (left_used, right_used)
    }

    /// Find the next Y coordinate where a float clears (lowest bottom of
    /// the shallowest float that overlaps the vertical span `[y, y+height)`).
    ///
    /// Uses the same overlap condition as `used_width_at()` so that
    /// `place_float()` correctly drops below floats that start within
    /// the new float's height span, not just at or above `y`.
    fn next_clear_y(&self, y: f32, height: f32) -> f32 {
        let mut min_bottom = f32::MAX;
        for f in self.left_floats.iter().chain(self.right_floats.iter()) {
            if f.overlaps_span(y, height) {
                min_bottom = min_bottom.min(f.bottom());
            }
        }
        if min_bottom == f32::MAX {
            y
        } else {
            min_bottom
        }
    }

    /// Get the bottom edge of all floats (for containing block height).
    pub fn float_bottom(&self) -> f32 {
        self.clear_y(Clear::Both, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context() {
        let ctx = FloatContext::new(800.0);
        assert!(!ctx.has_floats());
        assert_eq!(ctx.float_bottom(), 0.0);
    }

    #[test]
    fn place_left_float() {
        let mut ctx = FloatContext::new(800.0);
        let (x, y) = ctx.place_float(Float::Left, 200.0, 100.0, 0.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
        assert!(ctx.has_floats());
        assert_eq!(ctx.float_bottom(), 100.0);
    }

    #[test]
    fn place_right_float() {
        let mut ctx = FloatContext::new(800.0);
        let (x, y) = ctx.place_float(Float::Right, 200.0, 100.0, 0.0);
        assert_eq!(x, 600.0); // 800 - 200
        assert_eq!(y, 0.0);
    }

    #[test]
    fn left_floats_stack_horizontally() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(Float::Left, 200.0, 100.0, 0.0);
        let (x, y) = ctx.place_float(Float::Left, 200.0, 100.0, 0.0);
        assert_eq!(x, 200.0); // next to the first float
        assert_eq!(y, 0.0);
    }

    #[test]
    fn right_floats_stack_horizontally() {
        // CSS 2.1 §9.5.1 rule 3: right floats must not overlap.
        let mut ctx = FloatContext::new(800.0);
        let (x1, _) = ctx.place_float(Float::Right, 200.0, 100.0, 0.0);
        assert_eq!(x1, 600.0); // 800 - 200
        let (x2, _) = ctx.place_float(Float::Right, 200.0, 100.0, 0.0);
        assert_eq!(x2, 400.0); // 800 - 200 - 200
    }

    #[test]
    fn clear_left() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(Float::Left, 200.0, 100.0, 0.0);
        let y = ctx.clear_y(Clear::Left, 0.0);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn clear_right() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(Float::Right, 200.0, 150.0, 0.0);
        let y = ctx.clear_y(Clear::Right, 0.0);
        assert_eq!(y, 150.0);
    }

    #[test]
    fn clear_both() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(Float::Left, 200.0, 100.0, 0.0);
        ctx.place_float(Float::Right, 200.0, 150.0, 0.0);
        let y = ctx.clear_y(Clear::Both, 0.0);
        assert_eq!(y, 150.0);
    }

    #[test]
    fn clear_none_unchanged() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(Float::Left, 200.0, 100.0, 0.0);
        let y = ctx.clear_y(Clear::None, 50.0);
        assert_eq!(y, 50.0);
    }

    #[test]
    fn float_drops_when_no_space() {
        let mut ctx = FloatContext::new(400.0);
        ctx.place_float(Float::Left, 250.0, 100.0, 0.0);
        // Second float doesn't fit beside the first (250 + 250 > 400).
        let (_x, y) = ctx.place_float(Float::Left, 250.0, 100.0, 0.0);
        assert_eq!(y, 100.0); // Drops below the first float
    }

    #[test]
    fn float_drops_below_when_width_exhausted() {
        // Left float fills entire width. A second float must drop below it
        // instead of being stuck at y=0 (tests that the loop doesn't break
        // early on negative available width).
        let mut ctx = FloatContext::new(400.0);
        ctx.place_float(Float::Left, 400.0, 80.0, 0.0);
        // Second float can't fit at y=0 (available=0), drops below.
        let (_, y) = ctx.place_float(Float::Left, 100.0, 50.0, 0.0);
        assert_eq!(y, 80.0);
    }

    #[test]
    fn float_bottom_with_multiple() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(Float::Left, 200.0, 80.0, 0.0);
        ctx.place_float(Float::Left, 200.0, 120.0, 0.0);
        ctx.place_float(Float::Right, 200.0, 100.0, 50.0);
        assert_eq!(ctx.float_bottom(), 150.0); // max(80, 120, 50+100)
    }
}
