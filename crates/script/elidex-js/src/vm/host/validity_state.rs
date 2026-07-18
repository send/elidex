//! `ValidityState` wrapper + `ConstraintValidation` mixin install
//! (HTML §4.10.20.3 / §4.10.20.4).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  `validate_control` lives in the
//! engine-independent **leaf** crate `elidex_form_core` (re-exported by
//! the `elidex_form` facade, which still depends on the higher form
//! systems — `elidex-dom-api` / `elidex-script-session`; host code calls
//! it as `elidex_form::validate_control`); this module exposes the result
//! to JS via 11 boolean accessors.  Custom validity is stored on
//! [`elidex_form::FormControlState::custom_validity_message`] —
//! no standalone HashMap.
//!
//! The `[SameObject]` wrapper is interned in the unified wrapper
//! store under `WrapperKind::ValidityState`; sweep-pruned
//! weak-through-owner.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
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
            "Failed to execute '{method}' on 'ValidityState': Illegal invocation"
        )));
    };
    let ObjectKind::ValidityState { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'ValidityState': Illegal invocation"
        )));
    };
    Entity::from_bits(entity_bits)
        .ok_or_else(|| VmError::type_error("ValidityState: invalid entity"))
}

/// Run `validate_control` on the owner's `FormControlState` and pass
/// the validity to `f`.  Per HTML §4.10.20.3 the constraint-
/// validation algorithm is skipped on controls that are barred from
/// validation (no FormControlState, `!is_submittable()`, `disabled`,
/// `<input type=hidden>`, descendant of disabled `<fieldset>`).
/// For barred controls the spec leaves the stored ValidityState
/// bits at their initialised values (all false), so this getter
/// returns `default_when_no_state` — `false` for the anchor flags
/// (`valueMissing` / `tooLong` / …) and `true` for the aggregate
/// `valid` getter — without running `validate_control`.  Browsers
/// behave the same way: `disabled.validity.valueMissing` is `false`
/// even when `required=true`.
fn with_validity<F: FnOnce(&elidex_form::ValidityState) -> bool>(
    ctx: &mut NativeContext<'_>,
    method: &str,
    this: JsValue,
    default_when_no_state: bool,
    f: F,
) -> Result<JsValue, VmError> {
    let entity = require_validity_receiver(ctx, this, method)?;
    let dom = ctx.host().dom();
    let Ok(state) = dom.world().get::<&FormControlState>(entity) else {
        return Ok(JsValue::Boolean(default_when_no_state));
    };
    if !elidex_form::is_constraint_validation_candidate(&state, entity, dom) {
        return Ok(JsValue::Boolean(default_when_no_state));
    }
    let validity = elidex_form::validate_control(&state);
    Ok(JsValue::Boolean(f(&validity)))
}

// `is_constraint_validation_candidate` (HTML §4.10.20.3) hoisted to
// `elidex_form::is_constraint_validation_candidate` per CLAUDE.md
// "Layering mandate" — pure spec predicate belongs in the
// engine-independent crate.

fn native_validity_value_missing(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "valueMissing", this, false, |v| v.value_missing)
}

fn native_validity_type_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "typeMismatch", this, false, |v| v.type_mismatch)
}

fn native_validity_pattern_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "patternMismatch", this, false, |v| v.pattern_mismatch)
}

fn native_validity_too_short(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "tooShort", this, false, |v| v.too_short)
}

fn native_validity_too_long(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "tooLong", this, false, |v| v.too_long)
}

fn native_validity_range_underflow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "rangeUnderflow", this, false, |v| v.range_underflow)
}

fn native_validity_range_overflow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "rangeOverflow", this, false, |v| v.range_overflow)
}

fn native_validity_step_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "stepMismatch", this, false, |v| v.step_mismatch)
}

fn native_validity_bad_input(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "badInput", this, false, |v| v.bad_input)
}

fn native_validity_custom_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    with_validity(ctx, "customError", this, false, |v| v.custom_error)
}

fn native_validity_valid(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // No FormControlState → not a candidate → spec-equivalent to
    // `valid = true` (matches `checkValidity()`'s default).
    with_validity(
        ctx,
        "valid",
        this,
        true,
        elidex_form::ValidityState::is_valid,
    )
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
/// HTML §4.10.10.1 lists `<output>` as a listed form-control too;
/// slot `#11-tags-T2d-interactive` extends the brand check to include
/// it now that `HTMLOutputElement.prototype` is registered with the
/// ConstraintValidation mixin installed (`globals.rs`).  `<form>` has
/// its own delegate-to-children checkValidity / reportValidity bodies
/// on `HTMLFormElement.prototype` (R2) that use `require_form_receiver`,
/// so it does not flow through this brand check either.
fn is_constraint_validation_host_tag(dom: &elidex_ecs::EcsDom, entity: Entity) -> bool {
    let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(entity) else {
        return false;
    };
    let s = tag.0.as_str();
    [
        "input", "select", "textarea", "button", "fieldset", "output",
    ]
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
            "Failed to execute '{method}' on 'ConstraintValidation': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn alloc_validity_wrapper(vm: &mut VmInner, entity: Entity) -> ObjectId {
    vm.intern_wrapper(
        WrapperKey::entity(entity, WrapperKind::ValidityState),
        |vm| {
            let proto = vm
                .validity_state_prototype
                .expect("alloc_validity_wrapper before register_validity_state_prototype");
            vm.alloc_object(Object {
                kind: ObjectKind::ValidityState {
                    entity_bits: entity.to_bits().get(),
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: Some(proto),
                extensible: false,
            })
        },
    )
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
        .filter(|state| elidex_form::is_constraint_validation_candidate(state, entity, dom))
        .map_or_else(String::new, |state| {
            // HTML §4.10.20.2: `validationMessage` is empty for
            // controls barred from constraint validation (matches
            // the with_validity / willValidate gating).  Custom
            // error wins; otherwise produce a stub diagnostic from
            // the failed flag.  Browser-quality localised messages
            // are deferred to the UA shell.
            let v = elidex_form::validate_control(&state);
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
    // HTML §4.10.20.3 "candidate for constraint validation"
    // — centralised in `elidex_form::is_constraint_validation_candidate`.
    let candidate = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_some_and(|state| elidex_form::is_constraint_validation_candidate(&state, entity, dom));
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
            // validation always returns true (HTML §4.10.20.3).  The
            // candidate predicate is centralised in
            // `elidex_form::is_constraint_validation_candidate`.
            if !elidex_form::is_constraint_validation_candidate(&state, entity, dom) {
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
