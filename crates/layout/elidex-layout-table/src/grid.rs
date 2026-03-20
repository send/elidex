//! Cell grid construction and occupancy tracking.
//!
//! Builds the (row, col) grid of table cells from the DOM, handling
//! colspan/rowspan, anonymous row wrapping, and WHATWG dimension limits.

use elidex_ecs::AnonymousTableMarker;
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::{ComputedStyle, Display};

// ---------------------------------------------------------------------------
// Table dimension limits (WHATWG §4.9.11)
// ---------------------------------------------------------------------------

/// Maximum number of columns in a table (WHATWG colspan max).
pub(crate) const MAX_TABLE_COLS: usize = 1000;

/// Maximum number of rows in a table (WHATWG rowspan max).
pub(crate) const MAX_TABLE_ROWS: usize = 65534;

/// Maximum cells in the occupancy grid (~4 MB budget for `Vec<bool>`).
/// Tables exceeding this threshold are row-truncated.
pub(crate) const MAX_OCCUPANCY_CELLS: usize = 4_000_000;

// ---------------------------------------------------------------------------
// CellInfo
// ---------------------------------------------------------------------------

/// Metadata for a single table cell.
#[derive(Clone)]
pub(crate) struct CellInfo {
    pub(crate) entity: Entity,
    pub(crate) col: usize,
    pub(crate) row: usize,
    pub(crate) colspan: u32,
    pub(crate) rowspan: u32,
}

// ---------------------------------------------------------------------------
// Span helpers
// ---------------------------------------------------------------------------

/// End row (exclusive) of a cell's row span, clamped to table bounds.
#[inline]
#[must_use]
pub(crate) fn span_end_row(cell: &CellInfo, num_rows: usize) -> usize {
    cell.row.saturating_add(cell.rowspan as usize).min(num_rows)
}

/// End column (exclusive) of a cell's column span, clamped to table bounds.
#[inline]
#[must_use]
pub(crate) fn span_end_col(cell: &CellInfo, num_cols: usize) -> usize {
    cell.col.saturating_add(cell.colspan as usize).min(num_cols)
}

// ---------------------------------------------------------------------------
// Row group sorting
// ---------------------------------------------------------------------------

/// Sort row groups into header/body/footer order (CSS 2.1 §17.2.1).
///
/// Returns `(headers, bodies, footers)`.
pub(crate) fn sort_row_groups(
    dom: &EcsDom,
    row_groups: &[Entity],
) -> (Vec<Entity>, Vec<Entity>, Vec<Entity>) {
    let mut headers = Vec::new();
    let mut bodies = Vec::new();
    let mut footers = Vec::new();
    for &rg in row_groups {
        let rg_style = elidex_layout_block::get_style(dom, rg);
        match rg_style.display {
            Display::TableHeaderGroup => headers.push(rg),
            Display::TableFooterGroup => footers.push(rg),
            _ => bodies.push(rg),
        }
    }
    (headers, bodies, footers)
}

// ---------------------------------------------------------------------------
// Style inheritance for anonymous table entities (CSS 2.1 §17.2.1)
// ---------------------------------------------------------------------------

