//! PR3 C12: end-to-end dispatch integration tests.
//!
//! Drives the full path: register listeners via JS `addEventListener`,
//! call `script_dispatch_event` (the public dispatch entry point that
//! every shell uses), and observe both the return value (`prevented`)
//! and side-effects via JS-side global sentinels.
//!
//! These tests mirror the most-load-bearing scenarios from
//! `crates/script/elidex-js-boa/src/runtime/tests/events.rs` —
//! bubble / capture / stop / once / passive — translated to use the
//! VM engine.  The point is to confirm that PR3's per-commit unit
//! tests compose into spec-conforming dispatch when the shared
//! 3-phase machinery in `elidex-script-session` plays the lead.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit};
use elidex_script_session::event_dispatch::{script_dispatch_event, DispatchEvent};
use elidex_script_session::{ScriptContext, SessionCore};

use crate::engine::ElidexJsEngine;
use crate::vm::host_data::HostData;
use crate::vm::value::JsValue;

/// Construct an unbound engine + session + dom with a fresh
/// `document_root`.  Callers create their DOM tree, then call
/// `bind_after_dom` to start the VM's host-pointer lifecycle.
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

/// Bind the engine's VM against `session` / `dom` / `doc`.  Must be
/// called AFTER all DOM mutations are complete — the bound raw
/// pointers cannot coexist with concurrent `&mut dom` accesses
/// (stacked-borrows).
#[allow(unsafe_code)]
fn bind_after_dom(
    engine: &mut ElidexJsEngine,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
) {
    unsafe {
        engine
            .vm()
            .bind(std::ptr::from_mut(session), std::ptr::from_mut(dom), doc);
    }
}

/// Convenience: read a boolean global the listener was supposed to set.
fn get_bool(engine: &mut ElidexJsEngine, name: &str) -> bool {
    matches!(engine.vm().get_global(name), Some(JsValue::Boolean(true)))
}

#[test]
fn listener_fires_on_at_target_phase() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("button", Attributes::default());
    assert!(dom.append_child(doc, target));

    // Expose the target wrapper as a global so JS can call
    // addEventListener on it.
    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval(
            "globalThis.fired = false;
             el.addEventListener('click', function () { globalThis.fired = true; });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert!(get_bool(&mut engine, "fired"));
    engine.vm().unbind();
}

#[test]
fn prevent_default_returns_true_from_dispatch() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("a", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval("el.addEventListener('click', function (e) { e.preventDefault(); });")
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let prevented = script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert!(
        prevented,
        "preventDefault inside listener must propagate to dispatch return value"
    );
    engine.vm().unbind();
}

#[test]
fn event_bubbles_through_parent() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(doc, outer));
    assert!(dom.append_child(outer, inner));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let outer_w = engine.vm().inner.create_element_wrapper(outer);
    let inner_w = engine.vm().inner.create_element_wrapper(inner);
    engine.vm().set_global("outer", JsValue::Object(outer_w));
    engine.vm().set_global("inner", JsValue::Object(inner_w));

    engine
        .vm()
        .eval(
            "globalThis.outerFired = false;
             globalThis.innerFired = false;
             outer.addEventListener('click', function () { globalThis.outerFired = true; });
             inner.addEventListener('click', function () { globalThis.innerFired = true; });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", inner);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert!(get_bool(&mut engine, "innerFired"), "at-target listener");
    assert!(get_bool(&mut engine, "outerFired"), "bubble-phase listener");
    engine.vm().unbind();
}

/// Codex PR327 R4 (boa parity): the post-dispatch checkpoint
/// `script_dispatch_event` runs (`drain_reactions`) must deliver same-window
/// tasks a listener enqueued — not just custom-element reactions. A `click`
/// listener that calls `window.postMessage` enqueues a task; the `message`
/// listener must observe it within the dispatch turn. Read via `get_global`
/// (NOT `eval`, which would self-drain and mask a missing post-dispatch drain).
#[test]
fn dispatch_drains_same_window_tasks_enqueued_by_listeners() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("button", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));
    engine
        .vm()
        .eval(
            "globalThis.msg = 0;
             window.addEventListener('message', function (e) { globalThis.msg = e.data; });
             el.addEventListener('click', function () { window.postMessage(42, '*'); });",
        )
        .unwrap();
    // The setup eval registered listeners but posted nothing.
    assert!(
        matches!(engine.vm().get_global("msg"), Some(JsValue::Number(n)) if n == 0.0),
        "no message before dispatch"
    );

    let mut event = DispatchEvent::new_composed("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    // The click listener enqueued a `postMessage` task (a same-window task, not
    // a microtask — the per-listener `run_microtasks` does not drain it); the
    // post-dispatch `drain_reactions` must deliver it. Read directly — `eval`
    // would self-drain and hide the gap.
    assert!(
        matches!(engine.vm().get_global("msg"), Some(JsValue::Number(n)) if n == 42.0),
        "post-dispatch drain_reactions must deliver listener-enqueued same-window tasks"
    );
    engine.vm().unbind();
}

