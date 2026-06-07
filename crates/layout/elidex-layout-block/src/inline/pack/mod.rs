//! Line packing: `LinePacker`, `PackItem`, `build_pack_items`, `assign_inline_layout_boxes`.
//!
//! The `text-align`/`text-align: justify` math (`FlushReason`, alignment resolution,
//! justification opportunity counting + baking) lives in the [`justify`] submodule.

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity, InlineFlowLine, InlineFlowRun};
use elidex_plugin::{
    ComputedStyle, Direction, EdgeSizes, LayoutBox, Point, Rect, TextAlign, WhiteSpace,
};
use elidex_text::{measure_text, BreakOpportunity, FontDatabase};

use super::measure::measure_segment_widths;
use super::InlineItem;

mod justify;
use justify::{align_offset, bake_justify, justify_opportunity_counts, resolve_align, FlushReason};

/// Inline-alignment context for persisting an [`InlineFlow`](elidex_ecs::InlineFlow).
///
/// Present (`Some`) when the run is persistable (gated in — see
/// `layout_inline_context_fragmented`); `None` skips all flow recording. When present,
/// `LinePacker` records per-line positioned text runs with `text-align` baked into
/// each run's `inline_start` — **including `justify`** (the 4th alignment; CSS Text 3
/// §6), whose between-run expansion is baked here and whose within-run amount is
/// persisted per line as [`InlineFlowLine::justify_word_spacing`](elidex_ecs::InlineFlowLine).
/// Writing-mode-agnostic: the resolution operates on the **inline axis**
/// (`Left/Start → inline-start`, `Right/End → inline-end`, `Center → mid`), and
/// `containing_inline_size` is the inline-axis extent (content width for horizontal,
/// height for vertical), so the same logic positions both horizontal and vertical text
/// along their inline axis.
#[derive(Clone, Copy)]
pub(super) struct FlowAlign {
    pub text_align: TextAlign,
    pub direction: Direction,
    pub containing_inline_size: f32,
    /// The top-level run-group key (`first_eligible_child` of the IFC parent — render's
    /// `run[0]`). `text-align: justify` *distributes* free space within the **top-level
    /// group's** word-separators only (it is the group whose `InlineFlowLine` carries
    /// `justify_word_spacing > 0`). Converged `position:relative`/`sticky` sub-flow
    /// groups (keyed differently) do NOT justify internally — `justify_word_spacing = 0`
    /// — since per-flow justify of a sparse sub-flow over the parent's full
    /// `containing_inline_size` would over-stretch it (that line-unified case is slot
    /// `#11-justify-subflow-line-unified`). They ARE, however, shifted by the cumulative
    /// top-level justify expansion at their inline position (`flush_line`'s `cum_at`), so
    /// positioned content rides the same justification as the surrounding text and does
    /// not overlap it. `None` when the top-level run is itself unrecorded (no
    /// justification target).
    pub top_level_key: Option<Entity>,
    /// Whether the IFC's writing mode is vertical — read in `flush_line` to suppress
    /// `text-align: justify` in vertical writing modes. (CSS Text 3 §6 justifies along
    /// the *inline* axis, which is vertical here, so vertical justify IS spec-valid;
    /// implementing it needs word-spacing in render's vertical glyph path
    /// (`place_glyphs_vertical`, which has none) + a vertical bake — tracked as slot
    /// `#11-justify-vertical-writing-mode`. Until then a vertical line stays
    /// start-aligned with `justify_word_spacing = 0`, matching legacy [which never
    /// justified vertical text], so persisting it is non-regression.) Lives here — not on
    /// `LinePacker` — because its only consumer is justify resolution, gated on a `Some`
    /// `FlowAlign` (an alignment-resolution input like `text_align`).
    pub is_vertical: bool,
}

// ---------------------------------------------------------------------------
// Break segment — a piece of a StyledRun between break opportunities
// ---------------------------------------------------------------------------

/// A piece of inline content for line packing — either a text segment or an atomic box.
pub(super) enum PackItem {
    /// Text segment between break opportunities.
    Text {
        /// Index into the items array for style lookup.
        item_index: usize,
        /// The text of this segment.
        text: String,
        /// Break opportunity at the end of this segment, if any.
        break_after: Option<BreakOpportunity>,
    },
    /// An atomic inline box (no break inside).
    Atomic {
        /// Index into the items array.
        item_index: usize,
    },
    /// Absolutely positioned placeholder — records static position, zero-width.
    Placeholder { entity: Entity },
}

