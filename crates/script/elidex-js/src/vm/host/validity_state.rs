//! `ValidityState` wrapper + `ConstraintValidation` mixin install
//! (HTML §4.10.20.3 / §4.10.20.4).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  `validate_control()` lives
//! in [`elidex_form::validation`]; this module exposes the result
//! to JS via 11 boolean accessors.  Custom validity is stored on
//! [`elidex_form::FormControlState::custom_validity_message`] —
//! no standalone HashMap.
//!
//! The `[SameObject]` cache lives in
//! [`super::super::VmInner::validity_state_wrappers`]; sweep-pruned
//! weak-through-owner.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, VmInner};

use elidex_ecs::Entity;
use elidex_form::FormControlState;

impl VmInner {
    /// Allocate `ValidityState.prototype` chained to
    /// `Object.prototype` and install the 11 boolean accessors.
    pub(in crate::vm) fn register_validity_state_prototype(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_validity_state_prototype called before register_prototypes");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.validity_state_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (name_sid, getter) in [
            (
                self.well_known.value_missing,
                native_validity_value_missing as NativeFn,
            ),
            (self.well_known.type_mismatch, native_validity_type_mismatch),
            (
                self.well_known.pattern_mismatch,
                native_validity_pattern_mismatch,
            ),
            (self.well_known.too_short, native_validity_too_short),
            (self.well_known.too_long, native_validity_too_long),
            (
                self.well_known.range_underflow,
                native_validity_range_underflow,
            ),
            (
                self.well_known.range_overflow,
                native_validity_range_overflow,
            ),
            (self.well_known.step_mismatch, native_validity_step_mismatch),
            (self.well_known.bad_input, native_validity_bad_input),
            (self.well_known.custom_error, native_validity_custom_error),
            (self.well_known.valid_attr, native_validity_valid),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, None, attrs);
        }
    }

    /// Install the ConstraintValidation mixin (`validity` /
    /// `validationMessage` / `willValidate` / `checkValidity` /
    /// `reportValidity` / `setCustomValidity`) on `proto_id`.
    /// Called from each form-control prototype's `register_*`
    /// after its own accessors land.
    pub(in crate::vm) fn install_constraint_validation_mixin(&mut self, proto_id: ObjectId) {
        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        self.install_accessor_pair(
            proto_id,
            self.well_known.validity,
            native_get_validity,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.validation_message,
            native_get_validation_message,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.will_validate,
            native_get_will_validate,
            None,
            attrs,
        );
        let m = shape::PropertyAttrs::METHOD;
        self.install_native_method(
            proto_id,
            self.well_known.check_validity,
            native_check_validity,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.report_validity,
            native_report_validity,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_custom_validity,
            native_set_custom_validity,
            m,
        );
    }
}

// ---------------------------------------------------------------------------
// ValidityState — brand check + accessors
// ---------------------------------------------------------------------------

/// Recover the owning form-control entity from a `ValidityState`
/// receiver.
fn require_validity_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to read '{method}' from 'ValidityState': Illegal invocation"
        )));
    };
    let ObjectKind::ValidityState { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(VmError::type_error(format!(
            "Failed to read '{method}' from 'ValidityState': Illegal invocation"
        )));
    };
    Entity::from_bits(entity_bits)
        .ok_or_else(|| VmError::type_error("ValidityState: invalid entity"))
}

/// Run validate_control on the owner's FormControlState and pass
/// the validity to `f`.  Defaults to `false` when the owner has no
/// FormControlState.
fn with_validity<F: FnOnce(&elidex_form::ValidityState) -> bool>(
    ctx: &mut NativeContext<'_>,
    method: &str,
    this: JsValue,
    f: F,
) -> Result<JsValue, VmError> {
    let entity = require_validity_receiver(ctx, this, method)?;
    let dom = ctx.host().dom();
    let state = dom.world().get::<&FormControlState>(entity).ok();
    let validity = state.as_ref().map(|s| elidex_form::validate_control(s));
    Ok(JsValue::Boolean(validity.as_ref().is_some_and(f)))
}

