//! `slotchange` microtask delivery + lifecycle (unbind clear of
//! wrappers / signals / pending microtask) + R-loop regression
//! tests for the safety / spec edges Copilot surfaced.  See
//! [`super`] for shared helpers.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::super::test_helpers::bind_vm;
use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{build_doc, MANUAL_SLOT_PRELUDE};

/// Run `setup_and_signal` (which must call `slot.assign(...)` and
/// install a listener bumping `globalThis.fired`), then read
/// `globalThis.fired` from a SECOND eval so the post-eval microtask
/// drain has a chance to dispatch.  Returns the observed counter.
fn fired_count_after_eval_boundary(setup_and_signal: &str) -> f64 {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let script = format!("globalThis.fired = 0; {MANUAL_SLOT_PRELUDE}{setup_and_signal}");
    vm.eval(&script).unwrap();
    let count = vm.eval("globalThis.fired").unwrap();
    vm.unbind();
    let JsValue::Number(n) = count else {
        panic!("expected number, got {count:?}");
    };
    n
}

#[test]
fn slotchange_fired_state_observable_after_eval_boundary() {
    // First eval signals slot + returns; post-eval `drain_microtasks`
    // fires slotchange.  Second eval reads `globalThis.fired`.
    let n = fired_count_after_eval_boundary(
        "globalThis.slot.addEventListener('slotchange', function () { globalThis.fired += 1; }); \
         globalThis.slot.assign(globalThis.child);",
    );
    assert!(
        (n - 1.0).abs() < f64::EPSILON,
        "expected slotchange to fire once, got {n}"
    );
}

#[test]
fn slotchange_dedup_per_drain() {
    // Multiple `slot.assign()` calls before the microtask checkpoint
    // collapse to a single `slotchange` per signal-slots set
    // membership rule (no duplicate entries).
    let n = fired_count_after_eval_boundary(
        "var c2 = document.createElement('span'); globalThis.host.appendChild(c2); \
         globalThis.slot.addEventListener('slotchange', function () { globalThis.fired += 1; }); \
         globalThis.slot.assign(globalThis.child); globalThis.slot.assign(c2);",
    );
    assert!(
        (n - 1.0).abs() < f64::EPSILON,
        "expected exactly one slotchange across two assigns, got {n}"
    );
}

#[test]
fn slotchange_not_fired_when_assign_validation_fails() {
    // Named-mode shadow root → `EcsDom::slot_assign` returns
    // `NotManualMode`, no signal is queued, no event fires.  Cannot
    // reuse `MANUAL_SLOT_PRELUDE` since this test needs a Named-mode
    // shadow; inline the setup.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.fired = 0; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); \
         host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.addEventListener('slotchange', function () { globalThis.fired += 1; }); \
         slot.assign(c);",
    )
    .unwrap();
    let count = vm.eval("globalThis.fired").unwrap();
    let JsValue::Number(n) = count else {
        panic!("expected number, got {count:?}");
    };
    assert_eq!(n, 0.0);
    vm.unbind();
}

#[test]
fn slotchange_listener_promise_then_runs_in_same_checkpoint() {
    // R1 finding #3: a microtask queued by a slotchange listener
    // body must run within the same `drain_microtasks` pass, not
    // be deferred to the next checkpoint.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.slot_fired = 0; globalThis.then_fired = 0; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); \
         host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); \
         sr.append(slot); \
         slot.addEventListener('slotchange', function () { \
             globalThis.slot_fired += 1; \
             Promise.resolve().then(function () { globalThis.then_fired += 1; }); \
         }); \
         slot.assign(c);",
    )
    .unwrap();
    let slot_fired = vm.eval("globalThis.slot_fired").unwrap();
    let then_fired = vm.eval("globalThis.then_fired").unwrap();
    vm.unbind();
    let JsValue::Number(s) = slot_fired else {
        panic!("expected number, got {slot_fired:?}");
    };
    let JsValue::Number(t) = then_fired else {
        panic!("expected number, got {then_fired:?}");
    };
    assert_eq!(s, 1.0, "slotchange should fire once");
    assert_eq!(
        t, 1.0,
        "then() callback queued by listener should run in same checkpoint"
    );
}

