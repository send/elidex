//! (b) observing survives + still fires (callback rooted by the predicate,
//! not JS).

use elidex_ecs::EcsDom;
use elidex_plugin::Rect;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::{bind_vm, set_layout_box};
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc, child_list_added};

// ===========================================================================
// (b) observing survives + still fires (callback rooted by the predicate, not JS)
// ===========================================================================

#[test]
fn observing_mutation_observer_survives_gc_and_delivers() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let root = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, root));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(wrapper));

    // Observe inside a scope that drops the `mo` JS ref immediately — the only
    // retention path is the keepalive predicate (it IS observing `root`).
    vm.eval(
        "globalThis.calls = 0; \
         (function(){ \
             var mo = new MutationObserver(function(){ calls++; }); \
             mo.observe(root, {childList:true}); \
         })();",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "an observing observer survives GC (binding row retained)",
    );

    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    vm.deliver_mutation_records(&[child_list_added(root, added)]);
    assert_eq!(
        vm.eval("calls").unwrap(),
        JsValue::Number(1.0),
        "the survived observer's callback (rooted by the predicate, not a JS ref) still fires",
    );
    vm.unbind();
}

#[test]
fn observing_resize_observer_survives_gc_and_delivers() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.calls = 0; \
         (function(){ \
             var ro = new ResizeObserver(function(){ calls++; }); \
             ro.observe(target); \
         })();",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(binding_counts(&vm).1, 1, "an observing RO survives GC");

    set_layout_box(&mut vm, target, Rect::new(0.0, 0.0, 100.0, 50.0));
    vm.deliver_resize_observations();
    assert_eq!(vm.eval("calls").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}

#[test]
fn observing_intersection_observer_survives_gc_and_delivers() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.calls = 0; \
         (function(){ \
             var io = new IntersectionObserver(function(){ calls++; }, {threshold:[0]}); \
             io.observe(target); \
         })();",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(binding_counts(&vm).2, 1, "an observing IO survives GC");

    set_layout_box(&mut vm, target, Rect::new(10.0, 10.0, 100.0, 100.0));
    vm.deliver_intersection_observations();
    assert_eq!(vm.eval("calls").unwrap(), JsValue::Number(1.0));
    vm.unbind();
}
