//! Reconciling the IFC's render-visible flow state after packing: `InlineFlow`
//! persistence, atomic repositioning, and the multicol `ColumnFlowSlice` carrier.
//!
//! Split out of `inline/mod.rs`, where this ran as the tail of
//! [`super::layout_inline_context_fragmented`]; the body is that block verbatim
//! apart from the bindings the signature introduces
//! (`docs/plans/2026-08-inline-seam3-reconcile-split.md`).

use std::collections::HashMap;

use elidex_ecs::{ColumnFlowSlice, EcsDom, Entity, InlineFlow};
use elidex_plugin::Point;

use super::{
    clear_inline_flows, relpos_atomic_reposition_records, reposition_atomic_box,
    static_atomic_reposition_records,
};

/// Persist each render-run-group's lines as `InlineFlow` (or capture them into
/// the multicol mid-break `ColumnFlowSlice` carrier), reposition the static and
/// relpos/sticky atomics, and clear stale flows on every candidate key that was
/// not persisted.
///
/// Every effect is a `dom` mutation; nothing is returned to the caller.
///
/// ⚠ **The probe universal in the body below is scoped to THIS function.** The
/// body states that "a probe neither PUSHes … SHIFTs … CLEARs … nor WRITEs
/// persisted render state", and gates its own `clear_inline_flows` call on
/// `!env.is_probe`. Every other removal of either component is **ungated**.
/// Enumerate them **per component**, not as one tally — the two are gated
/// differently, and an earlier revision of this docstring said "three" by
/// counting only some of them:
///
/// ```text
/// git grep -n 'InlineFlow\|ColumnFlowSlice' -- 'crates/layout/**/*.rs'
/// ```
///
/// ⚠ Anchor that grep on the **component name**, as above, and classify the hits
/// by hand. A grep anchored on the *call* instead (`remove_one::<Foo>`) silently
/// undercounts: the removal in `elidex-layout-multicol/src/lib.rs` writes the
/// path-qualified `remove_one::<elidex_ecs::ColumnFlowSlice>`, and the insert
/// below spreads `ColumnFlowSlice { .. }` over several lines. The name is the
/// invariant; the call syntax is not. (The command matches this comment too.)
///
/// `InlineFlow` is removed only via `clear_inline_flows`: gated here, ungated at
/// `layout_inline_context_fragmented`'s two early returns (no items / no usable
/// font). `ColumnFlowSlice` is removed **ungated at every one** of its sites in
/// this crate — those same two early returns, which drop the carrier alongside
/// the flows, and the carrier reconcile's `else` arm below.
///
/// ⚠ The `else` arm **fires during a probe**: both `do_carrier` write arms are
/// `!env.is_probe`-gated, so under a probe the carrier payloads stay empty and
/// the `else` arm runs — this component's *write* is probe-gated while its
/// *remove* is not. The early returns carry the same asymmetry for the same
/// component, which is why this is enumerated per component rather than tallied.
///
/// ⚠ **Behaviour is not at risk.** The two early returns are reached on
/// `items.is_empty()` / no-usable-font, inputs that do not depend on `is_probe`,
/// so a probe and the definitive pass reach them identically; and the carrier is
/// drained within the same pass and never read by render (see
/// [`elidex_ecs::ColumnFlowSlice`]'s docstring). What the universal gets wrong is
/// its *scope*: it reads as engine-wide and is not. Stated here rather than in the
/// body because the body is proved byte-identical to its pre-split form; the text
/// below must not be edited.
///
/// **Spec vs bookkeeping.** Most of what this function does is elidex render
/// bookkeeping with no governing section: [`InlineFlow`] and
/// [`elidex_ecs::ColumnFlowSlice`] are engine-internal components, and no CSS
/// module specifies "persist a flow" or "carry a column slice". Two steps *are*
/// spec-governed and are cited so a later edit can tell them apart:
/// * the IFC-local logical → absolute physical fold keyed on `is_vertical`
///   implements the **axis assignment** of **css-writing-modes-4 §6.4
///   Abstract-to-Physical Mappings** — inline-axis → physical x (horizontal) /
///   y (vertical), block-axis → the other. ⚠ It does **not** implement the rest
///   of §6.4, whose mapping is keyed on the used `writing-mode` *and*
///   `direction` (its `block-start` row is `top`/`right`/`left`; its
///   `inline-start` row varies with both). **This fold reads `writing-mode`
///   only as the boolean `is_vertical`** — the property has five values and
///   all four *vertical* ones (`vertical-rl`, `vertical-lr`, `sideways-rl`,
///   `sideways-lr`) collapse to `is_vertical == true`, derived at
///   `inline/mod.rs` from `writing_mode.is_horizontal()` — **and
///   never reads `direction` at all.** So it applies no
///   `vertical-rl`/`sideways-rl` block-axis reversal, matching the box
///   convention (see the comment at the fold), and the `sideways-lr` inline
///   axis is likewise not distinguished. Cite the axis rows only; a later edit
///   must widen the existing `is_vertical` read rather than assume
///   `writing-mode` is not an input here, and must not read this as §6.4
///   conformance;
/// * the atomics' block-axis target is the line top, which leaves
///   **`vertical-align` within the line box** unimplemented. The governing
///   section is **CSS 2 §10.8 Line height calculations: the `line-height` and
///   `vertical-align` properties**. ⚠ Stated positively, because "only
///   `vertical-align` is missing" would be a claim over §10.8's whole
///   complement: what **is** implemented and cited elsewhere in this crate is
///   §10.8.1 leading/half-leading and the baseline derivation (`inline/mod.rs`,
///   `inline/pack/mod.rs`). What is **not** implemented includes `vertical-align`
///   alignment *and* §10.8's strut and its uppermost-top-to-lowermost-bottom
///   line-box height. What this crate takes instead is a max over per-item
///   **block contributions** — `line-height` for horizontal text, `font-size`
///   for vertical (`inline/pack/mod.rs`'s `seg_line_advance`), and the
///   margin-box block size for atomics — and it has no strut. See the inline comment at the `persist_flow`
///   reposition.
///
/// ⚠ The *uncited* spec-governed prose inside the body (relative/sticky offset
/// preservation, fragmentainer terminology, column-box continuation) is
/// pre-existing and untouched by the split — this function was relocated
/// byte-identically, so it authors no algorithm. Adding blanket module-level
/// citations for it would over-claim, which is the call #497 already made for
/// `collect.rs`/`styled_run.rs`.
///
/// `persist_flow` and `do_carrier` are **mutually exclusive**, and the caller
/// establishes it rather than this function checking it: `do_carrier` implies
/// `frag_is_column && !column_is_whole`, which falsifies `persist_flow`'s second
/// conjunct (see their derivation in [`super::layout_inline_context_fragmented`]).
/// The arms below are therefore written `if persist_flow … else if do_carrier`,
/// and the `else if` is exclusivity, not precedence — if both were ever true the
/// carrier payload would be silently dropped.
#[allow(clippy::too_many_arguments)]
pub(super) fn reconcile_flows(
    dom: &mut EcsDom,
    parent_entity: Entity,
    content_origin: Point,
    env: &crate::LayoutEnv<'_>,
    is_vertical: bool,
    persist_flow: bool,
    do_carrier: bool,
    candidate_keys: &[Entity],
    unoffset_origins: &HashMap<Entity, Point>,
    flow_lines: HashMap<Entity, Vec<elidex_ecs::InlineFlowLine>>,
    relpos_atomic_placements: &[(Entity, f32, f32)],
) {
    // Reconcile InlineFlow across the IFC's render-run-groups every pass: persist
    // each group's lines on its run-start key (the top-level run-start + one per
    // converged `position:relative`/`sticky` inline sub-flow), then clear stale
    // flows on every candidate key not persisted. `layout_generation` is constant 0
    // off the paged path, so this is an explicit reconcile (insert-or-remove), not
    // a generation comparison — what prevents render consuming a stale flow after a
    // realignment, a relpos→static transition, an abspos toggle, or a now-gated run.
    let mut persisted_keys: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    // The multicol mid-break carrier groups (run-start → this column's folded lines),
    // populated in the `do_carrier` arm below and written to `ColumnFlowSlice`. Empty
    // for the `persist_flow` case (lines go to `InlineFlow`) and for the no-content
    // case (carrier cleared).
    let mut carrier_groups: Vec<(Entity, Vec<elidex_ecs::InlineFlowLine>)> = Vec::new();
    // C-2: mid-break atomic reposition payload carried alongside `carrier_groups`,
    // one uniform `(entity, inline_abs, block_abs, unoffset_origin)` record per
    // atomic (IFC-absolute on-line target at column-0 base + reposition delta basis).
    // Covers BOTH static atomics (also `AtomicBox` flow members — their target rides
    // the `AtomicBox` run, captured here so the seam needs no second run-walk) and
    // relpos/sticky atomics (NOT flow members). Empty for the `persist_flow` case
    // (the atomics are repositioned in-line below) and for a probe (the `do_carrier`
    // arms are `!env.is_probe`-gated).
    let mut carrier_atomics: Vec<(Entity, f32, f32, Point)> = Vec::new();
    if persist_flow || do_carrier {
        // IFC-local logical → absolute physical, applying the SAME is_vertical
        // projection rule as `static_positions` and `assign_inline_layout_boxes`:
        // inline-axis → physical x (horizontal) / y (vertical), block-axis → the
        // other. After the fold each scalar is the absolute physical coordinate for
        // its axis, so render reads `block_start`/`inline_start` without a
        // transform. No vertical-rl block-axis reversal — matching the box convention.
        let (inline_origin, block_origin) = if is_vertical {
            (content_origin.y, content_origin.x)
        } else {
            (content_origin.x, content_origin.y)
        };
        // Each group's bucket (keyed on its run-start `run[0]`): the top-level group
        // and one per converged positioned-inline sub-flow. The fold + static-atomic
        // reposition are shared between the `persist_flow` sink (write each group's
        // `InlineFlow` here) and the `do_carrier` sink (capture the group's lines into
        // the carrier; multicol fill drains them and `position_column_fragments` folds
        // them into the run-start's `InlineFlow` offset per column).
        for (group_key, group_lines) in flow_lines {
            if group_lines.is_empty() {
                continue;
            }
            let lines: Vec<elidex_ecs::InlineFlowLine> = group_lines
                .into_iter()
                .map(|mut line| {
                    line.block_start += block_origin;
                    for run in &mut line.runs {
                        *run.inline_start_mut() += inline_origin;
                    }
                    line
                })
                .collect();
            if persist_flow {
                // Reposition each *static* atomic inline's `LayoutBox` in THIS group to
                // its on-line position (text-align already baked into `inline_start`).
                // `layout_atomic_items` laid the atomic out at `content_origin` (IFC
                // top-left); render paints it by `walk()`-ing its `LayoutBox`, so the
                // box must reflect the line position (layout owns geometry — render
                // does not paint-time-translate). Only *static* atomics are `AtomicBox`
                // flow members; *relative/sticky* atomics are NOT members (they go
                // through the `relpos_atomic_placements` pass below — slice 3p-b-2). The
                // delta basis is the atomic's un-offset margin-box origin
                // (`unoffset_origins`), which for a static atomic equals its current box
                // origin → identical reposition to slice 3p-a. Block-axis = line top
                // (baseline-naive; CSS 2 §10.8 `vertical-align` within the line box is
                // deferred — same as text runs).
                //
                // The mid-break (`do_carrier`) case CANNOT reposition here: a multicol
                // mid-break IFC re-runs `layout_atomic_items` for the WHOLE IFC every
                // column (continuation), resetting any earlier-column atomic's box back
                // to `content_origin`, and a single column's run only knows its own
                // slice — so repositioning here would fix only the LAST column and leave
                // earlier-column atomics displaced (Codex PR#316 R1). Instead the
                // `do_carrier` arm below carries each mid-break atomic's on-line target +
                // basis out via `ColumnFlowSlice`, and the multicol seam
                // `position_column_fragments` repositions them after the final re-lay,
                // offset per column (terminal-Z C-2 — atomic-as-fragment).
                for (atomic, inline_abs, block_abs, basis) in
                    static_atomic_reposition_records(&lines, unoffset_origins)
                {
                    reposition_atomic_box(
                        dom,
                        atomic,
                        inline_abs,
                        block_abs,
                        is_vertical,
                        Some(basis),
                        env.is_probe,
                    );
                }
                // I-paged writes one fragment per page (length-1 Vec): each page's
                // full re-layout replaces it (render walks the page interleaved before
                // the next), so the run-start carries this page's slice stamped with
                // the page generation. Multicol whole-in-column persists its 1-fragment
                // flow here too (shifted to its column by the column shift).
                //
                // A throwaway probe must NOT overwrite a persisted flow (Codex PR#316
                // R2, P2): `probe_total_height` (balanced-multicol fill, Step 1) lays
                // the IFC with NO fragmentainer, so a mid-break IFC reaches THIS
                // `persist_flow` arm (no column ⇒ `column_is_whole`) and would clobber
                // the live all-column mid-break flow with single-column col-0 probe
                // geometry — and the probe-guarded `position_column_fragments` will not
                // rebuild it, so the corruption survives to render. Skip the write
                // during a probe: the existing flow is preserved (the clear below is
                // `is_probe`-gated) and the definitive pass writes the real flow. This
                // completes probe-mutation-freeness — a probe neither PUSHes (box
                // store, #315/#318), SHIFTs (#318), CLEARs (R1), nor WRITEs persisted
                // render state. (The static-atomic reposition above still runs: it is
                // `is_probe`-aware and moves only the throwaway `LayoutBox`.)
                if !env.is_probe {
                    let _ = dom
                        .world_mut()
                        .insert_one(group_key, InlineFlow::single(env.layout_generation, lines));
                    persisted_keys.insert(group_key);
                }
            } else if do_carrier && !env.is_probe {
                // do_carrier: this column's slice for this run-start group. multicol
                // fill drains the carrier; `position_column_fragments` folds it into
                // `group_key`'s `InlineFlow` offset to the column's inline position,
                // AND repositions each mid-break atomic's `LayoutBox` to its per-column
                // on-line position (terminal-Z C-2 — the reposition the `persist_flow`
                // arm above runs in-line cannot run here because a column's re-lay
                // clobbers earlier columns, so it is deferred to the post-fill seam).
                // For a *static* atomic (an `AtomicBox` flow member) capture its on-line
                // target — the run's IFC-absolute `inline_start` + the line `block_start`
                // (column-0 base; the seam adds the column inline offset) — plus the
                // reposition delta basis (the SAME records the `persist_flow` arm would
                // reposition immediately, deferred to the seam instead).
                carrier_atomics.extend(static_atomic_reposition_records(&lines, unoffset_origins));
                carrier_groups.push((group_key, lines));
                // Else: a throwaway probe over a do_carrier run (write NO carrier — the
                // probe's per-column geometry is discarded; `position_column_fragments`
                // is `is_probe`-guarded and the live mid-break `InlineFlow` is preserved
                // by the `is_probe`-gated `clear_inline_flows` below, Codex PR#316 R3).
                // A clipping (`overflow:hidden`) mid-break block now ALSO takes the
                // carrier branch (terminal-Z C-1 retired the `midbreak_clips` exception):
                // render's fragment-walk consumes its per-column box store + re-emits the
                // converged `InlineFlow` under each per-column clip, so no col-0 content
                // is clipped away by a single last-column clip.
            }
        }
        // Reposition each `position:relative`/`sticky` atomic's `LayoutBox` to its
        // on-line position, PRESERVING the applied relative offset (slice 3p-b-2).
        // These are NOT flow members (render Layer 6 paints the positioned box, so a
        // member would double-paint) — they were collected into a flat, IFC-root-local
        // placement list, so fold each with the IFC-root `(inline_origin,
        // block_origin)` (the same fold a top-level flow member gets) and reposition.
        // The delta basis is the atomic's un-offset margin-box origin: for a relpos
        // atomic the current box already carries the baked `apply_relative_offset`, so
        // `delta = target − un-offset` lands it at `target + offset` (offset preserved,
        // not stripped). Same two-sink split as the static-atomic loop above:
        // `persist_flow` repositions immediately; `do_carrier` (mid-break) carries the
        // SAME records out to the multicol seam (terminal-Z C-2), since a column's
        // re-lay would clobber an in-line reposition.
        let relpos_records = relpos_atomic_reposition_records(
            relpos_atomic_placements,
            inline_origin,
            block_origin,
            unoffset_origins,
        );
        if persist_flow {
            for (atomic, inline_abs, block_abs, basis) in relpos_records {
                reposition_atomic_box(
                    dom,
                    atomic,
                    inline_abs,
                    block_abs,
                    is_vertical,
                    Some(basis),
                    env.is_probe,
                );
            }
        } else if do_carrier && !env.is_probe {
            // Mid-break relpos/sticky atomics (terminal-Z C-2): NOT flow members, so
            // carry the SAME records out to the seam (the placements are already
            // sliced+rebased to THIS column by `slice_and_rebase_fragment`).
            // `position_column_fragments` adds the column inline offset and repositions,
            // preserving the baked relative offset.
            carrier_atomics.extend(relpos_records);
        }
    }
    // Carrier reconcile (insert-or-remove, mirroring `clear_inline_flows`): the
    // multicol mid-break IFC writes its per-column slice on `parent_entity`; every
    // other case (whole/paged/non-fragmented persist, or empty) clears any stale one.
    // `ColumnFlowSlice` is never read by render (drained-only by multicol fill), so a
    // leak is benign; this keeps the entity clean across passes.
    if do_carrier && !(carrier_groups.is_empty() && carrier_atomics.is_empty()) {
        // Write the carrier when EITHER payload is present: a mid-break column whose
        // slice is only a relpos/sticky atomic (NOT a flow member) has empty
        // `carrier_groups` but a non-empty `carrier_atomics`, and must still reach the
        // seam to be repositioned (else it falls to the un-excluded generic shift).
        let _ = dom.world_mut().insert_one(
            parent_entity,
            ColumnFlowSlice {
                flow_groups: carrier_groups,
                atomic_repositions: carrier_atomics,
            },
        );
    } else {
        let _ = dom.world_mut().remove_one::<ColumnFlowSlice>(parent_entity);
    }
    // Invariant: every persisted key must be a candidate (else it could never be
    // cleared on a later pass → stale-flow leak). Holds by construction — a
    // persisted key is some run-parent's first eligible child, and candidates
    // include every run-parent's raw direct children.
    debug_assert!(
        persisted_keys.iter().all(|k| candidate_keys.contains(k)),
        "persisted InlineFlow key not in candidate set → stale-flow leak risk"
    );
    // A throwaway probe must NOT clear any persisted `InlineFlow` (Codex PR#316 R3
    // post-rebase, P2): a mid-break IFC is laid per-column, so a probe of one column
    // sees only THAT column's run groups in `persisted_keys`. Clearing here would
    // erase the live flow of a run group whose lines fall in ANOTHER column (e.g. a
    // `position:relative` inline sub-flow starting in a later column) — a flow the
    // probe deliberately does NOT rebuild (`position_column_fragments` is
    // `is_probe`-guarded), dropping that sub-flow to the legacy path until the next
    // definitive layout. The definitive pass owns clear+rebuild; a probe leaves every
    // persisted flow untouched (symmetric with the `is_probe`-guarded box-store push
    // and the shifter's `is_probe` skip, #318). This subsumes the per-group probe
    // preservation in the `do_carrier` arm above (which protected only the current
    // column's group — the multi-group gap this closes).
    if !env.is_probe {
        clear_inline_flows(dom, candidate_keys, &persisted_keys);
    }
}
