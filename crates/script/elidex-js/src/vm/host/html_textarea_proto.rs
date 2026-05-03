//! `HTMLTextAreaElement.prototype` intrinsic — per-tag prototype
//! layer for `<textarea>` wrappers (HTML §4.10.11).
//!
//! Chain (slot #11-tags-T1 Phase 6):
//!
//! ```text
//! textarea wrapper
//!   → HTMLTextAreaElement.prototype
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **Reflected attrs** (DOMString unless noted):
//!   `autocomplete` / `cols` (`unsigned long`) / `dirName` (`dirname`) /
//!   `disabled` (boolean) / `maxLength` (`maxlength`, `long`) /
//!   `minLength` (`minlength`, `long`) / `name` / `placeholder` /
//!   `readOnly` (`readonly`, boolean) / `required` (boolean) /
//!   `rows` (`unsigned long`) / `wrap` / `defaultValue`.
//! - **`value`** / **`textLength`** — value getter/setter backed by
//!   the per-element dirty-value slot in
//!   [`super::form_control_state::FormControlEntityState`]; falls
//!   back to the textContent (defaultValue) when the IDL setter has
//!   not fired.  `textLength` is the UTF-16 length of `value`.
//! - **`form`** / **`labels`** — derived getters that route through
//!   the shared [`super::form_assoc::resolve_form_association`] /
//!   [`super::form_assoc::collect_labels_for`] helpers.
//! - **Selection API** — `selectionStart` / `selectionEnd` /
//!   `selectionDirection` accessors plus `select()` / `setRangeText()`
//!   / `setSelectionRange()` methods.  Installed via the shared
//!   [`super::selection_api`] module so the HTMLInputElement
//!   prototype (Phase 8) can re-use the same install pattern.
//!
//! ## Deferrals (slot bind)
//!
//! - **ConstraintValidation** (`checkValidity` / `reportValidity` /
//!   `setCustomValidity`) → **Phase 9** of this PR via the shared
//!   `install_constraint_validation_methods` helper.
//! - **`form.reset()` integration** — Phase 9 / slot
//!   `#11c-followup-reset-form`.  When the form's reset algorithm
//!   walks descendants, the `dirty_value` slot here clears so the
//!   `value` accessor falls back to the defaultValue.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;
use super::form_control_state::utf16_len;
use super::selection_api::{self, SelectionAccessors};

const INTERFACE: &str = "HTMLTextAreaElement";

impl VmInner {
    /// Allocate `HTMLTextAreaElement.prototype`.  Must run after
    /// `register_html_element_prototype`.
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

