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

#[test]
fn writable_false_sloppy_silent() {
    assert_eq!(
        eval_number(
            "var o = {}; Object.defineProperty(o, 'x', { value: 1, writable: false }); o.x = 2; o.x;"
        ),
        1.0
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
fn non_configurable_delete_sloppy_returns_false() {
    assert!(!eval_bool(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1, configurable: false }); delete o.x;"
    ));
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
fn prototype_writable_false_blocks_own_sloppy() {
    assert_eq!(
        eval_number(
            "var proto = {}; Object.defineProperty(proto, 'x', { value: 1, writable: false }); \
             var o = Object.create(proto); o.x = 2; o.x;"
        ),
        1.0 // Inherited value, own assignment silently failed
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
fn prototype_accessor_no_setter_blocks_sloppy() {
    assert_eq!(
        eval_number(
            "var proto = {}; \
             Object.defineProperty(proto, 'x', { get: function() { return 1; } }); \
             var o = Object.create(proto); o.x = 2; o.x;"
        ),
        1.0 // Getter returns 1, assignment silently failed
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