/// Create a `ComputedStyle` for an anonymous table entity by inheriting
/// inheritable properties from the parent (CSS 2.1 §17.2.1).
///
/// Non-inherited properties use initial values. This is called when creating
/// or reusing anonymous rows, cells, and table wrappers.
#[must_use]
pub fn inherit_for_anonymous(parent_style: &ComputedStyle, display: Display) -> ComputedStyle {
    ComputedStyle {
        display,
        // Inherited text properties
        color: parent_style.color,
        font_size: parent_style.font_size,
        font_weight: parent_style.font_weight,
        font_style: parent_style.font_style,
        font_family: parent_style.font_family.clone(),
        line_height: parent_style.line_height,
        text_transform: parent_style.text_transform,
        text_align: parent_style.text_align,
        white_space: parent_style.white_space,
        list_style_type: parent_style.list_style_type,
        writing_mode: parent_style.writing_mode,
        text_orientation: parent_style.text_orientation,
        direction: parent_style.direction,
        visibility: parent_style.visibility,
        letter_spacing: parent_style.letter_spacing,
        word_spacing: parent_style.word_spacing,
        // Inherited table properties
        empty_cells: parent_style.empty_cells,
        border_collapse: parent_style.border_collapse,
        border_spacing_h: parent_style.border_spacing_h,
        border_spacing_v: parent_style.border_spacing_v,
        caption_side: parent_style.caption_side,
        // Inherited fragmentation properties
        orphans: parent_style.orphans,
        widows: parent_style.widows,
        // Inherited custom properties
        custom_properties: parent_style.custom_properties.clone(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Row collection
// ---------------------------------------------------------------------------

/// Collect all rows from row groups and direct rows, with row-group boundary info.
///
/// Row groups are ordered: thead first, then tbody, then tfoot.
/// Direct rows follow after all row-group rows.
///
/// Non-row children within a row group are wrapped in anonymous rows
/// (CSS 2.1 §17.2.1 Stage 2 Rule 2).
///
/// Each returned `RowInfo` records which row-group the row belongs to
/// (via `group_end_row`), needed for rowspan=0 resolution.
pub(crate) fn collect_all_rows(
    dom: &mut EcsDom,
    row_groups: &[Entity],
    direct_rows: &[Entity],
) -> Vec<RowInfo> {
    let mut rows: Vec<RowInfo> = Vec::new();

    let (headers, bodies, footers) = sort_row_groups(dom, row_groups);

    // Headers first, then bodies, then footers.
    for groups in [&headers, &bodies, &footers] {
        for &rg in groups {
            // CSS 2.1 §17.2.1 Stage 2 Rule 2: wrap non-row children in anonymous rows.
            wrap_non_matching_children(dom, rg, Display::TableRow, Display::TableRow, "tr");

            // Re-read children after potential wrapping.
            let rg_children = dom.composed_children(rg);
            let group_start = rows.len();
            for &child in &rg_children {
                let child_style = elidex_layout_block::get_style(dom, child);
                if child_style.display == Display::TableRow {
                    // group_end_row is a placeholder; we fix it after collecting all rows in this group.
                    rows.push(RowInfo {
                        entity: child,
                        group_end_row: 0,
                    });
                }
            }
            // Set group_end_row for all rows in this group.
            let group_end = rows.len();
            for row_info in &mut rows[group_start..group_end] {
                row_info.group_end_row = group_end;
            }
        }
    }

    // Add direct rows (including any anonymous row wrapping direct cells).
    // Direct rows are not in a row group, so group_end_row spans all remaining rows.
    let direct_start = rows.len();
    for &r in direct_rows {
        rows.push(RowInfo {
            entity: r,
            group_end_row: 0, // placeholder
        });
    }
    // Direct rows: group_end_row = total row count (set after all rows collected).
    let total = rows.len();
    for row_info in &mut rows[direct_start..total] {
        row_info.group_end_row = total;
    }

    rows
}

/// Wrap consecutive children of `parent` that don't match `keep_display` in
/// anonymous entities with `wrap_display`.
///
/// Uses pool-based reuse for idempotent re-layout. Children with `Display::None`
/// are kept unwrapped (they don't participate in table layout).
///
/// This is the shared implementation for CSS 2.1 §17.2.1 Stage 2 Rules 2 and 3:
/// - Rule 2: non-row children in a row group → anonymous table-row
/// - Rule 3: non-cell children in a row → anonymous table-cell
pub(crate) fn wrap_non_matching_children(
    dom: &mut EcsDom,
    parent: Entity,
    keep_display: Display,
    wrap_display: Display,
    wrap_tag: &str,
) {
    let children: Vec<Entity> = dom.composed_children(parent);
    let mut needs_wrap = false;
    for &child in &children {
        let cs = elidex_layout_block::get_style(dom, child);
        if cs.display != keep_display && cs.display != Display::None {
            needs_wrap = true;
            break;
        }
    }
    if !needs_wrap {
        return;
    }

    let mut pool = collect_anonymous_pool(dom, &children, wrap_display);

    let children: Vec<Entity> = dom.composed_children(parent);
    let mut run: Vec<Entity> = Vec::new();
    for &child in &children {
        let cs = elidex_layout_block::get_style(dom, child);
        if cs.display == keep_display || cs.display == Display::None {
            if !run.is_empty() {
                let anon = take_or_create_anonymous(dom, parent, wrap_display, wrap_tag, &mut pool);
                for &c in &run {
                    let _ = dom.append_child(anon, c);
                }
                run.clear();
            }
        } else {
            run.push(child);
        }
    }
    if !run.is_empty() {
        let anon = take_or_create_anonymous(dom, parent, wrap_display, wrap_tag, &mut pool);
        for &c in &run {
            let _ = dom.append_child(anon, c);
        }
    }
}

// ---------------------------------------------------------------------------
// Cell grid construction
// ---------------------------------------------------------------------------

/// Build the cell grid from collected rows.
///
/// Returns (cells, `num_cols`, `num_rows`).
///
/// Rows beyond [`MAX_TABLE_ROWS`] are silently truncated; column positions
/// beyond [`MAX_TABLE_COLS`] cause remaining cells in that row to be skipped.
///
/// `rowspan=0` is resolved to span the remaining rows in the cell's row group
/// (WHATWG §4.9.11), using the `group_end_row` from each `RowInfo`.
#[must_use]
pub(crate) fn build_cell_grid(dom: &EcsDom, rows: &[RowInfo]) -> (Vec<CellInfo>, usize, usize) {
    let mut cells = Vec::new();
    let mut num_rows = rows.len().min(MAX_TABLE_ROWS);
    // Budget check: limit rows so occupancy grid stays within memory budget.
    // Worst case each row could have MAX_TABLE_COLS columns of bool.
    if num_rows.saturating_mul(MAX_TABLE_COLS) > MAX_OCCUPANCY_CELLS {
        num_rows = MAX_OCCUPANCY_CELLS / MAX_TABLE_COLS;
    }
    if num_rows == 0 {
        return (cells, 0, 0);
    }

    // Occupancy grid: tracks which (row, col) positions are occupied by spanning cells.
    // Rows are pre-allocated; columns expand via `resize()`.
    let mut occupancy: Vec<Vec<bool>> = vec![Vec::new(); num_rows];
    let mut max_col = 0_usize;

    for (r, row_info) in rows.iter().enumerate().take(num_rows) {
        let row_children = dom.composed_children(row_info.entity);
        let mut col = 0_usize;

        for &child in &row_children {
            let child_style = elidex_layout_block::get_style(dom, child);
            if child_style.display == Display::None {
                continue;
            }
            // Absolutely positioned table children are removed from table layout.
            if elidex_layout_block::positioned::is_absolutely_positioned(&child_style) {
                continue;
            }

            // Read colspan/rowspan from HTML attributes.
            let (colspan, rowspan) = read_span_attributes(dom, child);

            // Find next unoccupied column.
            while col < occupancy[r].len() && occupancy[r][col] {
                col += 1;
            }

            // Cap column position to avoid unbounded growth.
            if col >= MAX_TABLE_COLS {
                break;
            }

            // Clamp colspan so it doesn't push past the column limit.
            let max_span = MAX_TABLE_COLS - col; // safe: col < MAX_TABLE_COLS
            let clamped_colspan = (colspan as usize).min(max_span);
            let span_end_col = col + clamped_colspan; // safe: <= MAX_TABLE_COLS

            // Resolve rowspan: 0 means "span remaining rows in the row group"
            // (WHATWG §4.9.11), not the entire table.
            let resolved_rowspan = if rowspan == 0 {
                (row_info.group_end_row.min(num_rows) - r).max(1)
            } else {
                (rowspan as usize).min(num_rows - r)
            };
            let span_end_row = r + resolved_rowspan; // safe: <= num_rows

            // Mark occupancy for this cell's span.
            for occ_row in &mut occupancy[r..span_end_row] {
                if occ_row.len() < span_end_col {
                    occ_row.resize(span_end_col, false);
                }
                for slot in &mut occ_row[col..span_end_col] {
                    *slot = true;
                }
            }

            #[allow(clippy::cast_possible_truncation)] // clamped values fit in u32
            cells.push(CellInfo {
                entity: child,
                col,
                row: r,
                colspan: clamped_colspan as u32,
                rowspan: resolved_rowspan as u32,
            });

            max_col = max_col.max(span_end_col);
            col = span_end_col;
        }
    }

    (cells, max_col.min(MAX_TABLE_COLS), num_rows)
}

/// Read colspan and rowspan attributes from an entity.
///
/// Per WHATWG §4.9.11: colspan is clamped to `[1, 1000]`, rowspan to `[0, 65534]`.
/// `rowspan=0` is preserved as 0 (sentinel for "span remaining rows in group"),
/// resolved by `build_cell_grid()`.
#[must_use]
fn read_span_attributes(dom: &EcsDom, entity: Entity) -> (u32, u32) {
    const MAX_COLSPAN: u32 = 1000;
    const MAX_ROWSPAN: u32 = 65534;

    let (mut colspan, mut rowspan) = (1u32, 1u32);
    if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
        if let Some(val) = attrs.get("colspan") {
            if let Ok(n) = val.parse::<u32>() {
                if n >= 1 {
                    colspan = n.min(MAX_COLSPAN);
                }
            }
        }
        if let Some(val) = attrs.get("rowspan") {
            if let Ok(n) = val.parse::<u32>() {
                // rowspan=0 means "span remaining rows in the row group".
                // We keep 0 as a sentinel; build_cell_grid() resolves it.
                rowspan = n.min(MAX_ROWSPAN);
            }
        }
    }
    (colspan, rowspan)
}

// ---------------------------------------------------------------------------
// Anonymous table object helpers (CSS 2.1 §17.2.1)
// ---------------------------------------------------------------------------

/// Find an existing anonymous table entity with `display` under `parent`, or create one.
///
/// If a child with `AnonymousTableMarker` and matching display is found, its children
/// are detached (for re-population) and the entity is returned.  Otherwise a new entity
/// is created with the marker, inherited style, and appended to `parent`.
///
/// **Note:** This function always returns the *first* matching anonymous entity.
/// For wrapping multiple separate runs under the same parent, use
/// [`collect_anonymous_pool`] + [`take_or_create_anonymous`] instead to avoid
/// reusing the same entity for different runs.
pub(crate) fn find_or_create_anonymous(
    dom: &mut EcsDom,
    parent: Entity,
    display: Display,
    tag: &str,
) -> Entity {
    // Search for an existing anonymous entity with the right display.
    let children: Vec<Entity> = dom.composed_children(parent);
    let parent_style = elidex_layout_block::get_style(dom, parent);
    let style = inherit_for_anonymous(&parent_style, display);

    for &child in &children {
        let has_marker = dom.world().get::<&AnonymousTableMarker>(child).is_ok();
        if has_marker {
            let child_display = dom
                .world()
                .get::<&ComputedStyle>(child)
                .map(|s| s.display)
                .ok();
            if child_display == Some(display) {
                // Reuse: detach all children so the caller can re-populate.
                detach_all_children(dom, child);
                // Update style in case parent style changed since last layout.
                let _ = dom.world_mut().insert_one(child, style);
                return child;
            }
        }
    }

    // Create a new anonymous entity.
    let anon = dom.create_element(tag, Attributes::default());
    let _ = dom.world_mut().insert(anon, (AnonymousTableMarker, style));
    let _ = dom.append_child(parent, anon);
    anon
}

/// Collect all anonymous entities with the given `display` from a child list.
///
/// Used to build a reuse pool before wrapping multiple runs. Each entity in the
/// returned vec can be popped and re-populated via [`take_or_create_anonymous`].
///
/// Callers supply their own child list (e.g. `composed_children` or
/// `composed_children_flat`) so the same logic works across crate boundaries.
pub fn collect_anonymous_pool(dom: &EcsDom, children: &[Entity], display: Display) -> Vec<Entity> {
    children
        .iter()
        .copied()
        .filter(|&child| {
            dom.world().get::<&AnonymousTableMarker>(child).is_ok()
                && dom
                    .world()
                    .get::<&ComputedStyle>(child)
                    .map(|s| s.display == display)
                    .unwrap_or(false)
        })
        .collect()
}

/// Take the next entity from the reuse pool, or create a new anonymous entity.
///
/// When reusing, detaches existing children and updates the inherited style.
/// When creating, appends the new entity to `parent`.
pub(crate) fn take_or_create_anonymous(
    dom: &mut EcsDom,
    parent: Entity,
    display: Display,
    tag: &str,
    pool: &mut Vec<Entity>,
) -> Entity {
    let parent_style = elidex_layout_block::get_style(dom, parent);
    let style = inherit_for_anonymous(&parent_style, display);
    if let Some(entity) = pool.pop() {
        detach_all_children(dom, entity);
        let _ = dom.world_mut().insert_one(entity, style);
        entity
    } else {
        let anon = dom.create_element(tag, Attributes::default());
        let _ = dom.world_mut().insert(anon, (AnonymousTableMarker, style));
        let _ = dom.append_child(parent, anon);
        anon
    }
}

/// Detach all children of `entity` from the tree.
fn detach_all_children(dom: &mut EcsDom, entity: Entity) {
    let children: Vec<Entity> = dom.composed_children(entity);
    for &child in &children {
        let _ = dom.remove_child(entity, child);
    }
}

// ---------------------------------------------------------------------------
// Row info for rowspan=0 resolution
// ---------------------------------------------------------------------------

/// Row metadata including row-group boundary information.
pub(crate) struct RowInfo {
    pub(crate) entity: Entity,
    /// The exclusive end row index of the row group this row belongs to.
    /// For direct rows (not in a row group), this is the total row count.
    pub(crate) group_end_row: usize,
}

// ---------------------------------------------------------------------------
// Cell width calculation
// ---------------------------------------------------------------------------

/// Calculate the available width for a cell (sum of spanned columns + inter-column spacing).
#[must_use]
#[allow(clippy::cast_precision_loss)] // span counts are small integers
pub(crate) fn cell_available_width(
    col_widths: &[f32],
    cell: &CellInfo,
    spacing_h: f32,
    num_cols: usize,
) -> f32 {
    let w: f32 = col_widths[cell.col..span_end_col(cell, num_cols)]
        .iter()
        .sum();
    // Add spacing between spanned columns.
    let extra_spacing = spacing_h * (cell.colspan as f32 - 1.0).max(0.0);
    w + extra_spacing
}
