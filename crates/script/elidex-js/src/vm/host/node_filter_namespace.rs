//! `NodeFilter` namespace (WHATWG DOM §6.3) — installs the 13 SHOW_*
//! constants + 3 FILTER_* result constants on `globalThis.NodeFilter`.
//!
//! WHATWG specifies `NodeFilter` as a callback interface; in practice
//! browsers also expose it as a constructor-less namespace object so
//! `NodeFilter.SHOW_ELEMENT` resolves.  This file installs that
//! namespace object only — there is no constructor.

#![cfg(feature = "engine")]

use elidex_dom_api::traversal::{
    FILTER_ACCEPT, FILTER_REJECT, FILTER_SKIP, SHOW_ALL, SHOW_COMMENT, SHOW_DOCUMENT, SHOW_ELEMENT,
    SHOW_TEXT,
};

use super::super::shape;
use super::super::value::{
    JsValue, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
};
use super::super::VmInner;

// WHATWG §6.3 — full constant set (the engine-indep traversal module
// re-exports only the common subset).  Listed inline here since the
// remaining bits are spec-mandated literal values.
const SHOW_ATTRIBUTE: u32 = 0x2;
const SHOW_CDATA_SECTION: u32 = 0x8;
const SHOW_ENTITY_REFERENCE: u32 = 0x10;
const SHOW_ENTITY: u32 = 0x20;
const SHOW_PROCESSING_INSTRUCTION: u32 = 0x40;
const SHOW_DOCUMENT_TYPE: u32 = 0x200;
const SHOW_DOCUMENT_FRAGMENT: u32 = 0x400;
const SHOW_NOTATION: u32 = 0x800;

impl VmInner {
    /// Install `globalThis.NodeFilter` — a plain object carrying the
    /// 13 SHOW_* + 3 FILTER_* constants per WHATWG §6.3.
    pub(in crate::vm) fn register_node_filter_namespace(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_node_filter_namespace called before register_prototypes");

        let ns = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let entries = [
            (self.well_known.filter_accept, f64::from(FILTER_ACCEPT)),
            (self.well_known.filter_reject, f64::from(FILTER_REJECT)),
            (self.well_known.filter_skip, f64::from(FILTER_SKIP)),
            (self.well_known.show_all_const, f64::from(SHOW_ALL)),
            (self.well_known.show_element_const, f64::from(SHOW_ELEMENT)),
            (
                self.well_known.show_attribute_const,
                f64::from(SHOW_ATTRIBUTE),
            ),
            (self.well_known.show_text_const, f64::from(SHOW_TEXT)),
            (
                self.well_known.show_cdata_section_const,
                f64::from(SHOW_CDATA_SECTION),
            ),
            (
                self.well_known.show_entity_reference_const,
                f64::from(SHOW_ENTITY_REFERENCE),
            ),
            (self.well_known.show_entity_const, f64::from(SHOW_ENTITY)),
            (
                self.well_known.show_processing_instruction_const,
                f64::from(SHOW_PROCESSING_INSTRUCTION),
            ),
            (self.well_known.show_comment_const, f64::from(SHOW_COMMENT)),
            (
                self.well_known.show_document_const,
                f64::from(SHOW_DOCUMENT),
            ),
            (
                self.well_known.show_document_type_const,
                f64::from(SHOW_DOCUMENT_TYPE),
            ),
            (
                self.well_known.show_document_fragment_const,
                f64::from(SHOW_DOCUMENT_FRAGMENT),
            ),
            (
                self.well_known.show_notation_const,
                f64::from(SHOW_NOTATION),
            ),
        ];
        for (key, value) in entries {
            self.define_shaped_property(
                ns,
                PropertyKey::String(key),
                PropertyValue::Data(JsValue::Number(value)),
                shape::PropertyAttrs::WEBIDL_RO,
            );
        }

        let name_sid = self.well_known.node_filter_global;
        self.globals.insert(name_sid, JsValue::Object(ns));
    }
}
