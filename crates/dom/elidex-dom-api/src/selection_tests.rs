use super::*;
use elidex_ecs::Attributes;

/// Create an element AND set its `AssociatedDocument` to `doc` so
/// the engine-indep `EcsDom::owner_document(node) == Some(doc)`
/// holds (matches the VM-side `document.createElement` path which
/// always sets the association via `create_element_with_owner`).
/// Tests without a Document anchor use the raw [`Self::elem`].
fn elem_in(dom: &mut EcsDom, tag: &str, doc: Entity) -> Entity {
    dom.create_element_with_owner(
        tag,
        Attributes::default(),
        dom.node_document_for_new_nodes(doc),
    )
}

#[allow(dead_code)] // kept for future cross-doc / no-association tests
fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element_ownerless_fixture(tag, Attributes::default())
}

fn fresh() -> (EcsDom, LiveRangeRegistry, Entity) {
    let dom = EcsDom::new();
    let (reg, _bridge) = LiveRangeRegistry::new_pair();
    let mut dom = dom;
    // Build a Document anchor entity that nodes can `owner_document` to.
    let doc = dom.create_document_root();
    (dom, reg, doc)
}

#[test]
fn new_state_is_empty_directionless() {
    let sel = SelectionState::new();
    assert_eq!(sel.range_count(), 0);
    assert!(sel.is_empty());
    assert_eq!(sel.direction, SelectionDirection::Directionless);
    assert!(sel.current_range_id().is_none());
}

#[test]
fn empty_selection_type_and_collapsed() {
    let (dom, mut reg, _doc) = fresh();
    let sel = SelectionState::new();
    assert_eq!(sel.selection_type(&mut reg, &dom), SelectionType::None);
    assert!(sel.is_collapsed(&mut reg, &dom));
    assert!(sel.anchor(&mut reg, &dom).is_none());
    assert!(sel.focus(&mut reg, &dom).is_none());
}

#[test]
fn collapse_sets_caret_directionless() {
    let (mut dom, mut reg, doc) = fresh();
    let child = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, child);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, child, 0).unwrap();
    assert_eq!(sel.range_count(), 1);
    assert_eq!(sel.selection_type(&mut reg, &dom), SelectionType::Caret);
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Directionless
    );
    assert_eq!(sel.anchor(&mut reg, &dom), Some((child, 0)));
    assert_eq!(sel.focus(&mut reg, &dom), Some((child, 0)));
}

#[test]
fn collapse_rejects_doctype() {
    let (mut dom, mut reg, doc) = fresh();
    let doctype = dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.collapse(&mut reg, &dom, doc, doctype, 0),
        Err(SelectionError::InvalidNodeType)
    );
    assert!(sel.is_empty(), "state untouched on reject");
}

#[test]
fn collapse_rejects_cross_document() {
    let (mut dom, mut reg, doc) = fresh();
    let other_doc = dom.create_document_root();
    let foreign = elem_in(&mut dom, "p", other_doc);
    let _ = dom.append_child(other_doc, foreign);
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.collapse(&mut reg, &dom, doc, foreign, 0),
        Err(SelectionError::WrongDocument)
    );
}

#[test]
fn collapse_rejects_out_of_bounds_offset() {
    let (mut dom, mut reg, doc) = fresh();
    let child = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, child);
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.collapse(&mut reg, &dom, doc, child, 99),
        Err(SelectionError::IndexSize)
    );
}

#[test]
fn select_all_children_makes_range() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let c0 = elem_in(&mut dom, "span", doc);
    let c1 = elem_in(&mut dom, "span", doc);
    let _ = dom.append_child(p, c0);
    let _ = dom.append_child(p, c1);
    let mut sel = SelectionState::new();
    sel.select_all_children(&mut reg, &dom, doc, p).unwrap();
    assert_eq!(sel.range_count(), 1);
    assert_eq!(sel.selection_type(&mut reg, &dom), SelectionType::Range);
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Directionless,
        "selectAllChildren does NOT establish direction"
    );
    assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 0)));
    assert_eq!(sel.focus(&mut reg, &dom), Some((p, 2)));
}

#[test]
fn extend_forward_when_focus_after_anchor() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let c0 = elem_in(&mut dom, "span", doc);
    let c1 = elem_in(&mut dom, "span", doc);
    let _ = dom.append_child(p, c0);
    let _ = dom.append_child(p, c1);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
    sel.extend(&mut reg, &dom, doc, p, 2).unwrap();
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Forward
    );
    assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 0)));
    assert_eq!(sel.focus(&mut reg, &dom), Some((p, 2)));
}

#[test]
fn extend_backward_when_focus_before_anchor() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let c0 = elem_in(&mut dom, "span", doc);
    let c1 = elem_in(&mut dom, "span", doc);
    let _ = dom.append_child(p, c0);
    let _ = dom.append_child(p, c1);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 2).unwrap();
    sel.extend(&mut reg, &dom, doc, p, 0).unwrap();
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Backward
    );
    assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 2)));
    assert_eq!(sel.focus(&mut reg, &dom), Some((p, 0)));
}

