//! CSS Grid layout algorithm (CSS Grid Level 1, simplified).
//!
//! Implements the core grid algorithm: track sizing, item placement,
//! and cell positioning.
//!
// Current simplifications:
// - No named grid lines (numeric only)
// - No `grid-template-areas`
// - No subgrid
// - `inline-grid` treated as block-level

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

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    effective_align, horizontal_pb, resolve_explicit_height, sanitize_border, ChildLayoutFn,
    EmptyContainerParams, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    AlignContent, AlignItems, AlignmentSafety, ComputedStyle, Dimension, Display, EdgeSizes,
    GridAutoFlow, GridLine, GridTrackList, JustifyContent, JustifyItems, JustifySelf, LayoutBox,
    Rect, TrackSize,
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
    let expanded_cols = col_track_list.expand(content_width, gap_col);
    let available_height_for_rows = resolve_explicit_height(&style, containing_height);
    let expanded_rows = row_track_list.expand(available_height_for_rows.unwrap_or(0.0), gap_row);
    let explicit_cols = expanded_cols.len();
    let explicit_rows = expanded_rows.len();

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

    // --- Build per-item intrinsic size contributions ---
    let col_contribs = build_contributions(&items, true);
    let row_contribs = build_contributions(&items, false);

    // --- 9. Resolve column tracks ---
    let col_defs = build_track_definitions(&expanded_cols, &style.grid_auto_columns, actual_cols);
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
    let row_defs = build_track_definitions(&expanded_rows, &style.grid_auto_rows, actual_rows);
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
    // CSS Grid §5.2: the grid container establishes a CB for absolute children
    // when it is itself positioned (or is the root).
    // CSS Transforms L1 §2: transform establishes CB for all descendants.
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != elidex_plugin::Position::Static || is_root || style.has_transform;
    if is_cb {
        let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
            dom, &children, content_x, content_y,
        );
        let pb = lb.padding_box();
        elidex_layout_block::positioned::layout_positioned_children(
            dom,
            entity,
            &pb,
            input.viewport,
            &static_positions,
            font_db,
            layout_child,
            depth,
        );
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

// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

/// Collect grid items from children, skipping `display:none` and text nodes.
fn collect_grid_items(
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
            justify,
            height_auto: child_style.height == Dimension::Auto,
            width_auto: child_style.width == Dimension::Auto,
            content_width: 0.0,
            content_height: 0.0,
            min_content_width: 0.0,
            min_content_height: 0.0,
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
        let padding = elidex_layout_block::resolve_padding(&child_style, container_width);
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

        // Min-content probe: layout at near-zero width (CSS Grid §12.3).
        // Save descendant styles first — layout probes can mutate styles
        // (e.g. flex's relayout_item_at_position overwrites child widths).
        let saved_styles = save_descendant_styles(dom, item.entity);
        let min_input = LayoutInput {
            containing_width: 1.0,
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
        let min_lb = layout_child(dom, item.entity, &min_input).layout_box;
        item.min_content_width = min_lb.content.width + item.pb.left + item.pb.right;
        item.min_content_height = min_lb.content.height + item.pb.top + item.pb.bottom;
        // Restore styles corrupted by the min-content probe.
        restore_descendant_styles(dom, &saved_styles);

        // Max-content probe: layout at container width.
        let max_input = LayoutInput {
            containing_width: container_width,
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
        let max_lb = layout_child(dom, item.entity, &max_input).layout_box;
        item.content_width = max_lb.content.width + item.pb.left + item.pb.right;
        item.content_height = max_lb.content.height + item.pb.top + item.pb.bottom;
    }
}

/// Save `ComputedStyle` for all descendants of `entity` (excluding `entity` itself).
///
/// Layout probes (e.g. min-content at `containing_width: 1.0`) can mutate
/// descendant styles via flex/grid `position_items`. This function captures
/// the styles so they can be restored after the probe.
fn save_descendant_styles(dom: &EcsDom, entity: Entity) -> Vec<(Entity, ComputedStyle)> {
    let mut result = Vec::new();
    let mut stack = Vec::new();
    // Push direct children.
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        stack.push(c);
        child = dom.get_next_sibling(c);
    }
    while let Some(e) = stack.pop() {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(e) {
            result.push((e, (*style).clone()));
        }
        // Push children of e.
        let mut c = dom.get_first_child(e);
        while let Some(ch) = c {
            stack.push(ch);
            c = dom.get_next_sibling(ch);
        }
    }
    result
}

/// Restore previously saved `ComputedStyle` components.
fn restore_descendant_styles(dom: &mut EcsDom, saved: &[(Entity, ComputedStyle)]) {
    for (entity, style) in saved {
        let _ = dom.world_mut().insert_one(*entity, style.clone());
    }
}

/// Compute intrinsic content sizes per track.
///
/// Returns `(min_content_sizes, max_content_sizes)` per track.
/// For items spanning a single track, contribute their full size.
/// For multi-span items, distribute proportionally.
#[allow(clippy::cast_precision_loss)]
/// Build per-item track contributions for the track sizing algorithm.
fn build_contributions(items: &[GridItem], is_column: bool) -> Vec<track::TrackContribution> {
    items
        .iter()
        .map(|item| {
            if is_column {
                track::TrackContribution {
                    start: item.col_start,
                    span: item.col_span,
                    min_content: item.min_content_width + item.margin.left + item.margin.right,
                    max_content: item.content_width + item.margin.left + item.margin.right,
                }
            } else {
                track::TrackContribution {
                    start: item.row_start,
                    span: item.row_span,
                    min_content: item.min_content_height + item.margin.top + item.margin.bottom,
                    max_content: item.content_height + item.margin.top + item.margin.bottom,
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
fn percentage_tracks_to_auto(defs: Vec<TrackSize>) -> Vec<TrackSize> {
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
fn build_track_definitions(
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
enum ContentDistribution {
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
fn distribute_tracks<D: Into<ContentDistribution>>(
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
            if free_space <= 0.0 {
                return;
            }
            let per_track = free_space / tracks.len() as f32;
            let half = per_track / 2.0;
            for (i, pos) in positions.iter_mut().enumerate() {
                *pos += half + per_track * i as f32;
            }
        }
        ContentDistribution::SpaceEvenly => {
            if free_space <= 0.0 {
                return;
            }
            let slot = free_space / (tracks.len() + 1) as f32;
            for (i, pos) in positions.iter_mut().enumerate() {
                *pos += slot * (i + 1) as f32;
            }
        }
    }
}

/// Resolve effective justify for a grid item.
///
/// `justify-self: auto` resolves to the container's `justify-items`.
fn effective_justify(
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