#[test]
fn slotchange_signal_during_dispatch_runs_in_same_drain() {
    // Per WHATWG DOM §4.3.4: each "notify mutation observers"
    // microtask snapshots the signal-slots set before dispatching
    // its slotchange events.  A `slot.assign()` from inside a
    // listener body re-arms the coalescing flag and enqueues a NEW
    // `NotifyMutationObservers` microtask in the same drain pass —
    // so both slot1 and slot2 fire within the same eval boundary,
    // each through its own microtask checkpoint.  Earlier impl
    // (R1 snapshot-only at drain tail) incorrectly deferred slot2
    // to the next eval; R5 microtask-queue-ordering fix made the
    // spec-correct behavior observable.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.fired = []; globalThis.reentered = false; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c1 = document.createElement('span'); host.appendChild(c1); \
         var c2 = document.createElement('span'); host.appendChild(c2); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         globalThis.slot1 = document.createElement('slot'); \
         globalThis.slot2 = document.createElement('slot'); \
         sr.append(globalThis.slot1); sr.append(globalThis.slot2); \
         globalThis.slot1.addEventListener('slotchange', function () { \
             globalThis.fired.push('s1'); \
             if (!globalThis.reentered) { \
                 globalThis.reentered = true; \
                 globalThis.slot2.assign(c2); \
             } \
         }); \
         globalThis.slot2.addEventListener('slotchange', function () { \
             globalThis.fired.push('s2'); \
         }); \
         globalThis.slot1.assign(c1);",
    )
    .unwrap();
    let observed = vm.eval("globalThis.fired.join(',')").unwrap();
    vm.unbind();
    let JsValue::String(sid) = observed else {
        panic!("expected string, got {observed:?}");
    };
    let s = vm.inner.strings.get_utf8(sid);
    assert_eq!(
        s, "s1,s2",
        "snapshot-and-re-enqueue: slot1 fires first, listener signals \
         slot2 which queues a fresh notify-MO microtask in same drain"
    );
}

#[test]
fn slotchange_ordered_in_microtask_queue_at_signal_time() {
    // R5 finding #4: the `NotifyMutationObservers` microtask is
    // enqueued at signal time (inside `signal_slot_change`), not at
    // drain-tail.  A `Promise.then(cb)` registered AFTER the
    // `slot.assign()` observes the post-slotchange state; one
    // registered BEFORE the assign still fires first.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.order = []; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var c = document.createElement('span'); host.appendChild(c); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         var slot = document.createElement('slot'); sr.append(slot); \
         slot.addEventListener('slotchange', function () { globalThis.order.push('sc'); }); \
         Promise.resolve().then(function () { globalThis.order.push('before'); }); \
         slot.assign(c); \
         Promise.resolve().then(function () { globalThis.order.push('after'); });",
    )
    .unwrap();
    let observed = vm.eval("globalThis.order.join(',')").unwrap();
    vm.unbind();
    let JsValue::String(sid) = observed else {
        panic!("expected string, got {observed:?}");
    };
    let s = vm.inner.strings.get_utf8(sid);
    assert_eq!(
        s, "before,sc,after",
        "notify-MO microtask interleaves with Promise reactions at signal time"
    );
}

#[test]
fn slot_assign_unchanged_list_does_not_signal_slotchange() {
    // R2 finding #2: WHATWG DOM §4.2.2.5 "assign slottables" step 2
    // — only signal a slot change when the resulting assigned-nodes
    // list differs from the prior list.  Repeated `slot.assign(c)`
    // with the SAME nodes across separate microtask checkpoints
    // must fire `slotchange` exactly once (initial change), not
    // twice.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // First eval: install listener, perform first assign (fires).
    vm.eval(&format!(
        "globalThis.fired = 0; {MANUAL_SLOT_PRELUDE} \
         globalThis.slot.addEventListener('slotchange', function () {{ globalThis.fired += 1; }}); \
         globalThis.slot.assign(globalThis.child);"
    ))
    .unwrap();
    let after_first = vm.eval("globalThis.fired").unwrap();
    let JsValue::Number(n1) = after_first else {
        panic!("expected number, got {after_first:?}");
    };
    assert!(
        (n1 - 1.0).abs() < f64::EPSILON,
        "first assign should fire once, got {n1}"
    );
    // Second eval: re-assign SAME node, then read counter.
    vm.eval("globalThis.slot.assign(globalThis.child);")
        .unwrap();
    let after_second = vm.eval("globalThis.fired").unwrap();
    vm.unbind();
    let JsValue::Number(n2) = after_second else {
        panic!("expected number, got {after_second:?}");
    };
    assert!(
        (n2 - 1.0).abs() < f64::EPSILON,
        "no-op re-assign should leave counter at 1, got {n2}"
    );
}

