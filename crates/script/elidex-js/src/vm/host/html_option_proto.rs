//! `HTMLOptionElement.prototype` intrinsic — per-tag prototype layer
//! for `<option>` wrappers (HTML §4.10.10).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! reflected attribute shaping, and JsValue↔Entity marshalling.
//! `<option>`'s spec is mostly direct DOM-attribute reflection
//! (`disabled`, `label`, `value`, `selected`); the IDL `selected`
//! dirty-tracking gap is documented inline (currently aliased to
//! `defaultSelected` — full dirty tracking lands when
//! `elidex-form::OptionState` is hoisted in a follow-up).
//!
//! ## Members installed
//!
//! - `disabled` — boolean reflect of the `disabled` content attribute.
//! - `label` — DOMString reflect; falls back to `text` when absent.
//! - `value` — DOMString reflect; falls back to `text` when absent.
//! - `text` — derived from the option's text content (HTML
//!   §4.10.10.1), reading via Node.textContent and writing back
//!   through the same channel.
//! - `defaultSelected` — boolean reflect of the `selected` content
//!   attribute.
//! - `selected` — IDL state aliased to `defaultSelected` (Phase 2
//!   approximation; dirty-tracked semantics land with
//!   `elidex-form::OptionState`).
//! - `index` — read-only index of this option in the parent
//!   `<select>` / `<datalist>`'s flat option list (-1 if no parent).
//! - `form` — read-only walks up via the parent `<select>` and
//!   uses `elidex_form::find_form_ancestor`.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Allocate `HTMLOptionElement.prototype` chained to
    /// `HTMLElement.prototype`.
    pub(in crate::vm) fn register_html_option_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_option_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_option_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // disabled — boolean reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.disabled,
            native_option_get_disabled,
            Some(native_option_set_disabled),
            attrs,
        );
        // label — DOMString reflect (falls back to text when absent).
        self.install_accessor_pair(
            proto_id,
            self.well_known.label_attr,
            native_option_get_label,
            Some(native_option_set_label),
            attrs,
        );
        // value — DOMString reflect (falls back to text when absent).
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_option_get_value,
            Some(native_option_set_value),
            attrs,
        );
        // text — derived from textContent.
        self.install_accessor_pair(
            proto_id,
            self.well_known.text,
            native_option_get_text,
            Some(native_option_set_text),
            attrs,
        );
        // defaultSelected — boolean reflect of `selected`.
        self.install_accessor_pair(
            proto_id,
            self.well_known.default_selected,
            native_option_get_default_selected,
            Some(native_option_set_default_selected),
            attrs,
        );
        // selected — Phase 2 approximation: aliased to
        // defaultSelected.  Dirty-tracking lands with
        // `elidex-form::OptionState` follow-up.
        self.install_accessor_pair(
            proto_id,
            self.well_known.selected,
            native_option_get_default_selected,
            Some(native_option_set_default_selected),
            attrs,
        );
        // index — read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.index,
            native_option_get_index,
            None,
            attrs,
        );
        // form — read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_option_get_form,
            None,
            attrs,
        );
    }
}

fn require_option_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLOptionElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "option") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOptionElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// Boolean reflect helpers
// ---------------------------------------------------------------------------

fn bool_reflect_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    attr: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, method)? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, attr),
    ))
}

fn bool_reflect_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    attr: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host().dom().set_attribute(entity, attr, String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, attr);
    }
    Ok(JsValue::Undefined)
}

fn native_option_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_reflect_get(ctx, this, "disabled", "disabled")
}

fn native_option_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_reflect_set(ctx, this, args, "disabled", "disabled")
}

fn native_option_get_default_selected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_reflect_get(ctx, this, "defaultSelected", "selected")
}

fn native_option_set_default_selected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    bool_reflect_set(ctx, this, args, "defaultSelected", "selected")
}

// ---------------------------------------------------------------------------
// label / value — DOMString reflect with `text` fallback
// ---------------------------------------------------------------------------

fn native_option_get_label(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_option_receiver(ctx, this, "label")? else {
        return Ok(JsValue::String(empty));
    };
    let attr_present = ctx.host().dom().has_attribute(entity, "label");
    if attr_present {
        let sid = match ctx.dom_and_strings_if_bound() {
            Some((dom, strings)) => {
                dom.with_attribute(entity, "label", |v| v.map_or(empty, |s| strings.intern(s)))
            }
            None => empty,
        };
        return Ok(JsValue::String(sid));
    }
    // Fall back to text content (HTML §4.10.10.2).
    option_text_value(ctx, entity)
}

fn native_option_set_label(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "label")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "label", s);
    Ok(JsValue::Undefined)
}

fn native_option_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_option_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    let attr_present = ctx.host().dom().has_attribute(entity, "value");
    if attr_present {
        let sid = match ctx.dom_and_strings_if_bound() {
            Some((dom, strings)) => {
                dom.with_attribute(entity, "value", |v| v.map_or(empty, |s| strings.intern(s)))
            }
            None => empty,
        };
        return Ok(JsValue::String(sid));
    }
    // Fall back to text content (HTML §4.10.10).
    option_text_value(ctx, entity)
}

fn native_option_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, "value", s);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// text — Node.textContent shim
// ---------------------------------------------------------------------------

fn option_text_value(ctx: &mut NativeContext<'_>, entity: Entity) -> Result<JsValue, VmError> {
    super::dom_bridge::invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn native_option_get_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "text")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    option_text_value(ctx, entity)
}

fn native_option_set_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "text")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let coerced = JsValue::String(super::super::coerce::to_string(ctx.vm, val)?);
    super::dom_bridge::invoke_dom_api(ctx, "textContent.set", entity, &[coerced])?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// index / form
// ---------------------------------------------------------------------------

fn native_option_get_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "index")? else {
        return Ok(JsValue::Number(-1.0));
    };
    // HTML §4.10.10 `option.index` algorithm hoisted to
    // `elidex_form::find_option_index_in_tree` per CLAUDE.md
    // "Layering mandate" — the ancestor walk + descendant counter
    // is engine-independent and was previously a `walk_options`
    // recursion + `find_options_container` ancestor walk inline
    // here.
    let idx = elidex_form::find_option_index_in_tree(ctx.host().dom(), entity).unwrap_or(-1);
    Ok(JsValue::Number(f64::from(idx)))
}

fn native_option_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_option_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    // HTML §4.10.10: option.form returns the form owner of the
    // option's select ancestor, walking through optgroup if needed.
    // Bounded by `MAX_ANCESTOR_DEPTH` so a hypothetical
    // `TreeRelation` cycle / corruption (e.g. from a buggy
    // `appendChild` cycle-check regression) cannot wedge this
    // accessor in an infinite loop — matches the convention used by
    // `elidex_form::find_form_ancestor` and other ancestor walkers
    // hoist target slot #11-tags-T1-v2-drift-hoist (D-1) inherits.
    let dom = ctx.host().dom();
    let mut current = dom.get_parent(entity);
    let mut select: Option<Entity> = None;
    for _ in 0..elidex_ecs::MAX_ANCESTOR_DEPTH {
        let Some(p) = current else {
            break;
        };
        if ctx.host().tag_matches_ascii_case(p, "select") {
            select = Some(p);
            break;
        }
        current = ctx.host().dom().get_parent(p);
    }
    let Some(select_entity) = select else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), select_entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}
