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
    let (dom, mut reg, doc) = fresh();
    let sel = SelectionState::new();
    assert_eq!(sel.range_count(&mut reg, &dom, doc), 0);
    assert!(sel.is_empty());
    assert_eq!(sel.direction, SelectionDirection::Directionless);
    assert!(sel.current_range_id().is_none());
}

#[test]
fn empty_selection_type_and_collapsed() {
    let (dom, mut reg, doc) = fresh();
    let sel = SelectionState::new();
    assert_eq!(sel.selection_type(&mut reg, &dom, doc), SelectionType::None);
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
    assert_eq!(sel.range_count(&mut reg, &dom, doc), 1);
    assert_eq!(
        sel.selection_type(&mut reg, &dom, doc),
        SelectionType::Caret
    );
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
fn collapse_cross_document_aborts() {
    let (mut dom, mut reg, doc) = fresh();
    let other_doc = dom.create_document_root();
    let foreign = elem_in(&mut dom, "p", other_doc);
    let _ = dom.append_child(other_doc, foreign);
    let mut sel = SelectionState::new();
    assert_eq!(sel.collapse(&mut reg, &dom, doc, foreign, 0), Ok(false));
    assert!(sel.is_empty(), "abort leaves the selection empty");
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
    assert_eq!(sel.range_count(&mut reg, &dom, doc), 1);
    assert_eq!(
        sel.selection_type(&mut reg, &dom, doc),
        SelectionType::Range
    );
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
    assert_eq!(
        sel.set_base_and_extent(&mut reg, &dom, doc, p, 0, p, 0),
        Ok(true)
    );
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
        !sel.add_range(&mut reg, &dom, doc, new_id),
        "must no-op: single-range selection"
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
    assert!(!sel.add_range(&mut reg, &dom, doc, new_id));
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
    assert!(sel.add_range(&mut reg, &dom, doc, id));
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
    assert_eq!(sel.collapse(&mut reg, &dom, doc, p, 0), Ok(true));
    assert!(!sel.is_empty());
    sel.remove_all_ranges();
    assert!(sel.is_empty());
}

#[test]
fn empty_is_alias_of_remove_all_ranges() {
    let (mut dom, mut reg, doc) = fresh();
    let p = elem_in(&mut dom, "p", doc);
    let _ = dom.append_child(doc, p);
    let mut sel = SelectionState::new();
    assert_eq!(sel.collapse(&mut reg, &dom, doc, p, 0), Ok(true));
    assert!(!sel.is_empty());
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
    assert_eq!(sel.collapse(&mut reg, &dom, doc, p, 0), Ok(true));
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

// ---------------------------------------------------------------------------
// tsel: Selection operations check the spec's tree preconditions
// (tree-model tsel plan §7, cells E1-E13). Every expected value is the
// Selection API / DOM text's, not an engine's.
// ---------------------------------------------------------------------------

/// `doc` > `parent` > [`pre` ("pre"), `host` (closed shadow root > `st`
/// ("SHADOW")), `post` ("postTAIL")]; `det` (a detached element) > `dt`
/// ("detached"); `doc2` > `foreign_el` > `ft` ("foreign").
struct Tree {
    dom: EcsDom,
    reg: LiveRangeRegistry,
    doc: Entity,
    parent: Entity,
    pre: Entity,
    host: Entity,
    post: Entity,
    sr: Entity,
    se: Entity,
    st: Entity,
    det: Entity,
    dt: Entity,
    foreign_el: Entity,
    ft: Entity,
}

fn text_in(dom: &mut EcsDom, data: &str, doc: Entity) -> Entity {
    let owner = dom.node_document_for_new_nodes(doc);
    dom.create_text_with_owner(data, owner)
}

fn tree() -> Tree {
    let (mut dom, reg, doc) = fresh();
    let parent = elem_in(&mut dom, "div", doc);
    let _ = dom.append_child(doc, parent);
    let pre = text_in(&mut dom, "pre", doc);
    let host = elem_in(&mut dom, "div", doc);
    let post = text_in(&mut dom, "postTAIL", doc);
    for c in [pre, host, post] {
        let _ = dom.append_child(parent, c);
    }
    let sr = dom
        .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
        .unwrap();
    let se = elem_in(&mut dom, "span", doc);
    let st = text_in(&mut dom, "SHADOW", doc);
    let _ = dom.append_child(sr, se);
    let _ = dom.append_child(se, st);
    let det = elem_in(&mut dom, "p", doc);
    let dt = text_in(&mut dom, "detached", doc);
    let _ = dom.append_child(det, dt);
    let doc2 = dom.create_document_root();
    let foreign_el = elem_in(&mut dom, "p", doc2);
    let _ = dom.append_child(doc2, foreign_el);
    let ft = text_in(&mut dom, "foreign", doc2);
    let _ = dom.append_child(foreign_el, ft);
    Tree {
        dom,
        reg,
        doc,
        parent,
        pre,
        host,
        post,
        sr,
        se,
        st,
        det,
        dt,
        foreign_el,
        ft,
    }
}

fn register_range(
    reg: &mut LiveRangeRegistry,
    dom: &EcsDom,
    node: Entity,
    start: usize,
    end: usize,
) -> RangeId {
    let mut r = Range::new_with_owner(node, dom.find_tree_root(node));
    r.set_start(node, start);
    r.set_end(node, end);
    reg.register(r)
}

fn range_root(t: &mut Tree, id: RangeId) -> (Entity, Entity) {
    t.reg
        .with_range(id, &t.dom, |r, dom| {
            (
                dom.find_tree_root(r.start_container),
                dom.find_tree_root(r.end_container),
            )
        })
        .unwrap()
}

#[test]
fn set_base_and_extent_cross_root_collapses() {
    for (a, ao, f, fo) in [(0, 1, 1, 3), (1, 3, 0, 1)] {
        let mut t = tree();
        let nodes = [t.pre, t.st];
        let mut sel = SelectionState::new();
        let r = sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, nodes[a], ao, nodes[f], fo);
        assert_eq!(r, Ok(true));
        assert!(sel.is_collapsed(&mut t.reg, &t.dom));
        let id = sel.current_range_id().unwrap();
        let (s, e) = range_root(&mut t, id);
        assert_eq!(s, e, "start root = end root");
    }
}

#[test]
fn extend_cross_root_collapses_at_new_focus() {
    // Forward shape: caret before the host, extend into the shadow Text.
    // Backward shape: caret after the host (the shadow Text orders before it).
    for (caret, caret_off) in [(0usize, 1usize), (1, 2)] {
        let mut t = tree();
        let carets = [t.pre, t.post];
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.collapse(&mut t.reg, &t.dom, t.doc, carets[caret], caret_off),
            Ok(true)
        );
        assert_eq!(sel.extend(&mut t.reg, &t.dom, t.doc, t.st, 3), Ok(true));
        assert!(sel.is_collapsed(&mut t.reg, &t.dom));
        assert_eq!(sel.anchor(&mut t.reg, &t.dom), Some((t.st, 3)));
        assert_eq!(sel.focus(&mut t.reg, &t.dom), Some((t.st, 3)));
    }
}

