//! CSS Flexbox layout algorithm (CSS Flexbox Level 1, simplified).
//!
//! Implements the core flexbox algorithm: item collection, line splitting,
//! flexible length resolution, and cross/main axis alignment.
//!
//! Current simplifications:
//! - `flex-basis: content` treated as `auto`
//! - `baseline` alignment treated as `flex-start`
//! - `inline-flex` treated as block-level

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    adjust_min_max_for_border_box, block::resolve_margin, clamp_min_max, effective_align,
    horizontal_pb, resolve_explicit_height, resolve_min_max, sanitize, sanitize_border,
    sanitize_padding, vertical_pb, ChildLayoutFn, EmptyContainerParams, LayoutInput,
    MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    AlignContent, AlignItems, BoxSizing, ComputedStyle, Dimension, Direction, Display, EdgeSizes,
    FlexDirection, FlexWrap, JustifyContent, LayoutBox, Rect,
};
/// Sentinel value representing an indefinite container main-axis size.
///
/// Uses `f32::MAX / 2.0` (approximately 1.7e38) to avoid overflow when adding margins/padding,
/// while remaining large enough to never be reached by real layout values.
const INDEFINITE_MAIN_SIZE: f32 = f32::MAX / 2.0;

mod algo;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

// ---------------------------------------------------------------------------
// Axis helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the flex main axis is horizontal.
fn is_main_horizontal(dir: FlexDirection) -> bool {
    matches!(dir, FlexDirection::Row | FlexDirection::RowReverse)
}

/// Returns `true` if the main axis direction is reversed.
pub(crate) fn is_reversed(dir: FlexDirection) -> bool {
    matches!(
        dir,
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    )
}

/// Resolve a percentage dimension against the containing main-axis size.
///
/// Returns `None` if the container is indefinite (percentage cannot be resolved).
fn resolve_percentage_main(pct: f32, containing_main: f32) -> Option<f32> {
    if containing_main >= INDEFINITE_MAIN_SIZE {
        None
    } else {
        Some(containing_main * pct / 100.0)
    }
}

/// Resolve flex-basis to a main-axis size in pixels.
///
/// Returns `None` when content sizing is needed.
/// When `box-sizing: border-box`, subtracts main-axis padding and border
/// from the resolved value so it represents content size.
fn resolve_flex_basis(
    style: &ComputedStyle,
    direction: FlexDirection,
    containing_main: f32,
) -> Option<f32> {
    let raw = match style.flex_basis {
        Dimension::Length(px) => Some(px),
        Dimension::Percentage(pct) => resolve_percentage_main(pct, containing_main),
        Dimension::Auto => {
            let fallback = if is_main_horizontal(direction) {
                style.width
            } else {
                style.height
            };
            match fallback {
                Dimension::Length(px) => Some(px),
                Dimension::Percentage(pct) => resolve_percentage_main(pct, containing_main),
                Dimension::Auto => None,
            }
        }
    };
    let px = raw?;
    // Adjust for box-sizing: border-box — convert from border-box to content size.
    if style.box_sizing == BoxSizing::BorderBox {
        let p = sanitize_padding(style);
        let b = sanitize_border(style);
        let pb = if is_main_horizontal(direction) {
            horizontal_pb(&p, &b)
        } else {
            vertical_pb(&p, &b)
        };
        return Some((px - pb).max(0.0));
    }
    Some(px)
}

// ---------------------------------------------------------------------------
// Flex item
// ---------------------------------------------------------------------------

/// A flex item with resolved metrics.
pub(crate) struct FlexItem {
    pub(crate) entity: Entity,
    pub(crate) source_order: usize,
    pub(crate) order: i32,
    /// Flex base size before min/max clamping (CSS Flexbox §9.2 step 3).
    pub(crate) flex_base_size: f32,
    /// Hypothetical main size: flex base size clamped by min/max (§9.5 step 5).
    pub(crate) hypo_main: f32,
    pub(crate) grow: f32,
    pub(crate) shrink: f32,
    /// Total margin on the main axis (start + end).
    pub(crate) margin_main: f32,
    /// Total margin on the cross axis (start + end).
    pub(crate) margin_cross: f32,
    pub(crate) pb_main: f32,
    pub(crate) pb_cross: f32,
    pub(crate) final_main: f32,
    pub(crate) final_cross: f32,
    pub(crate) align: AlignItems,
    /// Whether the item's cross-size dimension is `auto` (stretch only applies when true).
    pub(crate) cross_size_auto: bool,
    /// Minimum content size on the main axis (from min-width/min-height).
    pub(crate) min_main: f32,
    /// Maximum content size on the main axis (from max-width/max-height).
    pub(crate) max_main: f32,
}

// ---------------------------------------------------------------------------
// Container context — shared state for the layout pass
// ---------------------------------------------------------------------------

