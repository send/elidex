//! Inline formatting context layout algorithm.
//!
//! Handles text measurement and line breaking for inline content.
//! Text is collected as styled runs that preserve per-element style
//! (font-size, font-weight, font-family, spacing), then greedily
//! packed into line boxes that fit the containing block width.

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

mod atomic;
pub(crate) mod measure;
mod pack;

mod collect;
mod styled_run;
mod whitespace;

use std::collections::HashMap;

use elidex_ecs::{ColumnFlowSlice, EcsDom, Entity, InlineFlow, InlineFlowRun};
use elidex_plugin::Point;
// Kept as its own statement: the C-3a LayoutBox reader-audit allowlist tracks this
// line verbatim, so folding the other test-only re-exports into it would show up as
// a migrated reader and force allowlist + inventory churn for a move that touches no
// `LayoutBox` reader at all.
#[cfg(test)]
pub(crate) use elidex_plugin::LayoutBox;
#[cfg(test)]
pub(crate) use elidex_plugin::{ComputedStyle, Display};
use elidex_text::measure_text;
#[cfg(test)]
pub(crate) use elidex_text::{FontDatabase, TextMeasureParams};

pub use measure::{max_content_inline_size, min_content_inline_size};

// Re-exported so descendants (`atomic`, `measure`, `pack`, `tests`) keep reaching
// these as `super::…` / `super::super::…` — the split moved the definitions, not
// the paths callers use. The `#[cfg(test)]` re-exports above serve the same purpose
// for the `tests` submodules' `use super::*`: `ComputedStyle` / `Display` /
// `TextMeasureParams` left with the collection and styled-run code, so they are
// re-exported for the reason `LayoutBox` and `FontDatabase` already were.
pub(crate) use collect::collect_inline_items;
pub(crate) use styled_run::InlineItem;
pub use styled_run::StyledRun;
pub(in crate::inline) use whitespace::is_collapsible_space;

// ---------------------------------------------------------------------------
// Inline layout entry point
// ---------------------------------------------------------------------------

/// Result of inline layout, including static positions for absolutely positioned descendants.
pub struct InlineLayoutResult {
    /// Total block-axis dimension consumed by all line boxes.
    pub height: f32,
    /// Static positions for absolutely positioned placeholders
    /// (CSS 2.1 §10.3.7 left/right / §10.6.4 top). Already projected to PHYSICAL x/y
    /// here (see the `is_vertical` mapping below) — unlike `LinePacker::static_positions`,
    /// which is logical. Content-area-relative.
    pub static_positions: HashMap<Entity, Point>,
    /// First baseline offset from content box top edge.
    ///
    /// CSS 2.1 §10.8.1: computed from the first line box's text run baseline,
    /// accounting for half-leading distribution.
    pub first_baseline: Option<f32>,
    /// Total number of line boxes produced.
    pub line_count: usize,
    /// If fragmentation was applied, the number of lines in this fragment.
    /// Lines after this count should be laid out in the next fragmentainer.
    pub break_after_line: Option<usize>,
}

/// Fragmentation constraint for inline layout (CSS Fragmentation L3 §3.3
/// "Breaks Between Lines: orphans, widows").
pub struct InlineFragConstraint {
    /// Available block-axis space from the current cursor position.
    pub available_block: f32,
    /// CSS orphans value (inherited, default 2).
    pub orphans: u32,
    /// CSS widows value (inherited, default 2).
    pub widows: u32,
    /// Number of lines to skip (for resuming after a break).
    pub skip_lines: usize,
    /// Which fragmentation engine this run is inside, carried from
    /// [`FragmentainerContext::fragmentation_type`](crate::FragmentainerContext).
    /// Drives the fragmentation term of the persist gate: `Page` runs persist an
    /// `InlineFlow` per page (slice 4 / I-paged — per-page slice + continuation rebase,
    /// consumed via the page generation); `Column` (multicol) runs persist only when
    /// **whole in their column** (slice 4 / I-multicol — `skip_lines == 0` and no
    /// fragment break, shifted to the column offset by the multicol column shift). A
    /// mid-IFC column break (continuation or truncation) stays gated to legacy,
    /// deferred to the standalone fragment tree (Z).
    pub fragmentation_type: crate::FragmentationType,
}

