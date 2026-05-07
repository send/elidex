//! `HTMLInputElement.prototype` intrinsic — per-tag prototype layer
//! for `<input>` wrappers (HTML §4.10.5).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  All form-control state
//! (value / checked / selection / validation) lives in
//! [`elidex_form::FormControlState`]; this module reads / writes
//! that state through the public methods exposed by elidex-form.
//! No standalone HashMap state on the VM side.
//!
//! ## Members installed
//!
//! Reflected DOMString: name / accept / alt / autocomplete /
//! dirName / formAction / formEnctype / formMethod / formTarget /
//! max / min / pattern / placeholder / step / src.
//!
//! Reflected boolean: disabled / multiple / readOnly / required /
//! autofocus / formNoValidate.
//!
//! Reflected long: maxLength / minLength / size / width / height.
//!
//! Enumerated: type (default = "text").
//!
//! IDL state-backed:
//! - `value` reads/writes `FormControlState.value` (setter marks
//!   dirty per HTML §4.10.5.1.6).
//! - `defaultValue` reflects content attribute `value` (mirrors
//!   `default_value` in FormControlState).
//! - `checked` / `defaultChecked` / `indeterminate` for
//!   checkbox/radio.
//!
//! Read-only:
//! - `type`, `form`, `files` (null stub), `labels` (snapshot),
//!   `list` (null stub).
//!
//! Methods:
//! - `select()` — selects all (text controls).
//! - `setSelectionRange(start, end, dir?)` — text controls.
//! - `setRangeText(replacement, start?, end?, mode?)` — text
//!   controls; uses `elidex_form::selection`-flavoured replace.
//! - `stepUp(n?)` / `stepDown(n?)` — number/range; throw
//!   `InvalidStateError` for other types.
//! - `showPicker()` — `NotSupportedError` stub
//!   (`#11-show-picker`).
//!
//! `valueAsNumber` returns `NaN` for non-numeric types per
//! HTML §4.10.5.1.4 step 1 fallback.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};
use elidex_form::FormControlState;

