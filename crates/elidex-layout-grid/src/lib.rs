//! CSS Grid layout algorithm (CSS Grid Level 1, simplified).
//!
//! Implements the core grid algorithm: track sizing, item placement,
//! and cell positioning.
//!
//! Current simplifications:
//! - No named grid lines (numeric only)
//! - No `grid-template-areas`
//! - No subgrid
//! - `repeat(auto-fill/auto-fit, ...)` treated as `repeat(1, ...)`
//! - `fit-content()` treated as `auto`
//! - `inline-grid` treated as block-level
//! - `baseline` alignment treated as `start`

mod placement;
mod track;

/// Threshold for correcting layout sizes after final child layout.
///
/// If the difference between the grid-resolved height and the layout-computed
/// height exceeds this value, the `LayoutBox` is overwritten with the grid size.
const LAYOUT_SIZE_EPSILON: f32 = 0.5;

#[cfg(test)]
mod tests;

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    effective_align, horizontal_pb, resolve_explicit_height, sanitize, sanitize_border,
    sanitize_padding, ChildLayoutFn, EmptyContainerParams, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    AlignItems, ComputedStyle, Dimension, Direction, Display, EdgeSizes, GridAutoFlow, GridLine,
    LayoutBox, Rect, TrackSize,
};
use elidex_text::FontDatabase;

use elidex_layout_block::block::resolve_margin;

// ---------------------------------------------------------------------------
// GridItem
// ---------------------------------------------------------------------------

/// A grid item with resolved placement and sizing metrics.
struct GridItem {
    entity: Entity,
    source_order: usize,
    order: i32,
    /// 0-based row start index in the grid.
    row_start: usize,
    /// 0-based column start index.
    col_start: usize,
    /// Number of rows this item spans.
    row_span: usize,
    /// Number of columns this item spans.
    col_span: usize,
    /// Original grid-line values (for placement resolution).
    grid_row_start: GridLine,
    grid_row_end: GridLine,
    grid_column_start: GridLine,
    grid_column_end: GridLine,
    /// Resolved margins.
    margin: EdgeSizes,
    /// Padding + border on each side.
    pb: EdgeSizes,
    /// Align-self for this item.
    align: AlignItems,
    /// Whether the item's height is `auto` (for stretch).
    height_auto: bool,
    /// Whether the item's width is `auto` (for stretch).
    width_auto: bool,
    /// Content size from initial layout.
    content_width: f32,
    content_height: f32,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Layout a grid container and return its `LayoutBox`.
#[allow(clippy::too_many_lines)]
// Sequential algorithm phases sharing extensive local state; splitting would add indirection without improving clarity.
pub fn layout_grid(
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

    // --- 1. Container box model resolution ---
    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);
    let margin_left = resolve_margin(style.margin_left, containing_width);
    let margin_right = resolve_margin(style.margin_right, containing_width);
    let margin = EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left);

    let h_pb = horizontal_pb(&padding, &border);
    let content_width = elidex_layout_block::resolve_content_width(
        &style,
        containing_width,
        h_pb,
        margin_left + margin_right,
    );
    let content_x = offset_x + margin_left + border.left + padding.left;
    let content_y = offset_y + margin_top + border.top + padding.top;

    let gap_col = sanitize(style.column_gap).max(0.0);
    let gap_row = sanitize(style.row_gap).max(0.0);

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

    // --- 2. Collect child items (skip display:none) ---
    let mut items = collect_grid_items(dom, &children, &style);

    // --- 3. Sort by order (stable) ---
    items.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then(a.source_order.cmp(&b.source_order))
    });

    // --- 4. Determine explicit grid size ---
    let explicit_cols = style.grid_template_columns.len();
    let explicit_rows = style.grid_template_rows.len();

    // --- 5-7. Placement ---
    let column_flow = matches!(
        style.grid_auto_flow,
        GridAutoFlow::Column | GridAutoFlow::ColumnDense
    );
    let dense = matches!(
        style.grid_auto_flow,
        GridAutoFlow::RowDense | GridAutoFlow::ColumnDense
    );
    placement::place_items(&mut items, explicit_cols, explicit_rows, column_flow, dense);

    // --- Determine actual grid dimensions (may exceed explicit) ---
    let actual_cols = items
        .iter()
        .map(|item| item.col_start + item.col_span)
        .max()
        .unwrap_or(explicit_cols)
        .max(explicit_cols);
    let actual_rows = items
        .iter()
        .map(|item| item.row_start + item.row_span)
        .max()
        .unwrap_or(explicit_rows)
        .max(explicit_rows);

    // --- 8. Measure content sizes (initial layout) ---
    measure_item_content(
        dom,
        &mut items,
        content_width,
        containing_height,
        font_db,
        depth,
        layout_child,
    );

    // --- Build per-track max content sizes for intrinsic sizing ---
    let col_content_sizes = compute_max_content_per_track(&items, actual_cols, true);
    let row_content_sizes = compute_max_content_per_track(&items, actual_rows, false);

    // --- 9. Resolve column tracks ---
    let col_defs = build_track_definitions(
        &style.grid_template_columns,
        &style.grid_auto_columns,
        actual_cols,
    );
    let col_tracks = track::resolve_tracks(&col_defs, content_width, gap_col, &col_content_sizes);

    // --- 10. Resolve row tracks ---
    let available_height = resolve_explicit_height(&style, containing_height);
    let row_defs = build_track_definitions(
        &style.grid_template_rows,
        &style.grid_auto_rows,
        actual_rows,
    );
    // When the container height is indefinite, percentage row tracks should
    // behave like auto (CSS Grid §7.2.1).
    let row_defs = if available_height.is_none() {
        percentage_tracks_to_auto(row_defs)
    } else {
        row_defs
    };
    let row_tracks = track::resolve_tracks(
        &row_defs,
        available_height.unwrap_or(0.0),
        gap_row,
        &row_content_sizes,
    );

    // --- 11. Compute track positions ---
    let col_positions = track::compute_track_positions(&col_tracks, gap_col);
    let row_positions = track::compute_track_positions(&row_tracks, gap_row);

    // --- 12. Position items + final layout ---
    let placement = GridPlacement {
        col_tracks: &col_tracks,
        row_tracks: &row_tracks,
        col_positions: &col_positions,
        row_positions: &row_positions,
        content_x,
        content_y,
        content_width,
        direction: style.direction,
        containing_height,
    };
    position_items(dom, &items, &placement, font_db, depth, layout_child);

    // --- 13. Container height ---
    let total_row_size = track::total_track_size(&row_tracks, gap_row);
    let content_height = available_height.unwrap_or(total_row_size);

    // --- 14. LayoutBox ---
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
// Item collection
// ---------------------------------------------------------------------------

