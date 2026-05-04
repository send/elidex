//! Selection API IDL surface shared between
//! HTMLTextAreaElement.prototype (Phase 6) and
//! HTMLInputElement.prototype (Phase 8) — HTML §4.10.18.7.
//!
//! Members installed:
//!
//! - `selectionStart` / `selectionEnd` — `unsigned long` (RW)
//! - `selectionDirection` — DOMString enum `"forward" | "backward" | "none"` (RW)
//! - `select()` — selects all text + sets direction `"none"`.
//! - `setRangeText(replacement, start?, end?, selectMode?)` —
//!   replaces a range of the value and updates the selection.
//! - `setSelectionRange(start, end, direction?)` — updates the
//!   selection slot triple.
//!
//! All six members are installed against the host element's
//! prototype via [`install_selection_api_members`], passing a
//! [`SelectionAccessors`] struct of per-prototype native fn
//! pointers.  Each native is responsible for brand-checking its
//! receiver and materialising the value string, then dispatching
//! into the shared helpers (`get_selection_start`, `set_range_text`,
//! …) below.
//!
//! ## InvalidStateError gating (Phase 8 only)
//!
//! `<input>` selection only applies to "text-control" types
//! (`text` / `search` / `tel` / `url` / `email` / `password`).
//! HTMLInputElement's brand-check returns `Err(InvalidStateError)`
//! for non-text-control input types — see plan §G #6.
//! HTMLTextAreaElement always supports selection, so its brand
//! check passes through whenever the receiver is a `<textarea>`.
//!
//! ## Spec value source
//!
//! Reads / writes go through the IDL-side "value" of the element,
//! which is the dirty value if set and the defaultValue otherwise.
//! The brand-check fn is responsible for routing reads to the
//! correct backing source per element type — for HTMLTextAreaElement
//! the defaultValue is the textContent; for HTMLInputElement
//! (Phase 8) it is the `value` content attribute.

#![cfg(feature = "engine")]

use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::{NativeFn, VmInner};
use super::form_control_state::{utf16_len, utf16_offset_to_utf8, SelectionDirection};

