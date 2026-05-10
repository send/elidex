//! `HTMLTableCaptionElement.prototype` intrinsic — per-tag prototype
//! layer for `<caption>` wrappers (HTML §4.9.2, slot
//! `#11-tags-T2c-table`).
//!
//! Brand-only: the deprecated `align` attribute is intentionally not
//! surfaced (defer slot `#11-tags-deprecated-attr-sweep`).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", prototype install only.

#![cfg(feature = "engine")]

use super::super::VmInner;

impl VmInner {
    pub(in crate::vm) fn register_html_table_caption_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_table_caption_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_table_caption_prototype = Some(proto_id);
    }
}
