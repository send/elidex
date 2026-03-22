//! Grid item positioning within resolved track areas.
//!
//! Handles alignment (align-items/self, justify-items/self),
//! baseline alignment, stretch, and final child layout dispatch.

use std::collections::HashMap;

use elidex_ecs::EcsDom;
use elidex_layout_block::{LayoutEnv, LayoutInput, SubgridContext};
use elidex_plugin::{
    AlignItems, CssSize, Dimension, Direction, GridTrackList, JustifyItems, LayoutBox, Point, Rect,
    WritingMode,
};

use crate::track;
use crate::{GridItem, LAYOUT_SIZE_EPSILON};

// ---------------------------------------------------------------------------
// Subgrid helpers
// ---------------------------------------------------------------------------

/// Extract a slice of parent track sizes for a subgrid child.
fn extract_subgrid_sizes(
    parent_sizes: Option<&Vec<f32>>,
    start: usize,
    span: usize,
) -> Option<Vec<f32>> {
    parent_sizes.map(|sizes| {
        let end = start.saturating_add(span).min(sizes.len());
        sizes.get(start..end).unwrap_or_default().to_vec()
    })
}

/// Extract a slice of parent line names for a subgrid child.
///
/// Returns exactly `span + 1` entries (one per line boundary).  If the
/// parent grid has fewer names than needed, the result is padded with
/// empty vecs so that downstream merging always sees the correct count.
fn extract_subgrid_names(
    parent_names: &[Vec<String>],
    start: usize,
    span: usize,
) -> Vec<Vec<String>> {
    let expected = span.saturating_add(1);
    let start = start.min(parent_names.len());
    let end = start.saturating_add(expected).min(parent_names.len());
    let mut result = parent_names.get(start..end).unwrap_or_default().to_vec();
    // Pad with empty vecs to guarantee `span + 1` entries.
    result.resize_with(expected, Vec::new);
    result
}

/// Extract sizes and merged line names for one subgrid axis.
///
/// Returns `(None, vec![])` when the axis is not subgridded.
fn extract_axis_subgrid(
    is_subgrid: bool,
    parent_sizes: Option<&Vec<f32>>,
    parent_names: &[Vec<String>],
    start: usize,
    span: usize,
    track_list: &GridTrackList,
) -> (Option<Vec<f32>>, Vec<Vec<String>>) {
    if !is_subgrid {
        return (None, vec![]);
    }
    let sizes = extract_subgrid_sizes(parent_sizes, start, span);
    let names = extract_subgrid_names(parent_names, start, span);
    let merged = merge_line_names(&names, track_list);
    (sizes, merged)
}

/// Merge parent line names with a subgrid's own declared line names (CSS Grid L2 §2.2).
///
/// The subgrid's names augment the parent names at corresponding positions.
/// If the subgrid declares fewer names than inherited, trailing parent names are kept.
/// If the subgrid declares more names than inherited, extra names extend the list.
fn merge_line_names(parent_names: &[Vec<String>], track_list: &GridTrackList) -> Vec<Vec<String>> {
    let GridTrackList::Subgrid {
        line_names: subgrid_names,
    } = track_list
    else {
        return parent_names.to_vec();
    };
    let max_len = parent_names.len().max(subgrid_names.len());
    (0..max_len)
        .map(|i| {
            let mut merged = parent_names.get(i).cloned().unwrap_or_default();
            if let Some(sub) = subgrid_names.get(i) {
                for name in sub {
                    if !merged.contains(name) {
                        merged.push(name.clone());
                    }
                }
            }
            merged
        })
        .collect()
}

// ---------------------------------------------------------------------------
// GridPlacement
// ---------------------------------------------------------------------------

/// Grid track layout and positioning context.
pub(crate) struct GridPlacement<'a> {
    pub(crate) col_tracks: &'a [track::ResolvedTrack],
    pub(crate) row_tracks: &'a [track::ResolvedTrack],
    pub(crate) col_positions: &'a [f32],
    pub(crate) row_positions: &'a [f32],
    pub(crate) content_origin: Point,
    /// Container inline-axis size (CSS Writing Modes §6.3, used for RTL mirroring).
    pub(crate) container_inline_size: f32,
    pub(crate) direction: Direction,
    pub(crate) containing: CssSize,
    /// Subgrid context for passing parent track sizes to subgrid children.
    pub(crate) subgrid_ctx: Option<&'a SubgridContext>,
    /// Container's writing mode (CSS Writing Modes §6.3).
    pub(crate) writing_mode: WritingMode,
}

// ---------------------------------------------------------------------------
// position_items
// ---------------------------------------------------------------------------

