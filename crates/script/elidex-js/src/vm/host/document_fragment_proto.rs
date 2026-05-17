//! `DocumentFragment.prototype` intrinsic (WHATWG DOM §4.7).
//!
//! Inherits `Node.prototype` and carries the ParentNode mixin
//! (`prepend` / `append` / `replaceChildren` + querySelector /
//! querySelectorAll / children / childElementCount /
//! firstElementChild / lastElementChild).  Used by:
//! - `document.createDocumentFragment()` wrappers
//! - `<template>.content` wrappers (via
//!   [`super::html_template_proto::create_fragment_wrapper`])
//! - `ShadowRoot` wrappers (via the prototype chain set by
//!   [`super::shadow_root_proto`])
//!
//! Brand check for DocumentFragment-specific methods is not needed
//! here: the only DF-specific surface is the ParentNode mixin, which
//! already brand-checks against `NodeKind::DocumentFragment` (in
//! addition to Element / Document) at call sites.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `DocumentFragment.prototype` with `Node.prototype` as
    /// parent and install the ParentNode mixin (prepend / append /
    /// replaceChildren) plus selector/children accessors.  Must run
    /// after `register_node_prototype` and before any prototype that
    /// inherits from DocumentFragment.prototype
    /// (e.g. `register_shadow_root_prototype`).
    pub(in crate::vm) fn register_document_fragment_prototype(&mut self) {
        let node_proto = self
            .node_prototype
            .expect("register_document_fragment_prototype before register_node_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(node_proto),
            extensible: true,
        });
        self.document_fragment_prototype = Some(proto_id);

        // Install the ParentNode mixin (prepend / append /
        // replaceChildren).  The mixin natives already brand-check
        // `NodeKind::DocumentFragment` via `is_parent_node_kind`,
        // so the install is safe to reuse here.
        self.install_parent_node_mixin(proto_id);
    }
}
