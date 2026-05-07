//! `HTMLFormElement.prototype` intrinsic — per-tag prototype layer
//! for `<form>` wrappers (HTML §4.10.3).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", form submission and reset
//! algorithms live in [`elidex_form::submit`] (`reset_form` /
//! `read_form_attrs` / `find_form_ancestor`).  This module reflects
//! content attributes, dispatches the cancelable `reset` event, and
//! delegates the reset side-effect to elidex-form.
//!
//! ## Members installed
//!
//! Reflected DOMString attributes: `action`, `method`, `enctype`,
//! `encoding` (alias of `enctype`), `target`, `name`, `acceptCharset`,
//! `autocomplete`, `rel`.
//!
//! Reflected boolean: `noValidate`.
//!
//! Read-only:
//! - `elements` — `[SameObject]`-cached `HTMLFormControlsCollection`
//!   over listed-element descendants (`CollectionFilter::FormControls`,
//!   added in Phase 7).
//! - `length` — number of listed-element descendants (mirrors
//!   `elements.length`).
//!
//! Methods:
//! - `submit()` / `requestSubmit()` — `NotSupportedError` stub
//!   (defer slot `#11-form-submission`, navigation infra).
//! - `reset()` — dispatches a cancelable `reset` event then, if not
//!   default-prevented, calls `elidex_form::reset_form` to roll
//!   each form control back to its `default_value` /
//!   `default_checked` (Group β F-7 + F-8 fold).
//! - `checkValidity()` / `reportValidity()` — iterate the listed
//!   elements, run `validate_control` on each candidate (skipping
//!   disabled / `<input type=hidden>` per HTML §4.10.20.3), fire
//!   the synthetic `invalid` event on every failing control, and
//!   return `false` if any control failed.  Form-level methods are
//!   NOT installed via the `install_constraint_validation_mixin`
//!   helper because the form's behaviour is delegate-to-children
//!   rather than the per-control single-state shape.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};
use elidex_form::FormControlState;

impl VmInner {
    pub(in crate::vm) fn register_html_form_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_form_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_form_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // String reflect pairs: (idl_sid, html_attr_name).
        let pairs: [(super::super::StringId, &'static str); 8] = [
            (self.well_known.action, "action"),
            (self.well_known.method_attr, "method"),
            (self.well_known.enctype, "enctype"),
            (self.well_known.target, "target"),
            (self.well_known.name, "name"),
            (self.well_known.accept_charset, "accept-charset"),
            (self.well_known.autocomplete, "autocomplete"),
            (self.well_known.rel, "rel"),
        ];
        for (name_sid, attr_name) in pairs {
            let getter = string_reflect_getter_for(attr_name);
            let setter = string_reflect_setter_for(attr_name);
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // `encoding` is a legacy alias for `enctype` (HTML §4.10.3).
        self.install_accessor_pair(
            proto_id,
            self.well_known.encoding,
            form_get_enctype,
            Some(form_set_enctype),
            attrs,
        );

        // noValidate boolean reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.no_validate,
            native_form_get_no_validate,
            Some(native_form_set_no_validate),
            attrs,
        );

        // length / elements — live, backed by FormControls collection.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_form_get_length,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.elements_attr,
            native_form_get_elements,
            None,
            attrs,
        );

        // Methods.
        let method_attrs = shape::PropertyAttrs::METHOD;
        self.install_native_method(
            proto_id,
            self.well_known.submit_method,
            native_form_submit,
            method_attrs,
        );
        self.install_native_method(
            proto_id,
            self.well_known.request_submit,
            native_form_request_submit,
            method_attrs,
        );
        self.install_native_method(
            proto_id,
            self.well_known.reset_method,
            native_form_reset,
            method_attrs,
        );
        self.install_native_method(
            proto_id,
            self.well_known.check_validity,
            native_form_check_validity,
            method_attrs,
        );
        self.install_native_method(
            proto_id,
            self.well_known.report_validity,
            native_form_report_validity,
            method_attrs,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_form_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLFormElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "form") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLFormElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String reflect helpers
// ---------------------------------------------------------------------------

fn string_reflect_getter_for(attr_name: &'static str) -> NativeFn {
    match attr_name {
        "action" => form_get_action,
        "method" => form_get_method,
        "enctype" => form_get_enctype,
        "target" => form_get_target,
        "name" => form_get_name,
        "accept-charset" => form_get_accept_charset,
        "autocomplete" => form_get_autocomplete,
        "rel" => form_get_rel,
        _ => unreachable!("string_reflect_getter_for called with unsupported attr {attr_name}"),
    }
}

fn string_reflect_setter_for(attr_name: &'static str) -> NativeFn {
    match attr_name {
        "action" => form_set_action,
        "method" => form_set_method,
        "enctype" => form_set_enctype,
        "target" => form_set_target,
        "name" => form_set_name,
        "accept-charset" => form_set_accept_charset,
        "autocomplete" => form_set_autocomplete,
        "rel" => form_set_rel,
        _ => unreachable!("string_reflect_setter_for called with unsupported attr {attr_name}"),
    }
}

macro_rules! form_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_form_receiver(ctx, this, $label)? else {
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
            let Some(entity) = require_form_receiver(ctx, this, $label)? else {
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

form_string_attr!(form_get_action, form_set_action, "action", "action");
form_string_attr!(form_get_method, form_set_method, "method", "method");
form_string_attr!(form_get_enctype, form_set_enctype, "enctype", "enctype");
form_string_attr!(form_get_target, form_set_target, "target", "target");
form_string_attr!(form_get_name, form_set_name, "name", "name");
form_string_attr!(
    form_get_accept_charset,
    form_set_accept_charset,
    "accept-charset",
    "acceptCharset"
);
form_string_attr!(
    form_get_autocomplete,
    form_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
form_string_attr!(form_get_rel, form_set_rel, "rel", "rel");

// ---------------------------------------------------------------------------
// noValidate boolean reflect
// ---------------------------------------------------------------------------

fn native_form_get_no_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "noValidate")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "novalidate"),
    ))
}