/// What `place_item` records for a placed item (only when persisting —
/// `flow_align.is_some()`). A text segment coalesces into a contiguous same-entity
/// [`InlineFlowRun::Text`]; a *static* atomic inline-level box becomes its own
/// [`InlineFlowRun::AtomicBox`] flow member (render `walk()`s it at its
/// repositioned `LayoutBox`); a *positioned* (relative/sticky) atomic is recorded
/// in the separate `relpos_atomic_placements` reposition bucket — NOT a flow
/// member, because render paints it in Layer 6 and a flow member would double-paint
/// (`emit_inline_flow` walks every member in Layer 5 too; slice 3p-b-2).
#[derive(Clone, Copy)]
enum FlowMember<'a> {
    /// A text segment that contributes its `&str` to a text run.
    Text(&'a str),
    /// A static atomic inline-level box (no text; carries its own box geometry).
    Atomic,
    /// A `position:relative`/`sticky` atomic inline-level box: in-flow (advances the
    /// cursor) but painted in render Layer 6 from its own `LayoutBox`. Recorded in
    /// the reposition bucket, not a flow member (avoids double-paint, slice 3p-b-2).
    PositionedAtomic,
}

/// Build pack items from inline items.
///
/// Text runs are split at break opportunities. Atomic boxes become single pack items.
pub(super) fn build_pack_items(items: &[InlineItem]) -> Vec<PackItem> {
    use elidex_text::find_break_opportunities;
    let mut pack_items = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        match item {
            InlineItem::Text(run) => {
                if run.text.is_empty() {
                    continue;
                }
                let breaks = find_break_opportunities(&run.text);
                let mut prev_pos = 0;
                for &(bp, kind) in &breaks {
                    if bp > prev_pos {
                        pack_items.push(PackItem::Text {
                            item_index: idx,
                            text: run.text[prev_pos..bp].to_string(),
                            break_after: Some(kind),
                        });
                    }
                    prev_pos = bp;
                }
                if prev_pos < run.text.len() {
                    pack_items.push(PackItem::Text {
                        item_index: idx,
                        text: run.text[prev_pos..].to_string(),
                        break_after: None,
                    });
                }
            }
            InlineItem::Atomic { .. } => {
                pack_items.push(PackItem::Atomic { item_index: idx });
            }
            InlineItem::Placeholder(entity) => {
                pack_items.push(PackItem::Placeholder { entity: *entity });
            }
        }
    }
    pack_items
}

// ---------------------------------------------------------------------------
// Line box — a positioned line within an inline formatting context
// ---------------------------------------------------------------------------

/// A positioned line box produced during inline layout.
pub(super) struct LineBox {
    /// Block-axis size (height for horizontal writing mode).
    pub block_size: f32,
}

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
pub(super) struct EntityBounds {
    /// Inline-axis start on the first line.
    inline_start: f32,
    /// Inline-axis end on the last line.
    inline_end: f32,
    /// Block-axis offset of the first line.
    block_start: f32,
    /// Block-axis offset + size of the last line.
    block_end: f32,
    /// Per-line rectangles for `getClientRects()`.
    pub line_rects: Vec<InlineLineRect>,
}

// ---------------------------------------------------------------------------
// Line packer
// ---------------------------------------------------------------------------

