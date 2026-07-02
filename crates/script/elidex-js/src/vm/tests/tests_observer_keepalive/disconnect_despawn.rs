//! (c) observe-then-disconnect / unobserve → collectible, and
//! (d) the DESPAWN discriminator (D2 passes for free; D1 would need a despawn
//! hook).

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc};

// ===========================================================================
// (c) observe-then-disconnect → collectible
// ===========================================================================

#[test]
fn disconnected_mutation_observer_is_collected() {
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

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){}); \
         mo.observe(root, {childList:true}); \
         mo.disconnect(); \
         globalThis.mo = null;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "disconnect ends the only observation → the unreferenced observer is COLLECTED",
    );
    vm.unbind();
}

#[test]
fn unobserved_resize_observer_is_collected() {
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
        "globalThis.ro = new ResizeObserver(function(){}); \
         ro.observe(target); ro.unobserve(target); globalThis.ro = null;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).1,
        0,
        "unobserve of the sole target → the unreferenced RO is COLLECTED",
    );
    vm.unbind();
}

#[test]
fn unobserved_intersection_observer_is_collected() {
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
        "globalThis.io = new IntersectionObserver(function(){}, {threshold:[0]}); \
         io.observe(target); io.unobserve(target); globalThis.io = null;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).2,
        0,
        "unobserve of the sole target → the unreferenced IO is COLLECTED",
    );
    vm.unbind();
}

// ===========================================================================
// (d) DESPAWN discriminator (D2 passes for free; D1 would need a despawn hook)
// ===========================================================================

#[test]
fn despawn_of_sole_target_makes_mutation_observer_collectible() {
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
        "globalThis.mo = new MutationObserver(function(){}); \
         mo.observe(target, {childList:true}); globalThis.mo = null;",
    )
    .unwrap();
    // Despawn the sole observed entity — its `MutationObservedBy` vanishes with
    // it, dropping membership to zero with NO registry decrement hook (D2).
    {
        let dom = vm.host_data().unwrap().dom();
        assert!(dom.destroy_entity(target));
    }
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "despawn of the sole observed entity makes the observer COLLECTIBLE (D2 despawn-safe)",
    );
    vm.unbind();
}

#[test]
fn despawn_of_sole_target_makes_resize_observer_collectible() {
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
        "globalThis.ro = new ResizeObserver(function(){}); \
         ro.observe(target); globalThis.ro = null;",
    )
    .unwrap();
    {
        let dom = vm.host_data().unwrap().dom();
        assert!(dom.destroy_entity(target));
    }
    vm.inner.collect_garbage();
    assert_eq!(binding_counts(&vm).1, 0, "despawn → RO COLLECTIBLE (D2)");
    vm.unbind();
}

#[test]
fn despawn_of_sole_target_makes_intersection_observer_collectible() {
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
        "globalThis.io = new IntersectionObserver(function(){}, {threshold:[0]}); \
         io.observe(target); globalThis.io = null;",
    )
    .unwrap();
    {
        let dom = vm.host_data().unwrap().dom();
        assert!(dom.destroy_entity(target));
    }
    vm.inner.collect_garbage();
    assert_eq!(binding_counts(&vm).2, 0, "despawn → IO COLLECTIBLE (D2)");
    vm.unbind();
}
