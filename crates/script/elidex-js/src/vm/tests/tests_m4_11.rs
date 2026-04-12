//! M4-11 tests: NativeContext re-entrancy + property descriptor enforcement.

use super::{eval_bool, eval_number, eval_string, eval_throws};

// ─── A3: NativeContext re-entrancy (getter invocation from natives) ──────

#[test]
fn object_values_invokes_getter() {
    assert_eq!(
        eval_number(
            "var o = {}; Object.defineProperty(o, 'x', { get: function() { return 42; }, enumerable: true }); Object.values(o)[0];"
        ),
        42.0
    );
}

#[test]
fn object_values_mixed_data_and_accessor() {
    assert_eq!(
        eval_number(
            "var o = { a: 1 }; Object.defineProperty(o, 'b', { get: function() { return 2; }, enumerable: true }); \
             var v = Object.values(o); v[0] + v[1];"
        ),
        3.0
    );
}

#[test]
fn object_assign_invokes_getter() {
    assert_eq!(
        eval_number(
            "var src = {}; Object.defineProperty(src, 'x', { get: function() { return 99; }, enumerable: true }); \
             var dst = {}; Object.assign(dst, src); dst.x;"
        ),
        99.0
    );
}

#[test]
fn object_values_getter_mutates_later_data_property() {
    // Getter on 'a' (defined first) mutates 'b' (defined second).
    // Object.values enumerates in creation order: a then b.
    // a's getter sets b=99, so b must be read as 99.
    assert_eq!(
        eval_number(
            "var o = {}; \
             Object.defineProperty(o, 'a', { get: function() { o.b = 99; return 1; }, enumerable: true }); \
             Object.defineProperty(o, 'b', { value: 2, writable: true, enumerable: true, configurable: true }); \
             Object.values(o)[1];"
        ),
        99.0
    );
}

#[test]
fn object_assign_getter_mutates_later_data_property() {
    assert_eq!(
        eval_number(
            "var src = {}; \
             Object.defineProperty(src, 'a', { get: function() { src.b = 99; return 1; }, enumerable: true }); \
             Object.defineProperty(src, 'b', { value: 2, writable: true, enumerable: true, configurable: true }); \
             var dst = {}; Object.assign(dst, src); dst.b;"
        ),
        99.0
    );
}

#[test]
fn spread_getter_mutates_later_data_property() {
    assert_eq!(
        eval_number(
            "var src = {}; \
             Object.defineProperty(src, 'a', { get: function() { src.b = 99; return 1; }, enumerable: true }); \
             Object.defineProperty(src, 'b', { value: 2, writable: true, enumerable: true, configurable: true }); \
             var dst = { ...src }; dst.b;"
        ),
        99.0
    );
}

#[test]
fn spread_invokes_getter() {
    assert_eq!(
        eval_number(
            "var src = {}; Object.defineProperty(src, 'x', { get: function() { return 7; }, enumerable: true }); \
             var dst = { ...src }; dst.x;"
        ),
        7.0
    );
}

#[test]
fn define_property_descriptor_getter() {
    // Descriptor fields read via Get: the 'value' field is itself an accessor.
    assert_eq!(
        eval_number(
            "var o = {}; \
             var desc = {}; \
             Object.defineProperty(desc, 'value', { get: function() { return 123; } }); \
             Object.defineProperty(desc, 'writable', { get: function() { return true; } }); \
             Object.defineProperty(desc, 'enumerable', { get: function() { return true; } }); \
             Object.defineProperty(desc, 'configurable', { get: function() { return true; } }); \
             Object.defineProperty(o, 'x', desc); o.x;"
        ),
        123.0
    );
}

#[test]
fn define_property_descriptor_field_order() {
    // §6.2.5.5: enumerable getter runs before value getter.
    // The enumerable getter sets desc.value, which should be visible
    // when value is read later.
    assert_eq!(
        eval_number(
            "var o = {}; var desc = {}; \
             Object.defineProperty(desc, 'enumerable', { get: function() { \
                 Object.defineProperty(desc, 'value', { value: 77, configurable: true }); \
                 return true; \
             } }); \
             desc.value = 1; \
             Object.defineProperty(o, 'x', desc); o.x;"
        ),
        77.0
    );
}

#[test]
fn to_string_tag_accessor() {
    // Use o.toString() directly (Function.prototype.call not yet implemented).
    assert_eq!(
        eval_string(
            "var o = {}; Object.defineProperty(o, Symbol.toStringTag, { get: function() { return 'Custom'; } }); \
             o.toString();"
        ),
        "[object Custom]"
    );
}

// ─── Cross-frame exception propagation ──────────────────────────────────

