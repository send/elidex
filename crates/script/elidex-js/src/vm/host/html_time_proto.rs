//! `HTMLTimeElement.prototype` intrinsic — per-tag prototype layer
//! for `<time>` wrappers (HTML §4.5.14, slot `#11-tags-T2b-passive`).
//!
//! Single IDL attribute: `dateTime` — DOMString reflect of the
//! `datetime` (lowercase) content attribute.  Camel-case IDL → all-
//! lowercase content attribute mapping mirrors HTMLMeta `httpEquiv`
//! / `http-equiv` (this PR) and HTMLLink `imageSrcset` / `imagesrcset`
//! (T2a).
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
    pub(in crate::vm) fn register_html_time_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_time_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_time_prototype = Some(proto_id);

        let date_time_sid = self.strings.intern("dateTime");
        self.install_accessor_pair(
            proto_id,
            date_time_sid,
            time_get_date_time,
            Some(time_set_date_time),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_time_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTimeElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "time") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTimeElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn time_get_date_time(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_time_receiver(ctx, this, "dateTime")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("datetime");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn time_set_date_time(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_time_receiver(ctx, this, "dateTime")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("datetime");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