impl VmInner {
    #[allow(clippy::too_many_lines)]
    pub(in crate::vm) fn register_html_input_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_input_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_input_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // String reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.name,
                native_input_get_name as NativeFn,
                native_input_set_name as NativeFn,
            ),
            (
                self.well_known.accept,
                native_input_get_accept,
                native_input_set_accept,
            ),
            (
                self.well_known.alt,
                native_input_get_alt,
                native_input_set_alt,
            ),
            (
                self.well_known.autocomplete,
                native_input_get_autocomplete,
                native_input_set_autocomplete,
            ),
            (
                self.well_known.dir_name,
                native_input_get_dir_name,
                native_input_set_dir_name,
            ),
            (
                self.well_known.form_action,
                native_input_get_form_action,
                native_input_set_form_action,
            ),
            (
                self.well_known.form_enctype,
                native_input_get_form_enctype,
                native_input_set_form_enctype,
            ),
            (
                self.well_known.form_method,
                native_input_get_form_method,
                native_input_set_form_method,
            ),
            (
                self.well_known.form_target,
                native_input_get_form_target,
                native_input_set_form_target,
            ),
            (
                self.well_known.max,
                native_input_get_max,
                native_input_set_max,
            ),
            (
                self.well_known.min,
                native_input_get_min,
                native_input_set_min,
            ),
            (
                self.well_known.pattern,
                native_input_get_pattern,
                native_input_set_pattern,
            ),
            (
                self.well_known.placeholder,
                native_input_get_placeholder,
                native_input_set_placeholder,
            ),
            (
                self.well_known.step,
                native_input_get_step,
                native_input_set_step,
            ),
            (
                self.well_known.src,
                native_input_get_src,
                native_input_set_src,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Boolean reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.disabled,
                native_input_get_disabled as NativeFn,
                native_input_set_disabled as NativeFn,
            ),
            (
                self.well_known.multiple,
                native_input_get_multiple,
                native_input_set_multiple,
            ),
            (
                self.well_known.read_only,
                native_input_get_read_only,
                native_input_set_read_only,
            ),
            (
                self.well_known.required,
                native_input_get_required,
                native_input_set_required,
            ),
            (
                self.well_known.autofocus,
                native_input_get_autofocus,
                native_input_set_autofocus,
            ),
            (
                self.well_known.form_no_validate,
                native_input_get_form_no_validate,
                native_input_set_form_no_validate,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Long reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.max_length,
                native_input_get_max_length as NativeFn,
                native_input_set_max_length as NativeFn,
            ),
            (
                self.well_known.min_length,
                native_input_get_min_length,
                native_input_set_min_length,
            ),
            (
                self.well_known.size_attr,
                native_input_get_size,
                native_input_set_size,
            ),
            (
                self.well_known.width,
                native_input_get_width,
                native_input_set_width,
            ),
            (
                self.well_known.height,
                native_input_get_height,
                native_input_set_height,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // type — enumerated.
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_input_get_type,
            Some(native_input_set_type),
            attrs,
        );
        // value / defaultValue / checked / defaultChecked / indeterminate.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_input_get_value,
            Some(native_input_set_value),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_value,
            native_input_get_default_value,
            Some(native_input_set_default_value),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.checked_attr,
            native_input_get_checked,
            Some(native_input_set_checked),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_checked,
            native_input_get_default_checked,
            Some(native_input_set_default_checked),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.indeterminate,
            native_input_get_indeterminate,
            Some(native_input_set_indeterminate),
            attrs,
        );
        // valueAsNumber.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value_as_number,
            native_input_get_value_as_number,
            Some(native_input_set_value_as_number),
            attrs,
        );
        // valueAsDate (stub: getter returns null, setter accepts only null).
        self.install_accessor_pair(
            proto_id,
            self.well_known.value_as_date,
            native_input_get_value_as_date,
            Some(native_input_set_value_as_date),
            attrs,
        );
        // form / labels / files / list.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_input_get_form,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels,
            native_input_get_labels,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.files,
            native_input_get_files,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.list_attr,
            native_input_get_list,
            None,
            attrs,
        );
        // Selection API state accessors.
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_start,
            native_input_get_selection_start,
            Some(native_input_set_selection_start),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_end,
            native_input_get_selection_end,
            Some(native_input_set_selection_end),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_direction,
            native_input_get_selection_direction,
            Some(native_input_set_selection_direction),
            attrs,
        );
        // Methods.
        let m = shape::PropertyAttrs::METHOD;
        self.install_native_method(
            proto_id,
            self.well_known.select_method,
            native_input_select_method,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_selection_range,
            native_input_set_selection_range,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_range_text,
            native_input_set_range_text,
            m,
        );
        self.install_native_method(proto_id, self.well_known.step_up, native_input_step_up, m);
        self.install_native_method(
            proto_id,
            self.well_known.step_down,
            native_input_step_down,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.show_picker,
            native_input_show_picker,
            m,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_input_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLInputElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "input") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLInputElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String / boolean / long reflect macros
// ---------------------------------------------------------------------------

