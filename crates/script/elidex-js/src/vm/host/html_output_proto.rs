//! `HTMLOutputElement.prototype` intrinsic — per-tag prototype layer
//! for `<output>` wrappers (HTML §4.10.13, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.10.13):
//!
//! - `htmlFor` — `[SameObject, PutForwards=value]` DOMTokenList backed
//!   by the `for` content attribute.  Identity is preserved per
//!   `[SameObject]` via [`VmInner::output_html_for_wrappers`]; the
//!   `[PutForwards=value]` rewrite is handled by the engine assignment
//!   path so `output.htmlFor = "id1 id2"` is equivalent to
//!   `output.htmlFor.value = "id1 id2"`.  This getter therefore only
//!   needs to surface the cached DOMTokenList wrapper.
//! - `form` — form-owner accessor (descendant-of-form lookup, same
//!   resolution path as `<input>.form`).
//! - `name` — DOMString reflect of the `name` content attribute.
//! - `type` — DOMString readonly, returns the constant `"output"`.
//! - `defaultValue` — DOMString state.  Stored in
//!   [`elidex_ecs::OutputDefaultValue`]; getter falls through to
//!   descendant text content when the component is absent.  Setter
//!   writes the component AND, when the element is in default mode
//!   (no [`elidex_ecs::OutputValueOverride`]), updates the displayed
//!   text content via the engine-indep `textContent.set` handler.
//! - `value` — DOMString.  Getter returns the value-mode override
//!   (`OutputValueOverride.0`) when set, otherwise descendant text
//!   content.  Setter writes the override AND replaces the displayed
//!   text content.  Switching from default-mode to value-mode happens
//!   transparently (component insertion).
//! - `labels` — empty `NodeList` stub (matches `<input>.labels` —
//!   live-walker is `#11-form-labels-walker`).
//! - ConstraintValidation mixin (`willValidate` / `validity` /
//!   `validationMessage` / `checkValidity` / `reportValidity` /
//!   `setCustomValidity`) installed by the parent `register_globals`
//!   block via [`VmInner::install_constraint_validation_mixin`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  ECS state
//! reads/writes use the existing engine-indep components; textContent
//! display rewrites route through the engine-indep `textContent.set`
//! DomApiHandler so the spec-mandated children replacement reuses the
//! canonical write path (no bespoke tree mutation here).

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind, OutputDefaultValue, OutputValueOverride};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_output_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_output_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_output_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        let html_for_sid = self.strings.intern("htmlFor");
        self.install_accessor_pair(
            proto_id,
            html_for_sid,
            output_get_html_for,
            Some(output_set_html_for),
            attrs,
        );

        let form_sid = self.strings.intern("form");
        self.install_accessor_pair(proto_id, form_sid, output_get_form, None, attrs);

        let name_sid = self.strings.intern("name");
        self.install_accessor_pair(
            proto_id,
            name_sid,
            output_get_name,
            Some(output_set_name),
            attrs,
        );

        let type_sid = self.strings.intern("type");
        self.install_accessor_pair(proto_id, type_sid, output_get_type, None, attrs);

        let default_value_sid = self.strings.intern("defaultValue");
        self.install_accessor_pair(
            proto_id,
            default_value_sid,
            output_get_default_value,
            Some(output_set_default_value),
            attrs,
        );

        let value_sid = self.strings.intern("value");
        self.install_accessor_pair(
            proto_id,
            value_sid,
            output_get_value,
            Some(output_set_value),
            attrs,
        );

        let labels_sid = self.strings.intern("labels");
        self.install_accessor_pair(proto_id, labels_sid, output_get_labels, None, attrs);
    }
}

fn require_output_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLOutputElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "output") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOutputElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// htmlFor (DOMTokenList) / form / name / type
// ---------------------------------------------------------------------------

fn output_get_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "htmlFor")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_output_html_for(entity);
    Ok(JsValue::Object(id))
}

/// `<output>.htmlFor` setter — `[PutForwards=value]` semantics:
/// assigning to `output.htmlFor` is equivalent to
/// `output.htmlFor.value = ToString(v)`, which writes the `for`
/// content attribute directly.  Mirrors the
/// [`super::html_link_proto::link_set_sizes`] precedent.
fn output_set_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "htmlFor")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("for");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn output_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(form_entity) = elidex_form::find_form_ancestor(ctx.host().dom(), entity) else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.create_element_wrapper(form_entity);
    Ok(JsValue::Object(id))
}

