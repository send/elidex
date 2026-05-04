//! `HTMLInputElement.prototype` intrinsic — per-tag prototype layer
//! for `<input>` wrappers (HTML §4.10.5 — slot #11-tags-T1 Phase 8,
//! the largest of the T1 element protos).
//!
//! Chain:
//!
//! ```text
//! input wrapper
//!   → HTMLInputElement.prototype
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! ## Members installed
//!
//! ### Reflected attributes (DOMString unless noted)
//!
//! - `accept` / `alt` / `autocomplete` / `dirName` (`dirname`) /
//!   `name` / `pattern` / `placeholder` / `src` / `step` /
//!   `formAction` (`formaction`) / `formEnctype` (`formenctype`) /
//!   `formMethod` (`formmethod`) / `formTarget` (`formtarget`) /
//!   `max` / `min`.
//! - **boolean reflects**: `disabled` / `multiple` / `readOnly`
//!   (`readonly`) / `required` / `autofocus` /
//!   `formNoValidate` (`formnovalidate`).
//! - **`unsigned long` reflects**: `maxLength` (`maxlength`) /
//!   `minLength` (`minlength`) / `size` / `width` / `height`.
//! - **`type`** — enumerated reflection.  Missing-value default and
//!   invalid-value default are both `"text"` per HTML §4.10.5.1.18.
//!   The keyword set is the canonical 22 input types; setter writes
//!   the value verbatim and the getter normalises through the
//!   allowlist.
//!
//! ### Form-control state
//!
//! - **`value`** / **`defaultValue`** — backed by the per-element
//!   dirty-value slot (`FormControlEntityState`); `defaultValue`
//!   reflects the `value` content attribute.
//! - **`checked`** / **`defaultChecked`** — `checked` content
//!   attribute serves as both default and current state in this
//!   approximation (Phase 9 introduces the proper internal-flag
//!   separation alongside the elidex-form dep landing).
//! - **`valueAsNumber`** — `Number` form of `value`; NaN for
//!   non-numeric input types.  Setter throws `InvalidStateError`
//!   for non-numeric types per HTML §4.10.5.4.
//! - **`valueAsDate`** — `Date | null` for date-types
//!   (`date` / `time` / `month` / `week` / `datetime-local`); `null`
//!   otherwise.  Phase 8 returns `null` for the getter and accepts
//!   only `null` on set; full Date integration ships in a follow-up
//!   slot once `Date` parsing lands on the spec hot path.
//! - **`stepUp(n=1)`** / **`stepDown(n=1)`** — modify the value by
//!   `n × step` for numeric input types; throw `InvalidStateError`
//!   for non-numeric types.
//!
//! ### Selection API
//!
//! Installed via [`super::selection_api`] with a brand check that
//! gates the receiver to text-control input types per HTML
//! §4.10.5.2.10 (`text` / `search` / `tel` / `url` / `email` /
//! `password`).  Non-text-control receivers throw `InvalidStateError`
//! on access.
//!
//! ### Form association
//!
//! - `form` — derived getter (shared
//!   [`super::form_assoc::resolve_form_association`]).
//! - `labels` — derived getter (shared
//!   [`super::form_assoc::collect_labels_for`]).
//!
//! ## Deferrals (slot bind)
//!
//! - **`files`** → slot **`#11c-fl PR-file-api`** (HTML §4.10.5.4).
//!   Phase 8 returns `null` for `type !== "file"` and throws
//!   `InvalidStateError` for `type === "file"`.
//! - **`showPicker()`** → slot **`#11-show-picker`** (depends on
//!   shell platform-picker integration).  Throws
//!   `NotSupportedError` with explicit slot cite.
//! - **`list`** (HTMLDataListElement getter) → slot
//!   **`#11-tags-T2`** (HTMLDataListElement bundled there).  Phase
//!   8 returns `null` (matches "no datalist found" semantics).
//! - **`valueAsDate`** Date integration → slot
//!   **`#11-input-value-as-date`** once `Date` integration is
//!   audited.  Phase 8 returns `null` from the getter and accepts
//!   only `null` on the setter; non-`null` set throws TypeError.
//! - **ConstraintValidation methods** (`checkValidity` /
//!   `reportValidity` / `setCustomValidity`) → **Phase 9** of this
//!   PR via the shared `install_constraint_validation_methods`
//!   helper.
//! - **`form.reset()` integration** → Phase 9 / slot
//!   `#11c-followup-reset-form` (clears the `dirty_value` slot when
//!   the parent form resets).

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;
use super::form_control_state::utf16_len;
use super::selection_api::{self, SelectionAccessors};

