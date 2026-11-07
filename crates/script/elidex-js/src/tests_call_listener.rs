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
use elidex_script_session::{EventListeners, ListenerId, ScriptEngine, SessionCore};

use crate::engine::ElidexJsEngine;
use crate::tests_engine_common::with_bound_ctx;
use crate::vm::value::JsValue;

/// Compile a JS function and stash it in `listener_store` under
/// `listener_id`.  Returns nothing — the engine state carries the
/// registration.
///
/// Requires a **bound** VM: call it from inside a [`with_bound_ctx`] bracket.
/// It evals the JS to install the function on `globalThis` under
/// `__listener__`, then reads the ObjectId out of globals and stores it in
/// `HostData::listener_store`.
fn register_listener(engine: &mut ElidexJsEngine, js_source: &str, listener_id: ListenerId) {
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
    let target = dom.create_element_ownerless_fixture("button", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    // Build a cancelable click event targeted at the button.
    let mut event = DispatchEvent::new("click", target);
    event.cancelable = true;
    event.payload = EventPayload::None;
    assert!(
        !event.flags.default_prevented,
        "default_prevented must start false"
    );

    // The helper builds the ScriptContext first and binds FROM it, so the
    // VM's raw pointers derive from `ctx` and stay valid for the bracket.
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        register_listener(engine, "function (e) { e.preventDefault(); }", listener_id);
        // Drive call_listener. The VM impl ignores `ctx` (assume-bound: it
        // reaches the DOM through the bound VM).
        engine.call_listener(listener_id, &mut event, target, false, false, ctx);
    });

    assert!(
        event.flags.default_prevented,
        "preventDefault() inside listener must sync to event.flags"
    );
}

/// Copilot PR212 R2 #3 regression: the UA-initiated dispatch path
/// (`ScriptEngine::call_listener`, used by the session-crate dispatch
/// loop) must lazy-compile an inline event-handler IDL attribute source
/// before resolving its callable — not just the script `walk_phase`. A
/// `<body onload="...">`-style content attribute reaches handlers via this
/// path on UA fire (WHATWG HTML §8.1.8.1).
#[test]
fn call_listener_lazy_compiles_inline_handler() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element_ownerless_fixture("div", Attributes::default());

    // Pre-seed an event-handler listener whose value is an uncompiled
    // inline source (as the inline-attribute consumer would record).
    let listener_id = {
        let mut listeners = EventListeners::new();
        let id = listeners.add_event_handler("click");
        listeners.set_uncompiled(id, "globalThis.x = 1");
        dom.world_mut()
            .insert_one(target, listeners)
            .expect("insert EventListeners");
        id
    };

    let mut event = DispatchEvent::new("click", target);
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        engine.vm().eval("globalThis.x = 0;").unwrap();

        // No compiled callable is stored yet — only the uncompiled source.
        assert!(
            engine
                .vm()
                .host_data()
                .expect("HostData")
                .get_listener(listener_id)
                .is_none(),
            "inline handler must not be compiled before first dispatch"
        );

        engine.call_listener(
            listener_id,
            &mut event,
            target,
            false,
            /* is_handler */ true,
            ctx,
        );

        assert_eq!(
            engine.vm().eval("globalThis.x").unwrap(),
            JsValue::Number(1.0),
            "call_listener must lazy-compile + invoke the inline handler"
        );
    });
}