#[test]
fn slot_assign_empty_initial_does_not_signal() {
    // R6 finding #1: `slot.assign()` (no args) on a slot that has
    // never had a `SlotAssignment` component is a no-op vs. the
    // implicit-empty initial state.  No `slotchange` should fire.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(&format!(
        "globalThis.fired = 0; {MANUAL_SLOT_PRELUDE} \
         globalThis.slot.addEventListener('slotchange', function () {{ globalThis.fired += 1; }}); \
         globalThis.slot.assign();"
    ))
    .unwrap();
    let count = vm.eval("globalThis.fired").unwrap();
    vm.unbind();
    let JsValue::Number(n) = count else {
        panic!("expected number, got {count:?}");
    };
    assert_eq!(n, 0.0, "empty initial assign should not signal");
}

#[test]
fn slot_assign_cross_slot_dedup_fires_slotchange_at_both() {
    // R7 finding #3: WHATWG DOM §4.2.2.5 step 3 — assigning a node
    // to a second slot removes it from the first slot's assigned
    // list, and slotchange fires at BOTH slots.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.s1_fires = 0; globalThis.s2_fires = 0; \
         var host = document.createElement('div'); \
         document.body.appendChild(host); \
         globalThis.child = document.createElement('span'); host.appendChild(globalThis.child); \
         var sr = host.attachShadow({mode: 'open', slotAssignment: 'manual'}); \
         globalThis.s1 = document.createElement('slot'); \
         globalThis.s2 = document.createElement('slot'); \
         sr.append(globalThis.s1); sr.append(globalThis.s2); \
         globalThis.s1.addEventListener('slotchange', function () { globalThis.s1_fires += 1; }); \
         globalThis.s2.addEventListener('slotchange', function () { globalThis.s2_fires += 1; }); \
         globalThis.s1.assign(globalThis.child);",
    )
    .unwrap();
    let s1a = vm.eval("globalThis.s1_fires").unwrap();
    let s2a = vm.eval("globalThis.s2_fires").unwrap();
    vm.eval("globalThis.s2.assign(globalThis.child);").unwrap();
    let s1b = vm.eval("globalThis.s1_fires").unwrap();
    let s2b = vm.eval("globalThis.s2_fires").unwrap();
    let an1 = vm.eval("globalThis.s1.assignedNodes().length").unwrap();
    let an2 = vm
        .eval(
            "var arr = globalThis.s2.assignedNodes(); \
         (arr.length === 1 && arr[0] === globalThis.child) ? 1 : 0",
        )
        .unwrap();
    vm.unbind();
    let JsValue::Number(s1_first) = s1a else {
        panic!()
    };
    let JsValue::Number(s2_first) = s2a else {
        panic!()
    };
    let JsValue::Number(s1_second) = s1b else {
        panic!()
    };
    let JsValue::Number(s2_second) = s2b else {
        panic!()
    };
    let JsValue::Number(s1_len) = an1 else {
        panic!()
    };
    let JsValue::Number(s2_len) = an2 else {
        panic!()
    };
    assert_eq!(s1_first, 1.0, "s1 first assign should fire once");
    assert_eq!(s2_first, 0.0, "s2 no fire before its assign");
    assert_eq!(
        s1_second, 2.0,
        "s1 fires again on second eval (cross-slot removal)"
    );
    assert_eq!(s2_second, 1.0, "s2 fires from its own assignment");
    assert_eq!(s1_len, 0.0, "s1 list now empty (child moved to s2)");
    assert_eq!(s2_len, 1.0, "s2 list contains child");
}

// -------------------------------------------------------------------------
// Lifecycle / unbind regression
// -------------------------------------------------------------------------

#[test]
fn shadow_root_states_cleared_on_unbind() {
    // R4 finding #1: `shadow_root_states` (ObjectId-keyed) holds the
    // shadow-root Entity each wrapper resolves to.  Entity indices
    // are reused by a fresh `EcsDom`, so a retained ShadowRoot
    // wrapper must not silently resolve to an unrelated entity in
    // the new DOM post-rebind.  Unbind clears the side table; the
    // wrapper's brand check then throws "Illegal invocation" on
    // post-unbind accessor reads.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = vm
        .eval("var host = document.createElement('div'); host.attachShadow({mode: 'open'});")
        .unwrap();
    assert!(
        !vm.inner.shadow_root_states.is_empty(),
        "expected attachShadow to populate shadow_root_states"
    );
    vm.unbind();
    assert!(
        vm.inner.shadow_root_states.is_empty(),
        "expected shadow_root_states to be cleared on unbind, found {} entries",
        vm.inner.shadow_root_states.len()
    );
}