const INTERFACE: &str = "HTMLInputElement";

/// The 22 canonical `<input type=…>` keywords per HTML §4.10.5.1.18.
/// Lowercase only; the getter normalises via case-insensitive ASCII
/// match.  Missing-value default and invalid-value default both fall
/// back to `"text"`.
const INPUT_TYPE_KEYWORDS: [&str; 22] = [
    "button",
    "checkbox",
    "color",
    "date",
    "datetime-local",
    "email",
    "file",
    "hidden",
    "image",
    "month",
    "number",
    "password",
    "radio",
    "range",
    "reset",
    "search",
    "submit",
    "tel",
    "text",
    "time",
    "url",
    "week",
];

/// `<input type=…>` keywords that support the Selection API
/// (HTML §4.10.5.2.10).
const TEXT_CONTROL_INPUT_TYPES: [&str; 6] = ["text", "search", "tel", "url", "email", "password"];

/// Numeric input types — `valueAsNumber` getter returns a parsed
/// number (NaN otherwise); `stepUp` / `stepDown` accept these.
const NUMERIC_INPUT_TYPES: [&str; 6] = [
    "number", "range", "date", "month", "week",
    "time",
    // Note: `datetime-local` is also numeric per the algorithm
    // table but lives in DATE_INPUT_TYPES below for the
    // `valueAsDate` mapping; numeric handling for
    // `datetime-local` is checked separately.
];

/// Date-class input types — `valueAsDate` returns a Date for these
/// (Phase 8 stub returns null pending the Date-integration slot).
const DATE_INPUT_TYPES: [&str; 5] = ["date", "time", "month", "week", "datetime-local"];

/// Match the `type` content attribute against `keywords`
/// case-insensitively, falling back to `"text"` (or whatever the
/// caller treats as the missing/invalid default — caller's choice
/// via the keyword list).  Allocation-free hot path for the
/// gating call sites (Selection API / valueAsNumber / valueAsDate
/// / stepUp / stepDown / files).
fn input_type_matches(ctx: &mut NativeContext<'_>, entity: Entity, keywords: &[&str]) -> bool {
    ctx.host().dom().with_attribute(entity, "type", |v| {
        let s = v.unwrap_or("text");
        if keywords.iter().any(|k| s.eq_ignore_ascii_case(k)) {
            return true;
        }
        // Honour the "missing/invalid → text" fallback: when the
        // attribute is set to something outside `INPUT_TYPE_KEYWORDS`
        // the effective type is "text"; check the keyword list one
        // more time to see if "text" is permitted.
        let valid = INPUT_TYPE_KEYWORDS
            .iter()
            .any(|k| s.eq_ignore_ascii_case(k));
        !valid && keywords.iter().any(|k| *k == "text")
    })
}

/// Returns the canonical lower-cased type — only used by
/// `inp_get_type` (which must materialise the string for the JS
/// caller) and by error messages.  Hot-path gating sites use
/// [`input_type_matches`] instead to avoid the alloc.
fn input_type(ctx: &mut NativeContext<'_>, entity: Entity) -> String {
    let attr = ctx
        .host()
        .dom()
        .with_attribute(entity, "type", |v| v.map(|s| s.to_ascii_lowercase()));
    match attr.as_deref() {
        Some(s) if INPUT_TYPE_KEYWORDS.iter().any(|k| *k == s) => attr.unwrap(),
        _ => "text".to_string(),
    }
}

fn is_numeric_input_type(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    input_type_matches(ctx, entity, &NUMERIC_INPUT_TYPES)
        || input_type_matches(ctx, entity, &["datetime-local"])
}

fn is_date_input_type(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    input_type_matches(ctx, entity, &DATE_INPUT_TYPES)
}