/// Collect grid items from children, skipping `display:none` and text nodes.
fn collect_grid_items(
    dom: &EcsDom,
    children: &[Entity],
    container_style: &ComputedStyle,
) -> Vec<GridItem> {
    let mut items = Vec::new();
    for (i, &child) in children.iter().enumerate() {
        let Some(child_style) = elidex_layout_block::try_get_style(dom, child) else {
            continue; // Text node — skip.
        };
        if child_style.display == Display::None {
            continue;
        }

        let align = effective_align(child_style.align_self, container_style.align_items);

        items.push(GridItem {
            entity: child,
            source_order: i,
            order: child_style.order,
            row_start: 0,
            col_start: 0,
            row_span: 1,
            col_span: 1,
            grid_row_start: child_style.grid_row_start,
            grid_row_end: child_style.grid_row_end,
            grid_column_start: child_style.grid_column_start,
            grid_column_end: child_style.grid_column_end,
            margin: EdgeSizes::default(),
            pb: EdgeSizes::default(),
            align,
            height_auto: child_style.height == Dimension::Auto,
            width_auto: child_style.width == Dimension::Auto,
            content_width: 0.0,
            content_height: 0.0,
        });
    }
    items
}

// ---------------------------------------------------------------------------
// Content measurement
// ---------------------------------------------------------------------------

/// Measure each item's content size via a preliminary layout.
fn measure_item_content(
    dom: &mut EcsDom,
    items: &mut [GridItem],
    container_width: f32,
    containing_height: Option<f32>,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: ChildLayoutFn,
) {
    for item in items.iter_mut() {
        let child_style = elidex_layout_block::get_style(dom, item.entity);
        let padding = sanitize_padding(&child_style);
        let border = sanitize_border(&child_style);
        item.pb = EdgeSizes::new(
            padding.top + border.top,
            padding.right + border.right,
            padding.bottom + border.bottom,
            padding.left + border.left,
        );
        item.margin = EdgeSizes::new(
            resolve_margin(child_style.margin_top, container_width),
            resolve_margin(child_style.margin_right, container_width),
            resolve_margin(child_style.margin_bottom, container_width),
            resolve_margin(child_style.margin_left, container_width),
        );

        // Preliminary layout at container width to get intrinsic sizes.
        let child_input = LayoutInput {
            containing_width: container_width,
            containing_height,
            offset_x: 0.0,
            offset_y: 0.0,
            font_db,
            depth: depth + 1,
        };
        let lb = layout_child(dom, item.entity, &child_input);
        item.content_width = lb.content.width + item.pb.left + item.pb.right;
        item.content_height = lb.content.height + item.pb.top + item.pb.bottom;
    }
}

