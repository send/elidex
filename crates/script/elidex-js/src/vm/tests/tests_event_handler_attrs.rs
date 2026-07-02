//! D-28 `#11-event-handler-attribute-vm` integration tests.
//!
//! Drives event-handler IDL attributes through real JS
//! (`el.onclick = fn` / `el.setAttribute('onclick', ...)` /
//! `el.innerHTML = '<button onclick=...>'` / `<body>.onbeforeunload`)
//! and verifies dispatch, getter round-trips, kind-distinct identity,
//! last-write-wins, and bound-key event-type threading.
//!
//! WHATWG HTML §8.1.8.1 / §8.1.8.2.  Compiled only under `engine`.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

use super::super::test_helpers::{eval_str, setup_with_element};
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

/// RAII guard that unbinds the VM on drop — including during a panic
/// unwind, so `session`/`dom` are never dropped while the VM still holds
/// raw pointers into them (upholds [`Vm::bind`]'s safety contract even
/// when a test assertion panics inside the closure).
struct BoundVm(Vm);
impl Drop for BoundVm {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// Run `body` against a VM bound to a document with a single `<div>`
/// exposed as the JS global `el`.
///
/// `session` / `dom` are declared **before** the VM so that, on unwind,
/// the `BoundVm` guard (dropped first) unbinds before they are dropped —
/// [`Vm::bind`] (via [`setup_with_element`]) stores raw pointers into
/// them, so they MUST stay live and non-aliased until `unbind`.
fn with_el(body: impl FnOnce(&mut Vm)) {
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let mut vm = Vm::new();
    #[allow(unsafe_code)]
    let _el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };
    let mut guard = BoundVm(vm);
    body(&mut guard.0);
    // `guard` drops here (or on unwind), unbinding before `dom`/`session`.
}

/// Evaluate `src` and expect a boolean result.
fn eval_bool(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected boolean, got {other:?}"),
    }
}

/// Resolve the `Entity` behind a `HostObject` wrapper value.
fn entity_of(vm: &Vm, value: JsValue) -> Entity {
    let JsValue::Object(id) = value else {
        panic!("value is not an object: {value:?}")
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(id).kind else {
        panic!("value is not a HostObject")
    };
    Entity::from_bits(entity_bits).expect("valid entity bits")
}

/// Evaluate `src` and expect a number result.
fn eval_num(vm: &mut Vm, src: &str) -> f64 {
    match vm.eval(src).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

// ---- F-1: getter round-trip ----
#[test]
fn idl_setter_getter_roundtrip() {
    with_el(|vm| {
        assert!(eval_bool(
            vm,
            "var f = function () {}; el.onclick = f; el.onclick === f",
        ));
    });
}

// ---- F-2: dispatch invokes handler with `this === el` ----
#[test]
fn idl_handler_fires_with_currenttarget_this() {
    with_el(|vm| {
        vm.eval(
            "globalThis.hit = false; \
             el.onclick = function () { globalThis.hit = (this === el); }; \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.hit"));
    });
}

// ---- F-3: registration order interleaves with addEventListener ----
#[test]
fn registration_order_interleaves_with_add_event_listener() {
    with_el(|vm| {
        vm.eval(
            "globalThis.order = ''; \
             el.addEventListener('click', function () { globalThis.order += 'a'; }); \
             el.onclick = function () { globalThis.order += 'b'; }; \
             el.addEventListener('click', function () { globalThis.order += 'c'; }); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_str(vm, "globalThis.order"), "abc");
    });
}

// ---- F-4: second IDL set overwrites the same listener (no dup) ----
#[test]
fn second_idl_set_overwrites_without_duplicate() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             el.onclick = function () { globalThis.n += 1; }; \
             el.onclick = function () { globalThis.n += 10; }; \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(
            eval_num(vm, "globalThis.n"),
            10.0,
            "only second handler fires once"
        );
    });
}

// ---- F-5: clear to null → no-op + getter null ----
#[test]
fn idl_null_clears_handler() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             el.onclick = function () { globalThis.n += 1; }; \
             el.onclick = null; \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 0.0);
        assert!(eval_bool(vm, "el.onclick === null"));
    });
}

