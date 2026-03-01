//! Flexbox algorithm phases: line splitting, flexible length resolution,
//! cross-size resolution, and positioning.

use elidex_ecs::EcsDom;
use elidex_plugin::{
    AlignContent, AlignItems, ComputedStyle, Dimension, FlexWrap, JustifyContent, LayoutBox, Rect,
};
use elidex_text::FontDatabase;

use crate::block::layout_block;
use crate::sanitize;

use super::{is_reversed, resolve_explicit_height, FlexContext, FlexItem};

// ---------------------------------------------------------------------------
// Line splitting (§9.3)
// ---------------------------------------------------------------------------

/// Build line ranges from items. Returns `Vec<(start, end)>`.
pub(super) fn build_line_ranges(
    items: &[FlexItem],
    container_main: f32,
    wrap: FlexWrap,
) -> Vec<(usize, usize)> {
    let line_lengths = split_into_lines(items, container_main, wrap);
    let mut ranges = Vec::with_capacity(line_lengths.len());
    let mut start = 0;
    for len in line_lengths {
        ranges.push((start, start + len));
        start += len;
    }
    ranges
}

fn split_into_lines(items: &[FlexItem], container_main: f32, wrap: FlexWrap) -> Vec<usize> {
    if items.is_empty() {
        return Vec::new();
    }
    if matches!(wrap, FlexWrap::Nowrap) {
        return vec![items.len()];
    }

    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut line_main = 0.0_f32;
    for (i, item) in items.iter().enumerate() {
        let item_main = item.hypo_main + item.pb_main + item.margin_main;
        if i > line_start && line_main + item_main > container_main {
            lines.push(i - line_start);
            line_start = i;
            line_main = 0.0;
        }
        line_main += item_main;
    }
    lines.push(items.len() - line_start);
    lines
}

// ---------------------------------------------------------------------------
// Flexible length resolution (§9.7)
// ---------------------------------------------------------------------------

