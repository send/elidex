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

use std::collections::HashMap;

use elidex_ecs::{
    ColumnFlowSlice, EcsDom, Entity, InlineFlow, InlineFlowRun, PseudoElementMarker, TextContent,
};
#[cfg(test)]
pub(crate) use elidex_plugin::LayoutBox;
use elidex_plugin::{ComputedStyle, Display, Point, Position, TextTransform, WhiteSpace};
#[cfg(test)]
pub(crate) use elidex_text::FontDatabase;
use elidex_text::{
    apply_text_transform, measure_text, to_fontdb_style, FontStyle, TextMeasureParams,
};

use crate::MAX_LAYOUT_DEPTH;

pub use measure::{max_content_inline_size, min_content_inline_size};

// ---------------------------------------------------------------------------
// StyledRun — a segment of text with its originating element's style
// ---------------------------------------------------------------------------

/// An item in an inline formatting context — either text or an atomic inline box.
pub(crate) enum InlineItem {
    /// A text run with per-element style.
    Text(StyledRun),
    /// An atomic inline-level box (e.g. `inline-block`, replaced element).
    /// The entity has already been laid out; its dimensions are used as-is.
    Atomic {
        entity: Entity,
        /// Inline-axis size (width for horizontal).
        inline_size: f32,
        /// Block-axis size (height for horizontal).
        block_size: f32,
        /// Which render-run-group this atomic's `AtomicBox` member persists under
        /// (see [`StyledRun::group_key`]). `None` = not recorded.
        /// Ignored when `positioned` (a positioned atomic is never a flow member).
        group_key: Option<Entity>,
        /// `true` for a `position:relative`/`sticky` atomic. Such an atomic advances
        /// the IFC line cursor (in-flow, CSS 2 §9.4.3) but is painted in render's
        /// Layer 6 from its own `LayoutBox`, so it is NOT recorded as an
        /// [`InlineFlowRun::AtomicBox`] flow member (that would double-paint —
        /// `emit_inline_flow` walks every member in Layer 5 AND Layer 6 walks the
        /// positioned box). Instead `LinePacker` records its on-line position in a
        /// separate per-pass bucket and layout repositions its `LayoutBox`
        /// preserving the applied relative offset (slice 3p-b-2).
        positioned: bool,
    },
    /// Absolutely positioned element placeholder (zero-width, zero-height).
    /// Used to record static position for CSS 2.1 §10.6.5.
    Placeholder(Entity),
}

/// A segment of text within an inline formatting context, preserving the
/// originating element's style for measurement.
pub struct StyledRun {
    /// Entity this text belongs to (element or pseudo-element).
    pub entity: Entity,
    /// The text content.
    pub text: String,
    /// Font families for measurement.
    pub families: Vec<String>,
    /// Font size in px.
    pub font_size: f32,
    /// Font weight (100–900).
    pub font_weight: u16,
    /// Font style (Normal/Italic/Oblique).
    pub font_style: FontStyle,
    /// Letter spacing in px.
    pub letter_spacing: f32,
    /// Word spacing in px.
    pub word_spacing: f32,
    /// Resolved line height in px.
    pub line_height: f32,
    /// CSS `white-space` (drives §4.1.1 collapsing / segment-break handling).
    pub white_space: WhiteSpace,
    /// CSS `text-transform` (CSS Text 3 §2.1). Applied to `text` *after* §4.1.1
    /// collapse and *before* measuring/packing (§2.1.2 order of operations), so
    /// the persisted positions are for the final transformed glyphs and render
    /// paints `text` verbatim (no re-transform).
    pub text_transform: TextTransform,
    /// Which render-run-group this run's `InlineFlow` text member persists under:
    /// the run-start entity render reads `InlineFlow` off (its `run[0]`) for the
    /// `emit_inline_run` that paints this group — the IFC parent's first eligible
    /// child (top-level) or a `position:relative`/`sticky` inline's first eligible
    /// child (its Layer-6 sub-flow). `None` = not recorded into any flow (e.g. a
    /// positioned subtree with a direct block child → anonymous-block-in-inline,
    /// left to render's legacy path; CSS 2 §9.2.1.1).
    pub group_key: Option<Entity>,
}

