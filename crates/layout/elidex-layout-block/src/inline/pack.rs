//! Line packing: `LinePacker`, `PackItem`, `build_pack_items`, `assign_inline_layout_boxes`.

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, EdgeSizes, LayoutBox, Point, Rect};
use elidex_text::{measure_text, BreakOpportunity, FontDatabase};

use super::measure::measure_segment_widths;
use super::InlineItem;

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
    parent_entity: Entity,
    /// First baseline offset from the inline formatting context top.
    /// Captured from the first text run on the first line.
    pub first_baseline: Option<f32>,
}

impl LinePacker {
    pub fn new(parent_entity: Entity) -> Self {
        Self {
            line_boxes: Vec::new(),
            entity_bounds: HashMap::new(),
            static_positions: HashMap::new(),
            current_inline: 0.0,
            current_line_height: 0.0,
            current_block_offset: 0.0,
            on_line: false,
            parent_entity,
            first_baseline: None,
        }
    }

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

                // Capture first baseline: from the first text run on the first line.
                // CSS 2.1 §10.8.1: baseline = line_y + half_leading + ascent
                // half_leading = (line_height - (ascent - descent)) / 2
                if self.first_baseline.is_none() && !is_vertical {
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
                self.static_positions.insert(
                    *entity,
                    Point::new(self.current_inline, self.current_block_offset),
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

    pub fn finish(&mut self) {
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
pub(super) fn assign_inline_layout_boxes(
    dom: &mut EcsDom,
    entity_bounds: &HashMap<Entity, EntityBounds>,
    content_origin: Point,
    is_vertical: bool,
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
        };
        let _ = dom.world_mut().insert_one(*entity, lb);
    }
}