macro_rules! input_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_input_receiver(ctx, this, $label)? else {
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
            let Some(entity) = require_input_receiver(ctx, this, $label)? else {
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

input_string_attr!(native_input_get_name, native_input_set_name, "name", "name");
input_string_attr!(
    native_input_get_accept,
    native_input_set_accept,
    "accept",
    "accept"
);
input_string_attr!(native_input_get_alt, native_input_set_alt, "alt", "alt");
input_string_attr!(
    native_input_get_autocomplete,
    native_input_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
input_string_attr!(
    native_input_get_dir_name,
    native_input_set_dir_name,
    "dirname",
    "dirName"
);
input_string_attr!(
    native_input_get_form_action,
    native_input_set_form_action,
    "formaction",
    "formAction"
);
input_string_attr!(
    native_input_get_form_enctype,
    native_input_set_form_enctype,
    "formenctype",
    "formEnctype"
);
input_string_attr!(
    native_input_get_form_method,
    native_input_set_form_method,
    "formmethod",
    "formMethod"
);
input_string_attr!(
    native_input_get_form_target,
    native_input_set_form_target,
    "formtarget",
    "formTarget"
);
input_string_attr!(native_input_get_max, native_input_set_max, "max", "max");
input_string_attr!(native_input_get_min, native_input_set_min, "min", "min");
input_string_attr!(
    native_input_get_pattern,
    native_input_set_pattern,
    "pattern",
    "pattern"
);
input_string_attr!(
    native_input_get_placeholder,
    native_input_set_placeholder,
    "placeholder",
    "placeholder"
);
input_string_attr!(native_input_get_step, native_input_set_step, "step", "step");
input_string_attr!(native_input_get_src, native_input_set_src, "src", "src");

macro_rules! input_bool_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_input_receiver(ctx, this, $label)? else {
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
            let Some(entity) = require_input_receiver(ctx, this, $label)? else {
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

input_bool_attr!(
    native_input_get_disabled,
    native_input_set_disabled,
    "disabled",
    "disabled"
);
input_bool_attr!(
    native_input_get_multiple,
    native_input_set_multiple,
    "multiple",
    "multiple"
);
input_bool_attr!(
    native_input_get_read_only,
    native_input_set_read_only,
    "readonly",
    "readOnly"
);
input_bool_attr!(
    native_input_get_required,
    native_input_set_required,
    "required",
    "required"
);
input_bool_attr!(
    native_input_get_autofocus,
    native_input_set_autofocus,
    "autofocus",
    "autofocus"
);
input_bool_attr!(
    native_input_get_form_no_validate,
    native_input_set_form_no_validate,
    "formnovalidate",
    "formNoValidate"
);

fn long_get_with_default(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    attr: &str,
    default: i32,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
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
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    ctx.host().dom().set_attribute(entity, attr, n.to_string());
    Ok(JsValue::Undefined)
}

macro_rules! input_long_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr, $default:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            long_get_with_default(ctx, this, $label, $attr, $default)
        }

        fn $set(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            long_set(ctx, this, args, $label, $attr)
        }
    };
}

input_long_attr!(
    native_input_get_max_length,
    native_input_set_max_length,
    "maxlength",
    "maxLength",
    -1
);
input_long_attr!(
    native_input_get_min_length,
    native_input_set_min_length,
    "minlength",
    "minLength",
    -1
);
input_long_attr!(
    native_input_get_size,
    native_input_set_size,
    "size",
    "size",
    20
);
input_long_attr!(
    native_input_get_width,
    native_input_set_width,
    "width",
    "width",
    0
);
input_long_attr!(
    native_input_get_height,
    native_input_set_height,
    "height",
    "height",
    0
);

// ---------------------------------------------------------------------------
// type — enumerated keyword (default "text").  HTML §4.10.5.1
// ---------------------------------------------------------------------------

const KNOWN_INPUT_TYPES: &[&str] = &[
    "hidden",
    "text",
    "search",
    "tel",
    "url",
    "email",
    "password",
    "date",
    "month",
    "week",
    "time",
    "datetime-local",
    "number",
    "range",
    "color",
    "checkbox",
    "radio",
    "file",
    "submit",
    "image",
    "reset",
    "button",
];

fn native_input_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "type")? else {
        let sid = ctx.vm.strings.intern("text");
        return Ok(JsValue::String(sid));
    };
    let raw = ctx
        .host()
        .dom()
        .get_attribute(entity, "type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let canonical = if KNOWN_INPUT_TYPES.contains(&raw.as_str()) {
        // Echo the canonical lowercase form.
        raw.as_str()
    } else {
        // Missing / invalid → "text" per HTML §4.10.5.1 default.
        "text"
    };
    let sid = ctx.vm.strings.intern(canonical);
    Ok(JsValue::String(sid))
}

fn native_input_set_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "type")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "type", s.clone());
    // Mirror the type into `FormControlState.kind` so subsequent
    // value / valueAsNumber / Selection-API behaviour reflects the
    // new type without requiring a re-attach (HTML §4.10.5.1.6).
    use elidex_form::FormControlKind;
    let new_kind = FormControlKind::from_type_str(&s.to_ascii_lowercase());
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.kind = new_kind;
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// value / defaultValue / checked / defaultChecked / indeterminate
// ---------------------------------------------------------------------------

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

fn native_input_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    Ok(read_state_value(ctx, entity))
}

