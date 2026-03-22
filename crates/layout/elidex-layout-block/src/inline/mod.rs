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

use elidex_ecs::{EcsDom, Entity, PseudoElementMarker, TextContent};
#[cfg(test)]
pub(crate) use elidex_plugin::LayoutBox;
use elidex_plugin::{ComputedStyle, Display, Point};
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

/// Recursively collect inline items (text runs + atomic boxes) from inline children.
///
/// Text nodes produce a run inheriting the nearest ancestor element's style.
/// Inline elements use their own style for their children. `display: none`
/// elements are skipped. Atomic inline-level boxes (`inline-block`, `inline-flex`, etc.)
/// produce placeholder items — they are laid out separately and placed as
/// atomic units in the inline flow. Recursion stops at [`MAX_LAYOUT_DEPTH`].
pub(crate) fn collect_inline_items(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> Vec<InlineItem> {
    collect_inline_items_inner(dom, children, parent_style, parent_entity, 0)
}

fn collect_inline_items_inner(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    depth: u32,
) -> Vec<InlineItem> {
    if depth >= MAX_LAYOUT_DEPTH {
        return Vec::new();
    }
    let mut items = Vec::new();
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
            // Atomic inline-level box: placeholder with zero size (filled later).
            if is_atomic_inline(style.display) {
                items.push(InlineItem::Atomic {
                    entity: child,
                    inline_size: 0.0,
                    block_size: 0.0,
                });
                continue;
            }
            // Pseudo-element: use text directly with own style (skip child recursion).
            if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
                if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                    if !tc.0.is_empty() {
                        items.push(InlineItem::Text(StyledRun::from_style(
                            child,
                            tc.0.clone(),
                            &style,
                        )));
                    }
                }
                continue;
            }
            // Inline element: use its own style for its children.
            let grandchildren = dom.composed_children(child);
            items.extend(collect_inline_items_inner(
                dom,
                &grandchildren,
                &style,
                child,
                depth + 1,
            ));
        } else if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            // Text node: produce a run with the parent element's style.
            if !tc.0.is_empty() {
                items.push(InlineItem::Text(StyledRun::from_style(
                    parent_entity,
                    tc.0.clone(),
                    parent_style,
                )));
            }
        }
    }
    items
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
    let mut items = collect_inline_items(dom, children, &parent_style, parent_entity);
    if items.is_empty() {
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
            return InlineLayoutResult {
                height: 0.0,
                static_positions: HashMap::new(),
                first_baseline: None,
                line_count: 0,
                break_after_line: None,
            };
        }
    }

    let pack_items = pack::build_pack_items(&items);

    // Greedy line packing.
    let mut packer = pack::LinePacker::new(parent_entity);
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

    InlineLayoutResult {
        height: total_block,
        static_positions,
        first_baseline: packer.first_baseline,
        line_count,
        break_after_line,
    }
}