/// Line packer state — extracted to keep the main function under the line limit.
pub(super) struct LinePacker {
    pub line_boxes: Vec<LineBox>,
    pub entity_bounds: HashMap<Entity, EntityBounds>,
    /// Static positions for absolutely positioned placeholders (CSS 2.1 §10.6.5).
    /// Stored as `Point(inline_pos, block_pos)` in logical coordinates.
    pub static_positions: HashMap<Entity, Point>,
    current_inline: f32,
    current_line_height: f32,
    current_block_offset: f32,
    on_line: bool,
    /// Whether the current line has emitted any rendered content — a non-collapsible
    /// character (a rendered glyph, including a zero-advance one such as U+200B or a
    /// no-break space), an atomic inline box, or a forced break. The test is
    /// content presence, not advance width. A line whose content is only collapsible
    /// white space (which collapses away) generates no box and contributes zero
    /// block size (CSS 2 §9.2.2.1 / §9.2.1.1).
    any_rendered_content: bool,
    /// Per-entity rectangles tentatively collected for the current line. Committed
    /// into `entity_bounds` only when the line is flushed with rendered content;
    /// discarded if the line is suppressed (collapsible-whitespace-only), so a
    /// whitespace-only inline element does not get a phantom `getClientRects()`
    /// rectangle on a line that generates no box.
    current_line_entity_rects: Vec<(Entity, InlineLineRect)>,
    parent_entity: Entity,
    /// First baseline offset from the inline formatting context top.
    /// Captured from the first text run on the first line.
    pub first_baseline: Option<f32>,
    /// When `Some`, persist collapsed + positioned runs into `flow_lines` (the
    /// `InlineFlow` source, consumed by render). `None` skips recording.
    flow_align: Option<FlowAlign>,
    /// Per-line positioned members tentatively collected for the current line,
    /// **bucketed by render-run-group key** (the run-start `run[0]` each group's
    /// `InlineFlow` persists on — the top-level run-start or a `position:relative`/
    /// `sticky` inline's sub-flow key; see `StyledRun::group_key`). Committed into
    /// `flow_lines` on a rendered-content flush (the same commit-on-content seam as
    /// `current_line_entity_rects`), discarded on a suppressed line. The line's
    /// `text-align` offset is shared across all groups on the line (line-level).
    current_line_runs: HashMap<Entity, Vec<InlineFlowRun>>,
    /// Entity of the most recent flow member placed on the current line (text or
    /// atomic), or `None` at line start. Text break-pieces coalesce into one run
    /// only when **immediately** preceded by the same entity — with per-group
    /// buckets, a group's `last` run is no longer the globally-last placement, so a
    /// same-entity run separated by an intervening positioned-sub-flow member (e.g.
    /// `a` and `c` in `a<span rel>b</span>c`, both the parent's text) must NOT merge
    /// across the gap. This tracks the contiguity the single-Vec model got for free.
    last_placed_entity: Option<Entity>,
    /// Trailing collapsible-whitespace "hang" of the last segment placed on the
    /// current line (`full_width − trimmed_width`), subtracted from the line
    /// advance to get the trimmed line width for `text-align` (CSS Text 3 §4.1.2
    /// trailing spaces do not count toward alignment).
    current_line_last_hang: f32,
    /// Persisted per-line positioned runs in IFC-local coordinates, **bucketed by
    /// render-run-group key**: one entry per group (top-level + each converged
    /// positioned-inline sub-flow). The caller folds each to absolute and persists
    /// one `InlineFlow` per key.
    pub flow_lines: HashMap<Entity, Vec<InlineFlowLine>>,
    /// `position:relative`/`sticky` atomic boxes tentatively placed on the current
    /// line, `(entity, IFC-local inline_start)`. Committed into
    /// `relpos_atomic_placements` on a rendered-content flush (same commit-on-content
    /// seam as `current_line_runs`), discarded on a suppressed line. A **flat
    /// line-level `Vec`, NOT group-keyed** like `current_line_runs`: `text-align` is
    /// one line-level offset shared across all groups, so a positioned atomic's align
    /// shift equals every group's regardless of which (sub-)flow it sits inside.
    current_line_relpos_atomics: Vec<(Entity, f32)>,
    /// Committed `position:relative`/`sticky` atomic placements,
    /// `(entity, IFC-local inline_start, IFC-local block_start)` with the per-line
    /// `text-align` offset already baked into `inline_start`. The `inline_start` is
    /// **IFC-root-local** (an atomic shares the parent IFC's cursor; it never
    /// establishes a sub-flow of its own line geometry), so the caller folds it with
    /// the **IFC-root** `content_origin` — like `static_positions`, NOT a sub-flow
    /// origin — then repositions each box (NOT a flow member; render Layer 6 paints
    /// it from the box). slice 3p-b-2.
    pub relpos_atomic_placements: Vec<(Entity, f32, f32)>,
}

impl LinePacker {
    pub fn new(parent_entity: Entity, flow_align: Option<FlowAlign>) -> Self {
        Self {
            line_boxes: Vec::new(),
            entity_bounds: HashMap::new(),
            static_positions: HashMap::new(),
            current_inline: 0.0,
            current_line_height: 0.0,
            current_block_offset: 0.0,
            on_line: false,
            any_rendered_content: false,
            current_line_entity_rects: Vec::new(),
            parent_entity,
            first_baseline: None,
            flow_align,
            current_line_runs: HashMap::new(),
            last_placed_entity: None,
            current_line_last_hang: 0.0,
            flow_lines: HashMap::new(),
            current_line_relpos_atomics: Vec::new(),
            relpos_atomic_placements: Vec::new(),
        }
    }

