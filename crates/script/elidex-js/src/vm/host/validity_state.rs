//! `ValidityState` interface + `ConstraintValidation` mixin
//! (HTML §4.10.18.5 / §4.10.18.6 — slot #11-tags-T1 Phase 9).
//!
//! ## ValidityState
//!
//! Per WebIDL [Exposed=Window], ValidityState has no callable
//! constructor (`new ValidityState()` throws TypeError per WebIDL
//! §3.7.6).  Instances surface only via
//! `<form-control>.validity` accessor reads.
//!
//! Brand check: the wrapper is allocated as
//! [`ObjectKind::HostObject`] with `entity_bits` carrying the
//! owning control's [`elidex_ecs::Entity`].  Brand verification
//! consults [`VmInner::validity_state_wrappers`] (Entity →
//! ObjectId) and confirms the inverse association — `Object.create(ValidityState.prototype)`
//! does not produce a HostObject and so fails the check, and a
//! plain Element wrapper for the same entity has a different
//! ObjectId from the cached ValidityState wrapper.
//!
//! Identity: each control returns the same ValidityState across
//! repeated `.validity` reads (matches browser semantics) via
//! [`VmInner::validity_state_wrappers`].
//!
//! ## ConstraintValidation mixin
//!
//! Five elements participate in the mixin per HTML §4.10.18.5:
//! HTMLInputElement / HTMLSelectElement / HTMLTextAreaElement /
//! HTMLButtonElement / HTMLFieldSetElement.  HTMLFormElement also
//! exposes `checkValidity` / `reportValidity` for form-level walk
//! (§4.10.18.5).  This module installs the four mixin members
//! (`willValidate` / `validity` / `validationMessage` /
//! `checkValidity` / `reportValidity` / `setCustomValidity`) on
//! each prototype via [`install_constraint_validation_methods`].
//!
//! ## Validation backend (Phase 9 approximation)
//!
//! Without the `elidex-form` Cargo dep landing in this PR, the
//! validity computation uses an approximation:
//!
//! - `customError` is `true` iff a non-empty
//!   [`VmInner::form_control_custom_validity`] entry exists for the
//!   control's entity.
//! - All other 9 flags (`valueMissing` / `typeMismatch` / …) are
//!   reported as `false`.
//! - `valid` derives as the AND of "no flag set" → equivalent to
//!   `!customError` in this approximation.
//! - `validationMessage` returns the custom validity string when
//!   `customError`, otherwise the empty string.
//! - `willValidate` returns `true` for submittable elements that
//!   are not disabled and not inside a disabled fieldset (a more
//!   refined check would also gate by `<input type=hidden>`,
//!   button-type submitter exclusions, etc., but the basic
//!   `disabled` gate covers the most common case).
//!
//! Phase 10's elidex-form dep landing replaces this with the real
//! `validate_control` walk that populates every flag based on the
//! control's value / pattern / required / min / max / step
//! attributes.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::VmInner;

/// Tags whose elements participate in the ConstraintValidation
/// mixin (HTML §4.10.18.5).  Matched case-insensitively against
/// the receiver's tag at brand-check time.  HTMLFormElement is NOT
/// in this set — the form has its own form-level `checkValidity()`
/// / `reportValidity()` pair (HTML §4.10.18.5 step 1) installed
/// directly on `HTMLFormElement.prototype`, not the mixin.
const CV_HOST_TAGS: [&str; 5] = ["input", "select", "textarea", "button", "fieldset"];

