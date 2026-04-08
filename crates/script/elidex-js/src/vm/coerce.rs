//! Type coercions and operator semantics for the elidex-js VM.
//!
//! Implements ES2020 abstract operations: ToNumber, ToString, ToBoolean,
//! ToInt32, ToUint32, and the equality/relational/arithmetic operators.

use super::value::{JsValue, ObjectId, ObjectKind, PropertyKey, StringId, VmError};
use super::VmInner;
use num_bigint::BigInt as BigIntValue;
use num_bigint::Sign;

// ---------------------------------------------------------------------------
// ToBoolean (ES2020 §7.1.2)
// ---------------------------------------------------------------------------

/// ToBoolean. Never throws.
pub(crate) fn to_boolean(vm: &VmInner, val: JsValue) -> bool {
    match val {
        JsValue::Undefined | JsValue::Null => false,
        JsValue::Boolean(b) => b,
        JsValue::Number(n) => n != 0.0 && !n.is_nan(),
        JsValue::String(id) => !vm.strings.get(id).is_empty(),
        JsValue::BigInt(id) => vm.bigints.get(id).sign() != Sign::NoSign,
        JsValue::Symbol(_) | JsValue::Object(_) => true,
    }
}

// ---------------------------------------------------------------------------
// ToNumber (ES2020 §7.1.3)
// ---------------------------------------------------------------------------

/// ToNumber (ES2020 §7.1.4). Symbol → TypeError per spec.
pub(crate) fn to_number(vm: &VmInner, val: JsValue) -> Result<f64, VmError> {
    match val {
        JsValue::Undefined => Ok(f64::NAN),
        JsValue::Object(id) => match vm.get_object(id).kind {
            ObjectKind::NumberWrapper(n) => Ok(n),
            ObjectKind::BooleanWrapper(false) => Ok(0.0),
            ObjectKind::BooleanWrapper(true) => Ok(1.0),
            ObjectKind::StringWrapper(sid) => Ok(string_to_number_u16(vm.strings.get(sid))),
            ObjectKind::BigIntWrapper(_) => Err(VmError::type_error(
                "Cannot convert a BigInt value to a number",
            )),
            _ => Ok(f64::NAN),
        },
        JsValue::Symbol(_) => Err(VmError::type_error(
            "Cannot convert a Symbol value to a number",
        )),
        JsValue::BigInt(_) => Err(VmError::type_error(
            "Cannot convert a BigInt value to a number",
        )),
        JsValue::Null | JsValue::Boolean(false) => Ok(0.0),
        JsValue::Boolean(true) => Ok(1.0),
        JsValue::Number(n) => Ok(n),
        JsValue::String(id) => Ok(string_to_number_u16(vm.strings.get(id))),
    }
}

/// Check if a character is ES2020 whitespace (WhiteSpace + LineTerminator).
fn is_js_whitespace_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// Trim leading and trailing ES2020 whitespace from a `&str`.
pub(super) fn trim_js(s: &str) -> &str {
    let start = s
        .char_indices()
        .find(|(_, ch)| !is_js_whitespace_char(*ch))
        .map_or(s.len(), |(i, _)| i);
    let end = s
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_js_whitespace_char(*ch))
        .map_or(start, |(i, ch)| i + ch.len_utf8());
    &s[start..end]
}

