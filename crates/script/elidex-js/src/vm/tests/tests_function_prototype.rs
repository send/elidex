//! Tests for Function.prototype methods (ES2020 \u00a719.2.3).

// ---------------------------------------------------------------------------
// Function.prototype.call (\u00a719.2.3.3)
// ---------------------------------------------------------------------------

#[test]
fn call_basic_this_arg() {
    assert_eq!(
        super::eval_number("var obj = { x: 42 }; function f() { return this.x; } f.call(obj);"),
        42.0
    );
}

#[test]
fn call_with_multiple_args() {
    assert_eq!(
        super::eval_number("function f(a, b, c) { return a + b + c; } f.call(null, 10, 20, 30);"),
        60.0
    );
}

#[test]
fn call_this_arg_with_args() {
    assert_eq!(
        super::eval_number(
            "var obj = { x: 100 }; function f(a) { return this.x + a; } f.call(obj, 5);"
        ),
        105.0
    );
}

#[test]
fn call_no_args_this_undefined() {
    // When called without arguments, thisArg is undefined.
    // In sloppy mode, `this` is coerced to the global object.
    assert_eq!(
        super::eval_string("function f() { return typeof this; } f.call();"),
        "object"
    );
}

#[test]
fn call_on_non_function_throws() {
    // §19.2.3.1 step 1: IsCallable(this) must be true, else TypeError.
    // Use `Function.prototype.call.call(42)` to bypass property lookup
    // on the primitive and exercise the IsCallable branch directly.
    super::eval_throws("Function.prototype.call.call(42);");
}

// ---------------------------------------------------------------------------
// Function.prototype.apply (\u00a719.2.3.1)
// ---------------------------------------------------------------------------

#[test]
fn apply_basic_array_args() {
    assert_eq!(
        super::eval_number("function f(a, b) { return a + b; } f.apply(null, [10, 20]);"),
        30.0
    );
}

#[test]
fn apply_with_this_arg() {
    assert_eq!(
        super::eval_number(
            "var obj = { x: 7 }; function f(a) { return this.x * a; } f.apply(obj, [6]);"
        ),
        42.0
    );
}

#[test]
fn apply_null_args_array() {
    // apply with null argsArray means no arguments passed.
    assert_eq!(
        super::eval_number("function f() { return arguments.length; } f.apply(null, null);"),
        0.0
    );
}

#[test]
fn apply_undefined_args_array() {
    assert_eq!(
        super::eval_number("function f() { return arguments.length; } f.apply(null, undefined);"),
        0.0
    );
}

#[test]
fn apply_empty_array() {
    assert_eq!(
        super::eval_number("function f() { return arguments.length; } f.apply(null, []);"),
        0.0
    );
}

#[test]
fn apply_on_non_function_throws() {
    // §19.2.3.3 step 1: IsCallable(this) must be true.
    super::eval_throws("Function.prototype.apply.call({}, null, []);");
}

// ---------------------------------------------------------------------------
// Function.prototype.bind (\u00a719.2.3.2)
// ---------------------------------------------------------------------------

#[test]
fn bind_basic_this_arg() {
    assert_eq!(
        super::eval_number(
            "var obj = { x: 42 }; function f() { return this.x; } var g = f.bind(obj); g();"
        ),
        42.0
    );
}

#[test]
fn bind_with_partial_args() {
    assert_eq!(
        super::eval_number("function f(a, b) { return a + b; } var g = f.bind(null, 10); g(20);"),
        30.0
    );
}

#[test]
fn bind_additional_args_appended() {
    assert_eq!(
        super::eval_number(
            "function f(a, b, c) { return a + b + c; } var g = f.bind(null, 1, 2); g(3);"
        ),
        6.0
    );
}

#[test]
fn bind_nested() {
    assert_eq!(
        super::eval_number(
            "function f(a, b, c) { return a + b + c; } var g = f.bind(null, 10); var h = g.bind(null, 20); h(30);"
        ),
        60.0
    );
}

#[test]
fn bind_on_non_function_throws() {
    // §19.2.3.2 step 2: IsCallable(Target) must be true.
    super::eval_throws("Function.prototype.bind.call(42, null);");
}

#[test]
fn bind_preserves_this_across_calls() {
    assert_eq!(
        super::eval_number(
            "var obj = { x: 99 }; function f() { return this.x; } var g = f.bind(obj); g() + g();"
        ),
        198.0
    );
}

#[test]
fn bind_partial_application_multiple_calls() {
    // Bound function with partial args should work correctly across multiple calls.
    assert_eq!(
        super::eval_string(
            "function f(a, b) { return a + '-' + b; } var g = f.bind(null, 'hello'); g('world');"
        ),
        "hello-world"
    );
}

// ---------------------------------------------------------------------------
// Function.prototype.toString (\u00a719.2.3.5)
// ---------------------------------------------------------------------------

#[test]
fn tostring_named_function() {
    let s = super::eval_string("function foo() {} foo.toString();");
    assert!(
        s.contains("foo"),
        "toString should contain function name, got: {s}"
    );
    assert!(
        s.contains("function"),
        "toString should contain 'function', got: {s}"
    );
}

