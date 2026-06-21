//! B1 — end-to-end `MutationObserver` integration: a **real JS DOM mutation**
//! (driven through the bridge / `EcsDom` chokepoint) produces a `MutationRecord`
//! that is delivered to the observer callback on the §4.3 microtask checkpoint.
//!
//! This closes the B0-named *test-invisible gap*: the [`super::delivery`] tests
//! hand-build `SessionRecord`s and call `Vm::deliver_mutation_records` directly
//! (exercising the registry gating + record→JS marshalling for **all** kinds,
//! incl. attribute / characterData which B1 does not yet produce). Here we assert
//! the new B1 wiring: `appendChild` / `insertBefore` / `removeChild` /
//! `replaceChild` / `innerHTML` → `notify` → microtask → callback.
//!
//! Scope: childList only (B1). attribute (B2) / characterData (B1.3) /
//! direct tree ops (B1.2) are intentionally not driven here.

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

/// Create a detached element of `tag` and expose its wrapper as the JS global
/// `name`, returning the entity. The element is registered in the identity map
/// (via `create_element_wrapper`) so a `DomApiHandler` can resolve its
/// `ObjectRef` back to the entity.
fn expose_detached(vm: &mut Vm, dom: &mut EcsDom, tag: &str, name: &str) -> elidex_ecs::Entity {
    let e = dom.create_element(tag, Attributes::default());
    let wrapper = vm.inner.create_element_wrapper(e);
    vm.set_global(name, JsValue::Object(wrapper));
    e
}

fn string_global(vm: &mut Vm, expr: &str) -> String {
    let v = vm.eval(expr).unwrap();
    let JsValue::String(sid) = v else {
        panic!("expected string from `{expr}`, got {v:?}")
    };
    vm.inner.strings.get_utf8(sid).clone()
}

#[test]
fn real_append_child_delivers_one_childlist_record_on_microtask() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "span", "child");

    // The `syncSeen` probe runs in the SAME eval right after `appendChild`,
    // BEFORE the microtask drains at eval-end — so it proves the callback did
    // NOT fire synchronously inside the mutation (§4.3 microtask timing).
    vm.eval(
        "globalThis.records = null; \
         globalThis.syncSeen = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.appendChild(child); \
         globalThis.syncSeen = (records === null);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("syncSeen").unwrap(),
        JsValue::Boolean(true),
        "callback must NOT fire synchronously inside appendChild"
    );
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "exactly one record delivered after the microtask checkpoint"
    );
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(1.0)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === child").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_insert_before_records_added_and_next_sibling() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    // `existing` is appended in Rust (no record) so it is the reference child.
    let existing = dom.create_element("b", Attributes::default());
    assert!(dom.append_child(root, existing));
    let existing_wrapper = vm.inner.create_element_wrapper(existing);
    vm.set_global("existing", JsValue::Object(existing_wrapper));
    expose_detached(&mut vm, &mut dom, "i", "fresh");

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.insertBefore(fresh, existing);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === fresh").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].nextSibling === existing").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_remove_child_records_removed_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let kid = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(root, kid));
    let kid_wrapper = vm.inner.create_element_wrapper(kid);
    vm.set_global("kid", JsValue::Object(kid_wrapper));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.removeChild(kid);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].removedNodes.length").unwrap(),
        JsValue::Number(1.0)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes[0] === kid").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    vm.unbind();
}