impl StyledRun {
    /// Create a run from text content and a computed style.
    fn from_style(
        entity: Entity,
        text: String,
        style: &ComputedStyle,
        group_key: Option<Entity>,
    ) -> Self {
        Self {
            entity,
            text,
            families: style.font_family.clone(),
            font_size: style.font_size,
            font_weight: style.font_weight,
            font_style: to_fontdb_style(style.font_style),
            letter_spacing: style.letter_spacing.unwrap_or(0.0),
            word_spacing: style.word_spacing.unwrap_or(0.0),
            line_height: style.line_height.resolve_px(style.font_size),
            white_space: style.white_space,
            text_transform: style.text_transform,
            group_key,
        }
    }

    /// Build `TextMeasureParams` borrowing from the given families slice.
    pub(crate) fn measure_params<'a>(&self, families: &'a [&'a str]) -> TextMeasureParams<'a> {
        TextMeasureParams {
            families,
            font_size: self.font_size,
            weight: self.font_weight,
            style: self.font_style,
            letter_spacing: self.letter_spacing,
            word_spacing: self.word_spacing,
        }
    }

    /// Collect family name references for use with `measure_params`.
    pub(crate) fn family_refs(&self) -> Vec<&str> {
        self.families.iter().map(String::as_str).collect()
    }
}

// ---------------------------------------------------------------------------
// Styled run collection
// ---------------------------------------------------------------------------

/// Returns true if `display` is an atomic inline-level box that establishes
/// its own formatting context (e.g. `inline-block`, `inline-flex`).
fn is_atomic_inline(display: Display) -> bool {
    matches!(
        display,
        Display::InlineBlock | Display::InlineFlex | Display::InlineGrid | Display::InlineTable
    )
}

/// Render's `run[0]` for the inline run rooted at the element whose composed
/// children are `children` — it MUST match `paint_non_sc` / Layer-5 grouping
/// exactly (`crates/core/elidex-render/src/builder/walk.rs`), since layout persists
/// the group's `InlineFlow` on this entity and render reads it off `run.first()`.
/// Render's run builder: skip `is_positioned` children (`position != static` —
/// relpos/sticky/abspos/fixed, painted in another layer, NO flush); FLUSH (end the
/// run) at the first `is_block_child` (a block-level child SPLITS the run); push
/// **everything else** — so the first non-positioned, non-block child is `run[0]`,
/// **including a `display:none` element or a non-styled node (text/comment)** (both
/// generate no box but render still pushes them as run members). Returns `None` if
/// a block precedes any such child (the pre-block run is empty). Mirroring render's
/// predicate verbatim (NOT a stricter "skip display:none/non-text" filter) is what
/// keeps the persist key and `run[0]` in agreement.
fn first_eligible_child(dom: &EcsDom, children: &[Entity]) -> Option<Entity> {
    for &child in children {
        // Borrowed read (no `ComputedStyle` clone like `try_get_style`) — this scan
        // only reads `position`/`display`, in a per-child loop on the inline-collect path.
        match dom.world().get::<&ComputedStyle>(child).ok() {
            // Mirrors render: `is_positioned` (position != static) → skip without
            // flushing; `is_block_level` (render's `is_block_child`, which the box
            // it gets in-flow satisfies) → flush, ending the pre-block run; anything
            // else (incl. `display:none`) → render's `run[0]`.
            Some(style) => {
                if style.position != Position::Static {
                    continue;
                }
                if crate::block::is_block_level(style.display) {
                    return None;
                }
                return Some(child);
            }
            // A non-styled node (text or comment) — render pushes it into the inline
            // run unconditionally (not positioned, not a block child), so it is a
            // valid `run[0]`.
            None => return Some(child),
        }
    }
    None
}

/// Whether any DIRECT composed child of a positioned inline is block-level. Such a
/// subtree is anonymous-block-in-inline (CSS 2 §9.2.1.1): render `paint_non_sc`
/// SPLITS the run on the block (multiple runs + a separate `walk(block)`), so the
/// single-sub-flow-per-positioned-root model would over-collect and double-paint
/// the post-block content. A positioned subtree like this gets **no** sub-flow
/// (its content falls to render's legacy path, fail-safe — the anonymous-block-in-
/// inline feature owns it). Direct children only: a block nested in a *static*
/// inline within the subtree is flow-consumed (flattened) by render, not split, so
/// it stays safe in the sub-flow.
fn has_direct_block_child(dom: &EcsDom, children: &[Entity]) -> bool {
    children.iter().any(|&c| {
        // Borrowed read (no `ComputedStyle` clone) — only `display` is read here.
        dom.world()
            .get::<&ComputedStyle>(c)
            .is_ok_and(|s| crate::block::is_block_level(s.display))
    })
}

/// The render-run-group key a `position:relative`/`sticky` inline (`child`, the
/// sub-flow's run-parent) persists its members under: a per-subtree sub-flow keyed
/// on the subtree's first eligible child (= render's `run[0]` for `walk(child)`).
/// Returns `None` — no sub-flow, members fall to render's legacy path — when the
/// subtree is **not a single linear inline run render can consume** in the IFC
/// root's writing mode. The single boundary (One-issue-one-way: future cases —
/// float-in-positioned, etc. — land here):
/// - a writing mode differing from the IFC root's → layout projects every group
///   with the root's axis, but render reads a sub-flow's axis off the span (its
///   `emit_inline_run` run-parent), so the sub-flow would be transposed (CSS
///   Writing Modes 4 §3.2 would blockify it to inline-block, which gates anyway).
/// - a direct block child → render splits the run (anonymous-block-in-inline, CSS 2
///   §9.2.1.1); the single-sub-flow model would over-collect and double-paint.
///
/// Both the block-split check and the key use **`composed_children_flat`** — the
/// SAME `display:contents`-flattened child list render's `walk(child)` iterates
/// ([walk.rs]) — NOT the raw `composed_children`. Otherwise a `display:contents`
/// first child (or a block nested inside one) would key/gate differently than
/// render's `run[0]`: a key mismatch silently drops to legacy, and a missed
/// block-in-contents would over-collect into a sub-flow render then splits →
/// double-paint. (Members are still collected by the raw recursion under the
/// returned key — the key is what render reads off, so it must match render.)
fn positioned_subflow_key(
    dom: &EcsDom,
    child: Entity,
    style: &ComputedStyle,
    root_horizontal: bool,
) -> Option<Entity> {
    if style.writing_mode.is_horizontal() != root_horizontal {
        return None;
    }
    let flat = crate::composed_children_flat(dom, child);
    if has_direct_block_child(dom, &flat) {
        return None;
    }
    first_eligible_child(dom, &flat)
}

/// Recursively collect inline items (text runs + atomic boxes) from inline children.
///
/// Text nodes produce a run inheriting the nearest ancestor element's style.
/// Inline elements use their own style for their children. `display: none`
/// elements are skipped. Atomic inline-level boxes (`inline-block`, `inline-flex`, etc.)
/// produce placeholder items — they are laid out separately and placed as
/// atomic units in the inline flow. Recursion stops at [`MAX_LAYOUT_DEPTH`].
///
/// Also reports the **candidate-key set** for staleness reconciliation: a superset
/// of every entity that could carry an `InlineFlow` for this IFC in any pass = the
/// raw (unfiltered) direct children of the IFC parent plus the raw direct children
/// of every inline element recursed into (each is some run-parent's direct child,
/// hence a potential `run[0]`). The caller clears `InlineFlow` on candidates it does
/// not persist (see the reconcile in `layout_inline_context_fragmented`).
///
/// The top-level members are tagged with the **realigned** top-level run-start key
/// ([`first_eligible_child`] of `children` — render's Layer-5 `run[0]`, which is NOT
/// `children.first()` when a leading child is positioned), threaded into the walk as
/// the initial group key; the caller persists each group from the packer's buckets.
pub(crate) fn collect_inline_items(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> (Vec<InlineItem>, Vec<Entity>, Option<Entity>) {
    let mut items = Vec::new();
    // Candidate keys: seed with the IFC parent's raw direct children, then collect
    // every recursed inline element's raw direct children during the walk.
    let mut candidate_keys: Vec<Entity> = children.to_vec();
    let top_level_key = first_eligible_child(dom, children);
    // The IFC root's writing-mode axis — the projection axis used for every group
    // (gates sub-flows whose positioned root overrides writing-mode; see the relpos
    // branch in `collect_inline_items_inner`).
    let root_horizontal = parent_style.writing_mode.is_horizontal();
    collect_inline_items_inner(
        dom,
        children,
        parent_style,
        parent_entity,
        0,
        &mut items,
        top_level_key,
        &mut candidate_keys,
        root_horizontal,
    );
    collapse_inline_whitespace(&mut items);
    apply_text_transforms(&mut items);
    // `top_level_key` (render's `run[0]`) is the justification target group:
    // `text-align: justify` distributes free space over the top-level run group only
    // (see `FlowAlign::top_level_key`), so it is returned for the caller's `FlowAlign`.
    (items, candidate_keys, top_level_key)
}

/// Apply CSS `text-transform` (CSS Text 3 §2.1) to each text run's collapsed
/// text, in place, *after* §4.1.1 white-space collapse and *before* the line
/// packer measures/breaks it (§2.1.2 Order of Operations). Because the packer
/// reads `run.text` for both break opportunities and width measurement, the
/// transformed text drives line breaking and the persisted glyph positions, and
/// render paints `run.text` verbatim (no re-transform). Each run is transformed
/// independently — §2.1.1's "inline box boundaries must not introduce a word
/// boundary" across runs is a pre-existing gap, matching render's prior
/// per-segment behavior.
fn apply_text_transforms(items: &mut [InlineItem]) {
    for item in items {
        if let InlineItem::Text(run) = item {
            if run.text_transform != TextTransform::None {
                if let std::borrow::Cow::Owned(transformed) =
                    apply_text_transform(&run.text, run.text_transform)
                {
                    run.text = transformed;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_inline_items_inner(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    depth: u32,
    items: &mut Vec<InlineItem>,
    // Render-run-group this level's members persist under (the run-start `run[0]`
    // render reads `InlineFlow` off). `None` = not recorded (positioned subtree
    // with a direct block child → legacy; see `has_direct_block_child`).
    group_key: Option<Entity>,
    // Superset of every entity that could carry an `InlineFlow` for this IFC
    // (every run-parent's raw direct children) — the caller's staleness clear set.
    candidate_keys: &mut Vec<Entity>,
    // Whether the IFC root's writing mode is horizontal (the projection axis the
    // persist block uses for ALL groups). A positioned inline whose writing mode
    // differs gets no sub-flow (render would read it with the wrong axis).
    root_horizontal: bool,
) {
    if depth >= MAX_LAYOUT_DEPTH {
        return;
    }
    for &child in children {
        if let Some(style) = crate::try_get_style(dom, child) {
            if style.display == Display::None {
                continue;
            }
            // CSS 2.1 §9.3.1/§9.6: absolutely positioned elements are removed from flow.
            // Insert a zero-width placeholder to record static position (CSS 2.1 §10.6.5).
            if crate::positioned::is_absolutely_positioned(&style) {
                items.push(InlineItem::Placeholder(child));
                continue;
            }
            // Atomic inline-level box (CSS Display 3 §A `#atomic-inline`):
            // placeholder with zero size (filled later by `layout_atomic_items`).
            // A *static* atomic converges into its group's `InlineFlow` as an
            // `AtomicBox` member — render paints it by `walk()`-ing the entity at
            // its own (repositioned) `LayoutBox`. A *relative/sticky* atomic
            // (`positioned`) is painted in render's Layer 6 from its own `LayoutBox`
            // (CSS 2 §9.4.3 in-flow advance, Layer-6 paint), so it is NOT a flow
            // member (that would double-paint with Layer 6) — `LinePacker` records
            // its on-line position separately and layout repositions its box
            // preserving the relative offset (slice 3p-b-2). The `position` check
            // lives here because this arm `continue`s before the inline-element
            // relpos sub-flow handling below. The static atomic carries the current
            // `group_key` so a static atomic inside a relpos sub-flow becomes that
            // sub-flow's `AtomicBox` member (repositioned per group at persist);
            // `group_key` is ignored for a positioned atomic (never a flow member).
            if is_atomic_inline(style.display) {
                items.push(InlineItem::Atomic {
                    entity: child,
                    inline_size: 0.0,
                    block_size: 0.0,
                    group_key,
                    positioned: matches!(style.position, Position::Relative | Position::Sticky),
                });
                continue;
            }
            // Pseudo-element: use its resolved generated text directly with its
            // own style (skip child recursion). The pre-layout generated-content
            // pass has already resolved `content` (incl. counter()) into the
            // pseudo's `TextContent`, so layout measures the real text. bidi and
            // text-transform no longer gate: the run persists in logical order and
            // render reorders for paint (slice 4) / transform is applied in-place
            // after collapse (no gate).
            if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
                if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                    if !tc.0.is_empty() {
                        items.push(InlineItem::Text(StyledRun::from_style(
                            child,
                            tc.0.clone(),
                            &style,
                            group_key,
                        )));
                    }
                }
                continue;
            }
            // Inline element: use its own style for its children. Every inline
            // element's raw direct children are candidate `InlineFlow` keys (any
            // could become a run-start `run[0]` in some pass) — record before
            // recursing so the caller can clear stale flows on them.
            let grandchildren = dom.composed_children(child);
            candidate_keys.extend_from_slice(&grandchildren);
            // CSS 2 §9.4.3: a relative/sticky positioned inline stays in-flow in the
            // IFC, but render paints its whole subtree in Layer 6 via `walk(child)`.
            // Slice 3p-b converges it as a **sub-flow** keyed on the subtree's first
            // eligible child (the parent flow advances past it, leaving the in-flow
            // gap) — unless the subtree is not single-linear-representable, in which
            // case `positioned_subflow_key` returns `None` and it falls to render's
            // legacy path. A non-positioned inline stays in the enclosing group.
            let child_group = if matches!(style.position, Position::Relative | Position::Sticky) {
                positioned_subflow_key(dom, child, &style, root_horizontal)
            } else {
                group_key
            };
            collect_inline_items_inner(
                dom,
                &grandchildren,
                &style,
                child,
                depth + 1,
                items,
                child_group,
                candidate_keys,
                root_horizontal,
            );
        } else if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            // Text node: produce a run with the parent element's style. Neither bidi
            // nor text-transform gates persistence any more: the run persists in
            // logical order (render reorders RTL runs visually — slice 4) and
            // text-transform is applied in-place after collapse
            // (`apply_text_transforms`, threaded via `StyledRun::text_transform`).
            if !tc.0.is_empty() {
                items.push(InlineItem::Text(StyledRun::from_style(
                    parent_entity,
                    tc.0.clone(),
                    parent_style,
                    group_key,
                )));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// White space collapsing (CSS Text 3 §4.1.1 Phase I)
// ---------------------------------------------------------------------------

/// Apply CSS Text 3 §4.1.1 Phase I white-space collapsing/transformation to the
/// ordered text runs of an inline formatting context, parameterized by each
/// run's `white-space`.
///
/// Per CSS Text 3 §4.1.1 (`#white-space-phase-1`) + §4.1.3 (`#line-break-transform`),
/// for collapsible values (`normal`/`nowrap`/`pre-line`): tabs become spaces
/// (step 3); for `normal`/`nowrap` a segment break (`\n`) is collapsible and is
/// transformed to a space (§4.1.3, word-separator-language baseline), while for
/// `pre-line` it is preserved as a forced break with surrounding collapsible
/// spaces removed (step 1); a collapsible space immediately following another
/// collapsible space — even across inline (run) boundaries within the same IFC —
/// collapses to zero advance width (step 4), i.e. each run of collapsible spaces
/// becomes a single space. For preserve values (`pre`/`pre-wrap`) the text is left
/// intact (segment breaks stay as forced breaks).
///
/// Line-edge trimming (§4.1.2 Phase II) and the "white space that collapses away
/// generates no box" rule (CSS 2 §9.2.2.1 / §9.2.1.1) are applied at line-packing
/// time (see [`pack::LinePacker`]), not here.
fn collapse_inline_whitespace(items: &mut [InlineItem]) {
    // Cross-run collapse state: true when the previously emitted character (in any
    // earlier run of this IFC) was a collapsible space, so a following collapsible
    // space collapses to zero advance width (§4.1.1 step 4). Initialized to `true`
    // so leading collapsible white space at the start of the inline formatting
    // context collapses away rather than becoming a leading space that shifts
    // content (CSS Text §4.1.2; matches `elidex-render`'s `collapse_segments`).
    let mut prev_collapsible_space = true;
    // Index of the most recent text run, so a preserved segment break at the start
    // of a later run can remove a collapsible space left at the end of it (§4.1.1
    // step 1, across the run boundary).
    let mut prev_text_idx: Option<usize> = None;
    for i in 0..items.len() {
        // Move the run's text out (no clone) to collapse it, then write it back.
        let (text, white_space) = match &mut items[i] {
            InlineItem::Text(run) => (std::mem::take(&mut run.text), run.white_space),
            // Atomic inline boxes are rendered content: a collapsible space that
            // follows one is a fresh separator, not collapsed away.
            InlineItem::Atomic { .. } => {
                prev_collapsible_space = false;
                prev_text_idx = None;
                continue;
            }
            // Out-of-flow placeholders (absolutely positioned, CSS 2.1 §9.3.1/§9.6)
            // are removed from the normal flow and do not participate in the inline
            // text flow, so they neither emit nor reset collapse state.
            InlineItem::Placeholder(_) => continue,
        };
        let (collapsed, trim_prev_trailing_space) =
            collapse_run_text(&text, white_space, &mut prev_collapsible_space);
        if trim_prev_trailing_space {
            if let Some(j) = prev_text_idx {
                if let InlineItem::Text(prev) = &mut items[j] {
                    if prev.text.ends_with(' ') {
                        prev.text.pop();
                    }
                }
            }
        }
        let collapsed_is_empty = collapsed.is_empty();
        if let InlineItem::Text(run) = &mut items[i] {
            run.text = collapsed;
        }
        // Keep `prev_text_idx` pointing at the last run that actually emitted text. A
        // run that collapsed to empty holds no trailing space, so the cross-run
        // trim (§4.1.1 step 1) must target the earlier run that emitted the pending
        // space, not this empty one.
        if !collapsed_is_empty {
            prev_text_idx = Some(i);
        }
    }
}

/// Collapse a single run's text per its `white-space`, threading the cross-run
/// `prev_collapsible_space` state. See [`collapse_inline_whitespace`].
///
/// Returns the collapsed text and a flag requesting that the caller remove a
/// collapsible space left at the end of the *previous* run: true when this run
/// emits a preserved segment break before any content while a collapsible space
/// was pending from the previous run (§4.1.1 step 1, across the run boundary).
fn collapse_run_text(
    text: &str,
    white_space: WhiteSpace,
    prev_collapsible_space: &mut bool,
) -> (String, bool) {
    // CSS Text 3 §4.1.3: normalize line endings before segment-break handling so a
    // bare CR or CRLF becomes the single canonical segment break (`\n`) for every
    // `white-space` value (otherwise a CR would be mishandled — e.g. preserved as a
    // forced break under pre-line). Matches `elidex-render`'s `normalize_line_endings`.
    // The common case has no CR, so only allocate when one is actually present.
    let text: std::borrow::Cow<str> = if text.contains('\r') {
        std::borrow::Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(text)
    };
    match white_space {
        // Preserve values: apart from the line-ending normalization applied above,
        // the text is preserved as-is (no space/tab collapsing, segment breaks kept
        // as forced breaks). A non-empty preserved run is rendered content, so it
        // resets the collapse state (a following collapsible run's leading space is a
        // fresh separator, not collapsed into the run).
        WhiteSpace::Pre | WhiteSpace::PreWrap => {
            if !text.is_empty() {
                *prev_collapsible_space = false;
            }
            (text.into_owned(), false)
        }
        WhiteSpace::Normal | WhiteSpace::NoWrap | WhiteSpace::PreLine => {
            let preserve_break = white_space == WhiteSpace::PreLine;
            // Whether a collapsible space was pending from the previous run on entry
            // (needed to remove it across the run boundary before a leading break).
            let entry_prev_space = *prev_collapsible_space;
            let mut out = String::with_capacity(text.len());
            let mut trim_prev_trailing_space = false;
            for c in text.chars() {
                if c == '\n' && preserve_break {
                    // §4.1.1 step 1 / §4.1.3: collapsible spaces around a preserved
                    // segment break are removed.
                    if out.ends_with(' ') {
                        // The space is in this run's own output — drop it directly.
                        out.pop();
                    } else if out.is_empty() && entry_prev_space {
                        // The space immediately preceding this break was emitted at
                        // the end of the previous run; ask the caller to remove it.
                        trim_prev_trailing_space = true;
                    }
                    out.push('\n');
                    *prev_collapsible_space = true;
                } else if is_collapsible_space(c) || c == '\n' {
                    // A collapsible space/tab, or (for normal/nowrap) a segment break
                    // transformed to a space (§4.1.3). Collapse runs to a single
                    // space (step 4); a space following another collapsible space has
                    // zero advance width and is dropped from the string.
                    if !*prev_collapsible_space {
                        out.push(' ');
                        *prev_collapsible_space = true;
                    }
                } else {
                    out.push(c);
                    *prev_collapsible_space = false;
                }
            }
            (out, trim_prev_trailing_space)
        }
    }
}

/// CSS Text 3 collapsible space characters: space and tab. CR/CRLF are normalized
/// to the segment break `\n` upstream in [`collapse_run_text`], so they are not
/// treated here; the segment break itself is handled separately because its
/// transformation depends on `white-space` (§4.1.3).
fn is_collapsible_space(c: char) -> bool {
    matches!(c, ' ' | '\t')
}

// ---------------------------------------------------------------------------
// Inline layout entry point
// ---------------------------------------------------------------------------

/// Result of inline layout, including static positions for absolutely positioned descendants.
pub struct InlineLayoutResult {
    /// Total block-axis dimension consumed by all line boxes.
    pub height: f32,
    /// Static positions for absolutely positioned placeholders (CSS 2.1 §10.6.5).
    /// Positions are in content-area-relative coordinates.
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
                // NOT run for `do_carrier` (mid-break): a multicol mid-break IFC re-runs
                // `layout_atomic_items` for the WHOLE IFC every column (continuation),
                // resetting any earlier-column atomic's box back to `content_origin`,
                // and a single column's run only knows its own slice — so repositioning
                // here would fix only the LAST column and leave earlier-column atomics
                // displaced (Codex PR#316 R1). Correct per-fragment atomic positioning
                // needs the box-store fragment model (it must carry the un-offset basis
                // per column + position after the final re-lay) = the committed-next
                // all-box-type program (atomic-as-fragment, plan §C/§D). Z-1b is
                // text-only: the `AtomicBox` runs ARE carried in the per-column
                // `InlineFlow` (so render walks them), but the atomic `LayoutBox`
                // stays at its pre-Z-1b position (base + `col_children` shift) — no
                // regression, deferred whole rather than half-fixed.
                for line in &lines {
                    for run in &line.runs {
                        if let InlineFlowRun::AtomicBox {
                            entity: atomic,
                            inline_start,
                        } = run
                        {
                            reposition_atomic_box(
                                dom,
                                *atomic,
                                *inline_start,
                                line.block_start,
                                is_vertical,
                                unoffset_origins.get(atomic).copied(),
                            );
                        }
                    }
                }
                // I-paged writes one fragment per page (length-1 Vec): each page's
                // full re-layout replaces it (render walks the page interleaved before
                // the next), so the run-start carries this page's slice stamped with
                // the page generation. Multicol whole-in-column persists its 1-fragment
                // flow here too (shifted to its column by the column shift).
                let _ = dom
                    .world_mut()
                    .insert_one(group_key, InlineFlow::single(env.layout_generation, lines));
                persisted_keys.insert(group_key);
            } else {
                // do_carrier: this column's slice for this run-start group. multicol
                // fill drains the carrier; `position_column_fragments` folds it into
                // `group_key`'s `InlineFlow` offset to the column's inline position.
                // Static-atomic / relpos-atomic reposition is intentionally NOT run for
                // mid-break (see the `persist_flow` arm above) — committed-next.
                carrier_groups.push((group_key, lines));
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
        // not stripped). Gated on `persist_flow` (NOT `do_carrier`) for the same reason
        // as the static-atomic loop above — mid-break atomic positioning is
        // committed-next, not half-fixed here.
        if persist_flow {
            for (atomic, inline_local, block_local) in packer.relpos_atomic_placements {
                reposition_atomic_box(
                    dom,
                    atomic,
                    inline_local + inline_origin,
                    block_local + block_origin,
                    is_vertical,
                    unoffset_origins.get(&atomic).copied(),
                );
            }
        }
    }
    // Carrier reconcile (insert-or-remove, mirroring `clear_inline_flows`): the
    // multicol mid-break IFC writes its per-column slice on `parent_entity`; every
    // other case (whole/paged/non-fragmented persist, or empty) clears any stale one.
    // `ColumnFlowSlice` is never read by render (drained-only by multicol fill), so a
    // leak is benign; this keeps the entity clean across passes.
    if do_carrier && !carrier_groups.is_empty() {
        let _ = dom
            .world_mut()
            .insert_one(parent_entity, ColumnFlowSlice(carrier_groups));
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
    clear_inline_flows(dom, &candidate_keys, &persisted_keys);

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
fn reposition_atomic_box(
    dom: &mut EcsDom,
    atomic: Entity,
    inline_abs: f32,
    block_abs: f32,
    is_vertical: bool,
    unoffset_origin: Option<Point>,
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
    crate::block::shift_descendants(dom, &children, delta);
    if let Ok(mut lb) = dom.world_mut().get::<&mut elidex_plugin::LayoutBox>(atomic) {
        lb.content.origin += delta;
    }
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
