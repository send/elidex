//! The converged inline paint path: consuming a layout-produced [`InlineFlow`].
//!
//! Render no longer re-collects / re-collapses / re-measures the DOM for runs
//! layout already laid out — it walks the persisted lines + positioned members
//! and paints them at their absolute coordinates (honouring per-line line-breaking,
//! per-line `text-align`, and paint-time bidi reorder). The legacy single-line
//! collection path lives in the sibling [`super::inline`] module; the dispatcher
//! [`super::inline::emit_inline_run`] routes between the two on `InlineFlow` presence.

use elidex_ecs::{EcsDom, Entity, InlineFlow, InlineFlowRun, InlineFragment};
use elidex_plugin::transform_math::Perspective;
use elidex_plugin::{
    ComputedStyle, Direction, TextOrientation, TextTransform, Visibility, WritingMode,
};
use elidex_text::FontDatabase;

use crate::display_list::DisplayList;
use crate::font_cache::FontCache;

use super::inline::{
    emit_text_segment, emit_vertical_text_segment, vertical_text_orientation, StyledTextSegment,
};
use super::{bidi_visual_order, walk, PaintContext};

/// Whether a persisted [`InlineFragment`] belongs to the page currently being
/// walked: every fragment off the paged path (`expected == None` — a
/// non-fragmented flow has exactly one), else only the one stamped with this
/// page's generation. The consume gate and the `emit_inline_flow` paint loop
/// share this predicate so they never disagree on which fragments paint (D4).
pub(super) fn fragment_matches_page(frag: &InlineFragment, expected: Option<u32>) -> bool {
    expected.is_none_or(|g| frag.generation == g)
}

