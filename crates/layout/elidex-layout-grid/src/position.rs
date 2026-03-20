//! Grid item positioning within resolved track areas.
//!
//! Handles alignment (align-items/self, justify-items/self),
//! baseline alignment, stretch, and final child layout dispatch.

use std::collections::HashMap;

use elidex_ecs::EcsDom;
use elidex_layout_block::{ChildLayoutFn, LayoutInput};
use elidex_plugin::{AlignItems, Dimension, Direction, JustifyItems, LayoutBox, Rect};
use elidex_text::FontDatabase;

use crate::track;
use crate::{GridItem, LAYOUT_SIZE_EPSILON};

// ---------------------------------------------------------------------------
// GridPlacement
// ---------------------------------------------------------------------------

/// Grid track layout and positioning context.
pub(crate) struct GridPlacement<'a> {
    pub(crate) col_tracks: &'a [track::ResolvedTrack],
    pub(crate) row_tracks: &'a [track::ResolvedTrack],
    pub(crate) col_positions: &'a [f32],
    pub(crate) row_positions: &'a [f32],
    pub(crate) content_x: f32,
    pub(crate) content_y: f32,
    pub(crate) content_width: f32,
    pub(crate) direction: Direction,
    pub(crate) containing_height: Option<f32>,
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
    font_db: &FontDatabase,
    depth: u32,
    layout_child: ChildLayoutFn,
) -> Option<f32> {
    let is_rtl = placement.direction == Direction::Rtl;
    let content_x = placement.content_x;
    let content_y = placement.content_y;
    let content_width = placement.content_width;
    let containing_height = placement.containing_height;
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
            let area_width =
                cell_span_size(col_tracks, col_positions, item.col_start, item.col_span);
            let avail_w = (area_width - item.margin.left - item.margin.right).max(0.0);

            let prelim_input = LayoutInput {
                containing_width: avail_w,
                containing_height,
                offset_x: 0.0,
                offset_y: 0.0,
                font_db,
                depth: depth + 1,
                float_ctx: None,
                viewport: None,
                fragmentainer: None,
                break_token: None,
            };
            let prelim_lb = layout_child(dom, item.entity, &prelim_input).layout_box;

            if align_bl {
                // CSS Grid §10.6: synthesized baseline = border-box bottom from
                // margin-box top. Items without a baseline use the border-box bottom.
                if let Some(b) = prelim_lb.first_baseline {
                    let baseline = item.margin.top + item.pb.top + b;
                    let entry = row_baselines.entry(item.row_start).or_insert(0.0_f32);
                    *entry = entry.max(baseline);
                } else {
                    // Border-box bottom from margin-box top.
                    let baseline =
                        item.margin.top + item.pb.top + prelim_lb.content.height + item.pb.bottom;
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
                    let baseline =
                        item.margin.left + item.pb.left + prelim_lb.content.width + item.pb.right;
                    let entry = col_baselines.entry(item.col_start).or_insert(0.0_f32);
                    *entry = entry.max(baseline);
                }
            }
        }
    }

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
        let item_content_w = if item.width_auto && item.justify != JustifyItems::Stretch {
            // Non-stretch: use content width (shrink-wrap).
            item.content_width
        } else if item.width_auto {
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
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
        };
        let prelim_lb = layout_child(dom, item.entity, &prelim_input).layout_box;

        // Resolve item content height: stretch fills the area, otherwise use content.
        let prelim_content_h = prelim_lb.content.height;
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

        // Final layout at resolved position.
        let final_input = LayoutInput {
            containing_width: area_width,
            containing_height: Some(item_content_h),
            offset_x: margin_box_x,
            offset_y: margin_box_y,
            font_db,
            depth: depth + 1,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
        };
        let final_lb = layout_child(dom, item.entity, &final_input).layout_box;

        // Ensure the content height matches the grid-resolved value.
        if (item_content_h - final_lb.content.height).abs() > LAYOUT_SIZE_EPSILON {
            let corrected = LayoutBox {
                content: Rect::new(
                    final_lb.content.x,
                    final_lb.content.y,
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
                    .and_then(|lb| lb.first_baseline.map(|bl| lb.content.y - content_y + bl))
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
