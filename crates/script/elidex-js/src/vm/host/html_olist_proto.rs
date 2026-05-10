//! `HTMLOListElement.prototype` intrinsic — per-tag prototype layer
//! for `<ol>` wrappers (HTML §4.4.5, slot `#11-tags-T2b-passive`).
//!
//! Three IDL attributes:
//! - `reversed` — boolean reflect (presence of the `reversed` content
//!   attribute).
//! - `start` — long IDL with default 1; getter parses per HTML
//!   §2.4.4.1 ("rules for parsing integers") via the engine-indep
//!   [`elidex_dom_api::element::numeric_reflect::parse_long_or_default`]
//!   helper.
//! - `type` — DOMString "limited to only known values" per HTML
//!   §4.4.5: keywords `1` / `a` / `A` / `i` / `I` are case-sensitive
//!   (`a` and `A` are distinct list-marker styles).  Routes through
//!   [`elidex_dom_api::element::enumerated_reflect::canonicalize_limited_to_known_values`]
//!   which is distinct from the ASCII-CI [`canonicalize_enumerated_attr`]
//!   used by the T2a `referrerPolicy` family (those keywords differ
//!   only by spelling).
//!
//! The deprecated `compact` attribute is intentionally not surfaced
//! (defer slot `#11-tags-deprecated-attr-sweep`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", all three reflect algorithms
//! (boolean / signed-long parse / case-sensitive limited-to-known)
//! live in `elidex-dom-api`; this file is marshalling-only.

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    canonicalize_limited_to_known_values, OL_TYPE_VALUES,
};
use elidex_dom_api::element::numeric_reflect::parse_long_or_default;
use elidex_ecs::{Entity, NodeKind};

use super::super::coerce::to_boolean;
use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};
use super::idl_coerce::serialise_long_idl_arg;

/// `<ol>.start` IDL missing-value default per HTML §4.4.5: "Its
/// default value is 1".  Non-default `<ol reversed>` semantics
/// (default = list length) live in cascade-time evaluation, not in
/// the IDL surface.
const OL_START_DEFAULT: i32 = 1;

impl VmInner {
    pub(in crate::vm) fn register_html_olist_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_olist_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_olist_prototype = Some(proto_id);

        let reversed_sid = self.strings.intern("reversed");
        self.install_accessor_pair(
            proto_id,
            reversed_sid,
            ol_get_reversed,
            Some(ol_set_reversed),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let start_sid = self.strings.intern("start");
        self.install_accessor_pair(
            proto_id,
            start_sid,
            ol_get_start,
            Some(ol_set_start),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let type_sid = self.strings.intern("type");
        self.install_accessor_pair(
            proto_id,
            type_sid,
            ol_get_type,
            Some(ol_set_type),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_olist_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLOListElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "ol") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLOListElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// -- reversed (boolean reflect = attribute presence) ----------------------

fn ol_get_reversed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_olist_receiver(ctx, this, "reversed")? else {
        return Ok(JsValue::Boolean(false));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    let attr_sid = ctx.vm.strings.intern("reversed");
    invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])
}

fn ol_set_reversed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_olist_receiver(ctx, this, "reversed")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // HTML §6.5.5 boolean reflect setter: ECMAScript ToBoolean →
    // truthy sets attribute to "" (HTML legacy convention), falsy
    // removes.  Routes through the shared `coerce::to_boolean` so
    // `0n` → false (and any other ToBoolean spec edge stays in one
    // place), rather than the previous local hand-rolled match
    // which wrongly treated all BigInts as truthy.
    let raw = args.first().copied().unwrap_or(JsValue::Undefined);
    let truthy = to_boolean(ctx.vm, raw);
    let attr_sid = ctx.vm.strings.intern("reversed");
    if truthy {
        let empty_value = JsValue::String(ctx.vm.well_known.empty);
        invoke_dom_api(
            ctx,
            "setAttribute",
            entity,
            &[JsValue::String(attr_sid), empty_value],
        )
    } else {
        invoke_dom_api(ctx, "removeAttribute", entity, &[JsValue::String(attr_sid)])
    }
}

// -- start (long IDL, default 1) ------------------------------------------

fn ol_get_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_olist_receiver(ctx, this, "start")? else {
        return Ok(JsValue::Number(f64::from(OL_START_DEFAULT)));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(f64::from(OL_START_DEFAULT)));
    }
    let attr_sid = ctx.vm.strings.intern("start");
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw_str = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let parsed = parse_long_or_default(raw_str.as_deref(), OL_START_DEFAULT);
    Ok(JsValue::Number(f64::from(parsed)))
}

fn ol_set_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_olist_receiver(ctx, this, "start")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let serialised = serialise_long_idl_arg(ctx, args)?;
    let attr_sid = ctx.vm.strings.intern("start");
    let value_sid = ctx.vm.strings.intern(&serialised);
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

// -- type (DOMString limited-to-only-known-values, case-sensitive) --------

fn ol_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_olist_receiver(ctx, this, "type")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("type");
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let canonical = canonicalize_limited_to_known_values(raw.as_deref(), OL_TYPE_VALUES);
    let out_sid = ctx.vm.strings.intern(canonical);
    Ok(JsValue::String(out_sid))
}

fn ol_set_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_olist_receiver(ctx, this, "type")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("type");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}