/// Layout inline content (text nodes, inline elements, and atomic inline
/// boxes) within line boxes.
///
/// Returns the total block-axis dimension consumed by all line boxes
/// and static positions for absolutely positioned descendants.
/// For `horizontal-tb` this is the total height; for vertical writing
/// modes (`vertical-rl`/`vertical-lr`) this is the total width.
///
/// `containing_inline_size` is the available inline-axis space
/// (width for horizontal, height for vertical).
///
/// `content_origin` is the position of the parent's content area
/// in layout coordinates, used to assign absolute positions to inline elements.
///
/// `layout_child` dispatches layout for atomic inline-level boxes
/// (e.g. `display: inline-block`).
///
/// Each line box's height is the maximum `line_height` of all runs on
/// that line (CSS 2.1 §10.8). Atomic inline boxes contribute their
/// margin-box height.
///
/// Inline elements (entities with `ComputedStyle` that are not the parent)
/// receive a `LayoutBox` with their bounding rectangle across line boxes.
pub fn layout_inline_context(
    dom: &mut EcsDom,
    children: &[Entity],
    containing_inline_size: f32,
    parent_entity: Entity,
    content_origin: Point,
    env: &crate::LayoutEnv<'_>,
) -> InlineLayoutResult {
    layout_inline_context_fragmented(
        dom,
        children,
        containing_inline_size,
        parent_entity,
        content_origin,
        env,
        None,
    )
}