impl VmInner {
    /// Allocate `ValidityState.prototype` + install the global
    /// `ValidityState` ctor (which throws TypeError on call /
    /// construct per WebIDL [Exposed=Window]).  Must run after
    /// `register_object_prototype`.
    pub(in crate::vm) fn register_validity_state_global(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.validity_state_prototype = Some(proto_id);

        for &(prop_sid, getter) in &[
            (
                self.well_known.value_missing,
                vs_get_value_missing as super::super::NativeFn,
            ),
            (self.well_known.type_mismatch, vs_get_type_mismatch),
            (self.well_known.pattern_mismatch, vs_get_pattern_mismatch),
            (self.well_known.too_long, vs_get_too_long),
            (self.well_known.too_short, vs_get_too_short),
            (self.well_known.range_underflow, vs_get_range_underflow),
            (self.well_known.range_overflow, vs_get_range_overflow),
            (self.well_known.step_mismatch, vs_get_step_mismatch),
            (self.well_known.bad_input, vs_get_bad_input),
            (self.well_known.custom_error, vs_get_custom_error),
            (self.well_known.valid_attr, vs_get_valid),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Install the global `ValidityState` constructor name —
        // a callable that throws on construct.  Matches WebIDL
        // §3.7.6: "`new ValidityState()` throws TypeError" for
        // [Exposed] interfaces without a constructor.
        let ctor_id = self.create_native_function("ValidityState", vs_constructor);
        let global = self.global_object;
        let key = super::super::value::PropertyKey::String(self.well_known.validity_state);
        self.define_shaped_property(
            global,
            key,
            super::super::value::PropertyValue::Data(JsValue::Object(ctor_id)),
            shape::PropertyAttrs::METHOD,
        );
        // Wire `ValidityState.prototype` onto the ctor so
        // `obj instanceof ValidityState` resolves correctly via the
        // prototype-chain check.
        let proto_key = super::super::value::PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor_id,
            proto_key,
            super::super::value::PropertyValue::Data(JsValue::Object(proto_id)),
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Identity-preserving allocator for ValidityState wrappers.
    /// `<form-control>.validity` reads route through here so the
    /// same control returns the same ValidityState ObjectId across
    /// repeated reads.
    pub(crate) fn cached_or_alloc_validity_state(&mut self, control: Entity) -> ObjectId {
        if let Some(&id) = self.validity_state_wrappers.get(&control) {
            return id;
        }
        let proto = self
            .validity_state_prototype
            .expect("cached_or_alloc_validity_state before register_validity_state_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::HostObject {
                entity_bits: control.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.validity_state_wrappers.insert(control, id);
        id
    }
}

/// `new ValidityState()` / `ValidityState()` — both throw TypeError
/// per WebIDL §3.7.6.
fn vs_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Failed to construct 'ValidityState': Illegal constructor".to_string(),
    ))
}

/// Brand check for ValidityState accessors — confirms the receiver
/// is a wrapper allocated through
/// [`VmInner::cached_or_alloc_validity_state`].  Returns the owning
/// control entity on success.
fn require_validity_state_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    accessor: &'static str,
) -> Result<Entity, VmError> {
    let wrong = || {
        VmError::type_error(format!(
            "Failed to get '{accessor}' on 'ValidityState': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(wrong());
    };
    let entity_bits = match ctx.vm.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => entity_bits,
        _ => return Err(wrong()),
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(wrong)?;
    // Must be the cached ValidityState wrapper for this entity —
    // confirms the wrapper was allocated through the validity
    // accessor path and not, say, an Element wrapper for the same
    // entity that the user smuggled in via `Object.setPrototypeOf`.
    if ctx.vm.validity_state_wrappers.get(&entity) != Some(&id) {
        return Err(wrong());
    }
    Ok(entity)
}

/// Phase 9 approximation: every validity flag except `customError`
/// is `false`; `customError` is `true` iff
/// [`VmInner::form_control_custom_validity`] holds a non-empty
/// entry for the control entity.
fn has_custom_error(ctx: &NativeContext<'_>, entity: Entity) -> bool {
    ctx.vm
        .form_control_custom_validity
        .get(&entity)
        .is_some_and(|s| !s.is_empty())
}

// --- 11 boolean accessors -----------------------------------------

fn vs_get_value_missing(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "valueMissing")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_type_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "typeMismatch")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_pattern_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "patternMismatch")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_too_long(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "tooLong")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_too_short(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "tooShort")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_range_underflow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "rangeUnderflow")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_range_overflow(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "rangeOverflow")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_step_mismatch(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "stepMismatch")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_bad_input(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_validity_state_receiver(ctx, this, "badInput")?;
    Ok(JsValue::Boolean(false))
}

fn vs_get_custom_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_validity_state_receiver(ctx, this, "customError")?;
    Ok(JsValue::Boolean(has_custom_error(ctx, entity)))
}

fn vs_get_valid(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_validity_state_receiver(ctx, this, "valid")?;
    Ok(JsValue::Boolean(!has_custom_error(ctx, entity)))
}

// =========================================================================
// ConstraintValidation mixin install
// =========================================================================

