//! Unit tests for `value.rs` types (JsValue properties, Property
//! constructors, StringId equality).  Extracted from the bottom of
//! `value.rs` so that file stays under the 1000-line project convention.

use super::super::shape;
use super::super::value::{JsValue, Property, PropertyStorage, StringId};

#[test]
fn js_value_is_copy() {
    let v = JsValue::Number(42.0);
    let v2 = v; // Copy
    assert_eq!(v, v2);
}

#[test]
fn js_value_size() {
    // JsValue should be at most 16 bytes (tag + f64).
    assert!(std::mem::size_of::<JsValue>() <= 16);
}

#[test]
fn js_value_nan_inequality() {
    let nan = JsValue::Number(f64::NAN);
    assert_ne!(nan, nan); // NaN !== NaN
}

#[test]
fn js_value_zero_equality() {
    let pos = JsValue::Number(0.0);
    let neg = JsValue::Number(-0.0);
    assert_eq!(pos, neg); // +0 === -0
}

#[test]
fn js_value_nullish() {
    assert!(JsValue::Undefined.is_nullish());
    assert!(JsValue::Null.is_nullish());
    assert!(!JsValue::Boolean(false).is_nullish());
    assert!(!JsValue::Number(0.0).is_nullish());
}

#[test]
fn js_value_primitive_falsy() {
    assert!(JsValue::Undefined.is_primitive_falsy());
    assert!(JsValue::Null.is_primitive_falsy());
    assert!(JsValue::Boolean(false).is_primitive_falsy());
    assert!(JsValue::Number(0.0).is_primitive_falsy());
    assert!(JsValue::Number(f64::NAN).is_primitive_falsy());
    assert!(!JsValue::Boolean(true).is_primitive_falsy());
    assert!(!JsValue::Number(1.0).is_primitive_falsy());
    // String/Object falsiness requires Vm access (empty string check).
    assert!(!JsValue::String(StringId(0)).is_primitive_falsy());
    // Arbitrary ObjectId is OK for the falsiness check (no deref needed).
    assert!(!JsValue::Object(super::super::value::ObjectId(0)).is_primitive_falsy());
    let _ = PropertyStorage::shaped(shape::ROOT_SHAPE); // silence unused
}

#[test]
fn string_id_equality() {
    assert_eq!(StringId(0), StringId(0));
    assert_ne!(StringId(0), StringId(1));
}

#[test]
fn property_constructors() {
    let p = Property::data(JsValue::Number(42.0));
    assert!(p.writable && p.enumerable && p.configurable);

    let p = Property::builtin(JsValue::Undefined);
    assert!(!p.writable && !p.enumerable && !p.configurable);

    let p = Property::method(JsValue::Undefined);
    assert!(p.writable && !p.enumerable && p.configurable);
}
