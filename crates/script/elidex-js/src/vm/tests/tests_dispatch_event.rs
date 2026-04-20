//! `EventTarget.prototype.dispatchEvent(event)` integration tests.
//! Covers WHATWG DOM §2.9 algorithm behaviour:
//!
//! - Listener fires on target.
//! - Capture / at-target / bubble phase ordering.
//! - `preventDefault()` + `defaultPrevented` + return value.
//! - `stopPropagation` / `stopImmediatePropagation`.
//! - `once` listener auto-removal (before invocation per §2.10 step 15).
//! - Re-entrant dispatch throws `InvalidStateError` `DOMException`.
//! - `addEventListener` from inside a dispatch does NOT add to the
//!   current plan (snapshot semantics).
//! - `dispatchEvent(plainObj)` throws TypeError.
//! - `composedPath()` returns the path during dispatch, `[]` afterward.
//! - CustomEvent.detail flows through.
//! - Return value is `!default_prevented`.
//! - Receiver brand: unbound / non-HostObject silently returns `true`.
//!
//! All tests use a small HTML-ish tree built via `EcsDom::append_child`
//! and a single element wrapper exposed to JS as `el` (plus siblings
//! / ancestors where required).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

/// Build a `<html><body><p id="t"/></body></html>` tree, bind the VM,
/// and expose `p` as `el`, `body` as `body`, `html` as `html`.
fn build_simple_tree(vm: &mut Vm, session: &mut SessionCore, dom: &mut EcsDom) {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let p = dom.create_element("p", {
        let mut a = Attributes::default();
        a.set("id", "t");
        a
    });
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(body, p));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let p_wrapper = vm.inner.create_element_wrapper(p);
    let body_wrapper = vm.inner.create_element_wrapper(body);
    let html_wrapper = vm.inner.create_element_wrapper(html);
    vm.set_global("el", JsValue::Object(p_wrapper));
    vm.set_global("body", JsValue::Object(body_wrapper));
    vm.set_global("html", JsValue::Object(html_wrapper));
}

fn eval_tree_string(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    let out = match vm.eval(script).unwrap() {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        other => panic!("expected string, got {other:?}"),
    };
    vm.unbind();
    out
}

fn eval_tree_bool(script: &str) -> bool {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    let out = match vm.eval(script).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    };
    vm.unbind();
    out
}

fn eval_tree_number(script: &str) -> f64 {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    let out = match vm.eval(script).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    };
    vm.unbind();
    out
}

// ---------------------------------------------------------------------------
// Basic listener invocation
// ---------------------------------------------------------------------------

#[test]
fn dispatch_fires_target_listener_with_event_object() {
    let out = eval_tree_string(
        "var fired = 'no';\
         el.addEventListener('custom', function (e) { fired = e.type; });\
         el.dispatchEvent(new Event('custom'));\
         fired;",
    );
    assert_eq!(out, "custom");
}

#[test]
fn dispatch_returns_true_when_no_listener() {
    assert!(eval_tree_bool(
        "el.dispatchEvent(new Event('never-listened'));"
    ));
}

#[test]
fn dispatch_return_is_not_default_prevented() {
    // cancelable + preventDefault → return false.
    assert!(!eval_tree_bool(
        "el.addEventListener('x', function (e) { e.preventDefault(); });\
         el.dispatchEvent(new Event('x', {cancelable: true}));"
    ));
    // non-cancelable → preventDefault is no-op, return true.
    assert!(eval_tree_bool(
        "el.addEventListener('y', function (e) { e.preventDefault(); });\
         el.dispatchEvent(new Event('y'));"
    ));
}

#[test]
fn default_prevented_reflects_prevent_default_during_and_after() {
    // WHATWG §2.9: `defaultPrevented` reflects the canceled flag;
    // once dispatch ends the flag is NOT reset (return value reads
    // it).  Matches browser semantics.
    assert!(eval_tree_bool(
        "var evt = new Event('p', {cancelable: true});\
         el.addEventListener('p', function (e) { e.preventDefault(); });\
         el.dispatchEvent(evt);\
         evt.defaultPrevented;"
    ));
}

// ---------------------------------------------------------------------------
// Phase ordering
// ---------------------------------------------------------------------------

#[test]
fn bubbling_listener_fires_for_bubbling_event() {
    let out = eval_tree_string(
        "var order = '';\
         el.addEventListener('c', function (e) { order += 'target;'; });\
         body.addEventListener('c', function (e) { order += 'bubble;'; });\
         el.dispatchEvent(new Event('c', {bubbles: true}));\
         order;",
    );
    assert_eq!(out, "target;bubble;");
}

