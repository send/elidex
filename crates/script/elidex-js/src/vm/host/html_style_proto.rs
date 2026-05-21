//! `HTMLStyleElement.prototype` intrinsic — per-tag prototype layer
//! for `<style>` wrappers (HTML §4.2.6, slot `#11-tags-T2b-passive`).
//!
//! Surfaces:
//! - `media` — string reflect (matches anchor/script string-reflect
//!   pattern; T2b leaves the cascade-side media-query evaluation to
//!   the existing CSS pipeline).
//! - `type` — string reflect (no enumerated canonicalisation; spec
//!   §4.2.6 leaves the value uninterpreted at the IDL surface).
//! - `sheet` — `[SameObject]` `CSSStyleSheet` wrapper from PR-B's
//!   [`super::cssom_sheet::native_html_style_get_sheet`] (interned
//!   under `WrapperKind::StyleSheet`).
//!
//! `disabled` is intentionally NOT installed: HTML §4.2.6 +
//! CSSOM §6.2 specify shared cascade-application semantics with
//! `<style>.sheet.disabled`, which PR-B #177 deferred to slot
//! `#11-stylesheet-disabled` pending cross-crate cascade plumbing.
//! Adding HTMLStyleElement.disabled here without the cascade
//! integration would silently no-op the toggle (worse than not
//! exposing it at all).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  The CSSOM
//! sheet allocation lives in the engine-bound [`super::cssom_sheet`]
//! module (the `<style>` Entity → CSSStyleSheet wrapper identity
//! cache is a VM-side concern); the upstream stylesheet parse + rule
//! list live engine-indep in `elidex_dom_api::cssom_sheet`.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_style_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_style_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_style_prototype = Some(proto_id);

        let media_sid = self.strings.intern("media");
        self.install_accessor_pair(
            proto_id,
            media_sid,
            style_get_media,
            Some(style_set_media),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let type_sid = self.strings.intern("type");
        self.install_accessor_pair(
            proto_id,
            type_sid,
            style_get_type,
            Some(style_set_type),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `<style>.sheet` getter — installed here (not on
        // `HTMLElement.prototype`) so the surface is visible only on
        // `<style>` elements, matching WebIDL.  Backing fn lives in
        // `cssom_sheet.rs` next to `alloc_or_cached_stylesheet` so the
        // sheet wrapper allocator + accessor stay co-located.
        self.install_accessor_pair(
            proto_id,
            self.well_known.sheet,
            super::cssom_sheet::native_html_style_get_sheet,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_style_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLStyleElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "style") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLStyleElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

macro_rules! style_reflect_getter {
    ($name:ident, $attr:literal, $idl:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_style_receiver(ctx, this, $idl)? else {
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

macro_rules! style_reflect_setter {
    ($name:ident, $attr:literal, $idl:literal) => {
        fn $name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_style_receiver(ctx, this, $idl)? else {
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

style_reflect_getter!(style_get_media, "media", "media");
style_reflect_setter!(style_set_media, "media", "media");
style_reflect_getter!(style_get_type, "type", "type");
style_reflect_setter!(style_set_type, "type", "type");
