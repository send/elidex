//! S5-3c — the observer (Mutation / Resize / Intersection) GC-keepalive arm
//! (`#11-eventtarget-listener-keepalive-rooting`, the active-observation
//! predicate).
//!
//! **The oracle is FLIPPED vs S5-3a/b** (the WS/ES/MQL under-root fixes, whose
//! headline asserts a listener-held target *survives*). Observers were
//! **OVER-rooted** — the `(callback, instance)` binding was rooted at
//! construction for life (`gc_root_object_ids`), and `disconnect()` never
//! released it → immortal-until-`Vm::unbind` leak. S5-3c routes the observers
//! through the keepalive seam with the spec predicate **"has ≥1 active
//! observation"** (DOM §4.3 registered-observer-list / RO §3.5 / IO §3.3
//! Lifetime); a never-observed / disconnected unreferenced observer becomes
//! **collectible** (its binding-map row is pruned in the sweep). So the headline
//! here asserts an idle observer **IS collected**.
//!
//! Companion unit tests for the engine-indep membership query
//! (`observing_observer_ids`) live in `elidex-api-observers`.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::Rect;
use elidex_script_session::{MutationKind, MutationRecord as SessionRecord, SessionCore};

use super::super::test_helpers::{bind_vm, set_layout_box};
use super::super::value::JsValue;
use super::super::Vm;

// --- shared fixtures --------------------------------------------------------

fn build_doc(dom: &mut EcsDom) -> Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn body_of(dom: &EcsDom, doc: Entity) -> Entity {
    dom.first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap()
}

/// Counts of the three `*_observer_bindings` maps (the sweep-prune oracle).
fn binding_counts(vm: &Vm) -> (usize, usize, usize) {
    let hd = vm.inner.host_data.as_deref().unwrap();
    (
        hd.mutation_observer_bindings.len(),
        hd.resize_observer_bindings.len(),
        hd.intersection_observer_bindings.len(),
    )
}

/// A `ChildList` record adding `added` to `target`.
fn child_list_added(target: Entity, added: Entity) -> SessionRecord {
    SessionRecord {
        kind: MutationKind::ChildList,
        target,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }
}

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