#[test]
fn select_all_children_outside_document_tree_aborts() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    assert_eq!(sel.collapse(&mut t.reg, &t.dom, t.doc, t.pre, 1), Ok(true));
    let before = sel.current_range_id();
    let regs = t.reg.len();
    for n in [t.se, t.sr, t.det] {
        assert_eq!(
            sel.select_all_children(&mut t.reg, &t.dom, t.doc, n),
            Ok(false)
        );
        assert_eq!(sel.current_range_id(), before);
        assert_eq!(sel.anchor(&mut t.reg, &t.dom), Some((t.pre, 1)));
    }
    assert_eq!(t.reg.len(), regs, "no Range registered by an abort");
    assert_eq!(
        sel.select_all_children(&mut t.reg, &t.dom, t.doc, t.parent),
        Ok(true)
    );
}

#[test]
fn add_range_not_rooted_at_document_aborts() {
    let mut t = tree();
    let in_shadow = register_range(&mut t.reg, &t.dom, t.st, 0, 2);
    let detached = register_range(&mut t.reg, &t.dom, t.dt, 0, 2);
    let foreign = register_range(&mut t.reg, &t.dom, t.ft, 0, 2);
    let mut sel = SelectionState::new();
    for id in [in_shadow, detached, foreign] {
        assert!(!sel.add_range(&mut t.reg, &t.dom, t.doc, id));
        assert!(sel.is_empty());
    }
    let ok = register_range(&mut t.reg, &t.dom, t.pre, 0, 2);
    assert!(sel.add_range(&mut t.reg, &t.dom, t.doc, ok));
    assert_eq!(sel.current_range_id(), Some(ok));
}

