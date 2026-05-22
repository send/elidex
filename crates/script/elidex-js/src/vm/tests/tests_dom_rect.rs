//! Tests for `DOMRectReadOnly` + `DOMRect` (W3C Geometry Interfaces
//! Module Level 1 §3) — constructor coercion, NaN-safe edge derivation,
//! read-only vs read-write brand, subclass prototype chain, `fromRect`,
//! and `toJSON`.

#![cfg(feature = "engine")]

use super::super::Vm;
use super::{eval_bool, eval_number};

/// Run `source`, catch any thrown exception, and return its `.name`
/// (empty string if nothing was thrown).  Mirrors the DOMException
/// brand-check test harness.
fn caught_name(source: &str) -> String {
    let mut vm = Vm::new();
    let wrapped = format!(
        "var caught = '';\
         try {{ {source}; }} catch (e) {{ caught = e.name; }}\
         caught;"
    );
    match vm.eval(&wrapped).unwrap() {
        super::super::value::JsValue::String(id) => vm.get_string(id).clone(),
        other => panic!("expected string, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Constructor coercion (Geometry §3, unrestricted double args)
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_constructor_stores_coordinates() {
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).x"), 1.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).y"), 2.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).width"), 3.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).height"), 4.0);
}

#[test]
fn dom_rect_readonly_defaults_to_zero() {
    assert_eq!(eval_number("new DOMRectReadOnly().x"), 0.0);
    assert_eq!(eval_number("new DOMRectReadOnly().y"), 0.0);
    assert_eq!(eval_number("new DOMRectReadOnly().width"), 0.0);
    assert_eq!(eval_number("new DOMRectReadOnly().height"), 0.0);
}

#[test]
fn dom_rect_unrestricted_double_allows_nan_and_infinity() {
    // WebIDL `unrestricted double`: NaN / ±Infinity pass through, no
    // TypeError on construction.
    assert!(eval_bool("Number.isNaN(new DOMRect(NaN).x)"));
    assert!(eval_bool("new DOMRect(Infinity).x === Infinity"));
}

// ---------------------------------------------------------------------------
// Computed edges (NaN-safe min/max — negative dimensions swap)
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_edges_positive_dimensions() {
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).left"), 1.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).top"), 2.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).right"), 4.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).bottom"), 6.0);
}

#[test]
fn dom_rect_edges_negative_dimensions_swap() {
    // x=10, width=-5 → left=min(10,5)=5, right=max(10,5)=10.
    // y=10, height=-5 → top=min(10,5)=5, bottom=max(10,5)=10.
    assert_eq!(eval_number("new DOMRect(10, 10, -5, -5).left"), 5.0);
    assert_eq!(eval_number("new DOMRect(10, 10, -5, -5).right"), 10.0);
    assert_eq!(eval_number("new DOMRect(10, 10, -5, -5).top"), 5.0);
    assert_eq!(eval_number("new DOMRect(10, 10, -5, -5).bottom"), 10.0);
}

// ---------------------------------------------------------------------------
// Read-write (DOMRect) vs read-only (DOMRectReadOnly) brand
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_setter_reflects_and_recomputes_edges() {
    assert_eq!(
        eval_number("var r = new DOMRect(0, 0, 1, 1); r.width = 9; r.width"),
        9.0
    );
    // right is derived, so it must track the new width.
    assert_eq!(
        eval_number("var r = new DOMRect(0, 0, 1, 1); r.x = 2; r.width = 5; r.right"),
        7.0
    );
}

#[test]
fn dom_rect_setter_coerces_via_to_number() {
    // The setter parameter is an `unrestricted double` → ES ToNumber:
    // `undefined` (or a missing arg) → NaN, `null` → 0, strings parse.
    // This differs from the constructor / DOMRectInit, whose args are
    // optional-with-default-0.
    assert!(eval_bool(
        "var r = new DOMRect(1, 2, 3, 4); r.x = undefined; Number.isNaN(r.x)"
    ));
    assert_eq!(
        eval_number("var r = new DOMRect(1, 2, 3, 4); r.y = null; r.y"),
        0.0
    );
    assert_eq!(
        eval_number("var r = new DOMRect(1, 2, 3, 4); r.width = '5'; r.width"),
        5.0
    );
}

#[test]
fn dom_rect_readonly_assignment_throws() {
    // DOMRectReadOnly.x is a getter-only accessor; this VM rejects a
    // [[Set]] against a setter-less inherited accessor with a TypeError
    // (stricter than the ES sloppy-mode silent no-op, but VM-wide
    // consistent), and the coordinate is left unchanged.
    assert_eq!(
        caught_name("var r = new DOMRectReadOnly(1, 2, 3, 4); r.x = 99"),
        "TypeError"
    );
    assert_eq!(
        eval_number("var r = new DOMRectReadOnly(1, 2, 3, 4); try { r.x = 99; } catch (e) {} r.x"),
        1.0
    );
}

