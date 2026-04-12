//! Tests for M4-10.2 features: accessor properties, Number/Boolean prototypes,
//! primitive this boxing, arguments object, RegExp execution.

use crate::vm::value::{JsValue, VmError};
use crate::vm::Vm;

fn eval(source: &str) -> Result<JsValue, VmError> {
    let mut vm = Vm::new();
    vm.eval(source)
}

fn eval_number(source: &str) -> f64 {
    match eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    }
}

fn eval_bool(source: &str) -> bool {
    match eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected Boolean, got {other:?}"),
    }
}

// Use a single VM for string results.
fn eval_str(source: &str) -> String {
    let mut vm = Vm::new();
    let val = vm.eval(source).unwrap();
    if let JsValue::String(sid) = val {
        vm.get_string(sid)
    } else {
        panic!("expected String, got {val:?} for: {source}");
    }
}

// ── Accessor Properties ──────────────────────────────────────────────────

#[test]
fn accessor_object_literal_getter() {
    assert_eq!(
        eval_number("var o = { get x() { return 42; } }; o.x;"),
        42.0
    );
}

#[test]
fn accessor_object_literal_setter() {
    assert_eq!(
        eval_number("var o = { set x(v) { this._x = v * 2; } }; o.x = 5; o._x;"),
        10.0,
    );
}

#[test]
fn accessor_object_literal_getter_setter() {
    assert_eq!(
        eval_number("var o = { get x() { return this._x; }, set x(v) { this._x = v + 1; } }; o.x = 10; o.x;"),
        11.0,
    );
}

#[test]
fn accessor_class_getter() {
    assert_eq!(
        eval_number("class C { get val() { return 99; } } new C().val;"),
        99.0,
    );
}

#[test]
fn accessor_class_setter() {
    assert_eq!(
        eval_number("class C { set val(v) { this._v = v; } } var c = new C(); c.val = 7; c._v;"),
        7.0,
    );
}

#[test]
fn accessor_class_getter_setter() {
    assert_eq!(
        eval_number("class C { get x() { return this._x || 0; } set x(v) { this._x = v; } } var c = new C(); c.x = 42; c.x;"),
        42.0,
    );
}

#[test]
fn accessor_define_property_getter() {
    assert_eq!(
        eval_number(
            "var o = {}; Object.defineProperty(o, 'y', { get: function() { return 7; } }); o.y;"
        ),
        7.0,
    );
}

#[test]
fn accessor_define_property_setter() {
    assert_eq!(
        eval_number("var o = {}; Object.defineProperty(o, 'y', { set: function(v) { this._y = v; } }); o.y = 3; o._y;"),
        3.0,
    );
}

#[test]
fn accessor_getter_throws() {
    // Getter throws — error propagates as VmError.
    let mut vm = Vm::new();
    let result = vm.eval("var o = { get x() { throw new Error('fail'); } }; o.x;");
    assert!(result.is_err());
}

#[test]
fn accessor_setter_throws() {
    let mut vm = Vm::new();
    let result = vm.eval("var o = { set x(v) { throw new Error('oops'); } }; o.x = 1;");
    assert!(result.is_err());
}

#[test]
fn accessor_inherited_getter() {
    assert_eq!(
        eval_number("var proto = { get x() { return 100; } }; var o = Object.create(proto); o.x;"),
        100.0,
    );
}

#[test]
fn accessor_getter_this_is_receiver() {
    assert_eq!(
        eval_number("var proto = { get x() { return this._x; } }; var o = Object.create(proto); o._x = 55; o.x;"),
        55.0,
    );
}

// ── Number/Boolean bracket access on prototype ──────────────────────────

#[test]
fn number_bracket_access_to_string() {
    assert_eq!(eval_str("(42)['toString']();"), "42");
}

#[test]
fn boolean_bracket_access_to_string() {
    assert_eq!(eval_str("true['toString']();"), "true");
}

// ── Number.prototype / Boolean.prototype ─────────────────────────────────

#[test]
fn number_prototype_to_string() {
    assert_eq!(eval_str("(42).toString();"), "42");
}

#[test]
fn number_prototype_to_fixed() {
    assert_eq!(eval_str("(3.14159).toFixed(2);"), "3.14");
}

#[test]
fn number_prototype_value_of() {
    assert_eq!(eval_number("(7).valueOf();"), 7.0);
}

