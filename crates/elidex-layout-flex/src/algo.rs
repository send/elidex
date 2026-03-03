//! Flexbox algorithm phases: line splitting, flexible length resolution,
//! cross-size resolution, and positioning.

use elidex_ecs::EcsDom;
use elidex_layout_block::{clamp_min_max, resolve_explicit_height, sanitize, ChildLayoutFn};
use elidex_plugin::{
    AlignContent, AlignItems, ComputedStyle, Dimension, FlexWrap, JustifyContent, LayoutBox, Rect,
};
use elidex_text::FontDatabase;

use super::{is_reversed, FlexContext, FlexItem};

/// Epsilon for floating-point comparison in the flex freeze loop.
///
/// Values within this tolerance of their min/max-clamped result are
/// considered "not violated" — the final clamp pass after the loop
/// corrects any remaining sub-pixel differences.
const FLEX_FREEZE_EPSILON: f32 = 0.001;

/// Total gap space between `count` items at `gap` spacing.
///
/// Returns `0.0` when there are fewer than 2 items.
#[allow(clippy::cast_precision_loss)]
fn total_gap(count: usize, gap: f32) -> f32 {
    if count > 1 {
        gap * (count - 1) as f32
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Line splitting (§9.3)
// ---------------------------------------------------------------------------

/// Build line ranges from items. Returns `Vec<(start, end)>`.
pub(crate) fn build_line_ranges(
    items: &[FlexItem],
    container_main: f32,
    wrap: FlexWrap,
    gap_main: f32,
) -> Vec<(usize, usize)> {
    let line_lengths = split_into_lines(items, container_main, wrap, gap_main);
    let mut ranges = Vec::with_capacity(line_lengths.len());
    let mut start = 0;
    for len in line_lengths {
        ranges.push((start, start + len));
        start += len;
    }
    ranges
}

fn split_into_lines(
    items: &[FlexItem],
    container_main: f32,
    wrap: FlexWrap,
    gap_main: f32,
) -> Vec<usize> {
    if items.is_empty() {
        return Vec::new();
    }
    if matches!(wrap, FlexWrap::Nowrap) {
        return vec![items.len()];
    }

    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut line_main = 0.0_f32;
    let mut line_count = 0_usize;
    for (i, item) in items.iter().enumerate() {
        let item_main = item.hypo_main + item.pb_main + item.margin_main;
        // Include gap between items when checking overflow.
        let gap_before = if line_count > 0 { gap_main } else { 0.0 };
        if i > line_start && line_main + gap_before + item_main > container_main {
            lines.push(i - line_start);
            line_start = i;
            line_main = item_main;
            line_count = 1;
        } else {
            line_main += gap_before + item_main;
            line_count += 1;
        }
    }
    lines.push(items.len() - line_start);
    lines
}

// ---------------------------------------------------------------------------
// Flexible length resolution (§9.7)
// ---------------------------------------------------------------------------

/// Resolve flexible lengths with min/max clamping (CSS §9.7).
///
/// Uses a simplified freeze loop: after initial grow/shrink distribution,
/// items that violate min/max constraints are frozen at their clamped size,
/// and remaining free space is redistributed among unfrozen items.
pub(crate) fn resolve_flexible_lengths(items: &mut [FlexItem], container_main: f32, gap_main: f32) {
    if items.is_empty() {
        return;
    }
    // Subtract total gap from available space.
    let total_gap = total_gap(items.len(), gap_main);

    let mut frozen = vec![false; items.len()];

    // Iterative freeze loop (max iterations = item count to guarantee termination).
    for _ in 0..items.len() {
        let total_hypo: f32 = items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                if frozen[i] {
                    it.final_main + it.pb_main + it.margin_main
                } else {
                    it.hypo_main + it.pb_main + it.margin_main
                }
            })
            .sum();
        let free_space = container_main - total_hypo - total_gap;

        if free_space > 0.0 {
            let total_grow: f32 = items
                .iter()
                .enumerate()
                .filter(|(i, _)| !frozen[*i])
                .map(|(_, it)| it.grow)
                .sum();
            if total_grow > 0.0 {
                for (i, item) in items.iter_mut().enumerate() {
                    if !frozen[i] && item.grow > 0.0 {
                        let portion = free_space * (item.grow / total_grow);
                        item.final_main = (item.hypo_main + portion).max(0.0);
                    }
                }
            }
        } else if free_space < 0.0 {
            // CSS Flexbox §9.7: scaled flex shrink factor uses flex base size.
            let total_shrink_scaled: f32 = items
                .iter()
                .enumerate()
                .filter(|(i, _)| !frozen[*i])
                .map(|(_, it)| it.shrink * it.flex_base_size)
                .sum();
            if total_shrink_scaled > 0.0 {
                for (i, item) in items.iter_mut().enumerate() {
                    if !frozen[i] && item.shrink > 0.0 {
                        let scaled = item.shrink * item.flex_base_size;
                        let portion = free_space.abs() * (scaled / total_shrink_scaled);
                        item.final_main = (item.hypo_main - portion).max(0.0);
                    }
                }
            }
        }

        // Freeze items that violate min/max constraints.
        let mut any_frozen = false;
        for (i, item) in items.iter_mut().enumerate() {
            if frozen[i] {
                continue;
            }
            let clamped = clamp_min_max(item.final_main, item.min_main, item.max_main);
            // Epsilon accounts for floating-point rounding during grow/shrink
            // distribution. The final clamp pass (after the loop) corrects any
            // remaining sub-pixel violations.
            if (clamped - item.final_main).abs() > FLEX_FREEZE_EPSILON {
                item.final_main = clamped;
                frozen[i] = true;
                any_frozen = true;
            }
        }
        if !any_frozen {
            break;
        }
    }

    // Final clamp for any remaining items.
    for item in items.iter_mut() {
        item.final_main = clamp_min_max(item.final_main, item.min_main, item.max_main);
    }
}

