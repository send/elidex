//! `HTMLDataElement.prototype` intrinsic — per-tag prototype layer
//! for `<data>` wrappers (HTML §4.5.13, slot `#11-tags-T2b-passive`).
//!
//! Single IDL attribute: `value` — DOMString reflect of the `value`
//! content attribute.  No coercion / canonicalisation at the IDL
//! surface (machine-readable interpretation is consumer-driven).
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
    pub(in crate::vm) fn register_html_data_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_data_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_data_prototype = Some(proto_id);

        let value_sid = self.strings.intern("value");
        self.install_accessor_pair(
            proto_id,
            value_sid,
            data_get_value,
            Some(data_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_data_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLDataElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "data") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLDataElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn data_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_data_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("value");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn data_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_data_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("value");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
