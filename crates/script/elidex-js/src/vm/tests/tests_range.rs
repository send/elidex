//! Integration tests for `Range` (WHATWG DOM §5.5): constructor, boundary
//! setters, live-range tracking, compare and point queries. The stub,
//! `createRange`, `StaticRange` (§5.4), validation and coercion tests are in
//! `tests_range_stubs_validation.rs`.
//!
//! Slot `#11-traversal-and-range-pr-a2-bindings`.

#![cfg(feature = "engine")]
#![allow(unsafe_code)]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_num, eval_str};
use super::super::value::JsValue;
use super::super::Vm;

pub(super) fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

#[allow(unsafe_code)]
pub(super) unsafe fn bind(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: elidex_ecs::Entity,
) {
    unsafe { bind_vm(vm, session, dom, doc) };
}

pub(super) const TREE_SETUP: &str = "globalThis.root = document.createElement('div');\
     globalThis.t = document.createTextNode('hello');\
     root.appendChild(t);";

// ---------------------------------------------------------------------------
// Constructor + identity
// ---------------------------------------------------------------------------

#[test]
fn new_range_collapsed_at_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 0.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 0.0);
    assert_eq!(eval_str(&mut vm, "r.collapsed ? 'yes' : 'no'"), "yes");
    vm.unbind();
}

#[test]
fn range_constructor_requires_new() {
    super::assert_ctor_requires_new("Range()", "Range");
}

#[test]
fn static_range_ctor_requires_new() {
    super::assert_ctor_requires_new("StaticRange({})", "StaticRange");
}

#[test]
fn range_prototype_constants_live_on_ctor_and_prototype() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    assert_eq!(eval_num(&mut vm, "Range.START_TO_START"), 0.0);
    assert_eq!(eval_num(&mut vm, "Range.START_TO_END"), 1.0);
    assert_eq!(eval_num(&mut vm, "Range.END_TO_END"), 2.0);
    assert_eq!(eval_num(&mut vm, "Range.END_TO_START"), 3.0);
    assert_eq!(eval_num(&mut vm, "Range.prototype.START_TO_START"), 0.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Boundary setters
// ---------------------------------------------------------------------------

#[test]
fn set_start_and_end_round_trip() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4);"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 4.0);
    assert_eq!(eval_str(&mut vm, "r.collapsed ? 'yes' : 'no'"), "no");
    vm.unbind();
}

#[test]
fn select_node_contents_spans_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.selectNodeContents(t);"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 0.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 5.0);
    vm.unbind();
}

#[test]
fn collapse_to_start_and_end() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4); r.collapse();"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 4.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 4.0);
    vm.eval("r.setStart(t, 1); r.setEnd(t, 4); r.collapse(true);")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 1.0);
    vm.unbind();
}

#[test]
fn clone_range_independent_copy() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4);\
         globalThis.c = r.cloneRange();"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "c.startOffset"), 1.0);
    vm.eval("r.setStart(t, 2);").unwrap();
    assert_eq!(eval_num(&mut vm, "c.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 2.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Live-range mutation tracking
// ---------------------------------------------------------------------------

#[test]
fn range_boundary_collapses_on_full_data_replace() {
    // `t.data = 'hey'` is `setData`, which DOM §4.10 defines as
    // "replace data with node, offset 0, count node's length, data new
    // value". The "replace data" live-range steps 8-9 then collapse any
    // boundary inside the replaced span `[0, length]` to the offset (0):
    // a range at (2, 5) over "hello" → (0, 0). (Before B1.3-i, `setData`
    // routed through `set_text_data`/`TextChange`, which merely *clamped*
    // boundaries to the new length [→ (2, 3)] — not spec-faithful. B1.3-i
    // routes `setData` through `apply_replace_data`/`ReplaceData`, fixing
    // the live-range semantics to match the spec collapse.)
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 2); r.setEnd(t, 5);"
    ))
    .unwrap();
    vm.eval("t.data = 'hey';").unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 0.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 0.0);
    vm.unbind();
}

#[test]
fn range_boundary_increments_on_insert_before() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.p = document.createElement('p');\
         globalThis.a = document.createElement('a');\
         globalThis.b = document.createElement('b');\
         p.appendChild(a); p.appendChild(b);\
         globalThis.r = new Range(); r.setStart(p, 1); r.setEnd(p, 2);\
         globalThis.c = document.createElement('c');\
         p.insertBefore(c, a);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 2.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 3.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Boundary compare + point query
// ---------------------------------------------------------------------------

#[test]
fn compare_boundary_points_basic() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.a = new Range(); a.setStart(t, 1); a.setEnd(t, 3);\
         globalThis.b = new Range(); b.setStart(t, 2); b.setEnd(t, 4);"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "a.compareBoundaryPoints(0, b)"), -1.0);
    assert_eq!(eval_num(&mut vm, "a.compareBoundaryPoints(2, b)"), -1.0);
    vm.unbind();
}

#[test]
fn is_point_in_range_basic() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4);"
    ))
    .unwrap();
    assert_eq!(
        eval_str(&mut vm, "r.isPointInRange(t, 2) ? 'in' : 'out'"),
        "in"
    );
    assert_eq!(
        eval_str(&mut vm, "r.isPointInRange(t, 0) ? 'in' : 'out'"),
        "out"
    );
    assert_eq!(
        eval_str(&mut vm, "r.isPointInRange(t, 5) ? 'in' : 'out'"),
        "out"
    );
    vm.unbind();
}

