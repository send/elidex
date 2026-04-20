//! Tests for `new Event(type, init)` / `new CustomEvent(type, init)`
//! (WebIDL §2.2, §2.3).
//!
//! Covers:
//! - EventInit dictionary parsing (bubbles / cancelable / composed)
//! - `new` gate (call-mode throws TypeError)
//! - Required first argument (absent → TypeError)
//! - Non-string type coercion (Symbol throws, others coerce)
//! - Default values (null target/currentTarget, eventPhase 0, isTrusted false)
//! - Prototype chain: `instance → Event.prototype → Object.prototype`
//! - `CustomEvent.prototype → Event.prototype`
//! - Brand check: `Event.prototype.preventDefault.call(plainObj)` returns
//!   `undefined` (silent no-op — elidex convention for detached method
//!   handles, see `natives_event.rs` module doc rationale; WebIDL
//!   TypeError path is deferred)
//! - `timeStamp` is non-zero (monotonic origin shared with
//!   `performance.now()`)
//! - Core-9 shape slot layout invariant (S2 lock-in test)

#![cfg(feature = "engine")]

use super::super::host::event_shapes::CORE_KEY_COUNT;
use super::super::value::{JsValue, PropertyKey};
use super::super::Vm;
use super::{eval_bool, eval_number, eval_string};

// ---------------------------------------------------------------------------
// Constructor basics
// ---------------------------------------------------------------------------

#[test]
fn new_event_type_is_first_arg() {
    assert_eq!(eval_string("new Event('click').type"), "click");
}

#[test]
fn new_event_defaults_bubbles_cancelable_composed_false() {
    assert!(!eval_bool("new Event('click').bubbles"));
    assert!(!eval_bool("new Event('click').cancelable"));
    assert!(!eval_bool("new Event('click').composed"));
}

#[test]
fn new_event_init_bubbles_read() {
    assert!(eval_bool("new Event('x', {bubbles: true}).bubbles"));
}

#[test]
fn new_event_init_cancelable_and_bubbles() {
    assert!(eval_bool(
        "new Event('x', {bubbles: true, cancelable: true}).cancelable"
    ));
    assert!(eval_bool(
        "new Event('x', {bubbles: true, cancelable: true}).bubbles"
    ));
}

#[test]
fn new_event_default_target_and_phase_are_null_zero() {
    // WHATWG §2.2: a freshly-constructed event has no target yet;
    // eventPhase is NONE (0).  Both are mutated when dispatchEvent
    // starts walking the propagation path.
    assert!(matches!(
        Vm::new().eval("new Event('x').target").unwrap(),
        JsValue::Null
    ));
    assert!(matches!(
        Vm::new().eval("new Event('x').currentTarget").unwrap(),
        JsValue::Null
    ));
    assert_eq!(eval_number("new Event('x').eventPhase"), 0.0);
}

#[test]
fn new_event_is_trusted_is_false() {
    // WHATWG §2.1: script-dispatched events are NOT trusted (UA-
    // generated events set isTrusted=true; script construction does
    // not).
    assert!(!eval_bool("new Event('x').isTrusted"));
}

#[test]
fn new_event_timestamp_is_positive_number() {
    // HR-Time §5: timeStamp is monotonic ms since the time origin
    // (shared with performance.now via VmInner::start_instant).  Two
    // events in quick succession should still both be non-negative
    // finite numbers; ordering is not tested here.
    assert!(eval_bool("new Event('x').timeStamp >= 0"));
    assert!(eval_bool("isFinite(new Event('x').timeStamp)"));
}

// ---------------------------------------------------------------------------
// `new` gate (WebIDL [Constructor])
// ---------------------------------------------------------------------------

