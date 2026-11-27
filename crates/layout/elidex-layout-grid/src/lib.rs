//! CSS Grid layout algorithm (CSS Grid Level 1 + Level 2 subgrid).
//!
//! Implements the core grid algorithm: track sizing, item placement,
//! and cell positioning. Supports named grid lines, `grid-template-areas`,
//! grid shorthand properties, and subgrid (CSS Grid Level 2 §2).
//!
// Current simplifications:
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
    block::fragmentation::{
        find_best_break, is_avoid_break_value, is_forced_break, BreakCandidate, BreakClass,
    },
    resolve_explicit_height, BreakToken, BreakTokenData, ChildLayoutFn, EmptyContainerParams,
    LayoutInput, SubgridContext, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    AlignItems, EdgeSizes, GridAutoFlow, GridLine, GridTrackList, JustifyItems, LayoutBox, Point,
    Rect, Size,
};

use elidex_layout_block::horizontal_pb;

use helpers::{
    build_contributions, build_track_definitions, collect_grid_items, distribute_tracks,
    measure_item_content, percentage_tracks_to_auto, resolve_grid_abspos_cb,
};

// ---------------------------------------------------------------------------
// GridItem
// ---------------------------------------------------------------------------

/// A grid item with resolved placement and sizing metrics.
#[allow(clippy::struct_excessive_bools)]
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
    inline_safety: Option<elidex_plugin::AlignmentSafety>,
    block_safety: Option<elidex_plugin::AlignmentSafety>,
    /// Justify-self for this item.
    justify: JustifyItems,
    /// Max-content size from initial layout (at container width).
    content_size: Size,
    /// Min-content size from narrow-probe layout.
    min_content_size: Size,
    /// Whether this item is a subgrid on the column axis (CSS Grid Level 2 §2).
    is_subgrid_cols: bool,
    /// Whether this item is a subgrid on the row axis (CSS Grid Level 2 §2).
    is_subgrid_rows: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