/// Install the ConstraintValidation members on `proto_id`:
///
/// - `validity` (RO accessor → ValidityState)
/// - `validationMessage` (RO accessor → DOMString)
/// - `willValidate` (RO accessor → boolean)
/// - `checkValidity()` (method → boolean)
/// - `reportValidity()` (method → boolean — same as checkValidity
///   in headless mode; UI popup deferred to slot #11-validation-ui)
/// - `setCustomValidity(message)` (method → undefined)
///
/// The shared accessor / method natives all brand-check via
/// [`require_cv_host_receiver`], which gates by tag membership in
/// [`CV_HOST_TAGS`].  Each prototype caller passes the
/// already-allocated `proto_id` (HTMLInputElement.prototype etc.).
pub(super) fn install_constraint_validation_methods(vm: &mut VmInner, proto_id: ObjectId) {
    vm.install_accessor_pair(
        proto_id,
        vm.well_known.validity_attr,
        cv_get_validity,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        vm.well_known.validation_message,
        cv_get_validation_message,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        vm.well_known.will_validate,
        cv_get_will_validate,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_native_method(
        proto_id,
        vm.well_known.check_validity,
        cv_check_validity,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        vm.well_known.report_validity,
        cv_report_validity,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        vm.well_known.set_custom_validity,
        cv_set_custom_validity,
        shape::PropertyAttrs::METHOD,
    );
}

/// Brand check for ConstraintValidation members — confirms the
/// receiver is one of the six host element types
/// ([`CV_HOST_TAGS`]).  This complements each per-prototype
/// brand-check (which already verifies the specific tag); the
/// duplicate guard here keeps the shared natives self-contained.
fn require_cv_host_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = super::event_target::require_receiver(ctx, this, "Element", method, |k| {
        k == NodeKind::Element
    })?
    else {
        return Ok(None);
    };
    let tag_matches = ctx.host().dom().with_tag_name(entity, |t| match t {
        Some(s) => CV_HOST_TAGS.iter().any(|c| c.eq_ignore_ascii_case(s)),
        None => false,
    });
    if !tag_matches {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Element': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// `<form-control>.validity` getter — returns the cached
/// ValidityState wrapper, allocating one on first read.
fn cv_get_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cv_host_receiver(ctx, this, "validity")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.cached_or_alloc_validity_state(entity);
    Ok(JsValue::Object(id))
}

/// `validationMessage` getter — returns the custom-validity string
/// when `customError`, otherwise `""`.
fn cv_get_validation_message(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_cv_host_receiver(ctx, this, "validationMessage")? else {
        return Ok(JsValue::String(empty));
    };
    let msg = ctx.vm.form_control_custom_validity.get(&entity);
    let Some(s) = msg else {
        return Ok(JsValue::String(empty));
    };
    if s.is_empty() {
        return Ok(JsValue::String(empty));
    }
    let sid = ctx.vm.strings.intern(s);
    Ok(JsValue::String(sid))
}

/// `<input>` `type` keywords for which the `readonly` content
/// attribute is honoured per HTML §4.10.5.1.20.  Outside these
/// types, `readonly` has no defined effect (e.g. `<input
/// type=checkbox readonly>` is not actually read-only).
const READONLY_INPUT_TYPES: [&str; 13] = [
    "text",
    "search",
    "url",
    "tel",
    "email",
    "password",
    "date",
    "month",
    "week",
    "time",
    "datetime-local",
    "number",
    // Default-empty / unrecognised input types resolve to "text",
    // so callers fall back through the `None` arm of the type
    // attribute lookup which we treat as "text" too.
    "",
];

/// Returns `true` when `entity` participates in constraint
/// validation per HTML §4.10.18.3.
///
/// Bars applied:
///
/// - `disabled` content attribute on the entity itself.
/// - Ancestor `<fieldset>` with `disabled` set, EXCEPT entities
///   inside that fieldset's **first `<legend>` element child**
///   (HTML §4.10.12 carve-out).
/// - Tag-level: `<fieldset>` / `<output>` / `<object>` are listed
///   but NOT submittable, so they never participate.
/// - `<input>` types `hidden` / `button` / `reset` / `image` are
///   "not subject to constraint validation".
/// - `<button>` with `type=button` or `type=reset` (default
///   `type=submit` validates).
/// - `readonly` on a text-control (`<textarea>` or `<input>` with
///   a type that honours `readonly` — see [`READONLY_INPUT_TYPES`]).
///
/// Bars NOT yet applied (deferred with elidex-form dep landing,
/// slot `#11-validity-state-real-flags`): `<datalist>` ancestor.
pub(super) fn entity_will_validate(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    let dom = ctx.host().dom();
    if dom.has_attribute(entity, "disabled") {
        return false;
    }
    // Tag-level / type-level bars.
    let bar_by_kind = dom.with_tag_name(entity, |t| {
        let Some(tag) = t else { return false };
        if tag.eq_ignore_ascii_case("fieldset")
            || tag.eq_ignore_ascii_case("output")
            || tag.eq_ignore_ascii_case("object")
        {
            return true;
        }
        if tag.eq_ignore_ascii_case("input") {
            // Default `type` is "text" (validates); only the
            // explicit barred keywords short-circuit.  Then the
            // `readonly` bar applies to text-control types.
            let type_barred = dom.with_attribute(entity, "type", |v| match v {
                Some(s) => {
                    s.eq_ignore_ascii_case("hidden")
                        || s.eq_ignore_ascii_case("button")
                        || s.eq_ignore_ascii_case("reset")
                        || s.eq_ignore_ascii_case("image")
                }
                None => false,
            });
            if type_barred {
                return true;
            }
            if dom.has_attribute(entity, "readonly") {
                let is_readonly_type = dom.with_attribute(entity, "type", |v| match v {
                    Some(s) => READONLY_INPUT_TYPES
                        .iter()
                        .any(|kw| kw.eq_ignore_ascii_case(s)),
                    None => true, // absent type defaults to "text"
                });
                return is_readonly_type;
            }
            return false;
        }
        if tag.eq_ignore_ascii_case("button") {
            // `<button>` default type is `submit` (validates);
            // only `type=button` / `type=reset` are barred.
            return dom.with_attribute(entity, "type", |v| match v {
                Some(s) => s.eq_ignore_ascii_case("button") || s.eq_ignore_ascii_case("reset"),
                None => false,
            });
        }
        if tag.eq_ignore_ascii_case("textarea") {
            // `<textarea>` honours `readonly` unconditionally.
            return dom.has_attribute(entity, "readonly");
        }
        false
    });
    if bar_by_kind {
        return false;
    }
    let mut cur = dom.get_parent(entity);
    let mut depth: u32 = 0;
    while let Some(p) = cur {
        if depth > 1024 {
            break;
        }
        if dom.has_tag(p, "fieldset") && dom.has_attribute(p, "disabled") {
            // Disabled fieldset bars descendants — except those
            // inside its first `<legend>` child.
            let legend_exempt = match dom.first_child_with_tag(p, "legend") {
                Some(legend) => dom.is_ancestor_or_self(legend, entity),
                None => false,
            };
            if !legend_exempt {
                return false;
            }
        }
        cur = dom.get_parent(p);
        depth += 1;
    }
    true
}

/// Per-control validity check used by both
/// `ConstraintValidation.checkValidity()` (Phase 9 mixin) and
/// `HTMLFormElement.checkValidity()` (form-level statically-
/// validate walk).  Returns `true` when the control is exempt
/// (`willValidate == false`) or when no custom-validity message is
/// set.  Phase 9 approximation: the 9 non-customError validity
/// flags always read `false`, so this reduces to "exempt OR custom-
/// validity empty".  Real flag wiring lands with the elidex-form
/// dep (slot `#11-validity-state-real-flags`).
pub(super) fn entity_check_validity(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    if !entity_will_validate(ctx, entity) {
        return true;
    }
    match ctx.vm.form_control_custom_validity.get(&entity) {
        None => true,
        Some(s) => s.is_empty(),
    }
}

fn cv_get_will_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cv_host_receiver(ctx, this, "willValidate")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(entity_will_validate(ctx, entity)))
}

