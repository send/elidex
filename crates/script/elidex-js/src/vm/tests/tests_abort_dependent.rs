//! `AbortSignal.any` dependent-signal graph tests (WHATWG DOM §3.2
//! Interface AbortSignal: "create a dependent abort signal" and
//! "signal abort").
//!
//! Slot `#11-abort-signal-any-dependent-flattening`.  Cells (e)-(l) of the
//! plan's §6.  Kept out of [`super::tests_abort`] (950 lines) so that file
//! stays under the project's 1000-line convention.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;
use super::tests_abort::{eval_bool, new_vm};

/// (e) 10 000 nested `any([s])` with every intermediate kept, then a source
/// abort.  A dependent's source signals are never dependent (create a
/// dependent abort signal step 4.2), so the abort is one loop, not a chain
/// of Rust frames.
#[test]
fn any_chain_depth_one_kept_intermediates_10000() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "globalThis.c = new AbortController(); \
         globalThis.kept = []; \
         var s = c.signal; \
         for (var i = 0; i < 10000; i++) { s = AbortSignal.any([s]); kept.push(s); } \
         globalThis.last = s; \
         c.abort(); \
         last.aborted === true;"
    ));
}

/// (f) 50 nested `any()` with only the last kept: the unreferenced
/// intermediates are collected, and the abort still reaches the last one
/// (the spec never routes an abort *through* an intermediate).
#[test]
fn any_chain_dropped_intermediates_propagates_after_gc() {
    let mut vm = new_vm();
    vm.eval(
        "globalThis.c = new AbortController(); \
         (function () { \
            var s = c.signal; \
            for (var i = 0; i < 50; i++) { s = AbortSignal.any([s]); } \
            globalThis.s = s; \
         })();",
    )
    .unwrap();
    let before = vm.inner.abort_signal_states.len();
    vm.inner.collect_garbage();
    // Precondition: exactly the 49 unreferenced intermediates were swept
    // (independent of how many other rows the VM holds).
    assert_eq!(before - vm.inner.abort_signal_states.len(), 49);
    assert!(eval_bool(
        &mut vm,
        "c.abort(); s.aborted === true && s.reason === c.signal.reason;"
    ));
}

/// (g) "signal abort" step 4 sets every dependent's reason before step 5
/// fires the source's event, so a dependent reads as aborted from inside
/// the source's `'abort'` listener.
#[test]
fn dependents_are_aborted_inside_source_listener() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController(); \
         var d = AbortSignal.any([c.signal]); \
         var seen = null; \
         c.signal.addEventListener('abort', function () { seen = d.aborted; }); \
         c.abort(); \
         seen === true;"
    ));
}

/// (j) Default `AbortError`: one `DOMException` object shared by the
/// source and all its dependents (step 4.1.1 copies the reason value).
#[test]
fn reason_identity_across_dependents() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController(); \
         var d1 = AbortSignal.any([c.signal]); \
         var d2 = AbortSignal.any([c.signal]); \
         c.abort(); \
         d1.reason === c.signal.reason && d2.reason === c.signal.reason \
           && c.signal.reason instanceof DOMException;"
    ));
}

/// (l) A listener that aborts another source of a planned dependent finds
/// the dependent already aborted (step 4.1 skips it); its abort steps run
/// exactly once.
#[test]
fn listener_reentry_abort_other_source_fires_dependent_once() {
    let mut vm = new_vm();
    assert!(eval_bool(
        &mut vm,
        "var a = new AbortController(); \
         var b = new AbortController(); \
         var d = AbortSignal.any([a.signal, b.signal]); \
         var fired = 0; \
         d.addEventListener('abort', function () { fired++; }); \
         a.signal.addEventListener('abort', function () { b.abort(); }); \
         a.abort(); \
         fired === 1 && b.signal.aborted === true && d.aborted === true;"
    ));
}

// ---------------------------------------------------------------------------
// Graph / GC / seam cells (need the dependent-signal links and test seams)
// ---------------------------------------------------------------------------

use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::super::value::{NativeContext, ObjectId, VmError};
use super::signal_id;

fn sources(vm: &Vm, id: ObjectId) -> Vec<ObjectId> {
    vm.inner.abort_signal_states[&id]
        .links
        .source_signals()
        .to_vec()
}

fn dependents(vm: &Vm, id: ObjectId) -> Vec<ObjectId> {
    vm.inner.abort_signal_states[&id]
        .links
        .dependent_signals()
        .to_vec()
}

