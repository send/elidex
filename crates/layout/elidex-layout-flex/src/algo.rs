//! Flexbox algorithm phases: line splitting, flexible length resolution,
//! cross-size resolution, and positioning.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    clamp_min_max, resolve_explicit_height, sanitize, total_gap, LayoutInput,
};
use elidex_plugin::{
    AlignItems, BoxSizing, ComputedStyle, CssSize, Dimension, Direction, FlexWrap, LayoutBox,
    Point, Rect,
};

use super::align::{apply_justify_safety, compute_justify_offsets};

use super::{is_reversed, FlexContext, FlexItem};

/// Re-export shared layout environment from block crate.
pub(crate) use elidex_layout_block::LayoutEnv;

/// Line geometry for positioning items.
pub(crate) struct LineGeometry<'a> {
    pub(crate) line_ranges: &'a [(usize, usize)],
    pub(crate) line_cross_sizes: &'a [f32],
    pub(crate) line_offsets: &'a [f32],
    /// Per-line maximum baseline for baseline alignment (CSS Flexbox §9.4).
    pub(crate) line_baselines: &'a [f32],
    pub(crate) container_cross: f32,
}

/// Epsilon for floating-point comparison in the flex freeze loop.
///
/// Values within this tolerance of their min/max-clamped result are
/// considered "not violated" — the final clamp pass after the loop
/// corrects any remaining sub-pixel differences.
const FLEX_FREEZE_EPSILON: f32 = 0.001;

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

// Each flex item is laid out up to 3 times (collect_flex_items for content
// sizing, layout_items_cross for cross-size, position_items for final
// placement). Items with explicit cross sizes skip the layout call here,
// reducing redundant work for deep subtrees.
pub(crate) fn layout_items_cross(
    dom: &mut EcsDom,
    items: &mut [FlexItem],
    ctx: &FlexContext,
    env: &LayoutEnv<'_>,
) {
    for item in items.iter_mut() {
        // Optimization: skip layout if the item has an explicit cross size.
        // The cross size is known without layout, avoiding a redundant pass.
        if let Some(explicit_cross) = resolve_explicit_cross(dom, item.entity, ctx) {
            item.final_cross = explicit_cross + item.pb_cross;
            continue;
        }
        let child_containing = if ctx.horizontal {
            item.final_main
        } else {
            ctx.content_width
        };
        // In vertical writing mode, inline size is height, not width.
        // If height is indefinite, use 0.0 (not width — that's the wrong axis).
        let child_inline = if ctx.writing_mode.is_horizontal() {
            child_containing
        } else {
            ctx.resolved_height.unwrap_or(0.0)
        };
        let child_input = LayoutInput {
            containing: CssSize {
                width: child_containing,
                height: ctx.resolved_height,
            },
            containing_inline_size: child_inline,
            viewport: env.viewport,
            ..LayoutInput::probe(env, child_containing)
        };
        let child_lb = (env.layout_child)(dom, item.entity, &child_input).layout_box;
        item.final_cross = if ctx.horizontal {
            child_lb.content.size.height + item.pb_cross
        } else {
            child_lb.content.size.width + item.pb_cross
        };
        // child_lb is used only for cross-size computation above; descendants
        // will be re-laid out at the final position in position_items.
    }
}