    /// Emit the current line as a line box, then reset line state. A line with no
    /// rendered content — only collapsible white space that collapses away —
    /// generates no box at all (CSS 2 §9.2.2.1 / §9.2.1.1): it is not pushed and
    /// does not advance the block cursor, so it does not skew `line_count` or
    /// fragmentation.
    // A cohesive per-line commit: line box, entity-rect bounds, and the per-group
    // text-align/justify flow-line bake (incl. cross-flow `cum_at` positioning) — long
    // like the sibling `pack` / `layout_inline_context_fragmented`. (The justify math
    // helpers it calls live in the `justify` submodule.)
    #[allow(clippy::too_many_lines)]
    fn flush_line(&mut self, reason: FlushReason) {
        if self.any_rendered_content {
            self.line_boxes.push(LineBox {
                block_size: self.current_line_height,
            });
            // Commit this line's inline-element rectangles into entity_bounds (→ each
            // inline element's LayoutBox / getClientRects geometry). The commit is done
            // inside the alignment arms below: when persisting (`flow_align.is_some()`)
            // the rects ride the SAME per-line natural→painted transform as the runs
            // (`offset` + justify `cum_at`), merged to one box fragment per line per
            // element (`commit_aligned_entity_rects`); the non-persisting fragmentation
            // path keeps the natural per-segment commit (path 2 — deferred, the `else`
            // arm). Each fragment takes the line's *final* height (it may have grown after
            // the fragment was placed); per-entity bounds union all committed fragments (a
            // multi-line inline's box must enclose every line).
            let line_height = self.current_line_height;
            // Commit the line's positioned members into flow_lines, per group,
            // baking the per-line text-align offset into each run's inline_start.
            // The offset is line-level (computed once from the whole line's trimmed
            // width — excluding the trailing collapsible-whitespace hang of the last
            // segment, CSS Text 3 §4.1.2), so every group on the line shifts by the
            // same amount. Each group with runs appends one InlineFlowLine to its own
            // flow_lines bucket.
            if let Some(fa) = self.flow_align {
                // Trimmed line width / free space — trailing collapsible spaces hang and
                // count toward neither alignment NOR justification (CSS Text 3 §4.1.2).
                let line_width = self.current_inline - self.current_line_last_hang;
                let free = (fa.containing_inline_size - line_width).max(0.0);
                let block_start = self.current_block_offset;
                let block_size = self.current_line_height;
                // `text-align: justify` (CSS Text 3 §6.4) is suppressed on the block's
                // last line / a forced-break line (§6.3/§6.1 → start-aligned) and in
                // vertical writing modes (pre-existing render-capability limit — the
                // vertical path lacks word-spacing; see `is_vertical`), matching legacy.
                // Only the TOP-LEVEL run group *distributes* free space within itself
                // (`justify_word_spacing > 0`); a sub-flow group does NOT justify
                // internally (per-flow justify of a sparse sub-flow over the parent's
                // full width would over-stretch it — that line-unified case is slot
                // `#11-justify-subflow-line-unified`). But sub-flow groups + positioned
                // atomics ARE shifted by the cumulative top-level expansion at their
                // inline position (`cum_at` below), so positioned content rides the same
                // justification as the surrounding text and never overlaps it.
                let justify_eligible = fa.text_align == TextAlign::Justify
                    && !fa.is_vertical
                    && reason == FlushReason::SoftWrap;
                // Per-run justification opportunities of the top-level group (interior
                // word-separators; the trailing collapsible hang of the last run is
                // excluded, §4.1.2). A justify line with zero opportunities (a single
                // word, or a word before an inline-block that forced the wrap) is
                // *unexpandable* — §6.4.3 falls back to `text-align-last` = start.
                // Computed before the drain so it drives BOTH the offset and the bake.
                // Is the top-level group the line's inline-last content? (nothing — a
                // sub-flow run or a positioned atomic — sits inline-after it). Only then
                // does its last text run's trailing space hang; otherwise that space
                // precedes more inline-level content and stays an opportunity.
                let top_group_is_line_last = justify_eligible && {
                    let top_last = fa
                        .top_level_key
                        .and_then(|k| self.current_line_runs.get(&k))
                        .and_then(|r| r.last())
                        .map_or(f32::NEG_INFINITY, InlineFlowRun::inline_start);
                    let others_max = self
                        .current_line_runs
                        .iter()
                        .filter(|(k, _)| Some(**k) != fa.top_level_key)
                        .flat_map(|(_, r)| r.iter())
                        .map(InlineFlowRun::inline_start)
                        .chain(self.current_line_relpos_atomics.iter().map(|(_, s)| *s))
                        .fold(f32::NEG_INFINITY, f32::max);
                    top_last >= others_max
                };
                let top_run_opportunities = justify_eligible
                    .then(|| {
                        fa.top_level_key
                            .and_then(|k| self.current_line_runs.get(&k))
                    })
                    .flatten()
                    .map(|runs| justify_opportunity_counts(runs, top_group_is_line_last));
                let distribute = top_run_opportunities
                    .as_ref()
                    .is_some_and(|c| c.iter().sum::<usize>() > 0);
                // Line-level start/center/end offset, baked into every run's
                // `inline_start`. A *distributed* justify line resolves to `Justify` →
                // `align_offset` 0 (it fills the box from the start edge; `bake_justify`
                // does the distribution). Any other justify line — suppressed
                // (last/forced/vertical) OR unexpandable (zero opportunities) — is
                // start-aligned per §6.3/§6.4.3 (`text-align-last: auto` → `start`), NOT
                // left-pinned, so an RTL such line lands on the right (start) edge.
                let offset_align = if fa.text_align == TextAlign::Justify && !distribute {
                    resolve_align(TextAlign::Start, fa.direction)
                } else {
                    resolve_align(fa.text_align, fa.direction)
                };
                let offset = align_offset(offset_align, free);
                // Cross-flow justify coordination. When distributing, capture the
                // top-level group's (natural `inline_start`, opportunity count) pairs and
                // `extra = free / Σ opportunities`. `cum_at(pos)` is the cumulative
                // expansion of all top-level opportunities *before* `pos` — the same `cum`
                // `bake_justify` accrues for the top-level runs, exposed position-keyed so
                // sub-flow groups + positioned atomics ride the identical justification
                // (they shift right with the surrounding text instead of staying at their
                // natural cursor and being overlapped). `None` (non-distributing line) →
                // `cum_at` is 0 and every group gets only the line-level `offset`.
                let justify_dist = distribute.then(|| {
                    let per_run = top_run_opportunities.as_ref().expect("distribute ⇒ Some");
                    #[allow(clippy::cast_precision_loss)] // opportunity count is small
                    let extra = free / per_run.iter().sum::<usize>() as f32;
                    let positions: Vec<(f32, usize)> = fa
                        .top_level_key
                        .and_then(|k| self.current_line_runs.get(&k))
                        .map(|runs| {
                            runs.iter()
                                .map(InlineFlowRun::inline_start)
                                .zip(per_run.iter().copied())
                                .collect()
                        })
                        .unwrap_or_default();
                    (extra, positions)
                });
                let cum_at = |pos: f32| -> f32 {
                    justify_dist.as_ref().map_or(0.0, |(extra, positions)| {
                        #[allow(clippy::cast_precision_loss)]
                        let seps = positions
                            .iter()
                            .filter(|(start, _)| *start < pos)
                            .map(|(_, n)| *n)
                            .sum::<usize>() as f32;
                        extra * seps
                    })
                };
                let extra = justify_dist.as_ref().map(|(e, _)| *e);
                // Commit this line's inline-element rects (getClientRects / LayoutBox) in
                // painted coords — the 4th consumer of the same per-line natural→painted
                // transform the runs (`bake_justify`) and relpos placements use (`offset`
                // + `cum_at`), merged to one box fragment per line per element.
                self.commit_aligned_entity_rects(line_height, offset, cum_at);
                for (group_key, mut runs) in self.current_line_runs.drain() {
                    if runs.is_empty() {
                        continue;
                    }
                    let justify_word_spacing = if let (true, Some(e)) =
                        (Some(group_key) == fa.top_level_key, extra)
                    {
                        // Top-level distributing group: line offset + `bake_justify`'s
                        // sequential `cum` (== `cum_at` at each run's position). jws = extra.
                        for r in &mut runs {
                            *r.inline_start_mut() += offset;
                        }
                        bake_justify(&mut runs, e, top_run_opportunities.as_ref().expect("Some"));
                        e
                    } else {
                        // Sub-flow group (or any non-distributing group): line offset +
                        // the cumulative top-level expansion at the run's natural position,
                        // so it rides the justification (no overlap). No internal justify.
                        for r in &mut runs {
                            let shift = offset + cum_at(r.inline_start());
                            *r.inline_start_mut() += shift;
                        }
                        0.0
                    };
                    self.flow_lines
                        .entry(group_key)
                        .or_default()
                        .push(InlineFlowLine {
                            block_start,
                            block_size,
                            runs,
                            justify_word_spacing,
                        });
                }
                // Commit the line's positioned-atomic placements (relpos/sticky): line
                // `offset` + the cumulative top-level justify expansion at the atomic's
                // natural position (`cum_at`), so the box rides the same justification as
                // the surrounding text (slice 3p-b-2 + justify). A flat list, not
                // group-keyed: the caller folds with the IFC-root origin and repositions
                // each box; these are NOT flow members (render Layer 6 paints the
                // positioned box, so a flow member would double-paint).
                for (entity, inline_start) in self.current_line_relpos_atomics.drain(..) {
                    self.relpos_atomic_placements.push((
                        entity,
                        inline_start + offset + cum_at(inline_start),
                        block_start,
                    ));
                }
            } else {
                // flow_align == None: non-persisting fragmentation (block/flex/grid/table —
                // path 2). The packer carries no alignment info here (it lives only in
                // `FlowAlign`), so commit entity rects at the natural cursor, per break
                // segment, unmerged, as before. Aligning these too needs un-gating
                // alignment knowledge from run persistence — deferred slot
                // `#11-inline-align-clientrects-nonpersist-path` (re-evaluate at Z-slice
                // landing, which deletes the non-persist paths).
                for (entity, mut rect) in self.current_line_entity_rects.drain(..) {
                    rect.block_size = line_height;
                    let line_block_end = rect.block_start + rect.block_size;
                    self.entity_bounds
                        .entry(entity)
                        .and_modify(|b| {
                            b.inline_start = b.inline_start.min(rect.inline_start);
                            b.inline_end = b.inline_end.max(rect.inline_end);
                            b.block_start = b.block_start.min(rect.block_start);
                            b.block_end = b.block_end.max(line_block_end);
                            b.line_rects.push(rect.clone());
                        })
                        .or_insert(EntityBounds {
                            inline_start: rect.inline_start,
                            inline_end: rect.inline_end,
                            block_start: rect.block_start,
                            block_end: line_block_end,
                            line_rects: vec![rect],
                        });
                }
            }
            self.current_block_offset += self.current_line_height;
        } else {
            // Suppressed line (collapsible whitespace only): discard its rects so no
            // phantom getClientRects geometry is produced (CSS 2 §9.2.2.1). Likewise
            // discard the tentative flow runs + relpos-atomic placements (no box on a
            // no-box line, so nothing to reposition).
            self.current_line_entity_rects.clear();
            self.current_line_runs.clear();
            self.current_line_relpos_atomics.clear();
        }
        self.current_inline = 0.0;
        self.current_line_height = 0.0;
        self.current_line_last_hang = 0.0;
        self.any_rendered_content = false;
        // New line starts a fresh run even for the same entity (its buckets were
        // drained/cleared above, but reset explicitly so coalescing can't reach
        // across the line break).
        self.last_placed_entity = None;
    }