/// Consume a layout-produced [`InlineFlow`]: paint each line's members at their
/// absolute, already-projected coordinates. This is the converged path — render no
/// longer re-collects / re-collapses / re-measures the DOM, and (unlike the legacy
/// single-line `emit_styled_segments`) honours layout's per-line line-breaking and
/// per-line `text-align`. Two member kinds:
/// - [`InlineFlowRun::Text`]: shaped + emitted at `inline_start` with the run
///   entity's `ComputedStyle`.
/// - [`InlineFlowRun::AtomicBox`]: an atomic inline-level box, painted by
///   `walk()`-ing the entity (chrome + descendants + its own inner IFC) at the
///   `LayoutBox` layout repositioned to the member's line position. Atomic members
///   are collected during the borrow-scoped text loop and walked **after** the
///   `InlineFlow` borrow drops — `walk()` needs `&mut ctx`, which conflicts with the
///   read borrow; render is read-only on the DOM, so this avoids cloning the flow.
///
/// Layout applied the writing-mode projection at persist, so each scalar holds the
/// absolute physical coordinate for its axis; render reads them without a transform
/// and only branches the per-run glyph emit on `writing_mode` — the IFC parent's,
/// i.e. the writing mode layout used when projecting these coordinates (the caller
/// reads it from the parent, not per member):
/// - **horizontal**: `inline_start` = physical x, `block_start` = physical line top.
/// - **vertical**: `block_start`/`block_size` give the glyph-column center x;
///   `inline_start` = physical y (pen top). `text_orientation` selects the shaping.
///
/// **Bidi (UAX #9 L2)**: layout persists each line's `Text` runs in **logical**
/// order; render owns the paint-time visual reorder (master §4.2 — bidi is a
/// paint-time transform of already-collapsed, already-positioned logical runs; it
/// does not change layout advance). Per line: build the `(text, idx)` adapter from
/// the line's `Text` runs only and ask `bidi_visual_order` for the visual order
/// under the IFC parent's `direction`. The common LTR case is an identity
/// permutation → paint each run at its baked logical `inline_start` (no change). A
/// non-identity line is painted in visual order from the line's visual inline-start
/// (`min(inline_start)` = leftmost baked logical position = the span's left edge,
/// since text-align/justify offset is already baked into every run), the shared
/// cursor advancing by each run's shaped width (a hidden run still advances the
/// cursor — its reserved width is preserved — but paints nothing; see
/// `emit_flow_text_run`). Atomics are NOT in the adapter (Option (c)): they stay
/// collected + `walk()`-painted at their layout-baked `LayoutBox`, so an atomic+bidi
/// line is a net fix over legacy (which flattened atomics to text). LIMITATION: an
/// atomic *between* reordered text runs is not treated as a UAX #9 object-replacement
/// (U+FFFC) member — the text cursor does not reserve its width, so reordered text can
/// paint across the atomic's baked box, and the atomic is not visually repositioned.
/// This is one facet of the deferred **full-UBA bidi-fidelity** program, slot
/// `#11-bidi-full-uba-fidelity` (with paragraph-level cross-line level resolution and
/// cross-sub-flow reorder; larger scope = elidex-bidi object modelling +
/// box-reposition-at-paint). The current path runs the per-segment `analyze_bidi_simple`
/// per persisted line — the same approximation level as the legacy path (master §4.2).
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_inline_flow(
    ctx: &mut PaintContext,
    first: Entity,
    writing_mode: WritingMode,
    text_orientation: TextOrientation,
    direction: Direction,
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let vertical = !writing_mode.is_horizontal();
    let orient = vertical_text_orientation(writing_mode, text_orientation);
    // On the paged path, paint only the fragment(s) belonging to this page
    // (`expected_generation`); off it (`None`), paint every fragment — a
    // non-fragmented flow has exactly one (generation 0). The matching
    // gate mirrors the consume check in `emit_inline_run`.
    let expected = ctx.expected_generation;
    // Atomic members collected here, walked after the `InlineFlow` borrow drops.
    let mut atomics: Vec<Entity> = Vec::new();
    {
        // Copy the shared `&EcsDom` out so reads borrow the DOM (not `ctx`), leaving
        // `ctx.font_db`/`font_cache`/`dl` free for the text emit (disjoint fields).
        let dom = ctx.dom;
        let Ok(flow) = dom.world().get::<&InlineFlow>(first) else {
            return;
        };
        for line in flow
            .fragments
            .iter()
            .filter(|frag| fragment_matches_page(frag, expected))
            .flat_map(|frag| frag.lines.iter())
        {
            // Split the line's runs: `Text` runs (bidi-reordered for paint) vs atomics
            // (collected here, `walk()`-ed after the flow borrow drops — Option (c):
            // atomics are NOT in the reorder adapter, they paint at their baked box).
            let mut text_runs: Vec<(Entity, &str, f32)> = Vec::new();
            for run in &line.runs {
                match run {
                    InlineFlowRun::Text {
                        entity,
                        text,
                        inline_start,
                    } => text_runs.push((*entity, text.as_str(), *inline_start)),
                    InlineFlowRun::AtomicBox { entity, .. } => atomics.push(*entity),
                }
            }
            // UAX #9 L2 visual reorder (logical → visual) of this line's `Text` runs,
            // under the IFC parent's paragraph direction. Fast path: under an **LTR**
            // base, a line with NO strong-RTL character never reorders (every run
            // resolves to the even base level 0), so skip the adapter allocation + bidi
            // analysis and paint logical — the overwhelmingly common case, zero
            // overhead. The fast path is gated on `direction == Ltr` because under an
            // **RTL** base even neutral-only runs (punctuation/spaces) inherit the odd
            // paragraph level and can be reordered by L2, which `text_has_rtl` (strong
            // R/AL/AN only) would miss — so an RTL container always runs bidi. Otherwise
            // build the `(text, idx)` adapter the legacy path also feeds
            // `bidi_visual_order` — from `Text` runs ONLY (atomics excluded → no
            // cross-type index confusion) — and reorder unless the result is still
            // identity (e.g. a single run, or no actual L2 swap).
            let needs_bidi = direction == Direction::Rtl
                || text_runs
                    .iter()
                    .any(|&(_, text, _)| elidex_text::text_has_rtl(text));
            let order = needs_bidi
                .then(|| {
                    let adapter: Vec<(String, usize)> = text_runs
                        .iter()
                        .enumerate()
                        .map(|(i, (_, text, _))| ((*text).to_string(), i))
                        .collect();
                    bidi_visual_order(&adapter, direction)
                })
                .filter(|order| order.iter().enumerate().any(|(i, &vi)| i != vi));
            if let Some(order) = order {
                // The line needs reorder: paint runs in visual order from the line's
                // visual inline-start, the shared cursor advancing by each run's shaped
                // width (exactly the legacy `emit_styled_segments` loop, now per line).
                let line_start = text_runs
                    .iter()
                    .map(|&(_, _, inline_start)| inline_start)
                    .fold(f32::INFINITY, f32::min);
                let mut cursor = line_start;
                for &vi in &order {
                    let (entity, text, _) = text_runs[vi];
                    emit_flow_text_run(
                        dom,
                        entity,
                        text,
                        &mut cursor,
                        vertical,
                        orient,
                        line.block_start,
                        line.block_size,
                        line.justify_word_spacing,
                        ctx.font_db,
                        ctx.font_cache,
                        ctx.dl,
                    );
                }
            } else {
                // Logical order (LTR fast path, single RTL run, or empty line): paint
                // each run at its baked logical `inline_start` — unchanged behavior, no
                // LTR regression. The cursor is reset per run (positions are absolute).
                for &(entity, text, inline_start) in &text_runs {
                    let mut cursor = inline_start;
                    emit_flow_text_run(
                        dom,
                        entity,
                        text,
                        &mut cursor,
                        vertical,
                        orient,
                        line.block_start,
                        line.block_size,
                        line.justify_word_spacing,
                        ctx.font_db,
                        ctx.font_cache,
                        ctx.dl,
                    );
                }
            }
        }
    }
    // Paint each atomic inline-level box by walking it at its (layout-repositioned)
    // `LayoutBox` — the same depth/perspective/in_transform a block child gets
    // (`paint_non_sc` walks block children at `depth + 1`).
    for atomic in atomics {
        walk(ctx, atomic, depth + 1, child_perspective, in_transform);
    }
}

