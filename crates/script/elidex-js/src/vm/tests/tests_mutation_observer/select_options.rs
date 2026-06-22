//! B1.2b-2-select — end-to-end `MutationObserver` integration for the
//! `HTMLSelectElement` / `HTMLOptionsCollection` option tree-mutation surface:
//! `select.add`/`remove(index)` and `options.add`/`remove`/`length`-setter
//! (HTML §4.10.7 "act like" §2.6.4.3).
//!
//! These were MO-silent before this slice: the VM re-implemented the algorithm
//! and called `EcsDom` directly. The convergence routes them through the
//! engine-independent dom-api handlers (`options.add` / `options.remove` /
//! `options.length.set`) → the record-producing `apply_*` primitives → §4.3
//! microtask → callback. Here we drive a **real JS mutation** and assert the
//! delivered records (the handler-direct / boa-parity tests live in
//! `elidex-dom-api` `element::tests_select`).

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

fn string_global(vm: &mut Vm, expr: &str) -> String {
    let v = vm.eval(expr).unwrap();
    let JsValue::String(sid) = v else {
        panic!("expected string from `{expr}`, got {v:?}")
    };
    vm.inner.strings.get_utf8(sid).clone()
}

/// Bind the VM with a `<select>` appended to `root` and exposed as
/// `globalThis.sel`. The caller owns `vm`/`session`/`dom` so the unsafe bind's
/// raw pointers stay valid for the test's lifetime (mirrors `direct_tree_ops`).
fn setup_select(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom) {
    let (_doc, _root) = setup_with_root(vm, session, dom);
    vm.eval("globalThis.sel = document.createElement('select'); root.appendChild(sel);")
        .unwrap();
}

// ---------------------------------------------------------------------------
// add — fresh / move / before resolution
// ---------------------------------------------------------------------------

#[test]
fn select_add_fresh_option_delivers_one_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.opt = document.createElement('option'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.add(opt);",
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].addedNodes.length === 1 && records[0].addedNodes[0] === opt")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].target === sel").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn options_add_fresh_option_delivers_one_record() {
    // `sel.options.add(opt)` routes through the same handler as `sel.add(opt)`.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.opt = document.createElement('option'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.options.add(opt);",
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === opt").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn select_add_already_parented_option_delivers_two_records() {
    // A move (option currently a child of `root`) emits the §4.5-adopt
    // source-removal record (on root) + the destination record (on sel). A
    // subtree observer on root sees both.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.opt = document.createElement('option'); \
         root.appendChild(opt); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true, subtree:true}); \
         sel.add(opt);",
    )
    .unwrap();
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(2.0),
        "a move delivers source-removal + destination"
    );
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === opt")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval(
            "records[1].addedNodes.length === 1 && records[1].addedNodes[0] === opt \
             && records[1].target === sel"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn select_add_before_element_inserts_before_reference() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.a = document.createElement('option'); sel.add(a); \
         globalThis.records = null; \
         globalThis.b = document.createElement('option'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.add(b, a);",
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("sel.options[0] === b && sel.options[1] === a")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].nextSibling === a").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn select_add_before_index_inserts_before_indexth_option() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.a = document.createElement('option'); sel.add(a); \
         globalThis.b = document.createElement('option'); sel.add(b); \
         globalThis.c = document.createElement('option'); \
         sel.add(c, 1);", // before index 1 (== b) → [a, c, b]
    )
    .unwrap();
    assert_eq!(
        vm.eval("sel.options[0] === a && sel.options[1] === c && sel.options[2] === b")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn select_add_before_out_of_range_index_appends() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.a = document.createElement('option'); sel.add(a); \
         globalThis.b = document.createElement('option'); \
         sel.add(b, 99);", // out of range → append
    )
    .unwrap();
    assert_eq!(
        vm.eval("sel.options[0] === a && sel.options[1] === b")
            .unwrap(),
        JsValue::Boolean(true)
    );
    // Negative index also appends.
    vm.eval("globalThis.c = document.createElement('option'); sel.add(c, -1);")
        .unwrap();
    assert_eq!(
        vm.eval("sel.options[2] === c").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn select_add_optgroup_is_accepted() {
    // WebIDL union `(HTMLOptionElement or HTMLOptGroupElement)` accepts optgroup.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.og = document.createElement('optgroup'); \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.add(og);",
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === og").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// add — error mapping
// ---------------------------------------------------------------------------

#[test]
fn select_add_non_option_throws_type_error() {
    // A `<div>` is neither option nor optgroup → WebIDL union-conversion failure
    // = TypeError (NOT HierarchyRequestError, the pre-convergence behaviour).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    assert_eq!(
        string_global(
            &mut vm,
            "try { sel.add(document.createElement('div')); 'no-throw' } catch(e) { e.name }",
        ),
        "TypeError"
    );
    vm.unbind();
}

#[test]
fn select_add_ancestor_element_throws_hierarchy_request() {
    // step 1: element is an ancestor of select → HierarchyRequestError. Use an
    // optgroup containing the select (so it passes the union tag-guard yet is an
    // ancestor).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    assert_eq!(
        string_global(
            &mut vm,
            "globalThis.og = document.createElement('optgroup'); og.appendChild(sel); \
             try { sel.add(og); 'no-throw' } catch(e) { e.name }",
        ),
        "HierarchyRequestError"
    );
    vm.unbind();
}