        // String-reflect attributes.
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.autocomplete_attr,
                ta_get_autocomplete as super::super::NativeFn,
                ta_set_autocomplete as super::super::NativeFn,
            ),
            (self.well_known.dir_name, ta_get_dir_name, ta_set_dir_name),
            (self.well_known.name, ta_get_name, ta_set_name),
            (
                self.well_known.placeholder,
                ta_get_placeholder,
                ta_set_placeholder,
            ),
            (self.well_known.wrap_attr, ta_get_wrap, ta_set_wrap),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Boolean reflects.  Attribute names are baked into each
        // generated getter/setter via [`textarea_bool_attr!`], so the
        // install loop only needs the prop sid + fn pointers.
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.disabled,
                ta_get_disabled as super::super::NativeFn,
                ta_set_disabled as super::super::NativeFn,
            ),
            (
                self.well_known.read_only,
                ta_get_read_only,
                ta_set_read_only,
            ),
            (self.well_known.required, ta_get_required, ta_set_required),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Numeric reflects — cols / rows default to 20 / 2 per HTML
        // §4.10.11.4 when missing or zero / non-numeric; maxLength /
        // minLength default to -1 and are signed `long` per spec.
        self.install_accessor_pair(
            proto_id,
            self.well_known.cols_attr,
            ta_get_cols,
            Some(ta_set_cols),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.rows_attr,
            ta_get_rows,
            Some(ta_set_rows),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.max_length,
            ta_get_max_length,
            Some(ta_set_max_length),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.min_length,
            ta_get_min_length,
            Some(ta_set_min_length),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // value / defaultValue / textLength.
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            ta_get_value,
            Some(ta_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_value,
            ta_get_default_value,
            Some(ta_set_default_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.text_length,
            ta_get_text_length,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // form / labels.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            ta_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels_attr,
            ta_get_labels,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Selection API — shared with HTMLInputElement (Phase 8) via
        // [`super::selection_api`].
        selection_api::install_selection_api_members(
            self,
            proto_id,
            SelectionAccessors {
                get_start: ta_get_selection_start,
                set_start: ta_set_selection_start,
                get_end: ta_get_selection_end,
                set_end: ta_set_selection_end,
                get_direction: ta_get_selection_direction,
                set_direction: ta_set_selection_direction,
                select: ta_select_method,
                set_range_text: ta_set_range_text,
                set_selection_range: ta_set_selection_range,
            },
        );
    }
}

fn require_textarea_receiver(
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
    if !ctx.host().tag_matches_ascii_case(entity, "textarea") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// --- String-reflect macro -----------------------------------------

macro_rules! textarea_string_attr {
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

textarea_string_attr!(
    ta_get_autocomplete,
    ta_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
textarea_string_attr!(ta_get_dir_name, ta_set_dir_name, "dirname", "dirName");
textarea_string_attr!(ta_get_name, ta_set_name, "name", "name");
textarea_string_attr!(
    ta_get_placeholder,
    ta_set_placeholder,
    "placeholder",
    "placeholder"
);
textarea_string_attr!(ta_get_wrap, ta_set_wrap, "wrap", "wrap");

// --- Boolean-reflect helpers --------------------------------------

macro_rules! textarea_bool_attr {
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

textarea_bool_attr!(ta_get_disabled, ta_set_disabled, "disabled", "disabled");
textarea_bool_attr!(ta_get_read_only, ta_set_read_only, "readonly", "readOnly");
textarea_bool_attr!(ta_get_required, ta_set_required, "required", "required");

// --- Numeric reflects ---------------------------------------------

/// Reflect an `unsigned long` attribute with a default value when
/// the attribute is absent or fails to parse to a positive integer.
fn read_unsigned_long_attr(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr: &str,
    default: u32,
) -> u32 {
    ctx.host().dom().with_attribute(entity, attr, |v| {
        v.and_then(|s| s.parse::<u32>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(default)
    })
}

fn ta_get_cols(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "cols")? else {
        return Ok(JsValue::Number(20.0));
    };
    Ok(JsValue::Number(f64::from(read_unsigned_long_attr(
        ctx, entity, "cols", 20,
    ))))
}

fn ta_set_cols(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "cols")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    // Per HTML §4.10.11.4, setting cols to 0 throws IndexSizeError;
    // we surface it as a TypeError-equivalent DOMException so the
    // setter does not silently swallow an invalid integer.  Use the
    // legacy IndexSizeError path that maps to InvalidStateError in
    // our well_known set (no IndexSizeError StringId allocated yet —
    // tracked under slot #11-validation-ui).
    if n == 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to set 'cols' on 'HTMLTextAreaElement': value must be a positive integer",
        ));
    }
    ctx.host()
        .dom()
        .set_attribute(entity, "cols", n.to_string());
    Ok(JsValue::Undefined)
}

fn ta_get_rows(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "rows")? else {
        return Ok(JsValue::Number(2.0));
    };
    Ok(JsValue::Number(f64::from(read_unsigned_long_attr(
        ctx, entity, "rows", 2,
    ))))
}

fn ta_set_rows(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "rows")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    if n == 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to set 'rows' on 'HTMLTextAreaElement': value must be a positive integer",
        ));
    }
    ctx.host()
        .dom()
        .set_attribute(entity, "rows", n.to_string());
    Ok(JsValue::Undefined)
}

fn ta_get_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "maxLength")? else {
        return Ok(JsValue::Number(-1.0));
    };
    let n = ctx.host().dom().with_attribute(entity, "maxlength", |v| {
        v.and_then(|s| s.parse::<i32>().ok()).filter(|n| *n >= 0)
    });
    Ok(JsValue::Number(f64::from(n.unwrap_or(-1))))
}

fn ta_set_max_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "maxLength")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to set 'maxLength' on 'HTMLTextAreaElement': value must be non-negative",
        ));
    }
    ctx.host()
        .dom()
        .set_attribute(entity, "maxlength", n.to_string());
    Ok(JsValue::Undefined)
}

fn ta_get_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "minLength")? else {
        return Ok(JsValue::Number(-1.0));
    };
    let n = ctx.host().dom().with_attribute(entity, "minlength", |v| {
        v.and_then(|s| s.parse::<i32>().ok()).filter(|n| *n >= 0)
    });
    Ok(JsValue::Number(f64::from(n.unwrap_or(-1))))
}

fn ta_set_min_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "minLength")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to set 'minLength' on 'HTMLTextAreaElement': value must be non-negative",
        ));
    }
    ctx.host()
        .dom()
        .set_attribute(entity, "minlength", n.to_string());
    Ok(JsValue::Undefined)
}

