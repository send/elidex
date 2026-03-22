//! Helper functions for grid layout: item collection, measurement,
//! track definitions, content distribution, abs-pos containing blocks,
//! and intrinsic sizing.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    effective_align, sanitize_border, ChildLayoutFn, LayoutEnv, LayoutInput,
};
use elidex_plugin::{
    AlignContent, AlignmentSafety, ComputedStyle, CssSize, Dimension, Display, EdgeSizes,
    GridAutoFlow, JustifyContent, JustifyItems, JustifySelf, Point, Rect, Size, TrackSize,
};

use elidex_layout_block::block::resolve_margin;

/// Axis-specific grid track info for abs-pos containing block resolution.
pub(crate) struct GridAxisInfo<'a> {
    pub(crate) positions: &'a [f32],
    pub(crate) tracks: &'a [track::ResolvedTrack],
    pub(crate) gap: f32,
    pub(crate) name_map: &'a placement::LineNameMap,
    pub(crate) explicit_count: usize,
}

use crate::placement;
use crate::track;
use crate::GridItem;

// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

/// Collect grid items from children, skipping `display:none` and text nodes.
pub(crate) fn collect_grid_items(
    dom: &mut EcsDom,
    children: &[Entity],
    container_style: &ComputedStyle,
) -> Vec<GridItem> {
    let mut items = Vec::new();
    for (i, &child) in children.iter().enumerate() {
        let Some(mut child_style) = elidex_layout_block::try_get_style(dom, child) else {
            continue; // Text node — skip.
        };
        if child_style.display == Display::None {
            continue;
        }
        // Absolutely positioned grid children are removed from grid layout.
        if elidex_layout_block::positioned::is_absolutely_positioned(&child_style) {
            continue;
        }

        // Grid §6.1: blockify grid items.
        let blockified = child_style.display.blockify();
        if blockified != child_style.display {
            child_style.display = blockified;
            let _ = dom.world_mut().insert_one(child, child_style.clone());
        }

        let align = effective_align(child_style.align_self, container_style.align_items);
        let justify = effective_justify(child_style.justify_self, container_style.justify_items);

        // Detect subgrid (CSS Grid Level 2 §2).
        let child_is_grid = matches!(child_style.display, Display::Grid | Display::InlineGrid);
        let is_subgrid_cols = child_is_grid && child_style.grid_template_columns.is_subgrid();
        let is_subgrid_rows = child_is_grid && child_style.grid_template_rows.is_subgrid();

        items.push(GridItem {
            entity: child,
            source_order: i,
            order: child_style.order,
            row_start: 0,
            col_start: 0,
            row_span: 1,
            col_span: 1,
            grid_row_start: child_style.grid_row_start.clone(),
            grid_row_end: child_style.grid_row_end.clone(),
            grid_column_start: child_style.grid_column_start.clone(),
            grid_column_end: child_style.grid_column_end.clone(),
            margin: EdgeSizes::default(),
            pb: EdgeSizes::default(),
            align,
            justify,
            height_auto: child_style.height == Dimension::Auto,
            width_auto: child_style.width == Dimension::Auto,
            content_size: Size::ZERO,
            min_content_size: Size::ZERO,
            is_subgrid_cols,
            is_subgrid_rows,
        });
    }
    items
}

// ---------------------------------------------------------------------------
// Content measurement
// ---------------------------------------------------------------------------

