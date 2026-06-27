//! B1.2d-i — **transient registered observers** (WHATWG DOM §4.3 / §4.2.3
//! "remove" step 15) end-to-end through real JS DOM mutations.
//!
//! A node removed from a subtree observed by an ancestor's `subtree:true`
//! observer keeps delivering records for mutations inside the now-detached
//! subtree until the next microtask delivery clears the transient (§4.3 "notify
//! mutation observers" step 6.3). These tests drive the full path: real
//! `removeChild` / move-adopt → `notify_one` creation hook → microtask delivery
//! → per-observer clear.
//!
//! Companion to [`super::integration`] (the B1 childList delivery wiring).

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::setup_with_root;

/// Create a detached element of `tag` and expose its wrapper as the JS global
/// `name`, returning the entity (registered in the identity map so the bridge
/// can resolve it back).
fn expose_detached(vm: &mut Vm, dom: &mut EcsDom, tag: &str, name: &str) -> elidex_ecs::Entity {
    let e = dom.create_element(tag, Attributes::default());
    let wrapper = vm.inner.create_element_wrapper(e);
    vm.set_global(name, JsValue::Object(wrapper));
    e
}

#[test]
fn transient_delivers_detached_subtree_mutation_then_is_cleared() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "div", "mid");
    expose_detached(&mut vm, &mut dom, "span", "child");
    expose_detached(&mut vm, &mut dom, "span", "child2");

    // `midHits` counts delivered records whose target is `mid` — i.e. mutations
    // inside the detached subtree that only a transient observer can carry.
    // All four mutations below run in ONE eval (one microtask window), so the
    // transient created by `removeChild(mid)` is still live when
    // `mid.appendChild(child)` runs.
    vm.eval(
        "globalThis.midHits = 0; \
         var mo = new MutationObserver(function(r){ \
           for (var i = 0; i < r.length; i++) { if (r[i].target === mid) globalThis.midHits++; } \
         }); \
         mo.observe(root, {childList:true, subtree:true}); \
         root.appendChild(mid); \
         root.removeChild(mid); \
         mid.appendChild(child);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("midHits").unwrap(),
        JsValue::Number(1.0),
        "the detached-subtree mutation reached the ancestor observer via a transient"
    );

    // Next microtask window: the transient was cleared at the previous delivery
    // (step 6.3), and there is no re-observe — a further detached-subtree
    // mutation reaches no observer.
    vm.eval("mid.appendChild(child2);").unwrap();
    assert_eq!(
        vm.eval("midHits").unwrap(),
        JsValue::Number(1.0),
        "after the transient is cleared, the detached subtree is no longer observed"
    );
    vm.unbind();
}

#[test]
fn move_adopt_creates_transient_on_moved_node() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "div", "mid");
    expose_detached(&mut vm, &mut dom, "div", "otherParent");
    expose_detached(&mut vm, &mut dom, "span", "child");

    // `mid` starts under the observed `root`, then is adopted by an unobserved
    // parent (move). The move's source-parent removal (target = old parent
    // `root`, §4.5 adopt) creates a transient on `mid`, so the subsequent
    // mutation inside `mid` still reaches `root`'s subtree observer.
    vm.eval(
        "globalThis.midHits = 0; \
         var mo = new MutationObserver(function(r){ \
           for (var i = 0; i < r.length; i++) { if (r[i].target === mid) globalThis.midHits++; } \
         }); \
         mo.observe(root, {childList:true, subtree:true}); \
         root.appendChild(mid); \
         otherParent.appendChild(mid); \
         mid.appendChild(child);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("midHits").unwrap(),
        JsValue::Number(1.0),
        "move-adopt source removal created a transient that delivered the moved node's mutation"
    );
    vm.unbind();
}

#[test]
fn take_records_before_microtask_still_clears_transient() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "div", "mid");
    expose_detached(&mut vm, &mut dom, "span", "child");
    expose_detached(&mut vm, &mut dom, "span", "child2");

    // Within one microtask window: remove `mid` (transient created), mutate the
    // detached subtree (one record for the observer via the transient), then call
    // `mo.takeRecords()` — which drains the record queue but must NOT drop the
    // observer from the pending notifySet, so the microtask still clears the
    // transient (step 6.3).
    vm.eval(
        "globalThis.midHits = 0; \
         var mo = new MutationObserver(function(r){ \
           for (var i = 0; i < r.length; i++) { if (r[i].target === mid) globalThis.midHits++; } \
         }); \
         mo.observe(root, {childList:true, subtree:true}); \
         root.appendChild(mid); \
         root.removeChild(mid); \
         mid.appendChild(child); \
         var taken = mo.takeRecords(); \
         globalThis.takenMid = 0; \
         for (var i = 0; i < taken.length; i++) { if (taken[i].target === mid) globalThis.takenMid++; }",
    )
    .unwrap();

    assert_eq!(
        vm.eval("takenMid").unwrap(),
        JsValue::Number(1.0),
        "the detached-subtree mutation was observed via the transient and taken synchronously"
    );

    // Next microtask window: the transient must have been cleared despite the
    // takeRecords() drain, so a further detached-subtree mutation reaches nothing.
    vm.eval("mid.appendChild(child2);").unwrap();
    assert_eq!(
        vm.eval("midHits").unwrap(),
        JsValue::Number(0.0),
        "transient cleared at the microtask even though takeRecords() drained the queue"
    );
    vm.unbind();
}

#[test]
fn reobserve_clears_outstanding_transient() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (_doc, _root) = setup_with_root(&mut vm, &mut session, &mut dom);
    expose_detached(&mut vm, &mut dom, "div", "mid");
    expose_detached(&mut vm, &mut dom, "span", "child");

    // Within one microtask window: remove `mid` (creates a transient), then
    // re-observe `root` — §4.3.1 observe step 7.1 clears the observer's
    // outstanding transients, so the detached-subtree mutation that follows is
    // no longer carried.
    vm.eval(
        "globalThis.midHits = 0; \
         var mo = new MutationObserver(function(r){ \
           for (var i = 0; i < r.length; i++) { if (r[i].target === mid) globalThis.midHits++; } \
         }); \
         mo.observe(root, {childList:true, subtree:true}); \
         root.appendChild(mid); \
         root.removeChild(mid); \
         mo.observe(root, {childList:true, subtree:true}); \
         mid.appendChild(child);",
    )
    .unwrap();

    assert_eq!(
        vm.eval("midHits").unwrap(),
        JsValue::Number(0.0),
        "re-observe cleared the transient before the detached-subtree mutation"
    );
    vm.unbind();
}
