//! S5-4a / IBP-sandbox N1: HTML §8.1.8.1 scripting-disabled gates for
//! event handlers, driven through real JS.
//!
//! Split out of `tests_event_handler_attrs.rs` (touch-time split; the
//! IDL / content-attribute handler basics stay there).
//!
//! Compiled only under `engine`.

#![cfg(feature = "engine")]

use super::super::test_helpers::{entity_of, eval_str};
use super::tests_event_handler_attrs::{eval_bool, with_el};

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
            .set_sandbox_flags(elidex_plugin::SandboxingFlagSet::all_represented());
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
// insertion path does not run DOM §4.2.3 insert adoption (append_child
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
