//! `HTMLTemplateElement.prototype` intrinsic — per-tag prototype layer
//! for `<template>` wrappers (HTML §4.12.3, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.12.3):
//! - `content` — `[SameObject]` DocumentFragment.  Each `<template>`
//!   element has an associated DocumentFragment that holds its parsed
//!   children; M4-12 ships **lazy allocation** (first JS-side access
//!   creates a fresh fragment Entity, cached per-template).  Parser-
//!   side population (where html5ever inserts children directly into
//!   the fragment) is deferred to slot `#11-template-parser-content`
//!   paired with the html5ever replacement (Phase 5).  Until then
//!   dynamic JS `<template>` (e.g. `document.createElement('template')`
//!   followed by `tpl.content.appendChild(...)`) works.
//!
//! Declarative shadow-DOM IDL attrs (`shadowRootMode` /
//! `shadowRootDelegatesFocus` / `shadowRootClonable` /
//! `shadowRootSerializable`) are folded into the existing
//! `#11-shadow-dom-surface` (D-15) slot — they reflect content
//! attributes whose consumer is the parser, so they ship together
//! with shadow-DOM.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  DocumentFragment
//! creation is the existing engine-indep
//! `EcsDom::create_document_fragment_with_owner` path.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::VmInner;

impl VmInner {
    pub(in crate::vm) fn register_html_template_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_template_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_template_prototype = Some(proto_id);

        let content_sid = self.strings.intern("content");
        self.install_accessor_pair(
            proto_id,
            content_sid,
            template_get_content,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

fn require_template_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTemplateElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "template") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTemplateElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// `<template>.content` — `[SameObject]` DocumentFragment.  Lazy-
/// allocated on first JS-side read; cached per-template Entity in
/// [`VmInner::template_content_wrappers`].  The fragment's owner
/// document mirrors the template's owner document (via
/// `EcsDom::associated_document`).
fn template_get_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_template_receiver(ctx, this, "content")? else {
        return Ok(JsValue::Null);
    };
    if let Some(&cached) = ctx.vm.template_content_wrappers.get(&entity) {
        return Ok(JsValue::Object(cached));
    }
    if ctx.host_if_bound().is_none() {
        // Post-unbind: nothing to allocate against (no DOM world).
        return Ok(JsValue::Null);
    }
    let owner_doc = ctx.host().dom().get_associated_document(entity);
    let fragment_entity = ctx
        .host()
        .dom()
        .create_document_fragment_with_owner(owner_doc);
    let wrapper_id = create_fragment_wrapper(ctx, fragment_entity);
    ctx.vm.template_content_wrappers.insert(entity, wrapper_id);
    Ok(JsValue::Object(wrapper_id))
}

/// Create the wrapper for `fragment_entity` via the canonical
/// element-wrapper path.  `create_element_wrapper` routes
/// `DocumentFragment` node-kinds to `Node.prototype` directly, so the
/// returned wrapper carries Node-level members (`appendChild`, etc.)
/// without needing a dedicated `DocumentFragment.prototype` install.
fn create_fragment_wrapper(ctx: &mut NativeContext<'_>, fragment_entity: Entity) -> ObjectId {
    ctx.vm.create_element_wrapper(fragment_entity)
}