fn output_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "name")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("name");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn output_set_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "name")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("name");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn output_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_output_receiver(ctx, this, "type")?;
    let sid = ctx.vm.strings.intern("output");
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// defaultValue / value state machine
// ---------------------------------------------------------------------------

fn output_get_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    if let Some(stored) = read_default_value(ctx, entity) {
        let sid = ctx.vm.strings.intern(&stored);
        return Ok(JsValue::String(sid));
    }
    // No explicit default: fall through to descendant text content.
    invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn output_set_default_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "defaultValue")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let owned: String = ctx.vm.strings.get_utf8(value_sid);
    let in_default_mode = !has_value_override(ctx, entity);
    write_default_value(ctx, entity, owned);
    if in_default_mode {
        // Spec "run the rules to update the displayed value" — when in
        // default mode, the displayed text content tracks the default.
        invoke_dom_api(
            ctx,
            "textContent.set",
            entity,
            &[JsValue::String(value_sid)],
        )?;
    }
    Ok(JsValue::Undefined)
}

fn output_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    if let Some(override_value) = read_value_override(ctx, entity) {
        let sid = ctx.vm.strings.intern(&override_value);
        return Ok(JsValue::String(sid));
    }
    invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn output_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_output_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // Snapshot the current text content into `OutputDefaultValue` if
    // it has not been written before — without this, switching to
    // value mode loses the implicit default (which the spec says
    // tracks descendant text content while in default mode).  Form
    // reset later restores from this snapshot.
    if ctx
        .host()
        .dom()
        .world()
        .get::<&OutputDefaultValue>(entity)
        .is_err()
    {
        let snapshot = invoke_dom_api(ctx, "textContent.get", entity, &[])?;
        if let JsValue::String(text_sid) = snapshot {
            let snapshot_owned = ctx.vm.strings.get_utf8(text_sid);
            write_default_value(ctx, entity, snapshot_owned);
        }
    }
    let owned: String = ctx.vm.strings.get_utf8(value_sid);
    write_value_override(ctx, entity, owned);
    invoke_dom_api(
        ctx,
        "textContent.set",
        entity,
        &[JsValue::String(value_sid)],
    )?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// labels (stub)
// ---------------------------------------------------------------------------

fn output_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_output_receiver(ctx, this, "labels")?;
    Ok(JsValue::Object(
        super::dom_collection::empty_labels_collection(ctx.vm),
    ))
}

// ---------------------------------------------------------------------------
// ECS state helpers
// ---------------------------------------------------------------------------

fn read_default_value(ctx: &mut NativeContext<'_>, entity: Entity) -> Option<String> {
    ctx.host()
        .dom()
        .world()
        .get::<&OutputDefaultValue>(entity)
        .ok()
        .map(|d| d.0.clone())
}

fn write_default_value(ctx: &mut NativeContext<'_>, entity: Entity, value: String) {
    let world = ctx.host().dom().world_mut();
    if let Ok(mut existing) = world.get::<&mut OutputDefaultValue>(entity) {
        existing.0 = value;
        return;
    }
    let _ = world.insert_one(entity, OutputDefaultValue(value));
}

fn has_value_override(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    ctx.host()
        .dom()
        .world()
        .get::<&OutputValueOverride>(entity)
        .is_ok_and(|ov| ov.0.is_some())
}

fn read_value_override(ctx: &mut NativeContext<'_>, entity: Entity) -> Option<String> {
    ctx.host()
        .dom()
        .world()
        .get::<&OutputValueOverride>(entity)
        .ok()
        .and_then(|ov| ov.0.clone())
}

fn write_value_override(ctx: &mut NativeContext<'_>, entity: Entity, value: String) {
    let world = ctx.host().dom().world_mut();
    if let Ok(mut existing) = world.get::<&mut OutputValueOverride>(entity) {
        existing.0 = Some(value);
        return;
    }
    let _ = world.insert_one(entity, OutputValueOverride(Some(value)));
}
