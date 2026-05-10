//! `HTMLScriptElement.prototype` intrinsic — per-tag prototype layer
//! for `<script>` wrappers (HTML §4.12.1, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Engine-bound only.  Script execution lifecycle is out of scope —
//! the existing HTML parser already runs scripts at parse time.
//! Post-parse `async`/`defer`/`noModule` mutations have no observable
//! effect (defer slot `#11-tags-T2a-script-load-lifecycle`).

#![cfg(feature = "engine")]

use elidex_dom_api::element::enumerated_reflect::{
    CROSS_ORIGIN_INVALID_DEFAULT, CROSS_ORIGIN_MISSING_DEFAULT, CROSS_ORIGIN_VALUES,
    FETCH_PRIORITY_INVALID_DEFAULT, FETCH_PRIORITY_MISSING_DEFAULT, FETCH_PRIORITY_VALUES,
};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_script_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_script_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_script_prototype = Some(proto_id);

        self.install_html_script_string_reflect(proto_id);
        self.install_html_script_enumerated_reflect(proto_id);
        self.install_html_script_bool_reflect(proto_id);
        self.install_html_script_text_accessor(proto_id);
    }

    fn install_html_script_string_reflect(&mut self, proto_id: ObjectId) {
        let pairs: [(&'static str, &'static str); 3] =
            [("src", "src"), ("type", "type"), ("integrity", "integrity")];
        for (idl_name, attr_name) in pairs {
            let getter = script_string_reflect_getter_for(attr_name);
            let setter = script_string_reflect_setter_for(attr_name);
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

    fn install_html_script_enumerated_reflect(&mut self, proto_id: ObjectId) {
        for (idl_name, getter, setter) in [
            (
                "crossOrigin",
                script_get_cross_origin as NativeFn,
                script_set_cross_origin as NativeFn,
            ),
            (
                "referrerPolicy",
                script_get_referrer_policy,
                script_set_referrer_policy,
            ),
            (
                "fetchpriority",
                script_get_fetch_priority,
                script_set_fetch_priority,
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

    fn install_html_script_bool_reflect(&mut self, proto_id: ObjectId) {
        for (idl_name, attr_name) in [
            ("async", "async"),
            ("defer", "defer"),
            ("noModule", "nomodule"),
        ] {
            let getter = script_bool_reflect_getter_for(attr_name);
            let setter = script_bool_reflect_setter_for(attr_name);
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

    fn install_html_script_text_accessor(&mut self, proto_id: ObjectId) {
        let sid = self.strings.intern("text");
        self.install_accessor_pair(
            proto_id,
            sid,
            script_get_text,
            Some(script_set_text),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_script_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, super::super::value::VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLScriptElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "script") {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLScriptElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String-reflect dispatch
// ---------------------------------------------------------------------------

fn script_string_reflect_getter_for(attr: &'static str) -> NativeFn {
    match attr {
        "src" => script_get_src,
        "type" => script_get_type_,
        "integrity" => script_get_integrity,
        _ => unreachable!("script_string_reflect_getter_for: {attr}"),
    }
}

fn script_string_reflect_setter_for(attr: &'static str) -> NativeFn {
    match attr {
        "src" => script_set_src,
        "type" => script_set_type_,
        "integrity" => script_set_integrity,
        _ => unreachable!("script_string_reflect_setter_for: {attr}"),
    }
}

macro_rules! reflect_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_script_receiver(ctx, this, $method)? else {
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
            let Some(entity) = require_script_receiver(ctx, this, $method)? else {
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

reflect_getter!(script_get_src, "src", "src");
reflect_setter!(script_set_src, "src", "src");
reflect_getter!(script_get_type_, "type", "type");
reflect_setter!(script_set_type_, "type", "type");
reflect_getter!(script_get_integrity, "integrity", "integrity");
reflect_setter!(script_set_integrity, "integrity", "integrity");

// ---------------------------------------------------------------------------
// Enumerated reflect
// ---------------------------------------------------------------------------

fn script_get_cross_origin(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_script_receiver(ctx, this, "crossOrigin")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect(
        ctx,
        entity,
        "crossorigin",
        CROSS_ORIGIN_VALUES,
        CROSS_ORIGIN_MISSING_DEFAULT,
        CROSS_ORIGIN_INVALID_DEFAULT,
    )
}
reflect_setter!(script_set_cross_origin, "crossorigin", "crossOrigin");

fn script_get_referrer_policy(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_script_receiver(ctx, this, "referrerPolicy")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    super::html_hyperlink_mixin::get_enumerated_reflect_referrer_policy(ctx, entity)
}
reflect_setter!(
    script_set_referrer_policy,
    "referrerpolicy",
    "referrerPolicy"
);

fn script_get_fetch_priority(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_script_receiver(ctx, this, "fetchpriority")? else {
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
reflect_setter!(script_set_fetch_priority, "fetchpriority", "fetchpriority");

// ---------------------------------------------------------------------------
// Boolean reflect
// ---------------------------------------------------------------------------

fn script_bool_reflect_getter_for(attr: &'static str) -> NativeFn {
    match attr {
        "async" => script_get_async,
        "defer" => script_get_defer,
        "nomodule" => script_get_no_module,
        _ => unreachable!("script_bool_reflect_getter_for: {attr}"),
    }
}

fn script_bool_reflect_setter_for(attr: &'static str) -> NativeFn {
    match attr {
        "async" => script_set_async,
        "defer" => script_set_defer,
        "nomodule" => script_set_no_module,
        _ => unreachable!("script_bool_reflect_setter_for: {attr}"),
    }
}

macro_rules! bool_getter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_script_receiver(ctx, this, $method)? else {
                return Ok(JsValue::Boolean(false));
            };
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::Boolean(false));
            }
            let attr_sid = ctx.vm.strings.intern($attr);
            invoke_dom_api(ctx, "hasAttribute", entity, &[JsValue::String(attr_sid)])
        }
    };
}

macro_rules! bool_setter {
    ($name:ident, $attr:literal, $method:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, super::super::value::VmError> {
            let Some(entity) = require_script_receiver(ctx, this, $method)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let truthy = super::super::coerce::to_boolean(ctx.vm, val);
            if ctx.host_if_bound().is_none() {
                return Ok(JsValue::Undefined);
            }
            let attr_sid = ctx.vm.strings.intern($attr);
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
    };
}

bool_getter!(script_get_async, "async", "async");
bool_setter!(script_set_async, "async", "async");
bool_getter!(script_get_defer, "defer", "defer");
bool_setter!(script_set_defer, "defer", "defer");
bool_getter!(script_get_no_module, "nomodule", "noModule");
bool_setter!(script_set_no_module, "nomodule", "noModule");

// ---------------------------------------------------------------------------
// `<script>.text` — textContent alias
// ---------------------------------------------------------------------------

fn script_get_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_script_receiver(ctx, this, "text")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn script_set_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, super::super::value::VmError> {
    let Some(entity) = require_script_receiver(ctx, this, "text")? else {
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