/// Parse a string to a number following ES2020 rules.
fn string_to_number(s: &str) -> f64 {
    let trimmed = trim_js(s);
    if trimmed.is_empty() {
        return 0.0;
    }
    if trimmed == "Infinity" || trimmed == "+Infinity" {
        return f64::INFINITY;
    }
    if trimmed == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    // Hex literal
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        return match u64::from_str_radix(&trimmed[2..], 16) {
            #[allow(clippy::cast_precision_loss)] // JS number semantics: all numbers are f64
            Ok(n) => n as f64,
            Err(_) => f64::NAN,
        };
    }
    // Octal literal (0o)
    if trimmed.starts_with("0o") || trimmed.starts_with("0O") {
        return match u64::from_str_radix(&trimmed[2..], 8) {
            #[allow(clippy::cast_precision_loss)]
            Ok(n) => n as f64,
            Err(_) => f64::NAN,
        };
    }
    // Binary literal (0b)
    if trimmed.starts_with("0b") || trimmed.starts_with("0B") {
        return match u64::from_str_radix(&trimmed[2..], 2) {
            #[allow(clippy::cast_precision_loss)]
            Ok(n) => n as f64,
            Err(_) => f64::NAN,
        };
    }
    {
        // Reject Rust-specific float literals that are not valid JS numbers.
        let lower = trimmed.to_ascii_lowercase();
        if lower == "inf"
            || lower == "infinity"
            || lower == "+inf"
            || lower == "+infinity"
            || lower == "-inf"
            || lower == "-infinity"
            || lower == "nan"
        {
            return f64::NAN;
        }
        trimmed.parse::<f64>().unwrap_or(f64::NAN)
    }
}

/// Parse a WTF-16 string to a number without heap allocation for ASCII content.
/// Falls back to UTF-8 conversion only for non-ASCII strings.
fn string_to_number_u16(units: &[u16]) -> f64 {
    use crate::wtf16::is_js_whitespace;
    // Trim leading/trailing whitespace directly on &[u16].
    let start = units
        .iter()
        .position(|&u| !is_js_whitespace(u))
        .unwrap_or(units.len());
    let end = units
        .iter()
        .rposition(|&u| !is_js_whitespace(u))
        .map_or(start, |i| i + 1);
    let trimmed = &units[start..end];
    if trimmed.is_empty() {
        return 0.0;
    }
    // Fast path: if all code units are ASCII, use a stack buffer.
    if trimmed.iter().all(|&u| u <= 0x7F) {
        let mut buf = [0u8; 64];
        if trimmed.len() <= buf.len() {
            #[allow(clippy::cast_possible_truncation)]
            for (i, &u) in trimmed.iter().enumerate() {
                buf[i] = u as u8;
            }
            // All bytes are valid ASCII ⊂ UTF-8.
            let s = std::str::from_utf8(&buf[..trimmed.len()]).unwrap_or("");
            return string_to_number(s);
        }
        // Long ASCII string — fall through to heap path.
    }
    // Slow path: non-ASCII or long content — allocate.
    let s = String::from_utf16_lossy(trimmed);
    string_to_number(&s)
}

// ---------------------------------------------------------------------------
// ToString (ES2020 §7.1.12)
// ---------------------------------------------------------------------------

