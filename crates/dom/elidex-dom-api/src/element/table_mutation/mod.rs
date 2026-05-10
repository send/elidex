//! HTML §4.9.1 table-family DOM mutation algorithms (slot
//! `#11-tags-T2c-table`).
//!
//! Engine-independent home for `<table>.insertRow` /
//! `<table>.deleteRow` / `<table>.createTHead` / etc. and the
//! corresponding row/cell index walkers.  VM `host/` consumers
//! (T2c per-tag prototype files) are pure marshalling shims.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate": all DOM mutation algorithms in
//! the engine-independent crate.  The actual `EcsDom::*` calls live
//! here; per-tag prototype files in `vm/host/html_table_*_proto.rs`
//! call these helpers either directly (for the row/cell index
//! getters) or via `invoke_dom_api` registry handlers (for the
//! mutation methods).

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::not_found_error;

/// Section discriminator for the shared
/// [`find_table_section_insert_position`] helper.  Only `<thead>` /
/// `<tfoot>` / `<caption>` need explicit positions; `<tbody>` always
/// inserts after the last existing `<tbody>` / before the closing of
/// the table, which is computed inline by [`create_tbody`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    /// `<caption>` — first child of the table.
    Caption,
    /// `<thead>` — after caption / colgroup, before all other children.
    THead,
    /// `<tfoot>` — at the end of the table (HTML5 modern position).
    TFoot,
}

// ---------------------------------------------------------------------------
// ASCII-CI tag matching helpers
// ---------------------------------------------------------------------------

fn tag_eq_ci(dom: &EcsDom, entity: Entity, tag: &str) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .is_ok_and(|tt| tt.0.eq_ignore_ascii_case(tag))
}

fn tag_is_any_of(dom: &EcsDom, entity: Entity, tags: &[&str]) -> bool {
    dom.world().get::<&TagType>(entity).is_ok_and(|tt| {
        let s = tt.0.as_str();
        tags.iter().any(|t| s.eq_ignore_ascii_case(t))
    })
}

// ---------------------------------------------------------------------------
// Section position-finder (shared by `create_*` and `set_*`)
// ---------------------------------------------------------------------------

/// Compute the insertion position for a new section element of `kind`
/// inside `table` per HTML §4.9.1.  Returns `Some(ref_child)` to
/// `insert_before(table, new, ref_child)`, or `None` to append.
///
/// Position rules:
/// - `Caption` — insert before the first child of `table` (or append
///   if empty).  Caller MUST have removed any existing `<caption>`
///   first; otherwise the new caption lands before the existing one.
/// - `THead` — insert after the existing `<caption>` / `<colgroup>`
///   children (in tree order), before all other children.
/// - `TFoot` — append at the end of the table (HTML5 modern position).
#[must_use]
pub fn find_table_section_insert_position(
    table: Entity,
    dom: &EcsDom,
    kind: SectionKind,
) -> Option<Entity> {
    match kind {
        SectionKind::Caption => dom.children_iter(table).next(),
        SectionKind::THead => {
            // Skip leading <caption> / <colgroup> children; insert
            // before the first child that is neither.
            for child in dom.children_iter(table) {
                if !tag_is_any_of(dom, child, &["caption", "colgroup"]) {
                    return Some(child);
                }
            }
            None
        }
        SectionKind::TFoot => None, // append
    }
}

// ---------------------------------------------------------------------------
// `<table>` section accessors: createTHead / createTFoot / createCaption /
// createTBody / delete{THead,TFoot,Caption}
// ---------------------------------------------------------------------------

/// Find the first direct child of `table` whose tag (ASCII-CI)
/// equals `tag`.  Public so VM-side `<table>.{caption,tHead,tFoot}`
/// getters can reuse the same walk algorithm as `create_*` and
/// `delete_*` (avoids the inlined-walk drift the review flagged).
#[must_use]
pub fn first_section_child(table: Entity, dom: &EcsDom, tag: &str) -> Option<Entity> {
    dom.children_iter(table).find(|c| tag_eq_ci(dom, *c, tag))
}

/// `<table>.createTHead()` — return existing `<thead>` direct child
/// if any, else create + insert at the spec position.  Always returns
/// the (possibly newly-created) `<thead>` Entity.
pub fn create_thead(table: Entity, dom: &mut EcsDom) -> Entity {
    if let Some(existing) = first_section_child(table, dom, "thead") {
        return existing;
    }
    let new = dom.create_element("thead", Attributes::default());
    insert_section_at_position(table, new, dom, SectionKind::THead);
    new
}

