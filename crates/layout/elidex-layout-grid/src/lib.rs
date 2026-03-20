//! CSS Grid layout algorithm (CSS Grid Level 1).
//!
//! Implements the core grid algorithm: track sizing, item placement,
//! and cell positioning. Supports named grid lines, `grid-template-areas`,
//! and grid shorthand properties.
//!
// Current simplifications:
// - No subgrid
// - `inline-grid` treated as block-level

mod helpers;
mod occupancy;
mod placement;
pub(crate) mod position;
mod track;

/// Threshold for correcting layout sizes after final child layout.
///
/// If the difference between the grid-resolved height and the layout-computed
/// height exceeds this value, the `LayoutBox` is overwritten with the grid size.
const LAYOUT_SIZE_EPSILON: f32 = 0.5;

#[cfg(test)]
mod tests;

pub use helpers::compute_grid_intrinsic;

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    resolve_explicit_height, sanitize_border, ChildLayoutFn, EmptyContainerParams, LayoutInput,
    MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    AlignItems, EdgeSizes, GridAutoFlow, GridLine, GridTrackList, JustifyItems, LayoutBox, Rect,
};

use elidex_layout_block::block::resolve_margin;
use elidex_layout_block::horizontal_pb;

use helpers::{
    build_contributions, build_track_definitions, collect_grid_items, distribute_tracks,
    measure_item_content, percentage_tracks_to_auto, resolve_grid_abspos_cb,
};

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
    /// Justify-self for this item.
    justify: JustifyItems,
    /// Whether the item's height is `auto` (for stretch).
    height_auto: bool,
    /// Whether the item's width is `auto` (for stretch).
    width_auto: bool,
    /// Max-content size from initial layout (at container width).
    content_width: f32,
    content_height: f32,
    /// Min-content size from narrow-probe layout.
    min_content_width: f32,
    min_content_height: f32,
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
    let padding = elidex_layout_block::resolve_padding(&style, containing_width);
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

    let gap_col =
        elidex_layout_block::resolve_dimension_value(style.column_gap, content_width, 0.0).max(0.0);
    let gap_row =
        elidex_layout_block::resolve_dimension_value(style.row_gap, content_width, 0.0).max(0.0);

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

    // --- 4. Expand auto-repeat and determine explicit grid size ---
    let col_track_list = &style.grid_template_columns;
    let row_track_list = &style.grid_template_rows;
    let col_section = col_track_list.expand_with_names(content_width, gap_col);
    let available_height_for_rows = resolve_explicit_height(&style, containing_height);
    let row_section =
        row_track_list.expand_with_names(available_height_for_rows.unwrap_or(0.0), gap_row);
    let expanded_cols = &col_section.tracks;
    let expanded_rows = &row_section.tracks;
    let explicit_cols = expanded_cols.len();
    let explicit_rows = expanded_rows.len();

    // --- 4b. Build line name maps ---
    let col_name_map =
        placement::build_line_name_map(&col_section.line_names, &style.grid_template_areas, true);
    let row_name_map =
        placement::build_line_name_map(&row_section.line_names, &style.grid_template_areas, false);

    // --- 5-7. Placement ---
    let column_flow = matches!(
        style.grid_auto_flow,
        GridAutoFlow::Column | GridAutoFlow::ColumnDense
    );
    let dense = matches!(
        style.grid_auto_flow,
        GridAutoFlow::RowDense | GridAutoFlow::ColumnDense
    );
    placement::place_items(
        &mut items,
        explicit_cols,
        explicit_rows,
        column_flow,
        dense,
        &col_name_map,
        &row_name_map,
    );

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

    // --- Build per-item intrinsic size contributions ---
    let col_contribs = build_contributions(&items, true);
    let row_contribs = build_contributions(&items, false);

    // --- 9. Resolve column tracks ---
    let col_defs = build_track_definitions(expanded_cols, &style.grid_auto_columns, actual_cols);
    let mut col_tracks = track::resolve_tracks(
        &col_defs,
        content_width,
        gap_col,
        &col_contribs,
        false, // stretch handled by distribute_tracks
    );

    // auto-fit: collapse empty auto-repeated column tracks to 0.
    if col_track_list.is_auto_fit() {
        collapse_empty_auto_tracks(
            &mut col_tracks,
            col_track_list,
            content_width,
            gap_col,
            &items,
            true,
        );
    }

    // --- 10. Resolve row tracks ---
    let available_height = available_height_for_rows;
    let row_defs = build_track_definitions(expanded_rows, &style.grid_auto_rows, actual_rows);
    // When the container height is indefinite, percentage row tracks should
    // behave like auto (CSS Grid §7.2.1).
    let row_defs = if available_height.is_none() {
        percentage_tracks_to_auto(row_defs)
    } else {
        row_defs
    };
    let mut row_tracks = track::resolve_tracks(
        &row_defs,
        available_height.unwrap_or(0.0),
        gap_row,
        &row_contribs,
        false, // stretch handled by distribute_tracks
    );

    // auto-fit: collapse empty auto-repeated row tracks to 0.
    if row_track_list.is_auto_fit() {
        collapse_empty_auto_tracks(
            &mut row_tracks,
            row_track_list,
            available_height.unwrap_or(0.0),
            gap_row,
            &items,
            false,
        );
    }

    // --- 11. Compute track positions ---
    let mut col_positions = track::compute_track_positions(&col_tracks, gap_col);
    let mut row_positions = track::compute_track_positions(&row_tracks, gap_row);

    // --- 11b. Track distribution (justify-content / align-content) ---
    distribute_tracks(
        &mut col_positions,
        &col_tracks,
        gap_col,
        content_width,
        style.justify_content,
        style.justify_content_safety,
    );
    if let Some(h) = available_height {
        distribute_tracks(
            &mut row_positions,
            &row_tracks,
            gap_row,
            h,
            style.align_content,
            style.align_content_safety,
        );
    }

    // --- 12. Position items + final layout ---
    let placement = position::GridPlacement {
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
    let grid_baseline =
        position::position_items(dom, &items, &placement, font_db, depth, layout_child);

    // --- 13. Container height ---
    let total_row_size = track::total_track_size(&row_tracks, gap_row);
    let content_height = available_height.unwrap_or(total_row_size);

    // --- 14. LayoutBox ---
    // Grid container baseline (CSS Grid §4.2).
    let first_baseline = grid_baseline;

    let lb = LayoutBox {
        content: Rect::new(content_x, content_y, content_width, content_height),
        padding,
        border,
        margin,
        first_baseline,
    };
    let _ = dom.world_mut().insert_one(entity, lb.clone());

    // --- 15. Layout positioned descendants ---
    // CSS Grid §11: Grid containers establish containing blocks for abs-pos
    // children. When placement properties specify grid lines, the grid area
    // is the CB; otherwise, the container padding-box is used.
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != elidex_plugin::Position::Static || is_root || style.has_transform;
    if is_cb {
        let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
            dom, &children, content_x, content_y,
        );
        let pb = lb.padding_box();
        let (abs_children, fixed_children) =
            elidex_layout_block::positioned::collect_positioned_descendants(dom, entity);

        for child in abs_children {
            let cb = resolve_grid_abspos_cb(
                dom,
                child,
                &col_positions,
                &row_positions,
                &col_tracks,
                &row_tracks,
                gap_col,
                gap_row,
                content_x,
                content_y,
                &pb,
                &col_name_map,
                &row_name_map,
                explicit_cols,
                explicit_rows,
            );
            let sp = static_positions
                .get(&child)
                .copied()
                .unwrap_or((cb.x, cb.y));
            elidex_layout_block::positioned::layout_absolutely_positioned(
                dom,
                child,
                &cb,
                sp,
                font_db,
                layout_child,
                depth,
                input.viewport,
            );
        }

        // Fixed children use viewport CB (or transform CB).
        let has_transform = style.has_transform;
        for child in fixed_children {
            let (cb, sp_default) = if has_transform {
                (pb, (pb.x, pb.y))
            } else if let Some((vw, vh)) = input.viewport {
                (Rect::new(0.0, 0.0, vw, vh), (0.0, 0.0))
            } else {
                continue;
            };
            let sp = static_positions.get(&child).copied().unwrap_or(sp_default);
            elidex_layout_block::positioned::layout_absolutely_positioned(
                dom,
                child,
                &cb,
                sp,
                font_db,
                layout_child,
                depth,
                input.viewport,
            );
        }
    }

    lb
}

// ---------------------------------------------------------------------------
// Auto-fit collapse
// ---------------------------------------------------------------------------

/// Collapse empty auto-repeated tracks to zero size (CSS Grid Level 1 §7.2.3.2).
///
/// For `auto-fit`, tracks in the auto-repeat range that contain no items
/// are collapsed: their size is set to 0 and they don't contribute to the
/// grid container's intrinsic size.
fn collapse_empty_auto_tracks(
    tracks: &mut [track::ResolvedTrack],
    track_list: &GridTrackList,
    available: f32,
    gap: f32,
    items: &[GridItem],
    is_column: bool,
) {
    let Some(range) = track_list.auto_repeat_range(available, gap) else {
        return;
    };

    for idx in range {
        if idx >= tracks.len() {
            break;
        }
        // Check if any item occupies this track.
        let occupied = items.iter().any(|item| {
            let (start, span) = if is_column {
                (item.col_start, item.col_span)
            } else {
                (item.row_start, item.row_span)
            };
            idx >= start && idx < start + span
        });
        if !occupied {
            tracks[idx].size = 0.0;
            tracks[idx].base = 0.0;
            tracks[idx].limit = 0.0;
            tracks[idx].collapsed = true;
        }
    }
}