    /// Commit this line's inline-element rectangles into `entity_bounds` in PAINTED
    /// coordinates, merged to one box fragment per line per element.
    ///
    /// `getClientRects()` (CSSOM VIEW 1 §6 Extensions to the Element Interface) returns
    /// one border-box fragment per line per inline element, in painted coordinates. The
    /// per-break-segment rects `place_item` collected at the natural cursor are folded
    /// per entity into a single `[min start, max end]` line rect, then mapped
    /// natural→painted by the SAME per-line transform the persisted runs / positioned
    /// atomics use: `offset` (the line-level `text-align` shift) plus `cum_at` (CSS Text 3
    /// §6.4.1 Expanding and Compressing Text — the cumulative inter-word justify expansion
    /// to the left of a position; the trailing collapsible-space hang is already excluded
    /// from `cum_at`, §4.1.2). This is
    /// the 4th consumer of that one transform — runs (via [`bake_justify`]), sub-flow
    /// runs, and positioned atomics being the other three (one-issue-one-way: a single
    /// natural→painted map, not a 4th ad-hoc shift). For non-justify lines `cum_at ≡ 0`,
    /// so the rect is a uniform per-line translation by `offset` (correct for
    /// center/right); for start-aligned lines it is the identity.
    ///
    /// Folding *before* the transform makes it exact at element granularity:
    /// `cum_at(max_end)` counts the element's own interior separators (the element's run
    /// start `< max_end`), so the rect widens by its internal justify expansion, while
    /// `cum_at(min_start)` counts only separators to the element's left. The union into
    /// `entity_bounds` therefore runs over merged rects only (one per line) — a
    /// multi-segment line contributes exactly one `line_rects` entry, matching the CSSOM
    /// one-fragment-per-line shape and keeping the single-line fallback (`len() == 1` →
    /// `LayoutBox` border box in `getClientRects`).
    fn commit_aligned_entity_rects(
        &mut self,
        line_height: f32,
        offset: f32,
        cum_at: impl Fn(f32) -> f32,
    ) {
        // Fold this line's per-break-segment rects into one (min_start, max_end,
        // block_start) per entity. All segments on a line share `block_start` (the
        // `current_block_offset` snapshot at place time), so any segment's is the line's.
        // A small Vec (not a HashMap) keeps deterministic order and is cheap — at most a
        // handful of distinct inline elements per line.
        let mut merged: Vec<(Entity, f32, f32, f32)> = Vec::new();
        for (entity, rect) in self.current_line_entity_rects.drain(..) {
            if let Some(m) = merged.iter_mut().find(|(e, ..)| *e == entity) {
                m.1 = m.1.min(rect.inline_start);
                m.2 = m.2.max(rect.inline_end);
            } else {
                merged.push((entity, rect.inline_start, rect.inline_end, rect.block_start));
            }
        }
        for (entity, min_start, max_end, block_start) in merged {
            let painted = InlineLineRect {
                inline_start: min_start + offset + cum_at(min_start),
                inline_end: max_end + offset + cum_at(max_end),
                block_start,
                block_size: line_height,
            };
            let line_block_end = painted.block_start + painted.block_size;
            self.entity_bounds
                .entry(entity)
                .and_modify(|b| {
                    b.inline_start = b.inline_start.min(painted.inline_start);
                    b.inline_end = b.inline_end.max(painted.inline_end);
                    b.block_start = b.block_start.min(painted.block_start);
                    b.block_end = b.block_end.max(line_block_end);
                    b.line_rects.push(painted.clone());
                })
                .or_insert(EntityBounds {
                    inline_start: painted.inline_start,
                    inline_end: painted.inline_end,
                    block_start: painted.block_start,
                    block_end: line_block_end,
                    line_rects: vec![painted],
                });
        }
    }

