//! IBP-sandbox N1: HTML §8.1.8.1 scripting gates for event handlers, driven
//! through real JS (script through `vm.eval`; the compile gate reached by
//! dispatch and by the IDL getter).
//!
//! The only §8.1.3.4 reader on a handler path is the compile gate — *get the
//! current value of the event handler* step 3.2 — consulted for an *internal
//! raw uncompiled handler* only (`reconcile_event_handler`); *the event
//! handler processing algorithm* has no scripting check at invocation, so a
//! callback value runs wherever it moves (C8b). Cells: C8, C8b (i)–(iv), C8c,
//! C8d, C8e (VM half), C8g, C8h, C30, C31.
//!
//! Split out of `tests_event_handler_attrs.rs` (touch-time split; the IDL /
//! content-attribute handler basics stay there).
//!
//! Compiled only under `engine`.

#![cfg(feature = "engine")]

use elidex_ecs::Entity;

use super::super::test_helpers::{entity_of, eval_str, listeners_on, run, with_page};
use super::super::Vm;
use super::tests_event_handler_attrs::{eval_bool, with_el};

/// The `click` handler entry's pending (uncompiled) source on `entity` — the
/// **source oracle**: non-draining, so it fails under an arm 3 that takes the
/// source. Panics if there is no `click` entry.
fn click_source(vm: &mut Vm, entity: Entity) -> Option<String> {
    let listeners = listeners_on(vm, entity);
    let id = listeners
        .find_event_handler("click")
        .expect("a click handler entry exists");
    listeners.uncompiled_source(id).map(str::to_owned)
}

// ---- compile gate, settings clause: a sandboxed document keeps the getter null, dispatch runs nothing ----
#[test]
fn sandboxed_no_allow_scripts_compile_gate_nulls_getter_and_dispatch() {
    with_el(|vm| {
        vm.host_data()
            .expect("bound VM has HostData installed")
            .set_sandbox_flags(elidex_plugin::SandboxingFlagSet::all_represented());
        vm.eval(
            "globalThis.ran = false; \
             el.setAttribute('onclick', 'globalThis.ran = true'); \
             el.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        // *Get the current value* step 3.2: the raw inline source never
        // compiles → getter is null.
        assert!(eval_bool(vm, "el.onclick === null"));
        assert!(!eval_bool(vm, "globalThis.ran"));
    });
}

// ---- C8b (i): a callback value on a browsing-context-null document's node runs
// (the processing algorithm has no §8.1.3.4 check at invocation); the plain
// listener runs too, and the IDL getter still returns the callable ----
#[test]
fn idl_set_handler_on_domparser_node_runs() {
    with_el(|vm| {
        // A DOMParser document has no browsing context. The handler is a
        // callback object (the IDL setter stores the callable), so no gate is
        // consulted: step 3 of *get the current value* applies to a raw value
        // only.
        vm.eval(
            "globalThis.handlerRan = false; globalThis.listenerRan = false; \
             var doc2 = new DOMParser().parseFromString('<div id=\"d\"></div>', 'text/html'); \
             globalThis.d = doc2.getElementById('d'); \
             globalThis.d.onclick = function () { globalThis.handlerRan = true; }; \
             globalThis.d.addEventListener('click', function () { globalThis.listenerRan = true; }); \
             globalThis.d.dispatchEvent(new Event('click'));",
        )
        .unwrap();
        assert!(eval_bool(vm, "globalThis.handlerRan"));
        assert!(eval_bool(vm, "globalThis.listenerRan"));
        assert!(eval_bool(vm, "typeof globalThis.d.onclick === 'function'"));
    });
}

// ---- C8: compile gate on a DOMParser node — a raw inline source is never
// compiled (the getter reads null, dispatch runs nothing), and the source is
// KEPT (a later read where the gate does not fire compiles it) ----
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
        let d_val = vm.eval("globalThis.d").unwrap();
        let d = entity_of(vm, d_val);
        // Store oracle: no callable was ever stored for the handler's
        // ListenerId (inspect the store directly).
        let assert_no_callable = |vm: &mut Vm| {
            let id = listeners_on(vm, d)
                .find_event_handler("click")
                .expect("handler listener entry survives the suppressed dispatch");
            let host = vm.host_data().expect("bound VM has HostData installed");
            assert!(
                host.get_listener(id).is_none(),
                "suppressed read must not compile the raw inline handler"
            );
        };
        assert_no_callable(vm);
        // Source oracle: still pending after the dispatch (a take-the-source
        // arm 3 fails here, the store oracle alone does not see it).
        assert_eq!(
            click_source(vm, d).as_deref(),
            Some("globalThis.ran = true")
        );
        // The getter reads null, and arm 3 is idempotent: store and source
        // are unchanged after it.
        assert!(eval_bool(vm, "globalThis.d.onclick === null"));
        assert_no_callable(vm);
        assert_eq!(
            click_source(vm, d).as_deref(),
            Some("globalThis.ran = true")
        );
    });
}