#[test]
fn extend_rejects_without_initial_range() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.extend(&mut reg, &dom, doc, p, 0),
        Err(SelectionError::InvalidState)
    );
}

#[test]
fn set_base_and_extent_collapsed_is_directionless() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    sel.set_base_and_extent(&mut reg, &dom, doc, p, 0, p, 0)
        .unwrap();
    assert!(sel.is_collapsed(&mut reg, &dom));
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Directionless,
        "collapsed setBaseAndExtent should NOT report a direction"
    );
}

#[test]
fn set_base_and_extent_backward_reverses_range() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let c0 = elem_in(&mut dom, "span", doc);
    let c1 = elem_in(&mut dom, "span", doc);
    let _ = dom.append_child(p, c0);
    let _ = dom.append_child(p, c1);
    let mut sel = SelectionState::new();
    sel.set_base_and_extent(&mut reg, &dom, doc, p, 2, p, 0)
        .unwrap();
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Backward
    );
    assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 2)));
    assert_eq!(sel.focus(&mut reg, &dom), Some((p, 0)));
}

#[test]
fn add_range_no_op_when_range_set() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
    let original = sel.current_range_id().unwrap();
    let mut r = Range::new_with_owner(p, doc);
    r.set_start(p, 0);
    r.set_end(p, 0);
    let new_id = reg.register(r);
    assert!(
        !sel.add_range(doc, doc, new_id),
        "must no-op per Chrome single-range"
    );
    assert_eq!(sel.current_range_id(), Some(original));
}

#[test]
fn add_range_rejected_cross_document() {
    let (mut dom, mut reg, doc) = fresh();
    let other = dom.create_document_root();
    let p = elem_in(&mut dom, "p", other);
    let _ = dom.append_child(other, p);
    let mut sel = SelectionState::new();
    let mut r = Range::new_with_owner(p, other);
    r.set_start(p, 0);
    r.set_end(p, 0);
    let new_id = reg.register(r);
    assert!(!sel.add_range(other, doc, new_id));
    assert!(sel.is_empty());
}

#[test]
fn add_range_sets_when_empty() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    let mut r = Range::new_with_owner(p, doc);
    r.set_start(p, 0);
    r.set_end(p, 0);
    let id = reg.register(r);
    assert!(sel.add_range(doc, doc, id));
    assert_eq!(sel.current_range_id(), Some(id));
    assert_eq!(
        sel.current_direction(&mut reg, &dom),
        SelectionDirection::Directionless
    );
}

#[test]
fn remove_range_clears_when_matching() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
    let id = sel.current_range_id().unwrap();
    sel.remove_range(id).unwrap();
    assert!(sel.is_empty());
}

#[test]
fn remove_range_rejects_non_current() {
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.remove_range(RangeId(999)),
        Err(SelectionError::InvalidState)
    );
}

#[test]
fn remove_all_ranges_empties() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
    sel.remove_all_ranges();
    assert!(sel.is_empty());
}

#[test]
fn empty_is_alias_of_remove_all_ranges() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
    sel.empty();
    assert!(sel.is_empty());
}

#[test]
fn contains_node_full_contain_basic() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let c0 = elem_in(&mut dom, "span", doc);
    let c1 = elem_in(&mut dom, "span", doc);
    let _ = dom.append_child(p, c0);
    let _ = dom.append_child(p, c1);
    let mut sel = SelectionState::new();
    sel.select_all_children(&mut reg, &dom, doc, p).unwrap();
    assert!(sel.contains_node(&mut reg, &dom, doc, c0, false));
    assert!(sel.contains_node(&mut reg, &dom, doc, c1, false));
}

#[test]
fn contains_node_cross_document_false() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let other_doc = dom.create_document_root();
    let foreign = elem_in(&mut dom, "span", other_doc);
    let _ = dom.append_child(other_doc, foreign);
    let mut sel = SelectionState::new();
    sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
    assert!(
        !sel.contains_node(&mut reg, &dom, doc, foreign, false),
        "cross-doc must return false, not throw"
    );
    assert!(
        !sel.contains_node(&mut reg, &dom, doc, foreign, true),
        "cross-doc with allowPartial=true must still return false"
    );
}

#[test]
fn empty_selection_to_string_is_empty() {
    let (dom, mut reg, _doc) = fresh();
    let sel = SelectionState::new();
    assert_eq!(sel.to_string(&mut reg, &dom), "");
}

#[test]
fn direction_str_round_trip() {
    for d in [
        SelectionDirection::Forward,
        SelectionDirection::Backward,
        SelectionDirection::Directionless,
    ] {
        // Just exercise the helper; the actual values are spec
        // verbatim and asserted in tests_selection.rs.
        assert!(!d.as_str().is_empty());
    }
}
