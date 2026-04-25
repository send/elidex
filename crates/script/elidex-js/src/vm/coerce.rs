//! Type coercions and operator semantics for the elidex-js VM.
//!
//! Implements ES2020 abstract operations: ToNumber, ToString, ToBoolean,
//! ToInt32, ToUint32, and the equality/relational/arithmetic operators.

use super::coerce_format::write_number_es;
#[cfg(feature = "engine")]
use super::value::NativeContext;
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
        JsValue::Empty | JsValue::Undefined | JsValue::Null => false,
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
///
/// KNOWN LIMITATION: For non-wrapper Objects, this returns `NaN` instead
/// of performing `? ToPrimitive(val, "number") → ? ToNumber(prim)` per
/// §7.1.4 step 4.  Fixing this requires threading `&mut VmInner` through
/// ~175 call sites (all arithmetic / comparison / bitwise paths).  Tracked
/// as a dedicated follow-up PR — see phase4-plan.md.
pub(crate) fn to_number(vm: &VmInner, val: JsValue) -> Result<f64, VmError> {
    match val {
        JsValue::Empty | JsValue::Undefined => Ok(f64::NAN),
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
pub(super) fn string_to_number(s: &str) -> f64 {
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
        JsValue::Empty | JsValue::Undefined => Ok(vm.well_known.undefined),
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
            // KNOWN LIMITATION: §7.1.12 step 9 requires
            // `? ToPrimitive(val, "string") → ? ToString(prim)` for
            // non-wrapper Objects.  Tracked as dedicated follow-up
            // alongside ToNumber (see phase4-plan.md).
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
/// Uses ES §7.1.12.1 Number::toString formatting.
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
    let mut buf = String::with_capacity(24);
    write_number_es(n, &mut buf);
    vm.strings.intern(&buf)
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
pub(crate) fn f64_to_int32(n: f64) -> i32 {
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
pub(super) fn f64_to_uint32(n: f64) -> u32 {
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

/// ToUint16 (ES2020 §7.1.10 / WebIDL §3.10.5).  Modular truncation
/// to the `[0, 2^16)` range.  Used by WebIDL `unsigned short`
/// coercion (`MouseEventInit.buttons`) and by `String.fromCharCode`
/// ES §22.1.2.1 step 3.
///
/// NaN / ±Infinity / ±0 all map to `0`, matching the behaviour
/// of the sibling `f64_to_int32` / `f64_to_uint32` helpers.
///
/// **Do not use for WebIDL attributes tagged `[EnforceRange]`**
/// (e.g. `ResponseInit.status`).  Those must reject out-of-range
/// inputs rather than wrap; use [`enforce_range_unsigned_short`]
/// instead.
pub(crate) fn f64_to_uint16(n: f64) -> u16 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc().rem_euclid(65_536.0);
    // rem_euclid guarantees the result is in [0, 2^16), so the
    // cast is infallible.  The `cast_possible_truncation` lint
    // fires because the underlying f64 could theoretically hold
    // any finite value, but the preceding rem_euclid normalises
    // it into the u16 range first.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let result = int as u16;
    result
}

// ---------------------------------------------------------------------------
// ToInt8 / ToUint8 / ToUint8Clamp / ToInt16 (ES2020 §7.1.8-.11)
// ---------------------------------------------------------------------------
//
// Engine-feature-only: the helpers below are used exclusively by
// TypedArray (`vm::host::typed_array` — engine-gated).  Gating them
// behind the same feature keeps non-engine builds dead-code-warning
// free, matching the sibling `enforce_range_unsigned_short` above.

/// The modulo-2^8 signed conversion from f64 to i8 (ES §7.1.9).
/// Used by `Int8Array` indexed element writes.  NaN / ±∞ / ±0 → 0.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn f64_to_int8(n: f64) -> i8 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc().rem_euclid(256.0);
    // rem_euclid result is in [0, 256); fold the upper half back into
    // negative range (`[128, 256) -> [-128, 0)`) to complete ToInt8.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let as_u8 = int as u8;
    as_u8 as i8
}

/// The modulo-2^8 unsigned conversion from f64 to u8 (ES §7.1.10).
/// Used by `Uint8Array` indexed element writes.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn f64_to_uint8(n: f64) -> u8 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc().rem_euclid(256.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = int as u8;
    result
}

/// `ToUint8Clamp` (ES §7.1.11) — **not** a modular conversion.
/// NaN → 0, n ≤ 0 → 0, n ≥ 255 → 255, else round with
/// **roundTiesToEven** (IEEE 754 banker's rounding).  Used by
/// `Uint8ClampedArray` indexed element writes; diverges from
/// `ToUint8` on both the clamping and the rounding-mode choice
/// (`ToUint8` truncates toward zero and wraps; `ToUint8Clamp`
/// clamps at the domain edge and rounds ties to even).
#[cfg(feature = "engine")]
#[inline]
#[must_use]
pub(crate) fn f64_to_uint8_clamp(n: f64) -> u8 {
    if n.is_nan() || n <= 0.0 {
        return 0;
    }
    if n >= 255.0 {
        return 255;
    }
    // `f64::round_ties_even` stabilised Rust 1.77 — project pins
    // 1.95 per `rust-toolchain.toml`, so the unconditional call
    // is safe.  Produces 2.5 → 2, 3.5 → 4, 4.5 → 4, -0.5 → 0,
    // 255.5 → 255 (already clamped) behaviour.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rounded = n.round_ties_even() as u8;
    rounded
}

/// The modulo-2^16 signed conversion from f64 to i16 (ES §7.1.11
/// ToInt16).  Used by `Int16Array` indexed element writes.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn f64_to_int16(n: f64) -> i16 {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return 0;
    }
    let int = n.trunc().rem_euclid(65_536.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let as_u16 = int as u16;
    as_u16 as i16
}

/// ToInt8 wrapper — coerces `val` via `ToNumber` then folds into i8.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn to_int8(vm: &VmInner, val: JsValue) -> Result<i8, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_int8(n))
}