use helpers::{add_subgrid_axis_mbp, inherited_parent_tracks};

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
) -> elidex_layout_block::LayoutOutcome {
    let containing_width = input.containing.width;
    let containing_height = input.containing.height;
    let offset_x = input.offset.x;
    let offset_y = input.offset.y;
    let depth = input.depth;
    let style = elidex_layout_block::get_style(dom, entity);
    let input = &input.resolve_item(&style);

    // --- 1. Container box model resolution ---
    // Percentage margins/padding resolve against the containing block's inline size
    // (CSS Box Model §4/§5), not the physical width.
    let containing_inline = input.containing_inline_size;
    let (mut padding, mut border, margin) =
        elidex_layout_block::resolve_box_model(&style, containing_inline);
    let mut margin = input.assigned.margins(margin);
    let h_pb = horizontal_pb(&padding, &border);
    let content_width = input.assigned.target().width.unwrap_or_else(|| {
        elidex_layout_block::resolve_content_width(
            &style,
            containing_width,
            h_pb,
            margin.left + margin.right,
        )
    });
    input.truncate_cloned_edges(&style, &mut padding, &mut border, &mut margin);
    let margin_top = margin.top;
    let margin_bottom = margin.bottom;
    let margin_left = margin.left;

    let content_x = offset_x + margin_left + border.left + padding.left;
    let content_y = offset_y + margin_top + border.top + padding.top;
    let content_origin = Point::new(content_x, content_y);

    // CSS Grid §7.1: column tracks = inline axis, row tracks = block axis.
    // In vertical writing modes, the inline axis is physical Y (height) and
    // block axis is physical X (width).
    let is_horizontal_wm = style.writing_mode.is_horizontal();
    let (container_inline_size, block_available) = if is_horizontal_wm {
        // Horizontal: inline = width, block = height
        let avail_h = input
            .assigned
            .target()
            .height
            .or_else(|| resolve_explicit_height(&style, containing_height));
        (content_width, avail_h)
    } else {
        // Vertical: inline = height (physical Y), block = width (physical X)
        let v_pb = elidex_layout_block::vertical_pb(&padding, &border);
        let v_margin = margin_top + margin_bottom;
        let inline_h = {
            let available = input.available_height().unwrap_or(containing_inline);
            let auto_val = (available - v_pb - v_margin).max(0.0);
            let explicit = match style.height {
                elidex_plugin::Dimension::Length(px) => Some(px),
                elidex_plugin::Dimension::Percentage(pct) => {
                    containing_height.map(|height| height * pct / 100.)
                }
                elidex_plugin::Dimension::Auto => None,
            };
            explicit.map_or(auto_val, |height| {
                let height = elidex_layout_block::sanitize(height);
                if style.box_sizing == elidex_plugin::BoxSizing::BorderBox {
                    (height - v_pb).max(0.)
                } else {
                    height
                }
            })
        };
        // Block available = content_width (physical width = block direction).
        (
            input.assigned.target().height.unwrap_or(inline_h),
            input.assigned.target().width.or_else(|| {
                (style.width != elidex_plugin::Dimension::Auto).then_some(content_width)
            }),
        )
    };

    let mut gap_col =
        elidex_layout_block::resolve_dimension_value(style.column_gap, container_inline_size, 0.0)
            .max(0.0);
    let mut gap_row = elidex_layout_block::resolve_dimension_value(
        style.row_gap,
        block_available.unwrap_or(0.0),
        0.0,
    )
    .max(0.0);

    // Explicit tracks can size a grid even when it has no children.
    let children = elidex_layout_block::composed_children_flat(dom, entity);
    if depth >= MAX_LAYOUT_DEPTH {
        return elidex_layout_block::empty_container_box(
            dom,
            entity,
            &EmptyContainerParams {
                content_origin,
                content_width,
                resolved_height: input
                    .assigned
                    .target()
                    .height
                    .or_else(|| resolve_explicit_height(&style, containing_height)),
                padding,
                border,
                margin,
                layout_generation: input.layout_generation,
            },
        )
        .into();
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

    // CSS Grid L2 §2.4: subgridded axes inherit parent gap.
    if col_track_list.is_subgrid() {
        if let Some(pg) = input.subgrid.and_then(|sg| sg.col_gap) {
            gap_col = pg;
        }
    }
    if row_track_list.is_subgrid() {
        if let Some(pg) = input.subgrid.and_then(|sg| sg.row_gap) {
            gap_row = pg;
        }
    }

    // Column tracks expand against inline available, row tracks against block available.
    let col_section = col_track_list.expand_with_names(container_inline_size, gap_col);
    let row_section = row_track_list.expand_with_names(block_available.unwrap_or(0.0), gap_row);
    let expanded_cols = &col_section.tracks;
    let expanded_rows = &row_section.tracks;
    let inherited_cols = input
        .subgrid
        .filter(|_| col_track_list.is_subgrid())
        .and_then(|parent| parent.col_sizes.as_deref())
        .map(<[_]>::len);
    let inherited_rows = input
        .subgrid
        .filter(|_| row_track_list.is_subgrid())
        .and_then(|parent| parent.row_sizes.as_deref())
        .map(<[_]>::len);
    // #11-intrinsic-contribution-producer: area-defined tracks are missing from the layout count.
    let explicit_cols = inherited_cols.unwrap_or(expanded_cols.len());
    let explicit_rows = inherited_rows.unwrap_or(expanded_rows.len());

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

    // CSS Grid 2 §9: clamp both ends of each placement to the inherited tracks.
    for item in &mut items {
        placement::clamp_inherited_axis(&mut item.col_start, &mut item.col_span, inherited_cols);
        placement::clamp_inherited_axis(&mut item.row_start, &mut item.row_span, inherited_rows);
    }

    // --- Determine actual grid dimensions (may exceed explicit) ---
    let actual_cols = items
        .iter()
        .map(|item| item.col_start.saturating_add(item.col_span))
        .max()
        .unwrap_or(explicit_cols)
        .max(explicit_cols);
    let actual_rows = items
        .iter()
        .map(|item| item.row_start.saturating_add(item.row_span))
        .max()
        .unwrap_or(explicit_rows)
        .max(explicit_rows);

    // --- Subgrid: check if parent provided track context ---
    let has_subgrid = items.iter().any(|i| i.is_subgrid_cols || i.is_subgrid_rows);
    let max_passes = if has_subgrid { 3 } else { 1 };
    let mut prev_col_sizes: Vec<f32> = Vec::new();
    let mut prev_row_sizes: Vec<f32> = Vec::new();

    // Subgrid multi-pass convergence loop (CSS Grid Level 2 §2).
    // Phases 8-11 may iterate when subgrid items contribute content sizes
    // that change parent track sizing. Max 3 passes, early exit on convergence.
    let mut col_tracks: Vec<track::ResolvedTrack> = Vec::new();
    let mut row_tracks: Vec<track::ResolvedTrack> = Vec::new();
    let mut col_positions: Vec<f32> = Vec::new();
    let mut row_positions: Vec<f32> = Vec::new();

    for pass in 0..max_passes {
        // --- 8. Measure content sizes (initial layout) ---
        // Measure at inline-available width (column = inline axis).
        measure_item_content(
            dom,
            &mut items,
            container_inline_size,
            is_horizontal_wm,
            input,
            layout_child,
        );

        // --- Build per-item intrinsic size contributions ---
        let mut col_contribs = build_contributions(&items, true, is_horizontal_wm);
        let mut row_contribs = build_contributions(&items, false, is_horizontal_wm);

        // CSS Grid L2 §2.5: subgrid m/b/p contributes to parent track sizing.
        // A subgrid's margin, border, and padding on the subgridded axis are
        // added to the first and last tracks it spans. The edges must be
        // mapped through writing mode: columns = inline axis, rows = block axis.
        for item in &items {
            if item.is_subgrid_cols {
                // Inline-axis (column) m/b/p: horizontal→left/right, vertical→top/bottom.
                let (mbp_start, mbp_end) = if is_horizontal_wm {
                    (
                        item.margin.left + item.pb.left,
                        item.margin.right + item.pb.right,
                    )
                } else {
                    (
                        item.margin.top + item.pb.top,
                        item.margin.bottom + item.pb.bottom,
                    )
                };
                add_subgrid_axis_mbp(
                    &mut col_contribs,
                    item.col_start,
                    item.col_span,
                    mbp_start,
                    mbp_end,
                );
            }
            if item.is_subgrid_rows {
                // Block-axis (row) m/b/p: horizontal→top/bottom, vertical→left/right.
                let (mbp_start, mbp_end) = if is_horizontal_wm {
                    (
                        item.margin.top + item.pb.top,
                        item.margin.bottom + item.pb.bottom,
                    )
                } else {
                    (
                        item.margin.left + item.pb.left,
                        item.margin.right + item.pb.right,
                    )
                };
                add_subgrid_axis_mbp(
                    &mut row_contribs,
                    item.row_start,
                    item.row_span,
                    mbp_start,
                    mbp_end,
                );
            }
        }

        // --- 9. Resolve column tracks ---
        // If this grid is itself a subgrid on columns, use parent track sizes.
        col_tracks =
            if let Some(parent) = inherited_parent_tracks(col_track_list, input.subgrid, true) {
                parent
            } else {
                let col_defs =
                    build_track_definitions(expanded_cols, &style.grid_auto_columns, actual_cols);
                let mut ct = track::resolve_tracks(
                    &col_defs,
                    container_inline_size,
                    gap_col,
                    &col_contribs,
                    false,
                );
                if col_track_list.is_auto_fit() {
                    collapse_empty_auto_tracks(
                        &mut ct,
                        col_track_list,
                        container_inline_size,
                        gap_col,
                        &items,
                        true,
                    );
                }
                ct
            };

        // --- 10. Resolve row tracks ---
        row_tracks =
            if let Some(parent) = inherited_parent_tracks(row_track_list, input.subgrid, false) {
                parent
            } else {
                let row_defs =
                    build_track_definitions(expanded_rows, &style.grid_auto_rows, actual_rows);
                let row_defs = if block_available.is_none() {
                    percentage_tracks_to_auto(row_defs)
                } else {
                    row_defs
                };
                let mut rt = track::resolve_tracks(
                    &row_defs,
                    block_available.unwrap_or(0.0),
                    gap_row,
                    &row_contribs,
                    false,
                );
                if row_track_list.is_auto_fit() {
                    collapse_empty_auto_tracks(
                        &mut rt,
                        row_track_list,
                        block_available.unwrap_or(0.0),
                        gap_row,
                        &items,
                        false,
                    );
                }
                rt
            };

        // --- 11. Compute track positions ---
        col_positions = track::compute_track_positions(&col_tracks, gap_col);
        row_positions = track::compute_track_positions(&row_tracks, gap_row);

        // --- 11b. Track distribution (justify-content / align-content) ---
        if !col_track_list.is_subgrid()
            || input.subgrid.and_then(|sg| sg.col_sizes.as_ref()).is_none()
        {
            distribute_tracks(
                &mut col_positions,
                &col_tracks,
                gap_col,
                container_inline_size,
                style.justify_content,
                style.justify_content_safety,
            );
        }
        if !row_track_list.is_subgrid()
            || input.subgrid.and_then(|sg| sg.row_sizes.as_ref()).is_none()
        {
            if let Some(h) = block_available {
                distribute_tracks(
                    &mut row_positions,
                    &row_tracks,
                    gap_row,
                    h,
                    style.align_content,
                    style.align_content_safety,
                );
            }
        }

        // Convergence check for subgrid multi-pass.
        if has_subgrid && pass < max_passes - 1 {
            let cur_col: Vec<f32> = col_tracks.iter().map(|t| t.size).collect();
            let cur_row: Vec<f32> = row_tracks.iter().map(|t| t.size).collect();
            if pass > 0 {
                let cols_converged = cur_col.len() == prev_col_sizes.len()
                    && cur_col.iter().zip(prev_col_sizes.iter()).all(|(a, b)| {
                        let diff = (a - b).abs();
                        diff.is_finite() && diff < LAYOUT_SIZE_EPSILON
                    });
                let rows_converged = cur_row.len() == prev_row_sizes.len()
                    && cur_row.iter().zip(prev_row_sizes.iter()).all(|(a, b)| {
                        let diff = (a - b).abs();
                        diff.is_finite() && diff < LAYOUT_SIZE_EPSILON
                    });
                if cols_converged && rows_converged {
                    break;
                }
            }
            prev_col_sizes = cur_col;
            prev_row_sizes = cur_row;
        } else {
            break;
        }
    }

    // --- Parse break token for resumption ---
    let (resume_row, resume_child_bts) = match input.break_token {
        Some(bt) => match &bt.mode_data {
            Some(BreakTokenData::Grid {
                row_index,
                child_break_tokens,
                ..
            }) => (*row_index, child_break_tokens.as_slice()),
            _ => (0, &[][..]),
        },
        None => (0, &[][..]),
    };
    let incoming: std::collections::HashMap<_, _> = resume_child_bts
        .iter()
        .map(|(id, token)| (*id, token.as_ref()))
        .collect();
    let content_room = input.fragmentainer.map(|room| {
        let wm = elidex_plugin::WritingModeContext::new(style.writing_mode, style.direction);
        let logical = |edges| elidex_plugin::LogicalEdges::from_physical(edges, wm);
        let padding = logical(padding);
        let border = logical(border);
        let margin = logical(margin);
        room.content_room(
            input.break_token.is_none(),
            style.box_decoration_break,
            padding.block_start + border.block_start + margin.block_start,
            padding.block_end + border.block_end + margin.block_end,
        )
    });
    let (row_break, propagated_break_before, propagated_break_after) =
        if let Some(frag) = content_room {
            select_grid_rows(dom, &items, &row_tracks, gap_row, frag, resume_row)
        } else {
            (None, None, None)
        };
    let end_row = row_break.map_or(row_tracks.len(), |(row, _)| row);
    if input.fragmentainer.is_some() {
        items.retain(|item| {
            (resume_row..end_row).contains(&item.row_start) || incoming.contains_key(&item.entity)
        });
    }

    // --- 12. Position items + final layout ---
    // Build SubgridContext for child subgrids that need parent track info.
    let subgrid_ctx = if has_subgrid {
        Some(SubgridContext {
            col_sizes: Some(col_tracks.iter().map(|t| t.size).collect()),
            row_sizes: Some(row_tracks.iter().map(|t| t.size).collect()),
            col_line_names: col_section.line_names.clone(),
            row_line_names: row_section.line_names.clone(),
            col_gap: Some(gap_col),
            row_gap: Some(gap_row),
        })
    } else {
        None
    };
    let placement = position::GridPlacement {
        col_tracks: &col_tracks,
        row_tracks: &row_tracks,
        col_positions: &col_positions,
        row_positions: &row_positions,
        content_origin,
        container_inline_size,
        block_extent: block_available
            .unwrap_or_else(|| track::total_track_size(&row_tracks, gap_row)),
        direction: style.direction,
        subgrid_ctx: subgrid_ctx.as_ref(),
        writing_mode: style.writing_mode,
    };
    let grid_env = elidex_layout_block::LayoutEnv::from_input(input, layout_child);
    let mut fragment = position::ItemFragmentation {
        owner: entity,
        contribution: elidex_layout_block::source_progress::SourceProgress::default(),
        room: content_room.as_ref(),
        resume_row,
        incoming: &incoming,
        outgoing: Vec::new(),
        block_extent: 0.0,
        incomplete: false,
    };
    // #11-intrinsic-contribution-producer: vertical used widths can escape min/max limits.
    let full_extent = placement.block_extent;
    let selected_rows = &row_tracks[resume_row.min(end_row)..end_row];
    let target = if is_horizontal_wm || input.assigned.target().width.is_some() {
        block_available
    } else {
        match style.width {
            elidex_plugin::Dimension::Length(_) => block_available,
            elidex_plugin::Dimension::Percentage(_) if input.containing_width_definite => {
                block_available
            }
            _ => None,
        }
    };
    let resolve_fragment = |item_extent: f32, child_continues: bool| {
        let room = content_room?;
        // An internal split admits the child pieces, not their full tracks.
        let content = if child_continues {
            item_extent
        } else if let Some((_, extent)) = row_break {
            extent
        } else {
            track::total_track_size(selected_rows, gap_row).max(item_extent)
        };
        let interrupted = row_break.is_some() || child_continues;
        if input.break_token.is_some()
            || interrupted
            || target.is_some_and(|size| size > room.available_block_size)
        {
            let size = elidex_layout_block::FragmentSize::resolve(
                target,
                input.break_token.and_then(|t| t.specified_size_fraction),
                content,
                room.available_block_size,
                interrupted,
            );
            Some((size, content))
        } else {
            None
        }
    };
    let grid_baseline = position::position_items(
        dom,
        &items,
        &placement,
        &grid_env,
        &mut fragment,
        |extent, continues| {
            resolve_fragment(extent, continues).map_or(full_extent, |(size, _)| size.extent)
        },
    );
    let mut final_block = full_extent;
    let mut result_break_token = None;
    let contribution = fragment.contribution;
    let child_continues = !fragment.outgoing.is_empty();
    if let Some((size, consumed_block_size)) =
        resolve_fragment(fragment.block_extent, child_continues)
    {
        final_block = size.extent;
        if row_break.is_some() || child_continues || size.remaining {
            result_break_token = Some(BreakToken {
                specified_size_fraction: size.fraction,
                entity,
                consumed_block_size,
                mode_data: Some(BreakTokenData::Grid {
                    coverage: contribution.after_fragment(input.break_token),
                    row_index: end_row,
                    rows_exhausted: row_break.is_none(),
                    child_break_tokens: fragment.outgoing,
                }),
            });
        }
    }
    let (final_content_width, final_content_height) = if is_horizontal_wm {
        (content_width, final_block)
    } else {
        (final_block, container_inline_size)
    };

    // --- 14. LayoutBox ---
    // Grid container baseline (CSS Grid 2 §11.6).
    let first_baseline = grid_baseline;

    let lb = LayoutBox {
        content: Rect::from_origin_size(
            content_origin,
            Size::new(final_content_width, final_content_height),
        ),
        padding,
        border,
        margin,
        first_baseline,
        layout_generation: 0,
    };
    dom.set_layout_box(entity, lb.clone());

    // --- 15. Layout positioned descendants ---
    // CSS Grid §11: Grid containers establish containing blocks for abs-pos
    // children. When placement properties specify grid lines, the grid area
    // is the CB; otherwise, the container padding-box is used.
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != elidex_plugin::Position::Static || is_root || style.has_transform;
    if is_cb {
        let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
            dom,
            &children,
            content_origin,
        );
        let pb = lb.padding_box();
        let (abs_children, fixed_children) =
            elidex_layout_block::positioned::collect_positioned_descendants(dom, entity);

        let col_axis = helpers::GridAxisInfo {
            positions: &col_positions,
            tracks: &col_tracks,
            gap: gap_col,
            name_map: &col_name_map,
            explicit_count: explicit_cols,
        };
        let row_axis = helpers::GridAxisInfo {
            positions: &row_positions,
            tracks: &row_tracks,
            gap: gap_row,
            name_map: &row_name_map,
            explicit_count: explicit_rows,
        };
        for child in abs_children {
            let cb = resolve_grid_abspos_cb(dom, child, &col_axis, &row_axis, content_origin, &pb);
            let sp = static_positions.get(&child).copied().unwrap_or(cb.origin);
            elidex_layout_block::positioned::layout_absolutely_positioned(
                dom, child, &cb, sp, &grid_env,
            );
        }

        // Fixed children use viewport CB (or transform CB).
        let has_transform = style.has_transform;
        for child in fixed_children {
            let (cb, sp_default) = if has_transform {
                (pb, pb.origin)
            } else if let Some(vp) = input.viewport {
                (Rect::new(0.0, 0.0, vp.width, vp.height), Point::ZERO)
            } else {
                continue;
            };
            let sp = static_positions.get(&child).copied().unwrap_or(sp_default);
            elidex_layout_block::positioned::layout_absolutely_positioned(
                dom, child, &cb, sp, &grid_env,
            );
        }
    }

    elidex_layout_block::LayoutOutcome {
        contribution,
        layout_box: lb,
        end: if fragment.incomplete {
            elidex_layout_block::FragmentEnd::Incomplete
        } else {
            result_break_token.into()
        },
        propagated_break_before,
        propagated_break_after,
    }
}

