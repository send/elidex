//! `HTMLAreaElement.prototype` intrinsic — per-tag prototype layer
//! for `<area>` wrappers (HTML §4.6.2, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Same engine-bound discipline as `html_anchor_proto.rs` — this
//! module is marshalling-only; URL accessor mixin algorithm lives in
//! `elidex_dom_api::element::href_accessor`.
//!
//! ## Members planned (C3 phase)
//!
//! - **HTMLHyperlinkElementUtils mixin**: same 11 IDL attributes +
//!   `toString()` as anchor (shared install fn)
//! - **String reflect**: `alt` / `coords` / `target` / `download` /
//!   `ping`
//! - **Enumerated reflect canonical**: `shape` (`circle` / `default` /
//!   `poly` / `rect`、missing+invalid default = `rect`) /
//!   `referrerPolicy`
//! - **`relList`**: `DOMTokenList` backed by `rel` attr

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `HTMLAreaElement.prototype` chained to
    /// `HTMLElement.prototype`.  C1 stub.
    pub(in crate::vm) fn register_html_area_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_area_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_area_prototype = Some(proto_id);
    }
}