fn native_validity_value_missing(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "valueMissing", this, |v| v.value_missing)
}

fn native_validity_type_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "typeMismatch", this, |v| v.type_mismatch)
}

fn native_validity_pattern_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "patternMismatch", this, |v| v.pattern_mismatch)
}

fn native_validity_too_short(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "tooShort", this, |v| v.too_short)
}

fn native_validity_too_long(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "tooLong", this, |v| v.too_long)
}

fn native_validity_range_underflow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "rangeUnderflow", this, |v| v.range_underflow)
}

fn native_validity_range_overflow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "rangeOverflow", this, |v| v.range_overflow)
}

fn native_validity_step_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "stepMismatch", this, |v| v.step_mismatch)
}

fn native_validity_bad_input(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "badInput", this, |v| v.bad_input)
}

fn native_validity_custom_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "customError", this, |v| v.custom_error)
}

fn native_validity_valid(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "valid", this, elidex_form::ValidityState::is_valid)
}

// ---------------------------------------------------------------------------
// ConstraintValidation mixin natives
// ---------------------------------------------------------------------------

/// Recover the form-control entity from a `this` receiver that
/// could be any of the 5 control kinds (Input / Select / TextArea /
/// Button / FieldSet).  We accept any element wrapper (HostObject
/// kind) — the validate_control path inside the helper is the
/// authoritative gate.
/// Tags on which the ConstraintValidation mixin is installed
/// (Phase 9): `<input>`, `<select>`, `<textarea>`, `<button>`,
/// `<fieldset>`.  Cross-tag receivers (e.g.
/// `HTMLInputElement.prototype.checkValidity.call(div)`) must throw
/// TypeError per WebIDL "Illegal invocation".
///
/// `<output>` is a constraint-validation candidate per
/// HTML §4.10.10.1 but its HTMLOutputElement prototype is out of
/// scope for T1-v2 — the brand check excludes it until a follow-up
/// slot adds the output prototype install alongside the mixin
/// install.  `<form>` has its own delegate-to-children
/// checkValidity / reportValidity bodies on `HTMLFormElement.prototype`
/// (R2) that use `require_form_receiver`, so it does not flow
/// through this brand check either.
fn is_constraint_validation_host_tag(dom: &elidex_ecs::EcsDom, entity: Entity) -> bool {
    let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(entity) else {
        return false;
    };
    let s = tag.0.as_str();
    ["input", "select", "textarea", "button", "fieldset"]
        .iter()
        .any(|t| s.eq_ignore_ascii_case(t))
}

/// Resolve the form-control entity the ConstraintValidation mixin
/// methods/accessors target.  Convention follows
/// [`super::event_target::require_receiver`]:
///
/// - Non-Object / non-HostObject receivers → `Ok(None)` (callers
///   return the trivial default value); per WebIDL strict reading
///   these would throw, but the codebase has settled on a quieter
///   shape so detached prototype calls don't surface as JS errors.
/// - HostObject whose entity does not carry a form-control tag →
///   `Err(TypeError)` ("Illegal invocation" — this IS a brand
///   mismatch on a wrapper, where throwing matches the WebIDL
///   contract used by the rest of `validity_state.rs`).
fn require_form_control_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let JsValue::Object(id) = this else {
        return Ok(None);
    };
    let entity = match ctx.vm.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => {
            let Some(e) = Entity::from_bits(entity_bits) else {
                return Ok(None);
            };
            e
        }
        _ => return Ok(None),
    };
    // Brand check — even though the wrapper is a HostObject, the
    // entity it points at must carry one of the form-control tags
    // the ConstraintValidation mixin is installed on.  Without this,
    // `HTMLInputElement.prototype.checkValidity.call(div)` would
    // succeed instead of throwing.
    if !is_constraint_validation_host_tag(ctx.host().dom(), entity) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn alloc_validity_wrapper(vm: &mut VmInner, entity: Entity) -> ObjectId {
    if let Some(&existing) = vm.validity_state_wrappers.get(&entity) {
        return existing;
    }
    let proto = vm
        .validity_state_prototype
        .expect("alloc_validity_wrapper before register_validity_state_prototype");
    let id = vm.alloc_object(Object {
        kind: ObjectKind::ValidityState {
            entity_bits: entity.to_bits().get(),
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: false,
    });
    vm.validity_state_wrappers.insert(entity, id);
    id
}

fn native_get_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_control_receiver(ctx, this, "validity")? else {
        return Ok(JsValue::Null);
    };
    let id = alloc_validity_wrapper(ctx.vm, entity);
    Ok(JsValue::Object(id))
}

