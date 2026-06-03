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

use elidex_ecs::{EcsDom, Entity, InlineFlow, InlineFlowRun, PseudoElementMarker, TextContent};
#[cfg(test)]
pub(crate) use elidex_plugin::LayoutBox;
use elidex_plugin::{
    ComputedStyle, Display, Point, Position, TextAlign, TextTransform, WhiteSpace,
};
#[cfg(test)]
pub(crate) use elidex_text::FontDatabase;
use elidex_text::{measure_text, to_fontdb_style, FontStyle, TextMeasureParams};

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
}

impl StyledRun {
    /// Create a run from text content and a computed style.
    fn from_style(entity: Entity, text: String, style: &ComputedStyle) -> Self {
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

/// Whether an inline run contains members that make its render-side treatment
/// diverge from layout's IFC membership, so layout must **not** persist an
/// `InlineFlow` for it (render falls back to its own collect/collapse/emit).
///
/// (Slice 3 removed `has_pseudo`: pseudo-element `content` — including
/// `counter()` — is now resolved into the pseudo's `TextContent` by the
/// pre-layout generated-content pass, so layout measures the resolved text and
/// pseudo runs persist like any other text run, subject to the gates below.)
///
/// (Slice 3p-a removed `has_atomic`: a *static* `inline-block`/`-flex`/`-grid`/
/// `-table` now persists as an `AtomicBox` member of `InlineFlow` — render paints
/// it by `walk()`-ing the entity at its repositioned `LayoutBox`, no longer
/// flattening its text via `collect_styled_inline_text`. A *relative/sticky*
/// atomic still sets `has_relpos_sticky` (it paints in render's Layer 6); slice
/// 3p-b converges that case.)
///
/// - `has_relpos_sticky`: a `position: relative`/`sticky` inline (incl. a
///   relpos/sticky atomic) — in-flow in layout's IFC (CSS 2 §9.4.3) but painted
///   in render's Layer 6 (slice 3p-b).
/// - `has_bidi`: a run whose text contains right-to-left characters — layout
///   positions runs in logical order, but render reorders them visually
///   (`bidi_visual_order`); consuming the logical order would scramble them
///   (slice 4).
/// - `has_text_transform`: a run whose element applies `text-transform` — layout
///   measures/positions the untransformed text, but render transforms it before
///   shaping (`query_segment_font`), so the baked positions would be wrong
///   (slice 3; the deeper fix is layout transforming before measuring).
// Each field is an independent "gate out of InlineFlow persistence" reason;
// a flag set is the natural representation (bitflags would be overkill here).
#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy)]
pub(crate) struct RunComplexity {
    pub has_relpos_sticky: bool,
    pub has_bidi: bool,
    pub has_text_transform: bool,
}

/// Recursively collect inline items (text runs + atomic boxes) from inline children.
///
/// Text nodes produce a run inheriting the nearest ancestor element's style.
/// Inline elements use their own style for their children. `display: none`
/// elements are skipped. Atomic inline-level boxes (`inline-block`, `inline-flex`, etc.)
/// produce placeholder items — they are laid out separately and placed as
/// atomic units in the inline flow. Recursion stops at [`MAX_LAYOUT_DEPTH`].
///
/// Also reports a [`RunComplexity`] describing run members that gate the run out
/// of `InlineFlow` persistence (see its docs).
pub(crate) fn collect_inline_items(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> (Vec<InlineItem>, RunComplexity) {
    let mut items = Vec::new();
    let mut complexity = RunComplexity::default();
    collect_inline_items_inner(
        dom,
        children,
        parent_style,
        parent_entity,
        0,
        &mut items,
        &mut complexity,
    );
    collapse_inline_whitespace(&mut items);
    (items, complexity)
}

fn collect_inline_items_inner(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    depth: u32,
    items: &mut Vec<InlineItem>,
    complexity: &mut RunComplexity,
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
            // A *static* atomic converges into `InlineFlow` as an `AtomicBox`
            // member — render paints it by `walk()`-ing the entity at its own
            // (repositioned) `LayoutBox`. A *relative/sticky* atomic is painted in
            // render's Layer 6 (it `is_positioned`), so it must stay gated out of
            // persistence via `has_relpos_sticky` (slice 3p-b converges it). The
            // `position` check lives here because this arm `continue`s before the
            // inline-element relpos check below.
            if is_atomic_inline(style.display) {
                if matches!(style.position, Position::Relative | Position::Sticky) {
                    complexity.has_relpos_sticky = true;
                }
                items.push(InlineItem::Atomic {
                    entity: child,
                    inline_size: 0.0,
                    block_size: 0.0,
                });
                continue;
            }
            // Pseudo-element: use its resolved generated text directly with its
            // own style (skip child recursion). The pre-layout generated-content
            // pass has already resolved `content` (incl. counter()) into the
            // pseudo's `TextContent`, so layout measures the real text. The run is
            // still gated out of `InlineFlow` if it needs bidi reordering (slice 4)
            // or text-transform (render transforms before shaping) — the same
            // gates the text-node branch applies, here against the pseudo's own
            // computed text-transform and its resolved text.
            if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
                if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                    if !tc.0.is_empty() {
                        if style.text_transform != TextTransform::None {
                            complexity.has_text_transform = true;
                        }
                        if elidex_text::text_has_rtl(&tc.0) {
                            complexity.has_bidi = true;
                        }
                        items.push(InlineItem::Text(StyledRun::from_style(
                            child,
                            tc.0.clone(),
                            &style,
                        )));
                    }
                }
                continue;
            }
            // CSS 2 §9.4.3: a relative/sticky positioned inline stays in-flow in
            // the IFC here, but render pulls it (and its whole subtree) into a
            // separate stacking layer — so the run cannot be converged in slice 1.
            if matches!(style.position, Position::Relative | Position::Sticky) {
                complexity.has_relpos_sticky = true;
            }
            // Inline element: use its own style for its children.
            let grandchildren = dom.composed_children(child);
            collect_inline_items_inner(
                dom,
                &grandchildren,
                &style,
                child,
                depth + 1,
                items,
                complexity,
            );
        } else if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            // Text node: produce a run with the parent element's style.
            if !tc.0.is_empty() {
                // Gate the run out of InlineFlow persistence if it needs bidi
                // reordering (render reorders visually) or text-transform (render
                // transforms before shaping, diverging from layout's measured
                // positions). text-transform is inherited, so the parent element's
                // computed value is the one render will apply to this text.
                if parent_style.text_transform != TextTransform::None {
                    complexity.has_text_transform = true;
                }
                if elidex_text::text_has_rtl(&tc.0) {
                    complexity.has_bidi = true;
                }
                items.push(InlineItem::Text(StyledRun::from_style(
                    parent_entity,
                    tc.0.clone(),
                    parent_style,
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

/// Fragmentation constraint for inline layout (CSS Fragmentation L3 §4.3).
pub struct InlineFragConstraint {
    /// Available block-axis space from the current cursor position.
    pub available_block: f32,
    /// CSS orphans value (inherited, default 2).
    pub orphans: u32,
    /// CSS widows value (inherited, default 2).
    pub widows: u32,
    /// Number of lines to skip (for resuming after a break).
    pub skip_lines: usize,
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
    let (mut items, complexity) = collect_inline_items(dom, children, &parent_style, parent_entity);
    // Run-start entity = the key under which `InlineFlow` is persisted/cleared
    // (the first top-level run child, which render also derives as its run[0]).
    let run_start = children.first().copied();
    if items.is_empty() {
        clear_inline_flow(dom, run_start);
        return InlineLayoutResult {
            height: 0.0,
            static_positions: HashMap::new(),
            first_baseline: None,
            line_count: 0,
            break_after_line: None,
        };
    }

    let is_vertical = !parent_style.writing_mode.is_horizontal();
    // Layout atomic inline boxes and fill in their dimensions.
    atomic::layout_atomic_items(
        dom,
        &mut items,
        containing_inline_size,
        content_origin,
        font_db,
        layout_child,
        is_vertical,
        env.layout_generation,
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
            clear_inline_flow(dom, run_start);
            return InlineLayoutResult {
                height: 0.0,
                static_positions: HashMap::new(),
                first_baseline: None,
                line_count: 0,
                break_after_line: None,
            };
        }
    }

    // InlineFlow persistence gate: non-justify, non-fragmented runs with no
    // relative/sticky positioned inline, no bidi (RTL) text, no text-transform.
    // Each excluded case is a layout-IFC-vs-render divergence (or, for
    // fragmentation, a per-fragment slicing the persisted geometry does not yet
    // model). Slice 3p-a dropped the atomic gate: a *static* atomic inline now
    // persists as an `AtomicBox` member (render walks it at its repositioned
    // `LayoutBox`); a *relative/sticky* atomic is caught by `has_relpos_sticky`.
    // When the gate fails, render keeps its own collect/collapse/emit path. Slice 2
    // added vertical writing modes: the packer already produces inline/block-axis
    // positions and the align resolution is axis-agnostic, so persisting vertical
    // needs only the origin fold's is_vertical swap (below) + a writing-mode-aware
    // render consume path.
    let persist_flow = frag_constraint.is_none()
        && parent_style.text_align != TextAlign::Justify
        && !complexity.has_relpos_sticky
        && !complexity.has_bidi
        && !complexity.has_text_transform;
    let flow_align = persist_flow.then_some(pack::FlowAlign {
        text_align: parent_style.text_align,
        direction: parent_style.direction,
        containing_inline_size,
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

    // --- Fragmentation: orphans/widows enforcement (CSS Fragmentation L3 §4.3) ---
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
    let total_block: f32 = packer
        .line_boxes
        .iter()
        .skip(skip_lines)
        .take(effective_line_count.saturating_sub(skip_lines))
        .map(|lb| lb.block_size)
        .sum();

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

    // Reconcile the run-start entity's InlineFlow (render's single source of inline
    // geometry) every pass: persist when the gate passed and lines were produced,
    // else clear any stale flow from a prior pass. `layout_generation` is constant 0
    // off the paged path, so an explicit clear (not generation comparison) is what
    // prevents render consuming a stale flow after a run becomes non-persistable.
    let first_baseline = packer.first_baseline;
    if persist_flow && !packer.flow_lines.is_empty() {
        if let Some(entity) = run_start {
            // IFC-local logical → absolute physical, applying the SAME is_vertical
            // projection rule as `static_positions` (above) and
            // `assign_inline_layout_boxes`: inline-axis maps to physical x (horizontal)
            // or y (vertical), block-axis to y (horizontal) or x (vertical). After the
            // fold each scalar holds the absolute physical coordinate for its axis, so
            // render reads `block_start`/`inline_start` without a coordinate transform
            // (it selects the right field per writing mode). No vertical-rl block-axis
            // reversal — matching the box convention (see static_positions).
            let (inline_origin, block_origin) = if is_vertical {
                (content_origin.y, content_origin.x)
            } else {
                (content_origin.x, content_origin.y)
            };
            let lines: Vec<elidex_ecs::InlineFlowLine> = packer
                .flow_lines
                .into_iter()
                .map(|mut line| {
                    line.block_start += block_origin;
                    for run in &mut line.runs {
                        *run.inline_start_mut() += inline_origin;
                    }
                    line
                })
                .collect();
            // Reposition each static atomic inline's `LayoutBox` to its on-line
            // position (text-align already baked into `inline_start`). `layout_atomic_items`
            // laid the atomic out at `content_origin` (IFC top-left); render paints the
            // atomic by `walk()`-ing its `LayoutBox`, so the box must reflect the line
            // position (layout owns geometry — render does not paint-time-translate).
            // Only persisting runs reach here, and a persisting run has only *static*
            // atomics (relpos/sticky → `has_relpos_sticky` → gated), so this never
            // clobbers an `apply_relative_offset` (slice 3p-b owns positioned atomics).
            // Block-axis = line top (baseline-naive; CSS 2 §10.8 `vertical-align` within
            // the line box is deferred — the same leading-naive model as text runs).
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
                        );
                    }
                }
            }
            let _ = dom.world_mut().insert_one(
                entity,
                InlineFlow {
                    lines,
                    layout_generation: env.layout_generation,
                },
            );
        }
    } else {
        clear_inline_flow(dom, run_start);
    }

    InlineLayoutResult {
        height: total_block,
        static_positions,
        first_baseline,
        line_count,
        break_after_line,
    }
}

/// Reposition a static atomic inline's `LayoutBox` (and its descendants) from the
/// `content_origin` placement `layout_atomic_items` gave it to its packed on-line
/// position. `inline_abs`/`block_abs` are the absolute (writing-mode-folded) inline
/// start and line block-start; the atomic's margin-box origin is moved there so
/// render — which paints the atomic by `walk()`-ing it at its `LayoutBox` — sees
/// the correct rect (CSS 2 §9.2.2 inline-level box at its IFC position). Descendants
/// shift rigidly with the box (`shift_descendants`), the same operation relative
/// positioning uses (`elidex-layout::layout` apply-relative-offset). Block-axis is
/// the line top (baseline-naive; `vertical-align` within the line box is deferred).
fn reposition_atomic_box(
    dom: &mut EcsDom,
    atomic: Entity,
    inline_abs: f32,
    block_abs: f32,
    is_vertical: bool,
) {
    let Ok(lb) = dom.world().get::<&elidex_plugin::LayoutBox>(atomic) else {
        return;
    };
    // inline-axis → physical x (horizontal) or y (vertical); block-axis → the other.
    let target = if is_vertical {
        Point::new(block_abs, inline_abs)
    } else {
        Point::new(inline_abs, block_abs)
    };
    let delta = target - lb.margin_box().origin;
    drop(lb);
    if delta.x.abs() <= f32::EPSILON && delta.y.abs() <= f32::EPSILON {
        return;
    }
    let children = dom.composed_children(atomic);
    crate::block::shift_descendants(dom, &children, delta);
    if let Ok(mut lb) = dom.world_mut().get::<&mut elidex_plugin::LayoutBox>(atomic) {
        lb.content.origin += delta;
    }
}

/// Remove any stale [`InlineFlow`] from the run-start entity (the run is not being
/// persisted this pass — gated out, empty, or unrenderable). See the reconcile
/// comment in `layout_inline_context_fragmented`.
fn clear_inline_flow(dom: &mut EcsDom, run_start: Option<Entity>) {
    if let Some(entity) = run_start {
        let _ = dom.world_mut().remove_one::<InlineFlow>(entity);
    }
}