#[test]
fn is_point_in_range_cross_root_returns_false_not_throw() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R4 — WHATWG §4.4 step ORDER: root check (step 1)
    // precedes doctype rejection (step 2).  A cross-tree node, even
    // if it were a doctype, must return false rather than throw.
    // Here we use a regular element from a DETACHED tree.
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4);\
         globalThis.detached = document.createElement('section');"
    ))
    .unwrap();
    let result = eval_str(&mut vm, "r.isPointInRange(detached, 0) ? 'in' : 'out'");
    assert_eq!(
        result, "out",
        "cross-root point must return false, not throw"
    );
    vm.unbind();
}

#[test]
fn compare_point_returns_neg1_zero_pos1() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4);"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.comparePoint(t, 0)"), -1.0);
    assert_eq!(eval_num(&mut vm, "r.comparePoint(t, 2)"), 0.0);
    assert_eq!(eval_num(&mut vm, "r.comparePoint(t, 5)"), 1.0);
    vm.unbind();
}

#[test]
fn detach_is_noop() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    let v = vm.eval("r.detach()").unwrap();
    assert!(matches!(v, JsValue::Undefined));
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 0.0);
    vm.unbind();
}

#[test]
fn detach_brand_check_throws_on_non_range() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R1 — WebIDL operation must reject non-Range receiver
    // even though detach is a legacy no-op.
    let res = vm.eval("Range.prototype.detach.call({});");
    assert!(
        res.is_err(),
        "detach.call(non-Range) must throw TypeError (brand check)"
    );
    vm.unbind();
}

#[test]
fn set_start_after_end_collapses_end() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R2 — WHATWG §4.4 setStart step 4: if new start is
    // after end, collapse end to (node, offset).
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 2);\
         r.setStart(t, 4);"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 4.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 4.0);
    assert_eq!(eval_str(&mut vm, "r.collapsed ? 'yes' : 'no'"), "yes");
    vm.unbind();
}

#[test]
fn set_end_before_start_collapses_start() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // WHATWG §4.4 setEnd step 4 mirror.
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 3); r.setEnd(t, 5);\
         r.setEnd(t, 1);"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 1.0);
    vm.unbind();
}

#[test]
fn set_start_without_offset_throws() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R3 — WebIDL `unsigned long offset` is REQUIRED; a
    // missing arg must throw TypeError, not silently default to 0.
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range();"
    ))
    .unwrap();
    let res = vm.eval("r.setStart(t);");
    assert!(
        res.is_err(),
        "setStart(node) without offset must throw TypeError"
    );
    vm.unbind();
}

#[test]
fn static_range_requires_start_offset_member() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R3 — `StaticRangeInit.startOffset` / `.endOffset` are
    // REQUIRED dictionary members; missing must throw TypeError.
    vm.eval(TREE_SETUP).unwrap();
    let res = vm.eval("new StaticRange({ startContainer: t, endContainer: t });");
    assert!(
        res.is_err(),
        "missing startOffset/endOffset must throw TypeError"
    );
    vm.unbind();
}

#[test]
fn clone_range_after_unbind_throws_not_panics() {
    // Copilot R7: cloneRange / compareBoundaryPoints called
    // `split_dom_and_live_ranges` outside `read_range`'s gate;
    // retained Range refs across `Vm::unbind()` must throw, not panic.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    vm.unbind();
    let res = vm.eval("r.cloneRange();");
    assert!(res.is_err(), "cloneRange after unbind must throw");
}

#[test]
fn document_create_range_brand_check_throws_on_non_document() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Copilot R2 — `document.createRange.call({})` must throw
    // TypeError because `this` is not a Document.
    let res = vm.eval("document.createRange.call(document.createElement('div'));");
    assert!(
        res.is_err(),
        "createRange.call(non-Document) must throw TypeError"
    );
    vm.unbind();
}

#[test]
fn retained_range_after_unbind_throws_not_panics() {
    // Copilot R6: retained `Range` wrapper across `Vm::unbind()`
    // must surface `InvalidStateError` rather than panic on
    // `dom_ptr` null assertion.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    vm.unbind();
    // r is retained as a JS-side global, but the registry has
    // been cleared.  Accessing it must throw, not panic.
    let res = vm.eval("r.startOffset");
    assert!(res.is_err(), "retained Range read after unbind must throw");
}

#[test]
fn delete_contents_collapses_range() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 1); r.setEnd(t, 4);\
         r.deleteContents();"
    ))
    .unwrap();
    // WHATWG §4.4 deleteContents step 3: range collapses to (start_container, start_offset).
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 1.0);
    assert_eq!(eval_str(&mut vm, "r.collapsed ? 'yes' : 'no'"), "yes");
    vm.unbind();
}

#[test]
fn to_string_concatenates_text() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.t = document.createTextNode('hello world');\
         root.appendChild(t);\
         globalThis.r = new Range(); r.setStart(t, 0); r.setEnd(t, 5);",
    )
    .unwrap();
    assert_eq!(eval_str(&mut vm, "r.toString()"), "hello");
    vm.unbind();
}
