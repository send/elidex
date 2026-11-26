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
    pub(crate) block_extent: f32,
    pub(crate) direction: Direction,
    /// Subgrid context for passing parent track sizes to subgrid children.
    pub(crate) subgrid_ctx: Option<&'a SubgridContext>,
    /// Container's writing mode (CSS Writing Modes §6.3).
    pub(crate) writing_mode: WritingMode,
}

// ---------------------------------------------------------------------------
// position_items
// ---------------------------------------------------------------------------

impl GridPlacement<'_> {
    fn area(&self, item: &GridItem) -> elidex_plugin::Rect {
        let is_rtl = self.direction == Direction::Rtl;
        let is_horizontal_wm = self.writing_mode.is_horizontal();
        let col_pos = self
            .col_positions
            .get(item.col_start)
            .copied()
            .unwrap_or(0.0);
        let col_size = cell_span_size(
            self.col_tracks,
            self.col_positions,
            item.col_start,
            item.col_span,
        );
        let row_pos = self
            .row_positions
            .get(item.row_start)
            .copied()
            .unwrap_or(0.0);
        let row_size = cell_span_size(
            self.row_tracks,
            self.row_positions,
            item.row_start,
            item.row_span,
        );

        let (area_x, area_y, area_width, area_height) = if is_horizontal_wm {
            // RTL: mirror column position so columns flow right-to-left.
            let area_x = if is_rtl {
                self.container_inline_size - col_pos - col_size
            } else {
                col_pos
            };
            (area_x, row_pos, col_size, row_size)
        } else {
            // Vertical: columns → Y (inline axis), rows → X (block axis).
            let area_y = if is_rtl {
                self.container_inline_size - col_pos - col_size
            } else {
                col_pos
            };
            let x = if self.writing_mode == WritingMode::VerticalRl {
                self.block_extent - row_pos - row_size
            } else {
                row_pos
            };
            (x, area_y, row_size, col_size)
        };

        elidex_plugin::Rect::from_origin_size(
            Point::new(
                self.content_origin.x + area_x,
                self.content_origin.y + area_y,
            ),
            Size::new(area_width, area_height),
        )
    }
    fn child_subgrid(
        &self,
        item: &GridItem,
        child_style: &elidex_plugin::ComputedStyle,
    ) -> Option<SubgridContext> {
        if let Some(parent) = self
            .subgrid_ctx
            .as_ref()
            .filter(|_| item.is_subgrid_cols || item.is_subgrid_rows)
        {
            let parallel =
                self.writing_mode.is_horizontal() == child_style.writing_mode.is_horizontal();
            let (child_col_sizes, child_col_names) = extract_axis_subgrid(
                item.is_subgrid_cols,
                parent.col_sizes.as_ref(),
                &parent.col_line_names,
                item.col_start,
                item.col_span,
                if parallel {
                    &child_style.grid_template_columns
                } else {
                    &child_style.grid_template_rows
                },
            );
            let (child_row_sizes, child_row_names) = extract_axis_subgrid(
                item.is_subgrid_rows,
                parent.row_sizes.as_ref(),
                &parent.row_line_names,
                item.row_start,
                item.row_span,
                if parallel {
                    &child_style.grid_template_rows
                } else {
                    &child_style.grid_template_columns
                },
            );
            let mut context = SubgridContext {
                col_sizes: child_col_sizes,
                row_sizes: child_row_sizes,
                col_line_names: child_col_names,
                row_line_names: child_row_names,
                col_gap: parent.col_gap,
                row_gap: parent.row_gap,
            };
            if !parallel {
                std::mem::swap(&mut context.col_sizes, &mut context.row_sizes);
                std::mem::swap(&mut context.col_line_names, &mut context.row_line_names);
                std::mem::swap(&mut context.col_gap, &mut context.row_gap);
            }
            Some(context)
        } else {
            None
        }
    }
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

