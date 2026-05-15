//! Integration tests for `Range` / `StaticRange` (WHATWG DOM §4.4 / §4.5).
//!
//! Slot `#11-traversal-and-range-pr-a2-bindings`.

#![cfg(feature = "engine")]
#![allow(unsafe_code)]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_num, eval_str};
use super::super::value::JsValue;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

#[allow(unsafe_code)]
unsafe fn bind(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom, doc: elidex_ecs::Entity) {
    unsafe { bind_vm(vm, session, dom, doc) };
}

const TREE_SETUP: &str = "globalThis.root = document.createElement('div');\
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
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    let res = vm.eval("Range();");
    assert!(res.is_err(), "calling Range without new must throw");
    vm.unbind();
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
fn range_boundary_clamps_on_text_truncate() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range(); r.setStart(t, 2); r.setEnd(t, 5);"
    ))
    .unwrap();
    vm.eval("t.data = 'hey';").unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 2.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 3.0);
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

// ---------------------------------------------------------------------------
// Stubs throw NotSupportedError
// ---------------------------------------------------------------------------

#[test]
fn clone_contents_throws_not_supported() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    let res = vm.eval("r.cloneContents();");
    assert!(
        res.is_err(),
        "cloneContents must throw NotSupportedError as a Phase-A stub"
    );
    vm.unbind();
}

#[test]
fn create_contextual_fragment_throws_not_supported() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    let res = vm.eval("r.createContextualFragment('<b>x</b>');");
    assert!(res.is_err());
    vm.unbind();
}

// ---------------------------------------------------------------------------
// document.createRange
// ---------------------------------------------------------------------------

#[test]
fn document_create_range_returns_live_range() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = document.createRange();").unwrap();
    assert_eq!(eval_str(&mut vm, "r.collapsed ? 'yes' : 'no'"), "yes");
    assert_eq!(
        eval_str(&mut vm, "(r instanceof Range) ? 'yes' : 'no'"),
        "yes"
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// StaticRange — eager validation + lazy isValid
// ---------------------------------------------------------------------------

#[test]
fn static_range_holds_fields() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.sr = new StaticRange({{\
            startContainer: t, startOffset: 1, \
            endContainer: t, endOffset: 4\
         }});"
    ))
    .unwrap();
    assert_eq!(eval_num(&mut vm, "sr.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "sr.endOffset"), 4.0);
    assert_eq!(eval_str(&mut vm, "sr.collapsed ? 'yes' : 'no'"), "no");
    assert_eq!(eval_str(&mut vm, "sr.isValid() ? 'yes' : 'no'"), "yes");
    vm.unbind();
}

#[test]
fn static_range_is_valid_returns_false_when_offset_overflows() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.sr = new StaticRange({{\
            startContainer: t, startOffset: 0, \
            endContainer: t, endOffset: 99\
         }});"
    ))
    .unwrap();
    assert_eq!(eval_str(&mut vm, "sr.isValid() ? 'yes' : 'no'"), "no");
    vm.unbind();
}

#[test]
fn static_range_does_not_track_mutations() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.sr = new StaticRange({{\
            startContainer: t, startOffset: 2, \
            endContainer: t, endOffset: 5\
         }});"
    ))
    .unwrap();
    vm.eval("t.data = 'he';").unwrap();
    assert_eq!(eval_num(&mut vm, "sr.startOffset"), 2.0);
    assert_eq!(eval_num(&mut vm, "sr.endOffset"), 5.0);
    assert_eq!(eval_str(&mut vm, "sr.isValid() ? 'yes' : 'no'"), "no");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Spec-mandated input validation throws
// ---------------------------------------------------------------------------

#[test]
fn set_start_throws_on_oversize_offset() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.r = new Range();"
    ))
    .unwrap();
    // WHATWG §4.4 step 2 — offset > length must throw IndexSizeError.
    // Text 'hello' length is 5; offset 99 must throw.
    let res = vm.eval("r.setStart(t, 99);");
    assert!(res.is_err(), "setStart(t, 99) must throw IndexSizeError");
    vm.unbind();
}

#[test]
fn select_node_throws_on_parentless() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    // Standalone node, never appended.
    vm.eval("globalThis.orphan = document.createElement('p'); globalThis.r = new Range();")
        .unwrap();
    let res = vm.eval("r.selectNode(orphan);");
    assert!(
        res.is_err(),
        "selectNode on parentless node must throw InvalidNodeTypeError"
    );
    vm.unbind();
}

#[test]
fn set_start_before_throws_on_parentless() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.orphan = document.createElement('p'); globalThis.r = new Range();")
        .unwrap();
    let res = vm.eval("r.setStartBefore(orphan);");
    assert!(res.is_err());
    vm.unbind();
}

