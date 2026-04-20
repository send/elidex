//! Tests for `new PromiseRejectionEvent` / `ErrorEvent` /
//! `HashChangeEvent` / `PopStateEvent` (HTML §8).
//!
//! Covers:
//! - `[Constructor]` gate (call-mode throws TypeError)
//! - Required first argument (absent → TypeError)
//! - PromiseRejectionEvent requires `{promise}` init (both dict and key)
//! - ErrorEvent / HashChangeEvent / PopStateEvent all accept missing
//!   init dicts and use WebIDL defaults
//! - Prototype chain: descendant → Event.prototype (NOT UIEvent)
//! - Own-data instance members resolve via shape slots
//! - Getter throw propagates from init-dict coercion

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;
use super::{eval_bool, eval_number, eval_string};

fn expect_type_error(vm: &mut Vm, source: &str) {
    let result = vm
        .eval(&format!(
            "var caught = null; \
             try {{ {source}; }} catch (e) {{ caught = e.name; }} caught;"
        ))
        .unwrap();
    match result {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "TypeError"),
        other => panic!("expected TypeError, got {other:?} (source: {source})"),
    }
}

// ---------------------------------------------------------------------------
// Constructor gate (4 ctors)
// ---------------------------------------------------------------------------

#[test]
fn event_extras_ctors_reject_call_mode() {
    let mut vm = Vm::new();
    for source in [
        "PromiseRejectionEvent('r', {promise: {}})",
        "ErrorEvent('error')",
        "HashChangeEvent('hashchange')",
        "PopStateEvent('popstate')",
    ] {
        expect_type_error(&mut vm, source);
    }
}

#[test]
fn event_extras_ctors_reject_missing_type() {
    let mut vm = Vm::new();
    for source in [
        "new PromiseRejectionEvent()",
        "new ErrorEvent()",
        "new HashChangeEvent()",
        "new PopStateEvent()",
    ] {
        expect_type_error(&mut vm, source);
    }
}

// ---------------------------------------------------------------------------
// PromiseRejectionEvent
// ---------------------------------------------------------------------------

#[test]
fn promise_rejection_event_requires_init_dict_and_promise_key() {
    // Missing init dict → TypeError (spec requires the dict because
    // `promise` is a required member).
    let mut vm = Vm::new();
    expect_type_error(&mut vm, "new PromiseRejectionEvent('r')");
    // Dict present but `promise` missing → TypeError.
    expect_type_error(&mut vm, "new PromiseRejectionEvent('r', {})");
    // Explicit `undefined` also rejected (same code path as missing).
    expect_type_error(
        &mut vm,
        "new PromiseRejectionEvent('r', {promise: undefined})",
    );
}

#[test]
fn promise_rejection_event_null_second_arg_fails_on_required_promise() {
    // WebIDL §3.10.23 dictionary coercion: `null` / `undefined`
    // are treated as an empty dictionary; the `required promise`
    // check then surfaces the error.  Chrome reports the same
    // text ("required member promise is undefined") for
    // `new PromiseRejectionEvent('r', null)` — a "not of type"
    // error for null would deviate from the spec.
    let mut vm = Vm::new();
    for arg in ["null", "undefined"] {
        let err = vm
            .eval(&format!(
                "try {{ new PromiseRejectionEvent('r', {arg}); 'no-throw' }} \
                 catch (e) {{ String(e.message) }}"
            ))
            .unwrap();
        let JsValue::String(sid) = err else {
            panic!("expected string error message for arg {arg}");
        };
        let msg = vm.get_string(sid);
        assert!(
            msg.contains("required member promise is undefined"),
            "{arg} → required-member error expected, got: {msg}"
        );
    }
}

#[test]
fn promise_rejection_event_primitive_second_arg_is_dict_coercion_error() {
    // Non-object, non-nullish primitives (number / string / bool)
    // fail WebIDL `PromiseRejectionEventInit` dictionary coercion
    // with "parameter 2 is not of type 'PromiseRejectionEventInit'".
    // Null/undefined are handled separately (empty-dict coercion).
    let mut vm = Vm::new();
    for arg in ["42", "'x'", "true"] {
        let err = vm
            .eval(&format!(
                "try {{ new PromiseRejectionEvent('r', {arg}); 'no-throw' }} \
                 catch (e) {{ String(e.message) }}"
            ))
            .unwrap();
        let JsValue::String(sid) = err else {
            panic!("expected string error message for arg {arg}");
        };
        let msg = vm.get_string(sid);
        assert!(
            msg.contains("not of type 'PromiseRejectionEventInit'"),
            "{arg} → dict coercion error expected, got: {msg}"
        );
    }
}

#[test]
fn promise_rejection_event_missing_second_arg_is_arity_error() {
    // Truly-missing second arg reports the arity text and stays
    // distinct from the null / non-object error paths above.
    let mut vm = Vm::new();
    let err = vm
        .eval(
            "try { new PromiseRejectionEvent('r'); 'no-throw' } \
             catch (e) { String(e.message) }",
        )
        .unwrap();
    let JsValue::String(sid) = err else {
        panic!("expected string error message");
    };
    let msg = vm.get_string(sid);
    assert!(
        msg.contains("2 arguments required"),
        "missing 2nd arg → arity error expected, got: {msg}"
    );
}

#[test]
fn promise_rejection_event_exposes_promise_and_reason() {
    assert!(eval_bool(
        "(function(){ var p = {}; var r = 'boom'; \
         var e = new PromiseRejectionEvent('unhandledrejection', {promise: p, reason: r}); \
         return e.promise === p && e.reason === r; \
         })()"
    ));
}

