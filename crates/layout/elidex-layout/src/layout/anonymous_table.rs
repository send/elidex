//! CSS 2.1 §17.2.1: Anonymous table object generation.
//!
//! Implements Rules 2 and 3 of the anonymous table object algorithm:
//! - Rule 2: orphan table-cells → anonymous `Display::TableRow` wrapper
//! - Rule 3: orphan table-rows/row-groups/etc. → anonymous `Display::Table` wrapper

use elidex_ecs::{AnonymousTableMarker, EcsDom, Entity};
use elidex_plugin::{ComputedStyle, Display};

/// Returns `true` if the display type needs an anonymous table wrapper
/// (CSS 2.1 §17.2.1 Rule 3: row-like + caption + column elements).
fn needs_table_wrapper(display: Display) -> bool {
    matches!(
        display,
        Display::TableRow
            | Display::TableRowGroup
            | Display::TableHeaderGroup
            | Display::TableFooterGroup
            | Display::TableCaption
            | Display::TableColumn
            | Display::TableColumnGroup
    )
}

/// Generate anonymous table objects for orphan table-internal children
/// (CSS 2.1 §17.2.1 Rules 2 + 3).
///
/// Two-phase wrapping:
/// 1. Consecutive orphan `TableCell` children → anonymous `Display::TableRow`
/// 2. Consecutive table-internal children (including rows from phase 1)
///    → anonymous `Display::Table`
///
/// Uses pool-based reuse with `AnonymousTableMarker` for idempotent re-layout.
pub(crate) fn ensure_table_wrappers(dom: &mut EcsDom, parent: Entity) {
    // Phase 1: wrap orphan table-cells in anonymous rows (§17.2.1 Rule 2).
    wrap_orphan_cells_in_rows(dom, parent);

    // Phase 2: wrap table-internal children in anonymous tables (§17.2.1 Rule 3).
    wrap_table_internal_in_tables(dom, parent);
}

/// Phase 1: Wrap consecutive orphan `TableCell` children in anonymous `TableRow`.
fn wrap_orphan_cells_in_rows(dom: &mut EcsDom, parent: Entity) {
    let children: Vec<Entity> = elidex_layout_block::composed_children_flat(dom, parent);
    let mut has_orphan_cell = false;
    for &child in &children {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(child) {
            if style.display == Display::TableCell {
                has_orphan_cell = true;
                break;
            }
        }
    }
    if !has_orphan_cell {
        return;
    }

    // Collect existing anonymous TableRow entities for reuse.
    let mut pool: Vec<Entity> =
        elidex_layout_table::collect_anonymous_pool(dom, &children, Display::TableRow);

    let children: Vec<Entity> = elidex_layout_block::composed_children_flat(dom, parent);
    let mut run: Vec<Entity> = Vec::new();
    for &child in &children {
        let display = dom
            .world()
            .get::<&ComputedStyle>(child)
            .map(|s| s.display)
            .ok();
        if display == Some(Display::TableCell) {
            run.push(child);
        } else if !run.is_empty() {
            wrap_run_with_display(dom, parent, &run, Display::TableRow, "tr", &mut pool);
            run.clear();
        }
    }
    if !run.is_empty() {
        wrap_run_with_display(dom, parent, &run, Display::TableRow, "tr", &mut pool);
    }
}

/// Phase 2: Wrap consecutive table-internal children in anonymous `Table`.
fn wrap_table_internal_in_tables(dom: &mut EcsDom, parent: Entity) {
    let children: Vec<Entity> = elidex_layout_block::composed_children_flat(dom, parent);

    let mut has_orphan = false;
    for &child in &children {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(child) {
            if needs_table_wrapper(style.display) {
                has_orphan = true;
                break;
            }
        }
    }
    if !has_orphan {
        return;
    }

    let mut pool: Vec<Entity> =
        elidex_layout_table::collect_anonymous_pool(dom, &children, Display::Table);

    // Unpack: move each pool entity's children back to parent, then detach pool entity.
    for &anon in &pool {
        let anon_children: Vec<Entity> = dom.composed_children(anon);
        for &c in &anon_children {
            let _ = dom.insert_before(parent, c, anon);
        }
        let _ = dom.remove_child(parent, anon);
    }

    // Re-walk and wrap consecutive table-internal runs.
    let children: Vec<Entity> = elidex_layout_block::composed_children_flat(dom, parent);
    let mut run: Vec<Entity> = Vec::new();
    for &child in &children {
        let display = dom
            .world()
            .get::<&ComputedStyle>(child)
            .map(|s| s.display)
            .ok();
        if display.is_some_and(needs_table_wrapper) {
            run.push(child);
        } else if !run.is_empty() {
            wrap_run_with_display(dom, parent, &run, Display::Table, "table", &mut pool);
            run.clear();
        }
    }
    if !run.is_empty() {
        wrap_run_with_display(dom, parent, &run, Display::Table, "table", &mut pool);
    }
}

/// Wrap the given children in an anonymous entity with the specified display.
///
/// Reuses a pool entity if available, otherwise creates a new one.
fn wrap_run_with_display(
    dom: &mut EcsDom,
    parent: Entity,
    children: &[Entity],
    display: Display,
    tag: &str,
    pool: &mut Vec<Entity>,
) {
    debug_assert!(
        !children.is_empty(),
        "wrap_run_with_display called with empty children"
    );
    let parent_style = dom
        .world()
        .get::<&ComputedStyle>(parent)
        .map(|s| (*s).clone())
        .unwrap_or_default();
    let anon_style = elidex_layout_table::inherit_for_anonymous(&parent_style, display);

    let wrapper = if let Some(entity) = pool.pop() {
        let _ = dom.world_mut().insert_one(entity, anon_style);
        let _ = dom.insert_before(parent, entity, children[0]);
        entity
    } else {
        let w = dom.create_element(tag, elidex_ecs::Attributes::default());
        let _ = dom
            .world_mut()
            .insert(w, (AnonymousTableMarker, anon_style));
        let _ = dom.insert_before(parent, w, children[0]);
        w
    };

    for &child in children {
        let _ = dom.append_child(wrapper, child);
    }
}