/// Try to resolve the cross-axis content size from the item's explicit style,
/// avoiding a layout call. Returns `None` if the cross size depends on content.
fn resolve_explicit_cross(dom: &EcsDom, entity: Entity, ctx: &FlexContext) -> Option<f32> {
    let style = elidex_layout_block::get_style(dom, entity);
    let dim = if ctx.horizontal {
        style.height
    } else {
        style.width
    };
    match dim {
        Dimension::Length(px) if px.is_finite() => {
            let pb = if ctx.horizontal {
                let p = elidex_layout_block::resolve_padding(&style, ctx.inline_containing);
                let b = elidex_layout_block::sanitize_border(&style);
                p.top + b.top + p.bottom + b.bottom
            } else {
                let p = elidex_layout_block::resolve_padding(&style, ctx.inline_containing);
                let b = elidex_layout_block::sanitize_border(&style);
                p.left + b.left + p.right + b.right
            };
            if style.box_sizing == BoxSizing::BorderBox {
                Some((px - pb).max(0.0))
            } else {
                Some(px.max(0.0))
            }
        }
        Dimension::Percentage(pct) => {
            // Percentage cross size needs a definite containing size.
            let containing = if ctx.horizontal {
                ctx.resolved_height?
            } else {
                ctx.containing.width
            };
            let resolved = containing * pct / 100.0;
            let pb = if ctx.horizontal {
                let p = elidex_layout_block::resolve_padding(&style, ctx.inline_containing);
                let b = elidex_layout_block::sanitize_border(&style);
                p.top + b.top + p.bottom + b.bottom
            } else {
                let p = elidex_layout_block::resolve_padding(&style, ctx.inline_containing);
                let b = elidex_layout_block::sanitize_border(&style);
                p.left + b.left + p.right + b.right
            };
            if style.box_sizing == BoxSizing::BorderBox {
                Some((resolved - pb).max(0.0))
            } else {
                Some(resolved.max(0.0))
            }
        }
        _ => None,
    }
}