/// `<table>.createTFoot()` — return existing `<tfoot>` direct child
/// if any, else create + append at the end of the table.
pub fn create_tfoot(table: Entity, dom: &mut EcsDom) -> Entity {
    if let Some(existing) = first_section_child(table, dom, "tfoot") {
        return existing;
    }
    let new = dom.create_element("tfoot", Attributes::default());
    insert_section_at_position(table, new, dom, SectionKind::TFoot);
    new
}

/// `<table>.createCaption()` — return existing `<caption>` direct
/// child if any, else create + insert as the table's first child.
pub fn create_caption(table: Entity, dom: &mut EcsDom) -> Entity {
    if let Some(existing) = first_section_child(table, dom, "caption") {
        return existing;
    }
    let new = dom.create_element("caption", Attributes::default());
    insert_section_at_position(table, new, dom, SectionKind::Caption);
    new
}

/// `<table>.createTBody()` (HTML §4.9.1) — **always creates a new
/// `<tbody>`** (NOT idempotent unlike `createTHead`).  Inserts
/// immediately after the last existing `<tbody>` direct child if
/// any, else at the end of the table.
pub fn create_tbody(table: Entity, dom: &mut EcsDom) -> Entity {
    let new = dom.create_element("tbody", Attributes::default());
    // Find the last <tbody> direct child; insert after it (i.e.
    // before the next sibling, or append if it's the last child).
    let last_tbody = dom
        .children_iter(table)
        .filter(|c| tag_eq_ci(dom, *c, "tbody"))
        .last();
    if let Some(last) = last_tbody {
        if let Some(after) = dom.next_exposed_sibling(last) {
            let _ = dom.insert_before(table, new, after);
            return new;
        }
    }
    let _ = dom.append_child(table, new);
    new
}

/// `<table>.deleteTHead()` — remove the first `<thead>` direct child.
/// No-op when absent.
pub fn delete_thead(table: Entity, dom: &mut EcsDom) {
    if let Some(existing) = first_section_child(table, dom, "thead") {
        let _ = dom.remove_child(table, existing);
    }
}

/// `<table>.deleteTFoot()` — remove the first `<tfoot>` direct child.
pub fn delete_tfoot(table: Entity, dom: &mut EcsDom) {
    if let Some(existing) = first_section_child(table, dom, "tfoot") {
        let _ = dom.remove_child(table, existing);
    }
}

/// `<table>.deleteCaption()` — remove the first `<caption>` direct
/// child.
pub fn delete_caption(table: Entity, dom: &mut EcsDom) {
    if let Some(existing) = first_section_child(table, dom, "caption") {
        let _ = dom.remove_child(table, existing);
    }
}

fn insert_section_at_position(table: Entity, new: Entity, dom: &mut EcsDom, kind: SectionKind) {
    if let Some(ref_child) = find_table_section_insert_position(table, dom, kind) {
        let _ = dom.insert_before(table, new, ref_child);
    } else {
        let _ = dom.append_child(table, new);
    }
}

// ---------------------------------------------------------------------------
// `<table>.tHead` / `.tFoot` / `.caption` setter algorithms
// ---------------------------------------------------------------------------

/// Setter for `<table>.tHead`: replace existing `<thead>` (if any)
/// with `new_thead`.  `None` removes any existing `<thead>`.
///
/// Errors with `HierarchyRequestError` if `new_thead` is `Some(e)`
/// where `e` is not a `<thead>` element (HTML §4.9.1).
pub fn set_thead(
    table: Entity,
    dom: &mut EcsDom,
    new_thead: Option<Entity>,
) -> Result<(), DomApiError> {
    set_section_impl(table, dom, new_thead, SectionKind::THead)
}

/// Setter for `<table>.tFoot`: replace existing `<tfoot>` (if any)
/// with `new_tfoot`.  Errors with `HierarchyRequestError` for
/// non-`<tfoot>` arguments.
pub fn set_tfoot(
    table: Entity,
    dom: &mut EcsDom,
    new_tfoot: Option<Entity>,
) -> Result<(), DomApiError> {
    set_section_impl(table, dom, new_tfoot, SectionKind::TFoot)
}