#[test]
fn getter_throw_caught_by_outer_try() {
    assert_eq!(
        eval_number(
            "var o = {}; Object.defineProperty(o, 'x', { get: function() { throw 42; }, enumerable: true }); \
             try { Object.values(o); } catch (e) { e; }"
        ),
        42.0
    );
}

#[test]
fn assign_getter_throw_propagates() {
    assert_eq!(
        eval_string(
            "var src = {}; Object.defineProperty(src, 'x', { get: function() { throw 'boom'; }, enumerable: true }); \
             try { Object.assign({}, src); } catch (e) { e; }"
        ),
        "boom"
    );
}

#[test]
fn spread_getter_throw_propagates() {
    assert_eq!(
        eval_string(
            "var src = {}; Object.defineProperty(src, 'x', { get: function() { throw 'bang'; }, enumerable: true }); \
             try { var r = { ...src }; } catch (e) { e; }"
        ),
        "bang"
    );
}

#[test]
fn nested_reentrant_getter() {
    // Getter calls Object.values on another object with a getter.
    assert_eq!(
        eval_number(
            "var inner = {}; Object.defineProperty(inner, 'v', { get: function() { return 10; }, enumerable: true }); \
             var outer = {}; Object.defineProperty(outer, 'x', { get: function() { return Object.values(inner)[0]; }, enumerable: true }); \
             Object.values(outer)[0];"
        ),
        10.0
    );
}

// ─── D1: writable:false enforcement ─────────────────────────────────────

#[test]
fn writable_false_strict_throws() {
    eval_throws(
        "'use strict'; var o = {}; Object.defineProperty(o, 'x', { value: 1, writable: false }); o.x = 2;",
    );
}

// ─── D2: configurable:false delete enforcement ──────────────────────────

#[test]
fn non_configurable_delete_strict_throws() {
    eval_throws(
        "'use strict'; var o = {}; Object.defineProperty(o, 'x', { value: 1, configurable: false }); delete o.x;",
    );
}

#[test]
fn configurable_delete_succeeds() {
    assert!(eval_bool(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, configurable: true }); delete o.x;"
    ));
}

// ─── D3: prototype shadowing ────────────────────────────────────────────

#[test]
fn prototype_writable_false_blocks_own_strict() {
    eval_throws(
        "'use strict'; var proto = {}; Object.defineProperty(proto, 'x', { value: 1, writable: false }); \
         var o = Object.create(proto); o.x = 2;",
    );
}

#[test]
fn prototype_accessor_no_setter_blocks_strict() {
    eval_throws(
        "'use strict'; var proto = {}; \
         Object.defineProperty(proto, 'x', { get: function() { return 1; } }); \
         var o = Object.create(proto); o.x = 2;",
    );
}

#[test]
fn prototype_setter_invoked() {
    assert_eq!(
        eval_number(
            "var called = 0; var proto = {}; \
             Object.defineProperty(proto, 'x', { set: function(v) { called = v; } }); \
             var o = Object.create(proto); o.x = 42; called;"
        ),
        42.0
    );
}

// ─── D4: Object.defineProperty attribute constraints ────────────────────

#[test]
fn define_property_non_configurable_reject_configurable_change() {
    eval_throws(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, configurable: false }); \
         Object.defineProperty(o, 'x', { configurable: true });",
    );
}

#[test]
fn define_property_non_configurable_reject_enumerable_change() {
    eval_throws(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, configurable: false, enumerable: false }); \
         Object.defineProperty(o, 'x', { enumerable: true });",
    );
}

#[test]
fn define_property_non_configurable_reject_data_to_accessor() {
    eval_throws(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, configurable: false }); \
         Object.defineProperty(o, 'x', { get: function() { return 2; } });",
    );
}

#[test]
fn define_property_non_configurable_non_writable_reject_value_change() {
    eval_throws(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, writable: false, configurable: false }); \
         Object.defineProperty(o, 'x', { value: 2 });",
    );
}

#[test]
fn define_property_non_configurable_non_writable_reject_writable_true() {
    eval_throws(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, writable: false, configurable: false }); \
         Object.defineProperty(o, 'x', { writable: true });",
    );
}

#[test]
fn define_property_non_configurable_same_value_ok() {
    // Redefining with same value on non-configurable, non-writable is allowed.
    assert_eq!(
        eval_number(
            "var o = {}; Object.defineProperty(o, 'x', { value: 42, writable: false, configurable: false }); \
             Object.defineProperty(o, 'x', { value: 42 }); o.x;"
        ),
        42.0
    );
}

#[test]
fn define_property_non_configurable_accessor_reject_getter_change() {
    eval_throws(
        "var g1 = function() { return 1; }; var g2 = function() { return 2; }; \
         var o = {}; Object.defineProperty(o, 'x', { get: g1, configurable: false }); \
         Object.defineProperty(o, 'x', { get: g2 });",
    );
}

