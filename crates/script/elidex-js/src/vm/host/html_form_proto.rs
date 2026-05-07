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
//! Read-only stubs:
//! - `elements` — empty NodeList snapshot (Phase 4 stub; full
//!   `HTMLFormControlsCollection` lands in Phase 7).
//! - `length` — 0 (matches the empty stub above).
//!
//! Methods:
//! - `submit()` / `requestSubmit()` — `NotSupportedError` stub
//!   (defer slot `#11-form-submission`, navigation infra).
//! - `reset()` — dispatches a cancelable `reset` event then, if not
//!   default-prevented, calls `elidex_form::reset_form` to roll
//!   each form control back to its `default_value` /
//!   `default_checked` (Group β F-7 + F-8 fold).
//! - `checkValidity()` / `reportValidity()` — Phase 9 mixin.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};

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

        // length / elements — Phase 4 stubs.
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
// length / elements stubs
// ---------------------------------------------------------------------------

fn native_form_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_form_receiver(ctx, this, "length")?;
    Ok(JsValue::Number(0.0))
}

fn native_form_get_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_form_receiver(ctx, this, "elements")?;
    let id = ctx
        .vm
        .alloc_collection(elidex_dom_api::LiveCollection::new_snapshot(
            Vec::new(),
            elidex_dom_api::CollectionKind::NodeList,
        ));
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
/// Phase 4 ships only the F-8 fold (reset form controls).  F-7
/// (cancelable `reset` event dispatch) is deferred to a follow-up
/// slot once a thin event-dispatch helper covers the
/// "construct + dispatch a simple Event by type StringId" pattern
/// at parity with the existing UA-initiated path; reusing
/// `dispatch_script_event` directly is possible but inflates the
/// Phase 4 surface significantly with event-shape plumbing
/// orthogonal to T1-v2's prototype-install scope.  Deferred slot:
/// `#11-tags-T1-followup-reset-event` — re-evaluate 2026-Q3.
fn native_form_reset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "reset")? else {
        return Ok(JsValue::Undefined);
    };
    // F-8 fold: roll back form-control state.  `reset_form` is a
    // no-op on descendants without a `FormControlState` component,
    // so this is safe even before Phase 8/9 attach state to
    // JS-created inputs.
    elidex_form::reset_form(ctx.host().dom(), entity);
    Ok(JsValue::Undefined)
}