#[test]
fn dom_rect_setter_cross_called_on_readonly_throws() {
    // The DOMRect `x` setter requires a mutable (DOMRect) receiver; a
    // DOMRectReadOnly instance fails the brand check.
    assert_eq!(
        caught_name(
            "var s = Object.getOwnPropertyDescriptor(DOMRect.prototype, 'x').set;\
             s.call(new DOMRectReadOnly(1, 2, 3, 4), 99)"
        ),
        "TypeError"
    );
}

#[test]
fn dom_rect_readonly_has_no_x_setter() {
    assert!(eval_bool(
        "Object.getOwnPropertyDescriptor(DOMRectReadOnly.prototype, 'x').set === undefined"
    ));
}

// ---------------------------------------------------------------------------
// Subclass prototype chain (DOMRect : DOMRectReadOnly)
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_instanceof_both_interfaces() {
    assert!(eval_bool("new DOMRect() instanceof DOMRect"));
    assert!(eval_bool("new DOMRect() instanceof DOMRectReadOnly"));
}

#[test]
fn dom_rect_readonly_is_not_instanceof_dom_rect() {
    assert!(eval_bool("!(new DOMRectReadOnly() instanceof DOMRect)"));
}

#[test]
fn dom_rect_constructor_inherits_from_readonly_constructor() {
    // WebIDL interface inheritance: Object.getPrototypeOf(DOMRect) is
    // DOMRectReadOnly.
    assert!(eval_bool(
        "Object.getPrototypeOf(DOMRect) === DOMRectReadOnly"
    ));
}

#[test]
fn dom_rect_edges_inherited_from_readonly_prototype() {
    // top/right/bottom/left live only on DOMRectReadOnly.prototype; a
    // DOMRect instance reaches them through the chain.
    assert!(eval_bool(
        "!DOMRect.prototype.hasOwnProperty('top') && DOMRectReadOnly.prototype.hasOwnProperty('top')"
    ));
}

// ---------------------------------------------------------------------------
// fromRect static
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_from_rect_reads_init_with_defaults() {
    assert_eq!(eval_number("DOMRect.fromRect({ x: 1, width: 2 }).x"), 1.0);
    assert_eq!(
        eval_number("DOMRect.fromRect({ x: 1, width: 2 }).width"),
        2.0
    );
    // Absent members default to 0.
    assert_eq!(eval_number("DOMRect.fromRect({ x: 1, width: 2 }).y"), 0.0);
    assert_eq!(
        eval_number("DOMRect.fromRect({ x: 1, width: 2 }).height"),
        0.0
    );
}

#[test]
fn dom_rect_from_rect_no_arg_is_all_zero() {
    assert_eq!(eval_number("DOMRect.fromRect().x"), 0.0);
    assert_eq!(eval_number("DOMRect.fromRect().width"), 0.0);
}

#[test]
fn dom_rect_from_rect_preserves_interface_brand() {
    // DOMRect.fromRect → DOMRect (mutable); DOMRectReadOnly.fromRect →
    // DOMRectReadOnly (not a DOMRect).
    assert!(eval_bool("DOMRect.fromRect({}) instanceof DOMRect"));
    assert!(eval_bool(
        "!(DOMRectReadOnly.fromRect({}) instanceof DOMRect)"
    ));
}

#[test]
fn dom_rect_from_rect_non_object_throws() {
    // WebIDL dictionary conversion: a non-object, non-nullish value is
    // a TypeError.
    assert_eq!(caught_name("DOMRect.fromRect(42)"), "TypeError");
}

// ---------------------------------------------------------------------------
// toJSON (Geometry §3)
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_to_json_has_eight_keys() {
    assert_eq!(
        eval_number("Object.keys(new DOMRect(1, 2, 3, 4).toJSON()).length"),
        8.0
    );
}

#[test]
fn dom_rect_to_json_values() {
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).toJSON().x"), 1.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).toJSON().right"), 4.0);
    assert_eq!(eval_number("new DOMRect(1, 2, 3, 4).toJSON().bottom"), 6.0);
}

#[test]
fn dom_rect_json_stringify_roundtrip() {
    // JSON.stringify honours the inherited toJSON.
    assert_eq!(
        eval_number("JSON.parse(JSON.stringify(new DOMRect(1, 2, 3, 4))).width"),
        3.0
    );
}

// ---------------------------------------------------------------------------
// Brand check on getter cross-call (WebIDL §3.2)
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_getter_cross_call_on_alien_throws() {
    assert_eq!(
        caught_name(
            "Object.getOwnPropertyDescriptor(DOMRectReadOnly.prototype, 'x').get.call(null)"
        ),
        "TypeError"
    );
}

// ---------------------------------------------------------------------------
// Constructor requires `new`
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_call_without_new_throws() {
    assert_eq!(caught_name("DOMRect(1, 2, 3, 4)"), "TypeError");
    assert_eq!(caught_name("DOMRectReadOnly()"), "TypeError");
}

// ---------------------------------------------------------------------------
// Value-type: no own enumerable data properties (state is side-table)
// ---------------------------------------------------------------------------

#[test]
fn dom_rect_instance_has_no_own_keys() {
    // x/y/width/height are prototype accessors, not own data props.
    assert_eq!(
        eval_number("Object.keys(new DOMRect(1, 2, 3, 4)).length"),
        0.0
    );
}
