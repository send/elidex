//! `HTMLTitleElement.prototype` intrinsic — per-tag prototype layer
//! for `<title>` wrappers (HTML §4.2.2, slot `#11-tags-T2b-passive`).
//!
//! Surfaces the `text` IDL attribute, which per HTML §4.2.2 is an
//! alias for `Node.textContent`.  Pattern matches T2a's
//! `<a>.text` / `<script>.text` accessor: thin marshalling shim that
//! routes through `textContent.get` / `textContent.set` dom-api
//! handlers (the underlying replace-all-children algorithm lives
//! engine-indep in `elidex_dom_api::node::text_content`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file holds prototype install
//! + brand check + marshalling only.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_title_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_title_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_title_prototype = Some(proto_id);

        let sid = self.strings.intern("text");
        self.install_accessor_pair(
            proto_id,
            sid,
            title_get_text,
            Some(title_set_text),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_title_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTitleElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "title") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTitleElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn title_get_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_title_receiver(ctx, this, "text")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn title_set_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_title_receiver(ctx, this, "text")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    invoke_dom_api(
        ctx,
        "textContent.set",
        entity,
        &[JsValue::String(value_sid)],
    )
}
