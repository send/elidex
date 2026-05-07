//! `HTMLTextAreaElement.prototype` intrinsic — per-tag prototype
//! layer for `<textarea>` wrappers (HTML §4.10.11).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  The Selection API mixin
//! (`selectionStart` / `selectionEnd` / `setSelectionRange` /
//! `setRangeText` / `select`) is deferred — it requires per-control
//! state which lands when `FormControlState` attachment is wired
//! through createElement, at which point `elidex_form::selection`
//! becomes the algorithm host.  Tracked as
//! `#11-tags-T1-followup-selection-api`.
//!
//! ## Members installed (Phase 6 minimal scope)
//!
//! Reflected DOMString attrs: `name`, `placeholder`, `wrap`,
//! `dirName`, `autocomplete`.
//!
//! Reflected boolean: `disabled`, `readOnly`, `required`,
//! `autofocus`.
//!
//! Reflected long: `cols` (default 20), `rows` (default 2),
//! `maxLength` (-1 default), `minLength` (-1 default).
//!
//! Read-only:
//! - `type` returns the constant `"textarea"`.
//! - `form` walks via `find_form_ancestor`.
//! - `defaultValue` reads the textContent (HTML §4.10.11.1).
//! - `value` aliases defaultValue until `FormControlState` lands.
//!   Phase 6 ships the alias; Phase 8 follow-up wires the
//!   dirty-tracking proper.
//! - `textLength` returns `value.length` (UTF-16 code units).
//! - `labels` empty NodeList stub (same as `<button>`).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    pub(in crate::vm) fn register_html_textarea_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_textarea_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_textarea_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // String reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.name,
                native_textarea_get_name as crate::vm::NativeFn,
                native_textarea_set_name as crate::vm::NativeFn,
            ),
            (
                self.well_known.placeholder,
                native_textarea_get_placeholder as crate::vm::NativeFn,
                native_textarea_set_placeholder as crate::vm::NativeFn,
            ),
            (
                self.well_known.wrap,
                native_textarea_get_wrap as crate::vm::NativeFn,
                native_textarea_set_wrap as crate::vm::NativeFn,
            ),
            (
                self.well_known.dir_name,
                native_textarea_get_dir_name as crate::vm::NativeFn,
                native_textarea_set_dir_name as crate::vm::NativeFn,
            ),
            (
                self.well_known.autocomplete,
                native_textarea_get_autocomplete as crate::vm::NativeFn,
                native_textarea_set_autocomplete as crate::vm::NativeFn,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Boolean reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.disabled,
                native_textarea_get_disabled as crate::vm::NativeFn,
                native_textarea_set_disabled as crate::vm::NativeFn,
            ),
            (
                self.well_known.read_only,
                native_textarea_get_readonly as crate::vm::NativeFn,
                native_textarea_set_readonly as crate::vm::NativeFn,
            ),
            (
                self.well_known.required,
                native_textarea_get_required as crate::vm::NativeFn,
                native_textarea_set_required as crate::vm::NativeFn,
            ),
            (
                self.well_known.autofocus,
                native_textarea_get_autofocus as crate::vm::NativeFn,
                native_textarea_set_autofocus as crate::vm::NativeFn,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Long reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.cols,
                native_textarea_get_cols as crate::vm::NativeFn,
                native_textarea_set_cols as crate::vm::NativeFn,
            ),
            (
                self.well_known.rows,
                native_textarea_get_rows as crate::vm::NativeFn,
                native_textarea_set_rows as crate::vm::NativeFn,
            ),
            (
                self.well_known.max_length,
                native_textarea_get_max_length as crate::vm::NativeFn,
                native_textarea_set_max_length as crate::vm::NativeFn,
            ),
            (
                self.well_known.min_length,
                native_textarea_get_min_length as crate::vm::NativeFn,
                native_textarea_set_min_length as crate::vm::NativeFn,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_textarea_get_type,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_textarea_get_form,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels,
            native_textarea_get_labels,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.text_length,
            native_textarea_get_text_length,
            None,
            attrs,
        );
        // value / defaultValue — Phase 6 alias both to textContent.
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_value,
            native_textarea_get_default_value,
            Some(native_textarea_set_default_value),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_textarea_get_default_value,
            Some(native_textarea_set_default_value),
            attrs,
        );
    }
}

fn require_textarea_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTextAreaElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "textarea") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTextAreaElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String reflect macro
// ---------------------------------------------------------------------------

