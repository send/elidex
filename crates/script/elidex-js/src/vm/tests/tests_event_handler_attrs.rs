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

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{eval_str, setup_with_element};
use super::super::value::JsValue;
use super::super::Vm;

/// Run `body` against a VM bound to a document with a single `<div>`
/// exposed as the JS global `el`.
///
/// `session` / `dom` are kept alive as locals for the whole closure:
/// [`Vm::bind`] (via [`setup_with_element`]) stores raw pointers into
/// them, so they MUST NOT be moved (e.g. returned from a helper) while
/// the VM is bound — doing so dangles the pointers (UB / SIGBUS).
fn with_el(body: impl FnOnce(&mut Vm)) {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    let _el = unsafe { setup_with_element(&mut vm, &mut session, &mut dom, doc, "div") };
    body(&mut vm);
    vm.unbind();
}

/// Evaluate `src` and expect a boolean result.
fn eval_bool(vm: &mut Vm, src: &str) -> bool {
    match vm.eval(src).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected boolean, got {other:?}"),
    }
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
