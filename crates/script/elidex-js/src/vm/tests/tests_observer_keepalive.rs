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
use super::super::value::{JsValue, NativeContext, ObjectKind, ObserverKind, VmError};
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

/// Registry-internal row counts (mutation `records` / resize `registered` /
/// intersection `observers`) — the SECOND-HALF leak oracle: the GC sweep must
/// `retire_collected` these alongside the binding rows so no registry-side
/// residual survives a collection.
fn registry_counts(vm: &Vm) -> (usize, usize, usize) {
    let hd = vm.inner.host_data.as_deref().unwrap();
    (
        hd.mutation_observers.records_len(),
        hd.resize_observers.registered_len(),
        hd.intersection_observers.observers_len(),
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
// (d') PENDING-RECORDS × DESPAWN — a queued record survives GC + still delivers
// ===========================================================================

#[test]
fn pending_records_keep_mutation_observer_alive_across_despawn_and_still_deliver() {
    // The exact data-loss regression: observe N, queue a MutationRecord (into the
    // registry `records` queue, joining the `pending` set) WITHOUT running the
    // notify microtask, then drop the JS ref AND despawn N so its
    // `MutationObservedBy` vanishes (observation membership drops to zero). A GC
    // here must NOT collect the observer — the pending-records keepalive clause
    // keeps it alive so `deliver_pending_mutation_records` can still fire the
    // callback with the queued record. Without the clause the binding row is
    // pruned → delivery takes the records, finds no binding, and SILENTLY DROPS
    // them (the callback never fires). Analogue of the SSE §9.2.9 `es_keepalive`
    // has_queued_task clause.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(target, added));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    // Observe target, drop the only JS ref to the observer.
    vm.eval(
        "globalThis.calls = 0; globalThis.rec_count = -1; \
         (function(){ \
             var mo = new MutationObserver(function(records){ \
                 calls++; rec_count = records.length; \
             }); \
             mo.observe(target, {childList:true}); \
         })();",
    )
    .unwrap();

    // Queue a record WITHOUT delivering (the notify microtask is NOT run) — this
    // enqueues into the observer's `records` queue and joins the `pending` set.
    vm.inner
        .queue_mutation_record(&child_list_added(target, added));

    // Despawn the sole observed entity → its `MutationObservedBy` vanishes, so
    // observation membership drops to zero. The ONLY remaining anchor is the
    // queued record (pending-records clause).
    {
        let dom = vm.host_data().unwrap().dom();
        assert!(dom.destroy_entity(target));
    }
    assert!(
        elidex_api_observers::mutation::observing_observer_ids(&*vm.host_data().unwrap().dom())
            .is_empty(),
        "post-despawn there is NO active observation — only the queued record anchors the observer",
    );

    // GC — the observer MUST survive on the pending-records clause alone.
    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        1,
        "an observer with a pending undelivered record survives GC even after its sole target despawns",
    );

    // Now run the notify microtask — the callback fires with the queued record
    // (not dropped). This is the data-loss the clause prevents.
    vm.inner.deliver_pending_mutation_records();
    assert_eq!(
        vm.eval("calls").unwrap(),
        JsValue::Number(1.0),
        "the queued record is DELIVERED (callback fires) — not silently dropped",
    );
    assert_eq!(
        vm.eval("rec_count").unwrap(),
        JsValue::Number(1.0),
        "the callback receives the 1 queued MutationRecord",
    );
    vm.unbind();
}

#[test]
fn mid_delivery_gc_keeps_mutation_observer_alive_and_still_delivers() {
    // The intra-delivery use-after-collect regression (S5-3c helper temp-root
    // fix). Same setup as the pending-records test, but the GC fires MID-DELIVERY
    // instead of before it: `deliver_pending_mutation_records`'s `prepare` runs
    // `take_records` (draining the observer's queue → the pending-records
    // keepalive clause NO LONGER anchors it) and then the record-array is built
    // via `build_mutation_records_array`. The `force_gc_before_next_alloc`
    // one-shot lands a collection at that first array allocation, INSIDE the
    // build. With no active observation (target despawned) and no JS ref, only
    // the delivery helper's temp-root of `(instance, callback)` keeps the binding
    // alive across the build. Without that root, GC sweep-prunes the binding row
    // and frees its `ObjectId` slots → the copied `binding` dangles → stale
    // callback on a stale instance (use-after-collect) + lost notification.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(target, added));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    // Observe target; the callback writes to a global side channel; drop the only
    // JS ref to the observer so it has no JS root of its own.
    vm.eval(
        "globalThis.calls = 0; globalThis.rec_count = -1; \
         (function(){ \
             var mo = new MutationObserver(function(records){ \
                 calls++; rec_count = records.length; \
             }); \
             mo.observe(target, {childList:true}); \
         })();",
    )
    .unwrap();

    // Queue a record WITHOUT delivering (notify microtask not run).
    vm.inner
        .queue_mutation_record(&child_list_added(target, added));

    // Despawn the sole observed entity → its `MutationObservedBy` vanishes
    // (observation membership drops to zero). Now the ONLY anchor is the queued
    // record — and that anchor is released the instant `take_records` drains it
    // at the start of delivery.
    {
        let dom = vm.host_data().unwrap().dom();
        assert!(dom.destroy_entity(target));
    }
    assert!(
        elidex_api_observers::mutation::observing_observer_ids(&*vm.host_data().unwrap().dom())
            .is_empty(),
        "post-despawn there is NO active observation — only the queued record anchors the observer",
    );

    // Arm the one-shot GC: it fires at the NEXT `alloc_object`, which is the outer
    // array allocation inside `build_mutation_records_array` — i.e. AFTER
    // `take_records` drained the queue. So the binding is unanchored by both
    // keepalive clauses at the moment of collection; only the helper temp-root
    // saves it.
    vm.inner.force_gc_before_next_alloc = true;

    // Run the notify microtask. The callback MUST fire with the queued record —
    // proving the binding survived the mid-build GC.
    vm.inner.deliver_pending_mutation_records();
    assert_eq!(
        vm.eval("calls").unwrap(),
        JsValue::Number(1.0),
        "mid-delivery GC did NOT collect the observer — its callback still fires (no use-after-collect / lost notification)",
    );
    assert_eq!(
        vm.eval("rec_count").unwrap(),
        JsValue::Number(1.0),
        "the callback receives the 1 queued MutationRecord after the mid-build GC",
    );
    vm.unbind();
}

/// Test-only native `__disconnectPeerAndArmGc(peerObserver)`: reentrantly
/// `disconnect()`s a peer ResizeObserver (dropping its active-observation
/// membership → its RO §3.5 keepalive anchor) and arms the one-shot GC. Called
/// from an EARLIER observer's callback to reproduce the exact
/// gathered-but-undelivered-peer collection window: after this runs, the JS
/// harness nulls the peer's last JS reference and allocates `{}` (an
/// `alloc_object` that fires the armed GC) — all while still inside the earlier
/// observer's turn, BEFORE the delivery loop reaches the peer.
fn native_disconnect_peer_and_arm_gc(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(peer_id) = args.first().copied().unwrap_or(JsValue::Undefined) else {
        return Err(VmError::type_error("expected a ResizeObserver argument"));
    };
    let ObjectKind::Observer {
        kind: ObserverKind::Resize,
        observer_id,
    } = ctx.vm.get_object(peer_id).kind
    else {
        return Err(VmError::type_error("argument is not a ResizeObserver"));
    };
    // Drop the peer's sole active observation → its keepalive predicate stops
    // covering it (RO §3.5 Lifetime).
    let ro_id = elidex_api_observers::resize::ResizeObserverId::from_raw(observer_id);
    let (dom, observers) = ctx.host().split_dom_mut_and_resize_observers();
    observers.disconnect(dom, ro_id);
    // Arm the one-shot GC to fire at the NEXT `alloc_object` — the harness
    // triggers it with an object literal still inside this (earlier) callback.
    ctx.vm.force_gc_before_next_alloc = true;
    Ok(JsValue::Undefined)
}

#[test]
fn mid_delivery_gc_keeps_gathered_resize_peer_and_still_delivers() {
    // The gathered-but-undelivered PEER regression (S5-3c batch-root fix).
    // `deliver_resize_observations` pre-gathers entries for ALL size-changed
    // observers into a local map BEFORE the delivery loop. Two ROs — A (lower id,
    // delivered first) and B (higher id, gathered but not yet delivered) — both
    // observe distinct size-changed targets, so both are in the gathered batch. B
    // is UNREFERENCED from JS except via a single `globalThis.b` slot; the ONLY
    // other thing rooting it is its binding/keepalive.
    //
    // A's callback reentrantly `disconnect()`s B (dropping B's sole active
    // observation → its keepalive anchor), nulls `globalThis.b` (dropping B's last
    // JS ref), and allocates `{}` to fire the armed one-shot GC — all while still
    // inside A's turn, BEFORE the loop reaches B. At that GC B is non-observing +
    // unreferenced, so WITHOUT the batch root its binding row is sweep-pruned and
    // its `instance`/`callback` `ObjectId` slots freed; when the loop reaches B its
    // gathered entry is silently dropped (B's callback never fires) — a
    // GC-timing-dependent lost notification. The Phase-1 batch root keeps B's
    // `(instance, callback)` rooted for the whole delivery, so B still delivers.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let target_a = dom.create_element("div", elidex_ecs::Attributes::default());
    let target_b = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target_a));
    assert!(dom.append_child(body, target_b));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper_a = vm.inner.create_element_wrapper(target_a);
    let wrapper_b = vm.inner.create_element_wrapper(target_b);
    vm.set_global("target_a", JsValue::Object(wrapper_a));
    vm.set_global("target_b", JsValue::Object(wrapper_b));

    // Install the reentrant-disconnect + arm-GC test native on globalThis.
    let native = vm.inner.create_native_function(
        "__disconnectPeerAndArmGc",
        native_disconnect_peer_and_arm_gc,
    );
    vm.set_global("__disconnectPeerAndArmGc", JsValue::Object(native));

    // A observes target_a; its callback disconnects B, drops B's JS ref, and
    // allocates `{}` (firing the armed GC) — still inside A's turn. B observes
    // target_b; keep only a `globalThis.b` ref (nulled by A's callback). A is
    // constructed FIRST so it gets the lower observer id → delivered first.
    vm.eval(
        "globalThis.a_calls = 0; globalThis.b_calls = 0; \
         globalThis.a = new ResizeObserver(function(){ \
             a_calls++; \
             __disconnectPeerAndArmGc(b); \
             globalThis.b = null; \
             var _sink = {}; \
         }); \
         globalThis.b = new ResizeObserver(function(){ b_calls++; }); \
         a.observe(target_a); \
         b.observe(target_b);",
    )
    .unwrap();

    // Give both targets a LayoutBox so both are gathered (initial observation,
    // last_size = None ⇒ a change for each).
    set_layout_box(&mut vm, target_a, Rect::new(0.0, 0.0, 100.0, 50.0));
    set_layout_box(&mut vm, target_b, Rect::new(0.0, 0.0, 200.0, 60.0));

    vm.deliver_resize_observations();

    assert_eq!(
        vm.eval("a_calls").unwrap(),
        JsValue::Number(1.0),
        "A (delivered first) fires",
    );
    assert_eq!(
        vm.eval("b_calls").unwrap(),
        JsValue::Number(1.0),
        "the gathered-but-undelivered peer B still delivers after A disconnected it + dropped its \
         ref + forced a mid-batch GC — its binding was batch-rooted (no silent entry drop)",
    );
    vm.unbind();
}

