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
fn real_same_parent_move_delivers_two_records_source_then_dest() {
    // B1.2a: `root.appendChild(<existing child of root>)` is a *move*. Per WHATWG
    // DOM it adopts (source-parent removal, §4.5 step 2, NOT suppressed) then
    // inserts (§4.2.3) → TWO childList records on root: removal of `a`, then
    // insertion of `a` after `b`. The destination previousSibling is `b` =
    // root's last child captured pre-adopt (§4.2.3 insert step 6); since `a` is
    // not the last child the self-sibling case does not arise here.
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
    let b_wrapper = vm.inner.create_element_wrapper(b);
    vm.set_global("b", JsValue::Object(b_wrapper));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
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
    // Two records delivered on the microtask, in order: source-removal, then dest.
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(2.0),
        "a move delivers two records (source-removal + destination)"
    );
    // Record 0 = source-parent removal of `a` (added empty, removed = [a]).
    assert_eq!(
        vm.eval("records[0].removedNodes.length").unwrap(),
        JsValue::Number(1.0)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes[0] === a").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(0.0)
    );
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    // Record 1 = destination insertion of `a`; previousSibling = b, NOT a.
    assert_eq!(
        vm.eval("records[1].addedNodes[0] === a").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[1].previousSibling === b").unwrap(),
        JsValue::Boolean(true),
        "destination previousSibling is b (root's last child, captured pre-adopt)"
    );
    assert_eq!(
        vm.eval("records[1].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_cross_parent_move_routes_removal_to_source_insertion_to_dest() {
    // B1.2a cross-parent move: the source-removal record reaches an observer on
    // the OLD parent; the destination-insertion record reaches an observer on the
    // NEW parent — each via its own §4.3.2 inclusive-ancestor walk.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let src = dom.create_element("div", Attributes::default());
    let dst = dom.create_element("div", Attributes::default());
    let kid = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(root, src));
    assert!(dom.append_child(root, dst));
    assert!(dom.append_child(src, kid)); // src = [kid], dst = []
    let src_wrapper = vm.inner.create_element_wrapper(src);
    vm.set_global("src", JsValue::Object(src_wrapper));
    let dst_wrapper = vm.inner.create_element_wrapper(dst);
    vm.set_global("dst", JsValue::Object(dst_wrapper));
    let kid_wrapper = vm.inner.create_element_wrapper(kid);
    vm.set_global("kid", JsValue::Object(kid_wrapper));

    vm.eval(
        "globalThis.srcRec = null; globalThis.dstRec = null; \
         var moSrc = new MutationObserver(function(r){ globalThis.srcRec = r; }); \
         var moDst = new MutationObserver(function(r){ globalThis.dstRec = r; }); \
         moSrc.observe(src, {childList:true}); \
         moDst.observe(dst, {childList:true}); \
         dst.appendChild(kid);",
    )
    .unwrap();

    // Source observer: one removal record.
    assert_eq!(vm.eval("srcRec.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("srcRec[0].removedNodes[0] === kid").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("srcRec[0].target === src").unwrap(),
        JsValue::Boolean(true)
    );
    // Destination observer: one insertion record.
    assert_eq!(vm.eval("dstRec.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("dstRec[0].addedNodes[0] === kid").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("dstRec[0].target === dst").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_replace_child_move_delivers_source_removal_and_coalesced() {
    // B1.2a: `root.replaceChild(newc, oldc)` with `newc` already parented (in
    // `src`) delivers TWO records — the source-parent removal of `newc` (from the
    // adopt, NOT suppressed) to an observer on `src`, and the coalesced replace
    // record (added=newc, removed=oldc) to an observer on `root`.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    let src = dom.create_element("div", Attributes::default());
    let oldc = dom.create_element("b", Attributes::default());
    let newc = dom.create_element("i", Attributes::default());
    assert!(dom.append_child(root, src));
    assert!(dom.append_child(root, oldc)); // root = [src, oldc]
    assert!(dom.append_child(src, newc)); // src = [newc]
    let src_wrapper = vm.inner.create_element_wrapper(src);
    vm.set_global("src", JsValue::Object(src_wrapper));
    let oldc_wrapper = vm.inner.create_element_wrapper(oldc);
    vm.set_global("oldc", JsValue::Object(oldc_wrapper));
    let newc_wrapper = vm.inner.create_element_wrapper(newc);
    vm.set_global("newc", JsValue::Object(newc_wrapper));

    vm.eval(
        "globalThis.srcRec = null; globalThis.rootRec = null; \
         var moSrc = new MutationObserver(function(r){ globalThis.srcRec = r; }); \
         var moRoot = new MutationObserver(function(r){ globalThis.rootRec = r; }); \
         moSrc.observe(src, {childList:true}); \
         moRoot.observe(root, {childList:true}); \
         root.replaceChild(newc, oldc);",
    )
    .unwrap();

    // Source observer (on `src`): the adopt's removal of `newc`.
    assert_eq!(vm.eval("srcRec.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("srcRec[0].removedNodes[0] === newc").unwrap(),
        JsValue::Boolean(true)
    );
    // Root observer: the coalesced replace record (added newc, removed oldc).
    assert_eq!(vm.eval("rootRec.length").unwrap(), JsValue::Number(1.0));
    assert_eq!(
        vm.eval("rootRec[0].addedNodes[0] === newc").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("rootRec[0].removedNodes[0] === oldc").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("rootRec[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn real_document_fragment_append_expands_children_into_added_nodes() {
    // B1.2-fragment: `parent.appendChild(documentFragment)` reports the fragment's
    // CHILDREN in addedNodes (§4.2.3 insert step 1), not the fragment node. The
    // observer on `root` receives ONE destination record (target=root, addedNodes
    // = the expanded children); the §4.2.3 step-4.2 fragment record targets the
    // now-detached, empty fragment, which `root`'s observer does not see (it is not
    // an ancestor of the fragment). The fragment is emptied by the expansion.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         var frag = document.createDocumentFragment(); \
         globalThis.s1 = document.createElement('span'); \
         globalThis.s2 = document.createElement('i'); \
         frag.appendChild(s1); frag.appendChild(s2); \
         globalThis.fragEmptyBefore = (frag.childNodes.length === 2); \
         root.appendChild(frag); \
         globalThis.fragEmptyAfter = (frag.childNodes.length === 0);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("fragEmptyBefore").unwrap(),
        JsValue::Boolean(true),
        "fragment held its two children before the append"
    );
    assert_eq!(
        vm.eval("fragEmptyAfter").unwrap(),
        JsValue::Boolean(true),
        "fragment is emptied by expansion (children moved into root)"
    );
    // The observer on root sees exactly the destination record.
    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "root observer receives one destination record (fragment record targets the detached fragment)"
    );
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].target === root").unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(2.0),
        "addedNodes = the fragment's expanded children, not the fragment node"
    );
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === s1 && records[0].addedNodes[1] === s2")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval(
            "root.childNodes[root.childNodes.length-2] === s1 \
                 && root.childNodes[root.childNodes.length-1] === s2"
        )
        .unwrap(),
        JsValue::Boolean(true),
        "children were appended into root in order"
    );
    vm.unbind();
}