// TODO(Phase 3): Replace with spec-compliant frozen/unfrozen 2-pass loop (CSS §9.7)
// once min-width/max-width constraints are implemented. The current single-pass is
// sufficient while those constraints are absent.
pub(super) fn resolve_flexible_lengths(items: &mut [FlexItem], container_main: f32) {
    if items.is_empty() {
        return;
    }
    let total_hypo: f32 = items
        .iter()
        .map(|i| i.hypo_main + i.pb_main + i.margin_main)
        .sum();
    let free_space = container_main - total_hypo;

    if free_space > 0.0 {
        let total_grow: f32 = items.iter().map(|i| i.grow).sum();
        if total_grow > 0.0 {
            for item in items.iter_mut() {
                if item.grow > 0.0 {
                    let portion = free_space * (item.grow / total_grow);
                    item.final_main = (item.hypo_main + portion).max(0.0);
                }
            }
        }
    } else if free_space < 0.0 {
        let total_shrink_scaled: f32 = items.iter().map(|i| i.shrink * i.hypo_main).sum();
        if total_shrink_scaled > 0.0 {
            for item in items.iter_mut() {
                if item.shrink > 0.0 {
                    let scaled = item.shrink * item.hypo_main;
                    let portion = free_space.abs() * (scaled / total_shrink_scaled);
                    item.final_main = (item.hypo_main - portion).max(0.0);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-size resolution & stretching
// ---------------------------------------------------------------------------

// TODO(Phase 3): Each flex item is laid out up to 3 times (collect_flex_items
// for content sizing, layout_items_cross for cross-size, position_items for
// final placement). Consider caching intrinsic sizes to reduce redundant work
// for items with deep subtrees.
pub(super) fn layout_items_cross(
    dom: &mut EcsDom,
    items: &mut [FlexItem],
    ctx: &FlexContext,
    font_db: &FontDatabase,
) {
    for item in items.iter_mut() {
        let child_containing = if ctx.horizontal {
            item.final_main
        } else {
            ctx.content_width
        };
        let child_lb = layout_block(dom, item.entity, child_containing, 0.0, 0.0, font_db);
        item.final_cross = if ctx.horizontal {
            child_lb.content.height + item.pb_cross
        } else {
            child_lb.content.width + item.pb_cross
        };
        // child_lb is used only for cross-size computation above; descendants
        // will be re-laid out at the final position in position_items.
    }
}

pub(super) fn compute_line_cross_sizes(
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
) -> (Vec<f32>, f32) {
    let sizes: Vec<f32> = line_ranges
        .iter()
        .map(|&(s, e)| {
            items[s..e]
                .iter()
                .map(|i| i.final_cross + i.margin_cross)
                .fold(0.0_f32, f32::max)
        })
        .collect();
    let total = sizes.iter().sum();
    (sizes, total)
}

pub(super) fn stretch_items(
    items: &mut [FlexItem],
    line_ranges: &[(usize, usize)],
    line_cross_sizes: &[f32],
) {
    for (idx, &(start, end)) in line_ranges.iter().enumerate() {
        let line_cross = line_cross_sizes[idx];
        for item in &mut items[start..end] {
            // CSS spec: stretch only applies when align is Stretch AND cross-size is auto.
            if item.align == AlignItems::Stretch && item.cross_size_auto {
                let available = line_cross - item.margin_cross;
                if available > item.final_cross {
                    item.final_cross = available;
                }
            }
        }
    }
}

pub(super) fn resolve_container_cross(
    style: &ComputedStyle,
    ctx: &FlexContext,
    containing_width: f32,
    total_line_cross: f32,
) -> f32 {
    let explicit = if ctx.horizontal {
        resolve_explicit_height(style)
    } else {
        match style.width {
            Dimension::Length(px) => Some(sanitize(px)),
            Dimension::Percentage(pct) => Some(sanitize(containing_width * pct / 100.0)),
            Dimension::Auto => None,
        }
    };
    explicit.unwrap_or(total_line_cross)
}

// ---------------------------------------------------------------------------
// Positioning
// ---------------------------------------------------------------------------

/// Compute the cross-axis alignment offset for a single flex item.
///
/// Based on the item's `align-self` (resolved to `AlignItems`), returns the
/// offset within the line's cross space.
fn cross_align_offset(item: &FlexItem, line_cross: f32) -> f32 {
    debug_assert!(
        line_cross >= 0.0,
        "line_cross must be non-negative: {line_cross}"
    );
    let item_outer_cross = item.final_cross + item.margin_cross;
    match item.align {
        AlignItems::FlexEnd => line_cross - item_outer_cross,
        AlignItems::Center => (line_cross - item_outer_cross) / 2.0,
        _ => 0.0,
    }
}

/// Re-layout a flex item at its final position and overwrite its `LayoutBox`
/// with flex-resolved dimensions.
///
/// This overwrites the item's `ComputedStyle` width/height to the flex-resolved
/// values (NOT restored — re-layout requires re-resolving styles first), then
/// runs `layout_block` to position descendants, and finally patches the
/// `LayoutBox` with the correct flex content size.
fn relayout_item_at_position(
    dom: &mut EcsDom,
    item: &FlexItem,
    ctx: &FlexContext,
    margin_box_x: f32,
    margin_box_y: f32,
    font_db: &FontDatabase,
) {
    let item_content_width = if ctx.horizontal {
        item.final_main
    } else {
        (item.final_cross - item.pb_cross).max(0.0)
    };
    let item_content_height = if ctx.horizontal {
        (item.final_cross - item.pb_cross).max(0.0)
    } else {
        item.final_main
    };

    // Overwrite the item's width/height to flex-resolved values.
    {
        let mut style = crate::get_style(dom, item.entity);
        if ctx.horizontal {
            style.width = Dimension::Length(item.final_main);
            style.height = Dimension::Length(item_content_height);
        } else {
            style.width = Dimension::Length(item_content_width);
            style.height = Dimension::Length(item.final_main);
        }
        let _ = dom.world_mut().insert_one(item.entity, style);
    }

    // Re-layout the item at its final margin-box position so
    // descendants get correct absolute coordinates.
    let child_lb = layout_block(
        dom,
        item.entity,
        ctx.containing_width,
        margin_box_x,
        margin_box_y,
        font_db,
    );

    // Overwrite the item's LayoutBox with flex-resolved dimensions.
    // child_lb.content.x/y already include margin + border + padding
    // offsets from the margin-box position, so use them directly.
    let lb = LayoutBox {
        content: Rect {
            x: child_lb.content.x,
            y: child_lb.content.y,
            width: item_content_width,
            height: item_content_height,
        },
        padding: child_lb.padding,
        border: child_lb.border,
        margin: child_lb.margin,
    };
    let _ = dom.world_mut().insert_one(item.entity, lb);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn position_items(
    dom: &mut EcsDom,
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    line_cross_sizes: &[f32],
    line_offsets: &[f32],
    ctx: &FlexContext,
    container_cross: f32,
    font_db: &FontDatabase,
) {
    let reversed_main = is_reversed(ctx.direction);
    let reversed_cross = matches!(ctx.wrap, FlexWrap::WrapReverse);

    for (line_idx, &(start, end)) in line_ranges.iter().enumerate() {
        let line_items = &items[start..end];
        let line_cross = line_cross_sizes[line_idx];
        let cross_offset = if reversed_cross {
            container_cross - line_offsets[line_idx] - line_cross
        } else {
            line_offsets[line_idx]
        };

        let total_main_used: f32 = line_items
            .iter()
            .map(|i| i.final_main + i.pb_main + i.margin_main)
            .sum();
        let free_space = (ctx.container_main - total_main_used).max(0.0);
        let (mut main_cursor, gap) =
            compute_justify_offsets(ctx.justify, free_space, line_items.len());

        if reversed_main {
            main_cursor = ctx.container_main - main_cursor;
        }

        for item in line_items {
            let item_outer_main = item.final_main + item.pb_main + item.margin_main;

            if reversed_main {
                main_cursor -= item_outer_main;
            }

            let align_offset = cross_align_offset(item, line_cross);

            // Margin-box position: layout_block adds margins internally.
            let (margin_box_x, margin_box_y) = if ctx.horizontal {
                (
                    ctx.content_x + main_cursor,
                    ctx.content_y + cross_offset + align_offset,
                )
            } else {
                (
                    ctx.content_x + cross_offset + align_offset,
                    ctx.content_y + main_cursor,
                )
            };

            relayout_item_at_position(dom, item, ctx, margin_box_x, margin_box_y, font_db);

            if reversed_main {
                main_cursor -= gap;
            } else {
                main_cursor += item_outer_main + gap;
            }
        }
    }
}

pub(super) fn compute_container_height(
    style: &ComputedStyle,
    ctx: &FlexContext,
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    total_line_cross: f32,
) -> f32 {
    if ctx.horizontal {
        resolve_explicit_height(style).unwrap_or(total_line_cross)
    } else {
        let max_line_main: f32 = line_ranges
            .iter()
            .map(|&(s, e)| {
                items[s..e]
                    .iter()
                    .map(|i| i.final_main + i.pb_main + i.margin_main)
                    .sum::<f32>()
            })
            .fold(0.0_f32, f32::max);
        resolve_explicit_height(style).unwrap_or(max_line_main)
    }
}

// ---------------------------------------------------------------------------
// Justify-content
// ---------------------------------------------------------------------------

/// Compute justify-content start offset and gap.
#[allow(clippy::cast_precision_loss)] // item counts are small
fn compute_justify_offsets(justify: JustifyContent, free_space: f32, count: usize) -> (f32, f32) {
    if count == 0 {
        return (0.0, 0.0);
    }
    let n = count as f32;
    match justify {
        JustifyContent::FlexStart => (0.0, 0.0),
        JustifyContent::FlexEnd => (free_space, 0.0),
        JustifyContent::Center => (free_space / 2.0, 0.0),
        JustifyContent::SpaceBetween => {
            if count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, free_space / (n - 1.0))
            }
        }
        JustifyContent::SpaceAround => {
            let gap = free_space / n;
            (gap / 2.0, gap)
        }
        JustifyContent::SpaceEvenly => {
            let gap = free_space / (n + 1.0);
            (gap, gap)
        }
    }
}

// ---------------------------------------------------------------------------
// Align-content
// ---------------------------------------------------------------------------

/// Result of align-content distribution.
pub(super) struct AlignContentResult {
    /// Starting cross offset for each line.
    pub(super) offsets: Vec<f32>,
    /// Effective cross sizes for each line (may be increased by stretch).
    pub(super) effective_line_sizes: Vec<f32>,
}

#[allow(clippy::cast_precision_loss)] // line counts are small
pub(super) fn compute_align_content_offsets(
    line_cross_sizes: &[f32],
    container_cross: f32,
    align_content: AlignContent,
    wrap: FlexWrap,
) -> AlignContentResult {
    let n = line_cross_sizes.len();
    if n == 0 {
        return AlignContentResult {
            offsets: Vec::new(),
            effective_line_sizes: Vec::new(),
        };
    }
    if matches!(wrap, FlexWrap::Nowrap) {
        return AlignContentResult {
            offsets: vec![0.0],
            effective_line_sizes: line_cross_sizes.to_vec(),
        };
    }

    let total: f32 = line_cross_sizes.iter().sum();
    let free = (container_cross - total).max(0.0);
    let nf = n as f32;

    let mut cursor = match align_content {
        AlignContent::FlexEnd => free,
        AlignContent::Center => free / 2.0,
        AlignContent::SpaceAround => free / (2.0 * nf),
        AlignContent::FlexStart | AlignContent::SpaceBetween | AlignContent::Stretch => 0.0,
    };

    let gap = match align_content {
        AlignContent::SpaceBetween => {
            if n <= 1 {
                0.0
            } else {
                free / (nf - 1.0)
            }
        }
        AlignContent::SpaceAround => free / nf,
        _ => 0.0,
    };

    let stretch_extra = if align_content == AlignContent::Stretch {
        free / nf
    } else {
        0.0
    };

    let mut offsets = Vec::with_capacity(n);
    let mut effective_line_sizes = Vec::with_capacity(n);
    for (i, &line_size) in line_cross_sizes.iter().enumerate() {
        offsets.push(cursor);
        effective_line_sizes.push(line_size + stretch_extra);
        cursor += line_size + stretch_extra;
        if i < n - 1 {
            cursor += gap;
        }
    }
    AlignContentResult {
        offsets,
        effective_line_sizes,
    }
}