/// Paint one `InlineFlow` `Text` run at `cursor` (the inline-axis pen), advancing it
/// by the run's shaped width. Shared by the converged `emit_inline_flow` identity
/// (per-run baked `inline_start`) and reorder (shared accumulating cursor) branches.
/// `block_start`/`block_size` give the line's cross-axis geometry: horizontal → line
/// top; vertical → glyph-column center x. The run's `text-transform` was applied by
/// layout before measuring, so the persisted `text` is final — paint it verbatim
/// (force the segment transform to `None` so the shared emit path does not
/// re-transform; CSS Text 3 §2.1, render = paint-only).
#[allow(clippy::too_many_arguments)]
fn emit_flow_text_run(
    dom: &EcsDom,
    entity: Entity,
    text: &str,
    cursor: &mut f32,
    vertical: bool,
    orient: TextOrientation,
    block_start: f32,
    block_size: f32,
    justify_word_spacing: f32,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let Ok(style) = dom.world().get::<&ComputedStyle>(entity) else {
        return;
    };
    // visibility: hidden text reserves its advance but is not painted — pass
    // `visible` to the emit so the shared reorder cursor still moves past a hidden
    // run (CSS 2.1 §11.2; the bidi-reorder branch shares one accumulating cursor).
    let visible = style.visibility == Visibility::Visible;
    // Style-only segment: the collapsed text comes from the flow member (`text`),
    // so the segment's own text field is unused.
    let mut seg = StyledTextSegment::from_style(String::new(), &style);
    seg.text_transform = TextTransform::None;
    if vertical {
        // Vertical text is start-aligned (no inter-word justification on the block
        // axis, CSS Text 3 §6.4) — layout always persists `justify_word_spacing = 0`
        // for vertical, so there is no value to thread into the vertical emit.
        let center_x = block_start + block_size / 2.0;
        emit_vertical_text_segment(
            text, &seg, orient, center_x, cursor, visible, font_db, font_cache, dl,
        );
    } else {
        // `text-align: justify` extra advance (CSS Text 3 §6.4), applied by
        // `place_glyphs` at each within-run word-separator cluster. In the identity
        // branch the between-run expansion is already baked into the run's
        // `inline_start` (layout); in the bidi-reorder branch the shared cursor
        // re-accumulates it (the cursor advances by the justify-expanded run width) —
        // either way this one per-line value is all render needs.
        emit_text_segment(
            text,
            &seg,
            cursor,
            block_start,
            justify_word_spacing,
            visible,
            font_db,
            font_cache,
            dl,
        );
    }
}