#[test]
fn select_add_before_not_descendant_throws_not_found() {
    // step 2: before is an element not a descendant of select → NotFoundError.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    assert_eq!(
        string_global(
            &mut vm,
            "globalThis.opt = document.createElement('option'); \
             globalThis.stray = document.createElement('option'); \
             try { sel.add(opt, stray); 'no-throw' } catch(e) { e.name }",
        ),
        "NotFoundError"
    );
    vm.unbind();
}

#[test]
fn select_add_element_equals_before_is_noop() {
    // step 3: element == before → return (no insertion, no record).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.opt = document.createElement('option'); sel.add(opt); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.add(opt, opt);",
    )
    .unwrap();
    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    assert_eq!(vm.eval("sel.options.length").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn select_remove_index_delivers_one_remove_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.a = document.createElement('option'); sel.add(a); \
         globalThis.b = document.createElement('option'); sel.add(b); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.remove(0);", // remove `a`
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === a")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("sel.options.length === 1 && sel.options[0] === b")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn select_remove_out_of_range_is_noop_no_record() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.a = document.createElement('option'); sel.add(a); \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.remove(5); sel.remove(-1);",
    )
    .unwrap();
    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    assert_eq!(vm.eval("sel.options.length").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn select_remove_no_arg_detaches_the_select() {
    // HTML §4.10.7 `select.remove()` no-arg falls through to ChildNode.remove():
    // detach the select itself. Observe `root` to see the removal record.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         sel.remove();",
    )
    .unwrap();
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval(
            "records[0].removedNodes.length === 1 && records[0].removedNodes[0] === sel \
             && records[0].target === root"
        )
        .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// length setter
// ---------------------------------------------------------------------------

#[test]
fn options_length_grow_delivers_one_coalesced_record() {
    // §2.6.4.3 "append new option elements to select given count" creates a
    // DocumentFragment, appends count options to it, then appends the fragment to
    // select — ONE §4.2.3 childList insertion → ONE coalesced record with all
    // added options (the spec's fragment algorithm, NOT one record per option).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.options.length = 3;",
    )
    .unwrap();
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "fragment-append grow is one coalesced record per spec"
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(3.0)
    );
    assert_eq!(vm.eval("sel.options.length").unwrap(), JsValue::Number(3.0));
    assert_eq!(
        string_global(&mut vm, "sel.options[0].tagName.toLowerCase()"),
        "option"
    );
    vm.unbind();
}

#[test]
fn options_length_truncate_delivers_per_node_remove_records() {
    // §2.6.4.3 length-setter shrink: "remove the last n nodes from their parent
    // nodes" — per-node removal, so n distinct records (last n options, the spec
    // loops).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "sel.options.length = 4; \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.options.length = 1;",
    )
    .unwrap();
    // 3 options removed → 3 records, each removing one node.
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(3.0));
    assert_eq!(
        vm.eval("records.every(function(r){ return r.removedNodes.length === 1; })")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(vm.eval("sel.options.length").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn options_length_over_engine_cap_is_noop() {
    // The engine caps the addressable option count at MAX_ANCESTOR_DEPTH (10,000);
    // a target above it is a silent no-op (preserving the spec's >100,000 → return
    // shape), never a clamp — so length stays 0 and no record is delivered.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    setup_select(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(sel, {childList:true}); \
         sel.options.length = 10001;",
    )
    .unwrap();
    assert_eq!(vm.eval("records").unwrap(), JsValue::Null);
    assert_eq!(vm.eval("sel.options.length").unwrap(), JsValue::Number(0.0));
    vm.unbind();
}
