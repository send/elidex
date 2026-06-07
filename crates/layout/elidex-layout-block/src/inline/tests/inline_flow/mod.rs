//! Tests for `InlineFlow` persistence — the converged inline-text geometry that
//! render consumes (slice 1 of the render↔layout inline-pipeline convergence).
//!
//! Split into focused topic submodules (persist / align / atomic / justify /
//! transform / fragment / vertical) to keep each under the repo's ~1000-line
//! convention; the shared helpers (`env`, `run_start`, `layout_vertical`) stay here
//! and reach the submodules through their `use super::*`.

use super::*;

mod align;
mod atomic;
mod fragment;
mod justify;
mod persist;
mod transform;
mod vertical;

/// Build a `LayoutEnv` for the test font db. `pub(super)` so the sibling
/// `relpos_subflow` test module can reuse it.
pub(super) fn env(font_db: &FontDatabase) -> crate::LayoutEnv<'_> {
    crate::LayoutEnv {
        font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    }
}

/// The run-start key = the first composed child of the parent (render's run[0]).
fn run_start(dom: &EcsDom, parent: Entity) -> Entity {
    dom.composed_children(parent)[0]
}

/// Lay out `text` under writing mode `wm` and return `(dom, parent, run-start key)`.
/// `containing_inline_size` is the inline-axis extent (height for vertical modes).
fn layout_vertical(
    text: &str,
    wm: WritingMode,
    containing_inline_size: f32,
    origin: Point,
) -> Option<(EcsDom, Entity, Entity)> {
    let (mut dom, parent, mut style, font_db) = setup_inline_test(text)?;
    style.writing_mode = wm;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        containing_inline_size,
        parent,
        origin,
        &env(&font_db),
    );
    Some((dom, parent, key))
}
