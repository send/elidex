//! (f) REGISTRY-SIDE retirement — the sweep `retire_collected`s the registry
//! row too, so no engine-indep residual survives a collection.

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc, registry_counts};

// ===========================================================================
// (f) REGISTRY-SIDE retirement — the sweep `retire_collected`s the registry row
//     too, so no engine-indep residual survives a collection (the second half
//     of the leak S5-3c fixes). These FAIL if the `retire_collected` wiring is
//     removed from `gc/collect.rs` — the binding row would still prune, but the
//     registry `records`/`registered`/`observers` row would linger at 1.
// ===========================================================================

#[test]
fn collected_mutation_observer_retires_registry_row() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.mo = new MutationObserver(function(){}); globalThis.mo = null;")
        .unwrap();
    assert_eq!(
        registry_counts(&vm).0,
        1,
        "registry records row present before GC"
    );
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "binding row pruned (existing half of the fix)",
    );
    assert_eq!(
        registry_counts(&vm).0,
        0,
        "the registry-internal records row is ALSO retired by the sweep (no residual)",
    );
    vm.unbind();
}

#[test]
fn collected_resize_observer_retires_registry_row() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.ro = new ResizeObserver(function(){}); globalThis.ro = null;")
        .unwrap();
    assert_eq!(
        registry_counts(&vm).1,
        1,
        "registry registered id present before GC"
    );
    vm.inner.collect_garbage();
    assert_eq!(binding_counts(&vm).1, 0, "binding row pruned");
    assert_eq!(
        registry_counts(&vm).1,
        0,
        "the registry-internal registered id is ALSO retired by the sweep (no residual)",
    );
    vm.unbind();
}

#[test]
fn collected_intersection_observer_retires_registry_row() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.io = new IntersectionObserver(function(){}, {threshold:[0]}); \
         globalThis.io = null;",
    )
    .unwrap();
    assert_eq!(
        registry_counts(&vm).2,
        1,
        "registry observer config present before GC"
    );
    vm.inner.collect_garbage();
    assert_eq!(binding_counts(&vm).2, 0, "binding row pruned");
    assert_eq!(
        registry_counts(&vm).2,
        0,
        "the registry-internal per-observer config is ALSO retired by the sweep (no residual)",
    );
    vm.unbind();
}

#[test]
fn observing_mutation_observer_keeps_registry_row() {
    // Negative control: an OBSERVING observer is NOT pruned (binding kept), so its
    // registry row must ALSO survive — `retire_collected` fires only for pruned
    // rows, never for a live observing one.
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
        "(function(){ \
             var mo = new MutationObserver(function(){}); \
             mo.observe(root, {childList:true}); \
         })();",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "observing observer's binding kept"
    );
    assert_eq!(
        registry_counts(&vm).0,
        1,
        "an observing observer's registry row is NOT retired (retire fires only for pruned rows)",
    );
    vm.unbind();
}
