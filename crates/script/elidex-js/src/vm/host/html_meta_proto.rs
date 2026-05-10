//! `HTMLMetaElement.prototype` intrinsic — per-tag prototype layer
//! for `<meta>` wrappers (HTML §4.2.5, slot `#11-tags-T2b-passive`).
//!
//! Six DOMString reflect IDL attributes:
//!
//! | IDL attribute | Content attribute |
//! |---------------|-------------------|
//! | `name`        | `name`            |
//! | `httpEquiv`   | `http-equiv`      |
//! | `content`     | `content`         |
//! | `charset`     | `charset`         |
//! | `media`       | `media`           |
//! | `scheme`      | `scheme`          |
//!
//! `scheme` is deprecated per HTML §4.2.5.4 ("the scheme attribute is
//! deprecated") but reflected for legacy scripts that still read
//! `<meta scheme>` — surface compatibility wins over spec purity.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  Each accessor
//! is a thin shim over the `getAttribute` / `setAttribute` dom-api
//! handlers.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_meta_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_meta_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_meta_prototype = Some(proto_id);

        // The IDL ↔ content-attribute mapping (`httpEquiv` ↔
        // `http-equiv` etc.) is encoded in the per-attr macro
        // expansions below, so this install loop only needs the IDL
        // name (for the property key) plus its getter/setter pair.
        let pairs: [(&'static str, NativeFn, NativeFn); 6] = [
            ("name", meta_get_name, meta_set_name),
            ("httpEquiv", meta_get_http_equiv, meta_set_http_equiv),
            ("content", meta_get_content, meta_set_content),
            ("charset", meta_get_charset, meta_set_charset),
            ("media", meta_get_media, meta_set_media),
            ("scheme", meta_get_scheme, meta_set_scheme),
        ];
        for (idl_name, getter, setter) in pairs {
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
}

fn require_meta_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLMetaElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "meta") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLMetaElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

macro_rules! meta_reflect_getter {
    ($name:ident, $attr:literal, $idl:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_meta_receiver(ctx, this, $idl)? else {
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

macro_rules! meta_reflect_setter {
    ($name:ident, $attr:literal, $idl:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_meta_receiver(ctx, this, $idl)? else {
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

meta_reflect_getter!(meta_get_name, "name", "name");
meta_reflect_setter!(meta_set_name, "name", "name");
meta_reflect_getter!(meta_get_http_equiv, "http-equiv", "httpEquiv");
meta_reflect_setter!(meta_set_http_equiv, "http-equiv", "httpEquiv");
meta_reflect_getter!(meta_get_content, "content", "content");
meta_reflect_setter!(meta_set_content, "content", "content");
meta_reflect_getter!(meta_get_charset, "charset", "charset");
meta_reflect_setter!(meta_set_charset, "charset", "charset");
meta_reflect_getter!(meta_get_media, "media", "media");
meta_reflect_setter!(meta_set_media, "media", "media");
meta_reflect_getter!(meta_get_scheme, "scheme", "scheme");
meta_reflect_setter!(meta_set_scheme, "scheme", "scheme");