/// ToString (ES2020 §7.1.12). Returns a `StringId` or throws `TypeError`
/// for Symbol values, per spec.
pub(crate) fn to_string(vm: &mut VmInner, val: JsValue) -> Result<StringId, VmError> {
    match val {
        JsValue::Undefined => Ok(vm.well_known.undefined),
        JsValue::Null => Ok(vm.well_known.null),
        JsValue::Boolean(true) => Ok(vm.well_known.r#true),
        JsValue::Boolean(false) => Ok(vm.well_known.r#false),
        JsValue::Number(n) => Ok(number_to_string_id(vm, n)),
        JsValue::String(id) => Ok(id),
        JsValue::Symbol(_) => Err(VmError::type_error(
            "Cannot convert a Symbol value to a string",
        )),
        JsValue::BigInt(id) => {
            let s = vm.bigints.get(id).to_string();
            Ok(vm.strings.intern(&s))
        }
        JsValue::Object(id) => match vm.get_object(id).kind {
            ObjectKind::NumberWrapper(n) => Ok(number_to_string_id(vm, n)),
            ObjectKind::StringWrapper(sid) => Ok(sid),
            ObjectKind::BooleanWrapper(true) => Ok(vm.well_known.r#true),
            ObjectKind::BooleanWrapper(false) => Ok(vm.well_known.r#false),
            ObjectKind::BigIntWrapper(bi_id) => {
                let s = vm.bigints.get(bi_id).to_string();
                Ok(vm.strings.intern(&s))
            }
            _ => Ok(vm.well_known.object_to_string),
        },
    }
}

/// Display-oriented string conversion that never throws. Used for
/// `console.log`, error messages, and other contexts where a human-readable
/// representation is needed rather than strict ES2020 ToString semantics.
pub(crate) fn to_display_string(vm: &mut VmInner, val: JsValue) -> StringId {
    match val {
        JsValue::Symbol(sid) => {
            let desc = vm.symbols[sid.0 as usize]
                .description
                .map(|d| vm.strings.get_utf8(d));
            let s = match desc {
                Some(d) => format!("Symbol({d})"),
                None => "Symbol()".to_string(),
            };
            vm.strings.intern(&s)
        }
        JsValue::BigInt(id) => {
            let s = vm.bigints.get(id).to_string();
            vm.strings.intern(&s)
        }
        other => to_string(vm, other).unwrap_or(vm.well_known.empty),
    }
}

/// Convert a number to its string representation and intern it.
fn number_to_string_id(vm: &mut VmInner, n: f64) -> StringId {
    if n.is_nan() {
        return vm.well_known.nan;
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            vm.well_known.infinity
        } else {
            vm.well_known.neg_infinity
        };
    }
    if n == 0.0 {
        return vm.well_known.zero;
    }
    // Use integer format if the number is a safe integer.
    #[allow(clippy::cast_precision_loss)] // round-trip check is intentional
    let s = if n == (n as i64 as f64) && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        // Use Rust's default f64 Display which matches JS for most values.
        let s = format!("{n}");
        // JS uses "1e+21" style for very large numbers, Rust may differ.
        // For M4-10 this is acceptable; full Number.prototype.toString in M4-10.2.
        s
    };
    vm.strings.intern(&s)
}

// ---------------------------------------------------------------------------
// ToInt32 / ToUint32 (ES2020 §7.1.6, §7.1.7)
// ---------------------------------------------------------------------------

/// ToInt32 (ES2020 §7.1.6). Used by bitwise operators.
#[inline]
pub(crate) fn to_int32(vm: &VmInner, val: JsValue) -> Result<i32, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_int32(n))
}

/// ToUint32 (ES2020 §7.1.7). Used by `>>>`.
#[inline]
pub(crate) fn to_uint32(vm: &VmInner, val: JsValue) -> Result<u32, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_uint32(n))
}

/// The modulo-2^32 conversion from f64 to i32 (ES2020 §7.1.6 step 5-6).
fn f64_to_int32(n: f64) -> i32 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc();
    let int32bit = int.rem_euclid(4_294_967_296.0);
    if int32bit >= 2_147_483_648.0 {
        (int32bit - 4_294_967_296.0) as i32
    } else {
        int32bit as i32
    }
}

/// The modulo-2^32 conversion from f64 to u32 (ES2020 §7.1.7 step 5-6).
fn f64_to_uint32(n: f64) -> u32 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc();
    let int32bit = int.rem_euclid(4_294_967_296.0);
    // rem_euclid guarantees the result is in [0, 2^32), so the cast is safe.
    #[allow(clippy::cast_sign_loss)]
    let result = int32bit as u32;
    result
}

// ---------------------------------------------------------------------------
// Strict Equality (ES2020 §7.2.16)
// ---------------------------------------------------------------------------

/// Strict equality (`===`). Never throws.
///
/// BigInt values are compared by mathematical value, not by handle identity,
/// so the VM's `BigIntPool` is required.
pub(crate) fn strict_eq(vm: &VmInner, a: JsValue, b: JsValue) -> bool {
    match (a, b) {
        (JsValue::BigInt(ai), JsValue::BigInt(bi)) => vm.bigints.get(ai) == vm.bigints.get(bi),
        _ => a == b,
    }
}

// ---------------------------------------------------------------------------
// Abstract Equality (ES2020 §7.2.15)
// ---------------------------------------------------------------------------

