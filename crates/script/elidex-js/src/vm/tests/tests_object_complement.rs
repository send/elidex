//! Tests for Object.prototype methods and Object static methods (ES2020 §19.1).

use super::{eval_bool, eval_number, eval_string, eval_throws};

// ═══════════════════════════════════════════════════════════════════════════
// Object.prototype.hasOwnProperty (§19.1.3.2)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn has_own_property_own_returns_true() {
    assert!(eval_bool("var o = {a: 1}; o.hasOwnProperty('a');"));
}

#[test]
fn has_own_property_inherited_returns_false() {
    assert!(!eval_bool(
        "var p = {x: 1}; var c = Object.create(p); c.hasOwnProperty('x');"
    ));
}

#[test]
fn has_own_property_nonexistent_returns_false() {
    assert!(!eval_bool("var o = {a: 1}; o.hasOwnProperty('b');"));
}

#[test]
fn has_own_property_symbol_key() {
    assert!(eval_bool(
        "var s = Symbol('k'); var o = {}; o[s] = 42; o.hasOwnProperty(s);"
    ));
}

#[test]
fn has_own_property_missing_symbol_returns_false() {
    assert!(!eval_bool(
        "var s = Symbol('k'); var o = {}; o.hasOwnProperty(s);"
    ));
}