/// Measure each item's content size via a preliminary layout.
///
/// `parent_subgrid` is the parent's `SubgridContext`, passed through to subgrid
/// items during min/max-content probes so nested subgrids receive parent track
/// sizes (CSS Grid Level 2 §2).
pub(crate) fn measure_item_content(
    dom: &mut EcsDom,
    items: &mut [GridItem],
    container_width: f32,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) {
    let containing_height = input.containing.height;
    let containing_inline_size = input.containing_inline_size;
    let parent_subgrid = input.subgrid;
    let probe_env = LayoutEnv::from_input(input, layout_child);
    for item in items.iter_mut() {
        let child_style = elidex_layout_block::get_style(dom, item.entity);
        let padding = elidex_layout_block::resolve_padding(&child_style, containing_inline_size);
        let border = sanitize_border(&child_style);
        item.pb = EdgeSizes::new(
            padding.top + border.top,
            padding.right + border.right,
            padding.bottom + border.bottom,
            padding.left + border.left,
        );
        item.margin = EdgeSizes::new(
            resolve_margin(child_style.margin_top, containing_inline_size),
            resolve_margin(child_style.margin_right, containing_inline_size),
            resolve_margin(child_style.margin_bottom, containing_inline_size),
            resolve_margin(child_style.margin_left, containing_inline_size),
        );

        // For subgrid items, pass parent's subgrid context through so nested
        // subgrids receive parent track sizes during intrinsic probes.
        let probe_subgrid = if item.is_subgrid_cols || item.is_subgrid_rows {
            parent_subgrid
        } else {
            None
        };

        // Min-content probe: layout at near-zero width (CSS Grid §12.3).
        // Save descendant styles first — layout probes can mutate styles
        // (e.g. flex's relayout_item_at_position overwrites child widths).
        let saved_styles = elidex_layout_block::save_descendant_styles(dom, item.entity);
        let min_input = LayoutInput {
            containing: CssSize {
                width: 1.0,
                height: containing_height,
            },
            subgrid: probe_subgrid,
            ..LayoutInput::probe(&probe_env, 1.0)
        };
        let min_lb = layout_child(dom, item.entity, &min_input).layout_box;
        item.min_content_size = Size::new(
            min_lb.content.size.width + item.pb.horizontal(),
            min_lb.content.size.height + item.pb.vertical(),
        );
        // Restore styles corrupted by the min-content probe.
        elidex_layout_block::restore_descendant_styles(dom, &saved_styles);

        // Max-content probe: layout at container width.
        let max_input = LayoutInput {
            containing: CssSize {
                width: container_width,
                height: containing_height,
            },
            subgrid: probe_subgrid,
            ..LayoutInput::probe(&probe_env, container_width)
        };
        let max_lb = layout_child(dom, item.entity, &max_input).layout_box;
        item.content_size = Size::new(
            max_lb.content.size.width + item.pb.horizontal(),
            max_lb.content.size.height + item.pb.vertical(),
        );
    }
}