#[test]
fn bubbling_listener_not_fired_for_nonbubbling_event() {
    let out = eval_tree_string(
        "var order = '';\
         el.addEventListener('c', function (e) { order += 'target;'; });\
         body.addEventListener('c', function (e) { order += 'bubble;'; });\
         el.dispatchEvent(new Event('c'));\
         order;",
    );
    assert_eq!(out, "target;");
}

#[test]
fn capture_listener_fires_before_target() {
    let out = eval_tree_string(
        "var order = '';\
         html.addEventListener('c', function () { order += 'cap;'; }, true);\
         el.addEventListener('c', function () { order += 'tgt;'; });\
         el.dispatchEvent(new Event('c', {bubbles: true}));\
         order;",
    );
    // Capture on html, then target on el.  (No bubbling listener on
    // html so bubble phase is silent.)
    assert_eq!(out, "cap;tgt;");
}

#[test]
fn event_phase_visible_during_listener_invocation() {
    // eventPhase reflects the WHATWG §2.2 enum during dispatch.
    let out = eval_tree_string(
        "var log = '';\
         html.addEventListener('c', function (e) { log += 'cap:' + e.eventPhase + ';'; }, true);\
         el.addEventListener('c', function (e) { log += 'tgt:' + e.eventPhase + ';'; });\
         html.addEventListener('c', function (e) { log += 'bub:' + e.eventPhase + ';'; });\
         el.dispatchEvent(new Event('c', {bubbles: true}));\
         log;",
    );
    // Capturing=1, AtTarget=2, Bubbling=3.
    assert_eq!(out, "cap:1;tgt:2;bub:3;");
}

// ---------------------------------------------------------------------------
// Stop propagation
// ---------------------------------------------------------------------------

#[test]
fn stop_propagation_blocks_later_phases() {
    let out = eval_tree_string(
        "var order = '';\
         el.addEventListener('c', function (e) { order += 'tgt;'; e.stopPropagation(); });\
         body.addEventListener('c', function () { order += 'bub;'; });\
         el.dispatchEvent(new Event('c', {bubbles: true}));\
         order;",
    );
    // bubble listener on body should NOT fire.
    assert_eq!(out, "tgt;");
}

#[test]
fn stop_immediate_propagation_blocks_same_entity_listeners() {
    let out = eval_tree_string(
        "var order = '';\
         el.addEventListener('c', function (e) { order += 'a;'; e.stopImmediatePropagation(); });\
         el.addEventListener('c', function () { order += 'b;'; });\
         body.addEventListener('c', function () { order += 'bub;'; });\
         el.dispatchEvent(new Event('c', {bubbles: true}));\
         order;",
    );
    assert_eq!(out, "a;");
}

// ---------------------------------------------------------------------------
// Once / snapshot semantics
// ---------------------------------------------------------------------------

#[test]
fn once_listener_fires_once_then_autoremoves() {
    let out = eval_tree_number(
        "var n = 0;\
         el.addEventListener('c', function () { n++; }, {once: true});\
         el.dispatchEvent(new Event('c'));\
         el.dispatchEvent(new Event('c'));\
         el.dispatchEvent(new Event('c'));\
         n;",
    );
    assert_eq!(out, 1.0);
}

#[test]
fn add_event_listener_during_dispatch_does_not_fire_this_round() {
    // WHATWG §2.10 step 3: the listener list is frozen at the start
    // of each phase — listeners added during dispatch execute only
    // on subsequent dispatches.
    let out = eval_tree_number(
        "var n = 0;\
         el.addEventListener('c', function () {\
             el.addEventListener('c', function () { n++; });\
         });\
         el.dispatchEvent(new Event('c'));\
         n;",
    );
    assert_eq!(out, 0.0);
}

// ---------------------------------------------------------------------------
// Re-entrant dispatch + wrong-type arg
// ---------------------------------------------------------------------------

#[test]
fn reentrant_dispatch_throws_invalid_state_error() {
    let out = eval_tree_string(
        "var err = 'no-error';\
         el.addEventListener('c', function (e) {\
             try { el.dispatchEvent(e); err = 'no-throw'; }\
             catch (x) { err = x.name; }\
         });\
         el.dispatchEvent(new Event('c'));\
         err;",
    );
    assert_eq!(out, "InvalidStateError");
}

#[test]
fn sequential_dispatch_of_same_event_succeeds() {
    // `dispatched_events` is in-flight only — a completed dispatch
    // clears membership.  Matches Firefox / Chrome behaviour.
    let out = eval_tree_number(
        "var evt = new Event('c');\
         var n = 0;\
         el.addEventListener('c', function () { n++; });\
         el.dispatchEvent(evt);\
         el.dispatchEvent(evt);\
         n;",
    );
    assert_eq!(out, 2.0);
}

