//! Transient membership — a transient-only observer survives, collectible
//! after the notify step-6.3 clear.

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc};

// ===========================================================================
// transient membership — a transient-only observer survives, collectible after clear
// ===========================================================================

#[test]
fn transient_only_observer_survives_then_collectible_after_clear() {
    // An observer whose ONLY live membership is a transient registered observer
    // (DOM §4.2.3 remove step 15) survives GC while the transient exists, and is
    // collectible after the notify step-6.3 clear (if not otherwise observing /
    // referenced). The JS subtree-removal producer that spawns transients is
    // engine-internal, so drive the transient lifecycle at the registry level,
    // dropping the permanent registration (despawn `root`) so ONLY the transient
    // on `child` anchors membership.
    use elidex_api_observers::mutation::{clear_transient_observers, MutationObserverId};

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let root = dom.create_element("div", elidex_ecs::Attributes::default());
    let child = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, root));
    assert!(dom.append_child(root, child));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let root_w = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(root_w));

    // Observe root with subtree:true, drop the JS ref.
    vm.eval(
        "(function(){ \
             var mo = new MutationObserver(function(){}); \
             mo.observe(root, {childList:true, subtree:true}); \
         })();",
    )
    .unwrap();
    let observer_id = {
        let hd = vm.inner.host_data.as_deref().unwrap();
        *hd.mutation_observer_bindings.keys().next().unwrap()
    };

    // Spawn a transient onto `child` (walks child's ancestors, copies root's
    // subtree:true registration), then despawn `root` so ONLY the transient on
    // `child` remains as membership.
    {
        let (dom, observers) = vm.host_data().unwrap().split_dom_mut_and_observers();
        observers.add_transient_observers(dom, root, &[child]);
        assert!(dom.destroy_entity(root));
    }
    // Membership is now the transient on `child` only.
    {
        let dom = &*vm.host_data().unwrap().dom();
        assert!(
            elidex_api_observers::mutation::observing_observer_ids(dom).contains(&observer_id),
            "the transient-only observer is a member while the transient exists",
        );
    }
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "a transient-only observer survives GC while the transient exists",
    );

    // Clear the transient (notify step 6.3) → no membership → collectible.
    {
        let dom = vm.host_data().unwrap().dom();
        clear_transient_observers(dom, MutationObserverId::from_raw(observer_id));
    }
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "after the transient clears, the unreferenced observer is COLLECTED",
    );
    vm.unbind();
}
