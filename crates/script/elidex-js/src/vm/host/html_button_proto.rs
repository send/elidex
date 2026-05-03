//! `HTMLButtonElement.prototype` intrinsic — per-tag prototype layer
//! for `<button>` wrappers (HTML §4.10.6).
//!
//! Chain (slot #11-tags-T1 Phase 5):
//!
//! ```text
//! button wrapper
//!   → HTMLButtonElement.prototype
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - `disabled` (boolean reflect)
//! - `formAction` / `formEnctype` / `formMethod` / `formNoValidate`
//!   (boolean) / `formTarget` — string-reflect of `formaction` /
//!   `formenctype` / `formmethod` / `formnovalidate` /
//!   `formtarget` content attributes.  These mirror the parent
//!   form's submission settings when the button is the submitter.
//! - `name` / `value` — string reflect.
//! - `type` — enumerated reflection of the `type` content attribute.
//!   Per HTML §4.10.6 the missing-value default and invalid-value
//!   default are both `"submit"`.  Valid keywords: `submit` /
//!   `reset` / `button`.
//! - `form` derived getter (via shared `form_assoc::resolve_form_association`).
//! - `labels` derived getter — NodeList of associated labels.
//!
//! ConstraintValidation methods (`checkValidity` / `reportValidity` /
//! `setCustomValidity`) install on this prototype during Phase 9
//! through the shared mixin — see plan §B.
//!
//! Popover API members (`popoverTargetElement` /
//! `popoverTargetAction`) are deferred to slot **#11-popover-api**
//! (plan §F-6); not installed here.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLButtonElement";

impl VmInner {
    pub(in crate::vm) fn register_html_button_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_button_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_button_prototype = Some(proto_id);

        // String-reflect attributes.  formAction → "formaction" etc.
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.form_action,
                button_get_form_action as super::super::NativeFn,
                button_set_form_action as super::super::NativeFn,
            ),
            (
                self.well_known.form_enctype,
                button_get_form_enctype,
                button_set_form_enctype,
            ),
            (
                self.well_known.form_method,
                button_get_form_method,
                button_set_form_method,
            ),
            (
                self.well_known.form_target,
                button_get_form_target,
                button_set_form_target,
            ),
            (self.well_known.name, button_get_name, button_set_name),
            (self.well_known.value, button_get_value, button_set_value),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Boolean reflects.
        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_button_get_disabled,
            Some(native_button_set_disabled),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_no_validate,
            native_button_get_form_no_validate,
            Some(native_button_set_form_no_validate),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Enumerated `type` accessor — invalid/missing → "submit".
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_button_get_type,
            Some(native_button_set_type),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // form / labels — read-only derived getters.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_button_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels_attr,
            native_button_get_labels,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // ConstraintValidation mixin (Phase 9).
        super::validity_state::install_constraint_validation_methods(self, proto_id);
    }
}

fn require_button_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = super::event_target::require_receiver(ctx, this, INTERFACE, method, |k| {
        k == NodeKind::Element
    })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "button") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// --- String-reflect macro -----------------------------------------

macro_rules! button_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_button_receiver(ctx, this, $label)? else {
                return Ok(JsValue::String(empty));
            };
            let sid = match ctx.dom_and_strings_if_bound() {
                Some((dom, strings)) => {
                    dom.with_attribute(entity, $attr, |v| v.map_or(empty, |s| strings.intern(s)))
                }
                None => empty,
            };
            Ok(JsValue::String(sid))
        }

        fn $set(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_button_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let sid = super::super::coerce::to_string(ctx.vm, val)?;
            let s = ctx.vm.strings.get_utf8(sid);
            ctx.host().dom().set_attribute(entity, $attr, s);
            Ok(JsValue::Undefined)
        }
    };
}

button_string_attr!(
    button_get_form_action,
    button_set_form_action,
    "formaction",
    "formAction"
);
button_string_attr!(
    button_get_form_enctype,
    button_set_form_enctype,
    "formenctype",
    "formEnctype"
);
button_string_attr!(
    button_get_form_method,
    button_set_form_method,
    "formmethod",
    "formMethod"
);
button_string_attr!(
    button_get_form_target,
    button_set_form_target,
    "formtarget",
    "formTarget"
);
button_string_attr!(button_get_name, button_set_name, "name", "name");
button_string_attr!(button_get_value, button_set_value, "value", "value");

// --- Boolean reflects ---------------------------------------------

fn native_button_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "disabled"),
    ))
}

fn native_button_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "disabled", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "disabled");
    }
    Ok(JsValue::Undefined)
}

fn native_button_get_form_no_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "formNoValidate")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "formnovalidate"),
    ))
}

fn native_button_set_form_no_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "formNoValidate")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "formnovalidate", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "formnovalidate");
    }
    Ok(JsValue::Undefined)
}

// --- type (enumerated reflect with submit default) ----------------

fn native_button_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let submit = ctx.vm.well_known.submit_str;
    let Some(entity) = require_button_receiver(ctx, this, "type")? else {
        return Ok(JsValue::String(submit));
    };
    // Per HTML §4.10.6, missing-value default + invalid-value default
    // are both "submit"; the valid keywords are submit / reset /
    // button (case-insensitive at attribute parse time).
    let attr = ctx
        .host()
        .dom()
        .with_attribute(entity, "type", |v| v.map(|s| s.to_ascii_lowercase()));
    let normalised = match attr.as_deref() {
        Some("submit" | "reset" | "button") => attr.unwrap(),
        _ => return Ok(JsValue::String(submit)),
    };
    let sid = ctx.vm.strings.intern(&normalised);
    Ok(JsValue::String(sid))
}

fn native_button_set_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "type")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    // Setter writes the value verbatim — invalid values are
    // permitted on the content attribute and surfaced through the
    // getter's invalid-value fallback.
    ctx.host().dom().set_attribute(entity, "type", s);
    Ok(JsValue::Undefined)
}

// --- form / labels ------------------------------------------------

fn native_button_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    match super::form_assoc::resolve_form_association(ctx, entity) {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

fn native_button_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "labels")? else {
        return Ok(JsValue::Null);
    };
    let labels = super::form_assoc::collect_labels_for(ctx, entity);
    let kind = super::dom_collection::LiveCollectionKind::Snapshot { entities: labels };
    let id = ctx.vm.alloc_collection(kind);
    Ok(JsValue::Object(id))
}
