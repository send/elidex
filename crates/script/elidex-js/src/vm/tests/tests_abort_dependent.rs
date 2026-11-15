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

/// A `Vm` with an (unbound) `HostData` installed so an `AbortSignal`'s
/// unified `vm_event_listeners` callbacks resolve without a DOM bind
/// (see `super::tests_abort::new_vm`).
fn new_vm() -> Vm {
    let mut v = Vm::new();
    v.install_host_data(super::super::host_data::HostData::new());
    v
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

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
    vm.inner.collect_garbage();
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
