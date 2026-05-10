//! Inline unit tests for the engine-independent table mutation
//! algorithms (slot `#11-tags-T2c-table`).  Split into a sibling
//! file from `mod.rs` to keep production module under the 1000-line
//! convention (see `element/mod.rs` comment).

use super::*;

fn make_table(dom: &mut EcsDom) -> Entity {
    dom.create_element("table", Attributes::default())
}

#[test]
fn create_thead_idempotent() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let h1 = create_thead(table, &mut dom);
    let h2 = create_thead(table, &mut dom);
    assert_eq!(h1, h2);
    // Single <thead> direct child.
    let count = dom
        .children_iter(table)
        .filter(|c| tag_eq_ci(&dom, *c, "thead"))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn create_tbody_not_idempotent() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let b1 = create_tbody(table, &mut dom);
    let b2 = create_tbody(table, &mut dom);
    assert_ne!(b1, b2);
    let count = dom
        .children_iter(table)
        .filter(|c| tag_eq_ci(&dom, *c, "tbody"))
        .count();
    assert_eq!(count, 2);
}

#[test]
fn create_thead_position_after_caption() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let caption = create_caption(table, &mut dom);
    let thead = create_thead(table, &mut dom);
    let order: Vec<Entity> = dom.children_iter(table).collect();
    assert_eq!(order, vec![caption, thead]);
}

#[test]
fn create_caption_first_child() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let thead = create_thead(table, &mut dom);
    let caption = create_caption(table, &mut dom);
    let order: Vec<Entity> = dom.children_iter(table).collect();
    assert_eq!(order, vec![caption, thead]);
}

#[test]
fn delete_thead_noop_when_absent() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    delete_thead(table, &mut dom);
    assert_eq!(dom.children_iter(table).count(), 0);
}

#[test]
fn set_thead_with_div_throws() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let div = dom.create_element("div", Attributes::default());
    let res = set_thead(table, &mut dom, Some(div));
    assert!(matches!(
        res,
        Err(DomApiError {
            kind: DomApiErrorKind::HierarchyRequestError,
            ..
        })
    ));
}

#[test]
fn set_thead_replaces_existing() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let old = create_thead(table, &mut dom);
    let new = dom.create_element("thead", Attributes::default());
    set_thead(table, &mut dom, Some(new)).unwrap();
    let theads: Vec<Entity> = dom
        .children_iter(table)
        .filter(|c| tag_eq_ci(&dom, *c, "thead"))
        .collect();
    assert_eq!(theads, vec![new]);
    assert_ne!(theads[0], old);
}

#[test]
fn set_thead_null_removes() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let _old = create_thead(table, &mut dom);
    set_thead(table, &mut dom, None).unwrap();
    assert!(first_section_child(table, &dom, "thead").is_none());
}

#[test]
fn insert_row_implicit_tbody() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let tr = insert_row_into_table(table, &mut dom, -1).unwrap();
    // Implicit tbody created.
    let tbodies: Vec<Entity> = dom
        .children_iter(table)
        .filter(|c| tag_eq_ci(&dom, *c, "tbody"))
        .collect();
    assert_eq!(tbodies.len(), 1);
    assert_eq!(dom.get_parent(tr), Some(tbodies[0]));
}

#[test]
fn insert_row_bounds_check() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let res = insert_row_into_table(table, &mut dom, 1);
    assert!(matches!(
        res,
        Err(DomApiError {
            kind: DomApiErrorKind::IndexSizeError,
            ..
        })
    ));
    let res2 = insert_row_into_table(table, &mut dom, -2);
    assert!(matches!(
        res2,
        Err(DomApiError {
            kind: DomApiErrorKind::IndexSizeError,
            ..
        })
    ));
}

#[test]
fn delete_row_removes() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let tr = insert_row_into_table(table, &mut dom, -1).unwrap();
    delete_row_from_table(table, &mut dom, 0).unwrap();
    assert_eq!(row_index(tr, &dom), -1);
}

#[test]
fn row_index_walks_thead_tbody_tfoot() {
    let mut dom = EcsDom::new();
    let table = make_table(&mut dom);
    let thead = create_thead(table, &mut dom);
    let tbody = create_tbody(table, &mut dom);
    let tfoot = create_tfoot(table, &mut dom);
    let h_tr = dom.create_element("tr", Attributes::default());
    let b_tr = dom.create_element("tr", Attributes::default());
    let f_tr = dom.create_element("tr", Attributes::default());
    let _ = dom.append_child(thead, h_tr);
    let _ = dom.append_child(tbody, b_tr);
    let _ = dom.append_child(tfoot, f_tr);
    assert_eq!(row_index(h_tr, &dom), 0);
    assert_eq!(row_index(b_tr, &dom), 1);
    assert_eq!(row_index(f_tr, &dom), 2);
}

#[test]
fn section_row_index_within_section() {
    let mut dom = EcsDom::new();
    let tbody = dom.create_element("tbody", Attributes::default());
    let tr1 = dom.create_element("tr", Attributes::default());
    let tr2 = dom.create_element("tr", Attributes::default());
    let _ = dom.append_child(tbody, tr1);
    let _ = dom.append_child(tbody, tr2);
    assert_eq!(section_row_index(tr1, &dom), 0);
    assert_eq!(section_row_index(tr2, &dom), 1);
}

#[test]
fn cell_index_basic() {
    let mut dom = EcsDom::new();
    let tr = dom.create_element("tr", Attributes::default());
    let td = dom.create_element("td", Attributes::default());
    let th = dom.create_element("th", Attributes::default());
    let _ = dom.append_child(tr, td);
    let _ = dom.append_child(tr, th);
    assert_eq!(cell_index(td, &dom), 0);
    assert_eq!(cell_index(th, &dom), 1);
}

#[test]
fn cell_index_detached() {
    let mut dom = EcsDom::new();
    let td = dom.create_element("td", Attributes::default());
    assert_eq!(cell_index(td, &dom), -1);
}

#[test]
fn insert_cell_into_row_basic() {
    let mut dom = EcsDom::new();
    let tr = dom.create_element("tr", Attributes::default());
    let td = insert_cell_into_row(tr, &mut dom, -1).unwrap();
    assert_eq!(cell_index(td, &dom), 0);
}

#[test]
fn delete_cell_oob() {
    let mut dom = EcsDom::new();
    let tr = dom.create_element("tr", Attributes::default());
    let res = delete_cell_from_row(tr, &mut dom, 0);
    assert!(matches!(
        res,
        Err(DomApiError {
            kind: DomApiErrorKind::IndexSizeError,
            ..
        })
    ));
}