    // A per-PackItem dispatch (Text / Atomic / Placeholder) with baseline capture +
    // the static-vs-positioned atomic routing inline — naturally long, like the
    // sibling `layout_inline_context_fragmented` / `layout_atomic_items`.
    #[allow(clippy::too_many_lines)]
    pub fn pack(
        &mut self,
        pi: &PackItem,
        items: &[InlineItem],
        dom: &EcsDom,
        font_db: &FontDatabase,
        containing_inline_size: f32,
        is_vertical: bool,
    ) {
        match pi {
            PackItem::Text {
                item_index,
                text,
                break_after,
            } => {
                let InlineItem::Text(run) = &items[*item_index] else {
                    return;
                };
                let fam = run.family_refs();
                let params = run.measure_params(&fam);
                let seg_line_advance = if is_vertical {
                    run.font_size
                } else {
                    run.line_height
                };
                let (seg_width, trimmed_width) = measure_segment_widths(font_db, &params, text);

                // Whether this segment gives the line its height (generates a box).
                // For preserved white-space (`pre`/`pre-wrap`) every non-empty segment
                // is rendered content and occupies a line — including a lone preserved
                // segment break (`"\n"`), whose end-of-text break is filtered out of
                // `find_break_opportunities` so `force_break` never runs (e.g.
                // `<pre>\n</pre>`). For collapsible white-space a segment of only
                // collapsible white space hangs / collapses away and generates no box
                // (CSS 2 §9.2.2.1); rendered content is "has a non-collapsible character
                // after trimming ASCII space/tab", independent of measured advance — a
                // zero-advance glyph (U+200B) or a no-break space (U+00A0) is content.
                let contributes_content =
                    if matches!(run.white_space, WhiteSpace::Pre | WhiteSpace::PreWrap) {
                        !text.is_empty()
                    } else {
                        // Trim only the *collapsible* white space (ASCII space/tab,
                        // the same predicate as the trailing-hang measurement), not
                        // Unicode White_Space: a no-break space (U+00A0) renders and
                        // gives the line height, so a `&nbsp;`-only line must
                        // generate a box.
                        !text
                            .trim_end_matches(super::is_collapsible_space)
                            .is_empty()
                    };

                // Capture first baseline from the first text segment that actually
                // generates a line box (CSS 2.1 §10.8.1: baseline = line_y +
                // half_leading + ascent). Gating on `contributes_content` skips
                // suppressed collapsible-whitespace segments, so the baseline reflects
                // the first rendered line rather than whitespace that generates no box.
                if contributes_content && self.first_baseline.is_none() && !is_vertical {
                    if let Some(metrics) = measure_text(font_db, &params, text) {
                        let em_height = metrics.ascent - metrics.descent;
                        // Guard: em_height can be 0/negative (malformed font metrics) or
                        // line_height can be NaN/inf from bad CSS — sanitize to avoid
                        // propagating NaN into layout geometry.
                        let half_leading = if em_height > 0.0 && run.line_height.is_finite() {
                            (run.line_height - em_height) / 2.0
                        } else {
                            0.0
                        };
                        self.first_baseline =
                            Some(self.current_block_offset + half_leading + metrics.ascent);
                    }
                }

                self.place_item(
                    seg_width,
                    trimmed_width,
                    seg_line_advance,
                    run.entity,
                    containing_inline_size,
                    contributes_content,
                    FlowMember::Text(text),
                    run.group_key,
                );

                if *break_after == Some(BreakOpportunity::Mandatory) {
                    self.force_break();
                }
            }
            PackItem::Atomic { item_index } => {
                let InlineItem::Atomic {
                    entity,
                    inline_size,
                    block_size,
                    group_key,
                    positioned,
                } = &items[*item_index]
                else {
                    return;
                };

                // Capture baseline from atomic box if no text baseline yet.
                // CSS 2.1 §10.8.1: atomic inline boxes use their own first_baseline,
                // or fall back to the margin-box bottom edge.
                if self.first_baseline.is_none() && !is_vertical {
                    if let Ok(child_lb) = dom.world().get::<&LayoutBox>(*entity) {
                        // Fallback: content-bottom + padding.bottom + border.bottom + margin.bottom
                        // (the remaining distance from content top to margin-box bottom).
                        let bl = child_lb.first_baseline.unwrap_or(
                            child_lb.content.size.height
                                + child_lb.padding.bottom
                                + child_lb.border.bottom
                                + child_lb.margin.bottom,
                        );
                        self.first_baseline = Some(
                            self.current_block_offset
                                + child_lb.margin.top
                                + child_lb.border.top
                                + child_lb.padding.top
                                + bl,
                        );
                    }
                }

                // Atomic boxes don't break internally; treat as a single unit. An
                // atomic inline box is always rendered content. When persisting:
                // - a *static* atomic → `place_item` records an `AtomicBox` member in
                //   this atomic's group bucket; render paints it by `walk()`-ing the
                //   entity at the `LayoutBox` layout repositions to the member's
                //   `inline_start` (slice 3p-a). A `None` group key records nothing.
                // - a *positioned* (relpos/sticky) atomic → `FlowMember::PositionedAtomic`
                //   records its on-line position in the flat reposition bucket (NOT a
                //   flow member; render Layer 6 paints it, so a member would
                //   double-paint), and layout repositions its box preserving the
                //   applied relative offset (slice 3p-b-2).
                self.place_item(
                    *inline_size,
                    *inline_size,
                    *block_size,
                    *entity,
                    containing_inline_size,
                    true,
                    if *positioned {
                        FlowMember::PositionedAtomic
                    } else {
                        FlowMember::Atomic
                    },
                    *group_key,
                );
            }
            PackItem::Placeholder { entity } => {
                // CSS 2.1 §10.6.5: record static position at current inline/block position.
                // Zero-width, zero-height — does not advance cursor_x.
                self.static_positions.insert(
                    *entity,
                    Point::new(self.current_inline, self.current_block_offset),
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn place_item(
        &mut self,
        full_width: f32,
        trimmed_width: f32,
        block_advance: f32,
        entity: Entity,
        containing_inline_size: f32,
        contributes_content: bool,
        member: FlowMember<'_>,
        group_key: Option<Entity>,
    ) {
        if self.current_inline + trimmed_width > containing_inline_size && self.on_line {
            self.flush_line(FlushReason::SoftWrap);
        }

        let seg_inline_start = self.current_inline;
        self.current_inline += full_width;
        self.current_line_height = self.current_line_height.max(block_advance);
        self.on_line = true;
        self.any_rendered_content |= contributes_content;
        // Trailing collapsible-whitespace hang of this (now last-placed) segment,
        // used by flush_line for the trimmed line width (text-align).
        self.current_line_last_hang = (full_width - trimmed_width).max(0.0);

        if entity != self.parent_entity {
            // Collect tentatively for the current line; flush_line commits these into
            // entity_bounds only if the line has rendered content (else discards them).
            self.current_line_entity_rects.push((
                entity,
                InlineLineRect {
                    inline_start: seg_inline_start,
                    inline_end: seg_inline_start + full_width,
                    block_start: self.current_block_offset,
                    block_size: self.current_line_height,
                },
            ));
        }

        // Record this placed item when persisting (`flow_align.is_some()`):
        // - PositionedAtomic (relpos/sticky): its on-line position goes into the flat
        //   `current_line_relpos_atomics` reposition bucket — NOT a group flow member
        //   (render Layer 6 paints it; a member would double-paint), so it ignores
        //   `group_key`. Layout repositions its box preserving the offset (3p-b-2).
        // - Text / static Atomic: into the render-run-group bucket, but only with a
        //   group key (a positioned subtree with a direct block child has `None`,
        //   records nothing → render legacy). Text coalesces contiguous same-entity
        //   break-pieces on this line into one run so render shapes whole words rather
        //   than per-break fragments (CSS Text 3 §5.6 Shaping Across Intra-word
        //   Breaks). "Contiguous" = the IMMEDIATELY-previous placed member was the
        //   same entity (`last_placed_entity`) — NOT merely the group bucket's last
        //   run, which with sub-flows can be a same-entity run separated by an
        //   intervening positioned member (the `a … c` gap in `a<span rel>b</span>c`).
        //   Same entity ⟹ same group, so the bucket's last IS that previous run. A
        //   different-entity / post-flush / atomic-interrupted segment starts a fresh
        //   run (an atomic's entity differs from surrounding text, so the contiguity
        //   check breaks naturally). static Atomic: its own AtomicBox member at this
        //   position (render walk()s the entity at its repositioned LayoutBox).
        if self.flow_align.is_some() {
            match member {
                FlowMember::PositionedAtomic => {
                    self.current_line_relpos_atomics
                        .push((entity, seg_inline_start));
                }
                FlowMember::Text(text) => {
                    if let Some(gk) = group_key {
                        let coalesce = self.last_placed_entity == Some(entity);
                        let bucket = self.current_line_runs.entry(gk).or_default();
                        match bucket.last_mut() {
                            Some(InlineFlowRun::Text { text: t, .. }) if coalesce => {
                                t.push_str(text);
                            }
                            _ => bucket.push(InlineFlowRun::Text {
                                entity,
                                text: text.to_string(),
                                inline_start: seg_inline_start,
                            }),
                        }
                    }
                }
                FlowMember::Atomic => {
                    if let Some(gk) = group_key {
                        self.current_line_runs.entry(gk).or_default().push(
                            InlineFlowRun::AtomicBox {
                                entity,
                                inline_start: seg_inline_start,
                            },
                        );
                    }
                }
            }
        }
        // Track the last placed entity for the next member's contiguity check
        // (updated for every placed text/atomic member, recorded or not).
        self.last_placed_entity = Some(entity);
    }

    fn force_break(&mut self) {
        // A forced break (a preserved segment break under `white-space: pre*`, or a
        // `<br>`) always produces a real line, even when blank — e.g. a blank line in
        // `<pre>` has height. With normal/nowrap collapsing, segment breaks are
        // transformed to spaces upstream, so this path is reached only for genuine
        // forced breaks; mark the line as rendered content so it keeps its height.
        self.any_rendered_content = true;
        self.flush_line(FlushReason::Forced);
        self.on_line = false;
    }

    pub fn finish(&mut self) {
        if self.on_line {
            self.flush_line(FlushReason::LastLine);
        }
    }

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

/// Assign `LayoutBox` to inline elements based on their bounding rects.
///
/// Each entity that has a `ComputedStyle` (i.e. is an element, not a text node)
/// and was tracked during line packing receives a `LayoutBox` with its
/// bounding rectangle in layout coordinates.
pub(super) fn assign_inline_layout_boxes(
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

        // Store per-line rects for getClientRects() (CSSOM View §6). A multi-line
        // element exposes one border-box fragment per line; a single-line element
        // exposes its one rect via the LayoutBox border box (the getClientRects
        // fallback). On a relayout where an element that previously wrapped now fits one
        // line, the merge yields a single `line_rects` entry — so REMOVE any stale
        // multi-line `InlineClientRects` left from the prior layout, else getClientRects
        // reads the stale component instead of falling back (`layout_query.rs`).
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
        } else {
            let _ = dom
                .world_mut()
                .remove_one::<elidex_plugin::InlineClientRects>(*entity);
        }
    }
}