/// Selection API `addRange()` step 1 reads the root of BOTH boundary points:
/// a registered Range whose start is in the document and end in a shadow
/// tree (reachable through a shadow-root child moved out, which the
/// adjustment skips) is aborted, and the selection stays empty.
#[test]
fn add_range_two_root_range_aborts() {
    let mut t = tree();
    let id = register_range(&mut t.reg, &t.dom, t.pre, 0, 2);
    t.reg
        .with_range_mut(id, &t.dom, |r, _| r.set_end(t.st, 1))
        .unwrap();
    assert_eq!(
        range_root(&mut t, id),
        (t.doc, t.sr),
        "two-root range set up"
    );
    let mut sel = SelectionState::new();
    assert!(!sel.add_range(&mut t.reg, &t.dom, t.doc, id));
    assert!(sel.is_empty());
}

#[test]
fn setters_outside_document_abort() {
    for outside in 0..2 {
        let mut t = tree();
        let n = [t.dt, t.ft][outside];
        let mut sel = SelectionState::new();
        // On an empty selection: collapse / setBaseAndExtent abort.
        assert_eq!(sel.collapse(&mut t.reg, &t.dom, t.doc, n, 1), Ok(false));
        assert_eq!(
            sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, n, 0, n, 3),
            Ok(false)
        );
        assert!(sel.is_empty());
        // With a prior caret: extend aborts and the caret is untouched.
        assert_eq!(sel.collapse(&mut t.reg, &t.dom, t.doc, t.pre, 1), Ok(true));
        let id = sel.current_range_id();
        let regs = t.reg.len();
        assert_eq!(sel.extend(&mut t.reg, &t.dom, t.doc, n, 1), Ok(false));
        assert_eq!(sel.collapse(&mut t.reg, &t.dom, t.doc, n, 1), Ok(false));
        assert_eq!(
            sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, n, 0, t.pre, 1),
            Ok(false)
        );
        assert_eq!(sel.current_range_id(), id);
        assert_eq!(t.reg.len(), regs);
        assert_eq!(sel.anchor(&mut t.reg, &t.dom), Some((t.pre, 1)));
    }
}

#[test]
fn shadow_tree_boundary_is_accepted() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    assert_eq!(sel.collapse(&mut t.reg, &t.dom, t.doc, t.st, 1), Ok(true));
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.st, 0, t.st, 3),
        Ok(true)
    );
    assert!(!sel.is_collapsed(&mut t.reg, &t.dom));
}

#[test]
fn setter_step_order() {
    let mut t = tree();
    let own_dt = t.dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    let _ = t.dom.append_child(t.doc, own_dt);
    let doc2 = t.dom.find_tree_root(t.foreign_el);
    let fdt = t.dom.create_document_type(
        "html",
        "",
        "",
        elidex_ecs::NewNodeOwner::ownerless_fixture(),
    );
    let _ = t.dom.append_child(doc2, fdt);
    let (doc, p) = (t.doc, t.parent);
    let mut sel = SelectionState::new();
    // D1: the document check (step 2) precedes DOM step 1's DocumentType.
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, doc, p, 0, fdt, 0),
        Ok(false)
    );
    // D2: an own-document doctype throws.
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, doc, p, 0, own_dt, 0),
        Err(SelectionError::InvalidNodeType)
    );
    // D3: the offset (step 1) precedes the DocumentType.
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, doc, p, 0, own_dt, 5),
        Err(SelectionError::IndexSize)
    );
    // D5: extend's document check (step 1) precedes the emptiness check.
    assert_eq!(sel.extend(&mut t.reg, &t.dom, doc, fdt, 0), Ok(false));
    // collapse: the offset (step 3) precedes the document check (step 4).
    assert_eq!(
        sel.collapse(&mut t.reg, &t.dom, doc, t.ft, 99),
        Err(SelectionError::IndexSize)
    );
    assert_eq!(
        sel.extend(&mut t.reg, &t.dom, doc, p, 0),
        Err(SelectionError::InvalidState)
    );
    assert!(sel.is_empty());
}