#[test]
fn require_node_arg_rejects_shadow_root_with_destroyed_entity() {
    // R6 finding #2: A retained ShadowRoot wrapper whose backing
    // entity is destroyed must throw "Illegal invocation" via the
    // shared existence check, not silently hand a stale entity to
    // Node IDL methods.  Simulated by unbinding (which clears
    // `shadow_root_states`) and rebinding to a fresh DOM, then
    // calling a Node-arg method using the retained wrapper.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // Retain the wrapper across the unbind boundary in a global.
    vm.eval(
        "globalThis.host = document.createElement('div'); \
         globalThis.sr = globalThis.host.attachShadow({mode: 'open'});",
    )
    .unwrap();
    vm.unbind();
    // Rebind to a fresh DOM (entity indices likely reused).
    let mut next_dom = EcsDom::new();
    let next_root = build_doc(&mut next_dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut next_dom, next_root);
    }
    let out = vm
        .eval(
            "var caught = null; \
         var probe = document.createElement('div'); \
         try { probe.contains(globalThis.sr); } \
         catch (e) { caught = e; } \
         (caught !== null && caught.name === 'TypeError') \
           ? 'ok' : 'fail:' + (caught && caught.name);",
        )
        .unwrap();
    vm.unbind();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}");
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "ok",
        "post-unbind ShadowRoot wrapper should throw TypeError when used as Node arg"
    );
}

#[test]
fn shadow_root_wrappers_cleared_on_unbind() {
    // After `attachShadow`, `shadow_root_wrappers` should hold the
    // host→wrapper entry; `Vm::unbind()` must clear it so a rebind
    // to a different DOM cannot resolve the stale wrapper.  Mirrors
    // `attr_wrapper_cache_cleared_on_unbind` (in
    // `tests_named_node_map.rs`).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = vm
        .eval("var host = document.createElement('div'); host.attachShadow({mode: 'open'});")
        .unwrap();
    assert!(
        !vm.inner.shadow_root_wrappers.is_empty(),
        "expected attachShadow to populate shadow_root_wrappers"
    );
    vm.unbind();
    assert!(
        vm.inner.shadow_root_wrappers.is_empty(),
        "expected shadow_root_wrappers to be cleared on unbind, found {} entries",
        vm.inner.shadow_root_wrappers.len()
    );
}

#[test]
fn shadow_root_wrapper_survives_gc_while_host_alive() {
    // R10 finding #1: `shadow_root_wrappers` is weak-through-owner.
    // If the cached wrapper isn't marked while the host wrapper is
    // alive, an unreferenced `host.shadowRoot` is swept and the
    // cache prune drops the entry → next `host.shadowRoot` returns
    // a FRESH wrapper, breaking identity + expando-property
    // continuity.  Setup: attach, write an expando, drop the local
    // reference, force GC, re-read.  The expando must survive.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "globalThis.host = document.createElement('div'); \
         var sr1 = globalThis.host.attachShadow({mode: 'open'}); \
         sr1.expando = 42; \
         sr1 = null;",
    )
    .unwrap();
    vm.inner.collect_garbage();
    let out = vm
        .eval(
            "var sr2 = globalThis.host.shadowRoot; \
         (sr2 !== null && sr2.expando === 42) ? 'ok' : 'fail:' + (sr2 && sr2.expando);",
        )
        .unwrap();
    vm.unbind();
    let JsValue::String(sid) = out else {
        panic!("expected string, got {out:?}");
    };
    assert_eq!(
        vm.inner.strings.get_utf8(sid),
        "ok",
        "expando on cached ShadowRoot wrapper should survive GC while host is alive"
    );
}

#[test]
fn unbind_clears_pending_notify_mutation_observers_microtask() {
    // R9 finding #1: a queued `NotifyMutationObservers` microtask
    // must NOT survive `Vm::unbind`.  If it did, a fresh signal
    // after rebind would dispatch behind any Promise microtasks
    // queued in the new tick (wrong ordering).  Verified directly:
    // signal a slot (queues the notify-MO microtask), unbind, then
    // confirm the microtask queue contains no `NotifyMutationObservers`
    // entry.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let _ = vm
        .eval(&format!(
            "{MANUAL_SLOT_PRELUDE} globalThis.slot.assign(globalThis.child);"
        ))
        .unwrap();
    vm.unbind();
    assert!(
        !vm.inner.microtask_queue.iter().any(|t| matches!(
            t,
            super::super::super::natives_promise::Microtask::NotifyMutationObservers
        )),
        "no stale NotifyMutationObservers microtask should survive unbind"
    );
    assert!(
        !vm.inner.mutation_observer_microtask_queued,
        "coalescing flag should be cleared on unbind"
    );
}
