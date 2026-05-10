//! `HTMLAreaElement.prototype` intrinsic — per-tag prototype layer
//! for `<area>` wrappers (HTML §4.6.2, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Same engine-bound discipline as `html_anchor_proto.rs`.  URL
//! accessor algorithm lives in
//! `elidex_dom_api::element::href_accessor`; enumerated-reflect
//! canonicalisation lives in
//! `elidex_dom_api::element::enumerated_reflect`.

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    AREA_SHAPE_INVALID_DEFAULT, AREA_SHAPE_MISSING_DEFAULT, AREA_SHAPE_VALUES,
};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    /// Allocate `HTMLAreaElement.prototype` chained to
    /// `HTMLElement.prototype`.  Must run after
    /// `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_area_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_area_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_area_prototype = Some(proto_id);

        super::html_hyperlink_mixin::install_hyperlink_url_accessors(self, proto_id);
        self.install_html_area_string_reflect(proto_id);
        self.install_html_area_enumerated_reflect(proto_id);
        self.install_html_area_rel_list(proto_id);
    }

    fn install_html_area_string_reflect(&mut self, proto_id: ObjectId) {
        let pairs: [(&'static str, &'static str); 5] = [
            ("alt", "alt"),
            ("coords", "coords"),
            ("target", "target"),
            ("download", "download"),
            ("ping", "ping"),
        ];
        for (idl_name, attr_name) in pairs {
            let getter = area_string_reflect_getter_for(attr_name);
            let setter = area_string_reflect_setter_for(attr_name);
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

    fn install_html_area_enumerated_reflect(&mut self, proto_id: ObjectId) {
        // `shape` enumerated (HTML §4.6.6 / §6.4 area shape set).
        let sid = self.strings.intern("shape");
        self.install_accessor_pair(
            proto_id,
            sid,
            area_get_shape,
            Some(area_set_shape),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `referrerPolicy` enumerated (HTML §6.6.5).
        let sid = self.strings.intern("referrerPolicy");
        self.install_accessor_pair(
            proto_id,
            sid,
            area_get_referrer_policy,
            Some(area_set_referrer_policy),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_area_rel_list(&mut self, proto_id: ObjectId) {
        let sid = self.strings.intern("relList");
        self.install_accessor_pair(
            proto_id,
            sid,
            area_get_rel_list,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check — <area> only
// ---------------------------------------------------------------------------

fn require_area_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, super::super::value::VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLAreaElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "area") {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLAreaElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String-reflect dispatch
// ---------------------------------------------------------------------------

fn area_string_reflect_getter_for(attr: &'static str) -> NativeFn {
    match attr {
        "alt" => area_get_alt,
        "coords" => area_get_coords,
        "target" => area_get_target,
        "download" => area_get_download,
        "ping" => area_get_ping,
        _ => unreachable!("area_string_reflect_getter_for: {attr}"),
    }
}

fn area_string_reflect_setter_for(attr: &'static str) -> NativeFn {
    match attr {
        "alt" => area_set_alt,
        "coords" => area_set_coords,
        "target" => area_set_target,
        "download" => area_set_download,
        "ping" => area_set_ping,
        _ => unreachable!("area_string_reflect_setter_for: {attr}"),
    }
}

macro_rules! reflect_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_area_receiver(ctx, this, $method)? else {
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
            let Some(entity) = require_area_receiver(ctx, this, $method)? else {
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

reflect_getter!(area_get_alt, "alt", "alt");
reflect_setter!(area_set_alt, "alt", "alt");
reflect_getter!(area_get_coords, "coords", "coords");
reflect_setter!(area_set_coords, "coords", "coords");
reflect_getter!(area_get_target, "target", "target");
reflect_setter!(area_set_target, "target", "target");
reflect_getter!(area_get_download, "download", "download");
reflect_setter!(area_set_download, "download", "download");
reflect_getter!(area_get_ping, "ping", "ping");
reflect_setter!(area_set_ping, "ping", "ping");

// ---------------------------------------------------------------------------
// Enumerated reflect — shape / referrerPolicy
// ---------------------------------------------------------------------------

fn area_get_shape(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_area_receiver(ctx, this, "shape")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect(
        ctx,
        entity,
        "shape",
        AREA_SHAPE_VALUES,
        AREA_SHAPE_MISSING_DEFAULT,
        AREA_SHAPE_INVALID_DEFAULT,
    )
}

fn area_set_shape(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_area_receiver(ctx, this, "shape")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("shape");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn area_get_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_area_receiver(ctx, this, "referrerPolicy")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect_referrer_policy(ctx, entity)
}

fn area_set_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_area_receiver(ctx, this, "referrerPolicy")? else {
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
// relList
// ---------------------------------------------------------------------------

fn area_get_rel_list(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_area_receiver(ctx, this, "relList")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_rel_list(entity);
    Ok(JsValue::Object(id))
}