/// Setter for `<table>.caption`: replace existing `<caption>` (if
/// any) with `new_caption`.  Errors with `HierarchyRequestError` for
/// non-`<caption>` arguments.
pub fn set_caption(
    table: Entity,
    dom: &mut EcsDom,
    new_caption: Option<Entity>,
) -> Result<(), DomApiError> {
    set_section_impl(table, dom, new_caption, SectionKind::Caption)
}

fn set_section_impl(
    table: Entity,
    dom: &mut EcsDom,
    new: Option<Entity>,
    kind: SectionKind,
) -> Result<(), DomApiError> {
    let expected_tag = match kind {
        SectionKind::Caption => "caption",
        SectionKind::THead => "thead",
        SectionKind::TFoot => "tfoot",
    };
    if let Some(new_node) = new {
        if !tag_eq_ci(dom, new_node, expected_tag) {
            return Err(DomApiError {
                kind: DomApiErrorKind::HierarchyRequestError,
                message: format!(
                    "table.{} setter requires a <{}> element",
                    match kind {
                        SectionKind::Caption => "caption",
                        SectionKind::THead => "tHead",
                        SectionKind::TFoot => "tFoot",
                    },
                    expected_tag
                ),
            });
        }
    }
    // Remove the existing matching section (if any).  When the new
    // node IS the existing one, this also detaches it cleanly so the
    // re-insert below lands at the spec position with no duplicate.
    if let Some(existing) = first_section_child(table, dom, expected_tag) {
        let _ = dom.remove_child(table, existing);
    }
    if let Some(new_node) = new {
        insert_section_at_position(table, new_node, dom, kind);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Row collection (`<table>.rows` per HTML §4.9.1)
// ---------------------------------------------------------------------------

/// Spec-ordered `<table>.rows` — same algorithm as the `TableRows`
/// LiveCollection walker, but materialised to a `Vec<Entity>` for
/// the `insertRow` / `deleteRow` algorithms (which need indexable
/// access without going through the wrapper-cache infra).
fn collect_table_rows(table: Entity, dom: &EcsDom) -> Vec<Entity> {
    let mut rows = Vec::new();
    if let Some(thead) = dom.first_child_with_tag(table, "thead") {
        for tr in dom.children_iter(thead) {
            if tag_eq_ci(dom, tr, "tr") {
                rows.push(tr);
            }
        }
    }
    for child in dom.children_iter(table) {
        if tag_eq_ci(dom, child, "tr") {
            rows.push(child);
        } else if tag_eq_ci(dom, child, "tbody") {
            for tr in dom.children_iter(child) {
                if tag_eq_ci(dom, tr, "tr") {
                    rows.push(tr);
                }
            }
        }
    }
    if let Some(tfoot) = dom.first_child_with_tag(table, "tfoot") {
        for tr in dom.children_iter(tfoot) {
            if tag_eq_ci(dom, tr, "tr") {
                rows.push(tr);
            }
        }
    }
    rows
}

/// Direct-child `<tr>` of a `<thead>` / `<tbody>` / `<tfoot>`
/// section, materialised for index lookup.
fn collect_section_rows(section: Entity, dom: &EcsDom) -> Vec<Entity> {
    dom.children_iter(section)
        .filter(|c| tag_eq_ci(dom, *c, "tr"))
        .collect()
}

/// Direct-child `<td>`/`<th>` of a `<tr>`.
fn collect_row_cells(row: Entity, dom: &EcsDom) -> Vec<Entity> {
    dom.children_iter(row)
        .filter(|c| tag_is_any_of(dom, *c, &["td", "th"]))
        .collect()
}

// ---------------------------------------------------------------------------
// `<table>.insertRow` / `.deleteRow`
// ---------------------------------------------------------------------------

fn index_size_error(message: impl Into<String>) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::IndexSizeError,
        message: message.into(),
    }
}

