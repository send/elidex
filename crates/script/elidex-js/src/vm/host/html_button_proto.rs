//! `HTMLButtonElement.prototype` intrinsic — per-tag prototype layer
//! for `<button>` wrappers (HTML §4.10.6).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  Form association resolves
//! through [`elidex_form::find_form_ancestor`]; label collection is
//! deferred to the elidex-form integration once `FormControlState`
//! attachment lands on createElement-backed buttons.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

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

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // Reflected DOMString attrs.  `formAction` / `formEnctype` /
        // `formMethod` / `formTarget` overrides for submitter buttons.
        self.install_accessor_pair(
            proto_id,
            self.well_known.name,
            native_button_get_name,
            Some(native_button_set_name),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_button_get_value,
            Some(native_button_set_value),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_button_get_type,
            Some(native_button_set_type),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_action,
            native_button_get_form_action,
            Some(native_button_set_form_action),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_enctype,
            native_button_get_form_enctype,
            Some(native_button_set_form_enctype),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_method,
            native_button_get_form_method,
            Some(native_button_set_form_method),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_target,
            native_button_get_form_target,
            Some(native_button_set_form_target),
            attrs,
        );
        // Boolean reflects.
        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_button_get_disabled,
            Some(native_button_set_disabled),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_no_validate,
            native_button_get_form_no_validate,
            Some(native_button_set_form_no_validate),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.autofocus,
            native_button_get_autofocus,
            Some(native_button_set_autofocus),
            attrs,
        );
        // Read-only `form` accessor.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_button_get_form,
            None,
            attrs,
        );
        // `labels` — Phase 5 stub: returns an empty NodeList.  Full
        // walk lands when label collection is wired through the
        // form-association infrastructure.
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels,
            native_button_get_labels,
            None,
            attrs,
        );
    }
}

fn require_button_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLButtonElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "button") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLButtonElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String reflect macro
// ---------------------------------------------------------------------------

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
            ctx.host().dom().set_attribute(entity, $attr, &s);
            Ok(JsValue::Undefined)
        }
    };
}

button_string_attr!(
    native_button_get_name,
    native_button_set_name,
    "name",
    "name"
);
button_string_attr!(
    native_button_get_value,
    native_button_set_value,
    "value",
    "value"
);
button_string_attr!(
    native_button_get_form_action,
    native_button_set_form_action,
    "formaction",
    "formAction"
);
// `<button>.formEnctype` — HTML §4.10.5.4 enumerated-attribute
// override.  Same keyword set as `<form>.enctype`, but missing-value
// AND invalid-value defaults are both `""` (the empty string is the
// "no override" sentinel — the form-level enctype wins when the
// content attribute is absent or invalid).
fn native_button_get_form_enctype(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_button_receiver(ctx, this, "formEnctype")? else {
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

fn native_button_set_form_enctype(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "formEnctype")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "formenctype", &s);
    Ok(JsValue::Undefined)
}

// `<button>.formMethod` — HTML §4.10.5.4 enumerated-attribute
// override.  Same keyword set as `<form>.method`, but missing- and
// invalid-value defaults are both `""` (see `formEnctype` above).
fn native_button_get_form_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_button_receiver(ctx, this, "formMethod")? else {
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

fn native_button_set_form_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "formMethod")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "formmethod", &s);
    Ok(JsValue::Undefined)
}
button_string_attr!(
    native_button_get_form_target,
    native_button_set_form_target,
    "formtarget",
    "formTarget"
);

// type — enumerated, defaults to "submit".  HTML §4.10.6.
fn native_button_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "type")? else {
        let sid = ctx.vm.strings.intern("submit");
        return Ok(JsValue::String(sid));
    };
    let sid = super::element_attrs::enumerated_attr_reflect(
        ctx,
        entity,
        "type",
        &["submit", "reset", "button"],
        "submit",
    );
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
    ctx.host().dom().set_attribute(entity, "type", &s);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Boolean reflects
// ---------------------------------------------------------------------------

macro_rules! button_bool_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_button_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Boolean(false));
            };
            Ok(JsValue::Boolean(
                ctx.host().dom().has_attribute(entity, $attr),
            ))
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
            let flag = super::super::coerce::to_boolean(ctx.vm, val);
            if flag {
                ctx.host().dom().set_attribute(entity, $attr, "");
            } else {
                super::element_attrs::attr_remove(ctx, entity, $attr);
            }
            Ok(JsValue::Undefined)
        }
    };
}

button_bool_attr!(
    native_button_get_disabled,
    native_button_set_disabled,
    "disabled",
    "disabled"
);
button_bool_attr!(
    native_button_get_form_no_validate,
    native_button_set_form_no_validate,
    "formnovalidate",
    "formNoValidate"
);
button_bool_attr!(
    native_button_get_autofocus,
    native_button_set_autofocus,
    "autofocus",
    "autofocus"
);

// ---------------------------------------------------------------------------
// form / labels
// ---------------------------------------------------------------------------

fn native_button_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_button_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}

/// Phase 5 stub.  Real labels collection lands when
/// `FormControlState` attachment is wired through createElement and
/// elidex-form's `collect_labels_for` walker is exposed at
/// `elidex_form::label`.
fn native_button_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_button_receiver(ctx, this, "labels")?;
    Ok(JsValue::Object(
        super::dom_collection::empty_labels_collection(ctx.vm),
    ))
}