#[test]
fn has_own_property_numeric_string() {
    assert!(eval_bool("var o = {0: 'zero'}; o.hasOwnProperty('0');"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.prototype.valueOf (§19.1.3.7)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn value_of_returns_object_identity() {
    // valueOf returns the object itself; verify by mutating through the reference.
    assert_eq!(
        eval_number("var o = {x: 1}; var v = o.valueOf(); v.x = 42; o.x;"),
        42.0
    );
}

#[test]
fn value_of_same_reference() {
    assert!(eval_bool("var o = {}; o.valueOf() === o;"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.prototype.isPrototypeOf (§19.1.3.4)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn is_prototype_of_array() {
    // Verify isPrototypeOf via a regular prototype chain.
    assert!(eval_bool(
        "var p = {x: 1}; var c = Object.create(p); p.isPrototypeOf(c);"
    ));
}

#[test]
fn is_prototype_of_custom_chain() {
    assert!(eval_bool(
        "var p = {}; var c = Object.create(p); p.isPrototypeOf(c);"
    ));
}

#[test]
fn is_prototype_of_not_in_chain() {
    assert!(!eval_bool("var a = {}; var b = {}; a.isPrototypeOf(b);"));
}

#[test]
fn is_prototype_of_multi_level() {
    assert!(eval_bool(
        "var gp = {}; var p = Object.create(gp); var c = Object.create(p); gp.isPrototypeOf(c);"
    ));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.prototype.propertyIsEnumerable (§19.1.3.5)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn property_is_enumerable_data() {
    assert!(eval_bool("var o = {a: 1}; o.propertyIsEnumerable('a');"));
}

#[test]
fn property_is_enumerable_non_enumerable() {
    assert!(!eval_bool(
        "var o = {}; Object.defineProperty(o, 'x', {value: 1, enumerable: false}); o.propertyIsEnumerable('x');"
    ));
}

#[test]
fn property_is_enumerable_inherited() {
    // Inherited properties are not own — should return false.
    assert!(!eval_bool(
        "var p = {a: 1}; var c = Object.create(p); c.propertyIsEnumerable('a');"
    ));
}

#[test]
fn property_is_enumerable_nonexistent() {
    assert!(!eval_bool("var o = {}; o.propertyIsEnumerable('x');"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.entries (§19.1.2.5)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn entries_basic() {
    assert_eq!(eval_number("Object.entries({a: 1, b: 2}).length;"), 2.0);
}

#[test]
fn entries_key_value_pairs() {
    assert_eq!(
        eval_string("var e = Object.entries({x: 42}); e[0][0];"),
        "x"
    );
}

#[test]
fn entries_value() {
    assert_eq!(
        eval_number("var e = Object.entries({x: 42}); e[0][1];"),
        42.0
    );
}

#[test]
fn entries_only_own_enumerable() {
    assert_eq!(
        eval_number("var o = Object.create({inherited: 1}); o.own = 2; Object.entries(o).length;"),
        1.0
    );
}

#[test]
fn entries_skips_non_enumerable() {
    assert_eq!(
        eval_number(
            "var o = {a: 1}; Object.defineProperty(o, 'b', {value: 2, enumerable: false}); Object.entries(o).length;"
        ),
        1.0
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.is (§19.1.2.10)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn object_is_nan_nan() {
    assert!(eval_bool("Object.is(NaN, NaN);"));
}

#[test]
fn object_is_pos_zero_neg_zero() {
    assert!(!eval_bool("Object.is(+0, -0);"));
}

#[test]
fn object_is_same_object() {
    assert!(eval_bool("var o = {}; Object.is(o, o);"));
}

#[test]
fn object_is_different_values() {
    assert!(!eval_bool("Object.is(1, 2);"));
}

#[test]
fn object_is_same_string() {
    assert!(eval_bool("Object.is('abc', 'abc');"));
}

#[test]
fn object_is_null_null() {
    assert!(eval_bool("Object.is(null, null);"));
}

#[test]
fn object_is_undefined_undefined() {
    assert!(eval_bool("Object.is(undefined, undefined);"));
}

#[test]
fn object_is_null_undefined() {
    assert!(!eval_bool("Object.is(null, undefined);"));
}

#[test]
fn object_is_different_objects() {
    assert!(!eval_bool("Object.is({}, {});"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.getPrototypeOf / setPrototypeOf
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn get_prototype_of_plain_object() {
    // Plain objects have a non-null prototype.
    assert!(!eval_bool("Object.getPrototypeOf({}) === null;"));
}

#[test]
fn set_prototype_of_changes_chain() {
    assert_eq!(
        eval_number("var p = {x: 42}; var o = {}; Object.setPrototypeOf(o, p); o.x;"),
        42.0
    );
}

#[test]
fn get_prototype_of_null_proto() {
    assert!(eval_bool(
        "Object.getPrototypeOf(Object.create(null)) === null;"
    ));
}

#[test]
fn set_prototype_of_cyclic_throws() {
    eval_throws("var a = {}; var b = Object.create(a); Object.setPrototypeOf(a, b);");
}

#[test]
fn set_prototype_of_self_throws() {
    eval_throws("var o = {}; Object.setPrototypeOf(o, o);");
}

#[test]
fn set_prototype_of_returns_object() {
    assert_eq!(
        eval_number("var o = {v: 7}; var r = Object.setPrototypeOf(o, null); r.v;"),
        7.0
    );
}

#[test]
fn get_prototype_of_array() {
    assert!(eval_bool("Object.getPrototypeOf([]) === Array.prototype;"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.getOwnPropertyDescriptor (§19.1.2.6)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn get_own_property_descriptor_data() {
    assert!(eval_bool(
        "var d = Object.getOwnPropertyDescriptor({a: 1}, 'a'); d.value === 1;"
    ));
}

#[test]
fn get_own_property_descriptor_writable() {
    assert!(eval_bool(
        "var d = Object.getOwnPropertyDescriptor({a: 1}, 'a'); d.writable === true;"
    ));
}

#[test]
fn get_own_property_descriptor_enumerable() {
    assert!(eval_bool(
        "var d = Object.getOwnPropertyDescriptor({a: 1}, 'a'); d.enumerable === true;"
    ));
}

#[test]
fn get_own_property_descriptor_configurable() {
    assert!(eval_bool(
        "var d = Object.getOwnPropertyDescriptor({a: 1}, 'a'); d.configurable === true;"
    ));
}

#[test]
fn get_own_property_descriptor_nonexistent() {
    assert!(eval_bool(
        "Object.getOwnPropertyDescriptor({}, 'x') === undefined;"
    ));
}

#[test]
fn get_own_property_descriptor_define_property() {
    assert!(!eval_bool(
        "var o = {}; Object.defineProperty(o, 'x', {value: 1, writable: false, enumerable: false, configurable: false}); \
         var d = Object.getOwnPropertyDescriptor(o, 'x'); d.writable;"
    ));
}

#[test]
fn get_own_property_descriptor_accessor() {
    assert_eq!(
        eval_string(
            "var o = {}; Object.defineProperty(o, 'x', {get: function(){return 1;}, enumerable: true, configurable: true}); \
             var d = Object.getOwnPropertyDescriptor(o, 'x'); typeof d.get;"
        ),
        "function"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.getOwnPropertyNames (§19.1.2.8)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn get_own_property_names_includes_non_enumerable() {
    assert_eq!(
        eval_number(
            "var o = {a: 1}; Object.defineProperty(o, 'b', {value: 2, enumerable: false}); \
             Object.getOwnPropertyNames(o).length;"
        ),
        2.0
    );
}

#[test]
fn get_own_property_names_excludes_symbols() {
    assert_eq!(
        eval_number(
            "var s = Symbol('k'); var o = {a: 1}; o[s] = 2; Object.getOwnPropertyNames(o).length;"
        ),
        1.0
    );
}

#[test]
fn get_own_property_names_basic() {
    assert_eq!(
        eval_number("Object.getOwnPropertyNames({x: 1, y: 2}).length;"),
        2.0
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.freeze (§19.1.2.6)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn freeze_cannot_add_props() {
    // In sloppy mode, adding to frozen object silently fails.
    assert!(eval_bool(
        "var o = {a: 1}; Object.freeze(o); o.b = 2; o.b === undefined;"
    ));
}

#[test]
fn freeze_cannot_write_existing() {
    assert_eq!(
        eval_number("var o = {a: 1}; Object.freeze(o); o.a = 99; o.a;"),
        1.0
    );
}

#[test]
fn is_frozen_after_freeze() {
    assert!(eval_bool(
        "var o = {}; Object.freeze(o); Object.isFrozen(o);"
    ));
}

#[test]
fn is_frozen_false_for_normal() {
    assert!(!eval_bool("Object.isFrozen({a: 1});"));
}

#[test]
fn is_frozen_empty_non_extensible() {
    // An empty non-extensible object is vacuously frozen.
    assert!(eval_bool(
        "var o = {}; Object.preventExtensions(o); Object.isFrozen(o);"
    ));
}

#[test]
fn freeze_returns_same_object() {
    assert!(eval_bool("var o = {}; Object.freeze(o) === o;"));
}

#[test]
fn freeze_nested_not_deep() {
    // Freeze is shallow — nested objects are not frozen.
    assert_eq!(
        eval_number("var o = {inner: {x: 1}}; Object.freeze(o); o.inner.x = 42; o.inner.x;"),
        42.0
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.seal (§19.1.2.20)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn seal_can_write_existing() {
    assert_eq!(
        eval_number("var o = {a: 1}; Object.seal(o); o.a = 42; o.a;"),
        42.0
    );
}

#[test]
fn seal_cannot_add_new() {
    assert!(eval_bool(
        "var o = {a: 1}; Object.seal(o); o.b = 2; o.b === undefined;"
    ));
}

#[test]
fn is_sealed_after_seal() {
    assert!(eval_bool(
        "var o = {a: 1}; Object.seal(o); Object.isSealed(o);"
    ));
}

#[test]
fn is_sealed_false_for_normal() {
    assert!(!eval_bool("Object.isSealed({a: 1});"));
}

#[test]
fn seal_returns_same_object() {
    assert!(eval_bool("var o = {}; Object.seal(o) === o;"));
}

#[test]
fn sealed_cannot_delete() {
    assert!(eval_bool(
        "var o = {a: 1}; Object.seal(o); delete o.a; 'a' in o;"
    ));
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.preventExtensions / isExtensible (§19.1.2.18 / §19.1.2.11)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn prevent_extensions_makes_not_extensible() {
    assert!(!eval_bool(
        "var o = {}; Object.preventExtensions(o); Object.isExtensible(o);"
    ));
}

#[test]
fn cannot_add_after_prevent_extensions() {
    assert!(eval_bool(
        "var o = {}; Object.preventExtensions(o); o.x = 1; o.x === undefined;"
    ));
}

#[test]
fn is_extensible_normal_object() {
    assert!(eval_bool("Object.isExtensible({});"));
}

#[test]
fn prevent_extensions_existing_props_writable() {
    assert_eq!(
        eval_number("var o = {a: 1}; Object.preventExtensions(o); o.a = 42; o.a;"),
        42.0
    );
}

#[test]
fn prevent_extensions_can_delete() {
    assert!(!eval_bool(
        "var o = {a: 1}; Object.preventExtensions(o); delete o.a; 'a' in o;"
    ));
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn freeze_non_object_returns_value() {
    assert_eq!(eval_number("Object.freeze(42);"), 42.0);
}

#[test]
fn freeze_string_returns_value() {
    assert_eq!(eval_string("Object.freeze('hello');"), "hello");
}

#[test]
fn seal_non_object_returns_value() {
    assert_eq!(eval_number("Object.seal(42);"), 42.0);
}

#[test]
fn is_frozen_non_object_returns_true() {
    assert!(eval_bool("Object.isFrozen(42);"));
}

#[test]
fn is_sealed_non_object_returns_true() {
    assert!(eval_bool("Object.isSealed('hello');"));
}

#[test]
fn is_extensible_non_object_returns_false() {
    assert!(!eval_bool("Object.isExtensible(42);"));
}

#[test]
fn has_own_property_on_object_numeric_key() {
    // Object (not array) with numeric string key.
    assert!(eval_bool(
        "var o = {}; o['0'] = 'zero'; o.hasOwnProperty('0');"
    ));
}

#[test]
fn has_own_property_defined_prop() {
    assert!(eval_bool(
        "var o = {}; Object.defineProperty(o, 'x', {value: 1}); o.hasOwnProperty('x');"
    ));
}

#[test]
fn object_is_true_true() {
    assert!(eval_bool("Object.is(true, true);"));
}

#[test]
fn object_is_false_false() {
    assert!(eval_bool("Object.is(false, false);"));
}

#[test]
fn entries_empty_object() {
    assert_eq!(eval_number("Object.entries({}).length;"), 0.0);
}

#[test]
fn get_own_property_names_empty() {
    assert_eq!(eval_number("Object.getOwnPropertyNames({}).length;"), 0.0);
}

#[test]
fn get_prototype_of_primitive() {
    // §19.1.2.9: getPrototypeOf on a primitive wraps via ToObject.
    // getPrototypeOf(42) should return Number.prototype (not null).
    assert!(eval_bool("Object.getPrototypeOf(42) !== null;"));
    assert!(eval_bool(
        "Object.getPrototypeOf(42) === Object.getPrototypeOf(1);"
    ));
}

#[test]
fn set_prototype_of_non_object_value_returns_it() {
    assert_eq!(eval_number("Object.setPrototypeOf(42, null);"), 42.0);
}

#[test]
fn set_prototype_of_invalid_proto_throws() {
    eval_throws("Object.setPrototypeOf({}, 42);");
}

#[test]
fn frozen_object_property_is_not_configurable() {
    assert!(!eval_bool(
        "var o = {a: 1}; Object.freeze(o); Object.getOwnPropertyDescriptor(o, 'a').configurable;"
    ));
}

#[test]
fn frozen_object_property_is_not_writable() {
    assert!(!eval_bool(
        "var o = {a: 1}; Object.freeze(o); Object.getOwnPropertyDescriptor(o, 'a').writable;"
    ));
}

#[test]
fn sealed_object_property_is_not_configurable() {
    assert!(!eval_bool(
        "var o = {a: 1}; Object.seal(o); Object.getOwnPropertyDescriptor(o, 'a').configurable;"
    ));
}

#[test]
fn sealed_object_property_remains_writable() {
    assert!(eval_bool(
        "var o = {a: 1}; Object.seal(o); Object.getOwnPropertyDescriptor(o, 'a').writable;"
    ));
}

// ═══════════════════════════════════════════════════════════════════════════
// Combined / integration scenarios
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn entries_matches_keys_values() {
    assert!(eval_bool(
        "var o = {a: 1, b: 2}; \
         var e = Object.entries(o); \
         var k = Object.keys(o); \
         var v = Object.values(o); \
         e[0][0] === k[0] && e[0][1] === v[0] && e[1][0] === k[1] && e[1][1] === v[1];"
    ));
}

#[test]
fn freeze_then_seal_still_frozen() {
    assert!(eval_bool(
        "var o = {a: 1}; Object.freeze(o); Object.seal(o); Object.isFrozen(o);"
    ));
}

#[test]
fn prevent_extensions_then_check_sealed_frozen() {
    // An empty non-extensible object is both sealed and frozen (vacuously).
    assert!(eval_bool(
        "var o = {}; Object.preventExtensions(o); Object.isSealed(o) && Object.isFrozen(o);"
    ));
}

#[test]
fn has_own_property_after_delete() {
    assert!(!eval_bool(
        "var o = {a: 1}; delete o.a; o.hasOwnProperty('a');"
    ));
}

#[test]
fn property_is_enumerable_after_define_enumerable_true() {
    assert!(eval_bool(
        "var o = {}; Object.defineProperty(o, 'x', {value: 1, enumerable: true}); o.propertyIsEnumerable('x');"
    ));
}

#[test]
fn get_own_property_descriptor_after_seal() {
    // After seal, configurable becomes false but value/writable/enumerable stay.
    assert!(eval_bool(
        "var o = {a: 1}; Object.seal(o); \
         var d = Object.getOwnPropertyDescriptor(o, 'a'); \
         d.configurable === false && d.writable === true && d.enumerable === true && d.value === 1;"
    ));
}

#[test]
fn object_create_null_has_no_prototype_methods() {
    // Object.create(null) has no toString/hasOwnProperty etc.
    assert_eq!(
        eval_string("var o = Object.create(null); typeof o.hasOwnProperty;"),
        "undefined"
    );
}

#[test]
fn set_prototype_of_null_removes_prototype() {
    assert!(eval_bool(
        "var o = {}; Object.setPrototypeOf(o, null); Object.getPrototypeOf(o) === null;"
    ));
}

#[test]
fn set_prototype_of_null_loses_methods() {
    assert_eq!(
        eval_string("var o = {}; Object.setPrototypeOf(o, null); typeof o.hasOwnProperty;"),
        "undefined"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.fromEntries (ES2019)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn from_entries_basic() {
    assert_eq!(
        eval_number("var o = Object.fromEntries([['a', 1], ['b', 2]]); o.a + o.b;"),
        3.0
    );
}

#[test]
fn from_entries_empty() {
    assert_eq!(
        eval_number("Object.keys(Object.fromEntries([])).length;"),
        0.0
    );
}

#[test]
fn from_entries_overwrites_duplicate() {
    assert_eq!(
        eval_number("Object.fromEntries([['a', 1], ['a', 2]]).a;"),
        2.0
    );
}

#[test]
fn from_entries_roundtrip() {
    assert!(eval_bool(
        "var o = {x: 10, y: 20}; var o2 = Object.fromEntries(Object.entries(o)); o2.x === 10 && o2.y === 20;"
    ));
}

#[test]
fn from_entries_non_iterable_throws() {
    eval_throws("Object.fromEntries(42);");
}

// ═══════════════════════════════════════════════════════════════════════════
// Object.prototype.toLocaleString (§19.1.3.5)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn to_locale_string_exists() {
    assert_eq!(eval_string("typeof ({}).toLocaleString;"), "function");
}

#[test]
fn to_locale_string_returns_string() {
    assert_eq!(eval_string("({}).toLocaleString();"), "[object Object]");
}

// ═══════════════════════════════════════════════════════════════════════════
// P2 Tier 6: P1 deferred fixes
// ═══════════════════════════════════════════════════════════════════════════

// -- ToObject coercion in Object static methods -------------------------------

#[test]
fn object_keys_null_throws() {
    eval_throws("Object.keys(null);");
}

#[test]
fn object_keys_undefined_throws() {
    eval_throws("Object.keys(undefined);");
}

#[test]
fn object_keys_number_returns_empty() {
    // ToObject(42) → NumberWrapper with no own enumerable properties
    assert_eq!(eval_number("Object.keys(42).length;"), 0.0);
}

// -- setPrototypeOf RequireObjectCoercible ------------------------------------

#[test]
fn set_prototype_of_null_throws() {
    eval_throws("Object.setPrototypeOf(null, {});");
}

#[test]
fn set_prototype_of_undefined_throws() {
    eval_throws("Object.setPrototypeOf(undefined, {});");
}

#[test]
fn set_prototype_of_primitive_noop() {
    // Non-null/undefined primitives are returned unchanged
    assert_eq!(eval_number("Object.setPrototypeOf(42, {});"), 42.0);
}

// -- Function.prototype is callable -------------------------------------------

#[test]
fn function_prototype_callable() {
    // Function.prototype should be callable — access via a function's __proto__
    assert!(eval_bool(
        "var f = function(){}; Object.getPrototypeOf(f)() === undefined;"
    ));
}

#[test]
fn function_prototype_typeof() {
    assert!(eval_bool(
        "var f = function(){}; typeof Object.getPrototypeOf(f) === 'function';"
    ));
}
