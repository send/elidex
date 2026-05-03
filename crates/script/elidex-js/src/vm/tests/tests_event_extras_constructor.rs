//! Tests for `new PromiseRejectionEvent` / `ErrorEvent` /
//! `HashChangeEvent` / `PopStateEvent` / `AnimationEvent` /
//! `TransitionEvent` / `CloseEvent` (HTML §8 + CSS Animations §4.2 +
//! CSS Transitions §6 + WHATWG HTML §10.4).
//!
//! Covers:
//! - `[Constructor]` gate (call-mode throws TypeError)
//! - Required first argument (absent → TypeError)
//! - PromiseRejectionEvent requires `{promise}` init (both dict and key)
//! - ErrorEvent / HashChangeEvent / PopStateEvent / AnimationEvent /
//!   TransitionEvent / CloseEvent all accept missing init dicts and
//!   use WebIDL defaults
//! - Prototype chain: descendant → Event.prototype (NOT UIEvent)
//! - Own-data instance members resolve via shape slots
//! - Getter throw propagates from init-dict coercion
//! - WebIDL `unsigned short` modulo-2^16 (CloseEvent.code)
//! - WebIDL `boolean` ToBoolean coercion (CloseEvent.wasClean)
//! - GC survives across `collect_garbage()` for slot #11a ctors

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;
use super::{eval_bool, eval_number, eval_string};

// Local helpers for tests that need VM continuity across `eval` /
// `collect_garbage` (the module-level `eval_*` helpers each spin a
// fresh `Vm`).
fn vm_eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn vm_eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn vm_eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

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
        "AnimationEvent('animationstart')",
        "TransitionEvent('transitionstart')",
        "CloseEvent('close')",
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
        "new AnimationEvent()",
        "new TransitionEvent()",
        "new CloseEvent()",
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
        "new AnimationEvent('animationstart')",
        "new TransitionEvent('transitionstart')",
        "new CloseEvent('close')",
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

// ---------------------------------------------------------------------------
// AnimationEvent (CSS Animations Level 1 §4.2)
// ---------------------------------------------------------------------------

#[test]
fn animation_event_defaults() {
    // All init members optional.  Defaults: animationName="",
    // elapsedTime=0, pseudoElement="".
    assert_eq!(
        eval_string("new AnimationEvent('animationstart').animationName"),
        ""
    );
    assert_eq!(
        eval_number("new AnimationEvent('animationstart').elapsedTime"),
        0.0
    );
    assert_eq!(
        eval_string("new AnimationEvent('animationstart').pseudoElement"),
        ""
    );
}

#[test]
fn animation_event_init_pass_through() {
    let init = "{animationName: 'spin', elapsedTime: 1.25, pseudoElement: '::before'}";
    assert_eq!(
        eval_string(&format!(
            "new AnimationEvent('animationstart', {init}).animationName"
        )),
        "spin"
    );
    assert_eq!(
        eval_number(&format!(
            "new AnimationEvent('animationstart', {init}).elapsedTime"
        )),
        1.25
    );
    assert_eq!(
        eval_string(&format!(
            "new AnimationEvent('animationstart', {init}).pseudoElement"
        )),
        "::before"
    );
}

#[test]
fn animation_event_partial_init_uses_defaults_for_missing() {
    let init = "{animationName: 'fade'}";
    assert_eq!(
        eval_string(&format!("new AnimationEvent('a', {init}).animationName")),
        "fade"
    );
    assert_eq!(
        eval_number(&format!("new AnimationEvent('a', {init}).elapsedTime")),
        0.0
    );
    assert_eq!(
        eval_string(&format!("new AnimationEvent('a', {init}).pseudoElement")),
        ""
    );
}

#[test]
fn animation_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new AnimationEvent('a')) === AnimationEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(AnimationEvent.prototype) === Event.prototype"
    ));
    assert!(eval_bool(
        "new AnimationEvent('a').constructor === AnimationEvent"
    ));
    assert!(eval_bool("new AnimationEvent('a') instanceof Event"));
}

