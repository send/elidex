//! `HTMLScriptElement.prototype` intrinsic — per-tag prototype layer
//! for `<script>` wrappers (HTML §4.12.1, slot `#11-tags-T2a-url-bearing`).
//!
//! ## Layering
//!
//! Engine-bound only.  Script execution lifecycle is out of scope —
//! the existing HTML parser already runs scripts at parse time.
//! Post-parse async/defer/noModule mutations have no observable
//! effect (defer slot `#11-tags-T2a-script-load-lifecycle`).
//!
//! ## Members planned (C3 phase)
//!
//! - **String reflect**: `src` / `type` / `integrity`
//! - **`text` accessor**: `textContent` alias (explicit prototype
//!   install, not inherited — same as `<a>.text`)
//! - **Enumerated reflect canonical**: `crossOrigin` /
//!   `referrerPolicy` / `fetchpriority`
//! - **Boolean reflect**: `async` / `defer` / `noModule`

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `HTMLScriptElement.prototype` chained to
    /// `HTMLElement.prototype`.  C1 stub.
    pub(in crate::vm) fn register_html_script_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_script_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_script_prototype = Some(proto_id);
    }
}