/// Layout inline content with optional fragmentation constraint.
#[allow(clippy::too_many_lines)]
pub fn layout_inline_context_fragmented(
    dom: &mut EcsDom,
    children: &[Entity],
    containing_inline_size: f32,
    parent_entity: Entity,
    content_origin: Point,
    env: &crate::LayoutEnv<'_>,
    frag_constraint: Option<&InlineFragConstraint>,
) -> InlineLayoutResult {
    let parent_style = crate::get_style(dom, parent_entity);
    let font_db = env.font_db;
    let layout_child = env.layout_child;
    let (mut items, candidate_keys, top_level_key) =
        collect_inline_items(dom, children, &parent_style, parent_entity);
    // Staleness clear set: when nothing persists (here and the no-font return
    // below), clear `InlineFlow` on every candidate key (no persisted keys to
    // keep). `clear_inline_flows` removes the component on each — a no-op on
    // entities that never had one. The candidate set is a superset of every
    // entity that could have been a run-start key in a prior pass (§ reconcile).
    let no_persisted: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    if items.is_empty() {
        clear_inline_flows(dom, &candidate_keys, &no_persisted);
        let _ = dom.world_mut().remove_one::<ColumnFlowSlice>(parent_entity);
        return InlineLayoutResult {
            height: 0.0,
            static_positions: HashMap::new(),
            first_baseline: None,
            line_count: 0,
            break_after_line: None,
        };
    }

    let is_vertical = !parent_style.writing_mode.is_horizontal();
    // Layout atomic inline boxes and fill in their dimensions. Returns each atomic's
    // un-offset margin-box origin (the reposition delta basis — see the persist
    // block's `reposition_atomic_box` calls; preserves a relpos atomic's offset).
    let unoffset_origins = atomic::layout_atomic_items(
        dom,
        &mut items,
        containing_inline_size,
        content_origin,
        font_db,
        layout_child,
        is_vertical,
        env.layout_generation,
        env.is_probe,
    );

    // Verify at least one text run has a usable font (atomics don't need fonts).
    let has_text = items.iter().any(|item| matches!(item, InlineItem::Text(_)));
    if has_text {
        let any_font = items.iter().any(|item| match item {
            InlineItem::Text(run) => {
                let fam = run.family_refs();
                let p = run.measure_params(&fam);
                measure_text(font_db, &p, "x").is_some()
            }
            InlineItem::Atomic { .. } | InlineItem::Placeholder(_) => false,
        });
        if !any_font && !items.iter().any(|i| matches!(i, InlineItem::Atomic { .. })) {
            clear_inline_flows(dom, &candidate_keys, &no_persisted);
            let _ = dom.world_mut().remove_one::<ColumnFlowSlice>(parent_entity);
            return InlineLayoutResult {
                height: 0.0,
                static_positions: HashMap::new(),
                first_baseline: None,
                line_count: 0,
                break_after_line: None,
            };
        }
    }

    // InlineFlow persistence gate. There is NO cross-cutting legacy route left: the
    // three text-feature gates that once forced render's legacy collect/collapse/emit
    // path — text-transform, bidi, and **justify** — have all converged into the
    // packer (text-transform applied in-place before packing; RTL runs persisted in
    // logical order and reordered at paint, UAX #9 L2; justify positions baked here,
    // `flush_line`/`bake_justify`, like the other three alignments — CSS Text 3 §6).
    // Member-kind divergences are likewise gone (slice 3p-a static-atomic / 3p-b
    // relpos/sticky inline / 3p-b-2 relative/sticky atomic). The ONLY remaining gate is
    // fragmentation: persist when non-fragmented, **paged** (slice 4 / I-paged: the
    // per-page slice + continuation rebase below model the per-page geometry, fragment
    // stamped with the page generation), or **multicol whole-in-column** (slice 4 /
    // I-multicol — refined post-pack, see `persist_flow` below). A multicol IFC split
    // mid-column is the last legacy route (→ Z).
    //
    // `persist_candidate` (here, pre-pack) drives only `flow_align` and is OPTIMISTIC for
    // `Column`: the real multicol persist needs `break_after_line`/`skip_lines` (computed
    // after packing) to require whole-in-column. Including `Column` here is safe because
    // `flow_align` gates ONLY `flow_lines`/`relpos_atomic_placements` (discarded if the run
    // does not ultimately persist), NOT `entity_bounds`/`static_positions`/`line_boxes`/the
    // break computation (the packer commits those unconditionally) — so an optimistic
    // candidate that resolves mid-break perturbs no box geometry. (Vertical writing modes
    // persist too — the packer is axis-agnostic, the origin fold swaps axes.)
    let frag_is_paged =
        frag_constraint.is_some_and(|c| c.fragmentation_type == crate::FragmentationType::Page);
    let frag_is_column =
        frag_constraint.is_some_and(|c| c.fragmentation_type == crate::FragmentationType::Column);
    let persist_candidate = frag_constraint.is_none() || frag_is_paged || frag_is_column;
    let flow_align = persist_candidate.then_some(pack::FlowAlign {
        text_align: parent_style.text_align,
        direction: parent_style.direction,
        containing_inline_size,
        top_level_key,
        is_vertical,
    });

    let pack_items = pack::build_pack_items(&items);

    // Greedy line packing.
    let mut packer = pack::LinePacker::new(parent_entity, flow_align);
    for pi in &pack_items {
        packer.pack(
            pi,
            &items,
            dom,
            font_db,
            containing_inline_size,
            is_vertical,
        );
    }
    packer.finish();

    let line_count = packer.line_boxes.len();

    // --- Fragmentation: orphans/widows enforcement (CSS Fragmentation L3 §3.3) ---
    let break_after_line = if let Some(constraint) = frag_constraint {
        let skip = constraint.skip_lines;
        let line_heights: Vec<f32> = packer.line_boxes.iter().map(|lb| lb.block_size).collect();

        // Find the first line that overflows.
        let mut cumulative = 0.0_f32;
        let mut break_line: Option<usize> = None;
        for (i, &h) in line_heights.iter().enumerate().skip(skip) {
            if cumulative + h > constraint.available_block {
                break_line = Some(i);
                break;
            }
            cumulative += h;
        }

        if let Some(bl) = break_line {
            let total = line_heights.len();
            let orphans = constraint.orphans as usize;
            let widows = constraint.widows as usize;
            // orphans: at least `orphans` lines must stay in this fragment.
            let mut actual_break = bl.max(skip + orphans);
            // widows: at least `widows` lines must go to the next fragment.
            if total > actual_break && total - actual_break < widows {
                actual_break = total.saturating_sub(widows);
            }
            // If orphans + widows > total lines, treat as monolithic.
            if actual_break < skip + orphans || actual_break >= total {
                None // cannot satisfy constraints or all lines fit
            } else {
                Some(actual_break)
            }
        } else {
            None // all lines fit
        }
    } else {
        None
    };

    // Compute total block using only lines up to break point (if fragmented).
    let effective_line_count = break_after_line.unwrap_or(line_count);
    let skip_lines = frag_constraint.map_or(0, |c| c.skip_lines);

    // Refined persistence gate (slice 4 / I-multicol). `persist_flow` is the pre-pack
    // `persist_candidate` with its optimistic `Column` term narrowed to WHOLE-in-column:
    // the run starts at line 0 (not a continuation carried from a prior column) AND is
    // not truncated by a fragment break. A continuation (`skip_lines > 0`) would render
    // only the tail (the prior column's lines were gated out → lost); a truncation
    // (`break_after_line.is_some()`) drops its tail to a column the column shift won't
    // reach. Either ⇒ legacy, so no lines are lost. Mid-IFC column break converges with
    // box fragments at Z (G11: one LayoutBox/InlineFlow per entity; the column shift
    // moves a run-start's whole subtree by one delta, so a two-fragment run-start cannot
    // split across columns). A column run that resolves mid-break here just isn't
    // persisted — its optimistically-built `flow_lines` are discarded (box geometry is
    // `flow_align`-independent, see `persist_candidate`).
    let column_is_whole = skip_lines == 0 && break_after_line.is_none();
    let persist_flow = persist_candidate && (!frag_is_column || column_is_whole);
    // Multicol mid-break (the last non-persisted column route): the per-column line
    // slice does not go to an `InlineFlow` here (the IFC runs per column at column-0
    // base and does not know the column inline offset). It is captured into the
    // transient `ColumnFlowSlice` carrier on `parent_entity`, drained by multicol
    // fill, and folded into the run-start's `InlineFlow` (offset per column) by
    // `position_column_fragments` (Z-1b, Option D). Mutually exclusive with
    // `persist_flow`: `do_carrier` ⟹ `frag_is_column && !column_is_whole` ⟹
    // `persist_flow == false`.
    //
    // A mid-break IFC whose own block clips overflow ALSO carries now (terminal-Z
    // C-1, retiring the #316 `midbreak_clips` legacy-fallback): render's
    // fragment-walk consumes this entity's per-column box store fragments and pushes
    // a SEPARATE clip per column, then re-emits the converged all-column `InlineFlow`
    // under each disjoint column clip (each line survives in exactly one column). So
    // the carrier + the per-column box store (Z-1a, always populated for clipping
    // mid-break — `!is_probe` only, not clip-gated) coincide on the same entity, and
    // the `consumable` store flag steers render to the per-fragment path. Carrying
    // for the clipping case is the chain that fixes the col-0-clipped-away regression
    // #316 deferred (§2.6 hard invariant: this term and the C-1 render consume land
    // together).
    let do_carrier = frag_is_column && !column_is_whole;
    let total_block: f32 = packer
        .line_boxes
        .iter()
        .skip(skip_lines)
        .take(effective_line_count.saturating_sub(skip_lines))
        .map(|lb| lb.block_size)
        .sum();

    // I-paged: slice every recorded geometry to this fragment's kept lines and
    // continuation-rebase the block axis to the fragmentainer block-start, in a
    // single pass over the packer's one block-offset source (F2). Runs BEFORE the
    // box/static/flow consumers below so all of them (inline `LayoutBox`,
    // `InlineClientRects`, static positions, persisted fragment) read the
    // already-rebased values, and before the `content_origin` fold so the page
    // offset is added exactly once. Paged-scoped: multicol (`Column`) stays gated
    // to legacy (its column shift + accumulate is I-multicol), so its box geometry
    // is left unrebased here too (no change vs today). The rebase is the missing
    // continuation geometry the founding gate test flagged.
    if frag_is_paged || do_carrier {
        debug_assert!(
            skip_lines <= effective_line_count && effective_line_count <= line_count,
            "fragment slice bounds out of order: skip {skip_lines}, eff {effective_line_count}, count {line_count}"
        );
        // Multicol mid-break (`do_carrier`) needs the SAME per-column slice +
        // continuation rebase paged uses: this column's kept lines, the first
        // rebased to the column block-start. The column inline offset is applied
        // later (`position_column_fragments`), like the box snapshot. Like paged,
        // this also slices+rebases `entity_bounds` (the inline `LayoutBox`es /
        // `InlineClientRects`) and `static_positions` to this column — "more
        // correct" per-column geometry, but those remain G11 last-column-wins
        // (one box per entity) and are render-dark for text now (the `InlineFlow`
        // carries paint); per-fragment inline `LayoutBox`/clientRects + abspos-in-
        // mid-break placement is committed-next (cssom-view store consume).
        packer.slice_and_rebase_fragment(skip_lines, effective_line_count);
    }

    pack::assign_inline_layout_boxes(
        dom,
        &packer.entity_bounds,
        content_origin,
        is_vertical,
        env.layout_generation,
    );

    // Convert static positions from packer-relative to layout coordinates.
    let static_positions: HashMap<Entity, Point> = packer
        .static_positions
        .into_iter()
        .map(|(entity, logical_pos)| {
            if is_vertical {
                (
                    entity,
                    Point::new(
                        content_origin.x + logical_pos.y,
                        content_origin.y + logical_pos.x,
                    ),
                )
            } else {
                (
                    entity,
                    Point::new(
                        content_origin.x + logical_pos.x,
                        content_origin.y + logical_pos.y,
                    ),
                )
            }
        })
        .collect();

    // Reconcile InlineFlow across the IFC's render-run-groups every pass: persist
    // each group's lines on its run-start key (the top-level run-start + one per
    // converged `position:relative`/`sticky` inline sub-flow), then clear stale
    // flows on every candidate key not persisted. `layout_generation` is constant 0
    // off the paged path, so this is an explicit reconcile (insert-or-remove), not
    // a generation comparison — what prevents render consuming a stale flow after a
    // realignment, a relpos→static transition, an abspos toggle, or a now-gated run.
    let first_baseline = packer.first_baseline;
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
        for (group_key, group_lines) in packer.flow_lines {
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
                    static_atomic_reposition_records(&lines, &unoffset_origins)
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
                carrier_atomics.extend(static_atomic_reposition_records(&lines, &unoffset_origins));
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
            &packer.relpos_atomic_placements,
            inline_origin,
            block_origin,
            &unoffset_origins,
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
        clear_inline_flows(dom, &candidate_keys, &persisted_keys);
    }

    InlineLayoutResult {
        height: total_block,
        static_positions,
        first_baseline,
        line_count,
        break_after_line,
    }
}