// ---- F-6: set → null → set keeps original registration slot ----
#[test]
fn set_null_set_restores_handler() {
    with_el(|vm| {
        vm.eval(
            "globalThis.order = ''; \
             el.addEventListener('click', function () { globalThis.order += 'a'; }); \
             el.onclick = function () { globalThis.order += 'x'; }; \
             el.addEventListener('click', function () { globalThis.order += 'c'; }); \
             el.onclick = null; \
             el.onclick = function () { globalThis.order += 'b'; }; \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // Handler slot keeps its original mid position → a, b, c.
        assert_eq!(eval_str(vm, "globalThis.order"), "abc");
    });
}

// ---- F-7: non-callable assignment → null, no-op ----
#[test]
fn non_callable_assignment_clears() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             el.onclick = function () { globalThis.n += 1; }; \
             el.onclick = 42; \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 0.0);
        assert!(eval_bool(vm, "el.onclick === null"));
    });
}

// ---- F-8: inline handler via setAttribute fires (Arm 1) ----
#[test]
fn inline_set_attribute_handler_fires() {
    with_el(|vm| {
        vm.eval(
            "globalThis.x = 0; \
             el.setAttribute('onclick', 'globalThis.x = 1'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.x"), 1.0);
    });
}

// ---- F-9: removeAttribute clears inline handler ----
#[test]
fn remove_attribute_clears_inline_handler() {
    with_el(|vm| {
        vm.eval(
            "globalThis.x = 0; \
             el.setAttribute('onclick', 'globalThis.x = 1'); \
             el.removeAttribute('onclick'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.x"), 0.0);
    });
}

// ---- F-10: innerHTML baked inline handler fires (Arm 2) ----
#[test]
fn inner_html_baked_handler_fires() {
    with_el(|vm| {
        vm.eval(
            "globalThis.z = 0; \
             el.innerHTML = '<button onclick=\"globalThis.z = 3\">x</button>'; \
             el.querySelector('button').dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.z"), 3.0);
    });
}

// ---- F-10c: nested innerHTML descendant handler fires (Arm 2 walk) ----
#[test]
fn inner_html_nested_descendant_handler_fires() {
    with_el(|vm| {
        vm.eval(
            "globalThis.w = 0; \
             el.innerHTML = '<div><button onclick=\"globalThis.w = 4\"></button></div>'; \
             el.querySelector('button').dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.w"), 4.0);
    });
}

// ---- F-11: inline syntax error → null handler, no throw ----
#[test]
fn inline_syntax_error_is_silent() {
    with_el(|vm| {
        // Must not throw at dispatch; handler simply does not run.
        vm.eval(
            "globalThis.ran = false; \
             el.setAttribute('onclick', '((('); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert!(!eval_bool(vm, "globalThis.ran"));
        // §8.1.8.1 "if body is not parsable" → handler value is null.
        assert!(eval_bool(vm, "el.onclick === null"));
    });
}

// ---- F-12: <body>.onbeforeunload delegates to Window ----
#[test]
fn body_weh_delegates_to_window() {
    with_el(|vm| {
        assert!(eval_bool(
            vm,
            "var b = document.createElement('body'); \
             var f = function () {}; \
             b.onbeforeunload = f; \
             window.onbeforeunload === f",
        ));
    });
}

// ---- F-13: <body>.onclick stays on the body (GlobalEventHandlers) ----
#[test]
fn body_geh_is_not_delegated() {
    with_el(|vm| {
        assert!(eval_bool(
            vm,
            "var b = document.createElement('body'); \
             var f = function () {}; \
             b.onclick = f; \
             b.onclick === f && window.onclick === null",
        ));
    });
}

// ---- F-14: window.onload fires on Window dispatch ----
#[test]
fn window_handler_fires() {
    with_el(|vm| {
        vm.eval(
            "globalThis.loaded = false; \
             window.onload = function () { globalThis.loaded = true; }; \
             window.dispatchEvent(new Event('load'));",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.loaded"));
    });
}

// ---- F-15: handler throw does not abort dispatch ----
#[test]
fn handler_throw_does_not_abort_dispatch() {
    with_el(|vm| {
        vm.eval(
            "globalThis.after = false; \
             el.onclick = function () { throw new Error('boom'); }; \
             el.addEventListener('click', function () { globalThis.after = true; }); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.after"));
    });
}

// ---- F-16: removeEventListener does NOT remove a handler listener ----
#[test]
fn remove_event_listener_ignores_handler_listener() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             var f = function () { globalThis.n += 1; }; \
             el.onclick = f; \
             el.removeEventListener('click', f); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 1.0, "handler still fires");
    });
}

