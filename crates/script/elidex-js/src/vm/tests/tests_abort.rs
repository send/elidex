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
    // Default reason is a DOMException with `name === "AbortError"`
    // (The from plain Error).
    assert_eq!(
        eval_string(
            &mut vm,
            "var c = new AbortController(); c.abort(); c.signal.reason.name;"
        ),
        "AbortError"
    );
}

#[test]
fn abort_default_reason_is_dom_exception() {
    // The guard. the default reason is a proper
    // DOMException instance, not an ad-hoc Error wrapper.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController(); c.abort(); \
         c.signal.reason instanceof DOMException \
         && c.signal.reason.code === 20;"
    ));
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
    // WHATWG §8.1.5. event-handler IDL attribute fires "first in
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
    // AbortSignal is an EventTarget but not a Node. its prototype
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

// PR4d C3 `addEventListener({signal})` integration tests moved to
// `tests_abort_signal_option.rs` (Thesplit).

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

// ---------------------------------------------------------------------------
// Copilot review fixes (PR #80)
// ---------------------------------------------------------------------------

#[test]
fn abort_listeners_survive_gc_during_dispatch() {
    // Regression: an early abort listener that triggers GC must not
    // collect the not-yet-called later listeners.  Pre-fix, the
    // implementation `mem::take`'d `abort_listeners` into a Rust local
    // before iterating, which dropped the GC root for the closures —
    // a GC inside the first callback could then reclaim the second
    // closure's `ObjectId`, leading to use-after-free / wrong dispatch.
    let mut vm = Vm::new();
    // Force GC to fire frequently inside the listener body.
    vm.inner.gc_threshold = 128;
    vm.inner.gc_enabled = true;

    let result = vm
        .eval(
            "var c = new AbortController();
             var seq = '';
             c.signal.addEventListener('abort', function() {
               // Allocate enough garbage to trip the GC threshold; the
               // second listener (still in the dispatch pool) must
               // survive collection.
               for (var i = 0; i < 200; i++) { var tmp = {x: i, y: i + 1}; }
               seq += 'a';
             });
             c.signal.addEventListener('abort', function() { seq += 'b'; });
             c.signal.addEventListener('abort', function() { seq += 'c'; });
             c.abort();
             seq;",
        )
        .unwrap();
    let JsValue::String(id) = result else {
        panic!("expected string, got {result:?}");
    };
    assert_eq!(
        vm.get_string(id),
        "abc",
        "all three listeners must fire in order despite GC inside the first"
    );
}

#[test]
fn onabort_remains_observable_after_dispatch() {
    // Regression: pre-fix, `state.onabort.take()` cleared the slot
    // before invoking the handler, making `signal.onabort` read
    // `null` post-abort.  Browsers leave the IDL handler attribute
    // observable.
    let mut vm = Vm::new();
    let result = vm
        .eval(
            "var c = new AbortController();
             var fn = function() {};
             c.signal.onabort = fn;
             c.abort();
             c.signal.onabort === fn;",
        )
        .unwrap();
    assert_eq!(
        result,
        JsValue::Boolean(true),
        "signal.onabort must remain observable after abort fires"
    );
}

#[test]
fn abort_after_unbind_cleans_listener_store() {
    // Regression: pre-fix, `detach_bound_listeners` returned early
    // when `HostData` was unbound, leaking `listener_store` entries
    // (and keeping their JS function ObjectIds rooted) for any
    // listener registered with `{signal}` whose `controller.abort()`
    // happened to run across an unbind boundary.
    use elidex_ecs::{Attributes, EcsDom};
    use elidex_script_session::SessionCore;

    use super::super::test_helpers::bind_vm;

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(doc, el);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(el);
    let key = vm.inner.strings.intern("target");
    vm.inner.globals.insert(key, JsValue::Object(wrapper));

    // Register a listener with {signal} while bound.
    vm.eval(
        "globalThis.c = new AbortController();
         target.addEventListener('click', function() {}, {signal: c.signal});",
    )
    .unwrap();

    let store_size_before = vm.inner.host_data.as_ref().unwrap().listener_store.len();
    assert!(
        store_size_before >= 1,
        "listener should be registered before unbind"
    );

    // Unbind the VM (simulates the shell ticking past the script's
    // direct execution while JS retains the controller).
    vm.unbind();

    // Re-bind for the abort call (so JS can reach `c`), then unbind
    // again. the controller still holds the back-ref Vec built
    // during the first bind.  We need the second eval to actually
    // run, so re-bind transiently; the listener_store cleanup is
    // what we're verifying, and that runs regardless of bind state.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("c.abort();").unwrap();

    let store_size_after = vm.inner.host_data.as_ref().unwrap().listener_store.len();
    assert!(
        store_size_after < store_size_before,
        "listener_store entry must be removed by abort (had {store_size_before}, now {store_size_after})"
    );
    vm.unbind();
}