/// Reposition an atomic inline's `LayoutBox` (and its descendants) from the
/// `content_origin` placement `layout_atomic_items` gave it to its packed on-line
/// position. `inline_abs`/`block_abs` are the absolute (writing-mode-folded) inline
/// start and line block-start; the atomic's margin-box origin is moved there so
/// render — which paints the atomic by `walk()`-ing it at its `LayoutBox` — sees
/// the correct rect (CSS 2 §9.2.2 inline-level box at its IFC position). Descendants
/// shift rigidly with the box (`shift_descendants`), the same operation relative
/// positioning uses (`elidex-layout::layout` apply-relative-offset). Block-axis is
/// the line top (baseline-naive; `vertical-align` within the line box is deferred).
///
/// `unoffset_origin` is the atomic's margin-box origin BEFORE any relative offset
/// (captured from the un-offset `LayoutBox` `layout_atomic_items` returned) and is
/// the delta basis: `delta = target − unoffset_origin`. For a **static**/**sticky**
/// atomic (no offset baked) this equals `target − current box origin` → the box
/// lands exactly at `target` (identical to slice 3p-a). For a **relative** atomic
/// the current box already carries the baked `apply_relative_offset`, so the box
/// lands at `target + offset` — the relative offset is preserved, not stripped
/// (slice 3p-b-2). Using the captured un-offset origin (NOT `content_origin`) keeps
/// a vertical-rl asymmetric box correct (its un-offset origin ≠ `content_origin`).
/// `None` (atomic absent from the map — should not happen for a laid-out atomic)
/// skips the reposition.
///
/// Exposed `pub` so the multicol definitive seam (`position_column_fragments`) can
/// reuse the SAME reposition for a **mid-break** atomic — there the basis is carried
/// out via the per-column [`ColumnFlowSlice`] carrier (terminal-Z C-2), so a static
/// or relpos atomic logically in column *i* lands at its per-column on-line position
/// (the folded `inline_start` already carries the column inline offset → the box is
/// born-absolute and is pruned from the generic column shift, see the multicol
/// `exclude_subtrees` walk).
pub fn reposition_atomic_box(
    dom: &mut EcsDom,
    atomic: Entity,
    inline_abs: f32,
    block_abs: f32,
    is_vertical: bool,
    unoffset_origin: Option<Point>,
    is_probe: bool,
) {
    let Some(unoffset_origin) = unoffset_origin else {
        return;
    };
    // `Some(unoffset_origin)` ⟹ the atomic was laid out by `layout_atomic_items`
    // (which inserts the map entry and the `LayoutBox` together), so the box exists;
    // the final `content.origin` write is `if let Ok`-guarded regardless, so no
    // separate box-existence guard is needed here.
    // inline-axis → physical x (horizontal) or y (vertical); block-axis → the other.
    let target = if is_vertical {
        Point::new(block_abs, inline_abs)
    } else {
        Point::new(inline_abs, block_abs)
    };
    let delta = target - unoffset_origin;
    if delta.x.abs() <= f32::EPSILON && delta.y.abs() <= f32::EPSILON {
        return;
    }
    let children = dom.composed_children(atomic);
    crate::block::shift_descendants(dom, &children, delta, is_probe);
    if let Some(mut lb) = dom.layout_box_mut(atomic) {
        lb.content.origin += delta;
    }
}

