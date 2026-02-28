//! Bidirectional conversion between elidex `JsValue` and boa `JsValue`.

use boa_engine::js_string;

/// Convert an elidex `JsValue` to a boa `JsValue`.
///
/// # `ObjectRef` handling
///
/// `ObjectRef` values are converted to plain `f64` numbers, which means a
/// round-trip through `to_boa` → `from_boa` loses the `ObjectRef` type.
/// This is acceptable because `ObjectRef` values should be resolved to
/// proper boa element wrappers via [`super::globals::element::resolve_object_ref`],
/// not through this generic conversion.
pub fn to_boa(value: &elidex_plugin::JsValue) -> boa_engine::JsValue {
    match value {
        elidex_plugin::JsValue::Null => boa_engine::JsValue::null(),
        elidex_plugin::JsValue::Bool(b) => boa_engine::JsValue::from(*b),
        elidex_plugin::JsValue::Number(n) => boa_engine::JsValue::from(*n),
        elidex_plugin::JsValue::String(s) => boa_engine::JsValue::from(js_string!(s.as_str())),
        elidex_plugin::JsValue::ObjectRef(id) => {
            // ObjectRef is stored as a number — callers that need element wrappers
            // should use resolve_object_ref() instead of this function.
            boa_engine::JsValue::from(*id as f64)
        }
        _ => boa_engine::JsValue::undefined(),
    }
}

/// Convert a boa `JsValue` to an elidex `JsValue`.
///
/// Only primitive types are converted. Objects are mapped to `Undefined`
/// because boa objects cannot be meaningfully represented as elidex values.
/// Use the bridge for object reference resolution.
pub fn from_boa(
    value: &boa_engine::JsValue,
    ctx: &mut boa_engine::Context,
) -> elidex_plugin::JsValue {
    if value.is_undefined() {
        return elidex_plugin::JsValue::Undefined;
    }
    if value.is_null() {
        return elidex_plugin::JsValue::Null;
    }
    if let Some(b) = value.as_boolean() {
        return elidex_plugin::JsValue::Bool(b);
    }
    if let Some(n) = value.as_number() {
        return elidex_plugin::JsValue::Number(n);
    }
    if value.is_string() {
        if let Ok(s) = value.to_string(ctx) {
            return elidex_plugin::JsValue::String(s.to_std_string_escaped());
        }
    }
    // Objects and other types → Undefined for now.
    elidex_plugin::JsValue::Undefined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_undefined() {
        let mut ctx = boa_engine::Context::default();
        let original = elidex_plugin::JsValue::Undefined;
        let boa = to_boa(&original);
        let back = from_boa(&boa, &mut ctx);
        assert_eq!(back, original);
    }

    #[test]
    fn roundtrip_null() {
        let mut ctx = boa_engine::Context::default();
        let original = elidex_plugin::JsValue::Null;
        let boa = to_boa(&original);
        let back = from_boa(&boa, &mut ctx);
        assert_eq!(back, original);
    }

    #[test]
    fn roundtrip_bool() {
        let mut ctx = boa_engine::Context::default();
        for &b in &[true, false] {
            let original = elidex_plugin::JsValue::Bool(b);
            let boa = to_boa(&original);
            let back = from_boa(&boa, &mut ctx);
            assert_eq!(back, original);
        }
    }

    #[test]
    fn roundtrip_number() {
        let mut ctx = boa_engine::Context::default();
        let original = elidex_plugin::JsValue::Number(42.5);
        let boa = to_boa(&original);
        let back = from_boa(&boa, &mut ctx);
        assert_eq!(back, original);
    }

    #[test]
    fn roundtrip_string() {
        let mut ctx = boa_engine::Context::default();
        let original = elidex_plugin::JsValue::String("hello world".into());
        let boa = to_boa(&original);
        let back = from_boa(&boa, &mut ctx);
        assert_eq!(back, original);
    }

    #[test]
    fn object_ref_converts_to_number() {
        let original = elidex_plugin::JsValue::ObjectRef(7);
        let boa = to_boa(&original);
        assert_eq!(boa.as_number(), Some(7.0));
    }
}
