//! `HTMLTemplateElement.prototype` intrinsic — per-tag prototype layer
//! for `<template>` wrappers (HTML §4.12.3, slot
//! `#11-tags-T2d-interactive`).
//!
//! IDL surface (HTML §4.12.3):
//! - `content` — the associated DocumentFragment that holds the
//!   template's parsed children.  The fragment is created **eagerly at
//!   element creation** (slot `#11-template-parser-content`) by the
//!   shared `EcsDom::attach_template_contents` helper — both parser
//!   tiers, `createElement('template')`, and the `cloneNode` post-pass
//!   route through it — so by the time JS reads `.content` the fragment
//!   already exists and (for parsed templates) is already populated.
//!   This getter just returns the linked fragment.  Its stable identity
//!   (the §4.12.3 *associated DocumentFragment* — two reads return the
//!   same object) comes from the fragment entity's own primary-Node
//!   wrapper cache (`create_element_wrapper` dedups on the entity), not
//!   a dedicated wrapper-intern key.
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
//! Per CLAUDE.md "Layering mandate", marshalling-only.  The content
//! fragment's creation / linkage lives in the engine-indep
//! `EcsDom::attach_template_contents` helper (shared with both parser
//! tiers, `createElement`, and clone); this getter only reads the link
//! and marshals the fragment entity to a wrapper.

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

/// `<template>.content` — the associated content DocumentFragment
/// (HTML §4.12.3).  Returns the fragment linked by
/// `EcsDom::attach_template_contents` (attached eagerly at element
/// creation across every tier).  SameObject identity is the fragment
/// entity's own primary-Node wrapper cache (`create_element_wrapper`
/// dedups on the entity), so repeated reads return the same object — no
/// separate wrapper-intern key (§8 One-issue-one-way).
fn template_get_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_template_receiver(ctx, this, "content")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        // Post-unbind: no DOM world to read or allocate against.
        return Ok(JsValue::Null);
    }
    // Read the eagerly-attached fragment (Layering mandate — this getter is
    // marshalling-only; fragment creation lives solely in the engine-indep
    // `attach_template_contents` SSoT, called by every creation site). Every
    // `<template>` has a fragment by construction, so the `None` arm is
    // unreachable; assert in debug and surface `null` rather than fabricating a
    // second creation path here.
    let Some(fragment_entity) = ctx.host().dom().template_contents_fragment(entity) else {
        debug_assert!(
            false,
            "every <template> has an eagerly-attached content fragment \
             (#11-template-parser-content)"
        );
        return Ok(JsValue::Null);
    };
    Ok(JsValue::Object(create_fragment_wrapper(
        ctx,
        fragment_entity,
    )))
}

/// Create the wrapper for `fragment_entity` via the canonical
/// element-wrapper path.  `create_element_wrapper` routes
/// `DocumentFragment` node-kinds to `Node.prototype` directly, so the
/// returned wrapper carries Node-level members (`appendChild`, etc.)
/// without needing a dedicated `DocumentFragment.prototype` install.
fn create_fragment_wrapper(ctx: &mut NativeContext<'_>, fragment_entity: Entity) -> ObjectId {
    ctx.vm.create_element_wrapper(fragment_entity)
}