fn native_get_validation_message(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_form_control_receiver(ctx, this, "validationMessage")? else {
        return Ok(JsValue::String(empty));
    };
    let dom = ctx.host().dom();
    let msg = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .map_or_else(String::new, |state| {
            let v = elidex_form::validate_control(&state);
            // Custom error wins; otherwise produce a stub
            // diagnostic from the failed flag.  Browser-quality
            // localised messages are deferred to the UA shell.
            if v.custom_error {
                v.custom_error_message
            } else if !v.is_valid() {
                "Constraint validation failed".to_string()
            } else {
                String::new()
            }
        });
    // Skip interning when empty — `well_known.empty` is the canonical
    // empty StringId; otherwise re-interning pollutes the table.
    let sid = if msg.is_empty() {
        empty
    } else {
        ctx.vm.strings.intern(&msg)
    };
    Ok(JsValue::String(sid))
}

fn native_get_will_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_control_receiver(ctx, this, "willValidate")? else {
        return Ok(JsValue::Boolean(false));
    };
    let dom = ctx.host().dom();
    // HTML §4.10.20.4 step "candidate for constraint validation":
    // submittable, not disabled, not hidden type, not in disabled
    // fieldset.  We use FormControlState for kind/disabled and
    // `is_fieldset_disabled` for ancestor walk.
    let candidate = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_some_and(|state| {
            use elidex_form::FormControlKind;
            if !state.kind.is_submittable() || state.disabled {
                return false;
            }
            if matches!(state.kind, FormControlKind::Hidden) {
                return false;
            }
            // Check ancestor fieldset disabled state.
            !elidex_form::is_fieldset_disabled(entity, dom)
        });
    Ok(JsValue::Boolean(candidate))
}

fn native_check_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_control_receiver(ctx, this, "checkValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    let dom = ctx.host().dom();
    let valid = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_none_or(|state| {
            // Spec: a control that is not a candidate for constraint
            // validation always returns true (HTML §4.10.20.3 list of
            // "barred from constraint validation" exclusions).  Beyond
            // `!is_submittable()` (already excludes button-typed
            // inputs / Output / Meter / Progress) and `disabled`, the
            // spec also bars `<input type=hidden>` and `<output>`.
            // `Hidden` is the only kind that is_submittable but barred,
            // so call it out explicitly.
            use elidex_form::FormControlKind;
            if !state.kind.is_submittable()
                || state.disabled
                || matches!(state.kind, FormControlKind::Hidden)
            {
                return true;
            }
            elidex_form::validate_control(&state).is_valid()
        });
    if !valid {
        // HTML §4.10.20.4 step 1.b: fire a synthetic `invalid`
        // event at the element (cancelable=true, bubbles=false).
        // Listeners can `preventDefault()` to suppress the UA's
        // default validation reporting; `checkValidity()` itself
        // still returns `false` regardless.
        let invalid_sid = ctx.vm.well_known.invalid_event;
        let _ = super::event_target_dispatch::dispatch_simple_event(
            ctx,
            entity,
            invalid_sid,
            /*bubbles=*/ false,
            /*cancelable=*/ true,
        )?;
    }
    Ok(JsValue::Boolean(valid))
}

fn native_report_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Headless mode — no UA validation popup.  Returns
    // checkValidity's result (HTML §4.10.20.4 step 3 fallback).
    native_check_validity(ctx, this, args)
}

fn native_set_custom_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_control_receiver(ctx, this, "setCustomValidity")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.custom_validity_message = if s.is_empty() { None } else { Some(s) };
    }
    Ok(JsValue::Undefined)
}
