//! `HTMLInputElement` Selection API wrappers — split out of
//! `html_input_proto.rs` (B-5 of T1-v2 followup-cleanup) alongside
//! the Selection-body hoist (B-6) so the `<input>` thin-wrapper
//! layer over `vm/host/selection_api.rs` lives in one focused file.
//!
//! Members covered: `selectionStart` / `selectionEnd` /
//! `selectionDirection` accessor pairs and `select()` /
//! `setSelectionRange()` / `setRangeText()` methods (HTML
//! §4.10.5.2.10).
//!
//! All entry points share the same shape:
//!
//! 1. [`require_input_receiver`] for `<input>` brand check,
//! 2. [`super::selection_api::require_text_control`] to reject
//!    non-text input types with `InvalidStateError`, and
//! 3. forward to the corresponding shared body in
//!    `super::selection_api`.
//!
//! See the textarea sibling in `html_textarea_proto.rs` for the
//! mirrored thin-wrapper layer.

#![cfg(feature = "engine")]

use elidex_ecs::Entity;

use super::super::value::{JsValue, NativeContext, VmError};
use super::html_input_proto::require_input_receiver;

const INPUT_INTERFACE: &str = "HTMLInputElement";
const INPUT_ELEM_LABEL: &str = "input element";

fn input_check(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
        return Ok(None);
    };
    super::selection_api::require_text_control(
        ctx,
        entity,
        method,
        INPUT_INTERFACE,
        INPUT_ELEM_LABEL,
    )?;
    Ok(Some(entity))
}

pub(super) fn native_input_get_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Null);
    };
    Ok(super::selection_api::get_selection_start(ctx, entity))
}

pub(super) fn native_input_set_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_start(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Null);
    };
    Ok(super::selection_api::get_selection_end(ctx, entity))
}

pub(super) fn native_input_set_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_end(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Null);
    };
    Ok(super::selection_api::get_selection_direction(ctx, entity))
}

pub(super) fn native_input_set_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_direction(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_select_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `select()` applies to the text-like states plus number and the
    // date/time states, and is a no-op (never an error) for other kinds —
    // HTML "select() method", step 1.  So it gates on
    // `select_method_applies` rather than the throwing `require_text_control`
    // the selectionStart/setSelectionRange/setRangeText APIs use.
    let Some(entity) = require_input_receiver(ctx, this, "select")? else {
        return Ok(JsValue::Undefined);
    };
    if super::selection_api::select_method_applies(ctx, entity) {
        super::selection_api::select_all(ctx, entity);
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_set_selection_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "setSelectionRange")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_range(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_set_range_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = input_check(ctx, this, "setRangeText")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_range_text(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}