/// `<table>.insertRow(index)` per HTML §4.9.1 step 1-7.  See the
/// algorithm in the slot's plan-memo (`m4-12-pr-tags-t2c-table-plan.md`).
/// Returns the newly-created `<tr>` Entity on success.
pub fn insert_row_into_table(
    table: Entity,
    dom: &mut EcsDom,
    index: i32,
) -> Result<Entity, DomApiError> {
    let rows = collect_table_rows(table, dom);
    let len = rows.len();
    let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
    // Step 1: bounds check.  Allowed: -1 (append) or 0..=len.
    if index < -1 || index > len_i32 {
        return Err(index_size_error(format!(
            "insertRow index {index} out of range (rows.length = {len})"
        )));
    }
    let new_tr = dom.create_element("tr", Attributes::default());
    // Step 3: empty + no <tbody> → create implicit tbody, append tr to it.
    let has_tbody = dom.children_iter(table).any(|c| tag_eq_ci(dom, c, "tbody"));
    if rows.is_empty() && !has_tbody {
        let tbody = dom.create_element("tbody", Attributes::default());
        let _ = dom.append_child(table, tbody);
        let _ = dom.append_child(tbody, new_tr);
        return Ok(new_tr);
    }
    // Step 4: empty (table has tbody but no rows) → append to last <tbody>.
    if rows.is_empty() {
        let last_tbody = dom
            .children_iter(table)
            .filter(|c| tag_eq_ci(dom, *c, "tbody"))
            .last()
            .expect("has_tbody implies last tbody exists");
        let _ = dom.append_child(last_tbody, new_tr);
        return Ok(new_tr);
    }
    // Step 5: index == -1 OR index == len → append to parent of last row.
    let append_idx = index == -1 || index == len_i32;
    if append_idx {
        let last_row = rows[len - 1];
        let parent = dom
            .get_parent(last_row)
            .expect("rows entries always have a parent");
        let _ = dom.append_child(parent, new_tr);
        return Ok(new_tr);
    }
    // Step 6: insertBefore(new_tr, rows[index]) on rows[index]'s parent.
    let target_idx = usize::try_from(index)
        .map_err(|_| index_size_error(format!("insertRow index {index} out of range")))?;
    let target = rows[target_idx];
    let parent = dom
        .get_parent(target)
        .expect("rows entries always have a parent");
    let _ = dom.insert_before(parent, new_tr, target);
    Ok(new_tr)
}

/// `<table>.deleteRow(index)` per HTML §4.9.1.  Index `-1` removes
/// the last row.  Out-of-range → `IndexSizeError`.
pub fn delete_row_from_table(
    table: Entity,
    dom: &mut EcsDom,
    index: i32,
) -> Result<(), DomApiError> {
    let rows = collect_table_rows(table, dom);
    let len = rows.len();
    let target_idx = if index == -1 {
        if len == 0 {
            return Err(index_size_error(
                "deleteRow: cannot delete from an empty table",
            ));
        }
        len - 1
    } else {
        let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
        if index < 0 || index >= len_i32 {
            return Err(index_size_error(format!(
                "deleteRow index {index} out of range (rows.length = {len})"
            )));
        }
        usize::try_from(index)
            .map_err(|_| index_size_error(format!("deleteRow index {index} out of range")))?
    };
    let target = rows[target_idx];
    let parent = dom
        .get_parent(target)
        .expect("rows entries always have a parent");
    let _ = dom.remove_child(parent, target);
    Ok(())
}

// ---------------------------------------------------------------------------
// HTMLTableSectionElement.insertRow / .deleteRow
// ---------------------------------------------------------------------------

/// `<thead>`/`<tbody>`/`<tfoot>`.insertRow(index) per HTML §4.9.5-7.
/// Index `-1` appends; bounds = `0..=section.rows.length`.
pub fn insert_row_into_section(
    section: Entity,
    dom: &mut EcsDom,
    index: i32,
) -> Result<Entity, DomApiError> {
    let rows = collect_section_rows(section, dom);
    let len = rows.len();
    let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
    if index < -1 || index > len_i32 {
        return Err(index_size_error(format!(
            "insertRow index {index} out of range (section.rows.length = {len})"
        )));
    }
    let new_tr = dom.create_element("tr", Attributes::default());
    let append_idx = index == -1 || index == len_i32;
    if append_idx {
        let _ = dom.append_child(section, new_tr);
    } else {
        let target_idx = usize::try_from(index)
            .map_err(|_| index_size_error(format!("insertRow index {index} out of range")))?;
        let target = rows[target_idx];
        let _ = dom.insert_before(section, new_tr, target);
    }
    Ok(new_tr)
}

