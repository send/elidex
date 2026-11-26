//! Grid item positioning within resolved track areas.
//!
//! Handles alignment (align-items/self, justify-items/self),
//! baseline alignment, stretch, and final child layout dispatch.

use std::collections::HashMap;

use elidex_ecs::EcsDom;
use elidex_layout_block::{LayoutEnv, LayoutInput, SubgridContext};
use elidex_plugin::{
    AlignItems, CssSize, Direction, GridTrackList, JustifyItems, LayoutBox, Point, Size,
    WritingMode,
};

use crate::track;
use crate::GridItem;

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
    /// Subgrid context for passing parent track sizes to subgrid children.
    pub(crate) subgrid_ctx: Option<&'a SubgridContext>,
    /// Container's writing mode (CSS Writing Modes §6.3).
    pub(crate) writing_mode: WritingMode,
}

// ---------------------------------------------------------------------------
// position_items
// ---------------------------------------------------------------------------

pub(super) struct ItemFragmentation<'a> {
    pub owner: elidex_ecs::Entity,
    pub contribution: elidex_layout_block::source_progress::SourceProgress,
    pub room: Option<&'a elidex_layout_block::FragmentainerContext>,
    pub resume_row: usize,
    pub incoming: &'a HashMap<elidex_ecs::Entity, &'a elidex_layout_block::BreakToken>,
    pub outgoing: Vec<(elidex_ecs::Entity, Box<elidex_layout_block::BreakToken>)>,
    pub block_extent: f32,
    pub incomplete: bool,
}

fn item_baseline(lb: &LayoutBox, x_axis: bool) -> f32 {
    if x_axis {
        lb.margin.left
            + lb.border.left
            + lb.padding.left
            + lb.first_baseline
                .unwrap_or(lb.content.size.width + lb.padding.right + lb.border.right)
    } else {
        lb.margin.top
            + lb.border.top
            + lb.padding.top
            + lb.first_baseline
                .unwrap_or(lb.content.size.height + lb.padding.bottom + lb.border.bottom)
    }
}

fn measure_in_area(
    dom: &mut EcsDom,
    item: &GridItem,
    env: &LayoutEnv<'_>,
    area_width: f32,
    area_height: f32,
    is_horizontal_wm: bool,
) -> (elidex_layout_block::block::GridItemSizing, LayoutBox) {
    let area = CssSize::definite(area_width, area_height);
    let item_inline_size = if is_horizontal_wm {
        area_width
    } else {
        area_height
    };
    let inline_stretch = item.justify == JustifyItems::Stretch;
    let block_stretch = item.align == AlignItems::Stretch;
    let stretch = if is_horizontal_wm {
        Size {
            width: inline_stretch,
            height: block_stretch,
        }
    } else {
        Size {
            width: block_stretch,
            height: inline_stretch,
        }
    };
    let sizing = elidex_layout_block::block::GridItemSizing {
        stretch,
        intrinsic: Size {
            width: Some((item.content_size.width - item.pb.horizontal()).max(0.0)),
            height: None,
        },
    };
    let prelim_input = LayoutInput {
        item_context: elidex_layout_block::ItemContext::GridArea(sizing),
        containing: area,
        containing_inline_size: item_inline_size,
        ..LayoutInput::probe(env, area_width)
    };
    let prelim_lb = (env.layout_child)(dom, item.entity, &prelim_input).layout_box;
    let sizing = elidex_layout_block::block::GridItemSizing {
        intrinsic: Size {
            width: sizing.intrinsic.width,
            height: Some(prelim_lb.content.size.height),
        },
        ..sizing
    };
    (sizing, prelim_lb)
}