/// Build the atomic-reposition records for the **static** atomics (`AtomicBox` flow
/// members) in `lines`: `(entity, inline_abs, block_abs, basis)` at the lines' own
/// (IFC-absolute, column-0 base) coords — the on-line inline target is the run's
/// `inline_start`, the block target the line's `block_start`, and the delta basis the
/// atomic's un-offset margin-box origin. An atomic absent from `unoffset_origins` is
/// skipped (its reposition would no-op on a `None` basis anyway). This is the SINGLE
/// derivation shared by both the `persist_flow` sink (reposition immediately) and the
/// `do_carrier` sink (carry to the multicol seam) — one record shape, one place to
/// change how a static atomic's target is computed.
fn static_atomic_reposition_records(
    lines: &[elidex_ecs::InlineFlowLine],
    unoffset_origins: &HashMap<Entity, Point>,
) -> Vec<(Entity, f32, f32, Point)> {
    let mut records = Vec::new();
    for line in lines {
        for run in &line.runs {
            if let InlineFlowRun::AtomicBox {
                entity: atomic,
                inline_start,
            } = run
            {
                if let Some(basis) = unoffset_origins.get(atomic).copied() {
                    records.push((*atomic, *inline_start, line.block_start, basis));
                }
            }
        }
    }
    records
}