#[test]
fn second_abort_with_modified_state_does_nothing() {
    // Regression: with the new "leave callbacks in state" approach,
    // a second abort() must still be a no-op.  The latch + the
    // post-dispatch `abort_listeners.clear()` together guarantee
    // this. verify by adding a listener AFTER the first abort
    // and confirming a second abort doesn't invoke it (the
    // already-aborted guard short-circuits the registration).
    let mut vm = Vm::new();
    let result = vm
        .eval(
            "var c = new AbortController();
             var n = 0;
             c.signal.addEventListener('abort', function() { n++; });
             c.abort();
             // Try to register after abort. should be ignored.
             c.signal.addEventListener('abort', function() { n += 100; });
             c.abort();
             n;",
        )
        .unwrap();
    assert_eq!(
        result,
        JsValue::Number(1.0),
        "second abort must not refire listeners or pick up post-abort registrations"
    );
}

#[test]
fn dispatch_event_validates_abort_signal_receiver() {
    // Regression: prior to this guard, the `dispatchEvent` stub
    // ignored its receiver and silently returned `false`, allowing
    // `AbortSignal.prototype.dispatchEvent.call({})` to succeed —
    // inconsistent with the other AbortSignal methods which throw
    // TypeError on cross-call.
    let mut vm = Vm::new();
    let result = vm
        .eval(
            "var caught = '';
             try {
               AbortSignal.prototype.dispatchEvent.call({}, null);
               caught = 'no-throw';
             } catch(e) { caught = e.toString(); }
             caught;",
        )
        .unwrap();
    let s = match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert!(
        s.contains("TypeError") && s.contains("AbortSignal"),
        "expected TypeError mentioning AbortSignal, got {s}"
    );
}

#[test]
fn dispatch_event_on_real_signal_returns_false_stub() {
    // The stub still returns `false` for legitimate AbortSignal
    // receivers. only the cross-call case throws.
    let mut vm = Vm::new();
    assert_eq!(
        vm.eval(
            "var c = new AbortController();
             c.signal.dispatchEvent({type: 'abort'});"
        )
        .unwrap(),
        JsValue::Boolean(false),
        "dispatchEvent stub should return false for valid receiver"
    );
}

// ---------------------------------------------------------------------------
// Copilot R4: AbortController internal slot
// ---------------------------------------------------------------------------

#[test]
fn abort_call_on_alien_object_with_signal_property_throws() {
    // Regression: pre-fix, `controller.abort()` read the paired
    // signal from the public own property `signal`, so
    // `AbortController.prototype.abort.call({signal: realSignal})`
    // happily aborted `realSignal`.  With the internal-slot fix,
    // an alien receiver throws TypeError before reaching the
    // dispatch path.
    let mut vm = Vm::new();
    let result = vm
        .eval(
            "var c = new AbortController();
             var alien = {signal: c.signal};
             var caught = '';
             try {
               AbortController.prototype.abort.call(alien);
               caught = 'no-throw';
             } catch(e) { caught = e.toString(); }
             // The cross-call must throw AND must not abort the
             // signal.
             caught + '|aborted=' + c.signal.aborted;",
        )
        .unwrap();
    let s = match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert!(
        s.contains("TypeError") && s.ends_with("|aborted=false"),
        "expected TypeError + signal not aborted, got {s}"
    );
}

#[test]
fn defining_signal_property_does_not_retarget_abort() {
    // Regression: `Object.defineProperty(c, 'signal', {value: alien})`
    // pre-fix made `c.abort()` abort the alien signal because the
    // method read from the property storage.  With the internal
    // slot, the original signal aborts and the alien is untouched.
    let mut vm = Vm::new();
    let result = vm
        .eval(
            "var c = new AbortController();
             var original = c.signal;
             var alien = new AbortController().signal;
             // Replace the visible property. must not affect abort target.
             Object.defineProperty(c, 'signal', {value: alien, configurable: true});
             c.abort();
             // Original signal aborts via internal slot; alien stays untouched.
             original.aborted + '|' + alien.aborted;",
        )
        .unwrap();
    let s = match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(
        s, "true|false",
        "internal slot must drive abort target, not the JS-visible property; got {s}"
    );
}

