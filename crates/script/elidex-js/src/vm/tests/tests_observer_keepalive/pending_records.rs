//! (d') PENDING-RECORDS × DESPAWN — a queued record survives GC + still
//! delivers, plus the drained-queue (takeRecords) negative control.

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{binding_counts, body_of, build_doc, child_list_added};

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
