//! `HTMLProgressElement.prototype` intrinsic — per-tag prototype layer
//! for `<progress>` wrappers (HTML §4.10.14, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.10.14):
//! - `value` — `double`.  Reflect of the `value` content attribute,
//!   default 0, clamped to `0..=max` per spec.
//! - `max` — `double`.  Default 1; if the parsed value is `<= 0`,
//!   the spec says treat as 1.
//! - `position` — `double` readonly.  Returns `-1` (indeterminate) if
//!   no `value` content attribute is present, otherwise
//!   `clamp(value, 0, max) / max`.
//! - `labels` — empty `NodeList` stub.
//!
//! Setters route the JS Number through
//! [`super::idl_coerce::coerce_double_idl_arg`] (which delegates to
//! `coerce::to_number` so `valueOf` / `toString` on objects fires per
//! WebIDL §3.10.5), then serialise via the VM's ES `Number::ToString`
//! (`coerce::to_string`) so reflected attribute values match browser
//! semantics (notably `-0` → `"0"`, distinct from Rust `Display`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  The HTML
//! §2.4.4.3 floating-point parse algorithm lives engine-indep in
//! `elidex_dom_api::element::numeric_reflect::parse_double_or_default`.

#![cfg(feature = "engine")]

use elidex_dom_api::element::numeric_reflect::parse_double_or_default;
use elidex_ecs::{Attributes, Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::coerce_double_idl_arg;

impl VmInner {
    pub(in crate::vm) fn register_html_progress_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_progress_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_progress_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        let value_sid = self.strings.intern("value");
        self.install_accessor_pair(
            proto_id,
            value_sid,
            progress_get_value,
            Some(progress_set_value),
            attrs,
        );
        let max_sid = self.strings.intern("max");
        self.install_accessor_pair(
            proto_id,
            max_sid,
            progress_get_max,
            Some(progress_set_max),
            attrs,
        );
        let position_sid = self.strings.intern("position");
        self.install_accessor_pair(proto_id, position_sid, progress_get_position, None, attrs);
        let labels_sid = self.strings.intern("labels");
        self.install_accessor_pair(proto_id, labels_sid, progress_get_labels, None, attrs);
    }
}

fn require_progress_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLProgressElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "progress") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLProgressElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// Read a content attribute (raw, pre-parse) for the receiver.  Returns
/// `None` when the element has no Attributes component (unbound /
/// not-an-element) or the attribute is absent.
fn read_attr_raw(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) -> Option<String> {
    ctx.host()
        .dom()
        .world()
        .get::<&Attributes>(entity)
        .ok()
        .and_then(|a| a.get(name).map(str::to_owned))
}

/// Parse `<progress>.max` per the spec actual-max algorithm
/// (HTML §4.10.14): if missing / invalid → 1; if `<= 0` → 1.
fn parsed_max(ctx: &mut NativeContext<'_>, entity: Entity) -> f64 {
    let raw = read_attr_raw(ctx, entity, "max");
    let parsed = parse_double_or_default(raw.as_deref(), 1.0);
    if parsed <= 0.0 {
        1.0
    } else {
        parsed
    }
}

/// Parse `<progress>.value` per the spec actual-value algorithm
/// (HTML §4.10.14): if attribute missing / invalid → 0; otherwise
/// clamp to `0..=max`.
fn parsed_value(ctx: &mut NativeContext<'_>, entity: Entity) -> f64 {
    let raw = read_attr_raw(ctx, entity, "value");
    let parsed = parse_double_or_default(raw.as_deref(), 0.0);
    let max = parsed_max(ctx, entity);
    parsed.clamp(0.0, max)
}

fn progress_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_progress_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Number(0.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(0.0));
    }
    Ok(JsValue::Number(parsed_value(ctx, entity)))
}

fn progress_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_progress_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let n = coerce_double_idl_arg(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // ES Number ToString (ES2020 §7.1.12) — diverges from Rust's
    // Display in `-0` (ES → "0", Rust → "-0") and exponent edge cases.
    // Routing through `coerce::to_string` keeps reflected attribute
    // values matching browser JS semantics.
    let value_sid = super::super::coerce::to_string(ctx.vm, JsValue::Number(n))?;
    let attr_sid = ctx.vm.strings.intern("value");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn progress_get_max(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_progress_receiver(ctx, this, "max")? else {
        return Ok(JsValue::Number(1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(1.0));
    }
    Ok(JsValue::Number(parsed_max(ctx, entity)))
}

fn progress_set_max(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_progress_receiver(ctx, this, "max")? else {
        return Ok(JsValue::Undefined);
    };
    let n = coerce_double_idl_arg(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let value_sid = super::super::coerce::to_string(ctx.vm, JsValue::Number(n))?;
    let attr_sid = ctx.vm.strings.intern("max");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

/// `<progress>.position` (HTML §4.10.14): `-1` (indeterminate) if no
/// `value` content attribute is present, otherwise
/// `clamp(value, 0, max) / max`.
fn progress_get_position(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_progress_receiver(ctx, this, "position")? else {
        return Ok(JsValue::Number(-1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(-1.0));
    }
    let value_attr = read_attr_raw(ctx, entity, "value");
    if value_attr.is_none() {
        return Ok(JsValue::Number(-1.0));
    }
    let max = parsed_max(ctx, entity);
    let value = parse_double_or_default(value_attr.as_deref(), 0.0).clamp(0.0, max);
    // `parsed_max` already guarantees `max > 0`, so the division is safe.
    Ok(JsValue::Number(value / max))
}

fn progress_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_progress_receiver(ctx, this, "labels")?;
    Ok(JsValue::Object(
        super::dom_collection::empty_labels_collection(ctx.vm),
    ))
}