/// (h) The two sets are weak: a collected member vanishes from both, in
/// the same sweep that drops its row.
#[test]
fn sets_prune_dead_handles_at_sweep() {
    let mut vm = new_vm();
    vm.eval(
        "globalThis.a = new AbortController(); \
         globalThis.b = new AbortController(); \
         globalThis.d = AbortSignal.any([a.signal, b.signal]);",
    )
    .unwrap();
    let a = signal_id(&mut vm, "a.signal;");
    let b = signal_id(&mut vm, "b.signal;");
    let d = signal_id(&mut vm, "d;");
    vm.inner.collect_garbage();
    assert_eq!(sources(&vm, d), vec![a, b]);
    assert_eq!(dependents(&vm, b), vec![d]);

    vm.eval("globalThis.b = undefined;").unwrap();
    vm.inner.collect_garbage();
    assert!(
        !vm.inner.abort_signal_states.contains_key(&b),
        "the dropped source's row is collected"
    );
    assert_eq!(sources(&vm, d), vec![a], "the dead source left d's set");
    assert_eq!(dependents(&vm, a), vec![d]);
}

/// (i) A dropped dependent is pruned from its source's set; the source's
/// later abort is a silent no-op for dependents.
#[test]
fn dropped_dependent_is_pruned_from_source() {
    let mut vm = new_vm();
    vm.eval(
        "globalThis.a = new AbortController(); \
         globalThis.c = AbortSignal.any([a.signal]);",
    )
    .unwrap();
    let a = signal_id(&mut vm, "a.signal;");
    vm.inner.collect_garbage();
    assert_eq!(dependents(&vm, a).len(), 1, "a held dependent stays linked");

    vm.eval("globalThis.c = undefined;").unwrap();
    vm.inner.collect_garbage();
    assert!(dependents(&vm, a).is_empty(), "the dropped one is pruned");
    assert!(eval_bool(&mut vm, "a.abort('post-gc'); a.signal.aborted;"));
}

/// (k) The flags PR-B's predicate reads: an already-aborted `any()` result
/// is not dependent (step 3 is not reached); an empty `any([])` is
/// dependent with no sources (step 3 with an empty list).
#[test]
fn already_aborted_any_result_is_not_dependent() {
    let mut vm = new_vm();
    vm.eval(
        "globalThis.src = AbortSignal.abort('x'); \
         globalThis.r = AbortSignal.any([src]);",
    )
    .unwrap();
    let r = signal_id(&mut vm, "r;");
    let links = &vm.inner.abort_signal_states[&r].links;
    assert!(!links.is_dependent());
    assert!(links.source_signals().is_empty());
    assert!(eval_bool(&mut vm, "r.aborted && r.reason === 'x';"));
}

#[test]
fn empty_any_is_dependent_with_no_sources() {
    let mut vm = new_vm();
    let e = signal_id(&mut vm, "globalThis.e = AbortSignal.any([]);");
    let links = &vm.inner.abort_signal_states[&e].links;
    assert!(links.is_dependent());
    assert!(links.source_signals().is_empty());
}

/// A chain is flat: every result is sourced directly on the one
/// non-dependent signal (§4.2 of the plan, create a dependent abort signal
/// step 4.2).
#[test]
fn chain_sources_are_flattened_onto_the_non_dependent_signal() {
    let mut vm = new_vm();
    vm.eval(
        "globalThis.c = new AbortController(); \
         globalThis.d1 = AbortSignal.any([c.signal]); \
         globalThis.d2 = AbortSignal.any([d1]); \
         globalThis.d3 = AbortSignal.any([d2]);",
    )
    .unwrap();
    let s = signal_id(&mut vm, "c.signal;");
    let d1 = signal_id(&mut vm, "d1;");
    let d2 = signal_id(&mut vm, "d2;");
    let d3 = signal_id(&mut vm, "d3;");
    assert_eq!(dependents(&vm, s), vec![d1, d2, d3]);
    for d in [d1, d2, d3] {
        assert_eq!(sources(&vm, d), vec![s]);
        assert!(
            dependents(&vm, d).is_empty(),
            "a dependent has no dependents"
        );
    }
}

// (n) GC inside the plan loop on the timer path ------------------------------

fn native_arm_gc(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    ctx.vm.force_gc_before_next_alloc = true;
    Ok(JsValue::Undefined)
}

fn native_gc_fired(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(!ctx.vm.force_gc_before_next_alloc))
}