#[test]
fn stop_propagation_blocks_bubble_phase() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(doc, outer));
    assert!(dom.append_child(outer, inner));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let outer_w = engine.vm().inner.create_element_wrapper(outer);
    let inner_w = engine.vm().inner.create_element_wrapper(inner);
    engine.vm().set_global("outer", JsValue::Object(outer_w));
    engine.vm().set_global("inner", JsValue::Object(inner_w));

    engine
        .vm()
        .eval(
            "globalThis.outerFired = false;
             globalThis.innerFired = false;
             outer.addEventListener('click', function () { globalThis.outerFired = true; });
             inner.addEventListener('click', function (e) {
                 globalThis.innerFired = true;
                 e.stopPropagation();
             });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", inner);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert!(get_bool(&mut engine, "innerFired"));
    assert!(
        !get_bool(&mut engine, "outerFired"),
        "stopPropagation must prevent bubble-phase listener"
    );
    engine.vm().unbind();
}

#[test]
fn capture_phase_listener_fires_before_target() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(doc, outer));
    assert!(dom.append_child(outer, inner));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let outer_w = engine.vm().inner.create_element_wrapper(outer);
    let inner_w = engine.vm().inner.create_element_wrapper(inner);
    engine.vm().set_global("outer", JsValue::Object(outer_w));
    engine.vm().set_global("inner", JsValue::Object(inner_w));

    // Outer is in capture phase, fires FIRST; we use a counter to
    // confirm ordering.
    engine
        .vm()
        .eval(
            "globalThis.order = '';
             outer.addEventListener('click', function () { globalThis.order += 'O'; }, true);
             inner.addEventListener('click', function () { globalThis.order += 'I'; });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", inner);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    let JsValue::String(sid) = engine.vm().get_global("order").unwrap() else {
        panic!("order must be a string");
    };
    assert_eq!(
        engine.vm().inner.strings.get_utf8(sid),
        "OI",
        "capture (outer) must fire before at-target (inner)"
    );
    engine.vm().unbind();
}

/// Soundness regression for slot `#11-bound-safe-dispatch-dom-aliasing`.
///
/// Drives the BOUND dispatch path with a listener that MUTATES the DOM, under
/// the SOUND driving order the real shell uses: build the `ScriptContext`
/// ONCE, `bind` the engine THROUGH it (the bound `*mut EcsDom` is derived from
/// `ctx.dom`), and never create a fresh `&mut dom` afterward — dispatch is
/// then driven by passing `&mut ctx` through, which reborrows the `ctx` struct
/// but never `*ctx.dom`.
///
/// Before the bound-safe-dispatch fix, `script_dispatch_event_core` reborrowed
/// `&mut *ctx.dom` at the plan-build / retarget / once-removal sites,
/// invalidating the engine's bound `dom_ptr` under Stacked Borrows, so the
/// subsequent `call_listener` (which reads/mutates the DOM through that
/// now-invalidated pointer) was UB. With the fix, dispatch routes every dom
/// access through `engine.bound_dom_mut()` — the SAME `dom_ptr` — so there is
/// ONE derivation chain and no reborrow of `ctx.dom` while bound.
///
/// The assertions check BOTH halves: the listener fired (a global flag), and
/// its DOM mutations (`setAttribute` + `appendChild`) are visible when we read
/// the `EcsDom` back after `unbind` + dropping `ctx` — proving the engine
/// mutated the very `EcsDom` that `ctx.dom` aliases.
///
/// **Miri note**: this behavioral test asserts under the normal runner. It is
/// NOT run under miri because the listener's DOM mutation reaches a PRE-EXISTING
/// Stacked-Borrows issue in `elidex_ecs::EcsDom::dispatch_event`'s
/// mutation-dispatcher take-and-restore (`&raw mut self.dispatch_depth` is
/// invalidated by the immediately-following `self.dispatch_depth = ...` write) —
/// orthogonal to this dispatch-aliasing fix and reachable from ANY bound DOM
/// mutation. The Stacked-Borrows validation of THIS fix (the bound dispatch
/// routing) lives in the sibling
/// [`bound_dispatch_routing_is_stacked_borrows_clean`], which isolates it from
/// that elidex-ecs bug by not mutating the DOM.
#[test]
#[allow(unsafe_code)]
fn bound_dispatch_listener_dom_mutation_is_sound_and_visible() {
    use elidex_script_session::HostDriver;

    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let target = dom.create_element("button", Attributes::default());
    assert!(dom.append_child(doc, target));

    // Build `ctx` ONCE, then bind THROUGH it: the raw `*mut EcsDom` the VM
    // stores is derived from `ctx.dom`, and from here on nothing reborrows
    // `*ctx.dom`, so that raw pointer stays valid for the whole bracket.
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    // SAFETY: `ctx` (and thus `ctx.session`/`ctx.dom`) outlives the bracket and
    // is not reborrowed until `unbind`; the dispatch below routes dom access
    // through the bound pointer via `bound_dom_mut`, per the bound-safe contract.
    unsafe { HostDriver::bind(&mut engine, &mut ctx) };

    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));
    engine
        .vm()
        .eval(
            "globalThis.fired = false;
             el.addEventListener('click', function () {
                 globalThis.fired = true;
                 el.setAttribute('data-fired', 'yes');
                 el.appendChild(document.createElement('span'));
             });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", target);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert!(
        get_bool(&mut engine, "fired"),
        "bound-path listener must fire"
    );

    HostDriver::unbind(&mut engine);
    drop(ctx);

    // Read the mutations back through the same `EcsDom` `ctx.dom` aliased: if the
    // single-derivation-chain routing held, the engine's bound-pointer mutations
    // landed in THIS `dom`.
    assert_eq!(
        dom.get_attribute(target, "data-fired").as_deref(),
        Some("yes"),
        "listener's setAttribute must be visible in the shared EcsDom"
    );
    assert_eq!(
        dom.children(target).len(),
        1,
        "listener's appendChild must be visible in the shared EcsDom"
    );
}