/// Compute intrinsic content sizes per track.
///
/// Returns `(min_content_sizes, max_content_sizes)` per track.
/// For items spanning a single track, contribute their full size.
/// For multi-span items, distribute proportionally.
#[allow(clippy::cast_precision_loss)]
/// Build per-item track contributions for the track sizing algorithm.
pub(crate) fn build_contributions(
    items: &[GridItem],
    is_column: bool,
) -> Vec<track::TrackContribution> {
    items
        .iter()
        .map(|item| {
            if is_column {
                track::TrackContribution {
                    start: item.col_start,
                    span: item.col_span,
                    min_content: item.min_content_size.width + item.margin.left + item.margin.right,
                    max_content: item.content_size.width + item.margin.left + item.margin.right,
                }
            } else {
                track::TrackContribution {
                    start: item.row_start,
                    span: item.row_span,
                    min_content: item.min_content_size.height
                        + item.margin.top
                        + item.margin.bottom,
                    max_content: item.content_size.height + item.margin.top + item.margin.bottom,
                }
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Track definitions
// ---------------------------------------------------------------------------

/// Convert percentage tracks to auto (CSS Grid §7.2.1).
///
/// When the available size on an axis is indefinite, percentage-sized tracks
/// are treated as auto so that intrinsic content sizing takes over.
pub(crate) fn percentage_tracks_to_auto(defs: Vec<TrackSize>) -> Vec<TrackSize> {
    defs.into_iter()
        .map(|def| match def {
            TrackSize::Percentage(_) => TrackSize::Auto,
            other => other,
        })
        .collect()
}

/// Build the full list of track definitions (explicit + implicit).
///
/// Implicit tracks cycle through `auto_tracks` per CSS Grid §7.2.4.
pub(crate) fn build_track_definitions(
    explicit: &[TrackSize],
    auto_tracks: &[TrackSize],
    actual_count: usize,
) -> Vec<TrackSize> {
    (0..actual_count)
        .map(|i| {
            explicit.get(i).cloned().unwrap_or_else(|| {
                if auto_tracks.is_empty() {
                    TrackSize::Auto
                } else {
                    let implicit_idx = i.saturating_sub(explicit.len());
                    auto_tracks[implicit_idx % auto_tracks.len()].clone()
                }
            })
        })
        .collect()
}

/// Content distribution mode for track alignment.
#[derive(Clone, Copy)]
pub(crate) enum ContentDistribution {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Stretch,
}

impl From<JustifyContent> for ContentDistribution {
    fn from(jc: JustifyContent) -> Self {
        match jc {
            JustifyContent::FlexStart => Self::Start,
            JustifyContent::FlexEnd => Self::End,
            JustifyContent::Center => Self::Center,
            JustifyContent::SpaceBetween => Self::SpaceBetween,
            JustifyContent::SpaceAround => Self::SpaceAround,
            JustifyContent::SpaceEvenly => Self::SpaceEvenly,
            // CSS Grid §10.5: normal behaves as stretch for grid containers.
            JustifyContent::Stretch | JustifyContent::Normal => Self::Stretch,
        }
    }
}

impl From<AlignContent> for ContentDistribution {
    fn from(ac: AlignContent) -> Self {
        match ac {
            AlignContent::FlexStart => Self::Start,
            AlignContent::FlexEnd => Self::End,
            AlignContent::Center => Self::Center,
            AlignContent::SpaceBetween => Self::SpaceBetween,
            AlignContent::SpaceAround => Self::SpaceAround,
            AlignContent::SpaceEvenly => Self::SpaceEvenly,
            // CSS Grid §10.6: normal behaves as stretch for grid containers.
            AlignContent::Stretch | AlignContent::Normal => Self::Stretch,
        }
    }
}

#[allow(clippy::cast_precision_loss)]
/// Distribute tracks along the container axis (CSS Grid §10.5 / §10.6).
///
/// Adjusts track positions based on `justify-content` / `align-content`.
pub(crate) fn distribute_tracks<D: Into<ContentDistribution>>(
    positions: &mut [f32],
    tracks: &[track::ResolvedTrack],
    gap: f32,
    container_size: f32,
    distribution: D,
    safety: AlignmentSafety,
) {
    if positions.is_empty() || tracks.is_empty() {
        return;
    }
    let used_space = track::total_track_size(tracks, gap);
    let free_space = container_size - used_space;

    let dist = distribution.into();

    // Safety fallback: if free_space < 0 and safe, fall back to start.
    let dist = if safety == AlignmentSafety::Safe && free_space < 0.0 {
        ContentDistribution::Start
    } else {
        dist
    };

    match dist {
        ContentDistribution::Start | ContentDistribution::Stretch => {
            // No adjustment needed. Stretch is handled in track sizing phase.
        }
        ContentDistribution::End => {
            let offset = free_space.max(0.0);
            for pos in positions.iter_mut() {
                *pos += offset;
            }
        }
        ContentDistribution::Center => {
            let offset = (free_space / 2.0).max(0.0);
            for pos in positions.iter_mut() {
                *pos += offset;
            }
        }
        ContentDistribution::SpaceBetween => {
            if tracks.len() <= 1 || free_space <= 0.0 {
                return;
            }
            let extra_gap = free_space / (tracks.len() - 1) as f32;
            for (i, pos) in positions.iter_mut().enumerate() {
                *pos += extra_gap * i as f32;
            }
        }
        ContentDistribution::SpaceAround => {
            if free_space <= 0.0 || tracks.is_empty() {
                return;
            }
            let per_track = free_space / tracks.len() as f32;
            let half = per_track / 2.0;
            for (i, pos) in positions.iter_mut().enumerate() {
                *pos += half + per_track * i as f32;
            }
        }
        ContentDistribution::SpaceEvenly => {
            if free_space <= 0.0 || tracks.is_empty() {
                return;
            }
            let slot = free_space / (tracks.len() + 1) as f32;
            for (i, pos) in positions.iter_mut().enumerate() {
                *pos += slot * (i + 1) as f32;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Grid abs-pos containing block (CSS Grid §11)
// ---------------------------------------------------------------------------

/// Resolve the containing block for an absolutely positioned grid child.
///
/// CSS Grid §11: If the child specifies grid lines, the grid area is the CB.
/// If all placement is auto, the container padding-box is used.
/// If one axis is auto and the other is placed, the auto axis uses the
/// full extent of the container padding-box.
pub(crate) fn resolve_grid_abspos_cb(
    dom: &EcsDom,
    child: Entity,
    col: &GridAxisInfo<'_>,
    row: &GridAxisInfo<'_>,
    content_origin: Point,
    padding_box: &Rect,
) -> Rect {
    let style = elidex_layout_block::try_get_style(dom, child).unwrap_or_default();

    let col_range = resolve_abspos_axis(
        &style.grid_column_start,
        &style.grid_column_end,
        col.explicit_count,
        col.name_map,
    );
    let row_range = resolve_abspos_axis(
        &style.grid_row_start,
        &style.grid_row_end,
        row.explicit_count,
        row.name_map,
    );

    let (x, w) = match col_range {
        Some((start, end)) => track_range_rect(
            col.positions,
            col.tracks,
            col.gap,
            start,
            end,
            content_origin.x,
        ),
        None => (padding_box.origin.x, padding_box.size.width),
    };
    let (y, h) = match row_range {
        Some((start, end)) => track_range_rect(
            row.positions,
            row.tracks,
            row.gap,
            start,
            end,
            content_origin.y,
        ),
        None => (padding_box.origin.y, padding_box.size.height),
    };

    Rect::new(x, y, w, h)
}

/// Resolve an abs-pos grid line pair to a (`start_index`, `end_index`) range.
///
/// Returns `None` if both start and end are `Auto`.
fn resolve_abspos_axis(
    start: &elidex_plugin::GridLine,
    end: &elidex_plugin::GridLine,
    explicit_count: usize,
    name_map: &placement::LineNameMap,
) -> Option<(usize, usize)> {
    let s = placement::resolve_line(start, explicit_count, name_map, false);
    let e = placement::resolve_line(end, explicit_count, name_map, true);

    match (s, e) {
        (None, None) => None,
        (Some(si), Some(ei)) => match ei.cmp(&si) {
            std::cmp::Ordering::Greater => Some((si, ei)),
            std::cmp::Ordering::Less => Some((ei, si)),
            std::cmp::Ordering::Equal => Some((si, si + 1)),
        },
        (Some(si), None) => Some((si, si + 1)),
        (None, Some(ei)) => Some((ei.saturating_sub(1), ei)),
    }
}

/// Compute the position and size for a range of tracks.
#[allow(clippy::cast_precision_loss)] // Track counts are small.
fn track_range_rect(
    positions: &[f32],
    tracks: &[track::ResolvedTrack],
    gap: f32,
    start: usize,
    end: usize,
    content_offset: f32,
) -> (f32, f32) {
    let start = start.min(tracks.len());
    let end = end.min(tracks.len());
    if start >= end || positions.is_empty() {
        return (content_offset, 0.0);
    }
    let x = positions.get(start).copied().unwrap_or(0.0) + content_offset;
    // Width = sum of tracks + gaps between them
    let mut w = 0.0;
    for i in start..end {
        if let Some(t) = tracks.get(i) {
            w += t.size;
        }
    }
    if end > start + 1 {
        let mut actual_gaps = 0;
        for i in start..(end - 1) {
            if tracks.get(i).is_some_and(|t| !t.collapsed)
                && tracks.get(i + 1).is_some_and(|t| !t.collapsed)
            {
                actual_gaps += 1;
            }
        }
        w += gap * actual_gaps as f32;
    }
    (x, w)
}

/// `justify-self: auto` resolves to the container's `justify-items`.
pub(crate) fn effective_justify(
    justify_self: JustifySelf,
    container_justify_items: JustifyItems,
) -> JustifyItems {
    match justify_self {
        JustifySelf::Auto => container_justify_items,
        JustifySelf::Start => JustifyItems::Start,
        JustifySelf::End => JustifyItems::End,
        JustifySelf::Center => JustifyItems::Center,
        JustifySelf::Stretch => JustifyItems::Stretch,
        JustifySelf::Baseline => JustifyItems::Baseline,
    }
}

// ---------------------------------------------------------------------------
// Grid intrinsic sizing (placement-based)
// ---------------------------------------------------------------------------

/// Compute intrinsic sizes for a grid container using actual placement.
///
/// Runs the full placement pipeline (item collection → expand tracks → place →
/// measure → track sizing) and returns per-column min/max content sizes.
/// This replaces the round-robin approximation in `elidex-layout/intrinsic.rs`.
pub fn compute_grid_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    env: &LayoutEnv<'_>,
) -> elidex_layout_block::IntrinsicSizes {
    let style = elidex_layout_block::get_style(dom, entity);

    // Expand track lists (use 0.0 as available space for intrinsic sizing).
    let col_section = style.grid_template_columns.expand_with_names(0.0, 0.0);
    let row_section = style.grid_template_rows.expand_with_names(0.0, 0.0);
    let explicit_cols = col_section.tracks.len();
    let explicit_rows = row_section.tracks.len();

    // Build line name maps
    let col_name_map =
        placement::build_line_name_map(&col_section.line_names, &style.grid_template_areas, true);
    let row_name_map =
        placement::build_line_name_map(&row_section.line_names, &style.grid_template_areas, false);

    // Collect and place items
    let mut items = collect_grid_items(dom, children, &style);
    if items.is_empty() {
        return elidex_layout_block::IntrinsicSizes::default();
    }
    items.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then(a.source_order.cmp(&b.source_order))
    });

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

    // Measure content sizes (min-content + max-content probes)
    let probe = LayoutInput::probe(env, 0.0);
    measure_item_content(dom, &mut items, 0.0, &probe, env.layout_child);

    // Build column contributions from actual placement
    let col_contribs = build_contributions(&items, true);

    // Determine actual column count
    let actual_cols = items
        .iter()
        .map(|item| item.col_start + item.col_span)
        .max()
        .unwrap_or(explicit_cols)
        .max(explicit_cols)
        .max(1);

    // Resolve column tracks using the track sizing algorithm.
    // CSS Grid §7.2.1: in intrinsic sizing (available = 0), percentage tracks
    // are treated as auto so they participate via content contributions.
    let col_defs =
        build_track_definitions(&col_section.tracks, &style.grid_auto_columns, actual_cols);
    let col_defs = percentage_tracks_to_auto(col_defs);
    let col_tracks = track::resolve_tracks(&col_defs, 0.0, 0.0, &col_contribs, false);

    let gap = elidex_layout_block::resolve_dimension_value(style.column_gap, 0.0, 0.0).max(0.0);
    let gap_total = if actual_cols > 1 {
        elidex_layout_block::total_gap(actual_cols, gap)
    } else {
        0.0
    };

    // min-content: sum of base sizes (min-content contributions)
    let min: f32 = col_tracks.iter().map(|t| t.base).sum();
    // max-content: sum of limit sizes (max-content contributions)
    let max: f32 = col_tracks.iter().map(|t| t.limit.max(t.base)).sum();

    elidex_layout_block::IntrinsicSizes {
        min_content: min + gap_total,
        max_content: max + gap_total,
    }
}
