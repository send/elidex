//! PR3 C5: `ScriptEngine::call_listener` integration test —
//! **★ FIRST GREEN ★**.
//!
//! Validates the full vertical slice from PR3 C0..C5:
//!
//! 1. Compile a JS listener (`function h(e) { e.preventDefault(); }`).
//! 2. Register it in `HostData::listener_store` under a `ListenerId`.
//! 3. Build a real `DispatchEvent` (cancelable=true).
//! 4. Drive `ScriptEngine::call_listener`.
//! 5. Assert `event.flags.default_prevented == true` — i.e. the
//!    listener saw a working event object, called preventDefault, and
//!    the flag was synced back to `DispatchFlags`.
//!
//! This test is the architectural milestone for PR3.  Before this
//! commit, `call_listener` was a stub.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::EventPayload;
use elidex_script_session::event_dispatch::DispatchEvent;
use elidex_script_session::{ListenerId, ScriptContext, ScriptEngine, SessionCore};

use crate::engine::ElidexJsEngine;
use crate::vm::host_data::HostData;
use crate::vm::value::JsValue;

/// Compile a JS function and stash it in `listener_store` under
/// `ListenerId(1)`.  Returns nothing — the engine state carries the
/// registration.
#[allow(unsafe_code)]
fn register_listener_via_global(
    engine: &mut ElidexJsEngine,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    document: elidex_ecs::Entity,
    js_source: &str,
    listener_id: ListenerId,
) {
    // Bind the host pointers, eval the JS to install the function on
    // globalThis under `__listener__`, then read the ObjectId out of
    // globals and store it in HostData::listener_store.  The VM is
    // left bound on return; cleanup (`engine.vm().unbind()`) happens
    // at the call site after the listener invocation.
    engine.vm().install_host_data(HostData::new());
    unsafe {
        engine.vm().bind(session as *mut _, dom as *mut _, document);
    }
    engine
        .vm()
        .eval(&format!("globalThis.__listener__ = {js_source};"))
        .expect("listener compilation must succeed");

    let JsValue::Object(func_id) = engine
        .vm()
        .get_global("__listener__")
        .expect("__listener__ must be installed")
    else {
        panic!("__listener__ should be a function object");
    };

    engine
        .vm()
        .host_data()
        .expect("HostData installed")
        .store_listener(listener_id, func_id);
}

#[test]
fn first_green_listener_calls_prevent_default() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("button", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    register_listener_via_global(
        &mut engine,
        &mut session,
        &mut dom,
        doc,
        "function (e) { e.preventDefault(); }",
        listener_id,
    );

    // Build a cancelable click event targeted at the button.
    let mut event = DispatchEvent::new("click", target);
    event.cancelable = true;
    event.payload = EventPayload::None;
    assert!(
        !event.flags.default_prevented,
        "default_prevented must start false"
    );

    // Drive call_listener.  The placeholder ScriptContext is not used
    // by the VM impl (microtask drain wires up in PR3 C6); construct
    // a fresh one for each call.
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.call_listener(listener_id, &mut event, target, false, &mut ctx);

    assert!(
        event.flags.default_prevented,
        "preventDefault() inside listener must sync to event.flags"
    );

    engine.vm().unbind();
}

#[test]
fn passive_listener_prevent_default_is_silently_ignored() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("button", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    register_listener_via_global(
        &mut engine,
        &mut session,
        &mut dom,
        doc,
        "function (e) { e.preventDefault(); }",
        listener_id,
    );

    let mut event = DispatchEvent::new("click", target);
    event.cancelable = true;

    // passive=true threads through to ObjectKind::Event.passive, which
    // gates preventDefault into a silent no-op (WHATWG DOM §2.10
    // step 5.5).
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.call_listener(
        listener_id,
        &mut event,
        target,
        /* passive */ true,
        &mut ctx,
    );

    assert!(
        !event.flags.default_prevented,
        "passive listener must not be able to set default_prevented"
    );

    engine.vm().unbind();
}