fn text_data(t: &Tree, n: Entity) -> String {
    t.dom
        .world()
        .get::<&elidex_ecs::TextContent>(n)
        .unwrap()
        .0
        .clone()
}

#[test]
fn delete_from_document_outside_document_tree_does_nothing() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    // Empty selection: nothing.
    assert!(sel
        .delete_from_document(&mut t.reg, &mut t.dom, t.doc)
        .is_none());
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.st, 0, t.st, 3),
        Ok(true)
    );
    let id = sel.current_range_id();
    assert!(sel
        .delete_from_document(&mut t.reg, &mut t.dom, t.doc)
        .is_none());
    assert_eq!(text_data(&t, t.st), "SHADOW");
    assert_eq!(sel.current_range_id(), id);
    assert_eq!(sel.anchor(&mut t.reg, &t.dom), Some((t.st, 0)));
    assert_eq!(sel.focus(&mut t.reg, &t.dom), Some((t.st, 3)));
    // Control: the same over a light Text in `doc`.
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.pre, 0, t.pre, 2),
        Ok(true)
    );
    let records = sel
        .delete_from_document(&mut t.reg, &mut t.dom, t.doc)
        .expect("runs deleteContents()");
    assert_eq!(text_data(&t, t.pre), "e");
    let cd = records
        .iter()
        .filter(|r| r.kind == elidex_script_session::MutationKind::CharacterData)
        .count();
    assert_eq!(cd, 1);
}

#[test]
fn contains_node_root_not_document_is_false() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.select_all_children(&mut t.reg, &t.dom, t.doc, t.parent),
        Ok(true)
    );
    for partial in [false, true] {
        assert!(!sel.contains_node(&mut t.reg, &t.dom, t.doc, t.st, partial));
    }
    assert!(!sel.contains_node(&mut t.reg, &t.dom, t.doc, t.det, true));
    assert!(sel.contains_node(&mut t.reg, &t.dom, t.doc, t.host, false));
}

#[test]
fn range_at_outside_document_tree_is_index_size() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.range_at(0, &mut t.reg, &t.dom, t.doc),
        Err(SelectionError::OutOfRange),
        "empty"
    );
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.st, 0, t.st, 3),
        Ok(true)
    );
    assert_eq!(
        sel.range_at(0, &mut t.reg, &t.dom, t.doc),
        Err(SelectionError::OutOfRange)
    );
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.pre, 0, t.pre, 2),
        Ok(true)
    );
    assert_eq!(
        sel.range_at(0, &mut t.reg, &t.dom, t.doc),
        Ok(sel.current_range_id().unwrap())
    );
    assert_eq!(
        sel.range_at(1, &mut t.reg, &t.dom, t.doc),
        Err(SelectionError::OutOfRange)
    );
}