fn supports_selection(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    input_type_matches(ctx, entity, &TEXT_CONTROL_INPUT_TYPES)
}

impl VmInner {
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

        // String reflects.
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.accept_attr,
                inp_get_accept as super::super::NativeFn,
                inp_set_accept as super::super::NativeFn,
            ),
            (self.well_known.alt_attr, inp_get_alt, inp_set_alt),
            (
                self.well_known.autocomplete_attr,
                inp_get_autocomplete,
                inp_set_autocomplete,
            ),
            (self.well_known.dir_name, inp_get_dir_name, inp_set_dir_name),
            (
                self.well_known.form_action,
                inp_get_form_action,
                inp_set_form_action,
            ),
            (
                self.well_known.form_enctype,
                inp_get_form_enctype,
                inp_set_form_enctype,
            ),
            (
                self.well_known.form_method,
                inp_get_form_method,
                inp_set_form_method,
            ),
            (
                self.well_known.form_target,
                inp_get_form_target,
                inp_set_form_target,
            ),
            (self.well_known.max_attr, inp_get_max, inp_set_max),
            (self.well_known.min_attr, inp_get_min, inp_set_min),
            (self.well_known.name, inp_get_name, inp_set_name),
            (
                self.well_known.pattern_attr,
                inp_get_pattern,
                inp_set_pattern,
            ),
            (
                self.well_known.placeholder,
                inp_get_placeholder,
                inp_set_placeholder,
            ),
            (self.well_known.src_attr, inp_get_src, inp_set_src),
            (self.well_known.step_attr, inp_get_step, inp_set_step),
            (
                self.well_known.default_value,
                inp_get_default_value,
                inp_set_default_value,
            ),
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
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.disabled,
                inp_get_disabled as super::super::NativeFn,
                inp_set_disabled as super::super::NativeFn,
            ),
            (
                self.well_known.multiple_attr,
                inp_get_multiple,
                inp_set_multiple,
            ),
            (
                self.well_known.read_only,
                inp_get_read_only,
                inp_set_read_only,
            ),
            (self.well_known.required, inp_get_required, inp_set_required),
            (
                self.well_known.autofocus,
                inp_get_autofocus,
                inp_set_autofocus,
            ),
            (
                self.well_known.form_no_validate,
                inp_get_form_no_validate,
                inp_set_form_no_validate,
            ),
            (
                self.well_known.default_checked,
                inp_get_default_checked,
                inp_set_default_checked,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Numeric reflects (unsigned long with default 0 / -1).
        self.install_accessor_pair(
            proto_id,
            self.well_known.max_length,
            inp_get_max_length,
            Some(inp_set_max_length),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.min_length,
            inp_get_min_length,
            Some(inp_set_min_length),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.size_attr,
            inp_get_size,
            Some(inp_set_size),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.width,
            inp_get_width,
            Some(inp_set_width),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.height,
            inp_get_height,
            Some(inp_set_height),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // type — enumerated reflect with "text" default.
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            inp_get_type,
            Some(inp_set_type),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // value / checked.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            inp_get_value,
            Some(inp_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.checked_attr,
            inp_get_checked,
            Some(inp_set_checked),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // valueAsNumber / valueAsDate.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value_as_number,
            inp_get_value_as_number,
            Some(inp_set_value_as_number),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.value_as_date,
            inp_get_value_as_date,
            Some(inp_set_value_as_date),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // form / labels.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            inp_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels_attr,
            inp_get_labels,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // files / list / showPicker — stubs with explicit slot binds.
        self.install_accessor_pair(
            proto_id,
            self.well_known.files_attr,
            inp_get_files,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.list_attr,
            inp_get_list,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.show_picker,
            inp_show_picker,
            shape::PropertyAttrs::METHOD,
        );

        // stepUp / stepDown.
        self.install_native_method(
            proto_id,
            self.well_known.step_up,
            inp_step_up,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.step_down,
            inp_step_down,
            shape::PropertyAttrs::METHOD,
        );

        // ConstraintValidation mixin (Phase 9) — `validity` /
        // `validationMessage` / `willValidate` accessors +
        // `checkValidity()` / `reportValidity()` /
        // `setCustomValidity()` methods.  Shared install helper in
        // `super::validity_state`.
        super::validity_state::install_constraint_validation_methods(self, proto_id);

        // Selection API — gated by text-control input types.
        selection_api::install_selection_api_members(
            self,
            proto_id,
            SelectionAccessors {
                get_start: inp_get_selection_start,
                set_start: inp_set_selection_start,
                get_end: inp_get_selection_end,
                set_end: inp_set_selection_end,
                get_direction: inp_get_selection_direction,
                set_direction: inp_set_selection_direction,
                select: inp_select_method,
                set_range_text: inp_set_range_text,
                set_selection_range: inp_set_selection_range,
            },
        );
    }
}

fn require_input_receiver(
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
    if !ctx.host().tag_matches_ascii_case(entity, "input") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// Selection-API brand check — also gates by `supports_selection`
/// per HTML §4.10.5.2.10.  Returns `Err(InvalidStateError)` for
/// non-text-control input types.
fn require_selection_input_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
        return Ok(None);
    };
    if !supports_selection(ctx, entity) {
        // Materialise the lower-cased type only on the error path
        // so the common (passing) path stays allocation-free.
        let t = input_type(ctx, entity);
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'HTMLInputElement': \
                 The input element's type ('{t}') does not support \
                 selection."
            ),
        ));
    }
    Ok(Some(entity))
}

