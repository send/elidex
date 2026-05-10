//! `HTMLImageElement.prototype` intrinsic — per-tag prototype layer
//! for `<img>` wrappers (HTML §4.8.4, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Engine-bound only.  Numeric reflect (HTML §"non-negative integer"
//! parse rule) lives in `elidex_dom_api::element::numeric_reflect`;
//! enumerated reflect canonicalisation lives in
//! `elidex_dom_api::element::enumerated_reflect`.

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    CROSS_ORIGIN_INVALID_DEFAULT, CROSS_ORIGIN_MISSING_DEFAULT, CROSS_ORIGIN_VALUES,
    DECODING_INVALID_DEFAULT, DECODING_MISSING_DEFAULT, DECODING_VALUES,
    FETCH_PRIORITY_INVALID_DEFAULT, FETCH_PRIORITY_MISSING_DEFAULT, FETCH_PRIORITY_VALUES,
    LOADING_INVALID_DEFAULT, LOADING_MISSING_DEFAULT, LOADING_VALUES,
};
use elidex_dom_api::element::numeric_reflect::parse_unsigned_long;
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_image_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_image_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_image_prototype = Some(proto_id);

        self.install_html_image_string_reflect(proto_id);
        self.install_html_image_enumerated_reflect(proto_id);
        self.install_html_image_bool_reflect(proto_id);
        self.install_html_image_numeric_reflect(proto_id);
        self.install_html_image_stubs(proto_id);
    }

    fn install_html_image_string_reflect(&mut self, proto_id: ObjectId) {
        let pairs: [(&'static str, &'static str); 5] = [
            ("alt", "alt"),
            ("src", "src"),
            ("srcset", "srcset"),
            ("sizes", "sizes"),
            ("useMap", "usemap"),
        ];
        for (idl_name, attr_name) in pairs {
            let getter = image_string_reflect_getter_for(attr_name);
            let setter = image_string_reflect_setter_for(attr_name);
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

    fn install_html_image_enumerated_reflect(&mut self, proto_id: ObjectId) {
        let entries: [(&'static str, NativeFn, NativeFn); 5] = [
            (
                "crossOrigin",
                image_get_cross_origin,
                image_set_cross_origin,
            ),
            (
                "referrerPolicy",
                image_get_referrer_policy,
                image_set_referrer_policy,
            ),
            ("decoding", image_get_decoding, image_set_decoding),
            ("loading", image_get_loading, image_set_loading),
            (
                "fetchpriority",
                image_get_fetch_priority,
                image_set_fetch_priority,
            ),
        ];
        for (idl_name, getter, setter) in entries {
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

    fn install_html_image_bool_reflect(&mut self, proto_id: ObjectId) {
        let sid = self.strings.intern("isMap");
        self.install_accessor_pair(
            proto_id,
            sid,
            image_get_is_map,
            Some(image_set_is_map),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_image_numeric_reflect(&mut self, proto_id: ObjectId) {
        let sid = self.strings.intern("width");
        self.install_accessor_pair(
            proto_id,
            sid,
            image_get_width,
            Some(image_set_width),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let sid = self.strings.intern("height");
        self.install_accessor_pair(
            proto_id,
            sid,
            image_get_height,
            Some(image_set_height),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_image_stubs(&mut self, proto_id: ObjectId) {
        // Parity null/zero stubs — paint pipeline + decode Promise are deferred.
        for (idl_name, getter) in [
            ("naturalWidth", image_get_natural_width as NativeFn),
            ("naturalHeight", image_get_natural_height),
            ("complete", image_get_complete),
        ] {
            let sid = self.strings.intern(idl_name);
            self.install_accessor_pair(
                proto_id,
                sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        let sid = self.strings.intern("decode");
        self.install_native_method(
            proto_id,
            sid,
            image_decode_method,
            shape::PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_image_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, super::super::value::VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLImageElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "img") {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLImageElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String-reflect dispatch
// ---------------------------------------------------------------------------

fn image_string_reflect_getter_for(attr: &'static str) -> NativeFn {
    match attr {
        "alt" => image_get_alt,
        "src" => image_get_src,
        "srcset" => image_get_srcset,
        "sizes" => image_get_sizes,
        "usemap" => image_get_use_map,
        _ => unreachable!("image_string_reflect_getter_for: {attr}"),
    }
}

fn image_string_reflect_setter_for(attr: &'static str) -> NativeFn {
    match attr {
        "alt" => image_set_alt,
        "src" => image_set_src,
        "srcset" => image_set_srcset,
        "sizes" => image_set_sizes,
        "usemap" => image_set_use_map,
        _ => unreachable!("image_string_reflect_setter_for: {attr}"),
    }
}

macro_rules! reflect_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_image_receiver(ctx, this, $method)? else {
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
            let Some(entity) = require_image_receiver(ctx, this, $method)? else {
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

reflect_getter!(image_get_alt, "alt", "alt");
reflect_setter!(image_set_alt, "alt", "alt");
reflect_getter!(image_get_src, "src", "src");
reflect_setter!(image_set_src, "src", "src");
reflect_getter!(image_get_srcset, "srcset", "srcset");
reflect_setter!(image_set_srcset, "srcset", "srcset");
reflect_getter!(image_get_sizes, "sizes", "sizes");
reflect_setter!(image_set_sizes, "sizes", "sizes");
reflect_getter!(image_get_use_map, "usemap", "useMap");
reflect_setter!(image_set_use_map, "usemap", "useMap");

// ---------------------------------------------------------------------------
// Enumerated reflect
// ---------------------------------------------------------------------------

macro_rules! enum_getter {
    ($name:ident, $attr:literal, $method:literal, $values:ident, $missing:ident, $invalid:ident) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_image_receiver(ctx, this, $method)? else {
                return Ok(JsValue::String(ctx.vm.well_known.empty));
            };
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::String(ctx.vm.well_known.empty));
            }
            super::html_hyperlink_mixin::get_enumerated_reflect(
                ctx, entity, $attr, $values, $missing, $invalid,
            )
        }
    };
}

macro_rules! enum_setter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_image_receiver(ctx, this, $method)? else {
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

enum_getter!(
    image_get_cross_origin,
    "crossorigin",
    "crossOrigin",
    CROSS_ORIGIN_VALUES,
    CROSS_ORIGIN_MISSING_DEFAULT,
    CROSS_ORIGIN_INVALID_DEFAULT
);
enum_setter!(image_set_cross_origin, "crossorigin", "crossOrigin");

fn image_get_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_image_receiver(ctx, this, "referrerPolicy")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect_referrer_policy(ctx, entity)
}
enum_setter!(
    image_set_referrer_policy,
    "referrerpolicy",
    "referrerPolicy"
);

enum_getter!(
    image_get_decoding,
    "decoding",
    "decoding",
    DECODING_VALUES,
    DECODING_MISSING_DEFAULT,
    DECODING_INVALID_DEFAULT
);
enum_setter!(image_set_decoding, "decoding", "decoding");

enum_getter!(
    image_get_loading,
    "loading",
    "loading",
    LOADING_VALUES,
    LOADING_MISSING_DEFAULT,
    LOADING_INVALID_DEFAULT
);
enum_setter!(image_set_loading, "loading", "loading");

enum_getter!(
    image_get_fetch_priority,
    "fetchpriority",
    "fetchpriority",
    FETCH_PRIORITY_VALUES,
    FETCH_PRIORITY_MISSING_DEFAULT,
    FETCH_PRIORITY_INVALID_DEFAULT
);
enum_setter!(image_set_fetch_priority, "fetchpriority", "fetchpriority");

// ---------------------------------------------------------------------------
// Boolean reflect — isMap
// ---------------------------------------------------------------------------

fn image_get_is_map(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_image_receiver(ctx, this, "isMap")? else {
        return Ok(JsValue::Boolean(false));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Boolean(false));
    }
    let attr_sid = ctx.vm.strings.intern("ismap");
    let v = invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])?;
    Ok(v)
}

fn image_set_is_map(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_image_receiver(ctx, this, "isMap")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let truthy = super::super::coerce::to_boolean(ctx.vm, val);
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("ismap");
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
// Numeric reflect — width / height (engine-indep parse_unsigned_long)
// ---------------------------------------------------------------------------

macro_rules! numeric_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_image_receiver(ctx, this, $method)? else {
                return Ok(JsValue::Number(0.0));
            };
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::Number(0.0));
            }
            let attr_sid = ctx.vm.strings.intern($attr);
            let v = invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)])?;
            let raw = match v {
                JsValue::String(sid) => ctx.vm.strings.get_utf8(sid),
                _ => String::new(),
            };
            let parsed = parse_unsigned_long(&raw);
            Ok(JsValue::Number(f64::from(parsed)))
        }
    };
}

macro_rules! numeric_setter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_image_receiver(ctx, this, $method)? else {
                return Ok(JsValue::Undefined);
            };
            let arg = args.first().copied().unwrap_or(JsValue::Undefined);
            let n = super::super::coerce::to_uint32(ctx.vm, arg)?;
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::Undefined);
            }
            let attr_sid = ctx.vm.strings.intern($attr);
            let value_sid = ctx.vm.strings.intern(&n.to_string());
            invoke_dom_api(
                ctx,
                "setAttribute",
                entity,
                &[JsValue::String(attr_sid), JsValue::String(value_sid)],
            )
        }
    };
}