/// Build the atomic-reposition records for the **relpos/sticky** atomics (NOT flow
/// members) from the packer's IFC-root-local `placements`, folded to IFC-absolute
/// with `(inline_origin, block_origin)`. Same `(entity, inline_abs, block_abs, basis)`
/// shape as [`static_atomic_reposition_records`] so both atomic kinds reposition
/// uniformly through the same sink (immediate or carried).
fn relpos_atomic_reposition_records(
    placements: &[(Entity, f32, f32)],
    inline_origin: f32,
    block_origin: f32,
    unoffset_origins: &HashMap<Entity, Point>,
) -> Vec<(Entity, f32, f32, Point)> {
    placements
        .iter()
        .filter_map(|&(atomic, inline_local, block_local)| {
            unoffset_origins.get(&atomic).copied().map(|basis| {
                (
                    atomic,
                    inline_local + inline_origin,
                    block_local + block_origin,
                    basis,
                )
            })
        })
        .collect()
}

/// Remove stale [`InlineFlow`] components for an IFC: clear it from every candidate
/// key that was not persisted this pass. `candidates` is a superset of every entity
/// that could have carried this IFC's flow in any prior pass (every run-parent's raw
/// direct children); `persisted` is the set just written. `remove_one` on an entity
/// without the component is a cheap no-op. This is the single staleness reconciler
/// (F9 — `layout_generation` is constant 0 non-paged, so removal, not comparison).
/// See the reconcile comment in `layout_inline_context_fragmented`.
fn clear_inline_flows(
    dom: &mut EcsDom,
    candidates: &[Entity],
    persisted: &std::collections::HashSet<Entity>,
) {
    for &c in candidates {
        if !persisted.contains(&c) {
            let _ = dom.world_mut().remove_one::<InlineFlow>(c);
        }
    }
}