/// Position each item within its grid area and perform final layout.
///
/// Returns the grid container's first baseline (CSS Grid §4.2).
#[allow(clippy::too_many_lines)]
pub(crate) fn position_items(
    dom: &mut EcsDom,
    items: &[GridItem],
    placement: &GridPlacement<'_>,
    env: &LayoutEnv<'_>,
    entry: &mut ItemFragmentation<'_>,
) -> Option<f32> {
    let is_rtl = placement.direction == Direction::Rtl;
    let is_horizontal_wm = placement.writing_mode.is_horizontal();
    let content_x = placement.content_origin.x;
    let content_y = placement.content_origin.y;
    let container_inline_size = placement.container_inline_size;
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
            let (_, lb) = measure_in_area(dom, item, env, bl_area_w, bl_area_h, is_horizontal_wm);
            if align_bl {
                let entry = row_baselines.entry(item.row_start).or_insert(0.0);
                *entry = entry.max(item_baseline(&lb, !is_horizontal_wm));
            }
            if justify_bl {
                let entry = col_baselines.entry(item.col_start).or_insert(0.0);
                *entry = entry.max(item_baseline(&lb, is_horizontal_wm));
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

        // Grid 2 §§6.1–6.2: tracks establish the containing block. The item
        // formatter fits its content box in that area before alignment.
        let child_style = elidex_layout_block::get_style(dom, item.entity);
        let area = CssSize::definite(area_width, area_height);
        let item_inline_size = if is_horizontal_wm {
            area_width
        } else {
            area_height
        };
        let (sizing, prelim_lb) =
            measure_in_area(dom, item, env, area_width, area_height, is_horizontal_wm);
        let outer = prelim_lb.margin_box().size;
        let (outer_inline, outer_block) = if is_horizontal_wm {
            (outer.width, outer.height)
        } else {
            (outer.height, outer.width)
        };
        let block_offset = compute_alignment_offset(
            item.align,
            row_size,
            outer_block,
            (item.row_span == 1).then(|| item_baseline(&prelim_lb, !is_horizontal_wm)),
            row_baselines.get(&item.row_start).copied().unwrap_or(0.0),
        );
        let inline_offset = compute_justify_offset(
            item.justify,
            col_size,
            outer_inline,
            (item.col_span == 1).then(|| item_baseline(&prelim_lb, is_horizontal_wm)),
            col_baselines.get(&item.col_start).copied().unwrap_or(0.0),
        );
        let (x_offset, y_offset) = if is_horizontal_wm {
            (inline_offset, block_offset)
        } else {
            (block_offset, inline_offset)
        };

        // Build child SubgridContext if this item is a subgrid (CSS Grid Level 2 §2).
        let child_subgrid_ctx = if let Some(parent) = placement
            .subgrid_ctx
            .as_ref()
            .filter(|_| item.is_subgrid_cols || item.is_subgrid_rows)
        {
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
        let row_offset = if entry.resume_row == 0 {
            row_pos
        } else {
            (row_pos
                - row_positions
                    .get(entry.resume_row)
                    .copied()
                    .unwrap_or(row_pos))
            .max(0.0)
        };
        let incoming_token = entry.incoming.get(&item.entity).copied();
        // The alignment offset precedes the first piece only.
        let entry_offset = if incoming_token.is_some() {
            0.0
        } else {
            row_offset + if is_horizontal_wm { y_offset } else { x_offset }
        };
        let offset = if is_horizontal_wm {
            Point::new(content_x + area_x + x_offset, content_y + entry_offset)
        } else {
            Point::new(content_x + entry_offset, content_y + area_y + y_offset)
        };
        let room = entry
            .room
            .map(|room| elidex_layout_block::FragmentainerContext {
                available_block_size: (room.available_block_size - entry_offset).max(0.0),
                ..*room
            });
        let final_input = LayoutInput {
            available_height_if_indefinite: None,
            containing_width_definite: true,
            item_context: elidex_layout_block::ItemContext::GridArea(sizing),
            assigned: elidex_layout_block::UsedAssignment::default(),
            containing: area,
            containing_inline_size: item_inline_size,
            offset,
            font_db: env.font_db,
            depth: env.depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: room.as_ref(),
            break_token: incoming_token,
            subgrid: child_subgrid_ctx.as_ref(),
            layout_generation: env.layout_generation,
            medium: env.medium,
            placement: env.placement.at_offset(entry_offset),
            is_probe: env.is_probe,
        };
        let outcome = (env.layout_child)(dom, item.entity, &final_input);
        let final_lb = outcome.layout_box;
        entry.contribution.include(&outcome.contribution);
        if matches!(outcome.end, elidex_layout_block::FragmentEnd::Incomplete) {
            entry.incomplete = true;
            break;
        }
        if let elidex_layout_block::FragmentEnd::Continue(token) = outcome.end {
            entry.contribution.record_item(entry.owner, item.entity);
            entry.outgoing.push((item.entity, Box::new(token)));
        } else {
            entry.contribution.finish_item(entry.owner, item.entity);
        }
        if room.is_some() {
            let margin_box = final_lb.margin_box();
            let extent = if is_horizontal_wm {
                margin_box.size.height
            } else {
                margin_box.size.width
            };
            entry.block_extent = entry.block_extent.max(entry_offset + extent);
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