/// Focused Stacked-Borrows validation of the bound-safe DISPATCH routing
/// (slot `#11-bound-safe-dispatch-dom-aliasing`), isolated from DOM mutation.
///
/// Same sound driving order as
/// [`bound_dispatch_listener_dom_mutation_is_sound_and_visible`] (build `ctx`
/// once, `bind` through it, never reborrow `*ctx.dom`), but the listener only
/// READS the event and sets a JS global — it does not mutate the DOM, so it
/// does NOT reach the pre-existing (orthogonal) Stacked-Borrows issue in
/// `EcsDom::dispatch_event`'s mutation-dispatcher take-and-restore pattern.
/// This isolates the fix under test: the full bound 3-phase walk
/// (`build_dispatch_plan` / `apply_retarget` / `call_listener` →
/// `ensure_event_handler_current` → `HostData::dom`) runs entirely through the
/// engine's bound `dom_ptr`, never a fresh `ctx.dom` reborrow. **Verified
/// clean under `cargo +nightly miri test` (Stacked Borrows).** Before the fix,
/// dispatch's `&mut *ctx.dom` reborrows invalidated the bound pointer and this
/// test tripped miri at `call_listener`.
#[test]
#[allow(unsafe_code)]
fn bound_dispatch_routing_is_stacked_borrows_clean() {
    use elidex_script_session::HostDriver;

    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let outer = dom.create_element("div", Attributes::default());
    let inner = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(doc, outer));
    assert!(dom.append_child(outer, inner));

    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    // SAFETY: `ctx` outlives the bracket and is not reborrowed until `unbind`;
    // dispatch routes dom access through the bound pointer via `bound_dom_mut`.
    unsafe { HostDriver::bind(&mut engine, &mut ctx) };

    let outer_w = engine.vm().inner.create_element_wrapper(outer);
    let inner_w = engine.vm().inner.create_element_wrapper(inner);
    engine.vm().set_global("outer", JsValue::Object(outer_w));
    engine.vm().set_global("inner", JsValue::Object(inner_w));
    // Listeners READ the event only (no DOM mutation). A capture + bubble +
    // once listener exercises retarget + the once-removal borrow of the bound
    // dom, all off the single derivation chain.
    engine
        .vm()
        .eval(
            "globalThis.log = '';
             outer.addEventListener('click', function (e) { globalThis.log += 'C' + e.type; }, true);
             inner.addEventListener('click', function () { globalThis.log += 'T'; }, { once: true });
             outer.addEventListener('click', function () { globalThis.log += 'B'; });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", inner);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    let JsValue::String(sid) = engine.vm().get_global("log").unwrap() else {
        panic!("log must be a string");
    };
    assert_eq!(
        engine.vm().inner.strings.get_utf8(sid),
        "CclickTB",
        "capture → at-target(once) → bubble must all fire through bound dispatch"
    );

    HostDriver::unbind(&mut engine);
    drop(ctx);
}