numeric_getter!(image_get_width, "width", "width");
numeric_setter!(image_set_width, "width", "width");
numeric_getter!(image_get_height, "height", "height");
numeric_setter!(image_set_height, "height", "height");

// ---------------------------------------------------------------------------
// Stubs — naturalWidth / naturalHeight / complete / decode()
// ---------------------------------------------------------------------------

fn image_get_natural_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let _ = require_image_receiver(ctx, this, "naturalWidth")?;
    Ok(JsValue::Number(0.0))
}

fn image_get_natural_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let _ = require_image_receiver(ctx, this, "naturalHeight")?;
    Ok(JsValue::Number(0.0))
}

fn image_get_complete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let _ = require_image_receiver(ctx, this, "complete")?;
    // Always-loaded approximation — paired with deferred slot
    // `#11-tags-T2a-img-natural-size` for the real signal.
    Ok(JsValue::Boolean(true))
}

fn image_decode_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let _ = require_image_receiver(ctx, this, "decode")?;
    // Paired with deferred slot `#11-tags-T2a-img-decode` for the
    // real Promise-based decode signal.  Returns a resolved Promise
    // so callers using `.decode().then(...)` proceed without error.
    let promise_id = super::super::natives_promise::create_promise(ctx.vm);
    super::super::natives_promise::settle_promise(ctx.vm, promise_id, false, JsValue::Undefined)?;
    Ok(JsValue::Object(promise_id))
}