#[test]
fn drained_pending_observer_with_no_observation_is_collected() {
    // Companion negative control: an observer whose queue the page DRAINED via
    // takeRecords() (records emptied) with NO active observation and no JS ref is
    // collectible — the pending-records clause keys on NON-EMPTY `records`, not
    // stale `pending` membership, so a drained observer is NOT over-kept.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    let body = body_of(&dom, doc);
    let target = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, target));
    let added = dom.create_element("span", elidex_ecs::Attributes::default());
    assert!(dom.append_child(target, added));
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(target);
    vm.set_global("target", JsValue::Object(wrapper));

    vm.eval(
        "globalThis.mo = new MutationObserver(function(){}); \
         mo.observe(target, {childList:true});",
    )
    .unwrap();
    let observer_id = {
        let hd = vm.inner.host_data.as_deref().unwrap();
        *hd.mutation_observer_bindings.keys().next().unwrap()
    };

    // Queue a record, then drain it via takeRecords() (empties `records`, leaves
    // `pending`). Then despawn the target and drop the JS ref.
    vm.inner
        .queue_mutation_record(&child_list_added(target, added));
    vm.eval("mo.takeRecords(); globalThis.mo = null;").unwrap();
    {
        let dom = vm.host_data().unwrap().dom();
        assert!(dom.destroy_entity(target));
    }
    // No observation, empty record queue → not pending, not observing.
    assert!(
        vm.inner
            .host_data
            .as_deref()
            .unwrap()
            .mutation_observers
            .observers_with_pending_records()
            .is_empty(),
        "takeRecords() drained the queue ⇒ not pending",
    );
    let _ = observer_id;

    vm.inner.collect_garbage();
    assert_eq!(
        binding_counts(&vm).0,
        0,
        "a drained (empty-queue) observer with no observation + no JS ref is COLLECTED (not over-kept on stale pending)",
    );
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