/// `<thead>`/`<tbody>`/`<tfoot>`.deleteRow(index) per HTML §4.9.5-7.
/// Index `-1` removes the last row.
pub fn delete_row_from_section(
    section: Entity,
    dom: &mut EcsDom,
    index: i32,
) -> Result<(), DomApiError> {
    let rows = collect_section_rows(section, dom);
    let len = rows.len();
    let target_idx = if index == -1 {
        if len == 0 {
            return Err(index_size_error(
                "deleteRow: cannot delete from an empty section",
            ));
        }
        len - 1
    } else {
        let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
        if index < 0 || index >= len_i32 {
            return Err(index_size_error(format!(
                "deleteRow index {index} out of range (section.rows.length = {len})"
            )));
        }
        usize::try_from(index)
            .map_err(|_| index_size_error(format!("deleteRow index {index} out of range")))?
    };
    let target = rows[target_idx];
    let _ = dom.remove_child(section, target);
    Ok(())
}

// ---------------------------------------------------------------------------
// HTMLTableRowElement.insertCell / .deleteCell
// ---------------------------------------------------------------------------

/// `<tr>.insertCell(index)` per HTML §4.9.8.  Creates a new `<td>`
/// and inserts it at `index` in the row's cells list.  Index `-1`
/// appends; bounds = `0..=row.cells.length`.
pub fn insert_cell_into_row(
    row: Entity,
    dom: &mut EcsDom,
    index: i32,
) -> Result<Entity, DomApiError> {
    let cells = collect_row_cells(row, dom);
    let len = cells.len();
    let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
    if index < -1 || index > len_i32 {
        return Err(index_size_error(format!(
            "insertCell index {index} out of range (row.cells.length = {len})"
        )));
    }
    let new_td = dom.create_element("td", Attributes::default());
    let append_idx = index == -1 || index == len_i32;
    if append_idx {
        let _ = dom.append_child(row, new_td);
    } else {
        let target_idx = usize::try_from(index)
            .map_err(|_| index_size_error(format!("insertCell index {index} out of range")))?;
        let target = cells[target_idx];
        let _ = dom.insert_before(row, new_td, target);
    }
    Ok(new_td)
}

/// `<tr>.deleteCell(index)` per HTML §4.9.8.  Index `-1` removes the
/// last cell.
pub fn delete_cell_from_row(row: Entity, dom: &mut EcsDom, index: i32) -> Result<(), DomApiError> {
    let cells = collect_row_cells(row, dom);
    let len = cells.len();
    let target_idx = if index == -1 {
        if len == 0 {
            return Err(index_size_error(
                "deleteCell: cannot delete from an empty row",
            ));
        }
        len - 1
    } else {
        let len_i32 = i32::try_from(len).unwrap_or(i32::MAX);
        if index < 0 || index >= len_i32 {
            return Err(index_size_error(format!(
                "deleteCell index {index} out of range (row.cells.length = {len})"
            )));
        }
        usize::try_from(index)
            .map_err(|_| index_size_error(format!("deleteCell index {index} out of range")))?
    };
    let target = cells[target_idx];
    let _ = dom.remove_child(row, target);
    Ok(())
}

// ---------------------------------------------------------------------------
// rowIndex / sectionRowIndex / cellIndex
// ---------------------------------------------------------------------------

/// `<tr>.rowIndex` — position of `row` in its containing `<table>`'s
/// `rows` list (across thead, tbodies in order, tfoot).  Returns
/// `-1` if `row` is not in a table.
#[must_use]
pub fn row_index(row: Entity, dom: &EcsDom) -> i32 {
    let Some(table) = ancestor_table(row, dom) else {
        return -1;
    };
    let rows = collect_table_rows(table, dom);
    rows.iter()
        .position(|&r| r == row)
        .and_then(|p| i32::try_from(p).ok())
        .unwrap_or(-1)
}

/// `<tr>.sectionRowIndex` — position of `row` in its parent
/// `<thead>`/`<tbody>`/`<tfoot>`'s rows list.  Returns `-1` if the
/// parent is not a section element (e.g. detached or direct child
/// of `<table>`).
#[must_use]
pub fn section_row_index(row: Entity, dom: &EcsDom) -> i32 {
    let Some(parent) = dom.get_parent(row) else {
        return -1;
    };
    if !tag_is_any_of(dom, parent, &["thead", "tbody", "tfoot"]) {
        return -1;
    }
    let rows = collect_section_rows(parent, dom);
    rows.iter()
        .position(|&r| r == row)
        .and_then(|p| i32::try_from(p).ok())
        .unwrap_or(-1)
}