#[test]
fn stop_propagation_syncs_to_dispatch_flags() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("div", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    register_listener_via_global(
        &mut engine,
        &mut session,
        &mut dom,
        doc,
        "function (e) { e.stopImmediatePropagation(); }",
        listener_id,
    );

    let mut event = DispatchEvent::new("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.call_listener(listener_id, &mut event, target, false, &mut ctx);

    assert!(
        event.flags.propagation_stopped,
        "stopImmediatePropagation must set the outer flag"
    );
    assert!(
        event.flags.immediate_propagation_stopped,
        "stopImmediatePropagation must set the inner flag"
    );

    engine.vm().unbind();
}

#[test]
fn current_target_is_the_this_binding_for_the_listener() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("button", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    // The listener captures `this === e.currentTarget` into a global
    // sentinel; we verify it after the call returns.  Since we can't
    // easily compare the JS-side target wrapper from Rust without
    // additional machinery, we just verify the boolean assertion.
    register_listener_via_global(
        &mut engine,
        &mut session,
        &mut dom,
        doc,
        "function (e) { globalThis.__match__ = (this === e.currentTarget); }",
        listener_id,
    );

    let mut event = DispatchEvent::new("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.call_listener(listener_id, &mut event, target, false, &mut ctx);

    let m = engine
        .vm()
        .get_global("__match__")
        .expect("__match__ must be set by listener");
    assert_eq!(
        m,
        JsValue::Boolean(true),
        "this must equal currentTarget inside listener body (WHATWG DOM §2.10)"
    );

    engine.vm().unbind();
}

#[test]
fn microtask_scheduled_inside_listener_fires_on_next_run_microtasks() {
    // Mirrors the shared dispatch core's pattern (HTML §8.1.7.3
    // microtask checkpoint after each listener) — verifies that the
    // VM's microtask queue is correctly populated by user JS during
    // the listener body and drained by `engine.run_microtasks(ctx)`.
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("div", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    register_listener_via_global(
        &mut engine,
        &mut session,
        &mut dom,
        doc,
        // Listener schedules a microtask via Promise.resolve().then().
        // The microtask body sets `__fired__` to true.
        "function (e) {
            globalThis.__fired__ = false;
            Promise.resolve().then(function () {
                globalThis.__fired__ = true;
            });
        }",
        listener_id,
    );

    let mut event = DispatchEvent::new("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.call_listener(listener_id, &mut event, target, false, &mut ctx);

    // Inside the listener body the microtask is queued but not yet
    // run — verify it is still pending right after `call_listener`.
    let pre = engine
        .vm()
        .get_global("__fired__")
        .expect("__fired__ must be initialised by the listener");
    assert_eq!(
        pre,
        JsValue::Boolean(false),
        "microtask must NOT fire synchronously inside listener body"
    );

    // The shared dispatch loop calls engine.run_microtasks(ctx) after
    // each listener; mirror that explicitly.
    engine.run_microtasks(&mut ctx);

    let post = engine
        .vm()
        .get_global("__fired__")
        .expect("__fired__ must persist");
    assert_eq!(
        post,
        JsValue::Boolean(true),
        "Promise.resolve().then() callback must fire on engine.run_microtasks"
    );

    engine.vm().unbind();
}

#[test]
fn missing_listener_id_silently_no_ops() {
    // A listener removed between dispatch-plan freeze and invocation
    // must not panic — the dispatch loop carries a stale ListenerId
    // and our impl returns early.
    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("div", Attributes::default());
    #[allow(unsafe_code)]
    unsafe {
        engine
            .vm()
            .bind(&mut session as *mut _, &mut dom as *mut _, doc);
    }

    let mut event = DispatchEvent::new("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.call_listener(
        ListenerId::from_raw(999),
        &mut event,
        target,
        false,
        &mut ctx,
    );

    assert!(
        !event.flags.default_prevented,
        "no listener fired → flags unchanged"
    );

    engine.vm().unbind();
}
