//! `HTMLInputElement` value-family + step-method bodies — split out
//! of `html_input_proto.rs` (B-5 of T1-v2 followup-cleanup) to keep
//! the input prototype installer under the 1000-line convention.
//!
//! Members covered:
//!
//! - `value` / `defaultValue` accessor pair (HTML §4.10.5.1.6 dirty
//!   tracking via `FormControlState.set_value` / `set_value_initial`).
//! - `checked` / `defaultChecked` / `indeterminate` round-trip
//!   accessors.
//! - `valueAsNumber` / `valueAsDate` (Date support is the
//!   `#11-input-value-as-date` defer slot — getter null / setter
//!   accepts only null).
//! - `stepUp(n?)` / `stepDown(n?)` thin wrappers — the algorithm
//!   itself is `elidex_form::apply_step` (drift-hoist D-2).
//!
//! All members reuse `html_input_proto::require_input_receiver` for
//! brand check; this module is otherwise free-standing marshalling
//! glue between WebIDL coercions and `FormControlState` reads /
//! writes per CLAUDE.md "Layering mandate".

#![cfg(feature = "engine")]
// Cast-sign-loss / cast-truncation: WebIDL coercions clamp before
// the cast.  Module-wide allow matches `html_input_proto.rs`.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
// Reads through `Result<&FormControlState>` follow the same shared-
// borrow pattern as the parent file.
#![allow(clippy::map_unwrap_or)]

use elidex_ecs::Entity;
use elidex_form::FormControlState;

use super::super::value::{JsValue, NativeContext, VmError};
use super::html_input_proto::require_input_receiver;

// -------------------------------------------------------------------------
// value / defaultValue / checked / defaultChecked / indeterminate
// -------------------------------------------------------------------------

/// Read `FormControlState.value` for the input entity.  Returns
/// `""` if the state is missing (not a recognised form control).
fn read_state_value(ctx: &mut NativeContext<'_>, entity: Entity) -> JsValue {
    let empty = ctx.vm.well_known.empty;
    let dom = ctx.host().dom();
    let Some(state) = dom.world().get::<&FormControlState>(entity).ok() else {
        return JsValue::String(empty);
    };
    let v = state.value().to_owned();
    drop(state);
    let sid = ctx.vm.strings.intern(&v);
    JsValue::String(sid)
}

pub(super) fn native_input_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    Ok(read_state_value(ctx, entity))
}

pub(super) fn native_input_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.set_value(s);
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_input_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => {
            dom.with_attribute(entity, "value", |v| v.map_or(empty, |s| strings.intern(s)))
        }
        None => empty,
    };
    Ok(JsValue::String(sid))
}

pub(super) fn native_input_set_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "value", s.clone());
    // Mirror into FormControlState.default_value if not dirty —
    // matches HTML §4.10.5.1.7 step "default value mode".
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.default_value.clone_from(&s);
        if !state.is_dirty() {
            state.set_value_initial(s);
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_checked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "checked")? else {
        return Ok(JsValue::Boolean(false));
    };
    let dom = ctx.host().dom();
    let checked = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.checked)
        .unwrap_or(false);
    Ok(JsValue::Boolean(checked))
}

pub(super) fn native_input_set_checked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "checked")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.checked = flag;
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_default_checked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "defaultChecked")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "checked"),
    ))
}

pub(super) fn native_input_set_default_checked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "defaultChecked")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "checked", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "checked");
    }
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.default_checked = flag;
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_indeterminate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "indeterminate")? else {
        return Ok(JsValue::Boolean(false));
    };
    let dom = ctx.host().dom();
    let flag = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.indeterminate)
        .unwrap_or(false);
    Ok(JsValue::Boolean(flag))
}

pub(super) fn native_input_set_indeterminate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "indeterminate")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.indeterminate = flag;
    }
    Ok(JsValue::Undefined)
}

// -------------------------------------------------------------------------
// valueAsNumber / valueAsDate
// -------------------------------------------------------------------------

pub(super) fn native_input_get_value_as_number(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "valueAsNumber")? else {
        return Ok(JsValue::Number(f64::NAN));
    };
    let dom = ctx.host().dom();
    let Some(state) = dom.world().get::<&FormControlState>(entity).ok() else {
        return Ok(JsValue::Number(f64::NAN));
    };
    use elidex_form::FormControlKind;
    let parsed = match state.kind {
        FormControlKind::Number | FormControlKind::Range => state.value().parse::<f64>().ok(),
        _ => None,
    };
    Ok(JsValue::Number(parsed.unwrap_or(f64::NAN)))
}

pub(super) fn native_input_set_value_as_number(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "valueAsNumber")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Number(n) = val else {
        return Err(VmError::type_error(
            "valueAsNumber: argument is not a number".to_string(),
        ));
    };
    if !n.is_finite() {
        return Err(VmError::type_error(
            "valueAsNumber: non-finite values are not allowed".to_string(),
        ));
    }
    let invalid_state_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        use elidex_form::FormControlKind;
        match state.kind {
            FormControlKind::Number | FormControlKind::Range => {
                state.set_value(n.to_string());
            }
            _ => {
                drop(state);
                return Err(VmError::dom_exception(
                    invalid_state_sid,
                    "valueAsNumber: input type does not support number conversion",
                ));
            }
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_value_as_date(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Stub — Date IDL integration deferred to
    // `#11-input-value-as-date`.
    let _ = require_input_receiver(ctx, this, "valueAsDate")?;
    Ok(JsValue::Null)
}

pub(super) fn native_input_set_value_as_date(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_input_receiver(ctx, this, "valueAsDate")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(val, JsValue::Null) {
        return Ok(JsValue::Undefined);
    }
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_invalid_state_error,
        "valueAsDate: only null is accepted (Date integration not yet supported)",
    ))
}

// -------------------------------------------------------------------------
// stepUp / stepDown
// -------------------------------------------------------------------------

fn step_apply(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    direction: f64,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let n = if matches!(
        args.first().copied().unwrap_or(JsValue::Undefined),
        JsValue::Undefined
    ) {
        1.0
    } else {
        super::super::coerce::to_number(ctx.vm, args[0])?
    };
    // HTML §4.10.5.4 stepUp/stepDown algorithm hoisted to elidex-form
    // (slot #11-tags-T1-v2-drift-hoist D-2).  VM host/ retains brand
    // check + arg coercion + DOMException construction; the algorithm
    // mutating the FormControlState is engine-independent.
    let dom = ctx.host().dom();
    let result = if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        elidex_form::apply_step(&mut state, n, direction)
    } else {
        Ok(())
    };
    if let Err(elidex_form::StepError::NotSupported) = result {
        let invalid_state_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
        return Err(VmError::dom_exception(
            invalid_state_sid,
            format!(
                "Failed to execute '{method}' on 'HTMLInputElement': \
                 This input element does not have stepping"
            ),
        ));
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_step_up(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    step_apply(ctx, this, args, "stepUp", 1.0)
}

pub(super) fn native_input_step_down(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    step_apply(ctx, this, args, "stepDown", -1.0)
}