/// `<td>`/`<th>`.cellIndex — position of `cell` in its parent
/// `<tr>`'s cells list.  Returns `-1` if the parent isn't a `<tr>`.
#[must_use]
pub fn cell_index(cell: Entity, dom: &EcsDom) -> i32 {
    let Some(parent) = dom.get_parent(cell) else {
        return -1;
    };
    if !tag_eq_ci(dom, parent, "tr") {
        return -1;
    }
    let cells = collect_row_cells(parent, dom);
    cells
        .iter()
        .position(|&c| c == cell)
        .and_then(|p| i32::try_from(p).ok())
        .unwrap_or(-1)
}

fn ancestor_table(row: Entity, dom: &EcsDom) -> Option<Entity> {
    // Walk: row's parent (might be table directly, or thead/tbody/tfoot).
    // If it's a section, walk up once more to reach the table.
    let parent = dom.get_parent(row)?;
    if tag_eq_ci(dom, parent, "table") {
        return Some(parent);
    }
    if tag_is_any_of(dom, parent, &["thead", "tbody", "tfoot"]) {
        let grand = dom.get_parent(parent)?;
        if tag_eq_ci(dom, grand, "table") {
            return Some(grand);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// DomApiHandler bindings
//
// VM-side per-tag prototype files invoke these via
// `invoke_dom_api(ctx, "<name>", entity, &args)` in the standard
// marshalling-only shape.  Algorithms above are the engine-independent
// truth; handlers are thin coercion + dispatch shims.
// ---------------------------------------------------------------------------

fn require_long_arg(args: &[JsValue], index: usize) -> Result<i32, DomApiError> {
    match args.get(index) {
        Some(JsValue::Number(n)) => {
            if n.is_nan() || n.is_infinite() {
                return Ok(0);
            }
            #[allow(clippy::cast_possible_truncation)]
            let truncated = n.trunc() as i64;
            if truncated > i64::from(i32::MAX) {
                Ok(i32::MAX)
            } else if truncated < i64::from(i32::MIN) {
                Ok(i32::MIN)
            } else {
                #[allow(clippy::cast_possible_truncation)]
                Ok(truncated as i32)
            }
        }
        Some(JsValue::Bool(b)) => Ok(i32::from(*b)),
        Some(JsValue::Null) => Ok(0),
        None | Some(JsValue::Undefined) => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} is required"),
        }),
        Some(_) => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a number"),
        }),
    }
}

fn optional_long_arg(args: &[JsValue], index: usize, default: i32) -> Result<i32, DomApiError> {
    match args.get(index) {
        None | Some(JsValue::Undefined) => Ok(default),
        _ => require_long_arg(args, index),
    }
}

fn resolve_optional_entity_arg(
    args: &[JsValue],
    index: usize,
    session: &SessionCore,
) -> Result<Option<Entity>, DomApiError> {
    match args.get(index) {
        None | Some(JsValue::Undefined | JsValue::Null) => Ok(None),
        Some(JsValue::ObjectRef(raw)) => {
            let (entity, _kind) = session
                .identity_map()
                .get(JsObjectRef::from_raw(*raw))
                .ok_or_else(|| not_found_error("argument is not a known node"))?;
            Ok(Some(entity))
        }
        _ => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a Node or null"),
        }),
    }
}

fn entity_to_objectref(entity: Entity, session: &mut SessionCore, kind: ComponentKind) -> JsValue {
    let obj_ref = session.get_or_create_wrapper(entity, kind);
    JsValue::ObjectRef(obj_ref.to_raw())
}

/// `<table>.insertRow(index?)`.
pub struct TableInsertRow;
impl DomApiHandler for TableInsertRow {
    fn method_name(&self) -> &str {
        "table.insertRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = optional_long_arg(args, 0, -1)?;
        let new = insert_row_into_table(this, dom, index)?;
        Ok(entity_to_objectref(new, session, ComponentKind::Element))
    }
}

/// `<table>.deleteRow(index)`.
pub struct TableDeleteRow;
impl DomApiHandler for TableDeleteRow {
    fn method_name(&self) -> &str {
        "table.deleteRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = require_long_arg(args, 0)?;
        delete_row_from_table(this, dom, index)?;
        Ok(JsValue::Undefined)
    }
}