// ---- F-17: handler + addEventListener with same fn → fires twice ----
#[test]
fn handler_and_listener_are_distinct() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             var f = function () { globalThis.n += 1; }; \
             el.onclick = f; \
             el.addEventListener('click', f); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 2.0);
    });
}

// ---- F-18: inline handler compiles lazily (not eagerly run) ----
#[test]
fn inline_handler_compiles_lazily() {
    with_el(|vm| {
        vm.eval("globalThis.c = 0; el.setAttribute('onclick', 'globalThis.c = (globalThis.c || 0) + 1');")
            .unwrap();
        // Setting the attribute must not run (or eagerly compile-and-run) it.
        assert_eq!(eval_num(vm, "globalThis.c"), 0.0);
        vm.eval("el.dispatchEvent(new Event('click'));").unwrap();
        assert_eq!(eval_num(vm, "globalThis.c"), 1.0);
    });
}

// ---- F-19: compiled handler survives GC pressure (listener_store root) ----
#[test]
fn handler_callable_survives_gc() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             el.onclick = function () { globalThis.n += 1; }; \
             for (var i = 0; i < 2000; i++) { var junk = { a: i, b: [i, i + 1] }; } \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 1.0);
    });
}

// ---- F-20a: last-write-wins — content-attr after IDL ----
#[test]
fn last_write_wins_content_attr_after_idl() {
    with_el(|vm| {
        vm.eval(
            "globalThis.q = 0; \
             el.onclick = function () { globalThis.q = 99; }; \
             el.setAttribute('onclick', 'globalThis.q = 1'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.q"), 1.0);
    });
}

// ---- F-20b: last-write-wins — IDL after content-attr ----
#[test]
fn last_write_wins_idl_after_content_attr() {
    with_el(|vm| {
        vm.eval(
            "globalThis.q = 0; \
             el.setAttribute('onclick', 'globalThis.q = 1'); \
             el.onclick = function () { globalThis.q = 99; }; \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.q"), 99.0);
    });
}

