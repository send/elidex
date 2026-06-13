//! Shared argument extraction and validation helpers.

use std::borrow::Cow;

use elidex_ecs::{Attributes, EcsDom, Entity, NodeKind};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind};

/// CSSOM §6.6.1 property-name normalisation: ASCII-lowercase non-custom
/// names; preserve case for custom properties (`--*`) per CSS Variables
/// Level 1 §2. Returns a borrowed `&str` when no allocation is needed
/// (already lowercase / starts with `--`). Used by both `style.*` and
/// `rule.style.*` handler families.
#[must_use]
pub(crate) fn normalize_property_name(name: &str) -> Cow<'_, str> {
    if name.starts_with("--") {
        Cow::Borrowed(name)
    } else if name.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// In-place ASCII-lowercase variant of [`normalize_property_name`] for
/// the common path where the caller already owns a `String` from arg
/// coercion. Avoids the [`Cow`] `into_owned` round-trip for the
/// most-frequent shape (no uppercase).
#[must_use]
pub(crate) fn normalize_property_name_owned(mut name: String) -> String {
    if !name.starts_with("--") && name.bytes().any(|b| b.is_ascii_uppercase()) {
        name.make_ascii_lowercase();
    }
    name
}

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

/// WebIDL `unsigned long` conversion (ECMAScript `ToUint32`) for a CSSOM
/// indexed-getter argument.
///
/// CSSOM indexed getters take `unsigned long index` (e.g.
/// `CSSStyleDeclaration.item`), so a non-finite or out-of-`i32`-range
/// argument is mapped *through* `ToUint32` (NaN / ±∞ → 0, truncate toward
/// zero, modulo 2³²) rather than rejected — `style.item(NaN)` reads index
/// 0. An engine that pre-coerces (the VM host's `to_uint32`) passes an
/// already-`u32` value, for which this is the identity; an engine that
/// forwards the raw number (boa) relies on this. Doing the conversion in
/// the engine-independent handler keeps the indexed getters spec-correct
/// regardless of caller. A negative input wraps to a large index that
/// simply misses the bounds check → empty string.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn webidl_unsigned_long(n: f64) -> usize {
    if !n.is_finite() {
        return 0;
    }
    // 2³². `rem_euclid` keeps the result in `[0, 2³²)` and integer-valued.
    (n.trunc().rem_euclid(4_294_967_296.0)) as usize
}

/// Ensure `entity` is a live Element receiver, returning `NotFoundError`
/// otherwise.
///
/// Attribute-mutation handlers that route writes through the
/// [`EcsDom::set_attribute`] / [`EcsDom::remove_attribute`] chokepoints
/// use this to preserve the "stale / non-Element receiver → `NotFoundError`"
/// contract that the prior direct `require_attrs_mut` borrow surfaced.
/// `remove_attribute` returns `()` and silently no-ops on a dead receiver,
/// so a remove-path handler cannot derive the error from its return value
/// — this guards uniformly up front (the same `node_kind == Element`
/// predicate the chokepoints apply internally).
pub(crate) fn require_live_element(dom: &EcsDom, entity: Entity) -> Result<(), DomApiError> {
    if matches!(dom.node_kind(entity), Some(NodeKind::Element)) {
        Ok(())
    } else {
        Err(not_found_error("element not found"))
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
