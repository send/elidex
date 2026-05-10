//! `HTMLBaseElement.prototype` intrinsic — per-tag prototype layer
//! for `<base>` wrappers (HTML §4.2.3, slot `#11-tags-T2b-passive`).
//!
//! Surfaces:
//! - `href` — URL-resolved-fallback-to-raw via the engine-indep
//!   [`elidex_dom_api::element::href_accessor::href_value_or_raw`]
//!   helper.  Per HTML §4.2.3 step 3-4, the getter parses the `href`
//!   content attribute against the document's fallback base URL (the
//!   `about:blank` placeholder until `#11-base-href-resolution`
//!   lands real navigation state).
//! - `target` — plain string reflect.
//!
//! `<base href>` propagation into anchor / area / img / link / script
//! base resolution is a separate concern: T2a's per-element URL
//! accessors currently resolve against the same `about:blank`
//! placeholder, and integrating real `<base>` walking is deferred to
//! slot `#11-base-href-resolution` (re-noted from T2a defer ledger).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  The URL parse
//! and base-URL resolution algorithm lives engine-indep in
//! `elidex_dom_api::element::href_accessor`.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_base_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_base_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_base_prototype = Some(proto_id);

        let href_sid = self.strings.intern("href");
        self.install_accessor_pair(
            proto_id,
            href_sid,
            base_get_href,
            Some(base_set_href),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let target_sid = self.strings.intern("target");
        self.install_accessor_pair(
            proto_id,
            target_sid,
            base_get_target,
            Some(base_set_target),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_base_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLBaseElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "base") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLBaseElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// `<base>.href` — URL-resolved against the document's fallback base
// URL with raw fallback on parse failure (HTML §4.2.3 step 3-4).
// Routes through the same `hyperlink.href.get` dom-api handler as T2a
// `<a>.href` / `<area>.href` so the `about:blank` placeholder
// behaviour is shared and a future `#11-base-href-resolution` cutover
// updates all consumers atomically.
fn base_get_href(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_base_receiver(ctx, this, "href")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "hyperlink.href.get", entity, &[])
}

// `<base>.href` setter is plain string reflect (HTML §4.2.3 step 1
// — the IDL setter just writes the content attribute).  Distinct
// from per-component URL setters which round-trip through
// `url::Url::set_*`.
fn base_set_href(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_base_receiver(ctx, this, "href")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("href");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn base_get_target(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_base_receiver(ctx, this, "target")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("target");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn base_set_target(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_base_receiver(ctx, this, "target")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("target");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
