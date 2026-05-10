//! `HTMLQuoteElement.prototype` intrinsic — per-tag prototype shared
//! across `<blockquote>` (HTML §4.4.4) and `<q>` (HTML §4.5.7), per
//! WebIDL `HTMLQuoteElement` interface (slot `#11-tags-T2b-passive`).
//!
//! Both elements expose a single `cite` IDL attribute (DOMString
//! reflect of the `cite` content attribute).  Per WebIDL §"Reflect"
//! the value is plain DOMString — no URL parse / canonicalisation at
//! the IDL surface despite the attribute's value being expected to
//! be a URL by content-conformance rules.
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
    pub(in crate::vm) fn register_html_quote_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_quote_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_quote_prototype = Some(proto_id);

        let cite_sid = self.strings.intern("cite");
        self.install_accessor_pair(
            proto_id,
            cite_sid,
            quote_get_cite,
            Some(quote_set_cite),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_quote_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLQuoteElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "blockquote")
        && !ctx.host().tag_matches_ascii_case(entity, "q")
    {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLQuoteElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn quote_get_cite(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_quote_receiver(ctx, this, "cite")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("cite");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn quote_set_cite(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_quote_receiver(ctx, this, "cite")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("cite");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