/// Codex PR327 R2 (security/spec) — **C28b** through the UA path: the
/// sandboxed twin of `call_listener_lazy_compiles_inline_handler`. In a sandboxed
/// iframe without `allow-scripts`, scripting is disabled (HTML §8.1.3.4), so a raw
/// uncompiled inline `<button onclick=...>` handler is neither compiled nor run on
/// UA dispatch: *get the current value of the event handler* step 3.2 ("If
/// document's browsing context is null, or document's active sandboxing flag set
/// has its sandboxed scripts browsing context flag set, then return null.")
/// returns null for a raw value. The target here is a detached owner-`None`
/// element, so the settings clause refuses before the (unresolvable) node clause
/// is reached. The compile gate covers the listener-dispatch path, not only
/// classic-script `eval`.
#[test]
fn call_listener_skips_inline_handler_when_scripting_disabled() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element_ownerless_fixture("button", Attributes::default());

    // Pre-seed an inline handler exactly as `call_listener_lazy_compiles_inline_handler`
    // does — the only difference is the sandbox flag set below.
    let listener_id = {
        let mut listeners = EventListeners::new();
        let id = listeners.add_event_handler("click");
        listeners.set_uncompiled(id, "globalThis.ran = true");
        dom.world_mut()
            .insert_one(target, listeners)
            .expect("insert EventListeners");
        id
    };

    // Sandboxed WITHOUT `allow-scripts` → scripting is disabled for this context.
    engine
        .vm()
        .host_data()
        .expect("HostData")
        .set_sandbox_flags(elidex_plugin::SandboxingFlagSet::all_represented());

    let mut event = DispatchEvent::new("click", target);
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        engine.call_listener(
            listener_id,
            &mut event,
            target,
            false,
            /* is_handler */ true,
            ctx,
        );

        // The inline handler was never compiled (no callable stored) and never ran.
        assert!(
            engine
                .vm()
                .host_data()
                .expect("HostData")
                .get_listener(listener_id)
                .is_none(),
            "scripting-disabled context must not compile the inline handler"
        );
        assert!(
            !matches!(engine.vm().get_global("ran"), Some(JsValue::Boolean(true))),
            "the inline handler must not run when scripting is disabled (sandbox bypass)"
        );
    });
}

#[test]
fn passive_listener_prevent_default_is_silently_ignored() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element_ownerless_fixture("button", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    let mut event = DispatchEvent::new("click", target);
    event.cancelable = true;

    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        register_listener(engine, "function (e) { e.preventDefault(); }", listener_id);

        // passive=true threads through to ObjectKind::Event.passive, which
        // gates preventDefault into a silent no-op (WHATWG DOM §2.9 inner
        // invoke step 2.11 sets the in passive listener flag; §2.2 set the
        // canceled flag then no-ops).
        engine.call_listener(
            listener_id,
            &mut event,
            target,
            /* passive */ true,
            /* is_handler */ false,
            ctx,
        );
    });

    assert!(
        !event.flags.default_prevented,
        "passive listener must not be able to set default_prevented"
    );
}

#[test]
fn stop_propagation_syncs_to_dispatch_flags() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element_ownerless_fixture("div", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    let mut event = DispatchEvent::new("click", target);
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        register_listener(
            engine,
            "function (e) { e.stopImmediatePropagation(); }",
            listener_id,
        );
        engine.call_listener(listener_id, &mut event, target, false, false, ctx);
    });

    assert!(
        event.flags.propagation_stopped,
        "stopImmediatePropagation must set the outer flag"
    );
    assert!(
        event.flags.immediate_propagation_stopped,
        "stopImmediatePropagation must set the inner flag"
    );
}

