//! CSS Flexbox layout algorithm (CSS Flexbox Level 1, simplified).
//!
//! Implements the core flexbox algorithm: item collection, line splitting,
//! flexible length resolution, and cross/main axis alignment.
//!
//! Phase 2 simplifications:
//! - `flex-basis: content` treated as `auto`
//! - `baseline` alignment treated as `flex-start`
//! - `inline-flex` treated as block-level

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, ComputedStyle, Dimension, Display, EdgeSizes,
    FlexDirection, FlexWrap, JustifyContent, LayoutBox, Rect,
};
use elidex_text::FontDatabase;

use crate::block::{layout_block, resolve_margin};
use crate::sanitize;
use crate::{sanitize_edge_values, MAX_LAYOUT_DEPTH};

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
pub(super) fn is_reversed(dir: FlexDirection) -> bool {
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
        let pb = if is_main_horizontal(direction) {
            sanitize(style.padding_left)
                + sanitize(style.padding_right)
                + sanitize(style.border_left_width)
                + sanitize(style.border_right_width)
        } else {
            sanitize(style.padding_top)
                + sanitize(style.padding_bottom)
                + sanitize(style.border_top_width)
                + sanitize(style.border_bottom_width)
        };
        return Some((px - pb).max(0.0));
    }
    Some(px)
}

/// Resolve the effective alignment for a flex item.
///
/// `AlignSelf::Auto` inherits from the container's `align-items`.
/// `Baseline` is treated as `FlexStart` in Phase 2.
fn effective_align(item_align: AlignSelf, container_align: AlignItems) -> AlignItems {
    let resolved = match item_align {
        AlignSelf::Auto => container_align,
        AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::FlexStart => AlignItems::FlexStart,
        AlignSelf::FlexEnd => AlignItems::FlexEnd,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Baseline => AlignItems::Baseline,
    };
    // Phase 2: baseline → flex-start.
    if resolved == AlignItems::Baseline {
        AlignItems::FlexStart
    } else {
        resolved
    }
}

// ---------------------------------------------------------------------------
// Flex item
// ---------------------------------------------------------------------------

/// A flex item with resolved metrics.
pub(super) struct FlexItem {
    pub(super) entity: Entity,
    pub(super) source_order: usize,
    pub(super) order: i32,
    pub(super) hypo_main: f32,
    pub(super) grow: f32,
    pub(super) shrink: f32,
    /// Total margin on the main axis (start + end).
    pub(super) margin_main: f32,
    /// Total margin on the cross axis (start + end).
    pub(super) margin_cross: f32,
    pub(super) pb_main: f32,
    pub(super) pb_cross: f32,
    pub(super) final_main: f32,
    pub(super) final_cross: f32,
    pub(super) align: AlignItems,
    /// Whether the item's cross-size dimension is `auto` (stretch only applies when true).
    pub(super) cross_size_auto: bool,
}

// ---------------------------------------------------------------------------
// Container context — shared state for the layout pass
// ---------------------------------------------------------------------------

pub(super) struct FlexContext {
    pub(super) content_x: f32,
    pub(super) content_y: f32,
    pub(super) content_width: f32,
    pub(super) horizontal: bool,
    pub(super) container_main: f32,
    pub(super) direction: FlexDirection,
    pub(super) wrap: FlexWrap,
    pub(super) justify: JustifyContent,
    pub(super) align_items: AlignItems,
    pub(super) align_content: AlignContent,
    pub(super) containing_width: f32,
    pub(super) containing_height: Option<f32>,
    /// The container's own definite height (for children's percentage height resolution).
    pub(super) container_definite_height: Option<f32>,
    /// Gap between items on the main axis.
    pub(super) gap_main: f32,
    /// Gap between lines on the cross axis.
    pub(super) gap_cross: f32,
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
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn layout_flex(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
) -> LayoutBox {
    let style = crate::get_style(dom, entity);

    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);
    let margin_left = resolve_margin(style.margin_left, containing_width);
    let margin_right = resolve_margin(style.margin_right, containing_width);
    let content_width = resolve_content_width(
        &style,
        containing_width,
        &padding,
        &border,
        margin_left,
        margin_right,
    );
    let content_x = offset_x + margin_left + border.left + padding.left;
    let content_y = offset_y + margin_top + border.top + padding.top;
    let margin = EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left);
    let horizontal = is_main_horizontal(style.flex_direction);
    let container_main =
        resolve_container_main(&style, horizontal, content_width, containing_height);

    // --- Early return for empty containers ---
    let children = dom.children(entity);
    if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        let lb = LayoutBox {
            content: Rect {
                x: content_x,
                y: content_y,
                width: content_width,
                height: resolve_explicit_height(&style, containing_height).unwrap_or(0.0),
            },
            padding,
            border,
            margin,
        };
        let _ = dom.world_mut().insert_one(entity, lb.clone());
        return lb;
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
    };

    // --- Collect, sort, flex-resolve, layout, position ---
    let mut items = collect_flex_items(dom, &children, &ctx, font_db);
    items.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then(a.source_order.cmp(&b.source_order))
    });

    let line_ranges = algo::build_line_ranges(&items, ctx.container_main, ctx.wrap, ctx.gap_main);
    for &(start, end) in &line_ranges {
        algo::resolve_flexible_lengths(&mut items[start..end], ctx.container_main, ctx.gap_main);
    }

    algo::layout_items_cross(dom, &mut items, &ctx, font_db);
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

    algo::position_items(
        dom,
        &items,
        &line_ranges,
        &align_result.effective_line_sizes,
        &align_result.offsets,
        &ctx,
        container_cross,
        font_db,
    );

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

