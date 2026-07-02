//! (e) UNBOUND-GC keep-all + rebind-resume (the F1 fail-safe) and its
//! idle-collected-at-bound-GC companion.

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc, child_list_added};

// ===========================================================================
// (e) UNBOUND-GC keep-all + rebind-resume (the F1 fail-safe)
// ===========================================================================

#[test]
fn observing_mutation_observer_survives_unbound_gc_and_resumes_after_rebind() {
    // An OBSERVING but UNREFERENCED observer must survive an UNBOUND GC (the
    // World is unreadable, so keep-all fail-safe) and RESUME delivery after
    // rebind. Skipping-to-collect here would prune a still-observing observer's
    // binding = a NEW under-root regression.
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
        "globalThis.calls = 0; \
         (function(){ \
             var mo = new MutationObserver(function(){ calls++; }); \
             mo.observe(root, {childList:true}); \
         })();",
    )
    .unwrap();

    // Unbind, then GC WHILE UNBOUND — the binding row must be RETAINED.
    vm.unbind();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "an observing observer's binding must be RETAINED across an unbound GC (keep-all fail-safe)",
    );

    // Rebind the SAME document → mutate → deliver → the callback fires.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    vm.deliver_mutation_records(&[child_list_added(root, added)]);
    assert_eq!(
        vm.eval("calls").unwrap(),
        JsValue::Number(1.0),
        "delivery RESUMES post-rebind (the survived binding still fires)",
    );
    vm.unbind();
}

#[test]
fn idle_unreferenced_observer_is_collected_at_bound_gc() {
    // Companion to (e): the unbound keep-all is TRANSIENT — an IDLE unreferenced
    // observer IS collected at the next BOUND GC (leak fix preserved). Proves the
    // fix neither under-roots (e) nor fails to collect idles.
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

    // GC while unbound keeps it (keep-all fail-safe).
    vm.unbind();
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "unbound GC keeps ALL bindings (fail-safe), even an idle one",
    );

    // GC while bound collects the idle unreferenced observer.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "the next BOUND GC collects the idle unreferenced observer (self-correcting)",
    );
    vm.unbind();
}
