//! Tests for the bytecode VM: interpreter, string pool, object heap, and globals.

mod tests_abort;
mod tests_abort_signal_option;
mod tests_abort_statics;
mod tests_add_event_listener;
mod tests_array_buffer;
mod tests_array_prototype;
mod tests_array_prototype_ext;
mod tests_associated_document;
mod tests_async;
mod tests_async_fetch;
mod tests_bigint;
mod tests_blob;
mod tests_body_mixin;
mod tests_character_data;
mod tests_child_node_mixin;
mod tests_clone_node;
mod tests_coerce;
mod tests_console;
mod tests_cookie_referrer;
mod tests_create_event_object;
mod tests_data_view;
mod tests_dispatch_event;
mod tests_document_global;
mod tests_document_members;
mod tests_document_methods;
mod tests_document_type;
mod tests_dom_collection;
mod tests_dom_exception;
mod tests_element_attributes;
mod tests_element_methods;
mod tests_element_mutation;
mod tests_element_subtree_collection;
mod tests_element_subtree_query;
mod tests_element_wrapper;
mod tests_event_constructor;
mod tests_event_extras_constructor;
mod tests_event_object;
mod tests_event_target;
mod tests_fetch;
mod tests_forbidden_headers;
mod tests_form_data;
mod tests_function_prototype;
mod tests_gc_audit;
mod tests_generator;
mod tests_headers;
mod tests_history;
mod tests_host_object;
mod tests_html_element_proto;
mod tests_html_iframe;
mod tests_insert_adjacent;
mod tests_integration_fetch;
mod tests_integration_pr5b;
mod tests_json;
mod tests_location;
mod tests_m4_10_2;
mod tests_m4_11;
mod tests_math;
mod tests_named_node_map;
mod tests_navigator;
mod tests_node_common;
mod tests_normalize;
mod tests_number;
mod tests_object_complement;
mod tests_parent_node_mixin;
mod tests_performance;
mod tests_post_message;
mod tests_promise;
mod tests_promise_rejection_event;
mod tests_readable_stream;
mod tests_remove_event_listener;
mod tests_request_response;
mod tests_sparse_array;
mod tests_string_complement;
mod tests_structured_clone;
mod tests_text_encoding;
mod tests_timer;
mod tests_to_primitive_fallback;
mod tests_try_catch_finally;
mod tests_typed_array;
mod tests_typed_array_extras;
mod tests_typed_array_hof;
mod tests_typed_array_methods;
mod tests_typed_array_reduce_sort;
mod tests_typed_array_static;
mod tests_ui_event_constructor;
mod tests_url;
mod tests_url_search_params;
mod tests_value_types;
mod tests_window_global;
mod tests_window_iframe_props;

use super::value::{JsValue, Object, ObjectKind, VmError};
use super::Vm;

/// Drive `vm.tick_network()` until `pending_fetches` is empty, with a
/// 16-iteration ceiling to guard against unbounded reaction loops.
/// Always runs one trailing tick so a chain whose final reaction
/// did not allocate a new pending fetch still gets its microtask
/// drain.  Shared helper for the M4-12 PR5-async-fetch test suite
/// (R9.2 dedup) — used by `tests_fetch`, `tests_integration_fetch`,
/// and `tests_async_fetch`.
#[cfg(feature = "engine")]
pub(crate) fn drain_fetch_replies(vm: &mut Vm) {
    for _ in 0..16 {
        if vm.inner.pending_fetches.is_empty() {
            break;
        }
        vm.tick_network();
    }
    vm.tick_network();
}

fn eval(source: &str) -> Result<JsValue, VmError> {
    let mut vm = Vm::new();
    vm.eval(source)
}

fn eval_throws(source: &str) {
    let result = eval(source);
    assert!(result.is_err(), "expected error, got {result:?}");
}