fn sanitize_padding(style: &ComputedStyle) -> EdgeSizes {
    sanitize_edge_values(
        style.padding_top,
        style.padding_right,
        style.padding_bottom,
        style.padding_left,
    )
}

fn sanitize_border(style: &ComputedStyle) -> EdgeSizes {
    sanitize_edge_values(
        style.border_top_width,
        style.border_right_width,
        style.border_bottom_width,
        style.border_left_width,
    )
}

fn resolve_content_width(
    style: &ComputedStyle,
    containing_width: f32,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    margin_left: f32,
    margin_right: f32,
) -> f32 {
    let used =
        margin_left + margin_right + padding.left + padding.right + border.left + border.right;
    let auto_value = (containing_width - used).max(0.0);
    let mut w = sanitize(crate::resolve_dimension_value(
        style.width,
        containing_width,
        auto_value,
    ));
    if style.box_sizing == BoxSizing::BorderBox {
        if let Dimension::Length(_) | Dimension::Percentage(_) = style.width {
            let pb = padding.left + padding.right + border.left + border.right;
            w = (w - pb).max(0.0);
        }
    }
    w
}

pub(crate) fn resolve_explicit_height(
    style: &ComputedStyle,
    containing_height: Option<f32>,
) -> Option<f32> {
    match style.height {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                let pb = sanitize(style.padding_top)
                    + sanitize(style.padding_bottom)
                    + sanitize(style.border_top_width)
                    + sanitize(style.border_bottom_width);
                Some((px - pb).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_height.map(|ch| {
            let resolved = ch * pct / 100.0;
            if style.box_sizing == BoxSizing::BorderBox {
                let pb = sanitize(style.padding_top)
                    + sanitize(style.padding_bottom)
                    + sanitize(style.border_top_width)
                    + sanitize(style.border_bottom_width);
                (resolved - pb).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

fn collect_flex_items(
    dom: &mut EcsDom,
    children: &[Entity],
    ctx: &FlexContext,
    font_db: &FontDatabase,
) -> Vec<FlexItem> {
    let mut items = Vec::new();
    for (source_order, &child) in children.iter().enumerate() {
        let child_style = match dom.world().get::<&ComputedStyle>(child) {
            Ok(s) => (*s).clone(),
            Err(_) => continue,
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
            let child_lb = layout_block(dom, child, ctx.content_width, 0.0, 0.0, font_db);
            if ctx.horizontal {
                child_lb.content.width
            } else {
                child_lb.content.height
            }
        };

        items.push(FlexItem {
            entity: child,
            source_order,
            order: child_style.order,
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
        });
    }
    items
}

fn compute_pb(style: &ComputedStyle, horizontal: bool) -> (f32, f32) {
    if horizontal {
        (
            sanitize(style.padding_left)
                + sanitize(style.padding_right)
                + sanitize(style.border_left_width)
                + sanitize(style.border_right_width),
            sanitize(style.padding_top)
                + sanitize(style.padding_bottom)
                + sanitize(style.border_top_width)
                + sanitize(style.border_bottom_width),
        )
    } else {
        (
            sanitize(style.padding_top)
                + sanitize(style.padding_bottom)
                + sanitize(style.border_top_width)
                + sanitize(style.border_bottom_width),
            sanitize(style.padding_left)
                + sanitize(style.padding_right)
                + sanitize(style.border_left_width)
                + sanitize(style.border_right_width),
        )
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
