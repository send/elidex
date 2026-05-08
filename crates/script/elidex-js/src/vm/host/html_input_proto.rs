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
//! - `checked` / `defaultChecked` for checkbox/radio.
//! - `indeterminate` round-trips through
//!   `FormControlState.indeterminate` (HTML §4.10.5.1.16); a
//!   JS-only IDL bit independent of `checked`, observable via the
//!   `:indeterminate` CSS pseudo-class once styling lands.
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
// Cast-sign-loss: every `as usize` conversion in this module is
// gated by an explicit `n < 0` guard or a `n.max(0)` clamp, so
// the cast is value-preserving.  Module-wide allow keeps the
// reflected-attr setters readable rather than scattering
// `usize::try_from(...).unwrap_or(0)` boilerplate.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
// `map(...).unwrap_or(default)` on `Result<&FormControlState>` /
// `Result<&mut FormControlState>` reads the entity component
// straightforwardly; the canonical `is_ok_and` / `map_or` rewrites
// require closure arguments by value rather than by reference,
// which doesn't compose with the borrow checker for the
// shared-borrow patterns used here.  Module-wide allow.
#![allow(clippy::map_unwrap_or)]
// Trait + impl pairs that exist only to extend a foreign type
// (JsValueIntCoerce) live next to their use site for readability —
// moving them above the function body separates the helper from
// its single consumer for no observable benefit.
#![allow(clippy::items_after_statements)]
// Defensive unused-but-kept underscore bindings in dispatch
// fall-through paths (e.g. `report_validity → check_validity` proxy).
#![allow(clippy::used_underscore_binding)]

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
        // value / defaultValue / checked / defaultChecked / indeterminate
        // (bodies in `super::html_input_value`).
        use super::html_input_value as iv;
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            iv::native_input_get_value,
            Some(iv::native_input_set_value),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_value,
            iv::native_input_get_default_value,
            Some(iv::native_input_set_default_value),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.checked_attr,
            iv::native_input_get_checked,
            Some(iv::native_input_set_checked),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_checked,
            iv::native_input_get_default_checked,
            Some(iv::native_input_set_default_checked),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.indeterminate,
            iv::native_input_get_indeterminate,
            Some(iv::native_input_set_indeterminate),
            attrs,
        );
        // valueAsNumber.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value_as_number,
            iv::native_input_get_value_as_number,
            Some(iv::native_input_set_value_as_number),
            attrs,
        );
        // valueAsDate (stub: getter returns null, setter accepts only null).
        self.install_accessor_pair(
            proto_id,
            self.well_known.value_as_date,
            iv::native_input_get_value_as_date,
            Some(iv::native_input_set_value_as_date),
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
        // Selection API state accessors (bodies in
        // `super::html_input_selection`).
        use super::html_input_selection as is_;
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_start,
            is_::native_input_get_selection_start,
            Some(is_::native_input_set_selection_start),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_end,
            is_::native_input_get_selection_end,
            Some(is_::native_input_set_selection_end),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_direction,
            is_::native_input_get_selection_direction,
            Some(is_::native_input_set_selection_direction),
            attrs,
        );
        // Methods.
        let m = shape::PropertyAttrs::METHOD;
        self.install_native_method(
            proto_id,
            self.well_known.select_method,
            is_::native_input_select_method,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_selection_range,
            is_::native_input_set_selection_range,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_range_text,
            is_::native_input_set_range_text,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.step_up,
            iv::native_input_step_up,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.step_down,
            iv::native_input_step_down,
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

pub(super) fn require_input_receiver(
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
    native_input_get_multiple,
    native_input_set_multiple,
    "multiple",
    "multiple"
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

/// Boolean reflect setter that ALSO mirrors into the matching
/// `FormControlState` field via `apply`.  Used for
/// constraint-bearing attributes (`disabled` / `required` /
/// `readOnly`) so a JS-side `input.required = true` reflects in
/// `validate_control()` without requiring re-attach.
fn bool_attr_with_state_sync<F>(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
    apply: F,
) -> Result<JsValue, VmError>
where
    F: FnOnce(&mut FormControlState, bool),
{
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host().dom().set_attribute(entity, attr, String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, attr);
    }
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        apply(&mut state, flag);
    }
    Ok(JsValue::Undefined)
}

fn native_input_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "disabled"),
    ))
}

fn native_input_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_attr_with_state_sync(ctx, this, args, "disabled", "disabled", |s, flag| {
        s.disabled = flag;
    })
}

fn native_input_get_required(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "required")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "required"),
    ))
}

fn native_input_set_required(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_attr_with_state_sync(ctx, this, args, "required", "required", |s, flag| {
        s.required = flag;
    })
}

fn native_input_get_read_only(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "readOnly")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "readonly"),
    ))
}

fn native_input_set_read_only(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_attr_with_state_sync(ctx, this, args, "readOnly", "readonly", |s, flag| {
        s.readonly = flag;
    })
}

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

fn native_input_get_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_get_with_default(ctx, this, "maxLength", "maxlength", -1)
}

fn native_input_set_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "maxLength")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    // HTML §6.13.1 reflection rule for `unsigned long` length attrs:
    // negative values clear the content attribute (the IDL getter
    // then returns the default `-1`), instead of persisting an
    // illegal `maxlength="-1"`.
    if n < 0 {
        super::element_attrs::attr_remove(ctx, entity, "maxlength");
    } else {
        ctx.host()
            .dom()
            .set_attribute(entity, "maxlength", n.to_string());
    }
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.maxlength = if n < 0 { None } else { Some(n as usize) };
    }
    Ok(JsValue::Undefined)
}

fn native_input_get_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    long_get_with_default(ctx, this, "minLength", "minlength", -1)
}

fn native_input_set_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "minLength")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    // HTML §6.13.1 reflection rule (see `set_max_length`).
    if n < 0 {
        super::element_attrs::attr_remove(ctx, entity, "minlength");
    } else {
        ctx.host()
            .dom()
            .set_attribute(entity, "minlength", n.to_string());
    }
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.minlength = if n < 0 { None } else { Some(n as usize) };
    }
    Ok(JsValue::Undefined)
}

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
// value / defaultValue / checked / defaultChecked / indeterminate /
// valueAsNumber / valueAsDate / stepUp / stepDown
//
// Bodies live in `vm/host/html_input_value.rs` (B-5 split).
// ---------------------------------------------------------------------------

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
// Selection API thin wrappers live in `vm/host/html_input_selection.rs`,
// stepUp / stepDown bodies live in `vm/host/html_input_value.rs`.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// showPicker
// ---------------------------------------------------------------------------

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