#[test]
fn boolean_prototype_to_string() {
    assert_eq!(eval_str("true.toString();"), "true");
    assert_eq!(eval_str("false.toString();"), "false");
}

#[test]
fn boolean_prototype_value_of() {
    assert!(eval_bool("true.valueOf();"));
    assert!(!eval_bool("false.valueOf();"));
}

// ── Primitive this boxing ────────────────────────────────────────────────

#[test]
fn primitive_this_stays_primitive_number() {
    // §9.2.1.2 step 3: in strict mode the primitive receiver passes through
    // unboxed.  All code is strict post-PR1.5, so `this` inside a prototype
    // method invoked on a primitive stays primitive.
    assert_eq!(
        eval_str("Number.prototype.peek = function() { return typeof this; }; (5).peek();"),
        "number",
    );
}

#[test]
fn primitive_this_stays_primitive_string() {
    assert_eq!(
        eval_str("String.prototype.peek = function() { return typeof this; }; 'World'.peek();"),
        "string",
    );
}

#[test]
fn primitive_this_stays_primitive_boolean() {
    assert_eq!(
        eval_str("Boolean.prototype.peek = function() { return typeof this; }; true.peek();"),
        "boolean",
    );
}

// ── arguments object ────────────────────────────────────────────────────

#[test]
fn arguments_length() {
    assert_eq!(
        eval_number("function f() { return arguments.length; } f(1,2,3);"),
        3.0
    );
}

#[test]
fn arguments_index_access() {
    assert_eq!(
        eval_number("function f() { return arguments[1]; } f(10, 20, 30);"),
        20.0
    );
}

#[test]
fn arguments_zero_args() {
    assert_eq!(
        eval_number("function f() { return arguments.length; } f();"),
        0.0
    );
}

#[test]
fn arguments_excess_args() {
    assert_eq!(
        eval_number("function f(a) { return arguments[2]; } f(1, 2, 3);"),
        3.0,
    );
}

#[test]
fn arguments_not_in_arrow() {
    // Arrow functions don't have their own `arguments` — the identifier
    // resolves to the enclosing function's `arguments` binding.
    assert_eq!(
        eval_number("function f() { var g = () => arguments.length; return g(); } f(1,2);"),
        2.0,
    );
}

// ── RegExp ──────────────────────────────────────────────────────────────

#[test]
fn regexp_test_true() {
    assert!(eval_bool("/abc/.test('xabcy');"));
}

#[test]
fn regexp_test_false() {
    assert!(!eval_bool("/abc/.test('xyz');"));
}

#[test]
fn regexp_test_case_insensitive() {
    assert!(eval_bool("/abc/i.test('ABC');"));
}

#[test]
fn regexp_exec_basic() {
    assert_eq!(eval_number("var m = /a(b)c/.exec('xabcy'); m.index;"), 1.0);
}

#[test]
fn regexp_exec_groups() {
    assert_eq!(eval_str("var m = /(\\d+)/.exec('abc123def'); m[0];"), "123");
}

#[test]
fn regexp_exec_null() {
    assert!(eval_bool("/abc/.exec('xyz') === null;"));
}

#[test]
fn regexp_exec_capture_group() {
    assert_eq!(eval_str("/a(b)(c)/.exec('abc')[2];"), "c");
}

#[test]
fn regexp_to_string() {
    assert_eq!(eval_str("/abc/gi.toString();"), "/abc/gi");
}

#[test]
fn string_replace_regexp() {
    assert_eq!(eval_str("'hello'.replace(/l/g, 'r');"), "herro");
}

#[test]
fn string_replace_regexp_no_global() {
    assert_eq!(eval_str("'hello'.replace(/l/, 'r');"), "herlo");
}

#[test]
fn string_match_global() {
    assert_eq!(eval_number("'abcabc'.match(/a/g).length;"), 2.0,);
}

#[test]
fn string_match_no_match() {
    assert!(eval_bool("'abc'.match(/xyz/g) === null;"));
}

#[test]
fn string_search_found() {
    assert_eq!(eval_number("'abc'.search(/b/);"), 1.0);
}

#[test]
fn string_search_not_found() {
    assert_eq!(eval_number("'abc'.search(/z/);"), -1.0);
}

// ── GetProp/SetProp throw_error ─────────────────────────────────────────

