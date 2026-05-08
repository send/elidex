//! `HTMLTextAreaElement.prototype` intrinsic — per-tag prototype
//! layer for `<textarea>` wrappers (HTML §4.10.11).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  All form-control state
//! (value / dirty-tracking / selection) lives in
//! [`elidex_form::FormControlState`]; this module reads / writes
//! that state through the public methods exposed by elidex-form.
//! No standalone HashMap state on the VM side.
//!
//! ## Members installed
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
//! IDL state-backed:
//! - `value` reads/writes `FormControlState.value` (setter marks
//!   dirty per HTML §4.10.11.1).
//! - `defaultValue` reflects textContent; setter mirrors into
//!   `FormControlState.default_value` and (when not dirty) the
//!   live `value`.
//! - `textLength` returns `value.length` (UTF-16 code units, on
//!   the IDL value).
//!
//! Read-only:
//! - `type` returns the constant `"textarea"`.
//! - `form` walks via `find_form_ancestor`.
//! - `labels` empty NodeList stub (same as `<button>`).
//!
//! Selection API mixin (HTML §4.10.5.2.10): `selectionStart`,
//! `selectionEnd`, `selectionDirection`, `select()`,
//! `setSelectionRange(start, end, dir?)`, `setRangeText(...)`.
//! All operate on `FormControlState.value` via the same elidex-form
//! algorithms used by `<input>`.

#![cfg(feature = "engine")]
// Selection API setters clamp negatives via `.max(0)` before the cast,
// so the conversion is value-preserving.  Module-wide allow matches
// `html_input_proto.rs`.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
// `map(...).unwrap_or(default)` on `Result<&FormControlState>` reads the
// entity component straightforwardly; the canonical `is_ok_and` /
// `map_or` rewrites require closure arguments by value rather than by
// reference, which doesn't compose with the borrow checker for the
// shared-borrow patterns used here.
#![allow(clippy::map_unwrap_or)]
// `use elidex_form::SelectionDirection;` is a local import within
// each Selection accessor body, mirroring `html_input_proto.rs`.
#![allow(clippy::items_after_statements)]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};
use elidex_form::FormControlState;

impl VmInner {
    #[allow(clippy::too_many_lines)] // Phase 6 install: 5 string + 4 bool + 4 long + 6 read-only accessors fit in one place by design.
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
        // value (state-backed) / defaultValue (textContent-mirroring).
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
            native_textarea_get_value,
            Some(native_textarea_set_value),
            attrs,
        );
        // Selection API mixin (HTML §4.10.5.2.10) — same algorithms
        // as `<input>`, brand-checked for textarea.
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_start,
            native_textarea_get_selection_start,
            Some(native_textarea_set_selection_start),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_end,
            native_textarea_get_selection_end,
            Some(native_textarea_set_selection_end),
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selection_direction,
            native_textarea_get_selection_direction,
            Some(native_textarea_set_selection_direction),
            attrs,
        );
        let m = shape::PropertyAttrs::METHOD;
        self.install_native_method(
            proto_id,
            self.well_known.select_method,
            native_textarea_select_method,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_selection_range,
            native_textarea_set_selection_range,
            m,
        );
        self.install_native_method(
            proto_id,
            self.well_known.set_range_text,
            native_textarea_set_range_text,
            m,
        );
    }
}

pub(super) fn require_textarea_receiver(
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
    native_textarea_get_autofocus,
    native_textarea_set_autofocus,
    "autofocus",
    "autofocus"
);

/// Boolean reflect setter that ALSO mirrors into the matching
fn native_textarea_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "disabled"),
    ))
}

fn native_textarea_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    super::form_state_sync::bool_attr_with_state_sync(
        ctx,
        this,
        args,
        "disabled",
        "disabled",
        require_textarea_receiver,
        |s, flag| s.disabled = flag,
    )
}

fn native_textarea_get_required(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "required")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "required"),
    ))
}

fn native_textarea_set_required(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    super::form_state_sync::bool_attr_with_state_sync(
        ctx,
        this,
        args,
        "required",
        "required",
        require_textarea_receiver,
        |s, flag| s.required = flag,
    )
}

fn native_textarea_get_readonly(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "readOnly")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "readonly"),
    ))
}

fn native_textarea_set_readonly(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    super::form_state_sync::bool_attr_with_state_sync(
        ctx,
        this,
        args,
        "readOnly",
        "readonly",
        require_textarea_receiver,
        |s, flag| s.readonly = flag,
    )
}

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
    super::form_state_sync::length_set_with_state_sync(
        ctx,
        this,
        args,
        "maxLength",
        "maxlength",
        require_textarea_receiver,
        |s, n| s.maxlength = n,
    )
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
    super::form_state_sync::length_set_with_state_sync(
        ctx,
        this,
        args,
        "minLength",
        "minlength",
        require_textarea_receiver,
        |s, n| s.minlength = n,
    )
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
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let coerced = JsValue::String(sid);
    super::dom_bridge::invoke_dom_api(ctx, "textContent.set", entity, &[coerced])?;
    // Mirror into FormControlState — `default_value` always tracks
    // the textContent; the live `value` only resets when the
    // control hasn't been dirtied (HTML §4.10.11.1).
    let s = ctx.vm.strings.get_utf8(sid);
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.default_value.clone_from(&s);
        if !state.is_dirty() {
            state.set_value_initial(s);
        }
    }
    Ok(JsValue::Undefined)
}

fn native_textarea_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_textarea_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    let dom = ctx.host().dom();
    let v = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.value().to_owned())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(&v);
    Ok(JsValue::String(sid))
}

fn native_textarea_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "value")? else {
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

fn native_textarea_get_text_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "textLength")? else {
        return Ok(JsValue::Number(0.0));
    };
    let dom = ctx.host().dom();
    let len = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| u32::try_from(s.value().encode_utf16().count()).unwrap_or(u32::MAX))
        .unwrap_or(0);
    Ok(JsValue::Number(f64::from(len)))
}

// ---------------------------------------------------------------------------
// Selection API — selectionStart / selectionEnd / selectionDirection /
// setSelectionRange / setRangeText / select.
//
// Bodies live in `vm/host/selection_api.rs` and are shared with
// `<input>`; this section is brand-check + interface-name plumbing.
// ---------------------------------------------------------------------------

const TEXTAREA_INTERFACE: &str = "HTMLTextAreaElement";
const TEXTAREA_ELEM_LABEL: &str = "element";

fn textarea_check(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, method)? else {
        return Ok(None);
    };
    super::selection_api::require_text_control(
        ctx,
        entity,
        method,
        TEXTAREA_INTERFACE,
        TEXTAREA_ELEM_LABEL,
    )?;
    Ok(Some(entity))
}

fn native_textarea_get_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Null);
    };
    Ok(super::selection_api::get_selection_start(ctx, entity))
}

fn native_textarea_set_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_start(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

fn native_textarea_get_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Null);
    };
    Ok(super::selection_api::get_selection_end(ctx, entity))
}

fn native_textarea_set_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_end(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

fn native_textarea_get_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Null);
    };
    Ok(super::selection_api::get_selection_direction(ctx, entity))
}

fn native_textarea_set_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_direction(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

fn native_textarea_select_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "select")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::select_all(ctx, entity);
    Ok(JsValue::Undefined)
}

fn native_textarea_set_selection_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "setSelectionRange")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_selection_range(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}

fn native_textarea_set_range_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = textarea_check(ctx, this, "setRangeText")? else {
        return Ok(JsValue::Undefined);
    };
    super::selection_api::set_range_text(ctx, entity, args)?;
    Ok(JsValue::Undefined)
}