// --- value / defaultValue / textLength ----------------------------

/// Compute the textarea's defaultValue per HTML §4.10.11.5 — the
/// element's child text content (concatenated TextContent of every
/// descendant Text node, in tree order).
fn read_default_value(ctx: &mut NativeContext<'_>, entity: Entity) -> String {
    let dom = ctx.host().dom();
    let mut buf = String::new();
    dom.traverse_descendants(entity, |e| {
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
            buf.push_str(&text.0);
        }
        true
    });
    buf
}

/// Compute the textarea's IDL value — `dirty_value` slot when set,
/// otherwise the defaultValue (textContent walk).
fn read_value(ctx: &mut NativeContext<'_>, entity: Entity) -> String {
    if let Some(dirty) = ctx.vm.form_control_dirty_value(entity) {
        return dirty.to_string();
    }
    read_default_value(ctx, entity)
}

fn ta_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_textarea_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    let s = read_value(ctx, entity);
    if s.is_empty() {
        return Ok(JsValue::String(empty));
    }
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

fn ta_set_value(
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
    // Clamp existing selection to the new value's length per HTML
    // §4.10.18.6 step 3 (value setter resets selection to the end).
    let len = utf16_len(&s);
    let state = ctx.vm.form_control_state_mut(entity);
    state.dirty_value = Some(s);
    state.selection_start = len;
    state.selection_end = len;
    Ok(JsValue::Undefined)
}

fn ta_get_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_textarea_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::String(empty));
    };
    let s = read_default_value(ctx, entity);
    if s.is_empty() {
        return Ok(JsValue::String(empty));
    }
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

fn ta_set_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    // HTML §4.10.11.5: replace the element's child text with a
    // single Text node containing the new value.  Mirrors the
    // `textContent =` setter on Element.
    let dom = ctx.host().dom();
    let existing: Vec<Entity> = dom.children_iter(entity).collect();
    for child in existing {
        let _ = dom.remove_child(entity, child);
    }
    if !s.is_empty() {
        let text_entity = dom.create_text(s);
        let _ = dom.append_child(entity, text_entity);
    }
    Ok(JsValue::Undefined)
}

fn ta_get_text_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "textLength")? else {
        return Ok(JsValue::Number(0.0));
    };
    let s = read_value(ctx, entity);
    Ok(JsValue::Number(f64::from(utf16_len(&s))))
}

// --- form / labels ------------------------------------------------

fn ta_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    match super::form_assoc::resolve_form_association(ctx, entity) {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

fn ta_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "labels")? else {
        return Ok(JsValue::Null);
    };
    let labels = super::form_assoc::collect_labels_for(ctx, entity);
    let kind = super::dom_collection::LiveCollectionKind::Snapshot { entities: labels };
    let id = ctx.vm.alloc_collection(kind);
    Ok(JsValue::Object(id))
}

// --- Selection API ------------------------------------------------
//
// Each native here brand-checks the receiver and then routes into
// the shared helpers in [`super::selection_api`].  The `value`
// argument that the helpers need is materialised here via
// `read_value` so the textarea-specific defaultValue (textContent
// fallback) routes correctly.

fn ta_get_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Number(0.0));
    };
    selection_api::get_selection_start(ctx, entity)
}

fn ta_set_selection_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "selectionStart")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let value = read_value(ctx, entity);
    selection_api::set_selection_start(ctx, entity, &value, val)
}

fn ta_get_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Number(0.0));
    };
    selection_api::get_selection_end(ctx, entity)
}

fn ta_set_selection_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "selectionEnd")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let value = read_value(ctx, entity);
    selection_api::set_selection_end(ctx, entity, &value, val)
}

fn ta_get_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::String(ctx.vm.well_known.none_str));
    };
    selection_api::get_selection_direction(ctx, entity)
}

fn ta_set_selection_direction(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "selectionDirection")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    selection_api::set_selection_direction(ctx, entity, val)
}

fn ta_select_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "select")? else {
        return Ok(JsValue::Undefined);
    };
    let value = read_value(ctx, entity);
    selection_api::select_all(ctx, entity, &value)
}

fn ta_set_range_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "setRangeText")? else {
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

fn ta_set_selection_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_textarea_receiver(ctx, this, "setSelectionRange")? else {
        return Ok(JsValue::Undefined);
    };
    let value = read_value(ctx, entity);
    selection_api::set_selection_range(ctx, entity, &value, args)
}