/// ToUint8 wrapper — coerces `val` via `ToNumber` then folds into u8.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn to_uint8(vm: &VmInner, val: JsValue) -> Result<u8, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_uint8(n))
}

/// ToUint8Clamp wrapper — coerces `val` via `ToNumber` then clamps
/// (ES §7.1.11).
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn to_uint8_clamp(vm: &VmInner, val: JsValue) -> Result<u8, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_uint8_clamp(n))
}

/// ToInt16 wrapper — coerces `val` via `ToNumber` then folds into i16.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn to_int16(vm: &VmInner, val: JsValue) -> Result<i16, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_int16(n))
}

/// ToUint16 wrapper — coerces `val` via `ToNumber` then folds into u16.
#[cfg(feature = "engine")]
#[inline]
pub(crate) fn to_uint16(vm: &VmInner, val: JsValue) -> Result<u16, VmError> {
    let n = to_number(vm, val)?;
    Ok(f64_to_uint16(n))
}

/// WebIDL `[EnforceRange] unsigned short` conversion (§3.2.4.7
/// step 7).  NaN / ±∞ / out-of-`[0, 65535]` all throw `TypeError`
/// rather than silently wrapping — the spec-mandated path for
/// `ResponseInit.status`, `Response.redirect(..., status)`, and
/// every other `[EnforceRange] unsigned short` Fetch attribute.
///
/// `error_prefix` is the caller's reporting context
/// (e.g. `"Failed to construct 'Response'"` or
/// `"Failed to execute 'redirect' on 'Response'"`) so the
/// rejection message mirrors what the method would report on
/// any other WebIDL conversion failure.
#[cfg(feature = "engine")]
pub(crate) fn enforce_range_unsigned_short(n: f64, error_prefix: &str) -> Result<u16, VmError> {
    if !n.is_finite() {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Value is not a finite number."
        )));
    }
    // WebIDL §3.2.4.7 step 3: truncate toward zero (ToInteger).
    let truncated = n.trunc();
    if !(0.0..=65_535.0).contains(&truncated) {
        return Err(VmError::type_error(format!(
            "{error_prefix}: Value {n} is outside the range of unsigned short [0, 65535]."
        )));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = truncated as u16;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Slice / index abstract ops (ES §7.1.5 ToIntegerOrInfinity, §7.1.22 ToIndex)
// ---------------------------------------------------------------------------

/// ES §7.1.5 `ToIntegerOrInfinity`, **starting from a number**.
/// `NaN → 0`; `±Infinity` is preserved; otherwise truncate toward
/// zero.  Returns `f64` so the caller picks the final width.
///
/// This is the cheap pure-arithmetic tail of the spec algorithm —
/// the spec also runs `ToNumber` on the input before truncating, so
/// callers receiving a non-number `JsValue` must coerce via
/// [`to_number`] (or `NativeContext::to_number`) themselves.  For
/// the full `ToIndex`-with-coercion-and-range-check pipeline used
/// by TypedArray / DataView constructors, see `to_index_u32`.
pub(crate) fn to_integer_or_infinity(n: f64) -> f64 {
    if n.is_nan() {
        0.0
    } else {
        n.trunc()
    }
}

/// Clamp a relative index `n` to `[0, len_f]` after applying
/// [`to_integer_or_infinity`].  Negative values count from the end.
/// Returns `f64`; the caller picks the final width with a single
/// `as u32` / `as usize` cast — the returned value satisfies
/// `0.0 <= result <= len_f`, so when `len_f` originated from a
/// `u32` / `usize` value the cast is exact (the Rust 1.45+
/// saturating-cast fallback for out-of-range / non-finite inputs is
/// not exercised).
///
/// Shared by `Array.prototype.{slice, copyWithin, fill, splice}`,
/// `%TypedArray%.prototype.*`, `ArrayBuffer.prototype.slice`, and
/// `Blob.prototype.slice`.  Each caller knows its own length type
/// and chooses the cast — thin typed wrappers
/// (`relative_index_u32` in `vm::host::typed_array_methods`,
/// `relative_index` in `vm::host::array_buffer`, `resolve_index` in
/// `vm::natives_array`) keep the per-method ergonomics typed.
pub(crate) fn relative_index_f64(n: f64, len_f: f64) -> f64 {
    let trunc = to_integer_or_infinity(n);
    if trunc < 0.0 {
        (len_f + trunc).max(0.0)
    } else {
        trunc.min(len_f)
    }
}

/// ES §7.1.22 `ToIndex`, narrowed to `u32`.  Coerces `val` via
/// `ToNumber`, truncates toward zero per `ToIntegerOrInfinity`, and
/// rejects negative / non-finite / `> u32::MAX` results with a
/// `RangeError`.  Used by the TypedArray and DataView constructors
/// where the spec specifies `unsigned long long`-with-`ToIndex` but
/// our `[[ByteLength]]` slot is `u32`, so the upper bound has to
/// drop from `2^53 − 1` down to `u32::MAX`.
///
/// Error messages mirror V8's
/// `"Failed to construct '{ctor_name}': {what} ..."` shape so
/// browser-compat tests do not regress.  The returned value
/// satisfies `0 <= result <= u32::MAX`, so the `truncated as u32`
/// cast at the bottom is exact.
#[cfg(feature = "engine")]
pub(crate) fn to_index_u32(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    ctor_name: &str,
    what: &str,
) -> Result<u32, VmError> {
    let n = ctx.to_number(val)?;
    let truncated = to_integer_or_infinity(n);
    if !truncated.is_finite() || truncated < 0.0 {
        return Err(VmError::range_error(format!(
            "Failed to construct '{ctor_name}': {what} must be a non-negative safe integer"
        )));
    }
    if truncated > f64::from(u32::MAX) {
        return Err(VmError::range_error(format!(
            "Failed to construct '{ctor_name}': {what} exceeds the supported maximum"
        )));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let as_u32 = truncated as u32;
    Ok(as_u32)
}

// ---------------------------------------------------------------------------
// ToObject (ES2020 §7.1.13)
// ---------------------------------------------------------------------------

/// §6.2.4.5 RequireObjectCoercible: throws TypeError for null/undefined
/// (including the internal `Empty` sentinel, which should not leak out of
/// firewalled paths but is defensively rejected here), otherwise returns
/// `Ok(())`.  Used by property access/assignment/delete paths that must
/// reject nullish bases before any prototype-chain walk or ToObject boxing.
/// The TypeError message mirrors [`to_object`] for consistency across the
/// VM's user-visible nullish-conversion errors.
pub(super) fn require_object_coercible(val: JsValue) -> Result<(), VmError> {
    match val {
        JsValue::Null | JsValue::Undefined | JsValue::Empty => Err(VmError::type_error(
            "Cannot convert undefined or null to object",
        )),
        _ => Ok(()),
    }
}

/// Convert a value to an Object. Throws TypeError for null/undefined.
/// Primitives are wrapped in their corresponding wrapper objects.
pub(super) fn to_object(vm: &mut VmInner, val: JsValue) -> Result<ObjectId, VmError> {
    use super::shape;
    use super::value::{Object, PropertyStorage};

    match val {
        JsValue::Object(id) => Ok(id),
        JsValue::Null | JsValue::Undefined => Err(VmError::type_error(
            "Cannot convert undefined or null to object",
        )),
        JsValue::Number(n) => {
            let wrapper = vm.alloc_object(Object {
                kind: ObjectKind::NumberWrapper(n),
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: vm.number_prototype,
                extensible: true,
            });
            Ok(wrapper)
        }
        JsValue::String(s) => Ok(vm.create_string_wrapper(s)),
        JsValue::Boolean(b) => {
            let wrapper = vm.alloc_object(Object {
                kind: ObjectKind::BooleanWrapper(b),
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: vm.boolean_prototype,
                extensible: true,
            });
            Ok(wrapper)
        }
        JsValue::BigInt(id) => {
            let wrapper = vm.alloc_object(Object {
                kind: ObjectKind::BigIntWrapper(id),
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: vm.bigint_prototype,
                extensible: true,
            });
            Ok(wrapper)
        }
        JsValue::Symbol(id) => {
            let wrapper = vm.alloc_object(Object {
                kind: ObjectKind::SymbolWrapper(id),
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: vm.symbol_prototype,
                extensible: true,
            });
            Ok(wrapper)
        }
        JsValue::Empty => Err(VmError::type_error("Cannot convert value to object")),
    }
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

/// Abstract equality (`==`). May need string/number coercions.  Returns
/// `Err` when a user-defined `@@toPrimitive`/`valueOf`/`toString` throws
/// (§7.2.15 `?` steps 10-12 require abrupt-completion propagation).
pub(crate) fn abstract_eq(vm: &mut VmInner, a: JsValue, b: JsValue) -> Result<bool, VmError> {
    // Empty (sparse hole) is treated as Undefined in all coercions.
    let a = if a.is_empty() { JsValue::Undefined } else { a };
    let b = if b.is_empty() { JsValue::Undefined } else { b };

    // Same type → strict_eq
    if same_type(a, b) {
        return Ok(strict_eq(vm, a, b));
    }

    Ok(match (a, b) {
        // null == undefined (and vice versa)
        (JsValue::Null, JsValue::Undefined) | (JsValue::Undefined, JsValue::Null) => true,

        // Number == String → Number == ToNumber(String)
        (JsValue::Number(_), JsValue::String(s)) => {
            let n = string_to_number(&vm.strings.get_utf8(s));
            abstract_eq(vm, a, JsValue::Number(n))?
        }
        (JsValue::String(s), JsValue::Number(_)) => {
            let n = string_to_number(&vm.strings.get_utf8(s));
            abstract_eq(vm, JsValue::Number(n), b)?
        }

        // BigInt == BigInt handled by same_type above.
        // BigInt == Number (§7.2.14 step 5/6)
        (JsValue::BigInt(bi), JsValue::Number(n)) | (JsValue::Number(n), JsValue::BigInt(bi)) => {
            if n.is_nan() || n.is_infinite() {
                return Ok(false);
            }
            if n != n.floor() {
                return Ok(false);
            }
            // Integer Number → compare with BigInt value.
            #[allow(clippy::cast_possible_truncation)]
            if n.abs() < 2.0f64.powi(53) {
                // Fast path: compare via primitive integer conversion, avoiding
                // the temporary BigInt allocation that `from(n as i64)` costs.
                use num_traits::ToPrimitive;
                vm.bigints.get(bi).to_i64() == Some(n as i64)
            } else {
                // Large integer — use string round-trip.
                let Ok(n_big) = format!("{n:.0}").parse::<BigIntValue>() else {
                    return Ok(false);
                };
                vm.bigints.get(bi) == &n_big
            }
        }

        // BigInt == String → parse string as BigInt
        (JsValue::BigInt(bi), JsValue::String(s)) | (JsValue::String(s), JsValue::BigInt(bi)) => {
            let text = vm.strings.get_utf8(s);
            match super::dispatch_helpers::parse_bigint_literal(trim_js(&text)) {
                Some(parsed) => vm.bigints.get(bi) == &parsed,
                None => false,
            }
        }

        // BigInt == Boolean: compare against canonical 0n / 1n without
        // constructing a temporary BigInt.
        (JsValue::BigInt(bi), JsValue::Boolean(bl))
        | (JsValue::Boolean(bl), JsValue::BigInt(bi)) => {
            use num_traits::{One, Zero};
            let v = vm.bigints.get(bi);
            if bl {
                v.is_one()
            } else {
                v.is_zero()
            }
        }

        // Symbol == non-Symbol → false (symbols are unique; same-type
        // comparison is handled by strict_eq above).
        (JsValue::Symbol(_), _) | (_, JsValue::Symbol(_)) => false,

        // Boolean == x → ToNumber(Boolean) == x
        (JsValue::Boolean(bl), _) => {
            let n = if bl { 1.0 } else { 0.0 };
            abstract_eq(vm, JsValue::Number(n), b)?
        }
        (_, JsValue::Boolean(bl)) => {
            let n = if bl { 1.0 } else { 0.0 };
            abstract_eq(vm, a, JsValue::Number(n))?
        }

        // Object == primitive → ? ToPrimitive (§7.2.15 steps 10, 12).
        // Abrupt completion from a user-defined @@toPrimitive / valueOf /
        // toString must propagate per spec `?` mark.
        (JsValue::Object(_), _) => {
            let prim = vm.to_primitive(a, "default")?;
            abstract_eq(vm, prim, b)?
        }
        (_, JsValue::Object(_)) => {
            let prim = vm.to_primitive(b, "default")?;
            abstract_eq(vm, a, prim)?
        }
        _ => false,
    })
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
        JsValue::Empty | JsValue::Undefined => vm.well_known.undefined,
        JsValue::Null => vm.well_known.object_type,
        JsValue::Boolean(_) => vm.well_known.boolean_type,
        JsValue::Number(_) => vm.well_known.number_type,
        JsValue::String(_) => vm.well_known.string_type,
        JsValue::Symbol(_) => vm.well_known.symbol_type,
        JsValue::BigInt(_) => vm.well_known.bigint_type,
        JsValue::Object(id) => {
            if let Some(obj) = vm.objects[id.0 as usize].as_ref() {
                if obj.kind.is_callable() {
                    vm.well_known.function_type
                } else {
                    vm.well_known.object_type
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
/// Maximum prototype chain depth for property lookups.
const PROTO_CHAIN_LIMIT: usize = 10_000;

pub(crate) fn get_property(
    vm: &VmInner,
    obj_id: ObjectId,
    key: PropertyKey,
) -> Option<PropertyResult> {
    let mut current = Some(obj_id);
    for _ in 0..PROTO_CHAIN_LIMIT {
        let Some(id) = current else { break };
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
    for _ in 0..PROTO_CHAIN_LIMIT {
        let Some(id) = current else { break };
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