#[test]
fn real_replace_child_delivers_single_coalesced_record() {
    // §4.2.3 "replace" step 14: ONE childList record with both added + removed
    // (the inner remove/insert run with suppressObservers), NOT two records.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let old = dom.create_element("b", Attributes::default());
    assert!(dom.append_child(root, old));
    let old_wrapper = vm.inner.create_element_wrapper(old);
    vm.set_global("oldChild", JsValue::Object(old_wrapper));
    expose_detached(&mut vm, &mut dom, "i", "newChild");

    vm.eval(
        "globalThis.calls = 0; \
         globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.calls += 1; globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.replaceChild(newChild, oldChild);",
    )
    .unwrap();

    // Exactly one callback invocation, one record (coalesced), carrying both.
    assert_eq!(
        vm.eval("calls").unwrap(),
        JsValue::Number(1.0),
        "replaceChild must coalesce into a single callback"
    );
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "replaceChild must produce exactly ONE record (§4.2.3 step 14)"
    );
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === newChild").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes[0] === oldChild").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_inner_html_delivers_on_microtask_not_synchronously() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         globalThis.syncSeen = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         root.innerHTML = '<span></span>'; \
         globalThis.syncSeen = (records === null);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("syncSeen").unwrap(),
        JsValue::Boolean(true),
        "innerHTML callback must defer to the microtask, not fire in the setter"
    );
    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    vm.unbind();
}

#[test]
fn real_subtree_observer_receives_descendant_child_mutation() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    // `mid` is a child of root; the observer is on root with subtree:true.
    let mid = dom.create_element("div", Attributes::default());
    assert!(dom.append_child(root, mid));
    let mid_wrapper = vm.inner.create_element_wrapper(mid);
    vm.set_global("mid", JsValue::Object(mid_wrapper));
    expose_detached(&mut vm, &mut dom, "span", "leaf");

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true, subtree:true}); \
         mid.appendChild(leaf);",
    )
    .unwrap();

    assert_eq!(vm.eval("records.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("records[0].target === mid").unwrap(),
        JsValue::Boolean(true),
        "the record target is the mutated node, delivered to the subtree observer on the ancestor"
    );
    vm.unbind();
}

#[test]
fn real_same_parent_move_defers_record_no_malformed_delivery() {
    // Codex P2 (R1): `root.appendChild(<existing child of root>)` is a *move*.
    // `apply_append_child` snapshots previousSibling BEFORE the relink, so a
    // naive record could carry previousSibling == the moved node (malformed).
    // B1 defers move-record semantics to B1.2: the move applies but delivers NO
    // record (rather than a malformed one).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    // Two children appended in Rust (no records); `a` is first, `b` is last.
    let a = dom.create_element("a", Attributes::default());
    let b = dom.create_element("b", Attributes::default());
    assert!(dom.append_child(root, a));
    assert!(dom.append_child(root, b));
    let a_wrapper = vm.inner.create_element_wrapper(a);
    vm.set_global("a", JsValue::Object(a_wrapper));

    vm.eval(
        "globalThis.fired = false; \
         var mo = new MutationObserver(function(){ globalThis.fired = true; }); \
         mo.observe(root, {childList:true}); \
         root.appendChild(a);", // move `a` (currently first) to the end
    )
    .unwrap();

    // The move applied (read-your-writes): `a` is now the last child.
    assert_eq!(
        vm.eval("root.lastChild === a").unwrap(),
        JsValue::Boolean(true),
        "the move must apply at the chokepoint"
    );
    // …but no (malformed) record was delivered — move-record semantics = B1.2.
    assert_eq!(
        vm.eval("fired").unwrap(),
        JsValue::Boolean(false),
        "B1 must not deliver a record for a move (deferred to B1.2), never a malformed one"
    );
    vm.unbind();
}

#[test]
fn real_mutation_without_observer_delivers_nothing() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "span", "child");

    // An observer EXISTS but never called `observe()` — so the append below has
    // no interested observer: `notify` returns false, no microtask is scheduled,
    // and the callback never fires (the no-interested-observer fast path:
    // WHATWG DOM §4.3.2 "queue a mutation record" step 4 is a no-op when
    // interestedObservers is empty, so `notify` returns false → no schedule).
    vm.eval(
        "globalThis.fired = false; \
         var mo = new MutationObserver(function(){ globalThis.fired = true; }); \
         root.appendChild(child);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("fired").unwrap(),
        JsValue::Boolean(false),
        "an un-observed mutation must deliver nothing"
    );
    // The append itself still happened (read-your-writes at the chokepoint).
    assert_eq!(
        vm.eval("child.parentNode === root").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}