pub(crate) struct FlexContext {
    pub(crate) content_x: f32,
    pub(crate) content_y: f32,
    pub(crate) content_width: f32,
    pub(crate) horizontal: bool,
    pub(crate) container_main: f32,
    pub(crate) direction: FlexDirection,
    pub(crate) wrap: FlexWrap,
    pub(crate) justify: JustifyContent,
    pub(crate) align_items: AlignItems,
    pub(crate) align_content: AlignContent,
    pub(crate) containing_width: f32,
    pub(crate) containing_height: Option<f32>,
    /// The container's own definite height (for children's percentage height resolution).
    pub(crate) container_definite_height: Option<f32>,
    /// Gap between items on the main axis.
    pub(crate) gap_main: f32,
    /// Gap between lines on the cross axis.
    pub(crate) gap_cross: f32,
    /// CSS `direction` property (LTR/RTL) — affects main-axis order for row layouts.
    pub(crate) css_direction: Direction,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Resolve the container main-axis size.
fn resolve_container_main(
    style: &ComputedStyle,
    horizontal: bool,
    content_width: f32,
    containing_height: Option<f32>,
) -> f32 {
    if horizontal {
        content_width
    } else {
        match style.height {
            Dimension::Length(px) => sanitize(px),
            Dimension::Percentage(pct) => {
                containing_height.map_or(INDEFINITE_MAIN_SIZE, |ch| sanitize(ch * pct / 100.0))
            }
            Dimension::Auto => INDEFINITE_MAIN_SIZE,
        }
    }
}

/// Layout a flex container and return its `LayoutBox`.
#[allow(clippy::too_many_lines)]
// Sequential algorithm phases sharing extensive local state; splitting would add indirection without improving clarity.
pub fn layout_flex(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutBox {
    let containing_width = input.containing_width;
    let containing_height = input.containing_height;
    let offset_x = input.offset_x;
    let offset_y = input.offset_y;
    let font_db = input.font_db;
    let depth = input.depth;
    let style = elidex_layout_block::get_style(dom, entity);

    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);
    let margin_left = resolve_margin(style.margin_left, containing_width);
    let margin_right = resolve_margin(style.margin_right, containing_width);
    let h_pb = horizontal_pb(&padding, &border);
    let content_width = elidex_layout_block::resolve_content_width(
        &style,
        containing_width,
        h_pb,
        margin_left + margin_right,
    );
    let content_x = offset_x + margin_left + border.left + padding.left;
    let content_y = offset_y + margin_top + border.top + padding.top;
    let margin = EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left);
    let horizontal = is_main_horizontal(style.flex_direction);
    let container_main =
        resolve_container_main(&style, horizontal, content_width, containing_height);

    // --- Early return for empty containers ---
    let children = elidex_layout_block::composed_children_flat(dom, entity);
    if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        return elidex_layout_block::empty_container_box(
            dom,
            entity,
            &EmptyContainerParams {
                style: &style,
                content_x,
                content_y,
                content_width,
                containing_height,
                padding,
                border,
                margin,
            },
        );
    }

    // Resolve gap: row-gap applies between rows, column-gap between columns.
    // For flex-direction: row, main axis is horizontal → gap_main = column_gap, gap_cross = row_gap.
    // For flex-direction: column, main axis is vertical → gap_main = row_gap, gap_cross = column_gap.
    let (gap_main, gap_cross) = if horizontal {
        (
            sanitize(style.column_gap).max(0.0),
            sanitize(style.row_gap).max(0.0),
        )
    } else {
        (
            sanitize(style.row_gap).max(0.0),
            sanitize(style.column_gap).max(0.0),
        )
    };

    // Container's own definite height for children's percentage height resolution.
    let container_definite_height = resolve_explicit_height(&style, containing_height);

    let ctx = FlexContext {
        content_x,
        content_y,
        content_width,
        horizontal,
        container_main,
        direction: style.flex_direction,
        wrap: style.flex_wrap,
        justify: style.justify_content,
        align_items: style.align_items,
        align_content: style.align_content,
        containing_width,
        containing_height,
        container_definite_height,
        gap_main,
        gap_cross,
        css_direction: style.direction,
    };

    let env = algo::LayoutEnv {
        font_db,
        layout_child,
        depth,
    };

    // --- Collect, sort, flex-resolve, layout, position ---
    let mut items = collect_flex_items(dom, &children, &ctx, &env);
    items.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then(a.source_order.cmp(&b.source_order))
    });

    let line_ranges = algo::build_line_ranges(&items, ctx.container_main, ctx.wrap, ctx.gap_main);
    for &(start, end) in &line_ranges {
        algo::resolve_flexible_lengths(&mut items[start..end], ctx.container_main, ctx.gap_main);
    }

    algo::layout_items_cross(dom, &mut items, &ctx, &env);
    let (line_cross_sizes, total_line_cross) = algo::compute_line_cross_sizes(&items, &line_ranges);

    let container_cross =
        algo::resolve_container_cross(&style, &ctx, containing_width, total_line_cross);
    let align_result = algo::compute_align_content_offsets(
        &line_cross_sizes,
        container_cross,
        ctx.align_content,
        ctx.wrap,
        ctx.gap_cross,
    );

    // Stretch items using effective line sizes (includes align-content stretch extra).
    algo::stretch_items(&mut items, &line_ranges, &align_result.effective_line_sizes);

    let lines = algo::LineGeometry {
        line_ranges: &line_ranges,
        line_cross_sizes: &align_result.effective_line_sizes,
        line_offsets: &align_result.offsets,
        container_cross,
    };
    algo::position_items(dom, &items, &lines, &ctx, &env);

    // --- Container LayoutBox ---
    let content_height =
        algo::compute_container_height(&style, &ctx, &items, &line_ranges, total_line_cross);
    let lb = LayoutBox {
        content: Rect {
            x: content_x,
            y: content_y,
            width: content_width,
            height: content_height,
        },
        padding,
        border,
        margin,
    };
    let _ = dom.world_mut().insert_one(entity, lb.clone());
    lb
}

