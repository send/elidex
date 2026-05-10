//! HTMLHyperlinkElementUtils mixin install + native fns
//! (HTML §4.6.5, slot `#11-tags-T2a-url-bearing`).
//!
//! Shared between `<a>` (`html_anchor_proto`) and `<area>`
//! (`html_area_proto`) — both elements expose the same 11 URL accessor
//! IDL attributes plus `toString()`, all backed by their `href`
//! content attribute.
//!
//! ## Layering
//!
//! Per CLAUDE.md, every native body is a thin marshalling shim:
//! brand check → coerce args → `invoke_dom_api(ctx, "hyperlink.<...>", ...)`.
//! All URL parsing, base-URL resolution, and component formatting
//! happens in `elidex_dom_api::element::href_accessor` (engine-indep).
//!
//! Enumerated-reflect canonicalisation (`referrerPolicy` etc.) is
//! likewise a marshalling-side dispatch over the engine-indep
//! [`elidex_dom_api::element::enumerated_reflect`] table.

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    canonicalize_enumerated_attr, REFERRER_POLICY_INVALID_DEFAULT, REFERRER_POLICY_MISSING_DEFAULT,
    REFERRER_POLICY_VALUES,
};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

/// Install the 11 IDL accessor pairs + `toString()` method on a
/// hyperlink-bearing prototype (anchor / area).
pub(super) fn install_hyperlink_url_accessors(vm: &mut VmInner, proto_id: ObjectId) {
    // Read-write accessors.  `origin` is read-only (no setter).
    let rw_pairs: [(&'static str, NativeFn, Option<NativeFn>); 11] = [
        ("href", hyperlink_get_href, Some(hyperlink_set_href)),
        ("origin", hyperlink_get_origin, None),
        (
            "protocol",
            hyperlink_get_protocol,
            Some(hyperlink_set_protocol),
        ),
        (
            "username",
            hyperlink_get_username,
            Some(hyperlink_set_username),
        ),
        (
            "password",
            hyperlink_get_password,
            Some(hyperlink_set_password),
        ),
        ("host", hyperlink_get_host, Some(hyperlink_set_host)),
        (
            "hostname",
            hyperlink_get_hostname,
            Some(hyperlink_set_hostname),
        ),
        ("port", hyperlink_get_port, Some(hyperlink_set_port)),
        (
            "pathname",
            hyperlink_get_pathname,
            Some(hyperlink_set_pathname),
        ),
        ("search", hyperlink_get_search, Some(hyperlink_set_search)),
        ("hash", hyperlink_get_hash, Some(hyperlink_set_hash)),
    ];
    for (name, getter, setter) in rw_pairs {
        let sid = vm.strings.intern(name);
        vm.install_accessor_pair(
            proto_id,
            sid,
            getter,
            setter,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
    // `toString()` — alias for `href` getter per HTML §4.6.5.
    let sid = vm.strings.intern("toString");
    vm.install_native_method(
        proto_id,
        sid,
        hyperlink_to_string,
        shape::PropertyAttrs::METHOD,
    );
}

// ---------------------------------------------------------------------------
// Brand check — accepts <a> OR <area>
// ---------------------------------------------------------------------------

/// Brand check for HTMLHyperlinkElementUtils receivers.  Accepts
/// `<a>` or `<area>` Element entities; rejects everything else with
/// "Illegal invocation".
pub(super) fn require_hyperlink_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = super::event_target::require_receiver(
        ctx,
        this,
        "HTMLHyperlinkElementUtils",
        method,
        |k| k == NodeKind::Element,
    )?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "a")
        && !ctx.host().tag_matches_ascii_case(entity, "area")
    {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLHyperlinkElementUtils': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// Native fn macro — getter / setter / no-arg dispatchers
// ---------------------------------------------------------------------------

macro_rules! url_getter {
    ($name:ident, $method_name:literal, $idl:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_hyperlink_receiver(ctx, this, $idl)? else {
                return Ok(JsValue::String(ctx.vm.well_known.empty));
            };
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::String(ctx.vm.well_known.empty));
            }
            invoke_dom_api(ctx, $method_name, entity, &[])
        }
    };
}