#[test]
fn once_listener_auto_removed_after_first_invocation() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("button", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval(
            "globalThis.count = 0;
             el.addEventListener('click', function () { globalThis.count += 1; }, { once: true });",
        )
        .unwrap();

    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let mut e1 = DispatchEvent::new_composed("click", target);
    script_dispatch_event(&mut engine, &mut e1, &mut ctx);
    let mut e2 = DispatchEvent::new_composed("click", target);
    script_dispatch_event(&mut engine, &mut e2, &mut ctx);

    let count = engine.vm().get_global("count").unwrap();
    assert_eq!(
        count,
        JsValue::Number(1.0),
        "once:true listener must fire exactly once across two dispatches"
    );
    engine.vm().unbind();
}

#[test]
fn once_signal_listener_prunes_abort_back_ref() {
    // Regression for the {once, signal} interaction: when a listener
    // registered with both `{once: true}` and `{signal}` fires once,
    // the auto-removal path goes through `Engine::remove_listener`
    // (in `event_dispatch::dispatch_phase`) — not through
    // `removeEventListener`.  Both paths must scrub the AbortSignal
    // back-ref index, otherwise repeated `addEventListener({once,
    // signal})` + dispatch cycles leak entries in
    // `abort_listener_back_refs` and the per-signal back-ref
    // HashMap.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("button", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval(
            "globalThis.c = new AbortController();
             el.addEventListener('click', function () {}, {once: true, signal: c.signal});",
        )
        .unwrap();

    // Pre-dispatch: one back-ref entry, listener registered.
    assert_eq!(
        engine.vm().inner.abort_listener_back_refs.len(),
        1,
        "back-ref should exist before the listener fires"
    );

    // Fire the event — `{once}` auto-removal triggers
    // `Engine::remove_listener`, which must also scrub the back-ref.
    let mut event = DispatchEvent::new_composed("click", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert_eq!(
        engine.vm().inner.abort_listener_back_refs.len(),
        0,
        "{{once,signal}} auto-removal must prune `abort_listener_back_refs`"
    );

    // The per-signal HashMap must also be empty so a subsequent
    // `controller.abort()` does no spurious detach work.
    let signal_id = match engine.vm().eval("c.signal;").unwrap() {
        JsValue::Object(id) => id,
        other => panic!("c.signal is not an object: {other:?}"),
    };
    let removals_count = engine
        .vm()
        .inner
        .abort_signal_states
        .get(&signal_id)
        .map_or(usize::MAX, |s| s.bound_listener_removals.len());
    assert_eq!(
        removals_count, 0,
        "per-signal `bound_listener_removals` must drop the entry too"
    );
    engine.vm().unbind();
}

#[test]
fn passive_listener_prevent_default_does_not_propagate_to_return() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("div", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval(
            "el.addEventListener('touchstart',
                function (e) { e.preventDefault(); },
                { passive: true });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("touchstart", target);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let prevented = script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert!(
        !prevented,
        "passive listener's preventDefault must not affect dispatch outcome"
    );
    engine.vm().unbind();
}

#[test]
fn mouse_event_payload_visible_to_listener() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("button", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval(
            "globalThis.x = -1;
             globalThis.y = -1;
             el.addEventListener('click', function (e) {
                 globalThis.x = e.clientX;
                 globalThis.y = e.clientY;
             });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("click", target);
    event.payload = EventPayload::Mouse(MouseEventInit {
        client_x: 75.0,
        client_y: 99.0,
        ..Default::default()
    });
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    assert_eq!(engine.vm().get_global("x").unwrap(), JsValue::Number(75.0));
    assert_eq!(engine.vm().get_global("y").unwrap(), JsValue::Number(99.0));
    engine.vm().unbind();
}

#[test]
fn keyboard_event_key_property_visible() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let target = dom.create_element("input", Attributes::default());
    assert!(dom.append_child(doc, target));

    bind_after_dom(&mut engine, &mut session, &mut dom, doc);
    let wrapper = engine.vm().inner.create_element_wrapper(target);
    engine.vm().set_global("el", JsValue::Object(wrapper));

    engine
        .vm()
        .eval(
            "globalThis.last_key = '';
             el.addEventListener('keydown', function (e) { globalThis.last_key = e.key; });",
        )
        .unwrap();

    let mut event = DispatchEvent::new_composed("keydown", target);
    event.payload = EventPayload::Keyboard(KeyboardEventInit {
        key: "Escape".into(),
        code: "Escape".into(),
        ..Default::default()
    });
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    script_dispatch_event(&mut engine, &mut event, &mut ctx);

    let JsValue::String(sid) = engine.vm().get_global("last_key").unwrap() else {
        panic!("last_key must be a string");
    };
    assert_eq!(engine.vm().inner.strings.get_utf8(sid), "Escape");
    engine.vm().unbind();
}
