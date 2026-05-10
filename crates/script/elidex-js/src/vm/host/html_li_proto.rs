//! `HTMLLIElement.prototype` intrinsic — per-tag prototype layer for
//! `<li>` wrappers (HTML §4.4.8, slot `#11-tags-T2b-passive`).
//!
//! Single IDL attribute:
//! - `value` — long IDL with default 0 per HTML §4.4.8 (only
//!   meaningful when the `<li>` is inside an `<ol>`, but the reflect
//!   is unconditional).  Routes through the engine-indep
//!   [`elidex_dom_api::element::numeric_reflect::parse_long_or_default`]
//!   helper for both spec-faithful integer parsing and i32 saturation.
//!
//! Deprecated `type` attribute is intentionally not surfaced (defer
//! slot `#11-tags-deprecated-attr-sweep`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.

#![cfg(feature = "engine")]

use elidex_dom_api::element::numeric_reflect::parse_long_or_default;
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::serialise_long_idl_arg;

/// `<li>.value` IDL missing-value default — plain `long` reflect with
/// no per-attribute spec override (vs `<ol>.start` whose default is 1).
const LI_VALUE_DEFAULT: i32 = 0;

impl VmInner {
    pub(in crate::vm) fn register_html_li_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_li_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_li_prototype = Some(proto_id);

        let value_sid = self.strings.intern("value");
        self.install_accessor_pair(
            proto_id,
            value_sid,
            li_get_value,
            Some(li_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_li_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLLIElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "li") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLLIElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn li_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_li_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Number(f64::from(LI_VALUE_DEFAULT)));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(f64::from(LI_VALUE_DEFAULT)));
    }
    let attr_sid = ctx.vm.strings.intern("value");
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw_str = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let parsed = parse_long_or_default(raw_str.as_deref(), LI_VALUE_DEFAULT);
    Ok(JsValue::Number(f64::from(parsed)))
}

fn li_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_li_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let serialised = serialise_long_idl_arg(ctx, args)?;
    let attr_sid = ctx.vm.strings.intern("value");
    let value_sid = ctx.vm.strings.intern(&serialised);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