/// Abstract equality (`==`). May need string/number coercions.
pub(crate) fn abstract_eq(vm: &mut VmInner, a: JsValue, b: JsValue) -> bool {
    // Same type → strict_eq
    if same_type(a, b) {
        return strict_eq(vm, a, b);
    }

    match (a, b) {
        // null == undefined (and vice versa)
        (JsValue::Null, JsValue::Undefined) | (JsValue::Undefined, JsValue::Null) => true,

        // Number == String → Number == ToNumber(String)
        (JsValue::Number(_), JsValue::String(s)) => {
            let n = string_to_number(&vm.strings.get_utf8(s));
            abstract_eq(vm, a, JsValue::Number(n))
        }
        (JsValue::String(s), JsValue::Number(_)) => {
            let n = string_to_number(&vm.strings.get_utf8(s));
            abstract_eq(vm, JsValue::Number(n), b)
        }

        // BigInt == BigInt handled by same_type above.
        // BigInt == Number (§7.2.14 step 5/6)
        (JsValue::BigInt(bi), JsValue::Number(n)) | (JsValue::Number(n), JsValue::BigInt(bi)) => {
            if n.is_nan() || n.is_infinite() {
                return false;
            }
            if n != n.floor() {
                return false;
            }
            // Integer Number → compare with BigInt value.
            #[allow(clippy::cast_possible_truncation)]
            let n_big = if n.abs() < 2.0f64.powi(53) {
                BigIntValue::from(n as i64)
            } else {
                // Large integer — use string round-trip.
                match format!("{n:.0}").parse::<BigIntValue>() {
                    Ok(v) => v,
                    Err(_) => return false,
                }
            };
            vm.bigints.get(bi) == &n_big
        }

        // BigInt == String → parse string as BigInt
        (JsValue::BigInt(bi), JsValue::String(s)) | (JsValue::String(s), JsValue::BigInt(bi)) => {
            let text = vm.strings.get_utf8(s);
            match super::dispatch_helpers::parse_bigint_literal(trim_js(&text)) {
                Some(parsed) => vm.bigints.get(bi) == &parsed,
                None => false,
            }
        }

        // BigInt == Boolean → convert boolean to BigInt
        (JsValue::BigInt(bi), JsValue::Boolean(bl))
        | (JsValue::Boolean(bl), JsValue::BigInt(bi)) => {
            let bool_big = BigIntValue::from(i32::from(bl));
            vm.bigints.get(bi) == &bool_big
        }

        // Symbol == non-Symbol → false (symbols are unique; same-type
        // comparison is handled by strict_eq above).
        (JsValue::Symbol(_), _) | (_, JsValue::Symbol(_)) => false,

        // Boolean == x → ToNumber(Boolean) == x
        (JsValue::Boolean(bl), _) => {
            let n = if bl { 1.0 } else { 0.0 };
            abstract_eq(vm, JsValue::Number(n), b)
        }
        (_, JsValue::Boolean(bl)) => {
            let n = if bl { 1.0 } else { 0.0 };
            abstract_eq(vm, a, JsValue::Number(n))
        }

        // Object == primitive → ToPrimitive (§7.2.15 steps 10, 12)
        (JsValue::Object(_), _) => match vm.to_primitive(a, "default") {
            Ok(prim) => abstract_eq(vm, prim, b),
            Err(_) => false,
        },
        (_, JsValue::Object(_)) => match vm.to_primitive(b, "default") {
            Ok(prim) => abstract_eq(vm, a, prim),
            Err(_) => false,
        },
        _ => false,
    }
}

/// Check if two values have the same JS type.
fn same_type(a: JsValue, b: JsValue) -> bool {
    std::mem::discriminant(&a) == std::mem::discriminant(&b)
}

// ---------------------------------------------------------------------------
// typeof (ES2020 §12.5.6)
// ---------------------------------------------------------------------------