#[test]
fn tostring_anonymous_function() {
    let s = super::eval_string("(function() {}).toString();");
    assert!(
        s.contains("function"),
        "anonymous function toString should contain 'function', got: {s}"
    );
}

#[test]
fn tostring_bound_function() {
    let s = super::eval_string("function foo() {} var g = foo.bind(null); g.toString();");
    assert!(
        s.contains("bound") || s.contains("native code"),
        "bound function toString should contain 'bound' or 'native code', got: {s}"
    );
}

// ---------------------------------------------------------------------------
// BoundFunction dispatch
// ---------------------------------------------------------------------------

#[test]
fn bound_array_push_call() {
    // Using call on a bound version of Array.prototype.push.
    assert_eq!(
        super::eval_number(
            "var a = [1, 2]; var push = Array.prototype.push.bind(a); push(3); a.length;"
        ),
        3.0
    );
}

#[test]
fn bound_function_preserves_partial_application() {
    assert_eq!(
        super::eval_number(
            "function add(a, b) { return a + b; } var add5 = add.bind(null, 5); add5(10) + add5(20);"
        ),
        40.0 // 15 + 25
    );
}

#[test]
fn bound_method_keeps_this() {
    assert_eq!(
        super::eval_number(
            "var obj = { x: 10, get: function() { return this.x; } }; var g = obj.get.bind(obj); g();"
        ),
        10.0
    );
}

// ---------------------------------------------------------------------------
// Function.prototype on prototype chain
// ---------------------------------------------------------------------------

#[test]
fn all_functions_have_call() {
    assert!(super::eval_bool(
        "function f() {} typeof f.call === 'function';"
    ));
}

#[test]
fn all_functions_have_apply() {
    assert!(super::eval_bool(
        "function f() {} typeof f.apply === 'function';"
    ));
}

#[test]
fn all_functions_have_bind() {
    assert!(super::eval_bool(
        "function f() {} typeof f.bind === 'function';"
    ));
}

#[test]
fn all_functions_have_tostring() {
    assert!(super::eval_bool(
        "function f() {} typeof f.toString === 'function';"
    ));
}

#[test]
fn arrow_function_has_call() {
    assert!(super::eval_bool(
        "var f = () => 1; typeof f.call === 'function';"
    ));
}

#[test]
fn arrow_function_has_bind() {
    assert!(super::eval_bool(
        "var f = () => 1; typeof f.bind === 'function';"
    ));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn call_with_primitive_this_boxing() {
    // In sloppy mode, primitive thisArg should be boxed to object.
    assert_eq!(
        super::eval_string("function f() { return typeof this; } f.call(42);"),
        "object"
    );
}

#[test]
fn apply_with_primitive_this_boxing() {
    assert_eq!(
        super::eval_string("function f() { return typeof this; } f.apply('hello', []);"),
        "object"
    );
}

// ── bind length/name (§19.2.3.2 steps 4-5) ─────────────────────────

#[test]
fn bind_length_reflects_remaining_params() {
    assert_eq!(
        super::eval_number("function f(a, b, c) {} f.bind(null).length;"),
        3.0
    );
    assert_eq!(
        super::eval_number("function f(a, b, c) {} f.bind(null, 1).length;"),
        2.0
    );
    assert_eq!(
        super::eval_number("function f(a, b, c) {} f.bind(null, 1, 2, 3).length;"),
        0.0
    );
}

#[test]
fn bind_name_has_bound_prefix() {
    assert_eq!(
        super::eval_string("function foo() {} foo.bind(null).name;"),
        "bound foo"
    );
}

#[test]
fn bind_nested_name_has_double_bound() {
    assert_eq!(
        super::eval_string("function bar() {} bar.bind(null).bind(null).name;"),
        "bound bound bar"
    );
}

// ── toString nested BoundFunction ───────────────────────────────────

#[test]
fn tostring_double_bound() {
    assert_eq!(
        super::eval_string("function baz() {} baz.bind(null).bind(null).toString();"),
        "function bound bound baz() { [native code] }"
    );
}

// ── setPrototypeOf non-extensible ───────────────────────────────────

#[test]
fn set_prototype_of_frozen_throws() {
    super::eval_throws("var o = Object.freeze({}); Object.setPrototypeOf(o, {});");
}

#[test]
fn set_prototype_of_frozen_same_proto_ok() {
    // Setting the same prototype on a non-extensible object is allowed.
    assert!(super::eval_bool(
        "var p = {}; var o = Object.create(p); Object.preventExtensions(o); Object.setPrototypeOf(o, p); true;"
    ));
}

// ── keys/values/entries null TypeError ──────────────────────────────

#[test]
fn object_keys_null_throws() {
    super::eval_throws("Object.keys(null);");
}

#[test]
fn object_values_undefined_throws() {
    super::eval_throws("Object.values(undefined);");
}

#[test]
fn object_entries_null_throws() {
    super::eval_throws("Object.entries(null);");
}

// ── getOwnPropertyNames includes array indices ─────────────────────

#[test]
fn get_own_property_names_array_indices() {
    assert!(super::eval_bool(
        "var names = Object.getOwnPropertyNames([1,2,3]); names.indexOf('0') >= 0 && names.indexOf('1') >= 0 && names.indexOf('2') >= 0;"
    ));
}
