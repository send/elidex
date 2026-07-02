//! Mid-delivery GC — the intra-delivery use-after-collect (temp-root) fix and
//! the gathered-but-undelivered resize-peer (batch-root) fix.

use elidex_ecs::EcsDom;
use elidex_plugin::Rect;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::{bind_vm, set_layout_box};
use super::super::super::value::{JsValue, NativeContext, ObjectKind, ObserverKind, VmError};
use super::super::super::Vm;
use super::{body_of, build_doc, child_list_added};

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
