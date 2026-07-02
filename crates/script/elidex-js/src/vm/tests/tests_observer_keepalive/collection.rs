//! (a) HEADLINE — a never-observed unreferenced observer IS collected (the
//! flip), the JS-referenced-idle negative control (no over-collection), and
//! the binding-row prune-by-id correctness oracle.

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc, child_list_added};

// ===========================================================================
// (a) HEADLINE — a never-observed unreferenced observer IS collected (the flip)
// ===========================================================================

#[test]
fn never_observed_mutation_observer_is_collected() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Construct with NO observe + drop the only reference.
    vm.eval("globalThis.mo = new MutationObserver(function(){}); globalThis.mo = null;")
        .unwrap();
    assert_eq!(binding_counts(&vm).0, 1, "binding present before GC");
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "a never-observed unreferenced MutationObserver must be COLLECTED (row pruned) — the over-root/leak fix",
    );
    vm.unbind();
}

#[test]
fn never_observed_resize_observer_is_collected() {
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
    assert_eq!(binding_counts(&vm).1, 1);
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).1,
        0,
        "a never-observed unreferenced ResizeObserver must be COLLECTED",
    );
    vm.unbind();
}

#[test]
fn never_observed_intersection_observer_is_collected() {
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
    assert_eq!(binding_counts(&vm).2, 1);
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).2,
        0,
        "a never-observed unreferenced IntersectionObserver must be COLLECTED",
    );
    vm.unbind();
}

// ===========================================================================
// NEGATIVE CONTROL — a JS-referenced idle observer SURVIVES (no over-collection)
// ===========================================================================

#[test]
fn js_referenced_idle_observer_survives_and_can_observe_later() {
    // The predicate is ADDITIVE (RO §3.5 / IO §3.3 "no scripting references"
    // clause): a JS-referenced observer with ZERO observations survives via its
    // JS root, and a subsequent `observe()` works + delivers. Guards the
    // over-root fix from over-shooting into under-rooting.
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

    // Constructed + retained on globalThis, but NOT observing anything.
    vm.eval("globalThis.calls = 0; globalThis.mo = new MutationObserver(function(){ calls++; });")
        .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "a JS-referenced idle observer SURVIVES via its JS root (predicate is additive)",
    );

    // A later observe() on the retained observer works + delivers (the row was
    // kept because `instance` was JS-marked, so the binding is still there).
    vm.eval("mo.observe(root, {childList:true});").unwrap();
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    vm.deliver_mutation_records(&[child_list_added(root, added)]);
    assert_eq!(
        vm.eval("calls").unwrap(),
        JsValue::Number(1.0),
        "the retained observer's later observe() delivers",
    );
    vm.unbind();
}

// ===========================================================================
// binding-row prune correctness — the specific collected row is absent
// ===========================================================================

#[test]
fn collected_observer_binding_row_is_absent_by_id() {
    // The §4.3 deliverable: after collection the specific `*_observer_bindings`
    // ROW for the collected observer's id is ABSENT (not just the ObjectIds
    // unrooted) — the binding STRUCT is reclaimed, no dangling `instance`.
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
    let observer_id = {
        let hd = vm.inner.host_data.as_deref().unwrap();
        *hd.mutation_observer_bindings.keys().next().unwrap()
    };
    vm.inner.collect_garbage();
    assert!(
        !vm.inner
            .host_data
            .as_deref()
            .unwrap()
            .mutation_observer_bindings
            .contains_key(&observer_id),
        "the collected observer's specific binding row is pruned",
    );
    vm.unbind();
}
