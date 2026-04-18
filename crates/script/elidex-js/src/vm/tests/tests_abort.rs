//! PR4d C2: `AbortController` / `AbortSignal` primitive tests.
//!
//! Exercises construction, accessor reads, listener registration +
//! one-shot dispatch, the `onabort` event-handler IDL slot, and
//! `throwIfAborted`.  PR4d C3 adds the `addEventListener({signal})`
//! integration tests.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn constructor_returns_object_with_signal() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController(); typeof c === 'object' && typeof c.signal === 'object';"
    ));
}

#[test]
fn signal_initially_not_aborted() {
    let mut vm = Vm::new();
    assert!(!eval_bool(
        &mut vm,
        "var c = new AbortController(); c.signal.aborted;"
    ));
}

#[test]
fn signal_initial_reason_is_undefined() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController(); typeof c.signal.reason;"
        ),
        "undefined"
    );
}

#[test]
fn abort_sets_aborted_flag() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController(); c.abort(); c.signal.aborted;"
    ));
}

#[test]
fn abort_with_undefined_creates_default_abort_error() {
    let mut vm = Vm::new();
    // Default reason is an Error with `name === "AbortError"`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController(); c.abort(); c.signal.reason.name;"
        ),
        "AbortError"
    );
}

#[test]
fn abort_with_custom_reason_preserves_value() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController(); c.abort('custom'); c.signal.reason;"
        ),
        "custom"
    );
}

#[test]
fn abort_with_object_reason_preserves_identity() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var r = {tag: 1}; var c = new AbortController(); c.abort(r); c.signal.reason === r;"
    ));
}

#[test]
fn abort_is_idempotent() {
    let mut vm = Vm::new();
    // Second `abort('two')` must NOT overwrite the reason set by the first.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController(); c.abort('first'); c.abort('two'); c.signal.reason;"
        ),
        "first"
    );
}

#[test]
fn add_event_listener_fires_on_abort() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             var fired = '';
             c.signal.addEventListener('abort', function() { fired = 'yes'; });
             c.abort();
             fired;"
        ),
        "yes"
    );
}

#[test]
fn add_event_listener_multiple_callbacks_fire_in_order() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             var seq = '';
             c.signal.addEventListener('abort', function() { seq += 'a'; });
             c.signal.addEventListener('abort', function() { seq += 'b'; });
             c.abort();
             seq;"
        ),
        "ab"
    );
}

#[test]
fn add_event_listener_dedupes_identical_callback() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             var n = 0;
             function cb() { n++; }
             c.signal.addEventListener('abort', cb);
             c.signal.addEventListener('abort', cb);
             c.abort();
             String(n);"
        ),
        "1"
    );
}

#[test]
fn add_event_listener_filters_non_abort_types() {
    let mut vm = Vm::new();
    // Other event types are accepted (no throw) but never fire,
    // since the only event a signal dispatches is `'abort'`.
    assert!(!eval_bool(
        &mut vm,
        "var c = new AbortController();
         var fired = false;
         c.signal.addEventListener('click', function() { fired = true; });
         c.abort();
         fired;"
    ));
}

#[test]
fn add_event_listener_after_abort_is_noop() {
    let mut vm = Vm::new();
    // Per PR4d MVP: registering after abort is a no-op (full
    // microtask-queueing per spec lands in PR5a).
    assert!(!eval_bool(
        &mut vm,
        "var c = new AbortController();
         c.abort();
         var fired = false;
         c.signal.addEventListener('abort', function() { fired = true; });
         fired;"
    ));
}

#[test]
fn remove_event_listener_drops_callback() {
    let mut vm = Vm::new();
    assert!(!eval_bool(
        &mut vm,
        "var c = new AbortController();
         var fired = false;
         function cb() { fired = true; }
         c.signal.addEventListener('abort', cb);
         c.signal.removeEventListener('abort', cb);
         c.abort();
         fired;"
    ));
}

#[test]
fn second_abort_does_not_refire_listeners() {
    let mut vm = Vm::new();
    // One-shot: the listener pool is cleared on first abort, so a
    // second `c.abort()` cannot re-fire it.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             var n = 0;
             c.signal.addEventListener('abort', function() { n++; });
             c.abort();
             c.abort();
             String(n);"
        ),
        "1"
    );
}