// --- String reflects ----------------------------------------------

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

input_string_attr!(inp_get_accept, inp_set_accept, "accept", "accept");
input_string_attr!(inp_get_alt, inp_set_alt, "alt", "alt");
input_string_attr!(
    inp_get_autocomplete,
    inp_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
input_string_attr!(inp_get_dir_name, inp_set_dir_name, "dirname", "dirName");
input_string_attr!(
    inp_get_form_action,
    inp_set_form_action,
    "formaction",
    "formAction"
);
input_string_attr!(
    inp_get_form_enctype,
    inp_set_form_enctype,
    "formenctype",
    "formEnctype"
);
input_string_attr!(
    inp_get_form_method,
    inp_set_form_method,
    "formmethod",
    "formMethod"
);
input_string_attr!(
    inp_get_form_target,
    inp_set_form_target,
    "formtarget",
    "formTarget"
);
input_string_attr!(inp_get_max, inp_set_max, "max", "max");
input_string_attr!(inp_get_min, inp_set_min, "min", "min");
input_string_attr!(inp_get_name, inp_set_name, "name", "name");
input_string_attr!(inp_get_pattern, inp_set_pattern, "pattern", "pattern");
input_string_attr!(
    inp_get_placeholder,
    inp_set_placeholder,
    "placeholder",
    "placeholder"
);
input_string_attr!(inp_get_src, inp_set_src, "src", "src");
input_string_attr!(inp_get_step, inp_set_step, "step", "step");
input_string_attr!(
    inp_get_default_value,
    inp_set_default_value,
    "value",
    "defaultValue"
);

// --- Boolean reflects ---------------------------------------------

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

input_bool_attr!(inp_get_disabled, inp_set_disabled, "disabled", "disabled");
input_bool_attr!(inp_get_multiple, inp_set_multiple, "multiple", "multiple");
input_bool_attr!(inp_get_read_only, inp_set_read_only, "readonly", "readOnly");
input_bool_attr!(inp_get_required, inp_set_required, "required", "required");
input_bool_attr!(
    inp_get_autofocus,
    inp_set_autofocus,
    "autofocus",
    "autofocus"
);
input_bool_attr!(
    inp_get_form_no_validate,
    inp_set_form_no_validate,
    "formnovalidate",
    "formNoValidate"
);
input_bool_attr!(
    inp_get_default_checked,
    inp_set_default_checked,
    "checked",
    "defaultChecked"
);

// --- Numeric reflects (unsigned long) -----------------------------

fn read_unsigned_long_attr_with_default(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr: &str,
    default: u32,
) -> u32 {
    ctx.host().dom().with_attribute(entity, attr, |v| {
        v.and_then(|s| s.parse::<u32>().ok()).unwrap_or(default)
    })
}

macro_rules! input_unsigned_long {
    ($get:ident, $set:ident, $attr:expr, $label:expr, $default:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_input_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Number(f64::from($default)));
            };
            Ok(JsValue::Number(f64::from(
                read_unsigned_long_attr_with_default(ctx, entity, $attr, $default),
            )))
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
            let n = super::super::coerce::to_uint32(ctx.vm, val)?;
            ctx.host().dom().set_attribute(entity, $attr, n.to_string());
            Ok(JsValue::Undefined)
        }
    };
}