// ---------------------------------------------------------------------------
// Helpers — box model
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

fn collect_flex_items(
    dom: &mut EcsDom,
    children: &[Entity],
    ctx: &FlexContext,
    env: &algo::LayoutEnv<'_>,
) -> Vec<FlexItem> {
    let mut items = Vec::new();
    for (source_order, &child) in children.iter().enumerate() {
        let Some(child_style) = elidex_layout_block::try_get_style(dom, child) else {
            continue;
        };
        if child_style.display == Display::None {
            continue;
        }

        let (pb_main, pb_cross) = compute_pb(&child_style, ctx.horizontal);
        let (margin_main, margin_cross) =
            compute_margins(&child_style, ctx.horizontal, ctx.containing_width);

        // CSS spec: stretch only applies when the cross-size property is auto.
        let cross_size_auto = if ctx.horizontal {
            matches!(child_style.height, Dimension::Auto)
        } else {
            matches!(child_style.width, Dimension::Auto)
        };

        let basis = resolve_flex_basis(&child_style, ctx.direction, ctx.container_main);
        let hypo_main = if let Some(px) = basis {
            sanitize(px).max(0.0)
        } else {
            let child_input = LayoutInput {
                containing_width: ctx.content_width,
                containing_height: None,
                offset_x: 0.0,
                offset_y: 0.0,
                font_db: env.font_db,
                depth: env.depth + 1,
            };
            let child_lb = (env.layout_child)(dom, child, &child_input);
            if ctx.horizontal {
                child_lb.content.width
            } else {
                child_lb.content.height
            }
        };

        // Resolve min/max constraints on the main axis.
        // For box-sizing: border-box, subtract padding+border from min/max
        // so they compare correctly with content-level hypo_main.
        let containing_main = ctx.container_main;
        let (mut min_main, mut max_main) = if ctx.horizontal {
            (
                resolve_min_max(child_style.min_width, containing_main, 0.0),
                resolve_min_max(child_style.max_width, containing_main, f32::INFINITY),
            )
        } else {
            // Column direction: items' containing block is the flex container itself.
            let ch = ctx.container_definite_height.unwrap_or(0.0);
            (
                resolve_min_max(child_style.min_height, ch, 0.0),
                resolve_min_max(child_style.max_height, ch, f32::INFINITY),
            )
        };
        if child_style.box_sizing == BoxSizing::BorderBox {
            adjust_min_max_for_border_box(&mut min_main, &mut max_main, pb_main);
        }
        // Flex base size is pre-clamp (CSS Flexbox §9.2 step 3).
        let flex_base_size = hypo_main;
        // Clamp hypothetical main size by min/max (CSS §9.5 step 5).
        let hypo_main = clamp_min_max(hypo_main, min_main, max_main);

        items.push(FlexItem {
            entity: child,
            source_order,
            order: child_style.order,
            flex_base_size,
            hypo_main,
            grow: child_style.flex_grow,
            shrink: child_style.flex_shrink,
            margin_main,
            margin_cross,
            pb_main,
            pb_cross,
            final_main: hypo_main,
            final_cross: 0.0,
            align: effective_align(child_style.align_self, ctx.align_items),
            cross_size_auto,
            min_main,
            max_main,
        });
    }
    items
}

fn compute_pb(style: &ComputedStyle, horizontal: bool) -> (f32, f32) {
    let p = sanitize_padding(style);
    let b = sanitize_border(style);
    let h = horizontal_pb(&p, &b);
    let v = vertical_pb(&p, &b);
    if horizontal {
        (h, v)
    } else {
        (v, h)
    }
}

/// Returns `(margin_main, margin_cross)`.
fn compute_margins(style: &ComputedStyle, horizontal: bool, containing_width: f32) -> (f32, f32) {
    let ml = resolve_margin(style.margin_left, containing_width);
    let mr = resolve_margin(style.margin_right, containing_width);
    let mt = resolve_margin(style.margin_top, containing_width);
    let mb = resolve_margin(style.margin_bottom, containing_width);
    if horizontal {
        (ml + mr, mt + mb)
    } else {
        (mt + mb, ml + mr)
    }
}