/// Compute the maximum content size per track for intrinsic sizing.
///
/// For items spanning a single track, contribute their full size.
/// For multi-span items, distribute proportionally.
#[allow(clippy::cast_precision_loss)]
fn compute_max_content_per_track(
    items: &[GridItem],
    track_count: usize,
    is_column: bool,
) -> Vec<f32> {
    let mut sizes = vec![0.0_f32; track_count];
    for item in items {
        let (start, span, content_size) = if is_column {
            (
                item.col_start,
                item.col_span,
                item.content_width + item.margin.left + item.margin.right,
            )
        } else {
            (
                item.row_start,
                item.row_span,
                item.content_height + item.margin.top + item.margin.bottom,
            )
        };
        if span == 1 && start < track_count {
            sizes[start] = sizes[start].max(content_size);
        } else if span > 1 {
            let per_track = content_size / span as f32;
            for size in sizes.iter_mut().skip(start).take(span) {
                *size = size.max(per_track);
            }
        }
    }
    sizes
}

// ---------------------------------------------------------------------------
// Track definitions
// ---------------------------------------------------------------------------

/// Convert percentage tracks to auto (CSS Grid §7.2.1).
///
/// When the available size on an axis is indefinite, percentage-sized tracks
/// are treated as auto so that intrinsic content sizing takes over.
fn percentage_tracks_to_auto(defs: Vec<TrackSize>) -> Vec<TrackSize> {
    defs.into_iter()
        .map(|def| match def {
            TrackSize::Percentage(_) => TrackSize::Auto,
            other => other,
        })
        .collect()
}