fn native_form_set_no_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "noValidate")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "novalidate", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "novalidate");
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// length / elements (live FormControls collection)
// ---------------------------------------------------------------------------

fn native_form_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Number(0.0));
    };
    // HTML §4.10.3: form.length returns the number of listed elements
    // in form.elements.  Build a transient FormControls collection,
    // count, and discard.
    let mut coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::FormControls,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let len = coll.length(ctx.host().dom());
    Ok(JsValue::Number(
        u32::try_from(len).unwrap_or(u32::MAX).into(),
    ))
}

fn native_form_get_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "elements")? else {
        let id = ctx
            .vm
            .alloc_collection(elidex_dom_api::LiveCollection::new_snapshot(
                Vec::new(),
                elidex_dom_api::CollectionKind::NodeList,
            ));
        return Ok(JsValue::Object(id));
    };
    // `[SameObject]` per WebIDL — successive `form.elements` reads
    // return the same wrapper id (HTML §4.10.3.1).  The cache is
    // entity-keyed and pruned weak-through-owner in `gc/collect.rs`.
    if let Some(&existing) = ctx.vm.form_controls_collection_wrappers.get(&entity) {
        return Ok(JsValue::Object(existing));
    }
    let coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::FormControls,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let id = ctx.vm.alloc_collection(coll);
    ctx.vm.form_controls_collection_wrappers.insert(entity, id);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// submit / requestSubmit / reset
// ---------------------------------------------------------------------------

fn native_form_submit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_form_receiver(ctx, this, "submit")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "form.submit() is not yet supported (slot #11-form-submission)",
    ))
}

fn native_form_request_submit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_form_receiver(ctx, this, "requestSubmit")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "form.requestSubmit() is not yet supported (slot #11-form-submission)",
    ))
}

/// `form.reset()` — HTML §4.10.21.5.
///
/// Group β F-7 + F-8 fold:
///
/// 1. Construct a cancelable `reset` event (bubbles=true,
///    cancelable=true) at the precomputed core-9 shape.
/// 2. Dispatch through the script-event walk; if a listener calls
///    `preventDefault()` the reset is cancelled per spec.
/// 3. On non-cancelled dispatch call
///    [`elidex_form::reset_form`] to roll each descendant
///    form-control's `FormControlState` back to its
///    `default_value` / `default_checked`.
fn native_form_reset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "reset")? else {
        return Ok(JsValue::Undefined);
    };
    let type_sid = ctx.vm.well_known.reset_event;
    let cancelled = super::event_target_dispatch::dispatch_simple_event(
        ctx, entity, type_sid, /*bubbles=*/ true, /*cancelable=*/ true,
    )?;
    if cancelled {
        return Ok(JsValue::Undefined);
    }
    elidex_form::reset_form(ctx.host().dom(), entity);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// checkValidity / reportValidity (HTML §4.10.20.4)
// ---------------------------------------------------------------------------

/// Walk every listed-element descendant of the form, calling
/// `validate_control()` on each that is a candidate for constraint
/// validation.  Per HTML §4.10.20.4 step 1, `invalid` must fire on
/// EVERY failing control before the method returns; we therefore
/// iterate the full set, dispatch the synthetic event on each
/// failure, and return `false` if any control failed.  Mirrors the
/// per-control `checkValidity()` shape installed by the
/// ConstraintValidation mixin but iterates the form's submittable
/// element set rather than a single entity.
fn run_form_check_validity(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<bool, VmError> {
    use elidex_form::FormControlKind;

    let mut coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::FormControls,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let snap: Vec<Entity> = coll.snapshot(ctx.host().dom()).to_vec();

    let mut all_valid = true;
    for control in snap {
        let dom = ctx.host().dom();
        let Some(state) = dom.world().get::<&FormControlState>(control).ok() else {
            continue;
        };
        // HTML §4.10.20.3 — bar non-candidates from validation
        // (disabled, `<input type=hidden>`, descendant of disabled
        // `<fieldset>`).
        if !state.kind.is_submittable()
            || state.disabled
            || matches!(state.kind, FormControlKind::Hidden)
        {
            continue;
        }
        if elidex_form::is_fieldset_disabled(control, dom) {
            continue;
        }
        let valid = elidex_form::validate_control(&state).is_valid();
        drop(state);
        if !valid {
            all_valid = false;
            let invalid_sid = ctx.vm.well_known.invalid_event;
            let _ = super::event_target_dispatch::dispatch_simple_event(
                ctx,
                control,
                invalid_sid,
                /*bubbles=*/ false,
                /*cancelable=*/ true,
            )?;
        }
    }
    Ok(all_valid)
}

fn native_form_check_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "checkValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    let valid = run_form_check_validity(ctx, entity)?;
    Ok(JsValue::Boolean(valid))
}

fn native_form_report_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Headless mode — same behaviour as `checkValidity()`.  The UA
    // validation popup is deferred to the shell layer.
    let Some(entity) = require_form_receiver(ctx, this, "reportValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    let valid = run_form_check_validity(ctx, entity)?;
    Ok(JsValue::Boolean(valid))
}