// ---- regression pins, raw form (O6): targets whose node document IS the bound
// document stay un-gated — the Document (IDL-set: no raw producer reaches it),
// a detached main-document element, and the Window (a Window-delegating
// `<body onpopstate>` pair: the raw value is written on the Window as at HEAD,
// where the gate is the settings clause alone) ----
#[test]
fn compile_gate_does_not_gate_bound_document_targets() {
    with_el(|vm| {
        vm.eval(
            "globalThis.docRan = false; \
             globalThis.detachedRan = false; \
             globalThis.windowRan = false; \
             document.onclick = function () { globalThis.docRan = true; }; \
             document.dispatchEvent(new Event('click')); \
             var e2 = document.createElement('div'); \
             e2.setAttribute('onclick', 'globalThis.detachedRan = true'); \
             e2.dispatchEvent(new Event('click')); \
             var b2 = document.createElement('body'); \
             b2.setAttribute('onpopstate', 'globalThis.windowRan = true'); \
             window.dispatchEvent(new Event('popstate'));",
        )
        .unwrap();
        // The bound Document: node document is itself.
        assert!(eval_bool(vm, "globalThis.docRan"));
        // A detached main-document element: the node document is the bound
        // document — its raw handler compiles and runs.
        assert!(eval_bool(vm, "globalThis.detachedRan"));
        // The Window entity: settings clause only (enabled here); HEAD's
        // outcome under X-W — the raw value lands on the Window and runs.
        assert!(eval_bool(vm, "globalThis.windowRan"));
    });
}

// ---- C31: mirror of C8, production form — a DOMParser node APPENDED into the
// bound document's tree has the bound document as its effective node document
// (the composed tree root; elidex's insertion does not adopt, DOM §4.2.3
// insert step 7.1, slot `#11-cross-document-adopt-on-insert`), so a raw handler
// set on it compiles and runs ----
#[test]
fn appended_domparser_node_raw_handler_runs_adopt_equivalent() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.ran = 0; \
             var dx = new DOMParser().parseFromString('<div id=\"d\"></div>', 'text/html'); \
             var d = dx.getElementById('d'); \
             document.body.append(d); \
             d.setAttribute('onclick', 'globalThis.ran++'); \
             d.dispatchEvent(new MouseEvent('click')); \
             r.push(ran === 1, typeof d.onclick === 'function');",
        );
        assert_eq!(got, "true,true", "C31: the raw handler compiled and ran");
    });
}

// ---- C8b (ii): a callback value runs wherever it moves — appended into the
// live document, removed (back to the DOMParser owner), it still runs (nothing
// gates the call; the value is a callback) ----
#[test]
fn appended_then_removed_domparser_node_callback_still_runs() {
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
        assert_eq!(eval_str(vm, "String(globalThis.count)"), "2");
    });
}

// ---- C8b (iii): a node CREATED by the bound document, callback assigned, then
// appended INTO a DOMParser document — the callback value runs (`append_child`
// does not adopt; the effective node document is the foreign root, but a
// callback value never meets the gate). C30 is the raw-form reverse case ----
#[test]
fn bound_created_node_appended_into_domparser_doc_callback_runs() {
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
        assert!(eval_bool(vm, "globalThis.ran"));
    });
}

// ---- C8b (iv): `setAttribute` compiled while live (the element is marked at
// that write — its only deliverable ordinary handler attribute — so the later
// move skips it, K-1 / K-2), then moved into a DOMParser document: the value
// stays the callback and runs (arm 2) ----
#[test]
fn c8b_iv_compiled_attribute_handler_survives_move_into_domparser_doc() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.ran = 0; \
             var e = document.createElement('div'); \
             document.body.append(e); \
             e.setAttribute('onclick', 'globalThis.ran++'); \
             e.dispatchEvent(new Event('click')); \
             r.push(ran === 1); \
             dp.body.append(e); \
             e.dispatchEvent(new Event('click')); \
             r.push(ran === 2, typeof e.onclick === 'function');",
        );
        assert_eq!(got, "true,true,true", "C8b(iv)");
    });
}

// ---- C8c: a stale callable after a re-set (arm 3 drops it, keeps the source).
// `A` compiled live, moved into a DOMParser document (callback `A` stays,
// K-1), `setAttribute('onclick', B)` writes raw `B` while the store still holds
// `A` → neither runs, the getter is null, the source oracle reads `B`; moved
// back live (marked: the move skips it, so the value stays raw `B`) → `B` runs
// (arm 4) — which a take-the-source arm 3 would not do ----
#[test]
fn c8c_reset_attribute_in_domparser_doc_drops_stale_callable_keeps_source() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.log = ''; \
             globalThis.e = document.createElement('div'); \
             document.body.append(e); \
             e.setAttribute('onclick', 'globalThis.log += \"A\"'); \
             e.dispatchEvent(new Event('click')); \
             dp.body.append(e); \
             e.setAttribute('onclick', 'globalThis.log += \"B\"'); \
             e.dispatchEvent(new Event('click')); \
             r.push(log === 'A', e.onclick === null);",
        );
        assert_eq!(
            got, "true,true",
            "C8c first half: neither A nor B ran again"
        );
        let e_val = vm.eval("globalThis.e").unwrap();
        let e = entity_of(vm, e_val);
        assert_eq!(
            click_source(vm, e).as_deref(),
            Some("globalThis.log += \"B\""),
            "the source stays pending under arm 3"
        );
        vm.eval("document.body.append(e); e.dispatchEvent(new Event('click'));")
            .unwrap();
        assert_eq!(eval_str(vm, "globalThis.log"), "AB", "B compiled and ran");
    });
}

