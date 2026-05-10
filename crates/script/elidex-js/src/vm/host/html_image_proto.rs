//! `HTMLImageElement.prototype` intrinsic — per-tag prototype layer
//! for `<img>` wrappers (HTML §4.8.4, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Engine-bound only.  Numeric reflect (HTML §"non-negative integer"
//! parse rule) lives in `elidex_dom_api::element::numeric_reflect`.
//!
//! ## Members planned (C3 phase)
//!
//! - **String reflect**: `alt` / `src` / `srcset` / `sizes` /
//!   `useMap`
//! - **Enumerated reflect canonical**: `crossOrigin` /
//!   `referrerPolicy` / `decoding` / `loading` / `fetchpriority`
//! - **Boolean reflect**: `isMap`
//! - **Numeric reflect** (`unsigned long`): `width` / `height`
//! - **Stub accessors** (defer slots):
//!   - `naturalWidth` / `naturalHeight` → `0`
//!   - `complete` → `true`
//!   - `decode()` → `Promise.resolve(undefined)`

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `HTMLImageElement.prototype` chained to
    /// `HTMLElement.prototype`.  C1 stub.
    pub(in crate::vm) fn register_html_image_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_image_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_image_prototype = Some(proto_id);
    }
}