macro_rules! url_setter {
    ($name:ident, $method_name:literal, $idl:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_hyperlink_receiver(ctx, this, $idl)? else {
                return Ok(JsValue::Undefined);
            };
            let value = coerce_first_arg_to_string_id(ctx, args)?;
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::Undefined);
            }
            invoke_dom_api(ctx, $method_name, entity, &[JsValue::String(value)])
        }
    };
}

url_getter!(hyperlink_get_href, "hyperlink.href.get", "href");
url_setter!(hyperlink_set_href, "hyperlink.href.set", "href");
url_getter!(hyperlink_get_origin, "hyperlink.origin.get", "origin");
url_getter!(hyperlink_get_protocol, "hyperlink.protocol.get", "protocol");
url_setter!(hyperlink_set_protocol, "hyperlink.protocol.set", "protocol");
url_getter!(hyperlink_get_username, "hyperlink.username.get", "username");
url_setter!(hyperlink_set_username, "hyperlink.username.set", "username");
url_getter!(hyperlink_get_password, "hyperlink.password.get", "password");
url_setter!(hyperlink_set_password, "hyperlink.password.set", "password");
url_getter!(hyperlink_get_host, "hyperlink.host.get", "host");
url_setter!(hyperlink_set_host, "hyperlink.host.set", "host");
url_getter!(hyperlink_get_hostname, "hyperlink.hostname.get", "hostname");
url_setter!(hyperlink_set_hostname, "hyperlink.hostname.set", "hostname");
url_getter!(hyperlink_get_port, "hyperlink.port.get", "port");
url_setter!(hyperlink_set_port, "hyperlink.port.set", "port");
url_getter!(hyperlink_get_pathname, "hyperlink.pathname.get", "pathname");
url_setter!(hyperlink_set_pathname, "hyperlink.pathname.set", "pathname");
url_getter!(hyperlink_get_search, "hyperlink.search.get", "search");
url_setter!(hyperlink_set_search, "hyperlink.search.set", "search");
url_getter!(hyperlink_get_hash, "hyperlink.hash.get", "hash");
url_setter!(hyperlink_set_hash, "hyperlink.hash.set", "hash");

fn hyperlink_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_hyperlink_receiver(ctx, this, "toString")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "hyperlink.toString", entity, &[])
}

// ---------------------------------------------------------------------------
// Enumerated-reflect canonicalisation — referrerPolicy
// ---------------------------------------------------------------------------

/// Read a content attribute, canonicalise it per HTML's
/// "reflect" enumerated-attribute step (§2.3.5) using the supplied
/// keyword table + missing/invalid defaults, and return the canonical
/// IDL keyword as a `JsValue::String`.  Shared by every T2a element
/// that exposes an enumerated reflect IDL property
/// (anchor.referrerPolicy / area.shape / area.referrerPolicy /
/// img.{crossOrigin,referrerPolicy,decoding,loading,fetchpriority} /
/// script.{crossOrigin,referrerPolicy,fetchpriority} /
/// link.{crossOrigin,referrerPolicy,fetchpriority}).
pub(super) fn get_enumerated_reflect(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    attr_name: &str,
    table: &[&'static str],
    missing_default: &'static str,
    invalid_default: &'static str,
) -> Result<JsValue, VmError> {
    let attr_sid = ctx.vm.strings.intern(attr_name);
    let raw_value = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
    let raw = match raw_value {
        JsValue::String(sid) => Some(ctx.vm.strings.get_utf8(sid)),
        _ => None,
    };
    let canonical =
        canonicalize_enumerated_attr(raw.as_deref(), table, missing_default, invalid_default);
    let out_sid = ctx.vm.strings.intern(canonical);
    Ok(JsValue::String(out_sid))
}

/// Convenience: `get_enumerated_reflect` specialised for the
/// `referrerpolicy` content attribute.  The most common enumerated
/// reflect across T2a elements (used by 5 of the 5).
pub(super) fn get_enumerated_reflect_referrer_policy(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
) -> Result<JsValue, VmError> {
    get_enumerated_reflect(
        ctx,
        entity,
        "referrerpolicy",
        REFERRER_POLICY_VALUES,
        REFERRER_POLICY_MISSING_DEFAULT,
        REFERRER_POLICY_INVALID_DEFAULT,
    )
}