#[test]
fn onabort_handler_fires_on_abort() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController();
         var fired = false;
         c.signal.onabort = function() { fired = true; };
         c.abort();
         fired;"
    ));
}

#[test]
fn onabort_runs_before_addeventlistener_callbacks() {
    let mut vm = Vm::new();
    // WHATWG §8.1.5 — event-handler IDL attribute fires "first in
    // addition to others registered".  PR4d implements that order.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             var seq = '';
             c.signal.addEventListener('abort', function() { seq += 'a'; });
             c.signal.onabort = function() { seq += 'o'; };
             c.abort();
             seq;"
        ),
        "oa"
    );
}

#[test]
fn onabort_can_be_cleared_with_null() {
    let mut vm = Vm::new();
    assert!(!eval_bool(
        &mut vm,
        "var c = new AbortController();
         var fired = false;
         c.signal.onabort = function() { fired = true; };
         c.signal.onabort = null;
         c.abort();
         fired;"
    ));
}

#[test]
fn onabort_setter_silently_ignores_non_callable() {
    let mut vm = Vm::new();
    // WHATWG event-handler IDL: assigning a non-callable, non-null
    // value silently no-ops; the prior handler stays in place.
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController();
         var fired = false;
         c.signal.onabort = function() { fired = true; };
         c.signal.onabort = 'not a function';
         c.abort();
         fired;"
    ));
}

#[test]
fn throw_if_aborted_noop_when_not_aborted() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController();
         var ok = true;
         try { c.signal.throwIfAborted(); } catch(e) { ok = false; }
         ok;"
    ));
}

#[test]
fn throw_if_aborted_throws_reason_when_aborted() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             c.abort('boom');
             var caught = '';
             try { c.signal.throwIfAborted(); } catch(e) { caught = e; }
             caught;"
        ),
        "boom"
    );
}

#[test]
fn new_abort_signal_throws_type_error() {
    let mut vm = Vm::new();
    // WHATWG §3.1: `AbortSignal` is not user-constructable; only
    // `AbortController` produces them (PR5a will add the static
    // factories).
    assert_eq!(
        eval_string(
            &mut vm,
            "var msg = '';
             try { new AbortSignal(); } catch(e) { msg = e.message; }
             msg;"
        ),
        "AbortSignal is not constructable"
    );
}

#[test]
fn signal_is_event_target_but_not_node() {
    let mut vm = Vm::new();
    // AbortSignal.prototype chains to EventTarget.prototype but
    // skips Node.prototype (PR4c §7.2 separation).  `nodeType` /
    // `parentNode` etc. must remain `undefined`.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController();
             typeof c.signal.nodeType + '|' + typeof c.signal.parentNode;"
        ),
        "undefined|undefined"
    );
}

#[test]
fn signal_proto_chain_skips_node_prototype() {
    // AbortSignal is an EventTarget but not a Node — its prototype
    // chain must be `signal → AbortSignal.prototype →
    // EventTarget.prototype → Object.prototype` (3 hops up).
    // Verifying directly via VM internals is more robust than going
    // through `Object.prototype` (no global Object.prototype slot is
    // exposed to JS in elidex; the engine intrinsics are pinned via
    // `VmInner::object_prototype`).
    let vm = Vm::new();
    let signal_proto = vm.inner.abort_signal_prototype.expect("must exist");
    let p_event_target = vm
        .inner
        .get_object(signal_proto)
        .prototype
        .expect("AbortSignal.prototype must have a parent");
    assert_eq!(
        Some(p_event_target),
        vm.inner.event_target_prototype,
        "AbortSignal.prototype must chain to EventTarget.prototype"
    );
    let p_object = vm
        .inner
        .get_object(p_event_target)
        .prototype
        .expect("EventTarget.prototype must have a parent");
    assert_eq!(
        Some(p_object),
        vm.inner.object_prototype,
        "chain must reach Object.prototype"
    );
}

#[test]
fn abort_controller_constructor_requires_new() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "var msg = '';
             try { AbortController(); } catch(e) { msg = e.message; }
             msg;"
        ),
        "AbortController constructor cannot be invoked without 'new'"
    );
}
