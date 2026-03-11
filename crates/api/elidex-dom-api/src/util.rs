//! Shared argument extraction and validation helpers.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind};

/// Extract a required string argument, returning `TypeError` if missing.
///
/// Non-string primitives are coerced via `ToString` (matching the DOM IDL
/// `DOMString` algorithm): `Null` → `"null"`, `Undefined` → `"undefined"`,
/// `Bool` → `"true"`/`"false"`, `Number` → numeric string.
/// `ObjectRef` values are rejected (internal type that should not reach here).
pub fn require_string_arg(args: &[JsValue], index: usize) -> Result<String, DomApiError> {
    match args.get(index) {
        Some(JsValue::String(s)) => Ok(s.clone()),
        Some(JsValue::ObjectRef(_)) => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a string, not an object reference"),
        }),
        Some(other) => Ok(other.to_string()),
        None => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} is required"),
        }),
    }
}

/// Extract a required object reference argument, returning `TypeError` if missing.
pub fn require_object_ref_arg(args: &[JsValue], index: usize) -> Result<u64, DomApiError> {
    match args.get(index) {
        Some(JsValue::ObjectRef(id)) => Ok(*id),
        _ => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be an object"),
        }),
    }
}

/// Create a `NotFoundError` with the given message.
pub(crate) fn not_found_error(message: &str) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::NotFoundError,
        message: message.into(),
    }
}

/// Get immutable `Attributes` for an entity, returning `NotFoundError` if missing.
pub(crate) fn require_attrs(
    entity: Entity,
    dom: &EcsDom,
) -> Result<hecs::Ref<'_, Attributes>, DomApiError> {
    dom.world()
        .get::<&Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))
}

/// Get mutable `Attributes` for an entity, returning `NotFoundError` if missing.
pub(crate) fn require_attrs_mut(
    entity: Entity,
    dom: &mut EcsDom,
) -> Result<hecs::RefMut<'_, Attributes>, DomApiError> {
    dom.world_mut()
        .get::<&mut Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))
}

/// Escape HTML text content for serialization.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape an HTML attribute value for serialization.
pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_string_valid() {
        let args = vec![JsValue::String("hello".into())];
        assert_eq!(require_string_arg(&args, 0).unwrap(), "hello");
    }

    #[test]
    fn require_string_coerces_number() {
        let args = vec![JsValue::Number(42.0)];
        assert_eq!(require_string_arg(&args, 0).unwrap(), "42");
    }

    #[test]
    fn require_string_missing() {
        let args: Vec<JsValue> = vec![];
        let err = require_string_arg(&args, 0).unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::TypeError);
    }

    #[test]
    fn require_string_rejects_object_ref() {
        let args = vec![JsValue::ObjectRef(7)];
        let err = require_string_arg(&args, 0).unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::TypeError);
    }

    #[test]
    fn require_object_ref_valid() {
        let args = vec![JsValue::ObjectRef(7)];
        assert_eq!(require_object_ref_arg(&args, 0).unwrap(), 7);
    }

    #[test]
    fn require_object_ref_wrong_type() {
        let args = vec![JsValue::String("not an object".into())];
        let err = require_object_ref_arg(&args, 0).unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::TypeError);
    }

    #[test]
    fn escape_html_chars() {
        assert_eq!(escape_html("<div>&</div>"), "&lt;div&gt;&amp;&lt;/div&gt;");
        assert_eq!(escape_html("hello"), "hello");
    }

    #[test]
    fn escape_attr_chars() {
        assert_eq!(escape_attr("a\"b&c"), "a&quot;b&amp;c");
    }
}
