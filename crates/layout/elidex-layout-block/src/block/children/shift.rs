//! Descendant shifting utilities for block layout.

use std::collections::HashSet;

use elidex_ecs::{EcsDom, Entity, InlineFlow};
use elidex_plugin::{LayoutBox, Vector, WritingModeContext};

use super::super::is_block_level;

/// Shift all block-level children along the block axis by `delta`,
/// iteratively including descendants.
///
/// Writing-mode-aware: shifts Y in `horizontal-tb`, X in vertical modes.
/// Used after parent-child margin collapse to reposition children that were
/// laid out before the collapse was detected.
///
/// `is_probe` — when this reposition happens inside a throwaway measurement pass
/// (see the `shift_descendants_inner` docs), only the `LayoutBox` measurement
/// geometry is moved; the render-consumed persisted state (`InlineFlow` / box store)
/// is left untouched.
pub(in crate::block) fn shift_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    delta: f32,
    wm: WritingModeContext,
    is_probe: bool,
) {
    if delta.abs() < f32::EPSILON {
        return;
    }
    let v = if wm.is_horizontal() {
        Vector::y_only(delta)
    } else {
        Vector::x_only(delta)
    };
    // Margin-collapse is an ancestor reposition of an already-built subtree, so a
    // standalone box fragment under it moves too (no own-fragment exclusion).
    shift_descendants_inner(dom, children, v, true, None, None, is_probe);
}

/// Shift descendants by (dx, dy), used to reposition float/positioned contents
/// after placement, and as the canonical ancestor-reposition shifter (relative
/// positioning, margin collapse). Moves `LayoutBox`, a persisted `InlineFlow`,
/// **and** every standalone fragment-tree box fragment in the subtree (P2) — all
/// hold absolute coords that an ancestor shift must carry.
///
/// `is_probe` — see the `shift_descendants_inner` docs: during a throwaway probe
/// pass only the `LayoutBox` is shifted, the persisted render state is left put.
pub fn shift_descendants(dom: &mut EcsDom, children: &[Entity], delta: Vector, is_probe: bool) {
    shift_descendants_inner(dom, children, delta, false, None, None, is_probe);
}

/// Like [`shift_descendants`], but does **not** move the standalone box fragments
/// of the entities in `own` — for a multicol positioning its OWN columns
/// (`position_column_fragments`), where THIS multicol's mid-break box fragments
/// are **born-absolute** (the column offset is baked at commit, like a paged
/// fragment), so re-shifting them here would double-apply the offset.
///
/// Fragments of OTHER entities in the subtree **are** still shifted — crucially a
/// **nested** multicol whole-in-this-column has already committed its own
/// spanning-child fragments at this multicol's column-0 base, and those must move
/// with the column delta (else they paint back in column 0 once consumed). So the
/// exclusion is per-entity (this multicol's own snapshots), NOT the whole subtree.
/// `LayoutBox`/`InlineFlow` are born-at-base and always shift to the column.
///
/// `exclude_subtrees` — entity roots whose **entire subtree** is pruned from this
/// walk (the `LayoutBox` shift, the persisted-state shift, AND the descent are all
/// skipped). Used by `position_column_fragments` for a multicol's **mid-break
/// atomics** (terminal-Z C-2): an atomic is repositioned wholesale by
/// `reposition_atomic_box` from its raw un-offset basis to a born-absolute target
/// (the folded `inline_start` already carries the column inline offset), and that
/// reposition shifts the atomic's whole subtree. So the generic column shift must
/// not touch the atomic OR its descendants — letting it through would double-apply
/// the column offset (root) and double-shift the content (descendants). Distinct
/// from `own`, which excludes only the box-*fragment* arm for an entity whose
/// `LayoutBox`/descendants still shift.
///
/// `is_probe` — see the `shift_descendants_inner` docs. Under a probe the
/// persisted-state shift is skipped for ALL entities (probe-skip dominates), so
/// `own` is irrelevant then; only the `LayoutBox` measurement geometry moves.
#[allow(clippy::implicit_hasher)]
pub fn shift_descendants_excluding_own_fragments(
    dom: &mut EcsDom,
    children: &[Entity],
    delta: Vector,
    own: &HashSet<Entity>,
    exclude_subtrees: &HashSet<Entity>,
    is_probe: bool,
) {
    shift_descendants_inner(
        dom,
        children,
        delta,
        false,
        Some(own),
        Some(exclude_subtrees),
        is_probe,
    );
}