#[test]
fn dispatch_non_event_arg_throws_type_error() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    // Plain object, not an Event — WebIDL `Event event` rejection.
    let result = vm.eval("el.dispatchEvent({type: 'c'});");
    assert!(result.is_err(), "expected TypeError");
    vm.unbind();
}

#[test]
fn dispatch_missing_arg_throws_type_error() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    let result = vm.eval("el.dispatchEvent();");
    assert!(result.is_err(), "expected TypeError for missing arg");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// composedPath
// ---------------------------------------------------------------------------

#[test]
fn composed_path_returns_path_during_dispatch() {
    // During dispatch on a bubbling event, composedPath() returns
    // the propagation path (root → target).
    let out = eval_tree_number(
        "var len = -1;\
         el.addEventListener('c', function (e) { len = e.composedPath().length; });\
         el.dispatchEvent(new Event('c', {bubbles: true, composed: true}));\
         len;",
    );
    // path = [doc, html, body, p] → length 4.
    assert_eq!(out, 4.0);
}

#[test]
fn composed_path_is_empty_after_dispatch_completes() {
    let out = eval_tree_number(
        "var evt = new Event('c');\
         el.addEventListener('c', function () {});\
         el.dispatchEvent(evt);\
         evt.composedPath().length;",
    );
    assert_eq!(out, 0.0);
}

// ---------------------------------------------------------------------------
// CustomEvent detail
// ---------------------------------------------------------------------------

#[test]
fn custom_event_detail_survives_dispatch() {
    let out = eval_tree_number(
        "var got = -1;\
         el.addEventListener('c', function (e) { got = e.detail.foo; });\
         el.dispatchEvent(new CustomEvent('c', {detail: {foo: 42}}));\
         got;",
    );
    assert_eq!(out, 42.0);
}

// ---------------------------------------------------------------------------
// Target / currentTarget mutation + reset
// ---------------------------------------------------------------------------

#[test]
fn target_and_current_target_set_during_dispatch() {
    let out = eval_tree_string(
        "var log = '';\
         el.addEventListener('c', function (e) {\
             log = (e.target === el) + ',' + (e.currentTarget === el);\
         });\
         el.dispatchEvent(new Event('c'));\
         log;",
    );
    assert_eq!(out, "true,true");
}

#[test]
fn current_target_is_null_after_dispatch() {
    // WHATWG §2.9 step 31 — currentTarget restored to null at end.
    let out = eval_tree_bool(
        "var evt = new Event('c');\
         el.addEventListener('c', function () {});\
         el.dispatchEvent(evt);\
         evt.currentTarget === null;",
    );
    assert!(out);
}

#[test]
fn event_phase_is_zero_after_dispatch() {
    let out = eval_tree_number(
        "var evt = new Event('c');\
         el.addEventListener('c', function () {});\
         el.dispatchEvent(evt);\
         evt.eventPhase;",
    );
    assert_eq!(out, 0.0);
}

// ---------------------------------------------------------------------------
// Listener registration order
// ---------------------------------------------------------------------------

#[test]
fn multiple_listeners_fire_in_registration_order() {
    let out = eval_tree_string(
        "var order = '';\
         el.addEventListener('c', function () { order += 'a;'; });\
         el.addEventListener('c', function () { order += 'b;'; });\
         el.addEventListener('c', function () { order += 'c;'; });\
         el.dispatchEvent(new Event('c'));\
         order;",
    );
    assert_eq!(out, "a;b;c;");
}

// ---------------------------------------------------------------------------
// VM-state hygiene
// ---------------------------------------------------------------------------

#[test]
fn dispatched_events_membership_is_cleared_after_dispatch() {
    // Internal invariant: the `dispatched_events` set must be empty
    // once a successful dispatch completes (else sequential dispatch
    // would throw — covered functionally above, but this asserts
    // the lower-level state directly for regression-defence against
    // accidental leaks in the cleanup path).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);

    vm.eval(
        "el.addEventListener('c', function () {});\
         el.dispatchEvent(new Event('c'));",
    )
    .unwrap();

    assert!(
        vm.inner.dispatched_events.is_empty(),
        "dispatched_events leaked after successful dispatch: {:?}",
        vm.inner.dispatched_events
    );
    vm.unbind();
}