#[test]
fn current_target_is_the_this_binding_for_the_listener() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element_ownerless_fixture("button", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    let mut event = DispatchEvent::new("click", target);
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        // The listener captures `this === e.currentTarget` into a global
        // sentinel; we verify it after the call returns.  Since we can't
        // easily compare the JS-side target wrapper from Rust without
        // additional machinery, we just verify the boolean assertion.
        register_listener(
            engine,
            "function (e) { globalThis.__match__ = (this === e.currentTarget); }",
            listener_id,
        );

        engine.call_listener(listener_id, &mut event, target, false, false, ctx);

        let m = engine
            .vm()
            .get_global("__match__")
            .expect("__match__ must be set by listener");
        assert_eq!(
            m,
            JsValue::Boolean(true),
            "this must equal currentTarget inside listener body (WHATWG DOM §2.9 inner invoke step 2.13)"
        );
    });
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
    let target = dom.create_element_ownerless_fixture("div", Attributes::default());
    let listener_id = ListenerId::from_raw(1);

    let mut event = DispatchEvent::new("click", target);
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        register_listener(
            engine,
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

        engine.call_listener(listener_id, &mut event, target, false, false, ctx);

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
        engine.run_microtasks(ctx);

        let post = engine
            .vm()
            .get_global("__fired__")
            .expect("__fired__ must persist");
        assert_eq!(
            post,
            JsValue::Boolean(true),
            "Promise.resolve().then() callback must fire on engine.run_microtasks"
        );
    });
}

#[test]
fn missing_listener_id_silently_no_ops() {
    // A listener removed between dispatch-plan freeze and invocation
    // must not panic — the dispatch loop carries a stale ListenerId
    // and our impl returns early.
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element_ownerless_fixture("div", Attributes::default());

    let mut event = DispatchEvent::new("click", target);
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        engine.call_listener(
            ListenerId::from_raw(999),
            &mut event,
            target,
            false,
            /* is_handler */ false,
            ctx,
        );
    });

    assert!(
        !event.flags.default_prevented,
        "no listener fired → flags unchanged"
    );
}

/// C8e (ii): the UA path reads the handler value through the same reconcile as
/// the VM walk and the IDL getter. A raw handler on a `DOMParser`-document node
/// is not compiled (the node clause of the compile gate) and does not run; a
/// callback value there (IDL-set) runs — there is no §8.1.3.4 check at
/// invocation (step 1 of the processing algorithm is WebDriver BiDi).
#[test]
fn call_listener_foreign_document_node_raw_gated_callback_runs() {
    let mut engine = ElidexJsEngine::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    // A bound document, a second (`DOMParser`-like, null browsing context)
    // document, and an element owned by and appended into the second one.
    let doc = dom.create_document_root();
    let doc2 = dom.create_document_node();
    let foreign = dom.create_element_with_owner(
        "div",
        Attributes::default(),
        dom.node_document_for_new_nodes(doc2),
    );
    assert!(dom.append_child(doc2, foreign));

    let (raw_id, callback_id) = {
        let mut listeners = EventListeners::new();
        let raw_id = listeners.add_event_handler("click");
        listeners.set_uncompiled(raw_id, "globalThis.rawRan = true");
        let callback_id = listeners.add_event_handler("mousedown");
        dom.world_mut()
            .insert_one(foreign, listeners)
            .expect("insert EventListeners");
        (raw_id, callback_id)
    };
    with_bound_ctx(&mut engine, &mut session, &mut dom, doc, |engine, ctx| {
        register_listener(
            engine,
            "function () { globalThis.callbackRan = true; }",
            callback_id,
        );
        engine
            .vm()
            .eval("globalThis.rawRan = false; globalThis.callbackRan = false;")
            .unwrap();

        let mut click = DispatchEvent::new("click", foreign);
        engine.call_listener(raw_id, &mut click, foreign, false, true, ctx);
        let mut down = DispatchEvent::new("mousedown", foreign);
        engine.call_listener(callback_id, &mut down, foreign, false, true, ctx);

        assert!(
            engine
                .vm()
                .host_data()
                .expect("HostData")
                .get_listener(raw_id)
                .is_none(),
            "the node clause must keep the raw handler uncompiled"
        );
        assert_eq!(
            engine.vm().eval("globalThis.rawRan").unwrap(),
            JsValue::Boolean(false),
            "a raw handler on a null-browsing-context document's node reads null"
        );
        assert_eq!(
            engine.vm().eval("globalThis.callbackRan").unwrap(),
            JsValue::Boolean(true),
            "a callback value runs wherever it lives (nothing gates the call)"
        );
    });
}