#[test]
fn real_document_fragment_replace_child_delivers_coalesced_expansion() {
    // B1.2-fragment + §4.2.3 "replace": `parent.replaceChild(frag, old)` removes
    // `old` and inserts the fragment's children at its slot, delivering ONE
    // coalesced record (addedNodes = expanded children, removedNodes = [old]).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, root) = setup_with_root(&mut vm, &mut session, &mut dom);
    // `old` appended in Rust (no record) so it is the replacement target.
    let old = dom.create_element("b", Attributes::default());
    assert!(dom.append_child(root, old));
    let old_wrapper = vm.inner.create_element_wrapper(old);
    vm.set_global("old", JsValue::Object(old_wrapper));

    vm.eval(
        "globalThis.records = null; \
         var mo = new MutationObserver(function(r){ globalThis.records = r; }); \
         mo.observe(root, {childList:true}); \
         var frag = document.createDocumentFragment(); \
         globalThis.a = document.createElement('span'); \
         globalThis.b = document.createElement('i'); \
         frag.appendChild(a); frag.appendChild(b); \
         root.replaceChild(frag, old);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("records.length").unwrap(),
        JsValue::Number(1.0),
        "one coalesced record on root"
    );
    assert_eq!(string_global(&mut vm, "records[0].type"), "childList");
    assert_eq!(
        vm.eval("records[0].addedNodes.length").unwrap(),
        JsValue::Number(2.0)
    );
    assert_eq!(
        vm.eval("records[0].addedNodes[0] === a && records[0].addedNodes[1] === b")
            .unwrap(),
        JsValue::Boolean(true)
    );
    assert_eq!(
        vm.eval("records[0].removedNodes.length === 1 && records[0].removedNodes[0] === old")
            .unwrap(),
        JsValue::Boolean(true)
    );
    vm.unbind();
}

#[test]
fn mutation_observer_callback_slotchange_fires_in_later_microtask() {
    // Codex P2 (R2): §4.3 "notify mutation observers" clones+empties the
    // signal-slots set (steps 4–5) BEFORE invoking observer callbacks (step 6).
    // So a slotchange signaled by an MO callback body (`slot.assign()`) must be
    // handled by a FRESH notify microtask, not fired by the current one. This
    // test drives a real childList mutation → MO callback → `slot.assign()`, and
    // asserts the slotchange listener still fires (signal not lost) after the MO
    // callback ran.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "span", "trigger");

    vm.eval(
        "globalThis.moRan = false; \
         globalThis.slotFired = 0; \
         var host = document.createElement('div'); \
         var sr = host.attachShadow({mode:'open', slotAssignment:'manual'}); \
         var slot = document.createElement('slot'); \
         sr.appendChild(slot); \
         var light = document.createElement('span'); \
         host.appendChild(light); \
         root.appendChild(host); \
         slot.addEventListener('slotchange', function(){ globalThis.slotFired += 1; }); \
         var mo = new MutationObserver(function(){ \
           globalThis.moRan = true; \
           slot.assign(light); \
         }); \
         mo.observe(root, {childList:true}); \
         root.appendChild(trigger);",
    )
    .unwrap();

    // The MO callback ran (driven by the real childList mutation), and the
    // slotchange it signaled still fired (not lost by the snapshot reorder).
    assert_eq!(vm.eval("moRan").unwrap(), JsValue::Boolean(true));
    assert_eq!(
        vm.eval("slotFired").unwrap(),
        JsValue::Number(1.0),
        "slotchange signaled from an MO callback must still fire (in a later microtask)"
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