#[test]
fn animation_event_survives_gc() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.kept = new AnimationEvent('animationend', \
            {animationName: 'spin', elapsedTime: 2.5, pseudoElement: '::after'});",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        vm_eval_string(&mut vm, "globalThis.kept.animationName"),
        "spin"
    );
    assert_eq!(vm_eval_number(&mut vm, "globalThis.kept.elapsedTime"), 2.5);
    assert_eq!(
        vm_eval_string(&mut vm, "globalThis.kept.pseudoElement"),
        "::after"
    );
}

// ---------------------------------------------------------------------------
// TransitionEvent (CSS Transitions Level 1 §6)
// ---------------------------------------------------------------------------

#[test]
fn transition_event_defaults() {
    assert_eq!(
        eval_string("new TransitionEvent('transitionstart').propertyName"),
        ""
    );
    assert_eq!(
        eval_number("new TransitionEvent('transitionstart').elapsedTime"),
        0.0
    );
    assert_eq!(
        eval_string("new TransitionEvent('transitionstart').pseudoElement"),
        ""
    );
}

#[test]
fn transition_event_init_pass_through() {
    let init = "{propertyName: 'opacity', elapsedTime: 0.5, pseudoElement: '::after'}";
    assert_eq!(
        eval_string(&format!(
            "new TransitionEvent('transitionend', {init}).propertyName"
        )),
        "opacity"
    );
    assert_eq!(
        eval_number(&format!(
            "new TransitionEvent('transitionend', {init}).elapsedTime"
        )),
        0.5
    );
    assert_eq!(
        eval_string(&format!(
            "new TransitionEvent('transitionend', {init}).pseudoElement"
        )),
        "::after"
    );
}

#[test]
fn transition_event_partial_init_uses_defaults_for_missing() {
    let init = "{propertyName: 'transform'}";
    assert_eq!(
        eval_string(&format!("new TransitionEvent('t', {init}).propertyName")),
        "transform"
    );
    assert_eq!(
        eval_number(&format!("new TransitionEvent('t', {init}).elapsedTime")),
        0.0
    );
    assert_eq!(
        eval_string(&format!("new TransitionEvent('t', {init}).pseudoElement")),
        ""
    );
}

#[test]
fn transition_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new TransitionEvent('t')) === TransitionEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(TransitionEvent.prototype) === Event.prototype"
    ));
    assert!(eval_bool(
        "new TransitionEvent('t').constructor === TransitionEvent"
    ));
    assert!(eval_bool("new TransitionEvent('t') instanceof Event"));
}

#[test]
fn transition_event_survives_gc() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.kept = new TransitionEvent('transitionend', \
            {propertyName: 'color', elapsedTime: 1.0, pseudoElement: '::marker'});",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(
        vm_eval_string(&mut vm, "globalThis.kept.propertyName"),
        "color"
    );
    assert_eq!(vm_eval_number(&mut vm, "globalThis.kept.elapsedTime"), 1.0);
    assert_eq!(
        vm_eval_string(&mut vm, "globalThis.kept.pseudoElement"),
        "::marker"
    );
}

// ---------------------------------------------------------------------------
// CloseEvent (WHATWG HTML §10.4)
// ---------------------------------------------------------------------------

#[test]
fn close_event_defaults() {
    // All init members optional.  Defaults: code=0 (no status),
    // reason="", wasClean=false.
    assert_eq!(eval_number("new CloseEvent('close').code"), 0.0);
    assert_eq!(eval_string("new CloseEvent('close').reason"), "");
    assert!(!eval_bool("new CloseEvent('close').wasClean"));
}

#[test]
fn close_event_init_pass_through() {
    let init = "{code: 1006, reason: 'gone', wasClean: true}";
    assert_eq!(
        eval_number(&format!("new CloseEvent('close', {init}).code")),
        1006.0
    );
    assert_eq!(
        eval_string(&format!("new CloseEvent('close', {init}).reason")),
        "gone"
    );
    assert!(eval_bool(&format!(
        "new CloseEvent('close', {init}).wasClean"
    )));
}