fn native_input_set_value(
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

fn native_input_get_default_value(
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

fn native_input_set_default_value(
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
        state.default_value = s.clone();
        if !state.is_dirty() {
            state.set_value_initial(s);
        }
    }
    Ok(JsValue::Undefined)
}

fn native_input_get_checked(
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

fn native_input_set_checked(
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

fn native_input_get_default_checked(
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

fn native_input_set_default_checked(
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

fn native_input_get_indeterminate(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `indeterminate` is a JS-only IDL bit not stored in
    // FormControlState; tracked separately would inflate the side
    // table.  Phase 8 returns false; observable via UI when the
    // shell layer integrates.
    let _ = ctx;
    Ok(JsValue::Boolean(false))
}

fn native_input_set_indeterminate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_input_receiver(ctx, this, "indeterminate")?;
    // Accept any value, no-op.  Spec behaviour is observable only
    // through the rendered UI which hasn't landed.
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// valueAsNumber / valueAsDate
// ---------------------------------------------------------------------------

fn native_input_get_value_as_number(
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

fn native_input_set_value_as_number(
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

fn native_input_get_value_as_date(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Stub — Date IDL integration deferred to
    // `#11-input-value-as-date`.
    let _ = require_input_receiver(ctx, this, "valueAsDate")?;
    Ok(JsValue::Null)
}

fn native_input_set_value_as_date(
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

// ---------------------------------------------------------------------------
// form / labels / files / list
// ---------------------------------------------------------------------------

fn native_input_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}

fn native_input_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_input_receiver(ctx, this, "labels")?;
    let id = ctx
        .vm
        .alloc_collection(elidex_dom_api::LiveCollection::new_snapshot(
            Vec::new(),
            elidex_dom_api::CollectionKind::NodeList,
        ));
    Ok(JsValue::Object(id))
}

fn native_input_get_files(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Stub — File API surface deferred to `#11c-fl PR-file-api`.
    let _ = require_input_receiver(ctx, this, "files")?;
    Ok(JsValue::Null)
}

fn native_input_get_list(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Stub — datalist surface deferred to `#11-tags-T2d-interactive`.
    let _ = require_input_receiver(ctx, this, "list")?;
    Ok(JsValue::Null)
}

// ---------------------------------------------------------------------------
// Selection API — selectionStart / selectionEnd / selectionDirection /
// setSelectionRange / setRangeText / select
// ---------------------------------------------------------------------------

fn require_text_control(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    method: &str,
) -> Result<(), VmError> {
    let dom = ctx.host().dom();
    let supports = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.kind.supports_selection())
        .unwrap_or(false);
    if !supports {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'HTMLInputElement': \
                 The input element's type does not support selection"
            ),
        ));
    }
    Ok(())
}

fn native_input_get_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Null);
    };
    require_text_control(ctx, entity, "selectionStart")?;
    let dom = ctx.host().dom();
    let pos = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.selection_start())
        .unwrap_or(0);
    Ok(JsValue::Number(f64::from(
        u32::try_from(pos).unwrap_or(u32::MAX),
    )))
}

fn native_input_set_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Undefined);
    };
    require_text_control(ctx, entity, "selectionStart")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?.max(0) as usize;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.set_selection_start(n);
    }
    Ok(JsValue::Undefined)
}

fn native_input_get_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Null);
    };
    require_text_control(ctx, entity, "selectionEnd")?;
    let dom = ctx.host().dom();
    let pos = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.selection_end())
        .unwrap_or(0);
    Ok(JsValue::Number(f64::from(
        u32::try_from(pos).unwrap_or(u32::MAX),
    )))
}

fn native_input_set_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Undefined);
    };
    require_text_control(ctx, entity, "selectionEnd")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?.max(0) as usize;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.set_selection_end(n);
    }
    Ok(JsValue::Undefined)
}

fn native_input_get_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Null);
    };
    require_text_control(ctx, entity, "selectionDirection")?;
    let dom = ctx.host().dom();
    use elidex_form::SelectionDirection;
    let dir = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.selection_direction)
        .unwrap_or(SelectionDirection::None);
    let s = match dir {
        SelectionDirection::Forward => "forward",
        SelectionDirection::Backward => "backward",
        SelectionDirection::None => "none",
    };
    let sid = ctx.vm.strings.intern(s);
    Ok(JsValue::String(sid))
}