// ---------------------------------------------------------------------------
// Cross-size resolution & stretching
// ---------------------------------------------------------------------------

// TODO: Each flex item is laid out up to 3 times (collect_flex_items
// for content sizing, layout_items_cross for cross-size, position_items for
// final placement). Consider caching intrinsic sizes to reduce redundant work
// for items with deep subtrees.
pub(crate) fn layout_items_cross(
    dom: &mut EcsDom,
    items: &mut [FlexItem],
    ctx: &FlexContext,
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) {
    for item in items.iter_mut() {
        let child_containing = if ctx.horizontal {
            item.final_main
        } else {
            ctx.content_width
        };
        let child_lb = layout_child(
            dom,
            item.entity,
            child_containing,
            ctx.container_definite_height,
            0.0,
            0.0,
            font_db,
            depth + 1,
        );
        item.final_cross = if ctx.horizontal {
            child_lb.content.height + item.pb_cross
        } else {
            child_lb.content.width + item.pb_cross
        };
        // child_lb is used only for cross-size computation above; descendants
        // will be re-laid out at the final position in position_items.
    }
}

pub(crate) fn compute_line_cross_sizes(
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

pub(crate) fn stretch_items(
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

pub(crate) fn resolve_container_cross(
    style: &ComputedStyle,
    ctx: &FlexContext,
    containing_width: f32,
    total_line_cross: f32,
) -> f32 {
    let explicit = if ctx.horizontal {
        resolve_explicit_height(style, ctx.containing_height)
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
/// runs layout via `layout_child` to position descendants, and finally patches
/// the `LayoutBox` with the correct flex content size.
#[allow(clippy::too_many_arguments)]
fn relayout_item_at_position(
    dom: &mut EcsDom,
    item: &FlexItem,
    ctx: &FlexContext,
    margin_box_x: f32,
    margin_box_y: f32,
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
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
        let mut style = elidex_layout_block::get_style(dom, item.entity);
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
    let child_lb = layout_child(
        dom,
        item.entity,
        ctx.containing_width,
        ctx.container_definite_height,
        margin_box_x,
        margin_box_y,
        font_db,
        depth + 1,
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
pub(crate) fn position_items(
    dom: &mut EcsDom,
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    line_cross_sizes: &[f32],
    line_offsets: &[f32],
    ctx: &FlexContext,
    container_cross: f32,
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
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
        // Total gap between items on the main axis.
        let total_gap = total_gap(line_items.len(), ctx.gap_main);
        let free_space = (ctx.container_main - total_main_used - total_gap).max(0.0);
        let (mut main_cursor, justify_gap) =
            compute_justify_offsets(ctx.justify, free_space, line_items.len());
        // Effective gap = CSS gap + justify-content gap.
        let gap = ctx.gap_main + justify_gap;

        if reversed_main {
            main_cursor = ctx.container_main - main_cursor;
        }

        for item in line_items {
            let item_outer_main = item.final_main + item.pb_main + item.margin_main;

            if reversed_main {
                main_cursor -= item_outer_main;
            }

            let align_offset = cross_align_offset(item, line_cross);

            // Margin-box position: layout adds margins internally.
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

            relayout_item_at_position(
                dom,
                item,
                ctx,
                margin_box_x,
                margin_box_y,
                font_db,
                layout_child,
                depth,
            );

            if reversed_main {
                main_cursor -= gap;
            } else {
                main_cursor += item_outer_main + gap;
            }
        }
    }
}

pub(crate) fn compute_container_height(
    style: &ComputedStyle,
    ctx: &FlexContext,
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    total_line_cross: f32,
) -> f32 {
    if ctx.horizontal {
        // Auto height: include cross-axis gaps between lines.
        let cross_gaps = total_gap(line_ranges.len(), ctx.gap_cross);
        resolve_explicit_height(style, ctx.containing_height)
            .unwrap_or(total_line_cross + cross_gaps)
    } else {
        let max_line_main: f32 = line_ranges
            .iter()
            .map(|&(s, e)| {
                let item_sum: f32 = items[s..e]
                    .iter()
                    .map(|i| i.final_main + i.pb_main + i.margin_main)
                    .sum();
                // Add main-axis gap between items within the line.
                item_sum + total_gap(e - s, ctx.gap_main)
            })
            .fold(0.0_f32, f32::max);
        resolve_explicit_height(style, ctx.containing_height).unwrap_or(max_line_main)
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
pub(crate) struct AlignContentResult {
    /// Starting cross offset for each line.
    pub(crate) offsets: Vec<f32>,
    /// Effective cross sizes for each line (may be increased by stretch).
    pub(crate) effective_line_sizes: Vec<f32>,
}

#[allow(clippy::cast_precision_loss)] // line counts are small
pub(crate) fn compute_align_content_offsets(
    line_cross_sizes: &[f32],
    container_cross: f32,
    align_content: AlignContent,
    wrap: FlexWrap,
    gap_cross: f32,
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
    let total_cross_gap = total_gap(n, gap_cross);
    let free = (container_cross - total - total_cross_gap).max(0.0);
    let nf = n as f32;

    let mut cursor = match align_content {
        AlignContent::FlexEnd => free,
        AlignContent::Center => free / 2.0,
        AlignContent::SpaceAround => free / (2.0 * nf),
        AlignContent::SpaceEvenly => free / (nf + 1.0),
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
        AlignContent::SpaceEvenly => free / (nf + 1.0),
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
            cursor += gap + gap_cross;
        }
    }
    AlignContentResult {
        offsets,
        effective_line_sizes,
    }
}
