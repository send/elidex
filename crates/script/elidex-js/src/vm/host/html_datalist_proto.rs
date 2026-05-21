//! `HTMLDataListElement.prototype` intrinsic — per-tag prototype layer
//! for `<datalist>` wrappers (HTML §4.10.10, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.10.10):
//! - `options` — `[SameObject]` HTMLCollection of descendant `<option>`
//!   elements.  Reuses the existing `CollectionFilter::Options` filter
//!   (same shape as `<select>.options`).  Identity is preserved per
//!   `[SameObject]` by interning under `WrapperKind::DatalistOptions`.
//!
//! `<input>.list` back-reference is deferred to slot
//! `#11-tags-T2d-input-list` (the `<input>.list` accessor still
//! returns `null` from the existing stub at
//! `html_input_proto.rs:913-921`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.

#![cfg(feature = "engine")]

use elidex_dom_api::{CollectionFilter, CollectionKind, LiveCollection};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::VmInner;

impl VmInner {
    pub(in crate::vm) fn register_html_datalist_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_datalist_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_datalist_prototype = Some(proto_id);

        let options_sid = self.strings.intern("options");
        self.install_accessor_pair(
            proto_id,
            options_sid,
            datalist_get_options,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    /// Allocate (or return cached) `<datalist>.options` HTMLCollection
    /// wrapper.  HTML §4.10.10 mandates `[SameObject]`.
    pub(crate) fn alloc_or_cached_datalist_options(&mut self, owner: Entity) -> ObjectId {
        self.intern_wrapper(
            WrapperKey::entity(owner, WrapperKind::DatalistOptions),
            |vm| {
                let coll = LiveCollection::new(
                    owner,
                    CollectionFilter::Options,
                    CollectionKind::HtmlCollection,
                );
                vm.alloc_collection(coll)
            },
        )
    }
}

fn require_datalist_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLDataListElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "datalist") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLDataListElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn datalist_get_options(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_datalist_receiver(ctx, this, "options")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_or_cached_datalist_options(entity);
    Ok(JsValue::Object(id))
}
