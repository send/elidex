//! `DocumentFragment.prototype` intrinsic (WHATWG DOM §4.7).
//!
//! Inherits `Node.prototype` and carries the full ParentNode mixin
//! (WHATWG §4.2.6) — mutation methods (`prepend` / `append` /
//! `replaceChildren`) via
//! [`super::super::VmInner::install_parent_node_mixin`] and the read
//! surface (`children` / `firstElementChild` / `lastElementChild` /
//! `childElementCount` / `querySelector` / `querySelectorAll`) via
//! [`super::super::VmInner::install_parent_node_readers`].  Both
//! installs are shared with `Element.prototype` and the Document
//! wrapper.
//!
//! Used by:
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
    /// Allocate `DocumentFragment.prototype` with `Node.prototype`
    /// parent + install the ParentNode mixin (mutation + read).
    /// Must run after `register_node_prototype` and before any
    /// inheritor (`register_shadow_root_prototype`).
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

        self.install_parent_node_mixin(proto_id);
        self.install_parent_node_readers(proto_id);
    }
}