/// `checkValidity()` — returns `true` if the control is valid.  Per
/// HTML §4.10.18.3, controls whose `willValidate` is `false` (e.g.
/// disabled, inside a disabled fieldset) are exempt from constraint
/// validation and report `true` regardless of any custom-validity
/// message.  Otherwise the Phase 9 approximation reduces to "no
/// custom-validity message set".  HTMLFormElement's form-level
/// walk is implemented separately on `HTMLFormElement.prototype`.
fn cv_check_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cv_host_receiver(ctx, this, "checkValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    Ok(JsValue::Boolean(entity_check_validity(ctx, entity)))
}

/// `reportValidity()` — same as `checkValidity()` in headless mode.
/// Slot #11-validation-ui adds the UA validation popup once the
/// shell layer is ready.
fn cv_report_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cv_host_receiver(ctx, this, "reportValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    Ok(JsValue::Boolean(entity_check_validity(ctx, entity)))
}

/// `setCustomValidity(message)` — writes `message` into
/// [`VmInner::form_control_custom_validity`].  An empty string
/// clears the custom error per HTML §4.10.18.5.
fn cv_set_custom_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_cv_host_receiver(ctx, this, "setCustomValidity")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    if s.is_empty() {
        ctx.vm.form_control_custom_validity.remove(&entity);
    } else {
        ctx.vm.form_control_custom_validity.insert(entity, s);
    }
    Ok(JsValue::Undefined)
}