/// Build the full list of track definitions (explicit + implicit).
fn build_track_definitions(
    explicit: &[TrackSize],
    auto_track: &TrackSize,
    actual_count: usize,
) -> Vec<TrackSize> {
    (0..actual_count)
        .map(|i| {
            explicit
                .get(i)
                .cloned()
                .unwrap_or_else(|| auto_track.clone())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Item positioning
// ---------------------------------------------------------------------------

/// Grid track layout and positioning context.
struct GridPlacement<'a> {
    col_tracks: &'a [track::ResolvedTrack],
    row_tracks: &'a [track::ResolvedTrack],
    col_positions: &'a [f32],
    row_positions: &'a [f32],
    content_x: f32,
    content_y: f32,
    content_width: f32,
    direction: Direction,
    containing_height: Option<f32>,
}

/// Position each item within its grid area and perform final layout.
fn position_items(
    dom: &mut EcsDom,
    items: &[GridItem],
    placement: &GridPlacement<'_>,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: ChildLayoutFn,
) {
    let is_rtl = placement.direction == Direction::Rtl;
    let content_x = placement.content_x;
    let content_y = placement.content_y;
    let content_width = placement.content_width;
    let containing_height = placement.containing_height;
    let col_tracks = placement.col_tracks;
    let row_tracks = placement.row_tracks;
    let col_positions = placement.col_positions;
    let row_positions = placement.row_positions;
    for item in items {
        // Compute the grid area rectangle.
        let ltr_area_x = col_positions.get(item.col_start).copied().unwrap_or(0.0);
        let area_width = cell_span_size(col_tracks, col_positions, item.col_start, item.col_span);
        // RTL: mirror column position so columns flow right-to-left.
        let area_x = if is_rtl {
            (content_width - ltr_area_x - area_width).max(0.0)
        } else {
            ltr_area_x
        };
        let area_y = row_positions.get(item.row_start).copied().unwrap_or(0.0);
        let area_height = cell_span_size(row_tracks, row_positions, item.row_start, item.row_span);

        // Available space for the item after subtracting margins.
        let avail_w = (area_width - item.margin.left - item.margin.right).max(0.0);
        let avail_h = (area_height - item.margin.top - item.margin.bottom).max(0.0);

        // Resolve item content width.
        let child_style = elidex_layout_block::get_style(dom, item.entity);
        let item_content_w = if item.width_auto {
            (avail_w - item.pb.left - item.pb.right).max(0.0)
        } else {
            resolve_item_dimension(
                child_style.width,
                avail_w,
                item.pb.left + item.pb.right,
                child_style.box_sizing,
            )
            .max(0.0)
        };

        // Preliminary layout to measure content height.
        let prelim_input = LayoutInput {
            containing_width: avail_w,
            containing_height,
            offset_x: 0.0,
            offset_y: 0.0,
            font_db,
            depth: depth + 1,
        };
        let prelim_lb = layout_child(dom, item.entity, &prelim_input);

        // Resolve item content height: stretch fills the area, otherwise use content.
        let prelim_content_h = prelim_lb.content.height;
        let item_content_h = if should_stretch_cross(item.align, item.height_auto) {
            (avail_h - item.pb.top - item.pb.bottom).max(prelim_content_h)
        } else {
            prelim_content_h
        };

        let item_outer_h =
            item_content_h + item.pb.top + item.pb.bottom + item.margin.top + item.margin.bottom;

        // Cross-axis alignment (vertical).
        let y_offset = compute_alignment_offset(item.align, area_height, item_outer_h);

        // Override the child's width/height so layout_block_inner uses grid-resolved values.
        {
            let mut style = elidex_layout_block::get_style(dom, item.entity);
            style.width = Dimension::Length(item_content_w);
            style.height = Dimension::Length(item_content_h);
            let _ = dom.world_mut().insert_one(item.entity, style);
        }

        // Margin-box position: layout_child (layout_block_inner) adds
        // margin + border + padding offsets from here.
        let margin_box_x = content_x + area_x;
        let margin_box_y = content_y + area_y + y_offset;

        // Final layout at resolved position.
        let final_input = LayoutInput {
            containing_width: area_width,
            containing_height: Some(item_content_h),
            offset_x: margin_box_x,
            offset_y: margin_box_y,
            font_db,
            depth: depth + 1,
        };
        let final_lb = layout_child(dom, item.entity, &final_input);

        // Ensure the content height matches the grid-resolved value.
        if (item_content_h - final_lb.content.height).abs() > LAYOUT_SIZE_EPSILON {
            let corrected = LayoutBox {
                content: Rect {
                    x: final_lb.content.x,
                    y: final_lb.content.y,
                    width: item_content_w,
                    height: item_content_h,
                },
                ..final_lb
            };
            let _ = dom.world_mut().insert_one(item.entity, corrected);
        }
    }
}

/// Compute the pixel size of a span of tracks.
///
/// Gaps between spanned tracks are included naturally because track positions
/// already account for gaps.
fn cell_span_size(
    tracks: &[track::ResolvedTrack],
    positions: &[f32],
    start: usize,
    span: usize,
) -> f32 {
    if tracks.is_empty() || start >= tracks.len() {
        return 0.0;
    }
    let end = (start + span).min(tracks.len());
    let start_pos = positions.get(start).copied().unwrap_or(0.0);
    let last = end.saturating_sub(1);
    let end_pos = positions.get(last).copied().unwrap_or(start_pos)
        + tracks.get(last).map_or(0.0, |t| t.size);
    // Include gaps between spanned tracks but not the gap after the last track.
    end_pos - start_pos
}

/// Check if an item should stretch in the cross axis.
///
/// Stretch only applies when the item's resolved alignment is `Stretch`
/// AND the item's size on that axis is `auto`.
fn should_stretch_cross(item_align: AlignItems, size_auto: bool) -> bool {
    size_auto && matches!(item_align, AlignItems::Stretch)
}

/// Compute alignment offset within available space.
fn compute_alignment_offset(item_align: AlignItems, available: f32, item_size: f32) -> f32 {
    let free = (available - item_size).max(0.0);
    match item_align {
        AlignItems::Center => free / 2.0,
        AlignItems::FlexEnd => free,
        // Stretch, FlexStart, Baseline — all align to start.
        AlignItems::FlexStart | AlignItems::Stretch | AlignItems::Baseline => 0.0,
    }
}

/// Resolve an item dimension (width/height).
fn resolve_item_dimension(
    dim: Dimension,
    available: f32,
    pb: f32,
    box_sizing: elidex_plugin::BoxSizing,
) -> f32 {
    match dim {
        Dimension::Length(px) => {
            if box_sizing == elidex_plugin::BoxSizing::BorderBox {
                (px - pb).max(0.0)
            } else {
                px
            }
        }
        Dimension::Percentage(pct) => {
            let resolved = available * pct / 100.0;
            if box_sizing == elidex_plugin::BoxSizing::BorderBox {
                (resolved - pb).max(0.0)
            } else {
                resolved
            }
        }
        Dimension::Auto => (available - pb).max(0.0),
    }
}
