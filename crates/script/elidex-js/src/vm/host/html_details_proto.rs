//! `HTMLDetailsElement.prototype` intrinsic — per-tag prototype layer
//! for `<details>` wrappers (HTML §4.11.1, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.11.1):
//! - `open` — boolean reflect of the `open` content attribute.
//! - `name` — DOMString reflect of the `name` content attribute
//!   (current spec, accordion-style multi-disclosure groups).  Not
//!   deprecated, so in scope per the core/compat/deprecated tiering
//!   (`docs/design/ja/14-script-engines-webapi.md` §14.1.1 + §14.4.2).
//!
//! The spec-mandated `ToggleEvent` fire on open-state change is
//! deferred to slot `#11-tags-T2d-details-toggle-event` (paired with
//! D-10 `#11-events-misc` which lands the `ToggleEvent` class).  The
//! multi-disclosure exclusion semantics (auto-close sibling
//! `<details>` with the same `name`) are paired with that slot since
//! the exclusion fires on toggle event ordering.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_details_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_details_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_details_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        let open_sid = self.strings.intern("open");
        self.install_accessor_pair(
            proto_id,
            open_sid,
            details_get_open,
            Some(details_set_open),
            attrs,
        );
        let name_sid = self.strings.intern("name");
        self.install_accessor_pair(
            proto_id,
            name_sid,
            details_get_name,
            Some(details_set_name),
            attrs,
        );
    }
}

fn require_details_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLDetailsElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "details") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLDetailsElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn details_get_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "open")? else {
        return Ok(JsValue::Boolean(false));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    let attr_sid = ctx.vm.strings.intern("open");
    invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])
}

fn details_set_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "open")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let truthy = super::super::coerce::to_boolean(ctx.vm, val);
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("open");
    if truthy {
        let empty_sid = ctx.vm.well_known.empty;
        invoke_dom_api(
            ctx,
            "setAttribute",
            entity,
            &[JsValue::String(attr_sid), JsValue::String(empty_sid)],
        )
    } else {
        invoke_dom_api(ctx, "removeAttribute", entity, &[JsValue::String(attr_sid)])
    }
}

fn details_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "name")? else {
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

fn details_set_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_details_receiver(ctx, this, "name")? else {
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