#[test]
fn promise_rejection_event_reason_default_undefined() {
    // WebIDL `any reason` with no default — missing key leaves the
    // slot as undefined (matching Chrome's common-case read).
    assert!(matches!(
        Vm::new()
            .eval("new PromiseRejectionEvent('r', {promise: {}}).reason")
            .unwrap(),
        JsValue::Undefined
    ));
}

#[test]
fn promise_rejection_event_prototype_chain_to_event_not_uievent() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new PromiseRejectionEvent('r', {promise: {}})) === \
         PromiseRejectionEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(PromiseRejectionEvent.prototype) === Event.prototype"
    ));
}

// ---------------------------------------------------------------------------
// ErrorEvent
// ---------------------------------------------------------------------------

#[test]
fn error_event_defaults() {
    // All init-dict members optional.  Defaults: message="", filename="",
    // lineno=0, colno=0, error=null.
    assert_eq!(eval_string("new ErrorEvent('error').message"), "");
    assert_eq!(eval_string("new ErrorEvent('error').filename"), "");
    assert_eq!(eval_number("new ErrorEvent('error').lineno"), 0.0);
    assert_eq!(eval_number("new ErrorEvent('error').colno"), 0.0);
    assert!(matches!(
        Vm::new().eval("new ErrorEvent('error').error").unwrap(),
        JsValue::Null
    ));
}

#[test]
fn error_event_init_pass_through() {
    let init = "{message: 'boom', filename: 'a.js', lineno: 5, colno: 12, error: 42}";
    assert_eq!(
        eval_string(&format!("new ErrorEvent('e', {init}).message")),
        "boom"
    );
    assert_eq!(
        eval_string(&format!("new ErrorEvent('e', {init}).filename")),
        "a.js"
    );
    assert_eq!(
        eval_number(&format!("new ErrorEvent('e', {init}).lineno")),
        5.0
    );
    assert_eq!(
        eval_number(&format!("new ErrorEvent('e', {init}).colno")),
        12.0
    );
    assert_eq!(
        eval_number(&format!("new ErrorEvent('e', {init}).error")),
        42.0
    );
}

#[test]
fn error_event_lineno_coerces_via_to_uint32() {
    // WebIDL `unsigned long` — ToUint32 modulo semantics.
    assert_eq!(
        eval_number("new ErrorEvent('e', {lineno: 2.7}).lineno"),
        2.0
    );
    assert_eq!(
        eval_number("new ErrorEvent('e', {lineno: -1}).lineno"),
        4_294_967_295.0
    );
}

#[test]
fn error_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new ErrorEvent('e')) === ErrorEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(ErrorEvent.prototype) === Event.prototype"
    ));
    assert!(eval_bool("new ErrorEvent('e').constructor === ErrorEvent"));
}

// ---------------------------------------------------------------------------
// HashChangeEvent
// ---------------------------------------------------------------------------

#[test]
fn hash_change_event_defaults() {
    assert_eq!(eval_string("new HashChangeEvent('hc').oldURL"), "");
    assert_eq!(eval_string("new HashChangeEvent('hc').newURL"), "");
}

#[test]
fn hash_change_event_init_pass_through() {
    let init = "{oldURL: '#a', newURL: '#b'}";
    assert_eq!(
        eval_string(&format!("new HashChangeEvent('hc', {init}).oldURL")),
        "#a"
    );
    assert_eq!(
        eval_string(&format!("new HashChangeEvent('hc', {init}).newURL")),
        "#b"
    );
}

#[test]
fn hash_change_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new HashChangeEvent('hc')) === HashChangeEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(HashChangeEvent.prototype) === Event.prototype"
    ));
}

// ---------------------------------------------------------------------------
// PopStateEvent
// ---------------------------------------------------------------------------

#[test]
fn pop_state_event_defaults_state_null() {
    assert!(matches!(
        Vm::new().eval("new PopStateEvent('pop').state").unwrap(),
        JsValue::Null
    ));
}

#[test]
fn pop_state_event_state_any_pass_through() {
    assert_eq!(
        eval_number("new PopStateEvent('pop', {state: 42}).state"),
        42.0
    );
    // Object identity preserved.
    assert!(eval_bool(
        "(function(){ var s = {foo: 1}; \
         return new PopStateEvent('pop', {state: s}).state === s; \
         })()"
    ));
}

#[test]
fn pop_state_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new PopStateEvent('pop')) === PopStateEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(PopStateEvent.prototype) === Event.prototype"
    ));
}

// ---------------------------------------------------------------------------
// Cross-cutting
// ---------------------------------------------------------------------------

#[test]
fn error_event_init_getter_throw_propagates() {
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { \
                new ErrorEvent('e', { get message() { throw new Error('boom'); } }); \
             } catch (e) { caught = e.message; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "boom"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn event_extras_inherit_event_members() {
    // Every C4 subclass inherits `type` / `isTrusted` / `timeStamp` /
    // `preventDefault` via Event.prototype.
    for ctor in [
        "new PromiseRejectionEvent('r', {promise: {}})",
        "new ErrorEvent('e')",
        "new HashChangeEvent('hc')",
        "new PopStateEvent('pop')",
    ] {
        let bang_new = format!("({ctor}).type");
        assert!(!eval_bool(&format!("({ctor}).isTrusted")));
        assert!(eval_bool(&format!(
            "typeof ({ctor}).preventDefault === 'function'"
        )));
        // `type` must be a non-empty string (matches first arg).
        assert!(eval_bool(&format!(
            "typeof {bang_new} === 'string' && {bang_new}.length > 0"
        )));
    }
}