#[test]
fn controller_signal_property_still_readable_normally() {
    // Sanity: the internal-slot fix must not break the common
    // `controller.signal` JS read path.
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var c = new AbortController();
         var s = c.signal;
         typeof s === 'object' && s !== null && typeof s.aborted === 'boolean';"
    ));
}

mod bound_listener_pruning {
    //! Regression for Copilot R2: `bound_listener_removals` must be
    //! pruned when the underlying listener is removed (via
    //! `removeEventListener`). otherwise the back-ref grows
    //! unbounded across add/remove cycles for a long-lived signal.

    use super::*;
    use elidex_ecs::{Attributes, EcsDom};
    use elidex_script_session::SessionCore;

    use super::super::super::test_helpers::bind_vm;
    use super::super::super::Vm;

    fn bind_with_div(
        vm: &mut Vm,
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> elidex_ecs::Entity {
        let doc = dom.create_document_root();
        let el = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, el);
        #[allow(unsafe_code)]
        unsafe {
            bind_vm(vm, session, dom, doc);
        }
        let wrapper = vm.inner.create_element_wrapper(el);
        let key = vm.inner.strings.intern("target");
        vm.inner.globals.insert(key, JsValue::Object(wrapper));
        el
    }

    #[test]
    fn remove_event_listener_prunes_signal_back_ref() {
        let mut vm = Vm::new();
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let _ = bind_with_div(&mut vm, &mut session, &mut dom);

        // Five add/remove cycles.  Without pruning, the signal's
        // `bound_listener_removals` would grow to 5 stale entries.
        vm.eval(
            "globalThis.c = new AbortController();
             for (var i = 0; i < 5; i++) {
               function cb() {}
               target.addEventListener('click', cb, {signal: c.signal});
               target.removeEventListener('click', cb);
             }",
        )
        .unwrap();

        // Reach into VM state directly. no JS-visible API exposes
        // the back-ref Vec count.
        //
        // SAFETY-of-test: we never escape the borrow; just reading
        // the size of internal state for the assertion.
        let signal_id = match vm.eval("c.signal;").unwrap() {
            JsValue::Object(id) => id,
            other => panic!("c.signal is not an object: {other:?}"),
        };
        let removals_count = vm
            .inner
            .abort_signal_states
            .get(&signal_id)
            .map_or(usize::MAX, |s| s.bound_listener_removals.len());
        assert_eq!(
            removals_count, 0,
            "back-ref must be pruned by removeEventListener; found {removals_count} stale entries"
        );

        let back_refs_count = vm.inner.abort_listener_back_refs.len();
        assert_eq!(
            back_refs_count, 0,
            "reverse-index entries must drop in lockstep with removals"
        );
        vm.unbind();
    }

    #[test]
    fn back_ref_survives_abort_after_unbind_then_rebind() {
        // Defence-in-depth check: the GC sweep cleanup walks
        // `abort_listener_back_refs` and prunes entries whose
        // signal_id was collected.  Verify a live signal's entries
        // survive a GC pass.
        let mut vm = Vm::new();
        let mut session = SessionCore::new();
        let mut dom = EcsDom::new();
        let _ = bind_with_div(&mut vm, &mut session, &mut dom);

        vm.eval(
            "globalThis.c = new AbortController();
             target.addEventListener('click', function() {}, {signal: c.signal});",
        )
        .unwrap();

        let before = vm.inner.abort_listener_back_refs.len();
        assert_eq!(before, 1, "expected one back-ref entry pre-GC");

        // Force a GC pass while the signal is still rooted via
        // `globalThis.c`. entries must survive.
        vm.inner.collect_garbage();

        let after = vm.inner.abort_listener_back_refs.len();
        assert_eq!(
            after, 1,
            "back-ref entry must survive GC while signal is rooted"
        );
        vm.unbind();
    }
}

// Thestatic factory tests moved to tests_abort_statics.rs (C9 split).