#[test]
fn getprop_throw_error_propagates() {
    // Getter throws → error propagates. Cross-frame try/catch for accessor
    // errors requires the VM single dispatcher (M4-11).
    let mut vm = Vm::new();
    let result = vm.eval("var o = { get x() { throw 42; } }; o.x;");
    assert!(result.is_err());
}

#[test]
fn setprop_throw_error_propagates() {
    let mut vm = Vm::new();
    let result = vm.eval("var o = { set x(v) { throw 99; } }; o.x = 1;");
    assert!(result.is_err());
}

// ── RegExp lastIndex for global/sticky ──────────────────────────────────

#[test]
fn regexp_global_last_index() {
    assert!(eval_bool("var r = /a/g; r.test('aa'); r.lastIndex === 1;"));
}

#[test]
fn regexp_global_last_index_reset() {
    assert!(eval_bool(
        "var r = /a/g; r.test('a'); r.test('b'); r.lastIndex === 0;"
    ));
}

// ── Sticky (/y) tests ──────────────────────────────────────────────────

#[test]
fn regexp_sticky_match_at_last_index() {
    // Sticky matches only at lastIndex position.
    assert!(eval_bool("var r = /a/y; r.lastIndex = 2; r.test('xxa');"));
}

#[test]
fn regexp_sticky_fail_resets_last_index() {
    // Sticky failure resets lastIndex to 0.
    assert!(eval_bool(
        "var r = /a/y; r.lastIndex = 1; r.test('xxa'); r.lastIndex === 0;"
    ));
}

#[test]
fn regexp_sticky_no_scan_ahead() {
    // Sticky must not scan ahead — match must start exactly at lastIndex.
    assert!(!eval_bool("var r = /b/y; r.test('ab');"));
}

#[test]
fn regexp_sticky_exec_index() {
    assert_eq!(
        eval_number("var r = /a/y; r.lastIndex = 2; var m = r.exec('xxa'); m.index;"),
        2.0,
    );
}

// ── Additional coverage ────────────────────────────────────────────────

#[test]
fn accessor_on_global_this() {
    // Test via Vm::eval which persists globals across calls.
    let mut vm = Vm::new();
    // Define an accessor on the global object via `globalThis` (strict-mode
    // plain-call `this` is `undefined`, so reach the global object explicitly).
    vm.eval("Object.defineProperty(globalThis, '__test_acc', { get: function() { return 42; }, configurable: true });").unwrap();
    let result = vm.eval("__test_acc;").unwrap();
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn typeof_undeclared_vs_undefined() {
    assert_eq!(eval_str("typeof undeclared_xyz;"), "undefined");
    assert_eq!(eval_str("var x = undefined; typeof x;"), "undefined");
}

#[test]
fn arguments_write() {
    assert_eq!(
        eval_number("function f() { arguments[0] = 99; return arguments[0]; } f(1);"),
        99.0,
    );
}

#[test]
fn match_non_regexp_object() {
    // §21.1.3.11: non-RegExp argument → ToString → regex pattern.
    // `{}` → "[object Object]" which regress compiles as:
    //   `[object Objec]` (character class: o,b,j,e,c,t,space,O)
    //   followed by literal `t]`.
    // The regress engine's match result for this pattern against the
    // subject "[object Object]" is the single character "o" at index 1
    // (the character class consumes one char from the input).  Record
    // the observed behavior to catch regressions in ToString coercion.
    assert_eq!(eval_str("'[object Object]'.match({})[0];"), "o");
    assert_eq!(eval_number("'[object Object]'.match({}).index;"), 1.0);
}

#[test]
fn search_non_regexp_object() {
    // §21.1.3.14: non-RegExp → ToString → regex pattern.  First match
    // index matches the match_non_regexp_object observation above.
    assert_eq!(eval_number("'[object Object]'.search({});"), 1.0);
}

#[test]
fn to_fixed_non_finite() {
    assert_eq!(eval_str("Infinity.toFixed(2);"), "Infinity");
    assert_eq!(eval_str("(-Infinity).toFixed(0);"), "-Infinity");
}

#[test]
fn writable_false_enforcement() {
    // All code is strict post-PR1.5 — writing to a non-writable property
    // throws TypeError rather than silently failing.  Catch the throw and
    // then verify the value is unchanged.
    assert_eq!(
        eval_number(
            "var o = {}; Object.defineProperty(o, 'x', { value: 1, writable: false });
             try { o.x = 2; } catch(_) {}
             o.x;"
        ),
        1.0,
    );
}