input_unsigned_long!(inp_get_size, inp_set_size, "size", "size", 20);
input_unsigned_long!(inp_get_width, inp_set_width, "width", "width", 0);
input_unsigned_long!(inp_get_height, inp_set_height, "height", "height", 0);

// maxLength / minLength — signed long (-1 default per HTML §4.10.5).
fn inp_get_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "maxLength")? else {
        return Ok(JsValue::Number(-1.0));
    };
    let n = ctx.host().dom().with_attribute(entity, "maxlength", |v| {
        v.and_then(|s| s.parse::<i32>().ok()).filter(|n| *n >= 0)
    });
    Ok(JsValue::Number(f64::from(n.unwrap_or(-1))))
}

fn inp_set_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "maxLength")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to set 'maxLength' on 'HTMLInputElement': value must be non-negative",
        ));
    }
    ctx.host()
        .dom()
        .set_attribute(entity, "maxlength", n.to_string());
    Ok(JsValue::Undefined)
}

fn inp_get_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "minLength")? else {
        return Ok(JsValue::Number(-1.0));
    };
    let n = ctx.host().dom().with_attribute(entity, "minlength", |v| {
        v.and_then(|s| s.parse::<i32>().ok()).filter(|n| *n >= 0)
    });
    Ok(JsValue::Number(f64::from(n.unwrap_or(-1))))
}

fn inp_set_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "minLength")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to set 'minLength' on 'HTMLInputElement': value must be non-negative",
        ));
    }
    ctx.host()
        .dom()
        .set_attribute(entity, "minlength", n.to_string());
    Ok(JsValue::Undefined)
}

// --- type (enumerated reflect, "text" default) --------------------

fn inp_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "type")? else {
        // No receiver — return the default ("text" interned).
        let sid = ctx.vm.strings.intern("text");
        return Ok(JsValue::String(sid));
    };
    let t = input_type(ctx, entity);
    let sid = ctx.vm.strings.intern(&t);
    Ok(JsValue::String(sid))
}

fn inp_set_type(
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
    // Setter writes verbatim — invalid values are accepted at the
    // attribute level and surfaced through the getter's
    // invalid-value fallback (HTML §4.10.5.1.18).
    ctx.host().dom().set_attribute(entity, "type", s);
    Ok(JsValue::Undefined)
}

// --- value / checked ----------------------------------------------

/// Read the input's IDL value — delegates the `dirty ?? default`
/// pattern to [`super::form_control_state::read_value`]; the closure
/// supplies the input-specific defaultValue source (the `value`
/// content attribute).
fn read_value(ctx: &mut NativeContext<'_>, entity: Entity) -> String {
    super::form_control_state::read_value(ctx, entity, |ctx, e| {
        ctx.host()
            .dom()
            .with_attribute(e, "value", |v| v.unwrap_or("").to_string())
    })
}

/// Allocation-free length variant of [`read_value`].
fn value_utf16_len(ctx: &mut NativeContext<'_>, entity: Entity) -> u32 {
    super::form_control_state::value_utf16_len(ctx, entity, |ctx, e| {
        ctx.host()
            .dom()
            .with_attribute(e, "value", |v| utf16_len(v.unwrap_or("")))
    })
}

fn inp_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_input_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    let s = read_value(ctx, entity);
    if s.is_empty() {
        return Ok(JsValue::String(empty));
    }
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

fn inp_set_value(
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
    let len = utf16_len(&s);
    let state = ctx.vm.form_control_state_mut(entity);
    state.dirty_value = Some(s);
    state.selection_start = len;
    state.selection_end = len;
    Ok(JsValue::Undefined)
}

fn inp_get_checked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "checked")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "checked"),
    ))
}

