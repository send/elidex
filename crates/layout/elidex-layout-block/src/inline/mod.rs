//! Inline formatting context layout algorithm.
//!
//! Handles text measurement and line breaking for inline content.
//! Text is collected as styled runs that preserve per-element style
//! (font-size, font-weight, font-family, spacing), then greedily
//! packed into line boxes that fit the containing block width.

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity, PseudoElementMarker, TextContent};
use elidex_plugin::{ComputedStyle, Display, EdgeSizes, LayoutBox, Rect, WritingMode};
use elidex_text::{
    find_break_opportunities, measure_text, to_fontdb_style, BreakOpportunity, FontDatabase,
    FontStyle, TextMeasureParams,
};

use crate::MAX_LAYOUT_DEPTH;

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
    fn measure_params<'a>(&self, families: &'a [&'a str]) -> TextMeasureParams<'a> {
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
    fn family_refs(&self) -> Vec<&str> {
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

/// Compute max-content inline size (no line breaking) for shrink-to-fit width.
///
/// Sums the measured width of all text runs without line breaking.
/// Atomic inline-level boxes contribute zero (their intrinsic width
/// is not yet computed at this stage).
pub(crate) fn max_content_inline_size(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
    font_db: &FontDatabase,
) -> f32 {
    let items = collect_inline_items(dom, children, parent_style, parent_entity);
    let mut total = 0.0_f32;
    for item in &items {
        if let InlineItem::Text(run) = item {
            let families = run.family_refs();
            let params = run.measure_params(&families);
            if let Some(m) = measure_text(font_db, &params, &run.text) {
                total += m.width;
            }
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Segment measurement
// ---------------------------------------------------------------------------

/// Measure a segment's full and trimmed widths.
///
/// Returns `(full_width, trimmed_width)` where `trimmed_width` excludes trailing
/// whitespace per CSS Text Level 3 §4.1.2 (trailing spaces "hang" and don't
/// trigger line overflow).
fn measure_segment_widths(
    font_db: &FontDatabase,
    params: &TextMeasureParams<'_>,
    segment: &str,
) -> (f32, f32) {
    let seg_width = measure_text(font_db, params, segment).map_or(0.0, |m| m.width);
    let trimmed = segment.trim_end();
    let trimmed_width = if trimmed.len() == segment.len() {
        seg_width
    } else if trimmed.is_empty() {
        0.0
    } else {
        measure_text(font_db, params, trimmed).map_or(0.0, |m| m.width)
    };
    (seg_width, trimmed_width)
}

// ---------------------------------------------------------------------------
// Break segment — a piece of a StyledRun between break opportunities
// ---------------------------------------------------------------------------

/// A piece of inline content for line packing — either a text segment or an atomic box.
enum PackItem {
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

/// Build pack items from inline items.
///
/// Text runs are split at break opportunities. Atomic boxes become single pack items.
fn build_pack_items(items: &[InlineItem]) -> Vec<PackItem> {
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
struct LineBox {
    /// Block-axis size (height for horizontal writing mode).
    block_size: f32,
}

/// Tracks the bounding rectangle of an inline entity across line boxes.
struct EntityBounds {
    /// Inline-axis start on the first line.
    inline_start: f32,
    /// Inline-axis end on the last line.
    inline_end: f32,
    /// Block-axis offset of the first line.
    block_start: f32,
    /// Block-axis offset + size of the last line.
    block_end: f32,
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
    pub static_positions: HashMap<Entity, (f32, f32)>,
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
/// `content_origin` is the `(x, y)` position of the parent's content area
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
#[allow(clippy::too_many_arguments)]
pub fn layout_inline_context(
    dom: &mut EcsDom,
    children: &[Entity],
    containing_inline_size: f32,
    parent_style: &ComputedStyle,
    font_db: &FontDatabase,
    parent_entity: Entity,
    content_origin: (f32, f32),
    layout_child: crate::ChildLayoutFn,
) -> InlineLayoutResult {
    let mut items = collect_inline_items(dom, children, parent_style, parent_entity);
    if items.is_empty() {
        return InlineLayoutResult {
            height: 0.0,
            static_positions: HashMap::new(),
        };
    }

    let is_vertical = !matches!(parent_style.writing_mode, WritingMode::HorizontalTb);
    // Layout atomic inline boxes and fill in their dimensions.
    layout_atomic_items(
        dom,
        &mut items,
        containing_inline_size,
        content_origin,
        font_db,
        layout_child,
        is_vertical,
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
            };
        }
    }

    let pack_items = build_pack_items(&items);

    // Greedy line packing.
    let mut packer = LinePacker::new(parent_entity);
    for pi in &pack_items {
        packer.pack(pi, &items, font_db, containing_inline_size, is_vertical);
    }
    packer.finish();

    let total_block: f32 = packer.line_boxes.iter().map(|lb| lb.block_size).sum();
    assign_inline_layout_boxes(dom, &packer.entity_bounds, content_origin, is_vertical);

    // Convert static positions from packer-relative to layout coordinates.
    let static_positions: HashMap<Entity, (f32, f32)> = packer
        .static_positions
        .into_iter()
        .map(|(entity, (inline_pos, block_pos))| {
            if is_vertical {
                (
                    entity,
                    (content_origin.0 + block_pos, content_origin.1 + inline_pos),
                )
            } else {
                (
                    entity,
                    (content_origin.0 + inline_pos, content_origin.1 + block_pos),
                )
            }
        })
        .collect();

    InlineLayoutResult {
        height: total_block,
        static_positions,
    }
}

/// Layout all atomic inline items (`inline-block`, etc.) and fill their dimensions.
fn layout_atomic_items(
    dom: &mut EcsDom,
    items: &mut [InlineItem],
    containing_inline_size: f32,
    content_origin: (f32, f32),
    font_db: &FontDatabase,
    layout_child: crate::ChildLayoutFn,
    is_vertical: bool,
) {
    for item in items.iter_mut() {
        if let InlineItem::Atomic {
            entity,
            inline_size,
            block_size,
        } = item
        {
            let input = crate::LayoutInput {
                containing_width: containing_inline_size,
                containing_height: None,
                offset_x: content_origin.0,
                offset_y: content_origin.1,
                font_db,
                depth: 0,
                float_ctx: None,
                viewport: None,
                fragmentainer: None,
                break_token: None,
            };
            let lb = layout_child(dom, *entity, &input).layout_box;
            let margin_box = lb.margin_box();
            if is_vertical {
                *inline_size = margin_box.height;
                *block_size = margin_box.width;
            } else {
                *inline_size = margin_box.width;
                *block_size = margin_box.height;
            }
        }
    }
}

/// Line packer state — extracted to keep the main function under the line limit.
struct LinePacker {
    line_boxes: Vec<LineBox>,
    entity_bounds: HashMap<Entity, EntityBounds>,
    /// Static positions for absolutely positioned placeholders (CSS 2.1 §10.6.5).
    static_positions: HashMap<Entity, (f32, f32)>,
    current_inline: f32,
    current_line_height: f32,
    current_block_offset: f32,
    on_line: bool,
    parent_entity: Entity,
}

impl LinePacker {
    fn new(parent_entity: Entity) -> Self {
        Self {
            line_boxes: Vec::new(),
            entity_bounds: HashMap::new(),
            static_positions: HashMap::new(),
            current_inline: 0.0,
            current_line_height: 0.0,
            current_block_offset: 0.0,
            on_line: false,
            parent_entity,
        }
    }

    fn pack(
        &mut self,
        pi: &PackItem,
        items: &[InlineItem],
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

                self.place_item(
                    seg_width,
                    trimmed_width,
                    seg_line_advance,
                    run.entity,
                    containing_inline_size,
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
                } = &items[*item_index]
                else {
                    return;
                };
                // Atomic boxes don't break internally; treat as a single unit.
                self.place_item(
                    *inline_size,
                    *inline_size,
                    *block_size,
                    *entity,
                    containing_inline_size,
                );
            }
            PackItem::Placeholder { entity } => {
                // CSS 2.1 §10.6.5: record static position at current inline/block position.
                // Zero-width, zero-height — does not advance cursor_x.
                self.static_positions
                    .insert(*entity, (self.current_inline, self.current_block_offset));
            }
        }
    }

    fn place_item(
        &mut self,
        full_width: f32,
        trimmed_width: f32,
        block_advance: f32,
        entity: Entity,
        containing_inline_size: f32,
    ) {
        if self.current_inline + trimmed_width > containing_inline_size && self.on_line {
            self.line_boxes.push(LineBox {
                block_size: self.current_line_height,
            });
            self.current_block_offset += self.current_line_height;
            self.current_inline = 0.0;
            self.current_line_height = 0.0;
        }

        let seg_inline_start = self.current_inline;
        self.current_inline += full_width;
        self.current_line_height = self.current_line_height.max(block_advance);
        self.on_line = true;

        if entity != self.parent_entity {
            let seg_inline_end = seg_inline_start + full_width;
            let line_block_end = self.current_block_offset + self.current_line_height;
            self.entity_bounds
                .entry(entity)
                .and_modify(|b| {
                    b.inline_end = seg_inline_end;
                    b.block_end = line_block_end;
                })
                .or_insert(EntityBounds {
                    inline_start: seg_inline_start,
                    inline_end: seg_inline_end,
                    block_start: self.current_block_offset,
                    block_end: line_block_end,
                });
        }
    }

    fn force_break(&mut self) {
        self.line_boxes.push(LineBox {
            block_size: self.current_line_height,
        });
        self.current_block_offset += self.current_line_height;
        self.current_inline = 0.0;
        self.current_line_height = 0.0;
        self.on_line = false;
    }

    fn finish(&mut self) {
        if self.on_line {
            self.line_boxes.push(LineBox {
                block_size: self.current_line_height,
            });
        }
    }
}

/// Assign `LayoutBox` to inline elements based on their bounding rects.
///
/// Each entity that has a `ComputedStyle` (i.e. is an element, not a text node)
/// and was tracked during line packing receives a `LayoutBox` with its
/// bounding rectangle in layout coordinates.
fn assign_inline_layout_boxes(
    dom: &mut EcsDom,
    entity_bounds: &HashMap<Entity, EntityBounds>,
    content_origin: (f32, f32),
    is_vertical: bool,
) {
    let (origin_x, origin_y) = content_origin;
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
        };
        let _ = dom.world_mut().insert_one(*entity, lb);
    }
}