pub(crate) fn compute_line_cross_sizes(
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    line_baselines: &[f32],
) -> (Vec<f32>, f32) {
    let sizes: Vec<f32> = line_ranges
        .iter()
        .enumerate()
        .map(|(idx, &(s, e))| {
            let normal_max = items[s..e]
                .iter()
                .map(|i| i.final_cross + i.margin_cross)
                .fold(0.0_f32, f32::max);
            let line_bl = line_baselines.get(idx).copied().unwrap_or(0.0);
            if line_bl <= 0.0 {
                return normal_max;
            }
            // CSS Flexbox §9.4: line cross size must accommodate both
            // the tallest baseline-above and tallest baseline-below portions.
            let baseline_below = items[s..e]
                .iter()
                .filter(|i| {
                    i.align == AlignItems::Baseline
                        && !i.margin_cross_start_auto
                        && !i.margin_cross_end_auto
                })
                .map(|i| (i.final_cross + i.margin_cross) - i.baseline_or_synthesized())
                .fold(0.0_f32, f32::max);
            normal_max.max(line_bl + baseline_below)
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

/// Resolve the flex container's cross-axis size.
///
/// The cross axis is perpendicular to the main axis:
/// - `flex-direction: row` + `horizontal-tb` → cross = height (vertical)
/// - `flex-direction: column` + `horizontal-tb` → cross = width (horizontal)
/// - `flex-direction: row` + `vertical-rl/lr` → cross = width (horizontal)
/// - `flex-direction: column` + `vertical-rl/lr` → cross = height (vertical)
///
/// If the cross-axis dimension is explicitly set (via `width` or `height`
/// depending on `ctx.horizontal`), that value is used. Otherwise falls back
/// to `total_line_cross` (the sum of all flex line cross sizes).
pub(crate) fn resolve_container_cross(
    style: &ComputedStyle,
    ctx: &FlexContext,
    containing_width: f32,
    total_line_cross: f32,
) -> f32 {
    let explicit = if ctx.horizontal {
        resolve_explicit_height(style, ctx.containing.height)
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
/// offset within the line's cross space. For baseline alignment, `line_max_baseline`
/// is the maximum baseline among baseline-participating items in the line.
fn cross_align_offset(item: &FlexItem, line_cross: f32, line_max_baseline: f32) -> f32 {
    debug_assert!(
        line_cross >= 0.0,
        "line_cross must be non-negative: {line_cross}"
    );
    let item_outer_cross = item.final_cross + item.margin_cross;
    match item.align {
        AlignItems::FlexEnd => line_cross - item_outer_cross,
        AlignItems::Center => (line_cross - item_outer_cross) / 2.0,
        AlignItems::Baseline => {
            // CSS Flexbox §9.4: align item's baseline with the line's max baseline.
            (line_max_baseline - item.baseline_or_synthesized()).max(0.0)
        }
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
fn relayout_item_at_position(
    dom: &mut EcsDom,
    item: &FlexItem,
    ctx: &FlexContext,
    margin_box_pos: Point,
    env: &LayoutEnv<'_>,
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
    // Also zero out auto margins — flex handles them via position offsets;
    // block layout must not apply its own margin-auto centering on top.
    {
        let mut style = elidex_layout_block::get_style(dom, item.entity);
        if ctx.horizontal {
            style.width = Dimension::Length(item.final_main);
            style.height = Dimension::Length(item_content_height);
        } else {
            style.width = Dimension::Length(item_content_width);
            style.height = Dimension::Length(item.final_main);
        }
        // Zero any auto margins: flex handles them via position offsets.
        for m in [
            &mut style.margin_top,
            &mut style.margin_right,
            &mut style.margin_bottom,
            &mut style.margin_left,
        ] {
            if *m == Dimension::Auto {
                *m = Dimension::Length(0.0);
            }
        }
        let _ = dom.world_mut().insert_one(item.entity, style);
    }

    // Flex §9.9: stretched items have a definite cross size for their children's
    // percentage-height resolution. Pass the resolved cross size as the
    // containing height (for row flex) or containing width (for column flex).
    let child_containing_width;
    let child_containing_height;
    if ctx.horizontal {
        child_containing_width = ctx.containing.width;
        child_containing_height = if item.align == AlignItems::Stretch && item.cross_size_auto {
            Some(item_content_height)
        } else {
            ctx.resolved_height
        };
    } else {
        // Column flex: cross axis is width. Stretched items get a definite width.
        child_containing_width = if item.align == AlignItems::Stretch && item.cross_size_auto {
            item_content_width
        } else {
            ctx.containing.width
        };
        child_containing_height = ctx.resolved_height;
    }

    // Re-layout the item at its final margin-box position so
    // descendants get correct absolute coordinates.
    // In vertical writing mode, the inline dimension is height, not width.
    // If height is indefinite, use 0.0 (not width — that's the block axis).
    let child_inline_size = if ctx.writing_mode.is_horizontal() {
        child_containing_width
    } else {
        child_containing_height.unwrap_or(0.0)
    };
    let child_input = LayoutInput {
        containing: CssSize {
            width: child_containing_width,
            height: child_containing_height,
        },
        containing_inline_size: child_inline_size,
        offset: margin_box_pos,
        font_db: env.font_db,
        depth: env.depth + 1,
        float_ctx: None,
        viewport: env.viewport,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: env.layout_generation,
    };
    let child_lb = (env.layout_child)(dom, item.entity, &child_input).layout_box;

    // Overwrite the item's LayoutBox with flex-resolved dimensions.
    // child_lb.content.origin.x/y already include margin + border + padding
    // offsets from the margin-box position, so use them directly.
    let lb = LayoutBox {
        content: Rect::new(
            child_lb.content.origin.x,
            child_lb.content.origin.y,
            item_content_width,
            item_content_height,
        ),
        padding: child_lb.padding,
        border: child_lb.border,
        margin: child_lb.margin,
        first_baseline: child_lb.first_baseline,
        layout_generation: env.layout_generation,
    };
    let _ = dom.world_mut().insert_one(item.entity, lb);
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub(crate) fn position_items(
    dom: &mut EcsDom,
    items: &[FlexItem],
    lines: &LineGeometry<'_>,
    ctx: &FlexContext,
    env: &LayoutEnv<'_>,
) {
    let line_ranges = lines.line_ranges;
    let line_cross_sizes = lines.line_cross_sizes;
    let line_offsets = lines.line_offsets;
    let container_cross = lines.container_cross;
    // CSS Flexbox §4.2: RTL flips the main-axis direction for row layouts.
    // Row + RTL → reversed, RowReverse + RTL → not reversed (double reversal).
    let reversed_main = if ctx.horizontal && ctx.css_direction == Direction::Rtl {
        !is_reversed(ctx.direction)
    } else {
        is_reversed(ctx.direction)
    };
    let reversed_cross = matches!(ctx.wrap, FlexWrap::WrapReverse);

    for (line_idx, &(start, end)) in line_ranges.iter().enumerate() {
        let line_items = &items[start..end];
        let line_cross = line_cross_sizes[line_idx];
        let line_bl = lines.line_baselines.get(line_idx).copied().unwrap_or(0.0);
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

        // Flex §8.1: count auto main margins. When present, they absorb free space
        // and override justify-content.
        let auto_main_count: usize = line_items
            .iter()
            .map(|i| usize::from(i.margin_main_start_auto) + usize::from(i.margin_main_end_auto))
            .sum();
        let auto_main_per = if auto_main_count > 0 && free_space > 0.0 {
            free_space / auto_main_count as f32
        } else {
            0.0
        };

        // When auto margins are present, they absorb free space; skip justify-content.
        let (mut main_cursor, justify_gap) = if auto_main_count > 0 {
            (0.0, 0.0)
        } else {
            compute_justify_offsets(
                apply_justify_safety(ctx.justify, free_space, ctx.justify_content_safety),
                free_space,
                line_items.len(),
            )
        };
        // Effective gap = CSS gap + justify-content gap.
        let gap = ctx.gap_main + justify_gap;

        if reversed_main {
            main_cursor = ctx.container_main - main_cursor;
        }

        for item in line_items {
            let item_outer_main = item.final_main + item.pb_main + item.margin_main;

            // Flex §8.1: compute auto margin adjustments for this item.
            let auto_margin_main_start = if item.margin_main_start_auto {
                auto_main_per
            } else {
                0.0
            };
            let auto_margin_main_end = if item.margin_main_end_auto {
                auto_main_per
            } else {
                0.0
            };
            let auto_margin_outer = auto_margin_main_start + auto_margin_main_end;

            if reversed_main {
                main_cursor -= item_outer_main + auto_margin_outer;
            }

            // Flex §8.1: cross-axis auto margins.
            let (cross_auto_offset, skip_align) = {
                let both = item.margin_cross_start_auto && item.margin_cross_end_auto;
                let start_only = item.margin_cross_start_auto && !item.margin_cross_end_auto;
                let end_only = !item.margin_cross_start_auto && item.margin_cross_end_auto;
                let item_outer_cross = item.final_cross + item.margin_cross;
                let cross_free = (line_cross - item_outer_cross).max(0.0);
                if both {
                    (cross_free / 2.0, true)
                } else if end_only {
                    (0.0, true) // start-aligned (absorb into end margin)
                } else if start_only {
                    (cross_free, true) // end-aligned (absorb into start margin)
                } else {
                    (0.0, false)
                }
            };

            let align_offset = if skip_align {
                cross_auto_offset
            } else {
                cross_align_offset(item, line_cross, line_bl)
            };

            // Margin-box position: layout adds margins internally.
            // Auto main margin shifts the item's start position.
            let (margin_box_x, margin_box_y) = if ctx.horizontal {
                (
                    ctx.content_origin.x + main_cursor + auto_margin_main_start,
                    ctx.content_origin.y + cross_offset + align_offset,
                )
            } else {
                (
                    ctx.content_origin.x + cross_offset + align_offset,
                    ctx.content_origin.y + main_cursor + auto_margin_main_start,
                )
            };

            relayout_item_at_position(dom, item, ctx, Point::new(margin_box_x, margin_box_y), env);

            if reversed_main {
                main_cursor -= gap;
            } else {
                main_cursor += item_outer_main + auto_margin_outer + gap;
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
        resolve_explicit_height(style, ctx.containing.height)
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
        resolve_explicit_height(style, ctx.containing.height).unwrap_or(max_line_main)
    }
}

// Justify-content and align-content helpers are in the `align` module.
// Baseline helpers are in the `baseline` module.