/// Position each item within its grid area and perform final layout.
///
/// Returns the grid container's first baseline (CSS Grid §4.2).
#[allow(clippy::too_many_lines)]
pub(crate) fn position_items(
    dom: &mut EcsDom,
    items: &[GridItem],
    placement: &GridPlacement<'_>,
    env: &LayoutEnv<'_>,
) -> Option<f32> {
    let is_rtl = placement.direction == Direction::Rtl;
    let is_horizontal_wm = placement.writing_mode.is_horizontal();
    let content_x = placement.content_origin.x;
    let content_y = placement.content_origin.y;
    let container_inline_size = placement.container_inline_size;
    let containing_height = placement.containing.height;
    let col_tracks = placement.col_tracks;
    let row_tracks = placement.row_tracks;
    let col_positions = placement.col_positions;
    let row_positions = placement.row_positions;

    // Check if any item needs baseline alignment (CSS Grid §10.6).
    let has_baseline = items
        .iter()
        .any(|i| i.align == AlignItems::Baseline || i.justify == JustifyItems::Baseline);

    // Per-row/column baselines for baseline alignment (CSS Grid §10.6).
    let mut row_baselines: HashMap<usize, f32> = HashMap::new();
    let mut col_baselines: HashMap<usize, f32> = HashMap::new();

    if has_baseline {
        // Pass 1: preliminary layout for baseline-aligned items to collect baselines.
        // CSS Grid §10.6: items spanning more than one track do not participate
        // in baseline sharing groups.
        for item in items {
            let align_bl = item.align == AlignItems::Baseline && item.row_span == 1;
            let justify_bl = item.justify == JustifyItems::Baseline && item.col_span == 1;
            if !align_bl && !justify_bl {
                continue;
            }
            let col_size = cell_span_size(col_tracks, col_positions, item.col_start, item.col_span);
            let row_size = cell_span_size(row_tracks, row_positions, item.row_start, item.row_span);
            // Map logical track sizes to physical area dimensions.
            let (bl_area_w, bl_area_h) = if is_horizontal_wm {
                (col_size, row_size)
            } else {
                (row_size, col_size)
            };
            let avail_w = (bl_area_w - item.margin.left - item.margin.right).max(0.0);
            let _avail_h = (bl_area_h - item.margin.top - item.margin.bottom).max(0.0);

            let prelim_inline_size = if is_horizontal_wm {
                bl_area_w
            } else {
                bl_area_h
            };
            let prelim_input = LayoutInput {
                containing: CssSize {
                    width: avail_w,
                    height: containing_height,
                },
                containing_inline_size: prelim_inline_size,
                ..LayoutInput::probe(env, avail_w)
            };
            let prelim_lb = (env.layout_child)(dom, item.entity, &prelim_input).layout_box;

            if align_bl {
                // CSS Grid §10.6: synthesized baseline = border-box bottom from
                // margin-box top. Items without a baseline use the border-box bottom.
                if let Some(b) = prelim_lb.first_baseline {
                    let baseline = item.margin.top + item.pb.top + b;
                    let entry = row_baselines.entry(item.row_start).or_insert(0.0_f32);
                    *entry = entry.max(baseline);
                } else {
                    // Border-box bottom from margin-box top.
                    let baseline = item.margin.top
                        + item.pb.top
                        + prelim_lb.content.size.height
                        + item.pb.bottom;
                    let entry = row_baselines.entry(item.row_start).or_insert(0.0_f32);
                    *entry = entry.max(baseline);
                }
            }
            if justify_bl {
                if let Some(b) = prelim_lb.first_baseline {
                    let baseline = item.margin.left + item.pb.left + b;
                    let entry = col_baselines.entry(item.col_start).or_insert(0.0_f32);
                    *entry = entry.max(baseline);
                } else {
                    let baseline = item.margin.left
                        + item.pb.left
                        + prelim_lb.content.size.width
                        + item.pb.right;
                    let entry = col_baselines.entry(item.col_start).or_insert(0.0_f32);
                    *entry = entry.max(baseline);
                }
            }
        }
    }

    for item in items {
        // Compute the grid area rectangle.
        // CSS Grid §7.1: column tracks = inline axis, row tracks = block axis.
        // In horizontal: column → X, row → Y.
        // In vertical: column → Y (inline axis), row → X (block axis).
        let col_pos = col_positions.get(item.col_start).copied().unwrap_or(0.0);
        let col_size = cell_span_size(col_tracks, col_positions, item.col_start, item.col_span);
        let row_pos = row_positions.get(item.row_start).copied().unwrap_or(0.0);
        let row_size = cell_span_size(row_tracks, row_positions, item.row_start, item.row_span);

        let (area_x, area_y, area_width, area_height) = if is_horizontal_wm {
            // RTL: mirror column position so columns flow right-to-left.
            let area_x = if is_rtl {
                (container_inline_size - col_pos - col_size).max(0.0)
            } else {
                col_pos
            };
            (area_x, row_pos, col_size, row_size)
        } else {
            // Vertical: columns → Y (inline axis), rows → X (block axis).
            let area_y = if is_rtl {
                (container_inline_size - col_pos - col_size).max(0.0)
            } else {
                col_pos
            };
            (row_pos, area_y, row_size, col_size)
        };

        // Available space for the item after subtracting margins.
        let avail_w = (area_width - item.margin.left - item.margin.right).max(0.0);
        let avail_h = (area_height - item.margin.top - item.margin.bottom).max(0.0);

        // Resolve item content width.
        // CSS Grid §11.3: the containing block for a grid item is the grid area.
        // Percentage widths resolve against the grid area size (before margins).
        let child_style = elidex_layout_block::get_style(dom, item.entity);
        let item_content_w = if item.width_auto && item.justify != JustifyItems::Stretch {
            // Non-stretch: use content width (shrink-wrap).
            item.content_size.width
        } else if item.width_auto {
            (avail_w - item.pb.left - item.pb.right).max(0.0)
        } else {
            resolve_item_dimension(
                child_style.width,
                area_width,
                item.pb.left + item.pb.right,
                child_style.box_sizing,
            )
            .max(0.0)
        };

        // Preliminary layout to measure content height.
        // CSS Grid §11.3: margin/padding % on grid items resolves against the
        // grid area's content-box dimension (before margins), not avail (after margins).
        let item_inline_size = if is_horizontal_wm {
            area_width
        } else {
            area_height
        };
        let prelim_input = LayoutInput {
            containing: CssSize {
                width: area_width,
                height: containing_height,
            },
            containing_inline_size: item_inline_size,
            ..LayoutInput::probe(env, area_width)
        };
        let prelim_lb = (env.layout_child)(dom, item.entity, &prelim_input).layout_box;

        // Resolve item content height: stretch fills the area, otherwise use content.
        let prelim_content_h = prelim_lb.content.size.height;
        let item_content_h = if should_stretch_cross(item.align, item.height_auto) {
            (avail_h - item.pb.top - item.pb.bottom).max(prelim_content_h)
        } else {
            prelim_content_h
        };

        let item_outer_h =
            item_content_h + item.pb.top + item.pb.bottom + item.margin.top + item.margin.bottom;

        // Cross-axis alignment (vertical) — baseline-aware.
        // Multi-row items don't participate in baseline sharing (CSS Grid §10.6).
        let item_align_baseline = if item.align == AlignItems::Baseline && item.row_span == 1 {
            prelim_lb
                .first_baseline
                .map(|b| item.margin.top + item.pb.top + b)
        } else {
            None
        };
        let row_bl = row_baselines.get(&item.row_start).copied().unwrap_or(0.0);
        let y_offset = compute_alignment_offset(
            item.align,
            area_height,
            item_outer_h,
            item_align_baseline,
            row_bl,
        );

        // Inline-axis alignment (horizontal) — baseline-aware.
        let item_outer_w =
            item_content_w + item.pb.left + item.pb.right + item.margin.left + item.margin.right;
        // Multi-column items don't participate in baseline sharing (CSS Grid §10.6).
        let item_justify_baseline = if item.justify == JustifyItems::Baseline && item.col_span == 1
        {
            prelim_lb
                .first_baseline
                .map(|b| item.margin.left + item.pb.left + b)
        } else {
            None
        };
        let col_bl = col_baselines.get(&item.col_start).copied().unwrap_or(0.0);
        let x_offset = compute_justify_offset(
            item.justify,
            area_width,
            item_outer_w,
            item_justify_baseline,
            col_bl,
        );

        // Override the child's width/height so layout_block_inner uses grid-resolved values.
        {
            let mut style = elidex_layout_block::get_style(dom, item.entity);
            style.width = Dimension::Length(item_content_w);
            style.height = Dimension::Length(item_content_h);
            let _ = dom.world_mut().insert_one(item.entity, style);
        }

        // Margin-box position: layout_child (layout_block_inner) adds
        // margin + border + padding offsets from here.
        let margin_box_x = content_x + area_x + x_offset;
        let margin_box_y = content_y + area_y + y_offset;

        // Build child SubgridContext if this item is a subgrid (CSS Grid Level 2 §2).
        let child_subgrid_ctx = if let Some(parent) = placement
            .subgrid_ctx
            .as_ref()
            .filter(|_| item.is_subgrid_cols || item.is_subgrid_rows)
        {
            let child_style = elidex_layout_block::get_style(dom, item.entity);
            let (child_col_sizes, child_col_names) = extract_axis_subgrid(
                item.is_subgrid_cols,
                parent.col_sizes.as_ref(),
                &parent.col_line_names,
                item.col_start,
                item.col_span,
                &child_style.grid_template_columns,
            );
            let (child_row_sizes, child_row_names) = extract_axis_subgrid(
                item.is_subgrid_rows,
                parent.row_sizes.as_ref(),
                &parent.row_line_names,
                item.row_start,
                item.row_span,
                &child_style.grid_template_rows,
            );
            Some(SubgridContext {
                col_sizes: child_col_sizes,
                row_sizes: child_row_sizes,
                col_line_names: child_col_names,
                row_line_names: child_row_names,
                col_gap: parent.col_gap,
                row_gap: parent.row_gap,
            })
        } else {
            None
        };

        // Final layout at resolved position.
        // CSS Grid §11.3: margin/padding % resolves against the grid area's
        // content-box dimension, consistent with preliminary passes.
        let final_inline_size = if is_horizontal_wm {
            area_width
        } else {
            area_height
        };
        let final_input = LayoutInput {
            containing: CssSize::definite(area_width, item_content_h),
            containing_inline_size: final_inline_size,
            offset: Point::new(margin_box_x, margin_box_y),
            font_db: env.font_db,
            depth: env.depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
            subgrid: child_subgrid_ctx.as_ref(),
        };
        let final_lb = (env.layout_child)(dom, item.entity, &final_input).layout_box;

        // Ensure the content height matches the grid-resolved value.
        if (item_content_h - final_lb.content.size.height).abs() > LAYOUT_SIZE_EPSILON {
            let corrected = LayoutBox {
                content: Rect::new(
                    final_lb.content.origin.x,
                    final_lb.content.origin.y,
                    item_content_w,
                    item_content_h,
                ),
                ..final_lb
            };
            let _ = dom.world_mut().insert_one(item.entity, corrected);
        }
    }

    // Grid container baseline (CSS Grid §4.2):
    // Use the shared baseline of the first row if baseline alignment was used,
    // otherwise fall back to the first item in row 0's baseline from final layout.
    if let Some(&first_row_bl) = row_baselines.get(&0) {
        Some(first_row_bl)
    } else {
        // Find first item placed in row 0 (not source-order first).
        items
            .iter()
            .find(|i| i.row_start == 0)
            .and_then(|first_item| {
                dom.world()
                    .get::<&LayoutBox>(first_item.entity)
                    .ok()
                    .and_then(|lb| {
                        lb.first_baseline
                            .map(|bl| lb.content.origin.y - content_y + bl)
                    })
            })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
    let end = start.saturating_add(span).min(tracks.len());
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
///
/// `item_baseline` is the item's baseline measured from its margin-box edge.
/// `shared_baseline` is the shared baseline for the item's row/column, computed
/// in pass 1. Used by both cross-axis (align) and inline-axis (justify).
fn grid_align_offset(
    center: bool,
    end: bool,
    baseline: bool,
    available: f32,
    item_size: f32,
    item_baseline: Option<f32>,
    shared_baseline: f32,
) -> f32 {
    if baseline {
        let item_bl = item_baseline.unwrap_or(item_size.max(0.0));
        (shared_baseline - item_bl).max(0.0)
    } else if center {
        (available - item_size).max(0.0) / 2.0
    } else if end {
        (available - item_size).max(0.0)
    } else {
        // Start, Stretch — align to start.
        0.0
    }
}

/// Compute cross-axis (vertical) alignment offset for a grid item.
fn compute_alignment_offset(
    item_align: AlignItems,
    available: f32,
    item_size: f32,
    item_baseline: Option<f32>,
    row_baseline: f32,
) -> f32 {
    grid_align_offset(
        item_align == AlignItems::Center,
        item_align == AlignItems::FlexEnd,
        item_align == AlignItems::Baseline,
        available,
        item_size,
        item_baseline,
        row_baseline,
    )
}

/// Compute inline-axis (horizontal) alignment offset for a grid item.
fn compute_justify_offset(
    justify: JustifyItems,
    available: f32,
    item_size: f32,
    item_baseline: Option<f32>,
    col_baseline: f32,
) -> f32 {
    grid_align_offset(
        justify == JustifyItems::Center,
        justify == JustifyItems::End,
        justify == JustifyItems::Baseline,
        available,
        item_size,
        item_baseline,
        col_baseline,
    )
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
