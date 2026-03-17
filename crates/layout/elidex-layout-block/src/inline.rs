//! Inline formatting context layout algorithm.
//!
//! Handles text measurement and line breaking for inline content.
//! Text is collected as styled runs that preserve per-element style
//! (font-size, font-weight, font-family, spacing), then greedily
//! packed into line boxes that fit the containing block width.

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

/// Collect only text runs from inline items (for tests that don't need atomics).
#[cfg(test)]
fn collect_styled_runs(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> Vec<StyledRun> {
    collect_inline_items(dom, children, parent_style, parent_entity)
        .into_iter()
        .filter_map(|item| match item {
            InlineItem::Text(run) => Some(run),
            InlineItem::Atomic { .. } => None,
        })
        .collect()
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

/// Layout inline content (text nodes, inline elements, and atomic inline
/// boxes) within line boxes.
///
/// Returns the total block-axis dimension consumed by all line boxes.
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
) -> f32 {
    let mut items = collect_inline_items(dom, children, parent_style, parent_entity);
    if items.is_empty() {
        return 0.0;
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
            InlineItem::Atomic { .. } => false,
        });
        if !any_font && !items.iter().any(|i| matches!(i, InlineItem::Atomic { .. })) {
            return 0.0;
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

    total_block
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
            };
            let lb = layout_child(dom, *entity, &input);
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

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::Dimension;

    const TEST_FAMILIES: &[&str] = &[
        "Arial",
        "Helvetica",
        "Liberation Sans",
        "DejaVu Sans",
        "Noto Sans",
        "Hiragino Sans",
    ];

    /// Setup a DOM with a `<p>` parent and a text child, a default `ComputedStyle`
    /// with test font families, and a `FontDatabase`. Returns `None` if no font is available.
    fn setup_inline_test(
        text_content: &str,
    ) -> Option<(EcsDom, Entity, ComputedStyle, FontDatabase)> {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text(text_content);
        dom.append_child(parent, text);

        let style = ComputedStyle {
            font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
            ..Default::default()
        };
        let font_db = FontDatabase::new();
        let params = TextMeasureParams {
            families: TEST_FAMILIES,
            font_size: style.font_size,
            weight: 400,
            style: elidex_text::FontStyle::Normal,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        };
        measure_text(&font_db, &params, "x")?;
        Some((dom, parent, style, font_db))
    }

    #[test]
    fn empty_text_zero_height() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text("");
        dom.append_child(parent, text);

        let style = ComputedStyle::default();
        let font_db = FontDatabase::new();
        let children = dom.composed_children(parent);

        let h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!(h.abs() < f32::EPSILON);
    }

    #[test]
    fn no_children_zero_height() {
        let mut dom = EcsDom::new();
        let parent_entity = Entity::DANGLING;
        let style = ComputedStyle::default();
        let font_db = FontDatabase::new();

        let h = layout_inline_context(
            &mut dom,
            &[],
            800.0,
            &style,
            &font_db,
            parent_entity,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!(h.abs() < f32::EPSILON);
    }

    #[test]
    fn single_line_text() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
            return;
        };

        let css_line_height = style.line_height.resolve_px(style.font_size);
        let children = dom.composed_children(parent);
        let h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!((h - css_line_height).abs() < f32::EPSILON);
    }

    #[test]
    fn mandatory_newline_break() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("line1\nline2") else {
            return;
        };

        let css_line_height = style.line_height.resolve_px(style.font_size);
        // Wide container: should still produce 2 lines due to \n
        let children = dom.composed_children(parent);
        let h = layout_inline_context(
            &mut dom,
            &children,
            8000.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!((h - css_line_height * 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn text_wrapping_increases_height() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("hello world foo bar baz")
        else {
            return;
        };

        let css_line_height = style.line_height.resolve_px(style.font_size);
        // Use a very narrow width to force wrapping
        let children = dom.composed_children(parent);
        let h = layout_inline_context(
            &mut dom,
            &children,
            1.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!(h > css_line_height);
    }

    // --- M3.5-4: Vertical writing mode ---

    #[test]
    fn vertical_mode_uses_font_size_line_advance() {
        let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
            return;
        };
        style.writing_mode = WritingMode::VerticalRl;

        // In vertical mode, the block-axis advance per line is font_size, not line-height.
        let children = dom.composed_children(parent);
        let block_dim = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        // Single line: block dimension should be font_size.
        assert!(
            (block_dim - style.font_size).abs() < f32::EPSILON,
            "vertical single line should be font_size ({}), got {}",
            style.font_size,
            block_dim,
        );
    }

    #[test]
    fn vertical_lr_same_as_vertical_rl_for_height() {
        let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
            return;
        };
        style.writing_mode = WritingMode::VerticalLr;

        let children = dom.composed_children(parent);
        let block_dim = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!(
            (block_dim - style.font_size).abs() < f32::EPSILON,
            "vertical-lr single line should be font_size ({}), got {}",
            style.font_size,
            block_dim,
        );
    }

    #[test]
    fn horizontal_tb_uses_line_height() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
            return;
        };
        // Default writing_mode is HorizontalTb, no modification needed.

        let css_line_height = style.line_height.resolve_px(style.font_size);
        let children = dom.composed_children(parent);
        let h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        assert!(
            (h - css_line_height).abs() < f32::EPSILON,
            "horizontal-tb single line should be line-height ({css_line_height}), got {h}",
        );
    }

    // --- Step 1: Multi-style inline layout ---

    #[test]
    fn styled_runs_collect_from_nested_span() {
        let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
            return;
        };
        // Remove the empty text child and build: <p>Hello <span>World</span></p>
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        let text1 = dom.create_text("Hello ");
        dom.append_child(parent, text1);
        let span = dom.create_element("span", Attributes::default());
        let span_style = ComputedStyle {
            font_size: 24.0,
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(span, span_style);
        dom.append_child(parent, span);
        let text2 = dom.create_text("World");
        dom.append_child(span, text2);

        let children = dom.composed_children(parent);
        let runs = collect_styled_runs(&dom, &children, &style, parent);
        assert_eq!(runs.len(), 2, "should have 2 runs");
        assert_eq!(runs[0].text, "Hello ");
        assert!((runs[0].font_size - style.font_size).abs() < f32::EPSILON);
        assert_eq!(runs[1].text, "World");
        assert!((runs[1].font_size - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn multi_style_line_height_uses_max() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
            return;
        };
        // Build: <p>A<span style="font-size:32px">B</span></p>
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        let text1 = dom.create_text("A");
        dom.append_child(parent, text1);
        let span = dom.create_element("span", Attributes::default());
        let big_style = ComputedStyle {
            font_size: 32.0,
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let big_line_height = big_style.line_height.resolve_px(big_style.font_size);
        let _ = dom.world_mut().insert_one(span, big_style);
        dom.append_child(parent, span);
        let text2 = dom.create_text("B");
        dom.append_child(span, text2);

        let children = dom.composed_children(parent);
        let h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );
        // Line height should be max(parent line height, span line height) = big_line_height
        assert!(
            (h - big_line_height).abs() < 1.0,
            "line height should be the bigger style's line-height ({big_line_height}), got {h}",
        );
    }

    #[test]
    fn display_none_child_skipped_in_runs() {
        let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        let text1 = dom.create_text("visible");
        dom.append_child(parent, text1);
        let hidden = dom.create_element("span", Attributes::default());
        let hidden_style = ComputedStyle {
            display: Display::None,
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(hidden, hidden_style);
        dom.append_child(parent, hidden);
        let text2 = dom.create_text("hidden");
        dom.append_child(hidden, text2);

        let children = dom.composed_children(parent);
        let runs = collect_styled_runs(&dom, &children, &style, parent);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "visible");
    }

    // --- Step 2: Inline elements get LayoutBox ---

    #[test]
    fn inline_span_gets_layout_box() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        // Build: <p>Hello <span>World</span></p>
        let text1 = dom.create_text("Hello ");
        dom.append_child(parent, text1);
        let span = dom.create_element("span", Attributes::default());
        let span_style = ComputedStyle {
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(span, span_style);
        dom.append_child(parent, span);
        let text2 = dom.create_text("World");
        dom.append_child(span, text2);

        let children = dom.composed_children(parent);
        let _h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (10.0, 20.0),
            crate::layout_block_only,
        );

        // The span should now have a LayoutBox.
        let lb = dom.world().get::<&LayoutBox>(span);
        assert!(
            lb.is_ok(),
            "inline span should have a LayoutBox after layout"
        );
        let lb = lb.unwrap();
        // LayoutBox x should start after "Hello " at content_origin.x + offset.
        assert!(lb.content.x >= 10.0, "span x should be >= content_origin.x");
        assert!(
            (lb.content.y - 20.0).abs() < f32::EPSILON,
            "span y should be content_origin.y"
        );
        assert!(lb.content.width > 0.0, "span should have positive width");
        assert!(lb.content.height > 0.0, "span should have positive height");
    }

    #[test]
    fn parent_entity_does_not_get_inline_layout_box() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
            return;
        };

        // Parent should NOT get a LayoutBox from inline layout.
        let children = dom.composed_children(parent);
        let _h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );

        // Parent (the <p>) should not get LayoutBox from inline layout
        // (it's the parent_entity, excluded from inline LayoutBox assignment).
        assert!(
            dom.world().get::<&LayoutBox>(parent).is_err(),
            "parent entity should not get LayoutBox from inline layout"
        );
    }

    // --- Step 3: Atomic inline boxes (InlineBlock) ---

    #[test]
    fn inline_block_participates_in_ifc() {
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        // Build: <p>Hello <span style="display:inline-block; width:50px; height:30px">X</span> World</p>
        let text1 = dom.create_text("Hello ");
        dom.append_child(parent, text1);
        let ib = dom.create_element("span", Attributes::default());
        let ib_style = ComputedStyle {
            display: Display::InlineBlock,
            width: Dimension::Length(50.0),
            height: Dimension::Length(30.0),
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(ib, ib_style);
        dom.append_child(parent, ib);
        let ib_text = dom.create_text("X");
        dom.append_child(ib, ib_text);
        let text2 = dom.create_text(" World");
        dom.append_child(parent, text2);

        let children = dom.composed_children(parent);
        let h = layout_inline_context(
            &mut dom,
            &children,
            800.0,
            &style,
            &font_db,
            parent,
            (0.0, 0.0),
            crate::layout_block_only,
        );

        // The inline-block should get a LayoutBox from dispatch.
        let ib_lb = dom.world().get::<&LayoutBox>(ib);
        assert!(ib_lb.is_ok(), "inline-block should have a LayoutBox");
        let ib_lb = ib_lb.unwrap();
        assert!(
            (ib_lb.content.width - 50.0).abs() < f32::EPSILON,
            "inline-block width should be 50px, got {}",
            ib_lb.content.width
        );

        // Line height should be at least 30px (the inline-block's height).
        assert!(
            h >= 30.0,
            "line height should be >= inline-block height (30px), got {h}"
        );
    }

    #[test]
    fn inline_block_not_block_level() {
        assert!(
            !crate::block::is_block_level(Display::InlineBlock),
            "InlineBlock should not be block-level"
        );
        assert!(
            !crate::block::is_block_level(Display::InlineFlex),
            "InlineFlex should not be block-level"
        );
        assert!(
            !crate::block::is_block_level(Display::InlineGrid),
            "InlineGrid should not be block-level"
        );
        assert!(
            !crate::block::is_block_level(Display::InlineTable),
            "InlineTable should not be block-level"
        );
    }

    // --- Step 4: Anonymous block boxes (CSS 2.1 §9.2.1.1) ---

    #[test]
    fn mixed_block_inline_anonymous_box() {
        // <div>text <p style="display:block;height:40px">block</p> more text</div>
        // The text before and after the <p> should be wrapped in anonymous
        // block boxes and contribute height.
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        // Insert parent's ComputedStyle so stack_block_children can look it up.
        let _ = dom.world_mut().insert_one(parent, style.clone());

        let text1 = dom.create_text("Hello ");
        dom.append_child(parent, text1);
        let block = dom.create_element("p", Attributes::default());
        let block_style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(block, block_style);
        dom.append_child(parent, block);
        let text2 = dom.create_text(" World");
        dom.append_child(parent, text2);

        let children_list = dom.composed_children(parent);
        let input = crate::LayoutInput {
            containing_width: 800.0,
            containing_height: None,
            offset_x: 0.0,
            offset_y: 0.0,
            font_db: &font_db,
            depth: 0,
            float_ctx: None,
        };
        let result = crate::block::stack_block_children(
            &mut dom,
            &children_list,
            &input,
            crate::layout_block_only,
            false,
            parent,
        );

        // The block child has height 40. The text runs add line height.
        assert!(
            result.height >= 40.0,
            "height should be at least block child height (40), got {}",
            result.height
        );
        // With text content, anonymous boxes add inline layout height.
        let line_h = style.line_height.resolve_px(style.font_size);
        // Two anonymous blocks (before + after) + one block child = total.
        let expected_min = 40.0 + line_h; // at least one anonymous box contributes
        assert!(
            result.height >= expected_min,
            "height should include anonymous box height ({expected_min}), got {}",
            result.height
        );
    }

    #[test]
    fn block_only_children_no_anonymous_boxes() {
        // All children are block-level: no anonymous block boxes created.
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }
        let _ = dom.world_mut().insert_one(parent, style.clone());

        let block1 = dom.create_element("div", Attributes::default());
        let block_style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(20.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(block1, block_style.clone());
        dom.append_child(parent, block1);

        let block2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(block2, block_style);
        dom.append_child(parent, block2);

        let children_list = dom.composed_children(parent);
        let input = crate::LayoutInput {
            containing_width: 800.0,
            containing_height: None,
            offset_x: 0.0,
            offset_y: 0.0,
            font_db: &font_db,
            depth: 0,
            float_ctx: None,
        };
        let result = crate::block::stack_block_children(
            &mut dom,
            &children_list,
            &input,
            crate::layout_block_only,
            false,
            parent,
        );

        // Two blocks at 20px each = 40px.
        assert!(
            (result.height - 40.0).abs() < f32::EPSILON,
            "height should be 40.0 (2 × 20), got {}",
            result.height
        );
    }

    #[test]
    fn display_none_skipped_in_block_context() {
        // display:none children should not appear in inline runs.
        let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }
        let _ = dom.world_mut().insert_one(parent, style.clone());

        let hidden = dom.create_element("span", Attributes::default());
        let hidden_style = ComputedStyle {
            display: Display::None,
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(hidden, hidden_style);
        dom.append_child(parent, hidden);
        let hidden_text = dom.create_text("invisible");
        dom.append_child(hidden, hidden_text);

        let block = dom.create_element("div", Attributes::default());
        let block_style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(block, block_style);
        dom.append_child(parent, block);

        let children_list = dom.composed_children(parent);
        let input = crate::LayoutInput {
            containing_width: 800.0,
            containing_height: None,
            offset_x: 0.0,
            offset_y: 0.0,
            font_db: &font_db,
            depth: 0,
            float_ctx: None,
        };
        let result = crate::block::stack_block_children(
            &mut dom,
            &children_list,
            &input,
            crate::layout_block_only,
            false,
            parent,
        );

        // Only the block child contributes height; hidden span skipped.
        assert!(
            (result.height - 30.0).abs() < f32::EPSILON,
            "height should be 30.0 (block only), got {}",
            result.height
        );
    }

    #[test]
    fn atomic_inline_skipped_in_styled_runs() {
        let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
            return;
        };
        let children = dom.composed_children(parent);
        for &c in &children {
            dom.remove_child(parent, c);
        }

        // Build: <p>Hello <span style="display:inline-block">IB</span> World</p>
        let text1 = dom.create_text("Hello ");
        dom.append_child(parent, text1);
        let ib = dom.create_element("span", Attributes::default());
        let ib_style = ComputedStyle {
            display: Display::InlineBlock,
            font_family: style.font_family.clone(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(ib, ib_style);
        dom.append_child(parent, ib);
        let ib_text = dom.create_text("IB");
        dom.append_child(ib, ib_text);
        let text2 = dom.create_text(" World");
        dom.append_child(parent, text2);

        let children = dom.composed_children(parent);
        // collect_styled_runs should NOT include the InlineBlock's text.
        let runs = collect_styled_runs(&dom, &children, &style, parent);
        assert_eq!(runs.len(), 2, "should have 2 text runs (Hello + World)");
        assert_eq!(runs[0].text, "Hello ");
        assert_eq!(runs[1].text, " World");
    }
}
