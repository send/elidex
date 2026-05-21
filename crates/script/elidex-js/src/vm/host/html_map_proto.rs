//! `HTMLMapElement.prototype` intrinsic — per-tag prototype layer
//! for `<map>` wrappers (HTML §4.8.13, slot `#11-tags-T2b-passive`).
//!
//! Surfaces:
//! - `name` — DOMString reflect of the `name` content attribute.
//! - `areas` — `[SameObject]` `HTMLCollection` of descendant `<area>`
//!   elements.  Live: subsequent mutations of `<area>` descendants
//!   are visible through the same wrapper.  Backed by
//!   [`elidex_dom_api::live_collection::LiveCollection`] with
//!   `CollectionFilter::ByTagName("area")` rooted at the `<map>`
//!   entity, interned under `WrapperKind::MapAreas`.
//!
//! Deprecated alias `<map>.images` is intentionally not surfaced —
//! it's been a no-op in browsers since IE6 and does not appear in
//! any modern script (defer slot `#11-tags-deprecated-attr-sweep`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  The
//! descendant-walk + tag-match algorithm lives engine-indep in
//! `elidex_dom_api::live_collection`.

#![cfg(feature = "engine")]

use elidex_dom_api::{CollectionFilter, CollectionKind, LiveCollection};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

impl VmInner {
    pub(in crate::vm) fn register_html_map_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_map_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_map_prototype = Some(proto_id);

        let name_sid = self.strings.intern("name");
        self.install_accessor_pair(
            proto_id,
            name_sid,
            map_get_name,
            Some(map_set_name),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let areas_sid = self.strings.intern("areas");
        self.install_accessor_pair(
            proto_id,
            areas_sid,
            map_get_areas,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    /// Allocate (or return cached) `<map>.areas` HTMLCollection wrapper
    /// for `owner`.  HTML §4.8.13 mandates `[SameObject]` so two
    /// reads of `m.areas` return the identical `ObjectId`.
    pub(crate) fn alloc_or_cached_map_areas(&mut self, owner: Entity) -> ObjectId {
        self.intern_wrapper(WrapperKey::entity(owner, WrapperKind::MapAreas), |vm| {
            let coll = LiveCollection::new(
                owner,
                CollectionFilter::ByTagName("area".to_string()),
                CollectionKind::HtmlCollection,
            );
            vm.alloc_collection(coll)
        })
    }
}

fn require_map_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLMapElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "map") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLMapElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn map_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_map_receiver(ctx, this, "name")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.strings.intern("name");
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn map_set_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_map_receiver(ctx, this, "name")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.strings.intern("name");
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

fn map_get_areas(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_map_receiver(ctx, this, "areas")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_map_areas(entity);
    Ok(JsValue::Object(id))
}
