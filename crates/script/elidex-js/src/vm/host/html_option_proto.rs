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
    // Walk to ancestor select or datalist.  Per HTML §4.10.10
    // step 1: index = position in the list returned by the
    // select.options / datalist.options getter (or -1 if no
    // ancestor option-list container).  Direct parents that
    // qualify: `<select>` and `<datalist>`.  Indirect via
    // `<optgroup>`: `<optgroup>` whose own parent is one of the
    // two qualifies (per §4.10.10 optgroup nesting under
    // datalist is also valid).
    let Some(parent) = ctx.host().dom().get_parent(entity) else {
        return Ok(JsValue::Number(-1.0));
    };
    let parent_is_select = ctx.host().tag_matches_ascii_case(parent, "select");
    let parent_is_datalist = ctx.host().tag_matches_ascii_case(parent, "datalist");
    let parent_is_container = parent_is_select || parent_is_datalist;
    let parent_is_optgroup =
        !parent_is_container && ctx.host().tag_matches_ascii_case(parent, "optgroup");
    let grandparent = if parent_is_optgroup {
        ctx.host().dom().get_parent(parent)
    } else {
        None
    };
    let optgroup_grand = grandparent.is_some_and(|gp| {
        ctx.host().tag_matches_ascii_case(gp, "select")
            || ctx.host().tag_matches_ascii_case(gp, "datalist")
    });
    if !parent_is_container && !optgroup_grand {
        return Ok(JsValue::Number(-1.0));
    }
    let container_entity = if parent_is_container {
        parent
    } else {
        // optgroup → select / datalist grandparent (`grandparent`
        // is Some here because `optgroup_grand` was true).
        grandparent.unwrap_or(parent)
    };
    let mut count: u32 = 0;
    let mut found: i32 = -1;
    walk_options(
        ctx.host().dom(),
        container_entity,
        &mut count,
        entity,
        &mut found,
    );
    Ok(JsValue::Number(f64::from(found)))
}

fn walk_options(
    dom: &elidex_ecs::EcsDom,
    parent: Entity,
    count: &mut u32,
    target: Entity,
    found: &mut i32,
) {
    let Some(mut child) = dom.get_first_child(parent) else {
        return;
    };
    loop {
        let tag_is_option = dom
            .world()
            .get::<&elidex_ecs::TagType>(child)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("option"));
        let tag_is_optgroup = dom
            .world()
            .get::<&elidex_ecs::TagType>(child)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("optgroup"));
        if tag_is_option {
            if child == target {
                // u32 → i32 cast: forms typically hold ≤ a few
                // hundred options; far below i32::MAX.  Saturating
                // is a safe ceiling for the unlikely overflow.
                *found = i32::try_from(*count).unwrap_or(i32::MAX);
                // Index is fully determined; bail out of any
                // remaining sibling / optgroup-recursion work.
                return;
            }
            *count += 1;
        } else if tag_is_optgroup {
            walk_options(dom, child, count, target, found);
            if *found >= 0 {
                // Recursive call located the target; unwind.
                return;
            }
        }
        let Some(next) = dom.get_next_sibling(child) else {
            return;
        };
        child = next;
    }
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
    let dom = ctx.host().dom();
    let mut current = dom.get_parent(entity);
    let mut select: Option<Entity> = None;
    while let Some(p) = current {
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