fn measure_in_area<'a>(
    dom: &mut EcsDom,
    item: &GridItem,
    env: &LayoutEnv<'a>,
    rect: elidex_plugin::Rect,
    subgrid: Option<&'a SubgridContext>,
    is_horizontal_wm: bool,
    style: &elidex_plugin::ComputedStyle,
) -> (LayoutInput<'a>, LayoutBox) {
    let (area_width, area_height) = (rect.size.width, rect.size.height);
    let area = CssSize::definite(area_width, area_height);
    let item_inline_size = if is_horizontal_wm {
        area_width
    } else {
        area_height
    };
    let natural = elidex_layout_block::get_intrinsic_size(dom, item.entity);
    let inline_stretch = item.justify == JustifyItems::Stretch
        || (item.justify == JustifyItems::Normal && natural.is_none());
    let block_stretch = item.align == AlignItems::Stretch
        || (item.align == AlignItems::Normal && natural.is_none());
    let physical_axes = |inline, block| {
        if is_horizontal_wm {
            Size {
                width: inline,
                height: block,
            }
        } else {
            Size {
                width: block,
                height: inline,
            }
        }
    };
    let inner_inline = elidex_layout_block::intrinsic::inner_intrinsic_sizes(
        dom,
        item.entity,
        &env.deeper(),
        crate::compute_grid_intrinsic,
    );
    let sizing = elidex_layout_block::block::GridItemSizing {
        stretch: physical_axes(inline_stretch, block_stretch),
        subgridded: physical_axes(item.is_subgrid_cols, item.is_subgrid_rows),
        inner_inline,
        replaced: natural.is_some(),
        intrinsic: Size {
            width: None,
            height: None,
        },
    };
    let prelim_input = LayoutInput {
        item_context: elidex_layout_block::ItemContext::GridArea(sizing),
        containing: area,
        containing_inline_size: item_inline_size,
        subgrid,
        offset: rect.origin,
        ..LayoutInput::probe(env, area_width)
    };
    let mut prelim_lb = (env.layout_child)(dom, item.entity, &prelim_input).layout_box;
    let sizing = elidex_layout_block::block::GridItemSizing {
        intrinsic: Size {
            width: Some(prelim_lb.content.size.width),
            height: Some(prelim_lb.content.size.height),
        },
        ..sizing
    };
    // Natural block size is known only after layout. Resolve its auto margins
    // before using the margin box as the alignment subject.
    let measured = LayoutInput {
        item_context: elidex_layout_block::ItemContext::GridArea(sizing),
        ..prelim_input
    }
    .resolve_item(style);
    prelim_lb.margin = measured.assigned.margins(prelim_lb.margin);
    (measured, prelim_lb)
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
) -> Option<f32> {
    let is_rtl = placement.direction == Direction::Rtl;
    let is_horizontal_wm = placement.writing_mode.is_horizontal();
    let content_y = placement.content_origin.y;
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
            let style = elidex_layout_block::get_style(dom, item.entity);
            let subgrid = placement.child_subgrid(item, &style);
            let (_, lb) = measure_in_area(
                dom,
                item,
                env,
                placement.area(item),
                subgrid.as_ref(),
                is_horizontal_wm,
                &style,
            );
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
        let rect = placement.area(item);
        let (area_width, area_height) = (rect.size.width, rect.size.height);
        let (col_size, row_size) = if is_horizontal_wm {
            (area_width, area_height)
        } else {
            (area_height, area_width)
        };
        // Grid 2 §§6.1–6.2: tracks establish the containing block. The item
        // formatter fits its content box in that area before alignment.
        let child_style = elidex_layout_block::get_style(dom, item.entity);
        let child_subgrid_ctx = placement.child_subgrid(item, &child_style);
        let (measured, prelim_lb) = measure_in_area(
            dom,
            item,
            env,
            rect,
            child_subgrid_ctx.as_ref(),
            is_horizontal_wm,
            &child_style,
        );
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
        let block_offset = place_axis(
            block_offset,
            row_size - outer_block,
            outer_block,
            placement.writing_mode == WritingMode::VerticalRl,
            item.block_safety,
            if is_horizontal_wm {
                rect.origin.y
            } else {
                rect.origin.x
            },
            if is_horizontal_wm {
                env.scroll_limits.y
            } else {
                env.scroll_limits.x
            },
        );
        let inline_offset = place_axis(
            inline_offset,
            col_size - outer_inline,
            outer_inline,
            is_rtl,
            item.inline_safety,
            if is_horizontal_wm {
                rect.origin.x
            } else {
                rect.origin.y
            },
            if is_horizontal_wm {
                env.scroll_limits.x
            } else {
                env.scroll_limits.y
            },
        );
        let (x_offset, y_offset) = if is_horizontal_wm {
            (inline_offset, block_offset)
        } else {
            (block_offset, inline_offset)
        };

        // Place the same resolved item whose margin box was aligned above.
        let final_input = LayoutInput {
            offset: Point::new(rect.origin.x + x_offset, rect.origin.y + y_offset),
            viewport: None,
            layout_generation: env.layout_generation,
            is_probe: env.is_probe,
            ..measured
        };
        (env.layout_child)(dom, item.entity, &final_input);
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
        (available - item_size) / 2.0
    } else if end {
        available - item_size
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
        matches!(item_align, AlignItems::End | AlignItems::FlexEnd),
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

/// Map a signed logical margin-box offset to the physical low edge.
fn place_axis(
    requested: f32,
    free: f32,
    extent: f32,
    reversed: bool,
    safety: Option<elidex_plugin::AlignmentSafety>,
    origin: f32,
    limit: Option<elidex_layout_block::ScrollEdge>,
) -> f32 {
    let logical = if free < 0.0 && safety == Some(elidex_plugin::AlignmentSafety::Safe) {
        0.0
    } else {
        requested
    };
    let physical = if reversed { free - logical } else { logical };
    if free < 0.0 && safety.is_none() {
        if let Some(edge) = limit {
            // Limit only the overflow introduced by alignment; the area itself can overflow.
            return match edge {
                elidex_layout_block::ScrollEdge::Low(low) => physical.max((low - origin).min(0.0)),
                elidex_layout_block::ScrollEdge::High(high) => {
                    physical.min((high - origin - extent).max(free))
                }
            };
        }
    }
    physical
}
