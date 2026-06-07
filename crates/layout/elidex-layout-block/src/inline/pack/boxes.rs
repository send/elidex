//! Inline-element box assignment: the [`EntityBounds`]/[`InlineLineRect`] geometry the
//! [`LinePacker`](super::LinePacker) accumulates per line, and [`assign_inline_layout_boxes`],
//! the pass that folds those bounds into each inline element's `LayoutBox` +
//! `InlineClientRects` (`getClientRects()`). Split out of `pack.rs` to keep the line
//! packer under the repo's ~1000-line convention.

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, EdgeSizes, LayoutBox, Point, Rect};

// ---------------------------------------------------------------------------
// Line box geometry — per-line rectangles for inline entities
// ---------------------------------------------------------------------------

/// Per-line rectangle for an inline entity (used by `getClientRects()`).
#[derive(Clone, Debug)]
pub struct InlineLineRect {
    /// Inline-axis start on this line.
    pub inline_start: f32,
    /// Inline-axis end on this line.
    pub inline_end: f32,
    /// Block-axis offset of this line.
    pub block_start: f32,
    /// Block-axis size of this line.
    pub block_size: f32,
}

/// Tracks the bounding rectangle of an inline entity across line boxes.
pub(in crate::inline) struct EntityBounds {
    /// Inline-axis start on the first line.
    pub(super) inline_start: f32,
    /// Inline-axis end on the last line.
    pub(super) inline_end: f32,
    /// Block-axis offset of the first line.
    pub(super) block_start: f32,
    /// Block-axis offset + size of the last line.
    pub(super) block_end: f32,
    /// Per-line rectangles for `getClientRects()`.
    pub line_rects: Vec<InlineLineRect>,
}

/// Assign `LayoutBox` to inline elements based on their bounding rects.
///
/// Each entity that has a `ComputedStyle` (i.e. is an element, not a text node)
/// and was tracked during line packing receives a `LayoutBox` with its
/// bounding rectangle in layout coordinates.
pub(in crate::inline) fn assign_inline_layout_boxes(
    dom: &mut EcsDom,
    entity_bounds: &HashMap<Entity, EntityBounds>,
    content_origin: Point,
    is_vertical: bool,
    layout_generation: u32,
) {
    let (origin_x, origin_y) = (content_origin.x, content_origin.y);
    for (entity, bounds) in entity_bounds {
        if dom.world().get::<&ComputedStyle>(*entity).is_err() {
            continue;
        }
        // Skip entities that already have a LayoutBox (e.g. from layout_child
        // for atomic inline boxes like inline-block).
        if dom.world().get::<&LayoutBox>(*entity).is_ok() {
            continue;
        }
        let (x, y, w, h) = if is_vertical {
            (
                origin_x + bounds.block_start,
                origin_y + bounds.inline_start,
                bounds.block_end - bounds.block_start,
                bounds.inline_end - bounds.inline_start,
            )
        } else {
            (
                origin_x + bounds.inline_start,
                origin_y + bounds.block_start,
                bounds.inline_end - bounds.inline_start,
                bounds.block_end - bounds.block_start,
            )
        };
        let lb = LayoutBox {
            content: Rect::new(x, y, w, h),
            padding: EdgeSizes::default(),
            border: EdgeSizes::default(),
            margin: EdgeSizes::default(),
            first_baseline: None,
            layout_generation,
        };
        let _ = dom.world_mut().insert_one(*entity, lb);

        // Store per-line rects for getClientRects() (CSSOM View §6): one border-box
        // fragment per line. A single-line element (`len() == 1` after the per-line
        // merge) exposes its one rect via the LayoutBox border box (the getClientRects
        // fallback), so no component is stored.
        //
        // NOTE (pre-existing, engine-wide — slot `#11-inline-relayout-box-staleness`):
        // this runs only when the element has no LayoutBox yet (the `continue` above
        // skips entities that already carry one). `layout_tree` never clears stale
        // boxes, so on a *relayout* an inline element keeps its prior box and neither its
        // LayoutBox nor a stale `InlineClientRects` is refreshed here. Reconciling that
        // needs a generation-bump / box-teardown in the layout driver (it also leaves the
        // LayoutBox itself stale), out of scope for this getClientRects-alignment slice.
        if bounds.line_rects.len() > 1 {
            let rects: Vec<elidex_plugin::Rect> = bounds
                .line_rects
                .iter()
                .map(|lr| {
                    if is_vertical {
                        elidex_plugin::Rect::new(
                            origin_x + lr.block_start,
                            origin_y + lr.inline_start,
                            lr.block_size,
                            lr.inline_end - lr.inline_start,
                        )
                    } else {
                        elidex_plugin::Rect::new(
                            origin_x + lr.inline_start,
                            origin_y + lr.block_start,
                            lr.inline_end - lr.inline_start,
                            lr.block_size,
                        )
                    }
                })
                .collect();
            let _ = dom
                .world_mut()
                .insert_one(*entity, elidex_plugin::InlineClientRects(rects));
        }
    }
}