#[test]
fn event_call_without_new_throws_type_error() {
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { Event('click'); } catch (e) { caught = e.name; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn event_no_args_throws_type_error() {
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { new Event(); } catch (e) { caught = e.name; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn event_symbol_type_throws_type_error() {
    // ToString on a Symbol throws (existing `coerce::to_string`
    // behaviour).  We assert the error name only; the message
    // contains "Symbol" (implementation detail) but tests tie to
    // `name === 'TypeError'` to stay robust across minor phrasing
    // changes.
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { new Event(Symbol.iterator); } catch (e) { caught = e.name; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn event_init_getter_throw_propagates() {
    // WHATWG dictionary coercion: getters on the init object may
    // fire and their exceptions propagate out of the constructor.
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { \
                new Event('x', { get bubbles() { throw new Error('boom'); } }); \
             } catch (e) { caught = e.message; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "boom"),
        other => panic!("expected string, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Prototype chain (WHATWG §2.2)
// ---------------------------------------------------------------------------

#[test]
fn event_prototype_chain_ends_at_object_prototype() {
    // Instance → Event.prototype direct hop.
    assert!(eval_bool(
        "Object.getPrototypeOf(new Event('x')) === Event.prototype",
    ));
    // Event.prototype → Object.prototype (second hop).  The `Object`
    // global in this VM is a methods bag without a `.prototype` own
    // property, so `Object.prototype` literally evaluates to
    // `undefined`; reaching the real Object.prototype requires
    // allocating a plain object and peeking its `__proto__` (which
    // goes through the same chain the instance uses).
    assert!(eval_bool(
        "Object.getPrototypeOf(Event.prototype) === Object.getPrototypeOf({})",
    ));
}

#[test]
fn event_global_has_prototype_property() {
    // Diagnostic: Event constructor exposes `.prototype` referencing
    // the same object an instance inherits from.  Pinpoints broken
    // ctor wiring vs broken prototype chain.
    assert!(eval_bool(
        "Event.prototype === Object.getPrototypeOf(new Event('x'))",
    ));
    assert!(eval_bool("typeof Event.prototype === 'object'"));
}

#[test]
fn event_instance_constructor_is_event() {
    assert!(eval_bool("new Event('x').constructor === Event"));
}

#[test]
fn event_prototype_preventdefault_brand_check() {
    // Calling `Event.prototype.preventDefault` on a non-Event
    // receiver is a WebIDL brand check failure.  The existing
    // native_event_prevent_default returns Undefined for non-Event
    // receivers (silent no-op pattern) — this test pins that
    // behaviour so a future brand-check tightening surfaces visibly.
    // (A stricter spec-compliant TypeError would be a behaviour
    // change worth calling out separately.)
    let mut vm = Vm::new();
    let result = vm.eval("Event.prototype.preventDefault.call({})").unwrap();
    assert!(matches!(result, JsValue::Undefined));
}

// ---------------------------------------------------------------------------
// CustomEvent (WHATWG §2.3)
// ---------------------------------------------------------------------------

#[test]
fn new_custom_event_detail_default_null() {
    // WHATWG §2.3 `any detail = null;` — missing init or missing
    // `detail` key both default to JS null.
    assert!(matches!(
        Vm::new().eval("new CustomEvent('x').detail").unwrap(),
        JsValue::Null
    ));
    assert!(matches!(
        Vm::new().eval("new CustomEvent('x', {}).detail").unwrap(),
        JsValue::Null
    ));
}

#[test]
fn new_custom_event_detail_passthrough() {
    // Object detail is preserved as an own-property (slot 9 of the
    // custom_event shape).  Deep property access confirms the slot
    // value is the original object, not a clone or toString'd form.
    assert_eq!(
        eval_number("new CustomEvent('x', {detail: {foo: 42}}).detail.foo"),
        42.0,
    );
    // Primitive detail (number) flows through untouched.
    assert_eq!(eval_number("new CustomEvent('x', {detail: 7}).detail"), 7.0,);
}

#[test]
fn custom_event_prototype_chains_to_event_prototype() {
    assert!(eval_bool(
        "Object.getPrototypeOf(CustomEvent.prototype) === Event.prototype",
    ));
    assert!(eval_bool(
        "new CustomEvent('x').constructor === CustomEvent",
    ));
}

#[test]
fn custom_event_inherits_event_members() {
    // CustomEvent instance reaches Event's own methods/accessors via
    // its prototype chain.  `preventDefault()` on a non-cancelable
    // event is a no-op; `defaultPrevented` then stays false — the
    // point of the test is method REACHABILITY, not semantics.
    assert_eq!(eval_string("new CustomEvent('x').type"), "x");
    assert!(eval_bool(
        "typeof new CustomEvent('x').preventDefault === 'function'",
    ));
    assert!(eval_bool(
        "typeof new CustomEvent('x').stopPropagation === 'function'",
    ));
}

#[test]
fn custom_event_without_new_throws() {
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { CustomEvent('x'); } catch (e) { caught = e.name; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected string, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Shape slot layout invariant (S2 lock-in)
// ---------------------------------------------------------------------------

#[test]
fn core_9_slot_order_is_locked() {
    // Structural invariant: every Event shape extends from `core`
    // with the 9 canonical keys at slots 0..9.  `dispatchEvent`
    // mutates slots 3 / 4 / 5 (eventPhase / target / currentTarget)
    // by index without re-reading the shape — if this ever changes,
    // dispatchEvent would silently write wrong keys.  The assertion
    // ties the index → key mapping to the well-known StringIds.
    let vm = Vm::new();
    let shapes = vm
        .inner
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed shapes must be built during register_globals");
    let core = shapes.core;
    let wk = &vm.inner.well_known;
    let entries: Vec<_> = vm.inner.shapes[core as usize]
        .ordered_entries
        .iter()
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(entries.len(), CORE_KEY_COUNT);
    assert_eq!(entries[0], PropertyKey::String(wk.event_type));
    assert_eq!(entries[1], PropertyKey::String(wk.bubbles));
    assert_eq!(entries[2], PropertyKey::String(wk.cancelable));
    assert_eq!(entries[3], PropertyKey::String(wk.event_phase));
    assert_eq!(entries[4], PropertyKey::String(wk.target));
    assert_eq!(entries[5], PropertyKey::String(wk.current_target));
    assert_eq!(entries[6], PropertyKey::String(wk.time_stamp));
    assert_eq!(entries[7], PropertyKey::String(wk.composed));
    assert_eq!(entries[8], PropertyKey::String(wk.is_trusted));
}
