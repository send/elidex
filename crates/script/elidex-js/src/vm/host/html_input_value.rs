//! `HTMLInputElement` value-family + step-method bodies — split out
//! of `html_input_proto.rs` (B-5 of T1-v2 followup-cleanup) to keep
//! the input prototype installer under the 1000-line convention.
//!
//! Members covered:
//!
//! - `value` / `defaultValue` accessor pair (HTML §4.10.18.1 "A form
//!   control's value" dirty-value-flag tracking via
//!   `FormControlState.set_value` / `set_value_initial`).
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
// `use elidex_form::FormControlKind;` lives inside the `valueAsNumber`
// accessor bodies — local imports within statements, mirroring the
// parent `html_input_proto.rs`.
#![allow(clippy::items_after_statements)]

use elidex_form::{FormControlState, ValueMode, ValueSetAction};

use super::super::value::{JsValue, NativeContext, VmError};
use super::html_input_proto::require_input_receiver;

// -------------------------------------------------------------------------
// value / defaultValue / checked / defaultChecked / indeterminate
// -------------------------------------------------------------------------

pub(super) fn native_input_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_input_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    // HTML §4.10.5.4 `value` IDL getter — dispatch on the type attribute's
    // value mode.  The mode predicate + per-mode fallback logic live in
    // `elidex-form`; the host marshals the live value / content attribute
    // and interns the result (Layering mandate).  Read the mode + the live
    // value in a single `FormControlState` borrow (matches the boa sibling
    // getter's shape).
    let (mode, live) = {
        let dom = ctx.host().dom();
        match dom.world().get::<&FormControlState>(entity).ok() {
            Some(state) => (state.kind.value_idl_mode(), state.value().to_owned()),
            None => return Ok(JsValue::String(empty)),
        }
    };
    let result = if mode == ValueMode::Value {
        // value mode → the live `FormControlState.value`.
        live
    } else {
        // default / default-on → the `value` content attribute (fallback
        // "" / "on"); filename → "C:\fakepath\" + first selected file name,
        // or "" if the list is empty.  The selected-files list is not yet
        // modeled (`#11-input-file-shell-staging`), so `first_filename` is
        // `None` and the filename getter returns "".
        let dom = ctx.host().dom();
        let content_attr = dom.with_attribute(entity, "value", |v| v.map(str::to_owned));
        mode.idl_get(&live, content_attr.as_deref(), None)
    };
    let sid = ctx.vm.strings.intern(&result);
    Ok(JsValue::String(sid))
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
    // `HTMLInputElement.value` is `[LegacyNullToEmptyString] DOMString`, so a
    // `null` assignment is the empty string — NOT the JS `ToString(null)` =
    // "null".  This must happen BEFORE the value-mode dispatch, so that e.g.
    // `fileInput.value = null` clears the selected files (the empty branch)
    // rather than throwing `InvalidStateError` on the literal "null".
    let s = if matches!(val, JsValue::Null) {
        String::new()
    } else {
        let sid = super::super::coerce::to_string(ctx.vm, val)?;
        ctx.vm.strings.get_utf8(sid)
    };
    let invalid_state_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
    // HTML §4.10.5.4 `value` IDL setter — dispatch on the type attribute's
    // value mode.  `elidex-form` decides the action; the host executes the
    // marshalling (set live value / set content attribute / clear files /
    // raise InvalidStateError) per the Layering mandate.
    //
    // The mode probe ends its `dom` borrow before the match so the
    // `SetContentAttr` arm can route through the `attr_set` shim (record-
    // producing, B2-Slice-2) — the shim takes `ctx`, not a live `dom` borrow.
    // The other arms re-acquire `ctx.host().dom()` locally (their writes are
    // FormControlState mutations, NOT content-attribute writes → no record, I1).
    let Some(mode) = ctx
        .host()
        .dom()
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .map(|state| state.kind.value_idl_mode())
    else {
        return Ok(JsValue::Undefined);
    };
    match mode.idl_set_action(&s) {
        // value mode — §4.10.5.4 steps 1–5 (set live value + dirty flag +
        // sanitize + step-5 cursor).
        ValueSetAction::SetLiveValue => {
            if let Ok(mut state) = ctx
                .host()
                .dom()
                .world_mut()
                .get::<&mut FormControlState>(entity)
            {
                state.set_value(s);
            }
        }
        // default / default-on — set the `value` content attribute via the
        // record-producing `attr_set` chokepoint so the reconciler maintains
        // the derived `default_value` (and, when not dirty, the live value)
        // and the §4.9 "attributes" MutationObserver record is emitted.
        ValueSetAction::SetContentAttr => {
            super::element_attrs::attr_set(ctx, entity, "value", &s);
        }
        // filename mode, empty value — empty the list of selected files.
        // The list is not modeled yet (`#11-input-file-shell-staging`), but a
        // file input can carry a stale live backing value; clear it so the
        // empty set is observable (and not left for form submission).
        ValueSetAction::ClearFiles => {
            if let Ok(mut state) = ctx
                .host()
                .dom()
                .world_mut()
                .get::<&mut FormControlState>(entity)
            {
                state.clear_file_value();
            }
        }
        // filename mode, non-empty value — InvalidStateError.
        ValueSetAction::ThrowInvalidState => {
            return Err(VmError::dom_exception(
                invalid_state_sid,
                "Failed to set the 'value' property on 'HTMLInputElement': \
                 this input element's value may only be set to the empty \
                 string when its type is 'file'.",
            ));
        }
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
    super::element_attrs::attr_set(ctx, entity, "value", &s);
    // Mirror into FormControlState.default_value if not dirty — while
    // the dirty value flag is false, value mirrors the default value
    // (HTML §4.10.18.1 "A form control's value").
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
        super::element_attrs::attr_set(ctx, entity, "checked", "");
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
    // WebIDL `stepUp(optional long n = 1)` — coerce as `long` (ToInt32),
    // not raw ToNumber: this truncates fractions and maps NaN/±Infinity
    // to 0, so a non-integer/non-finite argument can never corrupt the
    // value (cf. `history.go(long delta)`).
    let n = if matches!(
        args.first().copied().unwrap_or(JsValue::Undefined),
        JsValue::Undefined
    ) {
        1
    } else {
        super::super::coerce::to_int32(ctx.vm, args[0])?
    };
    // HTML §4.10.5.4 stepUp/stepDown algorithm hoisted to elidex-form
    // (slot #11-tags-T1-v2-drift-hoist D-2).  VM host/ retains brand
    // check + arg coercion + DOMException construction; the algorithm
    // mutating the FormControlState is engine-independent.
    let dom = ctx.host().dom();
    let result = if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        elidex_form::apply_step(&mut state, f64::from(n), direction)
    } else {
        Ok(())
    };
    if let Err(err) = result {
        // HTML §4.10.5.4 steps 1 & 2 both raise InvalidStateError; the
        // detail message distinguishes the cause.
        let invalid_state_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
        let detail = match err {
            elidex_form::StepError::NotSupported => "This input element does not have stepping",
            elidex_form::StepError::NoAllowedValueStep => {
                "This input element does not have an allowed value step"
            }
        };
        return Err(VmError::dom_exception(
            invalid_state_sid,
            format!("Failed to execute '{method}' on 'HTMLInputElement': {detail}"),
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

// -------------------------------------------------------------------------
// formMethod / formEnctype — HTML §4.10.5.4 enumerated-attribute
// submit-button overrides.  Same keyword sets as `<form>.method` /
// `<form>.enctype`, but missing- and invalid-value defaults are both
// `""` (the no-override sentinel — the form-level value wins when
// the override is absent / invalid).  Distinct from the raw-string
// reflects in `html_input_proto.rs::input_string_attr!`.
// -------------------------------------------------------------------------

pub(super) fn native_input_get_form_enctype(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_input_receiver(ctx, this, "formEnctype")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = super::element_attrs::enumerated_attr_reflect(
        ctx,
        entity,
        "formenctype",
        &[
            "application/x-www-form-urlencoded",
            "multipart/form-data",
            "text/plain",
        ],
        "",
    );
    Ok(JsValue::String(sid))
}

pub(super) fn native_input_set_form_enctype(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "formEnctype")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    super::element_attrs::attr_set(ctx, entity, "formenctype", &s);
    Ok(JsValue::Undefined)
}

pub(super) fn native_input_get_form_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_input_receiver(ctx, this, "formMethod")? else {
        return Ok(JsValue::String(empty));
    };
    let sid = super::element_attrs::enumerated_attr_reflect(
        ctx,
        entity,
        "formmethod",
        &["get", "post", "dialog"],
        "",
    );
    Ok(JsValue::String(sid))
}

pub(super) fn native_input_set_form_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "formMethod")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    super::element_attrs::attr_set(ctx, entity, "formmethod", &s);
    Ok(JsValue::Undefined)
}