/// Install the six Selection API members on `proto_id`.  Each
/// per-prototype caller assembles the [`SelectionAccessors`] bundle
/// of native fn pointers, where every entry is a dedicated wrapper
/// fn that brand-checks the receiver before dispatching into the
/// shared helpers in this module.  Keeping the brand check in the
/// per-prototype native (rather than passing a closure here) lets
/// rust-friendly `fn` pointers stay first-class without runtime
/// allocation.
pub(super) fn install_selection_api_members(
    vm: &mut VmInner,
    proto_id: super::super::value::ObjectId,
    accessors: SelectionAccessors,
) {
    vm.install_accessor_pair(
        proto_id,
        vm.well_known.selection_start,
        accessors.get_start,
        Some(accessors.set_start),
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        vm.well_known.selection_end,
        accessors.get_end,
        Some(accessors.set_end),
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        vm.well_known.selection_direction,
        accessors.get_direction,
        Some(accessors.set_direction),
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_native_method(
        proto_id,
        vm.well_known.select_method,
        accessors.select,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        vm.well_known.set_range_text,
        accessors.set_range_text,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        vm.well_known.set_selection_range,
        accessors.set_selection_range,
        shape::PropertyAttrs::METHOD,
    );
}

/// Bundle of per-prototype native fn pointers — assembled at
/// register time by each prototype caller.  The functions hard-code
/// their brand-check at install time so [`install_selection_api_members`]
/// stays brand-agnostic.
pub(super) struct SelectionAccessors {
    pub(super) get_start: NativeFn,
    pub(super) set_start: NativeFn,
    pub(super) get_end: NativeFn,
    pub(super) set_end: NativeFn,
    pub(super) get_direction: NativeFn,
    pub(super) set_direction: NativeFn,
    pub(super) select: NativeFn,
    pub(super) set_range_text: NativeFn,
    pub(super) set_selection_range: NativeFn,
}

// -------------------------------------------------------------------------
// Shared accessor implementations — invoked from per-prototype natives
// that bind a `brand_check` closure
// -------------------------------------------------------------------------

/// `selectionStart` getter — returns the stored offset (or `0` for
/// untouched controls).
pub(super) fn get_selection_start(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
) -> Result<JsValue, VmError> {
    let n = ctx
        .vm
        .form_control_state(entity)
        .map_or(0, |s| s.selection_start);
    Ok(JsValue::Number(f64::from(n)))
}

/// `selectionStart` setter — clamps to the value's UTF-16 length;
/// adjusts `selection_end` upward when the new start exceeds it
/// (HTML §4.10.18.7 step 4).  Takes `value_len` directly rather than
/// a `&str` so the caller can compute the length without
/// materialising an owned String when only the count is needed.
pub(super) fn set_selection_start(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    value_len: u32,
    arg: JsValue,
) -> Result<JsValue, VmError> {
    let n = super::super::coerce::to_uint32(ctx.vm, arg)?;
    let clamped = n.min(value_len);
    let state = ctx.vm.form_control_state_mut(entity);
    state.selection_start = clamped;
    if state.selection_end < clamped {
        state.selection_end = clamped;
    }
    Ok(JsValue::Undefined)
}

/// `selectionEnd` getter.
pub(super) fn get_selection_end(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
) -> Result<JsValue, VmError> {
    let n = ctx
        .vm
        .form_control_state(entity)
        .map_or(0, |s| s.selection_end);
    Ok(JsValue::Number(f64::from(n)))
}

/// `selectionEnd` setter — clamps to the value's UTF-16 length and
/// to `>= selection_start`.
pub(super) fn set_selection_end(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    value_len: u32,
    arg: JsValue,
) -> Result<JsValue, VmError> {
    let n = super::super::coerce::to_uint32(ctx.vm, arg)?;
    let clamped = n.min(value_len);
    let state = ctx.vm.form_control_state_mut(entity);
    state.selection_end = clamped.max(state.selection_start);
    Ok(JsValue::Undefined)
}

/// `selectionDirection` getter — string roundtrip per HTML §4.10.18.7.
pub(super) fn get_selection_direction(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
) -> Result<JsValue, VmError> {
    let dir = ctx
        .vm
        .form_control_state(entity)
        .map_or(SelectionDirection::None, |s| s.selection_direction);
    let sid = match dir {
        SelectionDirection::Forward => ctx.vm.well_known.forward_str,
        SelectionDirection::Backward => ctx.vm.well_known.backward_str,
        SelectionDirection::None => ctx.vm.well_known.none_str,
    };
    Ok(JsValue::String(sid))
}

/// `selectionDirection` setter — invalid keywords map to `"none"`.
pub(super) fn set_selection_direction(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    arg: JsValue,
) -> Result<JsValue, VmError> {
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let s = ctx.vm.strings.get_utf8(sid);
    let dir = SelectionDirection::parse(&s);
    ctx.vm.form_control_state_mut(entity).selection_direction = dir;
    Ok(JsValue::Undefined)
}

/// `select()` — selects all text per HTML §4.10.18.7.
pub(super) fn select_all(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    value_len: u32,
) -> Result<JsValue, VmError> {
    let state = ctx.vm.form_control_state_mut(entity);
    state.selection_start = 0;
    state.selection_end = value_len;
    state.selection_direction = SelectionDirection::None;
    Ok(JsValue::Undefined)
}

/// `setSelectionRange(start, end, direction?)` — HTML §4.10.18.7.
/// `start > end` clamps `end := start` per spec step 4.  Missing
/// direction defaults to `"none"`.
pub(super) fn set_selection_range(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    value_len: u32,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let start_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let end_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let dir_arg = args.get(2).copied();
    let start = super::super::coerce::to_uint32(ctx.vm, start_arg)?;
    let end = super::super::coerce::to_uint32(ctx.vm, end_arg)?;
    let mut start_clamped = start.min(value_len);
    let mut end_clamped = end.min(value_len);
    if start_clamped > end_clamped {
        end_clamped = start_clamped;
    }
    let dir = match dir_arg {
        Some(v) if !matches!(v, JsValue::Undefined) => {
            let sid = super::super::coerce::to_string(ctx.vm, v)?;
            let s = ctx.vm.strings.get_utf8(sid);
            SelectionDirection::parse(&s)
        }
        _ => SelectionDirection::None,
    };
    // Adjust if the explicit start was somehow beyond max (already
    // clamped above; left as defence against future refactors).
    if end_clamped < start_clamped {
        start_clamped = end_clamped;
    }
    let state = ctx.vm.form_control_state_mut(entity);
    state.selection_start = start_clamped;
    state.selection_end = end_clamped;
    state.selection_direction = dir;
    Ok(JsValue::Undefined)
}

/// `setRangeText(replacement, start?, end?, selectMode?)` — HTML
/// §4.10.18.7.  Computes a new value by splicing `replacement` into
/// the value at the given range, then updates the selection per
/// `selectMode` (default `"preserve"`).  Returns the resulting value
/// string so the caller can store it in the dirty-value slot via
/// the brand-check's value-source convention.
///
/// 2-argument form (`setRangeText(replacement)`) uses the current
/// selection range.  3+ argument forms require both `start` and
/// `end`; `start > end` throws `IndexSizeError` per spec.
///
/// Returns `Ok((new_value, new_start, new_end))` on success.  The
/// caller writes `new_value` into the dirty-value slot and stores
/// the selection.
pub(super) fn compute_set_range_text(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    value: &str,
    args: &[JsValue],
) -> Result<(String, u32, u32), VmError> {
    let replacement_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let replacement_sid = super::super::coerce::to_string(ctx.vm, replacement_arg)?;
    let replacement = ctx.vm.strings.get_utf8(replacement_sid);

    let max = utf16_len(value);
    let (start, end) = if args.len() < 2 {
        // 1-argument form — use current selection range.
        let state = ctx.vm.form_control_state(entity);
        (
            state.map_or(0, |s| s.selection_start),
            state.map_or(0, |s| s.selection_end),
        )
    } else {
        let start_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let end_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        let start = super::super::coerce::to_uint32(ctx.vm, start_arg)?;
        let end = super::super::coerce::to_uint32(ctx.vm, end_arg)?;
        if start > end {
            // HTML §4.10.18.7 step 2 — IndexSizeError when start > end.
            return Err(VmError::dom_exception(
                ctx.vm.well_known.dom_exc_invalid_state_error,
                format!("setRangeText: start ({start}) is greater than end ({end})"),
            ));
        }
        (start.min(max), end.min(max))
    };

    let select_mode = args.get(3).copied().unwrap_or(JsValue::Undefined);
    let mode_str: String = match select_mode {
        JsValue::Undefined => "preserve".to_string(),
        v => {
            let sid = super::super::coerce::to_string(ctx.vm, v)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };

    // Splice replacement into the value at [start, end) — UTF-16
    // offsets need conversion to UTF-8 byte offsets first.
    let start_byte = utf16_offset_to_utf8(value, start);
    let end_byte = utf16_offset_to_utf8(value, end);
    let mut new_value = String::with_capacity(value.len() + replacement.len());
    new_value.push_str(&value[..start_byte]);
    new_value.push_str(&replacement);
    new_value.push_str(&value[end_byte..]);

    let replacement_units = utf16_len(&replacement);
    let new_length = utf16_len(&new_value);

    // Compute new selection per `selectMode` (HTML §4.10.18.7 step
    // 11).  We compute against the new value's UTF-16 length so
    // every mode ends up with bounds inside the new value.
    let (new_start, new_end) = match mode_str.as_str() {
        "select" => (
            start,
            start.saturating_add(replacement_units).min(new_length),
        ),
        "start" => (start, start),
        "end" => {
            let pos = start.saturating_add(replacement_units).min(new_length);
            (pos, pos)
        }
        // "preserve" (default) — HTML §4.10.18.7 "preserve":
        // - If sel > end:   sel += delta.
        // - Else if sel > start: sel := start.
        // - Else:           unchanged.
        // Both bounds are STRICT inequalities; the collapse target
        // is the original `start` (not `start + new length`).  This
        // intentionally surfaces "selection at the right edge of
        // the replaced range" as a collapse to `start`, matching
        // browser behaviour.
        _ => {
            let prev_state = ctx.vm.form_control_state(entity);
            let cur_start = prev_state.map_or(0, |s| s.selection_start);
            let cur_end = prev_state.map_or(0, |s| s.selection_end);
            let edit_old_len = end - start;
            let edit_new_len = replacement_units;
            let delta = i64::from(edit_new_len) - i64::from(edit_old_len);
            let adjust = |off: u32| -> u32 {
                if off > end {
                    let v = i64::from(off) + delta;
                    u32::try_from(v.max(0)).unwrap_or(u32::MAX).min(new_length)
                } else if off > start {
                    start
                } else {
                    off
                }
            };
            (adjust(cur_start), adjust(cur_end))
        }
    };

    Ok((new_value, new_start, new_end))
}