// ---- C8d: deactivation under the gate (arm 1 before the gate): as C8c to the
// move, then `removeAttribute('onclick')` → the removed callable does not run,
// the getter is null ----
#[test]
fn c8d_remove_attribute_in_domparser_doc_deactivates_handler() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.log = ''; \
             var e = document.createElement('div'); \
             document.body.append(e); \
             e.setAttribute('onclick', 'globalThis.log += \"A\"'); \
             e.dispatchEvent(new Event('click')); \
             dp.body.append(e); \
             e.removeAttribute('onclick'); \
             e.dispatchEvent(new Event('click')); \
             r.push(log === 'A', e.onclick === null);",
        );
        assert_eq!(got, "true,true", "C8d");
    });
}

// ---- C8e (VM half): one form on the dispatch walk and the IDL getter — a raw
// handler on a DOMParser node reads null on both, an IDL-set one runs ----
#[test]
fn c8e_vm_walk_and_getter_share_one_form() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.rawRan = 0; globalThis.idlRan = 0; \
             var raw = dp.createElement('div'); dp.body.append(raw); \
             raw.setAttribute('onclick', 'globalThis.rawRan++'); \
             var idl = dp.createElement('div'); dp.body.append(idl); \
             idl.onclick = function () { globalThis.idlRan++; }; \
             raw.dispatchEvent(new Event('click')); \
             idl.dispatchEvent(new Event('click')); \
             r.push(rawRan === 0, idlRan === 1, raw.onclick === null, typeof idl.onclick === 'function');",
        );
        assert_eq!(got, "true,true,true,true", "C8e (i), (iii)");
    });
}

// ---- C30: reverse case, production form — a bound-created element with a raw
// handler appended into a DOMParser document: the getter is null, the handler
// does not run ----
#[test]
fn c30_raw_handler_on_node_appended_into_domparser_doc_stays_suppressed() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.ran = 0; \
             var b = document.createElement('button'); \
             b.setAttribute('onclick', 'globalThis.ran++'); \
             dp.body.append(b); \
             b.dispatchEvent(new Event('click')); \
             r.push(ran === 0, b.onclick === null);",
        );
        assert_eq!(got, "true,true", "C30");
    });
}

// ---- C8g (window pin, NR8 (a)): an element with two deliverable ordinary
// handler attributes whose value is a callback before its first processing
// (the IDL write processes nothing, D-8 (a)), then first inserted into another
// DOMParser document (processed, T-a2: `A` raw over `fn`). N1: the getter reads
// null and dispatch runs neither `fn` nor `A`, the source reads `A`. HEAD: a
// function compiled from `A`, nothing runs. Spec: `fn`, which runs. Slice b's
// S-5 (slot `#11-eh-attrs-creation-time-call-sites`) inverts this to C8b's
// expectation ----
#[test]
fn c8g_window_pin_multi_attribute_callback_lost_at_first_insertion() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.a = 0; \
             var dd = new DOMParser().parseFromString('<div id=d onclick=\"globalThis.a++\" onmouseover=\"globalThis.m = 1\"></div>', 'text/html'); \
             globalThis.d = dd.getElementById('d'); \
             d.onclick = fn; \
             dp2.body.append(d); \
             d.dispatchEvent(new Event('click')); \
             r.push(fnRan === 0, a === 0, d.onclick === null);",
        );
        assert_eq!(got, "true,true,true", "C8g (window pin)");
        let d_val = vm.eval("globalThis.d").unwrap();
        let d = entity_of(vm, d_val);
        assert_eq!(
            click_source(vm, d).as_deref(),
            Some("globalThis.a++"),
            "the source oracle reads A"
        );
    });
}

// ---- C8h (spec pin, NR8 (b)): a Window-reflecting body name in a DOMParser
// document is element-targeted at HEAD (X-W); N1's getter reads null (arm 3:
// the node clause fires) where HEAD compiled the attribute, and dispatch runs
// nothing. Spec: null and nothing runs (the event handler IDL attribute's
// getter: *determine the target of an event handler* step 3 returns null for a
// node document that is not active) ----
#[test]
fn c8h_spec_pin_window_reflecting_body_name_in_domparser_doc() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.a = 0; \
             var dd = new DOMParser().parseFromString('<body></body>', 'text/html'); \
             dd.body.setAttribute('onfocus', 'globalThis.a++'); \
             r.push(dd.body.onfocus === null); \
             dd.body.dispatchEvent(new Event('focus')); \
             r.push(a === 0);",
        );
        assert_eq!(got, "true,true", "C8h (spec pin)");
    });
}
