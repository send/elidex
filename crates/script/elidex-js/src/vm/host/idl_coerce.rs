//! IDL argument-coercion helpers shared across per-tag prototype
//! setters (slot `#11-tags-T2b-passive` and beyond).
//!
//! Dedicated home for the cross-element `long` / `unsigned long` /
//! string IDL setter coercion logic so per-tag files stay focused on
//! their accessor surface rather than re-implementing the shape
//! dispatch for each new long-typed reflect.
//!
//! ## Layering
//!
//! Engine-bound thin marshalling layer.  The actual saturation
//! semantics live in
//! [`elidex_dom_api::element::numeric_reflect::js_number_to_i32_saturating`]
//! (engine-indep); this module's responsibility is routing JS argument
//! values through the spec ToNumber pipeline (which can invoke
//! user-defined `valueOf` / `toString`) and then handing the resulting
//! `f64` to the saturating cast.
//!
//! Non-`engine` builds skip this module entirely (matches the
//! `html_*_proto` per-tag files which all live behind
//! `#![cfg(feature = "engine")]`).

#![cfg(feature = "engine")]

use elidex_dom_api::element::numeric_reflect::js_number_to_i32_saturating;

use super::super::coerce::to_number;
use super::super::value::{JsValue, NativeContext, VmError};

/// Convert the first argument of an IDL `long` reflect setter to its
/// content-attribute serialisation (base-10 decimal `i32`).
///
/// Routes through ECMAScript ToNumber per WebIDL §3.10.7 (which fires
/// `@@toPrimitive` / `valueOf` / `toString` on objects), then
/// saturates at the i32 bound via the engine-indep helper.  This
/// matches Chromium / Firefox observable behaviour for
/// `<ol>.start = 1e20` (= `i32::MAX`) and for
/// `<ol>.start = {valueOf: () => 5.7}` (= `5`, not `"5.7"` —
/// `to_number` truncates the fractional part via the saturating cast).
///
/// `Symbol` and `BigInt` arguments throw `TypeError` per spec, raised
/// from `to_number`.
pub(super) fn serialise_long_idl_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<String, VmError> {
    let raw = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = to_number(ctx.vm, raw)?;
    Ok(js_number_to_i32_saturating(n).to_string())
}