#[test]
fn dispatched_events_cleared_after_invalid_state_throw() {
    // The outer dispatchEvent completes normally — the InvalidStateError
    // is raised by the re-entrant inner call which is caught by the
    // user's try/catch.  Both the outer and inner tracking must be
    // cleared on completion.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);

    vm.eval(
        "el.addEventListener('c', function (e) {\
             try { el.dispatchEvent(e); } catch (_) {}\
         });\
         el.dispatchEvent(new Event('c'));",
    )
    .unwrap();

    assert!(
        vm.inner.dispatched_events.is_empty(),
        "leaked ids after re-entrant throw: {:?}",
        vm.inner.dispatched_events
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Non-EventTarget receiver
// ---------------------------------------------------------------------------

#[test]
fn dispatch_on_non_host_object_silently_returns_true() {
    // WebIDL brand-check "silent no-op" convention: calling
    // EventTarget.prototype.dispatchEvent with a non-HostObject
    // `this` returns `true` (spec default for "event not
    // dispatched", matches addEventListener silent-no-op policy).
    let out = eval_tree_bool(
        "var f = el.dispatchEvent;\
         f.call({}, new Event('c'));",
    );
    assert!(out);
}

// ---------------------------------------------------------------------------
// GC sweep pruning (S3 pattern) — dispatched_events must not hold
// stale ids after GC reclaims a dispatched event object.
// ---------------------------------------------------------------------------

#[test]
fn dispatched_events_pruned_after_gc() {
    // Run a dispatch, force a GC, assert the set remains empty.
    // Functions as regression coverage for the `bit_get` prune
    // added in `gc.rs` sweep tail.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    vm.eval(
        "el.addEventListener('c', function () {});\
         el.dispatchEvent(new Event('c'));",
    )
    .unwrap();
    // Directly invoke GC to exercise the sweep pass on the now-
    // empty set.  With entries present this would trip the retain
    // guard; with none present it's a cheap noop — both are
    // desired post-conditions.
    vm.inner.collect_garbage();
    assert!(vm.inner.dispatched_events.is_empty());
    vm.unbind();
}

// ---------------------------------------------------------------------------
// `Event` prototype integrity after dispatch (regression defence
// against future refactors that might accidentally swap
// ObjectKind::Event → Ordinary during the walk)
// ---------------------------------------------------------------------------

#[test]
fn event_object_remains_event_kind_after_dispatch() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    vm.eval("el.addEventListener('c', function () {});")
        .unwrap();
    vm.eval("globalThis.__e = new Event('c');").unwrap();
    vm.eval("el.dispatchEvent(__e);").unwrap();
    let JsValue::Object(id) = vm.get_global("__e").unwrap() else {
        panic!("__e missing");
    };
    assert!(
        matches!(vm.inner.get_object(id).kind, ObjectKind::Event { .. }),
        "Event kind must survive dispatch"
    );
    vm.unbind();
}

#[test]
fn dispatch_after_user_delete_core_slot_does_not_panic() {
    // WebIDL core Event attributes install as `WEBIDL_RO`
    // (configurable=true), so `delete evt.target` is a legal JS
    // operation that flips the object from Shaped → Dictionary
    // storage.  Before this regression defence, the in-place slot
    // write in `set_event_slot_raw` `unreachable!`-panicked on
    // Dictionary storage, crashing the VM whenever a user
    // deleted any of `target` / `currentTarget` / `eventPhase`
    // and then dispatched the same event.  The Dictionary
    // fallback path preserves the semantic contract (dispatch
    // re-installs the attribute, user's delete is transparently
    // overwritten — matching Chrome's accessor-on-prototype
    // behaviour).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    let out = vm
        .eval(
            "var evt = new Event('c'); \
             delete evt.target; \
             delete evt.currentTarget; \
             delete evt.eventPhase; \
             var seen = 'no'; \
             el.addEventListener('c', function (e) { seen = (e.target === el); }); \
             el.dispatchEvent(evt); \
             seen;",
        )
        .unwrap();
    assert!(
        matches!(out, JsValue::Boolean(true)),
        "post-delete dispatch must restore target so listener sees the wrapper"
    );
    vm.unbind();
}

#[test]
fn dispatch_after_user_delete_preserves_prevent_default_return() {
    // Companion to the panic-avoidance regression: check that
    // return value + preventDefault still work after the user
    // deletes slot properties.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    build_simple_tree(&mut vm, &mut session, &mut dom);
    let out = vm
        .eval(
            "var evt = new Event('c', {cancelable: true}); \
             delete evt.target; \
             el.addEventListener('c', function (e) { e.preventDefault(); }); \
             el.dispatchEvent(evt);",
        )
        .unwrap();
    assert!(
        matches!(out, JsValue::Boolean(false)),
        "return value must reflect preventDefault even after delete"
    );
    vm.unbind();
}
