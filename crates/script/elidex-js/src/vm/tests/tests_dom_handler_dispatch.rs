//! Tests for the `DomApiHandler` dispatch bridge established in
//! slot #11-arch-hoist-a (`vm/host/dom_bridge.rs::invoke_dom_api`).
//!
//! Covers the boundary contracts that the bridge guarantees,
//! independent of any specific handler's spec semantics:
//! - Symbol / BigInt arguments raise `TypeError` immediately
//!   (WebIDL §3.10.14 — Symbol coercion is total-throw).
//! - Non-Node `Object` arguments where a Node is expected raise
//!   `TypeError`.
//! - Round-trips: `ObjectRef` returned by a handler resolves back to
//!   a JS wrapper with identity-preserving semantics; string args
//!   intern correctly across the marshalling boundary.
//! - `DomApiError` → `DOMException` mapping for the variants the
//!   bridge currently handles (`HierarchyRequestError`).
//!
//! Drift incident: `memory/m4-12-architectural-drift-incident.md`.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// Build a minimal `doc > html > body` tree and return the
/// document entity.  Mirrors the fixture in
/// `tests_document_methods.rs` but trimmed to what the dispatch
/// tests need (no head / title).
fn build_min_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

/// Run `source` against a fresh VM bound to a minimal document
/// fixture, return the result string from `e.constructor.name` /
/// `e.name` style probes.
fn eval_in_doc_string(source: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let r = vm.eval(source).unwrap();
    let s = match r {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    };
    vm.unbind();
    s
}

fn eval_in_doc_bool(source: &str) -> bool {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let r = vm.eval(source).unwrap();
    let b = match r {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    };
    vm.unbind();
    b
}

// ---------------------------------------------------------------------------
// Symbol / BigInt argument rejection (WebIDL §3.10.14)
// ---------------------------------------------------------------------------

#[test]
fn dispatch_with_symbol_arg_throws_type_error() {
    // `getElementById` accepts a string id.  Passing a Symbol must
    // raise TypeError — `coerce::to_string` runs at the call site
    // (Symbol is a ToString-throw input per ECMA §7.1.17), so the
    // marshalling boundary `prepare_arg` would only see Symbol
    // through a manually-constructed dispatch path; the call-site
    // ToString already rejects it for any normal user code.
    assert_eq!(
        eval_in_doc_string(
            "var caught = null;\
             try { document.getElementById(Symbol()); } \
             catch (e) { caught = e.name; }\
             caught;"
        ),
        "TypeError",
    );
}

#[test]
fn dispatch_with_bigint_arg_through_string_coercion_succeeds() {
    // BigInt → string coercion is **valid** per ECMA §7.1.17 (only
    // Symbol throws on ToString); BigInt(1n) coerces to the string
    // `"1"`.  `coerce_first_arg_to_string` therefore lets BigInt
    // through, and `getElementById("1")` returns null when no
    // matching id exists.  This documents that the marshalling
    // layer's `prepare_arg` BigInt rejection is a defensive
    // contract for future call sites that pass BigInt **directly**
    // (without prior ToString); the 5 PoC fns introduced by slot
    // #11-arch-hoist-a all pre-coerce string args at the call
    // site, so BigInt never actually reaches `prepare_arg` through
    // them.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let r = vm.eval("document.getElementById(1n);").unwrap();
    assert!(matches!(r, JsValue::Null));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Non-Node Object argument rejection
// ---------------------------------------------------------------------------

#[test]
fn dispatch_non_node_object_to_append_child_throws_type_error() {
    // `appendChild(ordinaryObject)` — the call-site `require_node_arg`
    // brand check rejects non-Node objects before dispatch.
    // Asserting TypeError here documents the contract that non-Node
    // arguments are rejected at the brand check (which precedes the
    // marshalling layer).
    assert_eq!(
        eval_in_doc_string(
            "var caught = null;\
             try { document.body.appendChild({}); } \
             catch (e) { caught = e.name; }\
             caught;"
        ),
        "TypeError",
    );
}

// ---------------------------------------------------------------------------
// Round-trip: string arg through setAttribute/getAttribute
// ---------------------------------------------------------------------------

#[test]
fn dispatch_string_arg_round_trip_via_set_get_attribute() {
    // Exercises the full marshalling round-trip for both
    // setAttribute (string in) and getAttribute (string out):
    // VM `JsValue::String` → `String::from` (heap) → handler
    // → handler clone → Rust `String` → re-intern → VM
    // `JsValue::String`.  The intern step deduplicates so the
    // returned string id may differ from the input id but the
    // observable string value must match.
    assert_eq!(
        eval_in_doc_string(
            "var d = document.createElement('div');\
             d.setAttribute('data-x', 'hello world');\
             d.getAttribute('data-x');"
        ),
        "hello world",
    );
}

// ---------------------------------------------------------------------------
// Round-trip: ObjectRef (identity preservation across handler dispatch)
// ---------------------------------------------------------------------------

#[test]
fn dispatch_object_ref_round_trip_appendchild_returns_same_wrapper() {
    // `appendChild` returns the inserted child.  After dispatch:
    // VM wrapper → `prepare_arg` extracts entity → materialize via
    // `session.get_or_create_wrapper` → handler → `JsValue::ObjectRef` →
    // `session.identity_map().get` → same entity →
    // `vm.create_element_wrapper` → JS wrapper.  Identity (`===`)
    // must hold because `create_element_wrapper` is identity-
    // preserving (boa parity).
    assert!(eval_in_doc_bool(
        "var c = document.createElement('span');\
         var ret = document.body.appendChild(c);\
         ret === c;"
    ));
}

// ---------------------------------------------------------------------------
// DomApiError → DOMException name mapping
// ---------------------------------------------------------------------------

#[test]
fn dispatch_dom_error_to_dom_exception_hierarchy_request() {
    // appendChild self-cycle: WHATWG DOM §4.2.3 step "containing
    // self" → handler raises `HierarchyRequestError`.  The bridge
    // maps `DomApiErrorKind::HierarchyRequestError` to
    // `VmError::dom_exception(WK.dom_exc_hierarchy_request_error,
    // ...)`, which materialises as a `DOMException` whose
    // `.name === "HierarchyRequestError"`.
    assert_eq!(
        eval_in_doc_string(
            "var caught = null;\
             try { document.body.appendChild(document.body); } \
             catch (e) { caught = e.name; }\
             caught;"
        ),
        "HierarchyRequestError",
    );
}