#[test]
fn compare_boundary_points_throws_on_invalid_how() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(&format!(
        "{TREE_SETUP}\
         globalThis.a = new Range(); a.setStart(t, 0); a.setEnd(t, 5);\
         globalThis.b = new Range(); b.setStart(t, 0); b.setEnd(t, 5);"
    ))
    .unwrap();
    // `how` = 99 is not one of the 4 spec constants — NotSupportedError.
    let res = vm.eval("a.compareBoundaryPoints(99, b);");
    assert!(res.is_err());
    vm.unbind();
}

// ---------------------------------------------------------------------------
// WebIDL coercion
// ---------------------------------------------------------------------------

#[test]
fn set_start_offset_coerces_via_to_uint32() {
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.t = document.createTextNode('hello');\
         root.appendChild(t);\
         globalThis.r = new Range();\
         /* Fractional offset 2.7 truncates to 2 (in bounds). */\
         r.setStart(t, 2.7);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 2.0);
    // Negative offset wraps to 2^32 - 1 via ToUint32, which then
    // exceeds the length-5 text → spec mandates IndexSizeError.
    let res = vm.eval("r.setStart(t, -1);");
    assert!(
        res.is_err(),
        "ToUint32(-1) = 4294967295 > 5 must throw IndexSizeError"
    );
    vm.unbind();
}

#[test]
fn arg_node_after_unbind_throws_not_panics() {
    // Copilot R12: `arg_node` → `require_node_arg` dereferences
    // `ctx.host().dom()` for brand check; a retained Range wrapper
    // across `Vm::unbind()` must surface the detached-range error
    // instead of panicking on any node-taking method.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.r = new Range();\
         globalThis.t = document.createTextNode('hello');",
    )
    .unwrap();
    vm.unbind();
    let res = vm.eval("r.setStart(t, 0);");
    assert!(
        res.is_err(),
        "setStart on retained Range after unbind must throw, not panic"
    );
}

#[test]
fn insert_node_failure_leaves_range_untouched() {
    // Copilot R12: when DOM insertion fails (e.g. cycle —
    // inserting an ancestor into a descendant), the Range
    // boundaries must remain at the pre-call values and the
    // method must throw a HierarchyRequestError instead of
    // committing the would-be collapsed-range expansion.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.child = document.createElement('span');\
         root.appendChild(child);\
         globalThis.r = new Range();\
         r.setStart(child, 0); r.setEnd(child, 0);",
    )
    .unwrap();
    // Inserting `root` (an ancestor of `child`) into `child` would
    // create a cycle — `is_ancestor_or_self` short-circuits the
    // pre-insert check; no `document.body` attachment needed.
    let res = vm.eval("r.insertNode(root);");
    assert!(res.is_err(), "insertNode(ancestor) must throw");
    // Boundaries unchanged.
    assert_eq!(
        eval_str(&mut vm, "r.startContainer === child ? 'yes' : 'no'"),
        "yes"
    );
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 0.0);
    assert_eq!(
        eval_str(&mut vm, "r.endContainer === child ? 'yes' : 'no'"),
        "yes"
    );
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 0.0);
    vm.unbind();
}

#[test]
fn insert_node_rejection_does_not_split_text() {
    // Copilot R13 (#1): rejection of `insertNode` (cycle / orphan
    // parent) must run BEFORE the text-node split, so a failed
    // call leaves the DOM untouched (no dangling tail Text node).
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.t = document.createTextNode('hello');\
         root.appendChild(t);\
         globalThis.r = new Range();\
         r.setStart(t, 2); r.setEnd(t, 2);\
         /* Insert root (an ancestor of t) into the range — cycle. */",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "root.childNodes.length"), 1.0);
    let res = vm.eval("r.insertNode(root);");
    assert!(res.is_err(), "cycle insertNode must throw");
    // Text node still has its original data; no split happened.
    assert_eq!(eval_str(&mut vm, "t.data"), "hello");
    assert_eq!(eval_num(&mut vm, "root.childNodes.length"), 1.0);
    vm.unbind();
}

#[test]
fn insert_node_non_collapsed_preserves_hook_adjustments() {
    // Copilot R13 (#2): for a non-collapsed range, `insertNode`
    // must NOT commit a stale snapshot over the hook-adjusted
    // live-range entry.  WHATWG §5.10 splitText migrates the
    // boundaries past the split offset to the new tail node; if
    // VM-side commit-back overwrote that, the end would point
    // back at the now-truncated head Text node.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.t = document.createTextNode('hello');\
         root.appendChild(t);\
         globalThis.r = new Range();\
         /* Non-collapsed: start at offset 1, end at offset 4. */\
         r.setStart(t, 1); r.setEnd(t, 4);\
         globalThis.elem = document.createElement('b');\
         r.insertNode(elem);",
    )
    .unwrap();
    // After insertNode at start (1): t splits into head='h' (length 1) and
    // tail='ello' (length 4). §5.10 migrates the boundary at (t, 4) — past
    // the split — to (tail, 3).  The start at (t, 1) is == split offset
    // so it stays at (t, 1) on the truncated head (length 1).
    assert_eq!(eval_str(&mut vm, "r.endContainer.data"), "ello");
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 3.0);
    vm.unbind();
}