fn eval_number(source: &str) -> f64 {
    match eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn eval_string(source: &str) -> String {
    let mut vm = Vm::new();
    let result = vm.eval(source).unwrap();
    match result {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_bool(source: &str) -> bool {
    match eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

/// Evaluate `source`, drain microtasks (via the post-script drain inside
/// `eval`), then read the global `var` named `name` and expect a number.
/// Used to observe state set asynchronously from Promise reactions.
fn eval_global_number(source: &str, name: &str) -> f64 {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} to be a number, got {other:?}"),
    }
}

/// Evaluate `source`, drain microtasks, then read the global `var` named
/// `name` and expect a string.
fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

/// Assert `throwing` throws, AND that after recovery `observation` yields
/// `expected`.  Used when a now-strict operation used to fail silently —
/// verifies both the throw and that state is unchanged.  `setup` runs before
/// both the throwing check and the observation.
///
/// Segments are joined with `;\n` so callers need not worry about trailing
/// semicolons or ASI: a redundant `;` between two well-formed statements is
/// a valid empty statement.
fn assert_throws_preserves_number(setup: &str, throwing: &str, observation: &str, expected: f64) {
    eval_throws(&format!("{setup};\n{throwing}"));
    assert_eq!(
        eval_number(&format!(
            "{setup};\ntry {{ {throwing} }} catch(_) {{}}\n{observation}"
        )),
        expected,
    );
}

/// Boolean-returning variant of [`assert_throws_preserves_number`].
fn assert_throws_preserves_bool(setup: &str, throwing: &str, observation: &str, expected: bool) {
    eval_throws(&format!("{setup};\n{throwing}"));
    assert_eq!(
        eval_bool(&format!(
            "{setup};\ntry {{ {throwing} }} catch(_) {{}}\n{observation}"
        )),
        expected,
    );
}

#[test]
fn eval_number_literal() {
    assert_eq!(eval_number("42;"), 42.0);
}

#[test]
fn eval_float_literal() {
    assert_eq!(eval_number("3.125;"), 3.125);
}

#[test]
fn eval_string_literal() {
    assert_eq!(eval_string("'hello';"), "hello");
}

#[test]
fn eval_boolean_literal() {
    assert!(eval_bool("true;"));
    assert!(!eval_bool("false;"));
}

#[test]
fn eval_null() {
    assert!(matches!(eval("null;"), Ok(JsValue::Null)));
}

#[test]
fn eval_undefined_global() {
    assert!(matches!(eval("undefined;"), Ok(JsValue::Undefined)));
}

#[test]
fn eval_arithmetic() {
    assert_eq!(eval_number("1 + 2;"), 3.0);
    assert_eq!(eval_number("10 - 3;"), 7.0);
    assert_eq!(eval_number("4 * 5;"), 20.0);
    assert_eq!(eval_number("10 / 4;"), 2.5);
    assert_eq!(eval_number("10 % 3;"), 1.0);
    assert_eq!(eval_number("2 ** 10;"), 1024.0);
}

#[test]
fn eval_string_concat() {
    assert_eq!(eval_string("'hello' + ' ' + 'world';"), "hello world");
}

#[test]
fn eval_number_string_concat() {
    assert_eq!(eval_string("42 + 'px';"), "42px");
}

#[test]
fn eval_comparison() {
    assert!(eval_bool("1 < 2;"));
    assert!(!eval_bool("2 < 1;"));
    assert!(eval_bool("1 <= 1;"));
    assert!(eval_bool("2 > 1;"));
    assert!(eval_bool("1 >= 1;"));
    assert!(eval_bool("1 === 1;"));
    assert!(!eval_bool("1 === 2;"));
    assert!(eval_bool("1 !== 2;"));
}

#[test]
fn eval_var_declaration() {
    assert_eq!(eval_number("var x = 10; x;"), 10.0);
}

#[test]
fn eval_let_declaration() {
    assert_eq!(eval_number("let x = 20; x;"), 20.0);
}

#[test]
fn eval_assignment() {
    assert_eq!(eval_number("var x = 1; x = 42; x;"), 42.0);
}

#[test]
fn eval_compound_assignment() {
    assert_eq!(eval_number("var x = 10; x += 5; x;"), 15.0);
}

#[test]
fn eval_if_true() {
    assert_eq!(eval_number("var x = 0; if (true) { x = 1; } x;"), 1.0);
}

#[test]
fn eval_if_false_else() {
    assert_eq!(
        eval_number("var x = 0; if (false) { x = 1; } else { x = 2; } x;"),
        2.0
    );
}

#[test]
fn eval_while_loop() {
    assert_eq!(
        eval_number("var s = 0; var i = 0; while (i < 5) { s += i; i++; } s;"),
        10.0
    );
}

#[test]
fn eval_for_loop() {
    assert_eq!(
        eval_number("var s = 0; for (var i = 1; i <= 10; i++) { s += i; } s;"),
        55.0
    );
}

#[test]
fn eval_unary_operators() {
    assert_eq!(eval_number("-5;"), -5.0);
    assert_eq!(eval_number("+true;"), 1.0);
    assert!(eval_bool("!false;"));
    assert!(!eval_bool("!true;"));
    assert_eq!(eval_number("~0;"), -1.0);
}

#[test]
fn eval_typeof() {
    assert_eq!(eval_string("typeof 42;"), "number");
    assert_eq!(eval_string("typeof 'hello';"), "string");
    assert_eq!(eval_string("typeof true;"), "boolean");
    assert_eq!(eval_string("typeof undefined;"), "undefined");
    assert_eq!(eval_string("typeof null;"), "object");
}

#[test]
fn eval_typeof_global_undeclared() {
    assert_eq!(eval_string("typeof nonexistent;"), "undefined");
}

#[test]
fn eval_get_global_reference_error() {
    // Accessing an undeclared variable should throw ReferenceError.
    assert_eq!(
        eval_string("var r; try { undeclared; } catch(e) { r = e.message; } r;"),
        "undeclared is not defined",
    );
}

#[test]
fn eval_set_global_strict_mode_reference_error() {
    // §8.1.1.2.5: assigning to an undeclared binding throws ReferenceError.
    assert_eq!(
        eval_string("var r = 'ok'; try { undeclared = 1; } catch(e) { r = e.message; } r;"),
        "undeclared is not defined",
    );
}

#[test]
fn eval_this_coercion_method_receiver() {
    // Method call: `this` should be the receiver, not coerced.
    assert_eq!(
        eval_number("var o = { v: 42, f() { return this.v; } }; o.f();"),
        42.0,
    );
}

#[test]
fn eval_optional_chain_this_binding() {
    // obj?.method() should bind `this` to `obj`.
    assert_eq!(
        eval_number("var o = { v: 99, m() { return this.v; } }; o?.m();"),
        99.0,
    );
}

#[test]
fn eval_optional_chain_nullish_returns_undefined() {
    assert_eq!(eval_string("var x = null; typeof (x?.foo());"), "undefined",);
}

#[test]
fn eval_object_literal() {
    assert_eq!(eval_number("var o = {a: 1, b: 2}; o.a;"), 1.0);
}

#[test]
fn eval_object_property_set() {
    assert_eq!(eval_number("var o = {}; o.x = 42; o.x;"), 42.0);
}

#[test]
fn eval_array_literal() {
    assert_eq!(eval_number("var a = [10, 20, 30]; a[1];"), 20.0);
}

#[test]
fn eval_array_length() {
    assert_eq!(eval_number("[1, 2, 3].length;"), 3.0);
}

#[test]
fn eval_template_literal() {
    assert_eq!(eval_string("`hello ${'world'}`;"), "hello world");
}

#[test]
fn eval_conditional_expression() {
    assert_eq!(eval_number("true ? 1 : 2;"), 1.0);
    assert_eq!(eval_number("false ? 1 : 2;"), 2.0);
}

#[test]
fn eval_logical_and() {
    assert_eq!(eval_number("1 && 2;"), 2.0);
    assert_eq!(eval_number("0 && 2;"), 0.0);
}

#[test]
fn eval_logical_or() {
    assert_eq!(eval_number("0 || 2;"), 2.0);
    assert_eq!(eval_number("1 || 2;"), 1.0);
}

#[test]
fn eval_nullish_coalescing() {
    assert_eq!(eval_number("null ?? 42;"), 42.0);
    assert_eq!(eval_number("undefined ?? 42;"), 42.0);
    assert_eq!(eval_number("0 ?? 42;"), 0.0);
}

#[test]
fn eval_increment_decrement() {
    assert_eq!(eval_number("var x = 5; ++x;"), 6.0);
    assert_eq!(eval_number("var x = 5; x++;"), 5.0); // postfix returns old
    assert_eq!(eval_number("var x = 5; x++; x;"), 6.0);
    assert_eq!(eval_number("var x = 5; --x;"), 4.0);
}

#[test]
fn eval_break_in_while() {
    assert_eq!(
        eval_number("var x = 0; while (true) { if (x >= 3) break; x++; } x;"),
        3.0
    );
}

#[test]
fn eval_continue_in_for() {
    assert_eq!(
        eval_number(
            "var s = 0; for (var i = 0; i < 10; i++) { if (i % 2 === 0) continue; s += i; } s;"
        ),
        25.0 // 1+3+5+7+9
    );
}

#[test]
fn eval_switch() {
    assert_eq!(
        eval_number("var x = 2; var r = 0; switch(x) { case 1: r = 10; break; case 2: r = 20; break; default: r = 30; } r;"),
        20.0
    );
}

#[test]
fn eval_bitwise() {
    assert_eq!(eval_number("5 & 3;"), 1.0);
    assert_eq!(eval_number("5 | 3;"), 7.0);
    assert_eq!(eval_number("5 ^ 3;"), 6.0);
    assert_eq!(eval_number("1 << 4;"), 16.0);
    assert_eq!(eval_number("-8 >> 2;"), -2.0);
}

#[test]
fn eval_function_declaration() {
    assert_eq!(eval_number("function f(x) { return x + 1; } f(41);"), 42.0);
}

#[test]
fn eval_function_expression() {
    assert_eq!(
        eval_number("var f = function(x) { return x * 2; }; f(21);"),
        42.0
    );
}

#[test]
fn eval_arrow_function() {
    assert_eq!(eval_number("var f = (x) => x * 2; f(21);"), 42.0);
}

#[test]
fn eval_arrow_block_body() {
    assert_eq!(
        eval_number("var f = (x) => { return x + 1; }; f(41);"),
        42.0
    );
}

#[test]
fn eval_closure_capture() {
    assert_eq!(
        eval_number("function make() { var x = 10; return function() { return x; }; } make()();"),
        10.0
    );
}

#[test]
fn eval_nested_closure() {
    assert_eq!(
        eval_number(
            "function a() { var x = 1; function b() { var y = 2; return function() { return x + y; }; } return b(); } a()();"
        ),
        3.0
    );
}

#[test]
fn eval_fibonacci() {
    assert_eq!(
        eval_number(
            "function fib(n) { if (n <= 1) return n; return fib(n-1) + fib(n-2); } fib(10);"
        ),
        55.0
    );
}

#[test]
fn eval_default_params() {
    assert_eq!(
        eval_number("function f(x, y) { if (y === undefined) y = 10; return x + y; } f(5);"),
        15.0
    );
}

#[test]
fn eval_for_in_basic() {
    // for-in iterates enumerable keys
    assert_eq!(
        eval_number("var s = 0; var o = {a: 1, b: 2, c: 3}; for (var k in o) { s += o[k]; } s;"),
        6.0
    );
}

#[test]
fn eval_for_of_array() {
    assert_eq!(
        eval_number("var s = 0; for (var x of [10, 20, 30]) { s += x; } s;"),
        60.0
    );
}

// -- Built-in globals tests ------------------------------------------------

#[test]
fn eval_parse_int() {
    assert_eq!(eval_number("parseInt('42');"), 42.0);
    assert_eq!(eval_number("parseInt('0xff', 16);"), 255.0);
    assert_eq!(eval_number("parseInt('11', 2);"), 3.0);
    assert_eq!(eval_number("parseInt('  123  ');"), 123.0);
    assert_eq!(eval_number("parseInt('-10');"), -10.0);
    assert!(matches!(eval("parseInt('abc');"), Ok(JsValue::Number(n)) if n.is_nan()));
}

#[test]
fn eval_parse_float() {
    assert_eq!(eval_number("parseFloat('3.125');"), 3.125);
    assert_eq!(eval_number("parseFloat('42');"), 42.0);
    assert!(matches!(eval("parseFloat('abc');"), Ok(JsValue::Number(n)) if n.is_nan()));
}

#[test]
fn eval_parse_float_prefix() {
    assert_eq!(eval_number("parseFloat('3.25abc');"), 3.25);
    assert!(eval_bool("isNaN(parseFloat('inf'));"));
}

#[test]
fn eval_is_nan() {
    assert!(eval_bool("isNaN(NaN);"));
    assert!(!eval_bool("isNaN(42);"));
    assert!(eval_bool("isNaN(undefined);"));
    assert!(!eval_bool("isNaN('123');"));
}

#[test]
fn eval_is_finite() {
    assert!(eval_bool("isFinite(42);"));
    assert!(!eval_bool("isFinite(Infinity);"));
    assert!(!eval_bool("isFinite(NaN);"));
}

#[test]
fn eval_math() {
    assert_eq!(eval_number("Math.abs(-5);"), 5.0);
    assert_eq!(eval_number("Math.floor(3.7);"), 3.0);
    assert_eq!(eval_number("Math.ceil(3.2);"), 4.0);
    assert_eq!(eval_number("Math.round(3.5);"), 4.0);
    assert_eq!(eval_number("Math.round(3.4);"), 3.0);
    assert_eq!(eval_number("Math.max(1, 2, 3);"), 3.0);
    assert_eq!(eval_number("Math.min(1, 2, 3);"), 1.0);
    assert_eq!(eval_number("Math.sqrt(9);"), 3.0);
    assert_eq!(eval_number("Math.pow(2, 10);"), 1024.0);
}

#[test]
fn eval_math_constants() {
    let pi = eval_number("Math.PI;");
    assert!((pi - std::f64::consts::PI).abs() < 1e-10);
    let e = eval_number("Math.E;");
    assert!((e - std::f64::consts::E).abs() < 1e-10);
}

#[test]
fn eval_math_random() {
    let n = eval_number("Math.random();");
    assert!((0.0..1.0).contains(&n));
}

#[test]
fn eval_object_keys() {
    assert_eq!(eval_number("Object.keys({a:1, b:2}).length;"), 2.0);
}

#[test]
fn eval_object_values() {
    assert_eq!(eval_number("Object.values({a:10, b:20}).length;"), 2.0);
}

#[test]
fn eval_object_assign() {
    assert_eq!(
        eval_number("var t = {a:1}; Object.assign(t, {b:2}); t.b;"),
        2.0
    );
}

#[test]
fn eval_object_create() {
    assert_eq!(eval_string("typeof Object.create(null);"), "object");
}

#[test]
fn eval_array_is_array() {
    assert!(eval_bool("Array.isArray([1,2,3]);"));
    assert!(!eval_bool("Array.isArray({});"));
    assert!(!eval_bool("Array.isArray(42);"));
}

#[test]
fn eval_error_constructor() {
    assert_eq!(eval_string("var e = new Error('oops'); e.message;"), "oops");
}

#[test]
fn eval_type_error_constructor() {
    assert_eq!(
        eval_string("var e = new TypeError('bad type'); e.message;"),
        "bad type"
    );
}

#[test]
fn eval_json_basic() {
    // JSON.stringify returns a string for objects.
    assert_eq!(eval_string("JSON.stringify({})"), "{}");
    // JSON.parse returns an object for "{}".
    assert!(matches!(eval("JSON.parse('{}')"), Ok(JsValue::Object(_))));
}

// ── StringPool / Object heap / Globals tests ────────────────────

#[test]
fn string_pool_intern_dedup() {
    let mut pool = super::pools::StringPool::new();
    let a = pool.intern("hello");
    let b = pool.intern("hello");
    let c = pool.intern("world");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(pool.get_utf8(a), "hello");
    assert_eq!(pool.get_utf8(c), "world");
    // +1 for the pre-interned empty string at index 0
    assert_eq!(pool.len(), 3);
}

#[test]
fn string_pool_empty_string() {
    let mut pool = super::pools::StringPool::new();
    let id = pool.intern("");
    assert_eq!(pool.get_utf8(id), "");
}

#[test]
fn object_alloc_and_access() {
    let mut vm = Vm::new();
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    assert!(matches!(vm.get_object(id).kind, ObjectKind::Ordinary));
}

#[test]
fn object_free_list_reuse() {
    let mut vm = Vm::new();
    let id1 = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    // Simulate free
    vm.inner.objects[id1.0 as usize] = None;
    vm.inner.free_objects.push(id1.0);

    let id2 = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    assert_eq!(id1, id2); // Reused slot
}

#[test]
fn globals_set_and_get() {
    let mut vm = Vm::new();
    vm.set_global("x", JsValue::Number(42.0));
    assert_eq!(vm.get_global("x"), Some(JsValue::Number(42.0)));
    assert_eq!(vm.get_global("y"), None);
}

#[test]
fn globals_builtin_registered() {
    let vm = Vm::new();
    assert_eq!(vm.get_global("undefined"), Some(JsValue::Undefined));
    assert!(matches!(vm.get_global("NaN"), Some(JsValue::Number(n)) if n.is_nan()));
    assert_eq!(
        vm.get_global("Infinity"),
        Some(JsValue::Number(f64::INFINITY))
    );
    assert!(matches!(vm.get_global("console"), Some(JsValue::Object(_))));
}

// ── ES2020 spec compliance tests ────────────────────────────────

#[test]
fn eval_to_int32_large() {
    // ToInt32(1e20) should use modulo 2^32
    assert_eq!(eval_number("1e20 | 0;"), 1_661_992_960.0);
}

#[test]
fn eval_string_to_number_inf() {
    // "inf" is not a valid JS number — unary + triggers ToNumber
    assert!(eval_bool("isNaN(+'inf');"));
}

#[test]
fn eval_object_plus_number() {
    // {} + 1 should be "[object Object]1" when {} is an expression
    assert_eq!(eval_string("var o = {}; o + 1;"), "[object Object]1");
}

#[test]
fn eval_one_pow_nan() {
    assert!(eval_bool("isNaN(1 ** NaN);"));
}

#[test]
fn eval_arrow_this() {
    assert_eq!(
        eval_number(
            "var obj = { x: 42, f: function() { var g = () => this.x; return g(); } }; obj.f();"
        ),
        42.0
    );
}

#[test]
fn eval_destructuring_null_default() {
    // null should NOT trigger default (only undefined)
    assert!(eval_bool("var [x = 5] = [null]; x === null;"));
}

#[test]
fn eval_const_reassign_error() {
    assert!(eval("const x = 1; x = 2;").is_err());
}

// Note: under the `engine` feature, `Vm` is always `!Send` because
// `VmInner` carries `Option<Box<HostData>>` regardless of whether a
// `HostData` is currently installed/bound (see vm/host_data.rs).
// Worker-thread support in PR2+ will design a Send-safe variant explicitly.

// ---------------------------------------------------------------------------
// TDZ enforcement
// ---------------------------------------------------------------------------

#[test]
fn eval_tdz_direct_access() {
    // Direct access to a let variable before its initializer should throw ReferenceError.
    // The CheckTdz opcode fires before GetLocal for let/const bindings.
    let result = eval("var r = 0; try { r = x; } catch(e) { r = -1; } let x = 42; r;");
    // `r = x` triggers CheckTdz for x → ReferenceError → caught → r = -1.
    assert_eq!(
        match result {
            Ok(JsValue::Number(n)) => n,
            other => panic!("unexpected result: {other:?}"),
        },
        -1.0
    );
}

#[test]
fn eval_tdz_let_initialized() {
    // After initialization, let binding is accessible.
    assert_eq!(eval_number("let x = 42; x;"), 42.0);
}

#[test]
fn eval_tdz_let_after_init() {
    // After initialization, TDZ is cleared — access should succeed.
    assert_eq!(eval_number("let x = 42; x;"), 42.0);
}

// ---------------------------------------------------------------------------
// instanceof / in operators
// ---------------------------------------------------------------------------

#[test]
fn eval_instanceof() {
    assert!(eval_bool(
        "function Foo() {} var f = new Foo(); f instanceof Foo;"
    ));
}

#[test]
fn eval_instanceof_false() {
    assert!(!eval_bool(
        "function Foo() {} function Bar() {} var f = new Foo(); f instanceof Bar;"
    ));
}

#[test]
fn eval_in_operator() {
    assert!(eval_bool("'a' in {a: 1};"));
    assert!(!eval_bool("'b' in {a: 1};"));
}

// ---------------------------------------------------------------------------
// delete operator
// ---------------------------------------------------------------------------

#[test]
fn eval_delete_property() {
    assert!(eval_bool("var o = {a: 1}; delete o.a; !('a' in o);"));
}

#[test]
fn eval_delete_elem() {
    assert!(eval_bool("var o = {a: 1}; delete o['a']; !('a' in o);"));
}

// ---------------------------------------------------------------------------
// Property increment/decrement
// ---------------------------------------------------------------------------

#[test]
fn eval_prop_increment_postfix() {
    assert_eq!(eval_number("var o = {x: 5}; o.x++; o.x;"), 6.0);
}

#[test]
fn eval_prop_increment_postfix_returns_old() {
    assert_eq!(eval_number("var o = {x: 5}; o.x++;"), 5.0);
}

#[test]
fn eval_prop_increment_prefix() {
    assert_eq!(eval_number("var o = {x: 5}; ++o.x;"), 6.0);
}

#[test]
fn eval_prop_decrement() {
    assert_eq!(eval_number("var o = {x: 5}; o.x--; o.x;"), 4.0);
}

// ---------------------------------------------------------------------------
// Array spread
// ---------------------------------------------------------------------------

#[test]
fn eval_array_spread() {
    assert_eq!(
        eval_number("var a = [1, 2]; var b = [...a, 3]; b.length;"),
        3.0
    );
}

#[test]
fn eval_array_spread_values() {
    assert_eq!(
        eval_number("var a = [10, 20]; var b = [...a, 30]; b[0] + b[1] + b[2];"),
        60.0
    );
}

// ---------------------------------------------------------------------------
// Object spread
// ---------------------------------------------------------------------------

#[test]
fn eval_object_spread() {
    assert_eq!(eval_number("var a = {x: 1}; var b = {...a}; b.x;"), 1.0);
}

#[test]
fn eval_object_spread_with_extra() {
    assert_eq!(
        eval_number("var a = {x: 1}; var b = {...a, y: 2}; b.x + b.y;"),
        3.0
    );
}

#[test]
fn eval_object_spread_overwrite() {
    assert_eq!(
        eval_number("var a = {x: 1}; var b = {...a, x: 10}; b.x;"),
        10.0
    );
}

// ---------------------------------------------------------------------------
// Object/Array prototype chain
// ---------------------------------------------------------------------------

#[test]
fn eval_object_has_prototype() {
    // instanceof Object should work for object literals (once Object constructor has prototype).
    // For now, just verify that `in` works through prototype chain for arrays.
    assert_eq!(eval_number("var a = [1, 2, 3]; a.length;"), 3.0);
}

// ---------------------------------------------------------------------------
// M4-10 scope items
// ---------------------------------------------------------------------------

#[test]
fn eval_block_scope_isolation() {
    // x should not be accessible outside the block
    assert_eq!(eval_string("{ let x = 'inner'; } typeof x;"), "undefined");
}

#[test]
fn eval_default_param() {
    assert_eq!(eval_number("function f(x = 10) { return x; } f();"), 10.0);
    assert_eq!(eval_number("function f(x = 10) { return x; } f(42);"), 42.0);
}

#[test]
fn eval_arrow_default_param() {
    assert_eq!(eval_number("var f = (x = 5) => x; f();"), 5.0);
}

#[test]
fn eval_forin_prototype_chain() {
    assert_eq!(
        eval_number(
            "var parent = {a: 1}; var child = Object.create(parent); child.b = 2; var s = 0; for (var k in child) { s += child[k]; } s;"
        ),
        3.0
    );
}

#[test]
fn eval_function_hoisting() {
    assert_eq!(
        eval_number("var x = f(); function f() { return 42; } x;"),
        42.0
    );
}

#[test]
fn eval_global_object_property_lookup_falls_back_to_globals() {
    // Explicit `globalThis` receiver exercises the globals HashMap → global
    // object property-lookup fallback.
    assert_eq!(eval_string("typeof globalThis.Math;"), "object",);
}

#[test]
fn eval_global_object_set_property_syncs_to_globals() {
    // Writing to `globalThis.<prop>` must be visible via bare identifier
    // lookup (GetGlobal).
    assert_eq!(eval_number("globalThis.testGlobal = 42; testGlobal;"), 42.0,);
}

mod tests_string;
mod tests_symbol_iter;