macro_rules! ta_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_textarea_receiver(ctx, this, $label)? else {
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
            let Some(entity) = require_textarea_receiver(ctx, this, $label)? else {
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

ta_string_attr!(
    native_textarea_get_name,
    native_textarea_set_name,
    "name",
    "name"
);
ta_string_attr!(
    native_textarea_get_placeholder,
    native_textarea_set_placeholder,
    "placeholder",
    "placeholder"
);
ta_string_attr!(
    native_textarea_get_wrap,
    native_textarea_set_wrap,
    "wrap",
    "wrap"
);
ta_string_attr!(
    native_textarea_get_dir_name,
    native_textarea_set_dir_name,
    "dirname",
    "dirName"
);
ta_string_attr!(
    native_textarea_get_autocomplete,
    native_textarea_set_autocomplete,
    "autocomplete",
    "autocomplete"
);

// ---------------------------------------------------------------------------
// Boolean reflect macro
// ---------------------------------------------------------------------------

macro_rules! ta_bool_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_textarea_receiver(ctx, this, $label)? else {
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
            let Some(entity) = require_textarea_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let flag = super::super::coerce::to_boolean(ctx.vm, val);
            if flag {
                ctx.host().dom().set_attribute(entity, $attr, String::new());
            } else {
                super::element_attrs::attr_remove(ctx, entity, $attr);
            }
            Ok(JsValue::Undefined)
        }
    };
}

ta_bool_attr!(
    native_textarea_get_disabled,
    native_textarea_set_disabled,
    "disabled",
    "disabled"
);
ta_bool_attr!(
    native_textarea_get_readonly,
    native_textarea_set_readonly,
    "readonly",
    "readOnly"
);
ta_bool_attr!(
    native_textarea_get_required,
    native_textarea_set_required,
    "required",
    "required"
);
ta_bool_attr!(
    native_textarea_get_autofocus,
    native_textarea_set_autofocus,
    "autofocus",
    "autofocus"
);

// ---------------------------------------------------------------------------
// Long (i32) reflect — `cols` / `rows` / `maxLength` / `minLength`
// ---------------------------------------------------------------------------

fn long_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    attr: &str,
    default: i32,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, method)? else {
        return Ok(JsValue::Number(f64::from(default)));
    };
    let v = ctx
        .host()
        .dom()
        .get_attribute(entity, attr)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(default);
    Ok(JsValue::Number(f64::from(v)))
}

fn long_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    ctx.host().dom().set_attribute(entity, attr, n.to_string());
    Ok(JsValue::Undefined)
}

fn native_textarea_get_cols(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_get(ctx, this, "cols", "cols", 20)
}

fn native_textarea_set_cols(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_set(ctx, this, args, "cols", "cols")
}

fn native_textarea_get_rows(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_get(ctx, this, "rows", "rows", 2)
}

fn native_textarea_set_rows(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_set(ctx, this, args, "rows", "rows")
}

fn native_textarea_get_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_get(ctx, this, "maxLength", "maxlength", -1)
}

fn native_textarea_set_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_set(ctx, this, args, "maxLength", "maxlength")
}

fn native_textarea_get_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_get(ctx, this, "minLength", "minlength", -1)
}

fn native_textarea_set_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_set(ctx, this, args, "minLength", "minlength")
}

// ---------------------------------------------------------------------------
// Read-only / derived
// ---------------------------------------------------------------------------

fn native_textarea_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_textarea_receiver(ctx, this, "type")?;
    let sid = ctx.vm.strings.intern("textarea");
    Ok(JsValue::String(sid))
}

fn native_textarea_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}

fn native_textarea_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_textarea_receiver(ctx, this, "labels")?;
    let id = ctx
        .vm
        .alloc_collection(elidex_dom_api::LiveCollection::new_snapshot(
            Vec::new(),
            elidex_dom_api::CollectionKind::NodeList,
        ));
    Ok(JsValue::Object(id))
}

fn native_textarea_get_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    super::dom_bridge::invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn native_textarea_set_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let coerced = JsValue::String(super::super::coerce::to_string(ctx.vm, val)?);
    super::dom_bridge::invoke_dom_api(ctx, "textContent.set", entity, &[coerced])?;
    Ok(JsValue::Undefined)
}

fn native_textarea_get_text_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "textLength")? else {
        return Ok(JsValue::Number(0.0));
    };
    let value = super::dom_bridge::invoke_dom_api(ctx, "textContent.get", entity, &[])?;
    let len = match value {
        JsValue::String(sid) => ctx.vm.strings.get_utf8(sid).encode_utf16().count(),
        _ => 0,
    };
    Ok(JsValue::Number(len as f64))
}