/// Returns the typeof string ID for a value.
pub(crate) fn typeof_str(vm: &VmInner, val: JsValue) -> StringId {
    match val {
        JsValue::Undefined => vm.well_known.undefined,
        JsValue::Null => vm.well_known.object_type,
        JsValue::Boolean(_) => vm.well_known.boolean_type,
        JsValue::Number(_) => vm.well_known.number_type,
        JsValue::String(_) => vm.well_known.string_type,
        JsValue::Symbol(_) => vm.well_known.symbol_type,
        JsValue::BigInt(_) => vm.well_known.bigint_type,
        JsValue::Object(id) => {
            if let Some(obj) = vm.objects[id.0 as usize].as_ref() {
                match &obj.kind {
                    ObjectKind::Function(_)
                    | ObjectKind::BoundFunction { .. }
                    | ObjectKind::NativeFunction(_) => vm.well_known.function_type,
                    _ => vm.well_known.object_type,
                }
            } else {
                vm.well_known.object_type
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property lookup helper
// ---------------------------------------------------------------------------

/// Result of a prototype-chain property lookup.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PropertyResult {
    /// A plain data value.
    Data(JsValue),
    /// An accessor getter function that the caller must invoke.
    Getter(ObjectId),
}

/// Look up a property on an object, following the prototype chain.
pub(crate) fn get_property(
    vm: &VmInner,
    obj_id: ObjectId,
    key: PropertyKey,
) -> Option<PropertyResult> {
    let mut current = Some(obj_id);
    while let Some(id) = current {
        if let Some(obj) = vm.objects[id.0 as usize].as_ref() {
            // Check own properties.
            if let Some((val, _attrs)) = obj.storage.get(key, &vm.shapes) {
                return Some(match val {
                    super::value::PropertyValue::Data(v) => PropertyResult::Data(*v),
                    super::value::PropertyValue::Accessor {
                        getter: Some(g), ..
                    } => PropertyResult::Getter(*g),
                    super::value::PropertyValue::Accessor { getter: None, .. } => {
                        PropertyResult::Data(JsValue::Undefined)
                    }
                });
            }
            // Check array length.
            if key == PropertyKey::String(vm.well_known.length) {
                if let ObjectKind::Array { ref elements } = obj.kind {
                    #[allow(clippy::cast_precision_loss)] // JS array length is always safe
                    return Some(PropertyResult::Data(JsValue::Number(elements.len() as f64)));
                }
            }
            current = obj.prototype;
        } else {
            break;
        }
    }
    None
}

/// Result of looking up an inherited property on the prototype chain.
///
/// Used by `set_property_val` to implement §9.1.9 OrdinarySet:
/// - Setter: invoke the setter.
/// - WritableFalse: reject the set (TypeError in strict, silent in sloppy).
/// - AccessorNoSetter: reject the set (same as WritableFalse).
/// - None: no inherited property found; create own property.
pub(crate) enum InheritedProperty {
    Setter(ObjectId),
    WritableFalse,
    AccessorNoSetter,
    None,
}

/// Look up an inherited property on an object's prototype chain (§9.1.9).
///
/// Skips the object's own properties and walks prototypes only.
pub(crate) fn find_inherited_property(
    vm: &VmInner,
    obj_id: ObjectId,
    key: PropertyKey,
) -> InheritedProperty {
    // Start from the prototype, not the object itself.
    let start = vm.objects[obj_id.0 as usize]
        .as_ref()
        .and_then(|o| o.prototype);
    let mut current = start;
    while let Some(id) = current {
        if let Some(obj) = vm.objects[id.0 as usize].as_ref() {
            if let Some((val, attrs)) = obj.storage.get(key, &vm.shapes) {
                return match val {
                    super::value::PropertyValue::Accessor {
                        setter: Some(s), ..
                    } => InheritedProperty::Setter(*s),
                    super::value::PropertyValue::Accessor { setter: None, .. } => {
                        InheritedProperty::AccessorNoSetter
                    }
                    super::value::PropertyValue::Data(_) if !attrs.writable => {
                        InheritedProperty::WritableFalse
                    }
                    super::value::PropertyValue::Data(_) => InheritedProperty::None,
                };
            }
            current = obj.prototype;
        } else {
            break;
        }
    }
    InheritedProperty::None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::coerce_ops::*;
    use super::super::Vm;
    use super::*;

    #[test]
    fn to_boolean_values() {
        let vm = Vm::new();
        let i = &vm.inner;
        assert!(!to_boolean(i, JsValue::Undefined));
        assert!(!to_boolean(i, JsValue::Null));
        assert!(!to_boolean(i, JsValue::Boolean(false)));
        assert!(to_boolean(i, JsValue::Boolean(true)));
        assert!(!to_boolean(i, JsValue::Number(0.0)));
        assert!(!to_boolean(i, JsValue::Number(f64::NAN)));
        assert!(to_boolean(i, JsValue::Number(1.0)));
        assert!(to_boolean(i, JsValue::Number(-1.0)));
        // Empty string → false
        let empty = i.well_known.empty;
        assert!(!to_boolean(i, JsValue::String(empty)));
        // Non-empty string → true
        let hello = i.well_known.undefined; // "undefined" is non-empty
        assert!(to_boolean(i, JsValue::String(hello)));
        // Object → always true
        assert!(to_boolean(i, JsValue::Object(ObjectId(0))));
    }

    #[test]
    fn to_number_values() {
        let vm = Vm::new();
        let i = &vm.inner;
        assert!(to_number(i, JsValue::Undefined).unwrap().is_nan());
        assert_eq!(to_number(i, JsValue::Null).unwrap(), 0.0);
        assert_eq!(to_number(i, JsValue::Boolean(true)).unwrap(), 1.0);
        assert_eq!(to_number(i, JsValue::Boolean(false)).unwrap(), 0.0);
        assert_eq!(to_number(i, JsValue::Number(42.0)).unwrap(), 42.0);
    }

    #[test]
    fn to_number_symbol_throws() {
        let mut vm = Vm::new();
        let sid = vm.inner.alloc_symbol(None);
        let result = to_number(&vm.inner, JsValue::Symbol(sid));
        assert!(result.is_err());
    }

    #[test]
    fn string_to_number_cases() {
        assert_eq!(string_to_number(""), 0.0);
        assert_eq!(string_to_number("  "), 0.0);
        assert_eq!(string_to_number("42"), 42.0);
        assert_eq!(string_to_number("  3.125  "), 3.125);
        assert_eq!(string_to_number("0xff"), 255.0);
        assert_eq!(string_to_number("0b1010"), 10.0);
        assert_eq!(string_to_number("0o17"), 15.0);
        assert_eq!(string_to_number("Infinity"), f64::INFINITY);
        assert_eq!(string_to_number("-Infinity"), f64::NEG_INFINITY);
        assert!(string_to_number("abc").is_nan());
        assert!(string_to_number("12abc").is_nan());
    }

    #[test]
    fn to_string_values() {
        let mut vm = Vm::new();
        let i = &mut vm.inner;

        let id = to_string(i, JsValue::Undefined).unwrap();
        assert_eq!(i.strings.get_utf8(id), "undefined");
        let id = to_string(i, JsValue::Null).unwrap();
        assert_eq!(i.strings.get_utf8(id), "null");
        let id = to_string(i, JsValue::Boolean(true)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "true");
        let id = to_string(i, JsValue::Boolean(false)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "false");
        let id = to_string(i, JsValue::Number(0.0)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "0");
        let id = to_string(i, JsValue::Number(42.0)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "42");
        let id = to_string(i, JsValue::Number(-1.5)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "-1.5");
        let id = to_string(i, JsValue::Number(f64::NAN)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "NaN");
        let id = to_string(i, JsValue::Number(f64::INFINITY)).unwrap();
        assert_eq!(i.strings.get_utf8(id), "Infinity");
    }

    #[test]
    fn to_string_symbol_throws() {
        let mut vm = Vm::new();
        let sid = vm.inner.alloc_symbol(None);
        let result = to_string(&mut vm.inner, JsValue::Symbol(sid));
        assert!(result.is_err());
    }

    #[test]
    fn to_display_string_symbol() {
        let mut vm = Vm::new();
        let desc = vm.inner.strings.intern("foo");
        let sid = vm.inner.alloc_symbol(Some(desc));
        let id = to_display_string(&mut vm.inner, JsValue::Symbol(sid));
        assert_eq!(vm.inner.strings.get_utf8(id), "Symbol(foo)");
    }

    #[test]
    fn to_int32_cases() {
        let vm = Vm::new();
        let i = &vm.inner;
        assert_eq!(to_int32(i, JsValue::Number(0.0)).unwrap(), 0);
        assert_eq!(to_int32(i, JsValue::Number(1.7)).unwrap(), 1);
        assert_eq!(to_int32(i, JsValue::Number(-1.7)).unwrap(), -1);
        assert_eq!(to_int32(i, JsValue::Number(f64::NAN)).unwrap(), 0);
        assert_eq!(to_int32(i, JsValue::Number(f64::INFINITY)).unwrap(), 0);
    }

    #[test]
    fn strict_eq_cases() {
        let vm = Vm::new();
        let i = &vm.inner;
        assert!(strict_eq(i, JsValue::Undefined, JsValue::Undefined));
        assert!(strict_eq(i, JsValue::Null, JsValue::Null));
        assert!(!strict_eq(i, JsValue::Undefined, JsValue::Null));
        assert!(strict_eq(i, JsValue::Number(1.0), JsValue::Number(1.0)));
        assert!(!strict_eq(
            i,
            JsValue::Number(f64::NAN),
            JsValue::Number(f64::NAN)
        ));
        assert!(strict_eq(i, JsValue::Number(0.0), JsValue::Number(-0.0)));
        assert!(strict_eq(
            i,
            JsValue::String(StringId(0)),
            JsValue::String(StringId(0))
        ));
        assert!(!strict_eq(
            i,
            JsValue::String(StringId(0)),
            JsValue::String(StringId(1))
        ));
    }

    #[test]
    fn abstract_eq_null_undefined() {
        let mut vm = Vm::new();
        assert!(abstract_eq(
            &mut vm.inner,
            JsValue::Null,
            JsValue::Undefined
        ));
        assert!(abstract_eq(
            &mut vm.inner,
            JsValue::Undefined,
            JsValue::Null
        ));
        assert!(!abstract_eq(
            &mut vm.inner,
            JsValue::Null,
            JsValue::Boolean(false)
        ));
    }

    #[test]
    fn abstract_eq_coercion() {
        let mut vm = Vm::new();
        let one_str = vm.inner.strings.intern("1");
        // "1" == 1
        assert!(abstract_eq(
            &mut vm.inner,
            JsValue::String(one_str),
            JsValue::Number(1.0)
        ));
        // true == 1
        assert!(abstract_eq(
            &mut vm.inner,
            JsValue::Boolean(true),
            JsValue::Number(1.0)
        ));
        // false == 0
        assert!(abstract_eq(
            &mut vm.inner,
            JsValue::Boolean(false),
            JsValue::Number(0.0)
        ));
    }

    #[test]
    fn typeof_values() {
        let vm = Vm::new();
        let i = &vm.inner;

        let id = typeof_str(i, JsValue::Undefined);
        assert_eq!(i.strings.get_utf8(id), "undefined");
        let id = typeof_str(i, JsValue::Null);
        assert_eq!(i.strings.get_utf8(id), "object");
        let id = typeof_str(i, JsValue::Boolean(true));
        assert_eq!(i.strings.get_utf8(id), "boolean");
        let id = typeof_str(i, JsValue::Number(0.0));
        assert_eq!(i.strings.get_utf8(id), "number");
        let s = i.well_known.empty;
        let id = typeof_str(i, JsValue::String(s));
        assert_eq!(i.strings.get_utf8(id), "string");
    }

    #[test]
    fn add_string_concat() {
        let mut vm = Vm::new();
        let hello = vm.inner.strings.intern("hello");
        let world = vm.inner.strings.intern(" world");
        let result = vm
            .inner
            .op_add(JsValue::String(hello), JsValue::String(world))
            .unwrap();
        let JsValue::String(id) = result else {
            panic!("expected string");
        };
        assert_eq!(vm.inner.strings.get_utf8(id), "hello world");
    }

    #[test]
    fn add_number_plus_string() {
        let mut vm = Vm::new();
        let s = vm.inner.strings.intern("px");
        let result = vm
            .inner
            .op_add(JsValue::Number(42.0), JsValue::String(s))
            .unwrap();
        let JsValue::String(id) = result else {
            panic!("expected string");
        };
        assert_eq!(vm.inner.strings.get_utf8(id), "42px");
    }

    #[test]
    fn add_numbers() {
        let mut vm = Vm::new();
        let result = vm
            .inner
            .op_add(JsValue::Number(1.0), JsValue::Number(2.0))
            .unwrap();
        assert_eq!(result, JsValue::Number(3.0));
    }

    #[test]
    fn relational_comparison() {
        let mut vm = Vm::new();
        assert_eq!(
            abstract_relational(
                &mut vm.inner,
                JsValue::Number(1.0),
                JsValue::Number(2.0),
                true,
            )
            .unwrap(),
            Some(true)
        );
        assert_eq!(
            abstract_relational(
                &mut vm.inner,
                JsValue::Number(2.0),
                JsValue::Number(1.0),
                true,
            )
            .unwrap(),
            Some(false)
        );
        // NaN comparison → None (undefined)
        assert_eq!(
            abstract_relational(
                &mut vm.inner,
                JsValue::Number(f64::NAN),
                JsValue::Number(1.0),
                true,
            )
            .unwrap(),
            None
        );
        // String comparison (lexicographic)
        let a = vm.inner.strings.intern("abc");
        let b = vm.inner.strings.intern("abd");
        assert_eq!(
            abstract_relational(&mut vm.inner, JsValue::String(a), JsValue::String(b), true,)
                .unwrap(),
            Some(true)
        );
    }

    #[test]
    fn bitwise_operations() {
        let mut vm = Vm::new();
        let i = &mut vm.inner;
        assert_eq!(
            op_bitwise(
                i,
                JsValue::Number(5.0),
                JsValue::Number(3.0),
                BitwiseOp::And
            )
            .unwrap(),
            JsValue::Number(1.0)
        );
        assert_eq!(
            op_bitwise(i, JsValue::Number(5.0), JsValue::Number(3.0), BitwiseOp::Or).unwrap(),
            JsValue::Number(7.0)
        );
        assert_eq!(
            op_bitwise(
                i,
                JsValue::Number(5.0),
                JsValue::Number(3.0),
                BitwiseOp::Xor
            )
            .unwrap(),
            JsValue::Number(6.0)
        );
        assert_eq!(
            op_bitwise(
                i,
                JsValue::Number(1.0),
                JsValue::Number(2.0),
                BitwiseOp::Shl
            )
            .unwrap(),
            JsValue::Number(4.0)
        );
        assert_eq!(
            op_bitwise(
                i,
                JsValue::Number(-8.0),
                JsValue::Number(2.0),
                BitwiseOp::Shr
            )
            .unwrap(),
            JsValue::Number(-2.0)
        );
    }

    #[test]
    fn unary_operators() {
        let mut vm = Vm::new();
        let i = &mut vm.inner;
        assert_eq!(
            op_neg(i, JsValue::Number(5.0)).unwrap(),
            JsValue::Number(-5.0)
        );
        assert_eq!(
            op_pos(i, JsValue::Boolean(true)).unwrap(),
            JsValue::Number(1.0)
        );
        assert_eq!(op_not(i, JsValue::Boolean(true)), JsValue::Boolean(false));
        assert_eq!(op_not(i, JsValue::Number(0.0)), JsValue::Boolean(true));
        assert_eq!(
            op_bitnot(i, JsValue::Number(5.0)).unwrap(),
            JsValue::Number(-6.0)
        );
        assert_eq!(op_void(), JsValue::Undefined);
    }
}