#[test]
fn attributes_outside_document_tree() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    let (reg, dom, doc) = (&mut t.reg, &t.dom, t.doc);
    // Empty: None, None, 0, None.
    assert_eq!(sel.anchor_in_document_tree(reg, dom, doc), None);
    assert_eq!(sel.focus_in_document_tree(reg, dom, doc), None);
    assert_eq!(sel.range_count(reg, dom, doc), 0);
    assert_eq!(sel.selection_type(reg, dom, doc), SelectionType::None);
    // Inside the shadow tree: the attributes' clause, with the concepts kept.
    assert_eq!(
        sel.set_base_and_extent(reg, dom, doc, t.st, 0, t.st, 3),
        Ok(true)
    );
    assert_eq!(sel.anchor_in_document_tree(reg, dom, doc), None);
    assert_eq!(sel.focus_in_document_tree(reg, dom, doc), None);
    assert_eq!(sel.range_count(reg, dom, doc), 0);
    assert_eq!(sel.selection_type(reg, dom, doc), SelectionType::None);
    assert_eq!(sel.anchor(reg, dom), Some((t.st, 0)));
    assert_eq!(sel.focus(reg, dom), Some((t.st, 3)));
    assert!(!sel.is_collapsed(reg, dom));
    assert_eq!(sel.collapse(reg, dom, doc, t.st, 1), Ok(true));
    assert_eq!(sel.range_count(reg, dom, doc), 0);
    assert_eq!(sel.selection_type(reg, dom, doc), SelectionType::None);
    // Control: a light Text in `doc`.
    assert_eq!(
        sel.set_base_and_extent(reg, dom, doc, t.pre, 0, t.pre, 2),
        Ok(true)
    );
    assert_eq!(sel.anchor_in_document_tree(reg, dom, doc), Some((t.pre, 0)));
    assert_eq!(sel.focus_in_document_tree(reg, dom, doc), Some((t.pre, 2)));
    assert_eq!(sel.range_count(reg, dom, doc), 1);
    assert_eq!(sel.selection_type(reg, dom, doc), SelectionType::Range);
}

#[test]
fn add_range_over_selection_outside_document_tree_sets() {
    let mut t = tree();
    let mut sel = SelectionState::new();
    assert_eq!(
        sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.st, 0, t.st, 3),
        Ok(true)
    );
    let r = register_range(&mut t.reg, &t.dom, t.pre, 0, 2);
    assert!(sel.add_range(&mut t.reg, &t.dom, t.doc, r));
    assert_eq!(sel.current_range_id(), Some(r));
    // Control: a selection in the document tree still blocks addRange().
    let r2 = register_range(&mut t.reg, &t.dom, t.post, 0, 2);
    assert!(!sel.add_range(&mut t.reg, &t.dom, t.doc, r2));
    assert_eq!(sel.current_range_id(), Some(r));
}

#[test]
fn selection_range_moved_out_by_range_write() {
    for which in 0..3 {
        let mut t = tree();
        let x = [t.st, t.det, t.ft][which];
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.set_base_and_extent(&mut t.reg, &t.dom, t.doc, t.pre, 0, t.pre, 2),
            Ok(true)
        );
        let id = sel.current_range_id().unwrap();
        // A Range native writes the selection's own registered Range.
        t.reg
            .with_range_mut(id, &t.dom, |r, dom| r.set_start_to_boundary(x, 0, dom))
            .unwrap();
        let collapsed = t.reg.with_range(id, &t.dom, |r, _| {
            (r.collapsed(), r.start_container, r.start_offset)
        });
        assert_eq!(collapsed, Some((true, x, 0)));
        let (reg, dom, doc) = (&mut t.reg, &t.dom, t.doc);
        assert_eq!(
            sel.current_range_id(),
            Some(id),
            "same range stays associated"
        );
        assert_eq!(sel.range_count(reg, dom, doc), 0);
        assert_eq!(sel.selection_type(reg, dom, doc), SelectionType::None);
        assert_eq!(sel.anchor_in_document_tree(reg, dom, doc), None);
        assert_eq!(sel.focus_in_document_tree(reg, dom, doc), None);
        assert_eq!(
            sel.range_at(0, reg, dom, doc),
            Err(SelectionError::OutOfRange)
        );
        assert!(sel
            .delete_from_document(&mut t.reg, &mut t.dom, t.doc)
            .is_none());
        let r = register_range(&mut t.reg, &t.dom, t.post, 0, 2);
        assert!(
            sel.add_range(&mut t.reg, &t.dom, t.doc, r),
            "step 2 reads 0"
        );
    }
}