/// Iterative tree walk that shifts `LayoutBox` (and persisted `InlineFlow`)
/// positions by `delta`.
///
/// When `block_only` is true, only block-level entities (with a `ComputedStyle`)
/// are shifted; non-block children are skipped (but their descendants are still
/// walked). `exclude_fragments_for` skips the standalone fragment-tree shift for
/// the listed entities (a multicol's own born-absolute snapshots); `None` shifts
/// every fragment (the ancestor-reposition case). `exclude_subtrees` prunes the
/// listed entity roots entirely — neither the root's `LayoutBox`/persisted state
/// nor any descendant is shifted, and the descent stops at the root (a multicol's
/// born-absolute mid-break atomics, repositioned wholesale elsewhere; C-2).
///
/// `is_probe` (P1 completion, Z-1b-0.5) — the shifter runs inside a **throwaway
/// measurement pass** (intrinsic sizing / 2-pass flex·grid·table re-measure /
/// balanced-multicol probe) whenever an ancestor input has `is_probe == true`. A
/// probe re-lays a subtree, then a **definitive** pass re-lays it again and has the
/// last word before render. So under a probe we move ONLY the `LayoutBox` (the
/// throwaway working geometry the probe measures and the definitive pass overwrites
/// via `insert_one` upsert) and leave the **render-consumed** persisted state put.
/// The two skipped arms are safe for *different* reasons:
///
/// - **`FragmentTree` box store** (dark today, consumed at committed-next): its
///   *write* IS `is_probe`-guarded (multicol `push_box`), so during a probe it
///   holds the *prior definitive pass's* coords. Shifting it here would corrupt
///   that definitive value (the probe never re-pushes it) — this is the load-
///   bearing fix (#316 R4, surfaced via the mid-break box fragment).
/// - **`InlineFlow` component** (visible — render paints it): its *write* is NOT
///   `is_probe`-guarded — the IFC persist (`insert_one`, a replace) re-writes it on
///   every pass, and `clear_inline_flows` + the always-last definitive pass re-lay
///   AND re-shift it (the per-run overwrite-safety the balanced-multicol path
///   documents in `fill_columns_balanced`; the one documented gap is a child the
///   definitive pass leaves un-relaid past the `MAX_OVERFLOW_COLUMNS` runaway cap —
///   a pre-existing stranded-flow truncation, unchanged by this gate). So a probe's
///   shift is *redundant* here today — the probe's transient value is overwritten —
///   and skipping it changes only throwaway intermediate state. (Under Z-1b/#316 the
///   mid-break `InlineFlow`
///   build moves into `position_column_fragments` and becomes `is_probe`-guarded
///   too, at which point this arm holds a prior-definitive value exactly like the
///   box store — so skipping the probe shift is forward-compatible, not just inert.)
///
/// Net: a probe neither PUSHes nor SHIFTs persisted render state; only the
/// `LayoutBox` measurement geometry moves, and the definitive pass is the last word.
fn shift_descendants_inner(
    dom: &mut EcsDom,
    children: &[Entity],
    delta: Vector,
    block_only: bool,
    exclude_fragments_for: Option<&HashSet<Entity>>,
    exclude_subtrees: Option<&HashSet<Entity>>,
    is_probe: bool,
) {
    let mut stack: Vec<Entity> = children.to_vec();
    while let Some(child) = stack.pop() {
        // A pruned subtree root (a born-absolute mid-break atomic, C-2): skip the
        // shift AND the descent — `reposition_atomic_box` already moves the whole
        // subtree to its born-absolute target, so any shift here would double-apply
        // the column offset (root) / double-shift the content (descendants).
        if exclude_subtrees.is_some_and(|s| s.contains(&child)) {
            continue;
        }
        let skip_shift = block_only
            && !crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display));
        if !skip_shift {
            if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(child) {
                lb.content.origin += delta;
            }
        }
        // Under a throwaway probe, only the `LayoutBox` above moves; the render-
        // consumed persisted state below (`InlineFlow` + box store) is left to the
        // always-last definitive pass. See the fn docstring for why each arm is safe
        // (box store = prior-definitive value to preserve; InlineFlow = re-shifted by
        // the definitive pass anyway). P1 completion, Z-1b-0.5.
        if !is_probe {
            // A persisted `InlineFlow` stores ABSOLUTE physical coordinates that render
            // consumes directly, so a subtree shift (relative positioning, out-of-flow
            // placement, atomic-inline reposition, margin collapse) must move it too —
            // else the converged inline text repaints at its pre-shift position. This is
            // NOT gated by the `block_only` `LayoutBox` filter: the run-start entity
            // carrying the flow is typically a styleless text node (which `block_only`
            // skips), yet its glyphs still move with the shifted ancestor block.
            // `InlineFlow` holds physical-per-axis values (the is_vertical fold at
            // persist): `block_start` = physical y (horizontal) / x (vertical),
            // `inline_start` = physical x (horizontal) / y (vertical) — so the physical
            // `delta` projects onto each field by the IFC's writing mode (the run-start
            // entity carrying the flow is a direct child of the IFC element).
            if dom.world().get::<&InlineFlow>(child).is_ok() {
                let is_vertical = dom
                    .get_parent(child)
                    .and_then(|p| crate::try_get_style(dom, p))
                    .is_some_and(|s| !s.writing_mode.is_horizontal());
                let (d_block, d_inline) = if is_vertical {
                    (delta.x, delta.y)
                } else {
                    (delta.y, delta.x)
                };
                if let Ok(mut flow) = dom.world_mut().get::<&mut InlineFlow>(child) {
                    for frag in &mut flow.fragments {
                        for line in &mut frag.lines {
                            line.block_start += d_block;
                            for run in &mut line.runs {
                                *run.inline_start_mut() += d_inline;
                            }
                        }
                    }
                }
            }
            // A standalone fragment-tree box fragment (multicol mid-break, Z-1a) also
            // holds ABSOLUTE coords, so an ANCESTOR subtree shift must move it too —
            // else the converged box (and, in Z-1b, its inline lines) would paint at
            // its pre-shift position once render consumes the store. Skipped only for
            // the entities in `exclude_fragments_for` (a multicol's OWN born-absolute
            // column snapshots, whose offset is already baked at commit) — a NESTED
            // multicol's fragments under this subtree are NOT excluded and still move.
            // Un-gated by `block_only` (the fragment is the entity's regardless of its
            // block-level-ness). O(1) via the entity index; a no-op for the common
            // entity that has no box fragment (P2).
            if exclude_fragments_for.is_none_or(|own| !own.contains(&child)) {
                dom.fragment_tree_mut().shift_entity(child, delta);
            }
        }
        // Always walk descendants regardless of block_only filter.
        stack.extend(dom.composed_children(child));
    }
}
