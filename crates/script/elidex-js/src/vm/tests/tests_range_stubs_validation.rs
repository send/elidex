//! `Range` stubs / `document.createRange` / `StaticRange` / input validation /
//! WebIDL coercion tests — split out from `tests_range.rs` (~1000-line file
//! convention). Shared helpers live in `tests_range.rs`.
//!
//! Slot `#11-traversal-and-range-pr-a2-bindings`.

#![cfg(feature = "engine")]
#![allow(unsafe_code)]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{eval_num, eval_str};
use super::super::Vm;
use super::tests_range::{bind, setup, TREE_SETUP};

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
fn insert_node_same_parent_move_offset_corrected() {
    // Copilot R21: when the inserted node is already an earlier
    // sibling in the same parent, `insert_before` implicitly
    // detaches it (Range *insert* step 9 "If node's parent is
    // non-null, then remove node"), shifting `reference_node` left by one
    // before the actual insertion.  Computing newOffset BEFORE
    // Range *insert* step 12 (pre-insert) missed this shift, so a collapsed Range's end
    // boundary was set one past the inserted node.
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval(
        "globalThis.parent_ = document.createElement('div');\
         globalThis.a = document.createElement('a');\
         globalThis.b = document.createElement('b');\
         globalThis.c = document.createElement('c');\
         parent_.appendChild(a);\
         parent_.appendChild(b);\
         parent_.appendChild(c);\
         /* Collapsed range at (parent_, 2) — the slot before c. */\
         globalThis.r = new Range();\
         r.setStart(parent_, 2); r.setEnd(parent_, 2);\
         /* Insert `b` (an earlier sibling) at the range's start. */\
         r.insertNode(b);",
    )
    .unwrap();
    // After the move: parent_ = [a, b, c].  b is at index 1, c at 2.
    // Spec §4.4 step 13: end = (parent_, newOffset) where newOffset
    // is c's post-insert index = 2.
    assert_eq!(eval_num(&mut vm, "parent_.childNodes.length"), 3.0);
    assert_eq!(eval_str(&mut vm, "parent_.childNodes[1].tagName"), "B");
    assert_eq!(eval_str(&mut vm, "parent_.childNodes[2].tagName"), "C");
    // end_offset = 2 (NOT 3 — pre-R21 bug would have been 3).
    assert_eq!(eval_num(&mut vm, "r.endOffset"), 2.0);
    vm.unbind();
}

#[test]
fn compare_boundary_points_arg_conversion_order() {
    // Copilot R20: WebIDL §3.7.6 mandates argument conversion in
    // declared order — `how` is converted first, then
    // `sourceRange`.  If the second argument is not a Range,
    // the WebIDL Range brand-check (TypeError) must fire BEFORE
    // the DOM `how`-validation step (NotSupportedError).
    let (mut vm, mut session, mut dom, doc) = setup();
    unsafe { bind(&mut vm, &mut session, &mut dom, doc) };
    vm.eval("globalThis.r = new Range();").unwrap();
    // `how = 999` is invalid AND second arg is not a Range — the
    // Range brand check on arg #2 must throw first.
    let err = vm.eval("r.compareBoundaryPoints(999, {});").unwrap_err();
    let msg = err.message;
    assert!(
        msg.contains("Range") || msg.contains("Illegal"),
        "expected Range brand-check TypeError (precedence), got: {msg}"
    );
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
