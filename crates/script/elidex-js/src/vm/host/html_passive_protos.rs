//! Brand-only `HTMLElement` per-tag prototype family for the T2b
//! passive grouping bundle (slot `#11-tags-T2b-passive`).
//!
//! Houses the 14 prototypes that surface no IDL attributes beyond the
//! inherited `HTMLElement` / `Element` / `Node` / `EventTarget`
//! members: html / head / body / div / span / br / hr / pre / p /
//! heading (shared h1-h6) / ul / dl / menu / picture.  Each is a fresh `Ordinary` object chained to
//! `HTMLElement.prototype`; the only observable difference from
//! `HTMLElement.prototype` itself is identity (so brand checks like
//! `Object.getPrototypeOf(div) === HTMLDivElement.prototype` hold).
//!
//! Single-file consolidation rationale: per-element files would each
//! be < 35 LoC, dominated by header / mod boilerplate.  The
//! `html_legend_proto.rs` style is reserved for prototypes that
//! actually carry accessors (HTMLTitle / HTMLBase / HTMLMeta /
//! HTMLStyle / HTMLQuote / HTMLOList / HTMLLI / HTMLMap / HTMLData /
//! HTMLTime — each lands in its own file).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate": prototype install only, no
//! algorithm, no accessor bodies.  Brand checks (when any accessor on
//! the inherited surface needs to verify the receiver is the right
//! element kind) live next to the accessor that needs them — not
//! here.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Install all 14 brand-only T2b passive prototypes in one pass.
    /// Each chains to `HTMLElement.prototype`; the call MUST happen
    /// after `register_html_element_prototype` so the parent slot is
    /// populated.
    pub(in crate::vm) fn register_html_passive_brand_only_prototypes(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_passive_brand_only_prototypes called before register_html_element_prototype",
        );
        self.html_html_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_head_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_body_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_div_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_span_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_br_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_hr_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_pre_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_p_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_heading_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_ulist_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_dlist_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_menu_prototype = Some(self.alloc_html_subclass_prototype(parent));
        self.html_picture_prototype = Some(self.alloc_html_subclass_prototype(parent));
    }

    /// Allocate a fresh `HTMLElement` subclass prototype chained to
    /// `parent` (typically `HTMLElement.prototype`).  Carries no own
    /// properties; per-tag accessor / method install happens at the
    /// caller site.  Centralises the `alloc_object(Object { kind:
    /// Ordinary, storage: shaped(ROOT_SHAPE), prototype: Some(parent),
    /// extensible: true })` snippet that every `register_html_<tag>_prototype`
    /// fn used to inline by hand.
    pub(in crate::vm) fn alloc_html_subclass_prototype(&mut self, parent: ObjectId) -> ObjectId {
        self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        })
    }
}