#[test]
fn insert_node_document_fragment_fans_out_children() {
    // Copilot R14 (#2): WHATWG §4.4 step 11 — for a DocumentFragment
    // `node`, newOffset advances by the fragment's child count (not
    // by 1).  The §4.2.3 `insert` algorithm pre-inserts each child
    // and empties the fragment.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.r = new Range();\
         /* Collapsed at (root, 0). */\
         r.setStart(root, 0); r.setEnd(root, 0);\
         globalThis.frag = document.createDocumentFragment();\
         frag.appendChild(document.createElement('a'));\
         frag.appendChild(document.createElement('b'));\
         frag.appendChild(document.createElement('c'));\
         r.insertNode(frag);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "root.childNodes.length"), 3.0);
    assert_eq!(eval_num(&mut vm, "frag.childNodes.length"), 0.0);
    assert_eq!(eval_str(&mut vm, "root.childNodes[0].tagName"), "A");
    assert_eq!(eval_str(&mut vm, "root.childNodes[1].tagName"), "B");
    assert_eq!(eval_str(&mut vm, "root.childNodes[2].tagName"), "C");
    // Collapsed pre-call → step 13 sets end = (root, fragment.length = 3).
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 0.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 3.0);
    vm.unbind();
}

#[test]
fn insert_node_empty_document_fragment_no_offset_bump() {
    // Copilot R14 (#2): an empty DocumentFragment must produce a +0
    // newOffset increment (spec: "node's length" = 0 for an empty
    // fragment).
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.root = document.createElement('div');\
         root.appendChild(document.createElement('x'));\
         root.appendChild(document.createElement('y'));\
         globalThis.r = new Range();\
         r.setStart(root, 1); r.setEnd(root, 1);\
         globalThis.frag = document.createDocumentFragment();\
         r.insertNode(frag);",
    )
    .unwrap();
    assert_eq!(eval_num(&mut vm, "root.childNodes.length"), 2.0);
    assert_eq!(eval_num(&mut vm, "r.startOffset"), 1.0);
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 1.0);
    vm.unbind();
}

#[test]
fn insert_node_fragment_into_itself_throws() {
    // Copilot R16: inserting a DocumentFragment into a Range whose
    // start container is that same fragment must trigger the
    // host-including-inclusive-ancestor check on the ORIGINAL
    // `node` argument — not just on its fanned-out children.
    // An empty fragment would otherwise pass with `nodes == []`
    // and silently succeed as a no-op.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.frag = document.createDocumentFragment();\
         globalThis.r = new Range();\
         r.setStart(frag, 0); r.setEnd(frag, 0);",
    )
    .unwrap();
    // Empty fragment into itself — must throw HierarchyRequestError.
    let res = vm.eval("r.insertNode(frag);");
    assert!(
        res.is_err(),
        "insertNode of fragment into itself must throw (cycle)"
    );
    // Fragment-with-children into itself — must also throw.
    vm.eval("frag.appendChild(document.createElement('x'));")
        .unwrap();
    let res2 = vm.eval("r.insertNode(frag);");
    assert!(
        res2.is_err(),
        "insertNode of non-empty fragment into itself must throw"
    );
    vm.unbind();
}

#[test]
fn prototypes_survive_gc_after_global_removal() {
    // Copilot R14 (#1): even after `delete globalThis.{Range,
    // StaticRange, TreeWalker, NodeIterator}`, `VmInner::*_prototype`
    // must keep the intrinsic alive across a forced GC, so the
    // next document factory call binds its wrapper to the live
    // prototype slot instead of a recycled `ObjectId` of an
    // unrelated type.  Without rooting the prototype here, the
    // post-GC `document.createRange()` would see a stale wrapper
    // whose `prototype: Some(<recycled>)` no longer matches the
    // expected interface chain.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "delete globalThis.Range;\
         delete globalThis.StaticRange;\
         delete globalThis.TreeWalker;\
         delete globalThis.NodeIterator;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    // Newly-allocated wrappers should still get the well-formed
    // prototype chain.  cloneRange / createTreeWalker construct
    // through the VM-stashed prototype IDs, so a swept prototype
    // would surface as a non-Object prototype or a missing method.
    vm.eval(
        "globalThis.root = document.createElement('div');\
         globalThis.r = document.createRange();\
         globalThis.r2 = r.cloneRange();\
         globalThis.w = document.createTreeWalker(\
             root, 0xFFFFFFFF, null);\
         globalThis.it = document.createNodeIterator(\
             root, 0xFFFFFFFF, null);",
    )
    .unwrap();
    // Method dispatch via the prototype chain must succeed.
    let _ = vm.eval("r.cloneRange();").unwrap();
    let _ = vm.eval("w.firstChild();").unwrap();
    let _ = vm.eval("it.nextNode();").unwrap();
    vm.unbind();
}
