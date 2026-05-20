//! `HTMLLinkElement.prototype` intrinsic — per-tag prototype layer
//! for `<link>` wrappers (HTML §4.6.7, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Engine-bound only.  The `link.sheet` getter (CSSOM `LinkStyle.sheet`)
//! is a brand-check + wrapper-alloc that delegates the "has an associated
//! CSS style sheet?" predicate to engine-independent
//! [`elidex_dom_api::link_has_loaded_sheet`]; the stylesheet load
//! lifecycle itself lives in the resource loader (`elidex-navigation`) +
//! the `LinkStylesheet` ECS component.  Dynamic (post-load) fetch is
//! deferred to slot `#11-link-stylesheet-dynamic-fetch`.
//!
//! `<link>.sizes` is a `[SameObject, PutForwards=value]` DOMTokenList
//! (HTML §4.6.7 — e.g. `<link rel="icon" sizes="16x16 32x32">`).
//! Identity is preserved via `link_sizes_wrapper_cache` (CRIT-2 Option A).

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    CROSS_ORIGIN_INVALID_DEFAULT, CROSS_ORIGIN_VALUES, FETCH_PRIORITY_INVALID_DEFAULT,
    FETCH_PRIORITY_MISSING_DEFAULT, FETCH_PRIORITY_VALUES,
};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_link_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_link_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_link_prototype = Some(proto_id);

        self.install_html_link_string_reflect(proto_id);
        self.install_html_link_enumerated_reflect(proto_id);
        self.install_html_link_bool_reflect(proto_id);
        self.install_html_link_token_lists(proto_id);
        self.install_html_link_sheet(proto_id);
    }

    fn install_html_link_string_reflect(&mut self, proto_id: ObjectId) {
        let pairs: [(&'static str, &'static str); 8] = [
            ("href", "href"),
            ("media", "media"),
            ("hreflang", "hreflang"),
            ("type", "type"),
            ("integrity", "integrity"),
            ("imageSrcset", "imagesrcset"),
            ("imageSizes", "imagesizes"),
            ("as", "as"),
        ];
        for (idl_name, attr_name) in pairs {
            let getter = link_string_reflect_getter_for(attr_name);
            let setter = link_string_reflect_setter_for(attr_name);
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

    fn install_html_link_enumerated_reflect(&mut self, proto_id: ObjectId) {
        for (idl_name, getter, setter) in [
            (
                "crossOrigin",
                link_get_cross_origin as NativeFn,
                link_set_cross_origin as NativeFn,
            ),
            (
                "referrerPolicy",
                link_get_referrer_policy,
                link_set_referrer_policy,
            ),
            (
                "fetchpriority",
                link_get_fetch_priority,
                link_set_fetch_priority,
            ),
        ] {
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

    fn install_html_link_bool_reflect(&mut self, proto_id: ObjectId) {
        let sid = self.strings.intern("disabled");
        self.install_accessor_pair(
            proto_id,
            sid,
            link_get_disabled,
            Some(link_set_disabled),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_link_token_lists(&mut self, proto_id: ObjectId) {
        // `relList` — DOMTokenList for `rel`.
        let sid = self.strings.intern("relList");
        self.install_accessor_pair(
            proto_id,
            sid,
            link_get_rel_list,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `sizes` — `[SameObject, PutForwards=value]` DOMTokenList.
        let sid = self.strings.intern("sizes");
        self.install_accessor_pair(
            proto_id,
            sid,
            link_get_sizes,
            Some(link_set_sizes),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_link_sheet(&mut self, proto_id: ObjectId) {
        let sid = self.strings.intern("sheet");
        self.install_accessor_pair(
            proto_id,
            sid,
            link_get_sheet,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_link_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, super::super::value::VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLLinkElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "link") {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLLinkElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String-reflect dispatch
// ---------------------------------------------------------------------------

fn link_string_reflect_getter_for(attr: &'static str) -> NativeFn {
    match attr {
        "href" => link_get_href,
        "media" => link_get_media,
        "hreflang" => link_get_hreflang,
        "type" => link_get_type_,
        "integrity" => link_get_integrity,
        "imagesrcset" => link_get_image_srcset,
        "imagesizes" => link_get_image_sizes,
        "as" => link_get_as_,
        _ => unreachable!("link_string_reflect_getter_for: {attr}"),
    }
}

fn link_string_reflect_setter_for(attr: &'static str) -> NativeFn {
    match attr {
        "href" => link_set_href,
        "media" => link_set_media,
        "hreflang" => link_set_hreflang,
        "type" => link_set_type_,
        "integrity" => link_set_integrity,
        "imagesrcset" => link_set_image_srcset,
        "imagesizes" => link_set_image_sizes,
        "as" => link_set_as_,
        _ => unreachable!("link_string_reflect_setter_for: {attr}"),
    }
}

macro_rules! reflect_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_link_receiver(ctx, this, $method)? else {
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
            let Some(entity) = require_link_receiver(ctx, this, $method)? else {
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

reflect_getter!(link_get_href, "href", "href");
reflect_setter!(link_set_href, "href", "href");
reflect_getter!(link_get_media, "media", "media");
reflect_setter!(link_set_media, "media", "media");
reflect_getter!(link_get_hreflang, "hreflang", "hreflang");
reflect_setter!(link_set_hreflang, "hreflang", "hreflang");
reflect_getter!(link_get_type_, "type", "type");
reflect_setter!(link_set_type_, "type", "type");
reflect_getter!(link_get_integrity, "integrity", "integrity");
reflect_setter!(link_set_integrity, "integrity", "integrity");
reflect_getter!(link_get_image_srcset, "imagesrcset", "imageSrcset");
reflect_setter!(link_set_image_srcset, "imagesrcset", "imageSrcset");
reflect_getter!(link_get_image_sizes, "imagesizes", "imageSizes");
reflect_setter!(link_set_image_sizes, "imagesizes", "imageSizes");
reflect_getter!(link_get_as_, "as", "as");
reflect_setter!(link_set_as_, "as", "as");

// ---------------------------------------------------------------------------
// Enumerated reflect
// ---------------------------------------------------------------------------

fn link_get_cross_origin(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "crossOrigin")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    super::html_hyperlink_mixin::get_enumerated_reflect_nullable(
        ctx,
        entity,
        "crossorigin",
        CROSS_ORIGIN_VALUES,
        CROSS_ORIGIN_INVALID_DEFAULT,
    )
}
reflect_setter!(link_set_cross_origin, "crossorigin", "crossOrigin");

fn link_get_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "referrerPolicy")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect_referrer_policy(ctx, entity)
}
reflect_setter!(link_set_referrer_policy, "referrerpolicy", "referrerPolicy");

fn link_get_fetch_priority(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "fetchpriority")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect(
        ctx,
        entity,
        "fetchpriority",
        FETCH_PRIORITY_VALUES,
        FETCH_PRIORITY_MISSING_DEFAULT,
        FETCH_PRIORITY_INVALID_DEFAULT,
    )
}
reflect_setter!(link_set_fetch_priority, "fetchpriority", "fetchpriority");

// ---------------------------------------------------------------------------
// Boolean reflect — disabled
// ---------------------------------------------------------------------------

fn link_get_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Boolean(false));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    let attr_sid = ctx.vm.strings.intern("disabled");
    invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])
}

fn link_set_disabled(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "disabled")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let truthy = super::super::coerce::to_boolean(ctx.vm, val);
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("disabled");
    if truthy {
        let empty_sid = ctx.vm.well_known.empty;
        invoke_dom_api(
            ctx,
            "setAttribute",
            entity,
            &[JsValue::String(attr_sid), JsValue::String(empty_sid)],
        )
    } else {
        invoke_dom_api(ctx, "removeAttribute", entity, &[JsValue::String(attr_sid)])
    }
}

// ---------------------------------------------------------------------------
// DOMTokenList accessors — relList / sizes
// ---------------------------------------------------------------------------

fn link_get_rel_list(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "relList")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_link_rel_list(entity);
    Ok(JsValue::Object(id))
}

fn link_get_sizes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "sizes")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_link_sizes(entity);
    Ok(JsValue::Object(id))
}

/// `<link>.sizes` setter — `[PutForwards=value]` semantics: assigning
/// to `link.sizes` is equivalent to `link.sizes.value = ToString(v)`,
/// which writes the `sizes` content attribute directly.
fn link_set_sizes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "sizes")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("sizes");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

// ---------------------------------------------------------------------------
// sheet getter — CSSOM `LinkStyle.sheet`
// ---------------------------------------------------------------------------

/// `HTMLLinkElement.prototype.sheet` (CSSOM `LinkStyle.sheet`,
/// `[SameObject]`). Returns the `[SameObject]` `CSSStyleSheet` wrapper
/// for the `<link>`'s associated CSS style sheet (present only after a
/// successful load, HTML §4.6.7), or `null` when there is none (no
/// `LinkStylesheet` component — non-stylesheet `rel`, missing href, or
/// failed fetch). The "has a sheet?" predicate is delegated to the
/// engine-independent [`elidex_dom_api::link_has_loaded_sheet`]; this
/// body does only brand-check + wrapper marshalling.
fn link_get_sheet(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_link_receiver(ctx, this, "sheet")? else {
        return Ok(JsValue::Null);
    };
    let Some(hd) = ctx.host_if_bound() else {
        return Ok(JsValue::Null);
    };
    if !elidex_dom_api::link_has_loaded_sheet(entity, hd.dom()) {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_stylesheet(entity);
    Ok(JsValue::Object(id))
}
