//! `HTMLLinkElement.prototype` intrinsic — per-tag prototype layer
//! for `<link>` wrappers (HTML §4.6.7, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Engine-bound only.  Stylesheet load lifecycle and `link.sheet`
//! accessor are deferred (`#11-tags-T2a-link-stylesheet`, paired with
//! PR-B `#11-link-stylesheet-loading`).  `<link rel="stylesheet">.sheet`
//! returns `null` in this PR.
//!
//! ## Members planned (C3 phase)
//!
//! - **String reflect**: `href` / `media` / `hreflang` / `type` /
//!   `integrity` / `imageSrcset` / `imageSizes` / `as`
//! - **Enumerated reflect canonical**: `crossOrigin` /
//!   `referrerPolicy` / `fetchpriority`
//! - **Boolean reflect**: `disabled`
//! - **DOMTokenList**: `relList` (shared generalised wrapper, separate
//!   cache `link_rel_list_wrapper_cache`) + `sizes`
//!   (`[SameObject, PutForwards=value] DOMTokenList`, separate cache
//!   `link_sizes_wrapper_cache`)
//! - **Stub**: `sheet` → `null`

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `HTMLLinkElement.prototype` chained to
    /// `HTMLElement.prototype`.  C1 stub.
    pub(in crate::vm) fn register_html_link_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_link_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_link_prototype = Some(proto_id);
    }
}
