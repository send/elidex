//! JavaScript value representation for cross-engine interop.

use std::fmt;

/// A JavaScript value exchanged between the script engine and DOM bindings.
///
/// This enum mirrors the fundamental JS value types. Object references are
/// represented as opaque `u64` handles managed by `IdentityMap`
/// (defined in the `elidex-script-session` crate).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum JsValue {
    /// The `undefined` value.
    #[default]
    Undefined,
    /// The `null` value.
    Null,
    /// A boolean value.
    Bool(bool),
    /// A 64-bit floating-point number (IEEE 754).
    ///
    /// **Note:** `PartialEq` follows IEEE 754 semantics — `NaN != NaN`.
    Number(f64),
    /// A UTF-8 string.
    String(String),
    /// An opaque reference to a JS-side object.
    ObjectRef(u64),
}

impl fmt::Display for JsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undefined => f.write_str("undefined"),
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
            Self::ObjectRef(id) => write!(f, "[object #{id}]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction() {
        let _ = JsValue::Undefined;
        let _ = JsValue::Null;
        let _ = JsValue::Bool(true);
        let _ = JsValue::Number(3.125);
        let _ = JsValue::String("hello".into());
        let _ = JsValue::ObjectRef(42);
    }

    #[test]
    fn equality() {
        assert_eq!(JsValue::Null, JsValue::Null);
        assert_eq!(JsValue::Bool(true), JsValue::Bool(true));
        assert_ne!(JsValue::Bool(true), JsValue::Bool(false));
        assert_eq!(JsValue::Number(1.0), JsValue::Number(1.0));
        assert_eq!(JsValue::String("a".into()), JsValue::String("a".into()));
        assert_ne!(JsValue::Undefined, JsValue::Null);
        assert_eq!(JsValue::ObjectRef(1), JsValue::ObjectRef(1));
        assert_ne!(JsValue::ObjectRef(1), JsValue::ObjectRef(2));
    }

    #[test]
    fn display() {
        assert_eq!(JsValue::Undefined.to_string(), "undefined");
        assert_eq!(JsValue::Null.to_string(), "null");
        assert_eq!(JsValue::Bool(true).to_string(), "true");
        assert_eq!(JsValue::Number(42.0).to_string(), "42");
        assert_eq!(JsValue::String("hi".into()).to_string(), "hi");
        assert_eq!(JsValue::ObjectRef(7).to_string(), "[object #7]");
    }

    #[test]
    fn default_is_undefined() {
        assert_eq!(JsValue::default(), JsValue::Undefined);
    }
}