fn inp_set_checked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "checked")? else {
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
    Ok(JsValue::Undefined)
}

// --- valueAsNumber / valueAsDate ----------------------------------

fn inp_get_value_as_number(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "valueAsNumber")? else {
        return Ok(JsValue::Number(f64::NAN));
    };
    if !is_numeric_input_type(ctx, entity) {
        return Ok(JsValue::Number(f64::NAN));
    }
    let s = read_value(ctx, entity);
    Ok(JsValue::Number(s.trim().parse::<f64>().unwrap_or(f64::NAN)))
}

fn inp_set_value_as_number(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "valueAsNumber")? else {
        return Ok(JsValue::Undefined);
    };
    if !is_numeric_input_type(ctx, entity) {
        let t = input_type(ctx, entity);
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to set 'valueAsNumber' on 'HTMLInputElement': \
                 The input element's type ('{t}') does not support \
                 setting a numeric value."
            ),
        ));
    }
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_number(ctx.vm, val)?;
    let s = if n.is_nan() {
        String::new()
    } else if n == n.trunc() && n.is_finite() {
        // Integer-valued — emit without the trailing `.0`.
        format!("{n}")
    } else {
        format!("{n}")
    };
    let len = utf16_len(&s);
    let state = ctx.vm.form_control_state_mut(entity);
    state.dirty_value = Some(s);
    state.selection_start = len;
    state.selection_end = len;
    Ok(JsValue::Undefined)
}

fn inp_get_value_as_date(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "valueAsDate")? else {
        return Ok(JsValue::Null);
    };
    if !is_date_input_type(ctx, entity) {
        return Ok(JsValue::Null);
    }
    // Date integration deferred per the module docstring (slot
    // #11-input-value-as-date).  Returning `null` matches the spec
    // wording for an unparseable value but with no parsing yet —
    // any caller that reads this gets `null`.
    Ok(JsValue::Null)
}

fn inp_set_value_as_date(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "valueAsDate")? else {
        return Ok(JsValue::Undefined);
    };
    if !is_date_input_type(ctx, entity) {
        let t = input_type(ctx, entity);
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to set 'valueAsDate' on 'HTMLInputElement': \
                 The input element's type ('{t}') does not support \
                 setting a Date value."
            ),
        ));
    }
    // Date integration deferred — accept `null` as a clear, throw
    // TypeError for any other value pending the slot
    // #11-input-value-as-date follow-up.
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(val, JsValue::Null) {
        let state = ctx.vm.form_control_state_mut(entity);
        state.dirty_value = Some(String::new());
        return Ok(JsValue::Undefined);
    }
    Err(VmError::type_error(
        "Failed to set 'valueAsDate' on 'HTMLInputElement': \
         non-null Date values are deferred to slot #11-input-value-as-date"
            .to_string(),
    ))
}

// --- stepUp / stepDown --------------------------------------------

fn step_for(t: &str, step_attr: Option<&str>) -> Option<f64> {
    let default_step: f64 = match t {
        "number" | "range" => 1.0,
        "date" | "month" | "week" => 1.0,
        "time" => 60.0,
        "datetime-local" => 60.0,
        _ => return None,
    };
    if let Some(s) = step_attr {
        if s.eq_ignore_ascii_case("any") {
            // step="any" disables stepping per HTML §4.10.5.4.4.
            return None;
        }
        if let Ok(n) = s.parse::<f64>() {
            if n > 0.0 {
                return Some(n);
            }
        }
        // Invalid step → fall back to default.
        return Some(default_step);
    }
    Some(default_step)
}