#[test]
fn close_event_partial_init_uses_defaults_for_missing() {
    let init = "{code: 1000}";
    assert_eq!(
        eval_number(&format!("new CloseEvent('c', {init}).code")),
        1000.0
    );
    assert_eq!(
        eval_string(&format!("new CloseEvent('c', {init}).reason")),
        ""
    );
    assert!(!eval_bool(&format!("new CloseEvent('c', {init}).wasClean")));
}

#[test]
fn close_event_code_modulo_uint16() {
    // WebIDL `unsigned short` (no `[EnforceRange]` on the IDL):
    // ToNumber → modulo 2^16 truncation.  65536 wraps to 0; 70000
    // wraps to 70000 - 65536 = 4464; -1 wraps to 65535.
    assert_eq!(eval_number("new CloseEvent('c', {code: 65536}).code"), 0.0);
    assert_eq!(
        eval_number("new CloseEvent('c', {code: 70000}).code"),
        4464.0
    );
    assert_eq!(eval_number("new CloseEvent('c', {code: -1}).code"), 65535.0);
    // Fractional truncation toward zero before modulo.
    assert_eq!(
        eval_number("new CloseEvent('c', {code: 1006.9}).code"),
        1006.0
    );
}

#[test]
fn close_event_was_clean_uses_to_boolean() {
    // WebIDL `boolean` → ToBoolean (truthy / falsy), not strict
    // identity to `true`.
    assert!(eval_bool("new CloseEvent('c', {wasClean: 1}).wasClean"));
    assert!(eval_bool("new CloseEvent('c', {wasClean: 'x'}).wasClean"));
    assert!(eval_bool("new CloseEvent('c', {wasClean: {}}).wasClean"));
    assert!(!eval_bool("new CloseEvent('c', {wasClean: 0}).wasClean"));
    assert!(!eval_bool("new CloseEvent('c', {wasClean: ''}).wasClean"));
    assert!(!eval_bool("new CloseEvent('c', {wasClean: null}).wasClean"));
}

#[test]
fn close_event_prototype_chain() {
    assert!(eval_bool(
        "Object.getPrototypeOf(new CloseEvent('c')) === CloseEvent.prototype"
    ));
    assert!(eval_bool(
        "Object.getPrototypeOf(CloseEvent.prototype) === Event.prototype"
    ));
    assert!(eval_bool("new CloseEvent('c').constructor === CloseEvent"));
    assert!(eval_bool("new CloseEvent('c') instanceof Event"));
}

#[test]
fn close_event_survives_gc() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.kept = new CloseEvent('close', \
            {code: 1011, reason: 'server error', wasClean: false});",
    )
    .unwrap();
    vm.inner.collect_garbage();
    assert_eq!(vm_eval_number(&mut vm, "globalThis.kept.code"), 1011.0);
    assert_eq!(
        vm_eval_string(&mut vm, "globalThis.kept.reason"),
        "server error"
    );
    assert!(!vm_eval_bool(&mut vm, "globalThis.kept.wasClean"));
}

// ---------------------------------------------------------------------------
// Cross-cutting (slot #11a init-dict getter throw)
// ---------------------------------------------------------------------------

#[test]
fn animation_event_init_getter_throw_propagates() {
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { \
                new AnimationEvent('a', { get animationName() { throw new Error('ka'); } }); \
             } catch (e) { caught = e.message; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "ka"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn close_event_code_getter_throw_propagates() {
    // Verify that getter-throw inside `code` (which goes through the
    // new `read_uint16` helper → `ToNumber`) surfaces the user error
    // unchanged, matching the existing ErrorEvent pattern.
    let mut vm = Vm::new();
    let check = vm
        .eval(
            "var caught = null; \
             try { \
                new CloseEvent('c', { get code() { throw new Error('boom'); } }); \
             } catch (e) { caught = e.message; } caught;",
        )
        .unwrap();
    match check {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "boom"),
        other => panic!("expected string, got {other:?}"),
    }
}