// ---- F-21c: removeAttribute is a no-op for an IDL-set handler ----
#[test]
fn remove_attribute_does_not_clear_idl_handler() {
    with_el(|vm| {
        vm.eval(
            "globalThis.n = 0; \
             el.onclick = function () { globalThis.n += 1; }; \
             el.removeAttribute('onclick'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 1.0);
    });
}

// ---- F-9b: inline handler cleared after a prior lazy compile (Copilot R1 #3) ----
#[test]
fn inline_handler_cleared_after_compile_on_remove_attribute() {
    with_el(|vm| {
        // Compile + fire once (drains uncompiled, stores the callable).
        vm.eval(
            "globalThis.n = 0; \
             el.setAttribute('onclick', 'globalThis.n += 1'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert_eq!(eval_num(vm, "globalThis.n"), 1.0);
        // Remove the content attribute, then dispatch again — the
        // already-compiled callable must NOT fire (handler value is null).
        vm.eval("el.removeAttribute('onclick'); el.dispatchEvent(new Event('click'));")
            .unwrap();
        assert_eq!(
            eval_num(vm, "globalThis.n"),
            1.0,
            "cleared handler must not fire"
        );
        assert!(eval_bool(vm, "el.onclick === null"));
    });
}

// ---- F-23: normal-pair brand check rejects non-Element/Document/Window (Copilot R1 #2) ----
#[test]
fn handler_accessor_rejects_text_receiver() {
    with_el(|vm| {
        vm.eval(
            "globalThis.r = 'init'; globalThis.msg = ''; \
             var p = Object.getPrototypeOf(el); \
             while (p && !Object.getOwnPropertyDescriptor(p, 'onclick')) p = Object.getPrototypeOf(p); \
             var setter = Object.getOwnPropertyDescriptor(p, 'onclick').set; \
             var t = document.createTextNode('x'); \
             try { setter.call(t, function () {}); globalThis.r = 'no-throw'; } \
             catch (e) { globalThis.r = 'threw'; globalThis.msg = String(e.message); }",
        )
        .unwrap();
        assert_eq!(eval_str(vm, "globalThis.r"), "threw");
        // Copilot R2 #2: the Illegal-invocation message names the IDL
        // property ("onclick"), not the event type ("click").
        assert!(
            eval_str(vm, "globalThis.msg").contains("onclick"),
            "brand-check error message should name the property 'onclick'"
        );
    });
}

// ---- F-12b: inline `<body on{weh}>` content attribute delegates to Window (Copilot R2 #1) ----
#[test]
fn inline_body_weh_attribute_delegates_to_window() {
    with_el(|vm| {
        // A WindowEventHandlers content attribute on <body> must be
        // recorded on the Window (where the IDL accessor + dispatch read),
        // not the body entity (WHATWG HTML §8.1.8.2).
        vm.eval(
            "globalThis.popped = 0; \
             var b = document.createElement('body'); \
             b.setAttribute('onpopstate', 'globalThis.popped = 1');",
        )
        .unwrap();
        // Visible through the Window IDL getter (compiles the inline source).
        assert!(eval_bool(vm, "window.onpopstate !== null"));
        // And fires on a Window dispatch.
        vm.eval("window.dispatchEvent(new Event('popstate'));")
            .unwrap();
        assert_eq!(eval_num(vm, "globalThis.popped"), 1.0);
    });
}

// ---- F-24: body-delegation brand check rejects non-<body> element (Copilot R1 #1) ----
#[test]
fn body_weh_accessor_rejects_non_body_receiver() {
    with_el(|vm| {
        vm.eval(
            "globalThis.r = 'init'; \
             var b = document.createElement('body'); \
             var setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(b), 'onbeforeunload').set; \
             var d = document.createElement('div'); \
             try { setter.call(d, function () {}); globalThis.r = 'no-throw'; } \
             catch (e) { globalThis.r = 'threw'; }",
        )
        .unwrap();
        assert_eq!(eval_str(vm, "globalThis.r"), "threw");
    });
}

// ---- F-22: distinct event types resolve independently (bound key) ----
#[test]
fn distinct_event_types_resolve_independently() {
    with_el(|vm| {
        vm.eval(
            "globalThis.log = ''; \
             globalThis.a = function () { globalThis.log += 'a'; }; \
             globalThis.b = function () { globalThis.log += 'b'; }; \
             el.onclick = globalThis.a; \
             el.onmousedown = globalThis.b;",
        )
        .unwrap();
        // Getter resolves each attribute to its own callable.
        assert!(eval_bool(
            vm,
            "el.onclick === globalThis.a && el.onmousedown === globalThis.b",
        ));
        // Dispatch routes each event type only to its own handler.
        vm.eval("el.dispatchEvent(new Event('click'));").unwrap();
        assert_eq!(eval_str(vm, "globalThis.log"), "a");
        vm.eval("el.dispatchEvent(new Event('mousedown'));")
            .unwrap();
        assert_eq!(eval_str(vm, "globalThis.log"), "ab");
    });
}

// ---------------------------------------------------------------------------
// S5-4a: HTML §8.1.8.1 scripting-disabled gates — the COMPILE gate
// ("getting the current value of the event handler" step 3.2, flag-only)
// vs the INVOKE gate ("the event handler processing algorithm" step 1,
// full §8.1.3.4 predicate incl. the platform-object clause (b)).
// ---------------------------------------------------------------------------

// ---- S5-4a: compile gate — sandboxed doc keeps getter null, dispatch runs nothing ----
#[test]
fn sandboxed_no_allow_scripts_compile_gate_nulls_getter_and_dispatch() {
    with_el(|vm| {
        vm.host_data()
            .expect("bound VM has HostData installed")
            .set_sandbox_flags(Some(elidex_plugin::IframeSandboxFlags::empty()));
        vm.eval(
            "globalThis.ran = false; \
             el.setAttribute('onclick', 'globalThis.ran = true'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // Step 3.2: the raw inline source never compiles → getter is null.
        assert!(eval_bool(vm, "el.onclick === null"));
        assert!(!eval_bool(vm, "globalThis.ran"));
    });
}

// ---- S5-4a: step-1 invoke gate — compiled handler on a browsing-context-null
// document's node is suppressed; addEventListener on the same target is NOT ----
#[test]
fn compiled_handler_on_null_bc_document_node_suppressed_listener_still_runs() {
    with_el(|vm| {
        // A DOMParser document has no browsing context (§8.1.3.4 clause (b):
        // the target implements Node and its node document's browsing
        // context is null). The handler COMPILES (the IDL setter stores the
        // callable; scripting is enabled settings-level), so only the step-1
        // invocation gate can suppress it.
        vm.eval(
            "globalThis.handlerRan = false; globalThis.listenerRan = false; \
             var doc2 = new DOMParser().parseFromString('<div id=\"d\"></div>', 'text/html'); \
             globalThis.d = doc2.getElementById('d'); \
             globalThis.d.onclick = function () { globalThis.handlerRan = true; }; \
             globalThis.d.addEventListener('click', function () { globalThis.listenerRan = true; }); \
             globalThis.d.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // Handler-derived callable: invocation suppressed (step 1).
        assert!(!eval_bool(vm, "globalThis.handlerRan"));
        // Plain listener: step 1 gates event HANDLERS only.
        assert!(eval_bool(vm, "globalThis.listenerRan"));
        // Step 1 suppresses INVOCATION only — the compiled callable is still
        // the IDL getter's value (deleting it would be the step-3.2 conflation).
        assert!(eval_bool(vm, "typeof globalThis.d.onclick === 'function'"));
    });
}

// ---- S5-4a: step-1 gate precedes step 2 — a suppressed target's raw inline
// source is NOT compiled during dispatch (§8.1.8.1 step 1 fires before
// "getting the current value of the event handler") ----
#[test]
fn suppressed_target_dispatch_does_not_compile_raw_inline_handler() {
    with_el(|vm| {
        vm.eval(
            "globalThis.ran = false; \
             var doc2 = new DOMParser().parseFromString('<div id=\"d\"></div>', 'text/html'); \
             globalThis.d = doc2.getElementById('d'); \
             globalThis.d.setAttribute('onclick', 'globalThis.ran = true'); \
             globalThis.d.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert!(!eval_bool(vm, "globalThis.ran"));
        // Oracle: the raw inline source is STILL uncompiled after dispatch —
        // the step-1 gate returned before the step-2 compile, so no callable
        // was ever stored for the handler's ListenerId. (Reading the IDL
        // getter would itself lazily compile, so inspect the store directly.)
        let d_val = vm.eval("globalThis.d").unwrap();
        let d = entity_of(vm, d_val);
        let host = vm.host_data().expect("bound VM has HostData installed");
        let id = {
            let listeners = host
                .dom()
                .world_mut()
                .get::<&elidex_script_session::EventListeners>(d)
                .expect("target has an EventListeners component");
            listeners
                .find_event_handler("click")
                .expect("handler listener entry survives the suppressed dispatch")
        };
        assert!(
            host.get_listener(id).is_none(),
            "suppressed dispatch must not compile the raw inline handler"
        );
    });
}

// ---- S5-4a: step-1 gate regression pins — targets whose node document IS the
// bound document (or that are not Nodes at all) stay un-suppressed ----
#[test]
fn step1_gate_does_not_suppress_bound_document_targets() {
    with_el(|vm| {
        vm.eval(
            "globalThis.docRan = false; \
             globalThis.detachedRan = false; \
             globalThis.windowRan = false; \
             document.onclick = function () { globalThis.docRan = true; }; \
             document.dispatchEvent(new Event('click')); \
             var e2 = document.createElement('div'); \
             e2.onclick = function () { globalThis.detachedRan = true; }; \
             e2.dispatchEvent(new Event('click')); \
             window.onpopstate = function () { globalThis.windowRan = true; }; \
             window.dispatchEvent(new Event('popstate'));",
        )
        .unwrap();
        // The bound Document: node document is itself — clause (b) must not
        // misread `owner_document == None` (Document.ownerDocument is null).
        assert!(eval_bool(vm, "globalThis.docRan"));
        // A detached main-document element: node document is the bound
        // document (browsing context non-null) — handlers run.
        assert!(eval_bool(vm, "globalThis.detachedRan"));
        // The Window entity is not a Node (clause (c) never fires while
        // bound) — settings-level only, which is enabled here.
        assert!(eval_bool(vm, "globalThis.windowRan"));
    });
}

// ---- S5-4a (PR #444 Codex R2): adopt-equivalent clause (b) — a DOMParser
// node APPENDED into the bound document's tree is NOT suppressed. elidex's
// insertion path does not run DOM §4.2.3 pre-insert adoption (append_child
// relinks; `AssociatedDocument` stays stale = doc2), so the predicate must
// resolve the node document via the composed tree root (the `isConnected`
// query): in-tree ⇒ the bound document, spec-adopt-equivalently. The missing
// insertion-adoption itself is defer slot `#11-cross-document-adopt-on-insert`. ----
#[test]
fn appended_domparser_node_handler_runs_adopt_equivalent() {
    with_el(|vm| {
        vm.eval(
            "globalThis.ran = false; \
             var doc2 = new DOMParser().parseFromString('<div id=\"d\"></div>', 'text/html'); \
             globalThis.d = doc2.getElementById('d'); \
             document.appendChild(globalThis.d); \
             globalThis.d.onclick = function () { globalThis.ran = true; }; \
             globalThis.d.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // In the bound document's tree ⇒ node document IS the bound document
        // (a spec-correct adopt would have re-homed it) ⇒ handler runs.
        assert!(eval_bool(vm, "globalThis.ran"));
    });
}

// ---- S5-4a (PR #444 Codex R2): the adopt-equivalence is tree-membership,
// not a one-way latch — removing the node detaches it with its stale
// `AssociatedDocument` (doc2) intact, so clause (b) suppresses again. ----
#[test]
fn appended_then_removed_domparser_node_suppressed_again() {
    with_el(|vm| {
        vm.eval(
            "globalThis.count = 0; \
             var doc2 = new DOMParser().parseFromString('<div id=\"d\"></div>', 'text/html'); \
             globalThis.d = doc2.getElementById('d'); \
             globalThis.d.onclick = function () { globalThis.count += 1; }; \
             document.appendChild(globalThis.d); \
             globalThis.d.dispatchEvent(new Event('click')); \
             document.removeChild(globalThis.d); \
             globalThis.d.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // First dispatch (in-tree): runs. Second (detached again, owner
        // falls back to the stale doc2): suppressed.
        assert_eq!(eval_str(vm, "String(globalThis.count)"), "1");
    });
}

// ---- S5-4a (PR #444 Codex R4): the MIRROR of the adopt-equivalent case —
// a node CREATED by the bound document, handler assigned, then appended INTO
// a DOMParser (null-BC) document. `append_child` does not adopt, so the stale
// `AssociatedDocument` still points at the bound document; the directional
// rule would read that and NOT suppress. But the node's effective node
// document is its composed tree root (the foreign Document, browsing context
// null) → clause (b) SUPPRESSES. The unified effective-document rule reads the
// tree root, not the stale owner. Adoption itself = `#11-cross-document-adopt-on-insert`. ----
#[test]
fn bound_created_node_appended_into_domparser_doc_suppressed() {
    with_el(|vm| {
        vm.eval(
            "globalThis.ran = false; \
             var doc2 = new DOMParser().parseFromString('<div id=\"host\"></div>', 'text/html'); \
             globalThis.e = document.createElement('div'); \
             globalThis.e.onclick = function () { globalThis.ran = true; }; \
             doc2.getElementById('host').appendChild(globalThis.e); \
             globalThis.e.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // Appended into the foreign (null-BC) tree ⇒ effective node document
        // is the foreign root ⇒ handler SUPPRESSED (step 1), even though the
        // stale owner still points at the bound document.
        assert!(!eval_bool(vm, "globalThis.ran"));
    });
}