fn native_input_set_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Undefined);
    };
    require_text_control(ctx, entity, "selectionDirection")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    use elidex_form::SelectionDirection;
    let dir = match s.as_str() {
        "forward" => SelectionDirection::Forward,
        "backward" => SelectionDirection::Backward,
        _ => SelectionDirection::None,
    };
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.selection_direction = dir;
    }
    Ok(JsValue::Undefined)
}

fn native_input_select_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "select")? else {
        return Ok(JsValue::Undefined);
    };
    require_text_control(ctx, entity, "select")?;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        elidex_form::select_all(&mut state);
    }
    Ok(JsValue::Undefined)
}

fn native_input_set_selection_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "setSelectionRange")? else {
        return Ok(JsValue::Undefined);
    };
    require_text_control(ctx, entity, "setSelectionRange")?;
    let start_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let end_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let dir_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    let start = super::super::coerce::to_int32(ctx.vm, start_arg)?.max(0) as usize;
    let end = super::super::coerce::to_int32(ctx.vm, end_arg)?.max(0) as usize;
    use elidex_form::SelectionDirection;
    let dir = if matches!(dir_arg, JsValue::Undefined) {
        SelectionDirection::None
    } else {
        let sid = super::super::coerce::to_string(ctx.vm, dir_arg)?;
        let s = ctx.vm.strings.get_utf8(sid);
        match s.as_str() {
            "forward" => SelectionDirection::Forward,
            "backward" => SelectionDirection::Backward,
            _ => SelectionDirection::None,
        }
    };
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.set_selection(start, end);
        state.selection_direction = dir;
    }
    Ok(JsValue::Undefined)
}

fn native_input_set_range_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "setRangeText")? else {
        return Ok(JsValue::Undefined);
    };
    require_text_control(ctx, entity, "setRangeText")?;
    let replacement_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, replacement_arg)?;
    let replacement = ctx.vm.strings.get_utf8(sid);
    // Optional start / end / mode args.
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        let (cur_s, cur_e) = state.safe_selection_range();
        let start = if let Some(v) = args.get(1).copied() {
            (v.try_to_int_or_zero()).max(0) as usize
        } else {
            cur_s
        };
        let end = if let Some(v) = args.get(2).copied() {
            (v.try_to_int_or_zero()).max(0) as usize
        } else {
            cur_e
        };
        state.set_selection(start, end);
        // FormControlState exposes `replace_selection` directly,
        // matching `elidex_form::selection::replace_selection` but
        // accessible without the private-module re-export.
        state.replace_selection(replacement.as_str());
    }
    Ok(JsValue::Undefined)
}

trait JsValueIntCoerce {
    fn try_to_int_or_zero(self) -> i32;
}
impl JsValueIntCoerce for JsValue {
    fn try_to_int_or_zero(self) -> i32 {
        match self {
            JsValue::Number(n) if n.is_finite() => n as i32,
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// stepUp / stepDown / showPicker
// ---------------------------------------------------------------------------

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
    let invalid_state_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        use elidex_form::FormControlKind;
        let supports = matches!(state.kind, FormControlKind::Number | FormControlKind::Range);
        if !supports {
            drop(state);
            return Err(VmError::dom_exception(
                invalid_state_sid,
                format!(
                    "Failed to execute '{method}' on 'HTMLInputElement': \
                     This input element does not have stepping"
                ),
            ));
        }
        let step = state
            .step
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0);
        let cur = state.value().parse::<f64>().unwrap_or(0.0);
        let new = cur + direction * n * step;
        state.set_value(new.to_string());
    }
    Ok(JsValue::Undefined)
}

fn native_input_step_up(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    step_apply(ctx, this, args, "stepUp", 1.0)
}

fn native_input_step_down(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    step_apply(ctx, this, args, "stepDown", -1.0)
}

fn native_input_show_picker(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_input_receiver(ctx, this, "showPicker")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "showPicker() is not yet supported (slot #11-show-picker)",
    ))
}
