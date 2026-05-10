//! `HTMLAnchorElement.prototype` intrinsic — per-tag prototype layer
//! for `<a>` wrappers (HTML §4.6.1, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! reflected-attribute getter/setter shaping, and JsValue↔Entity
//! marshalling.  The HTMLHyperlinkElementUtils mixin algorithm
//! (URL parse + base resolve + serialise) lives in
//! `elidex_dom_api::element::href_accessor` (engine-indep), and the
//! enumerated reflect canonicalisation lives in
//! `elidex_dom_api::element::enumerated_reflect`.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    /// Allocate `HTMLAnchorElement.prototype` chained to
    /// `HTMLElement.prototype`.  Must run after
    /// `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_anchor_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_anchor_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_anchor_prototype = Some(proto_id);

        super::html_hyperlink_mixin::install_hyperlink_url_accessors(self, proto_id);
        self.install_html_anchor_string_reflect(proto_id);
        self.install_html_anchor_enumerated_reflect(proto_id);
        self.install_html_anchor_text_accessor(proto_id);
        self.install_html_anchor_rel_list(proto_id);
    }

    fn install_html_anchor_string_reflect(&mut self, proto_id: ObjectId) {
        // (IDL property name, HTML attribute lowercase name).  HTML
        // §4.6.5 DOMString reflect attributes for `<a>`.
        let pairs: [(&'static str, &'static str); 5] = [
            ("target", "target"),
            ("download", "download"),
            ("ping", "ping"),
            ("hreflang", "hreflang"),
            ("type", "type"),
        ];
        for (idl_name, attr_name) in pairs {
            let getter = anchor_string_reflect_getter_for(attr_name);
            let setter = anchor_string_reflect_setter_for(attr_name);
            let sid = self.strings.intern(idl_name);
            self.install_accessor_pair(
                proto_id,
                sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_html_anchor_enumerated_reflect(&mut self, proto_id: ObjectId) {
        // `referrerPolicy` enumerated reflect (HTML §6.6.5).
        let sid = self.strings.intern("referrerPolicy");
        self.install_accessor_pair(
            proto_id,
            sid,
            anchor_get_referrer_policy,
            Some(anchor_set_referrer_policy),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_anchor_text_accessor(&mut self, proto_id: ObjectId) {
        // `<a>.text` — textContent alias (HTML §4.6.5).
        let sid = self.strings.intern("text");
        self.install_accessor_pair(
            proto_id,
            sid,
            anchor_get_text,
            Some(anchor_set_text),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_anchor_rel_list(&mut self, proto_id: ObjectId) {
        // `relList` — DOMTokenList for `rel` attribute.
        let sid = self.strings.intern("relList");
        self.install_accessor_pair(
            proto_id,
            sid,
            anchor_get_rel_list,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Brand check for `<a>` receivers.  Used by all anchor-only
/// accessors (text / referrerPolicy / relList / per-spec attrs).
/// The shared HTMLHyperlinkElementUtils URL accessors accept either
/// `<a>` or `<area>`; that broader brand check lives in
/// [`super::html_hyperlink_mixin::require_hyperlink_receiver`].
pub(super) fn require_anchor_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, super::super::value::VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLAnchorElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "a") {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLAnchorElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String-reflect dispatch — one static fn per attr (NativeFn signature
// requires fn pointer, not closure).
// ---------------------------------------------------------------------------

fn anchor_string_reflect_getter_for(attr: &'static str) -> NativeFn {
    match attr {
        "target" => anchor_get_target,
        "download" => anchor_get_download,
        "ping" => anchor_get_ping,
        "hreflang" => anchor_get_hreflang,
        "type" => anchor_get_type_,
        _ => unreachable!("anchor_string_reflect_getter_for: {attr}"),
    }
}

fn anchor_string_reflect_setter_for(attr: &'static str) -> NativeFn {
    match attr {
        "target" => anchor_set_target,
        "download" => anchor_set_download,
        "ping" => anchor_set_ping,
        "hreflang" => anchor_set_hreflang,
        "type" => anchor_set_type_,
        _ => unreachable!("anchor_string_reflect_setter_for: {attr}"),
    }
}

macro_rules! reflect_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_anchor_receiver(ctx, this, $method)? else {
                return Ok(JsValue::String(ctx.vm.well_known.empty));
            };
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::String(ctx.vm.well_known.empty));
            }
            let attr_sid = ctx.vm.strings.intern($attr);
            invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| {
                match v {
                    JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
                    other => other,
                }
            })
        }
    };
}

macro_rules! reflect_setter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_anchor_receiver(ctx, this, $method)? else {
                return Ok(JsValue::Undefined);
            };
            let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::Undefined);
            }
            let attr_sid = ctx.vm.strings.intern($attr);
            invoke_dom_api(
                ctx,
                "setAttribute",
                entity,
                &[JsValue::String(attr_sid), JsValue::String(value_sid)],
            )
        }
    };
}

reflect_getter!(anchor_get_target, "target", "target");
reflect_setter!(anchor_set_target, "target", "target");
reflect_getter!(anchor_get_download, "download", "download");
reflect_setter!(anchor_set_download, "download", "download");
reflect_getter!(anchor_get_ping, "ping", "ping");
reflect_setter!(anchor_set_ping, "ping", "ping");
reflect_getter!(anchor_get_hreflang, "hreflang", "hreflang");
reflect_setter!(anchor_set_hreflang, "hreflang", "hreflang");
reflect_getter!(anchor_get_type_, "type", "type");
reflect_setter!(anchor_set_type_, "type", "type");

// ---------------------------------------------------------------------------
// Enumerated reflect — referrerPolicy
// ---------------------------------------------------------------------------

fn anchor_get_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_anchor_receiver(ctx, this, "referrerPolicy")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect_referrer_policy(ctx, entity)
}

fn anchor_set_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_anchor_receiver(ctx, this, "referrerPolicy")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("referrerpolicy");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

// ---------------------------------------------------------------------------
// `<a>.text` — textContent alias
// ---------------------------------------------------------------------------

fn anchor_get_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_anchor_receiver(ctx, this, "text")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn anchor_set_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_anchor_receiver(ctx, this, "text")? else {
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

// ---------------------------------------------------------------------------
// relList — DOMTokenList wrapper backed by `rel` attr
// ---------------------------------------------------------------------------

fn anchor_get_rel_list(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_anchor_receiver(ctx, this, "relList")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_rel_list(entity);
    Ok(JsValue::Object(id))
}