fn step_apply(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &'static str,
    direction: f64,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    if !is_numeric_input_type(ctx, entity) {
        let t = input_type(ctx, entity);
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'HTMLInputElement': \
                 The input element's type ('{t}') does not support stepping."
            ),
        ));
    }
    // `step` lookup needs the canonical type name to drive the
    // default-step table (per `step_for`); the alloc here is bounded
    // to the stepUp/stepDown call rate (not an accessor hot path).
    let t = input_type(ctx, entity);
    let step_attr = ctx
        .host()
        .dom()
        .with_attribute(entity, "step", |v| v.map(String::from));
    let Some(step) = step_for(&t, step_attr.as_deref()) else {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'HTMLInputElement': \
                 step has no applicable default for type '{t}'."
            ),
        ));
    };
    let count_arg = args.first().copied().unwrap_or(JsValue::Number(1.0));
    let count = super::super::coerce::to_number(ctx.vm, count_arg)?;
    let cur = read_value(ctx, entity).trim().parse::<f64>().unwrap_or(0.0);
    let new = cur + direction * count * step;
    let s = format!("{new}");
    let len = utf16_len(&s);
    let state = ctx.vm.form_control_state_mut(entity);
    state.dirty_value = Some(s);
    state.selection_start = len;
    state.selection_end = len;
    Ok(JsValue::Undefined)
}

fn inp_step_up(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    step_apply(ctx, this, args, "stepUp", 1.0)
}

fn inp_step_down(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    step_apply(ctx, this, args, "stepDown", -1.0)
}

// --- form / labels ------------------------------------------------

fn inp_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    match super::form_assoc::resolve_form_association(ctx, entity) {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

fn inp_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "labels")? else {
        return Ok(JsValue::Null);
    };
    let labels = super::form_assoc::collect_labels_for(ctx, entity);
    let kind = super::dom_collection::LiveCollectionKind::Snapshot { entities: labels };
    let id = ctx.vm.alloc_collection(kind);
    Ok(JsValue::Object(id))
}

// --- files / list / showPicker (deferred stubs) -------------------

fn inp_get_files(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_input_receiver(ctx, this, "files")? else {
        return Ok(JsValue::Null);
    };
    if input_type_matches(ctx, entity, &["file"]) {
        // FileList exposure deferred to slot #11c-fl PR-file-api.
        // Per spec, `input.files` for `type=file` returns a
        // FileList of currently selected files; without the File
        // API, we can't model this — throw InvalidStateError so
        // any code that touches it surfaces the deferral instead
        // of silently misbehaving.
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "input.files is deferred to slot #11c-fl PR-file-api \
             (FileList / Blob exposure not yet implemented)",
        ));
    }
    Ok(JsValue::Null)
}

fn inp_get_list(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // HTMLDataListElement is bundled in slot #11-tags-T2; stub
    // returns null which matches "no datalist found" semantics.
    Ok(JsValue::Null)
}

fn inp_show_picker(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_input_receiver(ctx, this, "showPicker")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "input.showPicker() is deferred to slot #11-show-picker \
         (depends on shell platform-picker integration)",
    ))
}

// --- Selection API wrappers ---------------------------------------

fn inp_get_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Number(0.0));
    };
    selection_api::get_selection_start(ctx, entity)
}

fn inp_set_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let len = value_utf16_len(ctx, entity);
    selection_api::set_selection_start(ctx, entity, len, val)
}

fn inp_get_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Number(0.0));
    };
    selection_api::get_selection_end(ctx, entity)
}

fn inp_set_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let len = value_utf16_len(ctx, entity);
    selection_api::set_selection_end(ctx, entity, len, val)
}

fn inp_get_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::String(ctx.vm.well_known.none_str));
    };
    selection_api::get_selection_direction(ctx, entity)
}

fn inp_set_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    selection_api::set_selection_direction(ctx, entity, val)
}

fn inp_select_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "select")? else {
        return Ok(JsValue::Undefined);
    };
    let len = value_utf16_len(ctx, entity);
    selection_api::select_all(ctx, entity, len)
}

fn inp_set_range_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "setRangeText")? else {
        return Ok(JsValue::Undefined);
    };
    let value = read_value(ctx, entity);
    let (new_value, new_start, new_end) =
        selection_api::compute_set_range_text(ctx, entity, &value, args)?;
    let state = ctx.vm.form_control_state_mut(entity);
    state.dirty_value = Some(new_value);
    state.selection_start = new_start;
    state.selection_end = new_end;
    Ok(JsValue::Undefined)
}

fn inp_set_selection_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_selection_input_receiver(ctx, this, "setSelectionRange")? else {
        return Ok(JsValue::Undefined);
    };
    let len = value_utf16_len(ctx, entity);
    selection_api::set_selection_range(ctx, entity, len, args)
}