/// `<thead>`/`<tbody>`/`<tfoot>`.insertRow(index?).
pub struct SectionInsertRow;
impl DomApiHandler for SectionInsertRow {
    fn method_name(&self) -> &str {
        "section.insertRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = optional_long_arg(args, 0, -1)?;
        let new = insert_row_into_section(this, dom, index)?;
        Ok(entity_to_objectref(new, session, ComponentKind::Element))
    }
}

/// `<thead>`/`<tbody>`/`<tfoot>`.deleteRow(index).
pub struct SectionDeleteRow;
impl DomApiHandler for SectionDeleteRow {
    fn method_name(&self) -> &str {
        "section.deleteRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = require_long_arg(args, 0)?;
        delete_row_from_section(this, dom, index)?;
        Ok(JsValue::Undefined)
    }
}

/// `<tr>.insertCell(index?)`.
pub struct RowInsertCell;
impl DomApiHandler for RowInsertCell {
    fn method_name(&self) -> &str {
        "row.insertCell"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = optional_long_arg(args, 0, -1)?;
        let new = insert_cell_into_row(this, dom, index)?;
        Ok(entity_to_objectref(new, session, ComponentKind::Element))
    }
}

/// `<tr>.deleteCell(index)`.
pub struct RowDeleteCell;
impl DomApiHandler for RowDeleteCell {
    fn method_name(&self) -> &str {
        "row.deleteCell"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = require_long_arg(args, 0)?;
        delete_cell_from_row(this, dom, index)?;
        Ok(JsValue::Undefined)
    }
}

/// `<table>.createTHead()` (idempotent).
pub struct TableCreateTHead;
impl DomApiHandler for TableCreateTHead {
    fn method_name(&self) -> &str {
        "table.createTHead"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_thead(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.createTFoot()` (idempotent).
pub struct TableCreateTFoot;
impl DomApiHandler for TableCreateTFoot {
    fn method_name(&self) -> &str {
        "table.createTFoot"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_tfoot(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.createCaption()` (idempotent).
pub struct TableCreateCaption;
impl DomApiHandler for TableCreateCaption {
    fn method_name(&self) -> &str {
        "table.createCaption"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_caption(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.createTBody()` (NOT idempotent — always creates).
pub struct TableCreateTBody;
impl DomApiHandler for TableCreateTBody {
    fn method_name(&self) -> &str {
        "table.createTBody"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_tbody(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.deleteTHead()` (no-op when absent).
pub struct TableDeleteTHead;
impl DomApiHandler for TableDeleteTHead {
    fn method_name(&self) -> &str {
        "table.deleteTHead"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        delete_thead(this, dom);
        Ok(JsValue::Undefined)
    }
}

/// `<table>.deleteTFoot()`.
pub struct TableDeleteTFoot;
impl DomApiHandler for TableDeleteTFoot {
    fn method_name(&self) -> &str {
        "table.deleteTFoot"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        delete_tfoot(this, dom);
        Ok(JsValue::Undefined)
    }
}

/// `<table>.deleteCaption()`.
pub struct TableDeleteCaption;
impl DomApiHandler for TableDeleteCaption {
    fn method_name(&self) -> &str {
        "table.deleteCaption"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        delete_caption(this, dom);
        Ok(JsValue::Undefined)
    }
}

/// `<table>.tHead = <thead> | null` setter.
pub struct TableSetTHead;
impl DomApiHandler for TableSetTHead {
    fn method_name(&self) -> &str {
        "table.tHead.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = resolve_optional_entity_arg(args, 0, session)?;
        set_thead(this, dom, new)?;
        Ok(JsValue::Undefined)
    }
}

/// `<table>.tFoot = <tfoot> | null` setter.
pub struct TableSetTFoot;
impl DomApiHandler for TableSetTFoot {
    fn method_name(&self) -> &str {
        "table.tFoot.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = resolve_optional_entity_arg(args, 0, session)?;
        set_tfoot(this, dom, new)?;
        Ok(JsValue::Undefined)
    }
}

/// `<table>.caption = <caption> | null` setter.
pub struct TableSetCaption;
impl DomApiHandler for TableSetCaption {
    fn method_name(&self) -> &str {
        "table.caption.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = resolve_optional_entity_arg(args, 0, session)?;
        set_caption(this, dom, new)?;
        Ok(JsValue::Undefined)
    }
}

#[cfg(test)]
mod tests;