#[test]
fn define_property_non_configurable_accessor_same_getter_ok() {
    assert_eq!(
        eval_number(
            "var g = function() { return 7; }; \
             var o = {}; Object.defineProperty(o, 'x', { get: g, configurable: false }); \
             Object.defineProperty(o, 'x', { get: g }); o.x;"
        ),
        7.0
    );
}

// ─── OrdinarySet ordering: own property takes precedence over prototype ──

#[test]
fn own_property_takes_precedence_over_inherited_setter() {
    assert_eq!(
        eval_number(
            "var proto = {}; var called = 0; \
             Object.defineProperty(proto, 'x', { set: function(v) { called = v; } }); \
             var o = Object.create(proto); \
             Object.defineProperty(o, 'x', { value: 1, writable: true, configurable: true }); \
             o.x = 42; o.x;"
        ),
        42.0
    );
}

#[test]
fn own_property_takes_precedence_over_inherited_writable_false() {
    assert_eq!(
        eval_number(
            "var proto = {}; Object.defineProperty(proto, 'x', { value: 1, writable: false }); \
             var o = Object.create(proto); \
             Object.defineProperty(o, 'x', { value: 10, writable: true, configurable: true }); \
             o.x = 42; o.x;"
        ),
        42.0
    );
}

// ─── PR4: VM single dispatcher ─────────────────────────────────────────

#[test]
fn deep_call_chain_no_stack_overflow() {
    // 1000-deep JS→JS call chain. With recursive run() this would consume
    // ~1000 Rust stack frames; with single dispatcher it uses 0 extra frames.
    assert_eq!(
        eval_number(
            "function countdown(n) { if (n <= 0) return 0; return countdown(n - 1); } \
             countdown(1000);"
        ),
        0.0
    );
}

#[test]
fn deep_call_chain_return_value() {
    assert_eq!(
        eval_number("function sum(n) { if (n <= 0) return 0; return n + sum(n - 1); } sum(100);"),
        5050.0
    );
}

#[test]
fn deep_method_call_chain() {
    assert_eq!(
        eval_number(
            "var obj = { count: function(n) { if (n <= 0) return 0; return obj.count(n - 1); } }; \
             obj.count(500);"
        ),
        0.0
    );
}

#[test]
fn constructor_in_single_dispatcher() {
    assert_eq!(
        eval_number(
            "function Foo(x) { this.val = x; } \
             var f = new Foo(42); f.val;"
        ),
        42.0
    );
}

#[test]
fn constructor_returning_object() {
    assert_eq!(
        eval_number(
            "function Foo() { return { val: 99 }; } \
             var f = new Foo(); f.val;"
        ),
        99.0
    );
}

#[test]
fn constructor_returning_primitive_uses_instance() {
    assert_eq!(
        eval_number(
            "function Foo() { this.val = 7; return 42; } \
             var f = new Foo(); f.val;"
        ),
        7.0
    );
}

#[test]
fn nested_constructor_calls() {
    assert_eq!(
        eval_number(
            "function Inner(x) { this.x = x; } \
             function Outer(y) { this.inner = new Inner(y * 2); } \
             var o = new Outer(5); o.inner.x;"
        ),
        10.0
    );
}

#[test]
fn closure_across_single_dispatcher_frames() {
    assert_eq!(
        eval_number(
            "function make() { var x = 10; return function() { return x + 5; }; } \
             var f = make(); f();"
        ),
        15.0
    );
}

#[test]
fn exception_across_inline_frames() {
    assert_eq!(
        eval_number(
            "function inner() { throw 42; } \
             function outer() { try { inner(); } catch(e) { return e; } } \
             outer();"
        ),
        42.0
    );
}

#[test]
fn exception_unwinds_multiple_inline_frames() {
    assert_eq!(
        eval_number(
            "function a() { throw 1; } \
             function b() { return a(); } \
             function c() { try { return b(); } catch(e) { return e + 10; } } \
             c();"
        ),
        11.0
    );
}

#[test]
fn mutual_recursion_single_dispatcher() {
    assert!(eval_bool(
        "function isEven(n) { if (n === 0) return true; return isOdd(n - 1); } \
         function isOdd(n) { if (n === 0) return false; return isEven(n - 1); } \
         isEven(100);"
    ));
}

#[test]
fn native_reentrant_during_inline_call() {
    // Object.values invokes a getter (native → JS re-entrant call) while
    // the top-level call chain uses the single dispatcher.
    assert_eq!(
        eval_number(
            "function outer() { \
                 var o = {}; \
                 Object.defineProperty(o, 'x', { get: function() { return 77; }, enumerable: true }); \
                 return Object.values(o)[0]; \
             } \
             outer();"
        ),
        77.0
    );
}

#[test]
fn new_arrow_function_throws() {
    eval_throws("var f = () => {}; new f();");
}