// ---------------------------------------------------------------------------
// Grid fragmentation (CSS Grid L1 §10)
// ---------------------------------------------------------------------------

/// CSS Grid Level 1 §10: Fragmenting Grid Layout.
///
/// When the grid container is inside a fragmentainer, the block-axis extent
/// of row tracks is checked against the available block size. Between rows,
/// break opportunities are evaluated (forced breaks, avoid constraints).
///
/// Returns the selected row boundary and consumed extent, plus the propagated breaks.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn select_grid_rows(
    dom: &EcsDom,
    items: &[GridItem],
    row_tracks: &[track::ResolvedTrack],
    gap_row: f32,
    frag: elidex_layout_block::FragmentainerContext,
    resume_row: usize,
) -> (
    Option<(usize, f32)>,
    Option<elidex_plugin::BreakValue>,
    Option<elidex_plugin::BreakValue>,
) {
    let frag_type = frag.fragmentation_type;
    let available = frag.available_block_size;
    let num_rows = row_tracks.len();

    // Propagated breaks: break-before from items in the first row,
    // break-after from items in the last row.
    let propagated_before = items
        .iter()
        .filter(|item| item.row_start == 0)
        .find_map(|item| {
            let st = elidex_layout_block::try_get_style(dom, item.entity)?;
            if is_forced_break(st.break_before, frag_type) {
                Some(st.break_before)
            } else {
                None
            }
        });

    let last_row = if num_rows > 0 { num_rows - 1 } else { 0 };
    let propagated_after = items
        .iter()
        .filter(|item| item.row_start + item.row_span > last_row)
        .find_map(|item| {
            let st = elidex_layout_block::try_get_style(dom, item.entity)?;
            if is_forced_break(st.break_after, frag_type) {
                Some(st.break_after)
            } else {
                None
            }
        });

    if num_rows == 0 {
        return (None, propagated_before, propagated_after);
    }

    // Accumulate consumed block size from row tracks.
    let mut consumed: f32 = 0.0;

    let mut candidates: Vec<BreakCandidate> = Vec::new();

    for (row_idx, track) in row_tracks.iter().enumerate().skip(resume_row) {
        let row_size = track.size;

        // Check forced break-before on items starting at this row (not first row).
        if row_idx > resume_row {
            let has_forced_before =
                items
                    .iter()
                    .filter(|item| item.row_start == row_idx)
                    .any(|item| {
                        elidex_layout_block::try_get_style(dom, item.entity)
                            .is_some_and(|st| is_forced_break(st.break_before, frag_type))
                    });
            if has_forced_before {
                return (
                    Some((row_idx, consumed)),
                    propagated_before,
                    propagated_after,
                );
            }
        }

        // Check forced break-after on items ending at the previous row.
        if row_idx > resume_row && row_idx > 0 {
            let prev_row_end = row_idx; // items ending at row_idx-1 have row_start + row_span == row_idx
            let has_forced_after = items
                .iter()
                .filter(|item| item.row_start + item.row_span == prev_row_end)
                .any(|item| {
                    elidex_layout_block::try_get_style(dom, item.entity)
                        .is_some_and(|st| is_forced_break(st.break_after, frag_type))
                });
            if has_forced_after {
                return (
                    Some((row_idx, consumed)),
                    propagated_before,
                    propagated_after,
                );
            }
        }

        // Record a break candidate between rows (Class A break opportunity).
        if row_idx > resume_row {
            let violates_avoid = check_avoid_between_rows(dom, items, row_idx, frag_type);
            candidates.push(BreakCandidate {
                child_index: row_idx,
                class: BreakClass::A,
                cursor_block: consumed,
                violates_avoid,
                orphan_widow_penalty: false,
            });
        }

        // Add this row's size to consumed.
        consumed += row_size;

        // Check if consumed exceeds available space.
        if consumed > available && !candidates.is_empty() {
            if let Some(best_idx) = find_best_break(&candidates, available) {
                let break_row = candidates[best_idx].child_index;
                let consumed_at_break = candidates[best_idx].cursor_block;
                return (
                    Some((break_row, consumed_at_break)),
                    propagated_before,
                    propagated_after,
                );
            }
        }

        // Add row gap after this row (before the next).
        if row_idx + 1 < num_rows {
            consumed += gap_row;
        }
    }

    // All rows fit — no break needed.
    (None, propagated_before, propagated_after)
}

/// Check whether breaking between rows at `row_idx` violates an avoid constraint.
fn check_avoid_between_rows(
    dom: &EcsDom,
    items: &[GridItem],
    row_idx: usize,
    frag_type: elidex_layout_block::FragmentationType,
) -> bool {
    // break-after on items ending at row_idx - 1.
    if row_idx > 0 {
        for item in items.iter().filter(|i| i.row_start + i.row_span == row_idx) {
            if let Some(st) = elidex_layout_block::try_get_style(dom, item.entity) {
                if is_avoid_break_value(st.break_after, frag_type) {
                    return true;
                }
            }
        }
    }

    // break-before on items starting at row_idx.
    for item in items.iter().filter(|i| i.row_start == row_idx) {
        if let Some(st) = elidex_layout_block::try_get_style(dom, item.entity) {
            if is_avoid_break_value(st.break_before, frag_type) {
                return true;
            }
        }
    }

    false
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