/// Drive `AbortSignal.timeout(0)`'s listener to force a collection after the
/// abort has been planned and before the dependents' abort steps run.
/// `kept` = reference each dependent from a global (control).
fn timer_path_gc_case(n_dependents: usize, kept: bool) {
    let mut vm = new_vm();
    let arm = vm.inner.create_native_function("__armGc", native_arm_gc);
    vm.set_global("__armGc", JsValue::Object(arm));
    let fired = vm
        .inner
        .create_native_function("__gcFired", native_gc_fired);
    vm.set_global("__gcFired", JsValue::Object(fired));

    vm.eval("globalThis.t = AbortSignal.timeout(0);").unwrap();
    let t = signal_id(&mut vm, "t;");
    let mut deps = Vec::new();
    for i in 0..n_dependents {
        let before: HashSet<ObjectId> = vm.inner.abort_signal_states.keys().copied().collect();
        let src = if kept {
            format!("globalThis.keep{i} = AbortSignal.any([t]); 0;")
        } else {
            "AbortSignal.any([t]); 0;".to_string()
        };
        vm.eval(&src).unwrap();
        let new: Vec<ObjectId> = vm
            .inner
            .abort_signal_states
            .keys()
            .copied()
            .filter(|k| !before.contains(k))
            .collect();
        assert_eq!(new.len(), 1, "exactly one row gained per any() call");
        deps.push(new[0]);
    }
    // Precondition: every dependent is planned-to-be (linked from `t`).
    assert_eq!(dependents(&vm, t), deps);

    // The listener arms the collection from inside the window (after
    // planning), allocates so it runs there, and then allocates a fresh
    // signal whose abort must never fire.
    vm.eval(
        "t.addEventListener('abort', function () { \
            __armGc(); ({}); \
            globalThis.g = __gcFired(); \
            globalThis.fresh = new AbortController(); \
            fresh.signal.addEventListener('abort', function () { globalThis.misfire = true; }); \
         });",
    )
    .unwrap();

    vm.inner
        .drain_timers(Instant::now() + Duration::from_millis(10));

    // Preconditions (both builds): the collection really ran inside the
    // listener.
    assert!(
        eval_bool(&mut vm, "g === true;"),
        "the forced collection was not placed inside the listener"
    );
    // Kill assertion: each planned dependent's row still exists and holds
    // t's reason (a recycled occupant's row has no reason; a swept row is
    // absent).
    let t_reason = vm.eval("t.reason;").unwrap();
    for &dep in &deps {
        let row = vm.inner.abort_signal_states.get(&dep);
        assert_eq!(
            row.and_then(|s| s.reason),
            Some(t_reason),
            "planned dependent {dep:?} was swept or recycled inside the plan loop"
        );
    }
    assert!(eval_bool(
        &mut vm,
        "typeof misfire === 'undefined' && fresh.signal.aborted === false && t.aborted === true;"
    ));
}

/// (n) The plan's handles are rooted on the VM stack: an unreferenced
/// planned dependent survives a collection that runs inside a listener on
/// the timer path (`gc_enabled = true`).  Mutant: drop the pushes onto the
/// scope's stack in `abort_signal` — the dependent is swept inside `t`'s
/// listener and the kill assertion is red.
#[test]
fn timer_path_plan_handles_survive_gc_inside_listener() {
    timer_path_gc_case(1, false);
}

#[test]
fn timer_path_two_dropped_dependents_survive_gc_inside_listener() {
    timer_path_gc_case(2, false);
}

/// Control: green with or without the root.
#[test]
fn timer_path_kept_dependent_survives_gc_inside_listener() {
    timer_path_gc_case(1, true);
}

// (o) the `Err` contract ------------------------------------------------------

/// (o) "signal abort" steps 5-6 are unconditional: an `Err` from the first
/// abort step must not skip the dependents' abort steps; the first `Err` is
/// returned after the loop.  Driven through `internal_abort_signal` (the
/// timer path's entry); the `Err` comes from the `#[cfg(test)]`
/// `fail_next_abort_dispatch` seam (`dispatch_vm_simple_event` never returns
/// `Err` today).  Mutant: `?` on the first `run_abort_steps` (or inside the
/// loop) — the dependents' listeners do not run.
#[test]
fn abort_err_does_not_skip_dependents() {
    let mut vm = new_vm();
    vm.eval(
        "globalThis.c = new AbortController(); \
         globalThis.d1 = AbortSignal.any([c.signal]); \
         globalThis.d2 = AbortSignal.any([c.signal]); \
         globalThis.ran1 = false; globalThis.ran2 = false; \
         d1.addEventListener('abort', function () { ran1 = true; }); \
         d2.addEventListener('abort', function () { ran2 = true; });",
    )
    .unwrap();
    let c_signal = signal_id(&mut vm, "c.signal;");
    vm.inner.fail_next_abort_dispatch = true;
    let result = super::super::host::abort::internal_abort_signal(
        &mut vm.inner,
        c_signal,
        JsValue::Undefined,
    );
    assert!(
        result.is_err(),
        "the injected Err is returned after the loop"
    );
    assert!(
        !vm.inner.fail_next_abort_dispatch,
        "the one-shot seam fired"
    );
    assert!(eval_bool(
        &mut vm,
        "d1.aborted && d2.aborted && c.signal.aborted && ran1 && ran2;"
    ));
}
