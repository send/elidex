//! Element (and more generally, DOM) wrapper creation.
//!
//! `create_element_wrapper(entity)` is the single entry point used by
//! every host-side DOM API that needs to surface an Entity as a JS
//! object.  It enforces two invariants:
//!
//! 1. **Identity** — `el === el` across repeated lookups.  The
//!    wrapper `ObjectId` is cached in `HostData::wrapper_cache`, keyed
//!    by `Entity::to_bits().get()`.  A cache hit returns the existing
//!    ObjectId without allocating.
//! 2. **Prototype chain dispatched by node kind** —
//!    `HostData::prototype_kind_for` routes each entity to one of
//!    four prototype chains:
//!    - Element (`TagType` present) →
//!      `Element.prototype → Node.prototype → EventTarget.prototype`
//!    - Text (`NodeKind::Text` or legacy `TextContent`) →
//!      `Text.prototype → CharacterData.prototype → Node.prototype`
//!    - Other CharacterData (Comment / CDATA / PI) →
//!      `CharacterData.prototype → Node.prototype`
//!    - Other Node (Document / DocumentFragment / DocumentType /
//!      unclassified) → `Node.prototype` directly.
//!
//!    All chains terminate at `Object.prototype` via `Node.prototype
//!    → EventTarget.prototype`, so Node-level members (`parentNode`,
//!    `nodeType`, `textContent`, …) are visible on every DOM wrapper.
//!    Window is wrapped independently (see `vm/globals.rs`) and does
//!    *not* chain through `Node.prototype` — Window is an
//!    EventTarget but not a Node per WHATWG.
//!
//! The wrapper carries only `ObjectKind::HostObject { entity_bits }`
//! and its prototype slot — no properties are installed at creation.
//! Per-interface methods (e.g. `getAttribute`, `textContent`) are
//! installed on the shared prototypes rather than duplicated onto
//! each wrapper.

#[cfg(feature = "engine")]
use super::super::shape;
#[cfg(feature = "engine")]
use super::super::value::{Object, ObjectId, ObjectKind, PropertyStorage};
#[cfg(feature = "engine")]
use super::super::VmInner;

#[cfg(feature = "engine")]
impl VmInner {
    /// Return the shared JS wrapper ObjectId for `entity`, allocating a
    /// new `HostObject` on the first call and reusing the cached one on
    /// every subsequent call.
    ///
    /// # Panics
    ///
    /// Panics if `HostData` has not been *installed* via
    /// `Vm::install_host_data` (the cache lives on `HostData` so
    /// nowhere to cache the result), or if `event_target_prototype`
    /// has not been initialised (`register_globals` not yet run —
    /// should be impossible after `Vm::new` returns).
    ///
    /// Bind state is **irrelevant** here: the wrapper cache is a
    /// HashMap on `HostData`, not a session/dom dereference, so this
    /// function works after `Vm::unbind()` too — useful for code
    /// paths that build wrappers as part of pre-eval setup.  Calling
    /// methods on the returned wrapper that touch `dom()` does still
    /// require a bound HostData; see the per-native checks in
    /// `vm/host/event_target.rs`.
    ///
    /// # GC safety
    ///
    /// `alloc_object` may trigger a collection before the new object
    /// is installed.  The caller must not hold any `&Object` references
    /// across this call.  The freshly-returned `ObjectId` is rooted by
    /// `wrapper_cache` immediately after allocation; until that point
    /// the only reference is the local — no GC-traceable structure
    /// points at it, and no intervening allocation happens, so GC
    /// cannot run in that window.
    pub(crate) fn create_element_wrapper(&mut self, entity: elidex_ecs::Entity) -> ObjectId {
        // Fast path: identity cache hit.  `HostData` borrow is scoped
        // to this block so the subsequent `alloc_object` call (which
        // needs `&mut self`) is unblocked on miss.
        if let Some(existing) = self
            .host_data
            .as_deref()
            .and_then(|hd| hd.get_cached_wrapper(entity))
        {
            return existing;
        }

        // Pick the prototype based on the entity's DOM node kind.
        // `prototype_kind_for` centralises the Element / Text /
        // Comment / other-Node dispatch for wrapper creation:
        //
        // - Element             → `Element.prototype`
        //                         (→ Node.prototype → EventTarget.prototype).
        // - Text                → `Text.prototype`
        //                         (→ CharacterData.prototype → Node.prototype).
        // - Comment / PI / CDATA → `CharacterData.prototype`
        //                         (→ Node.prototype).
        // - Document / DocumentFragment / DocumentType / unbound
        //                       → `Node.prototype` directly.
        //
        // Pre-bind / unbound wrapper allocation falls through to the
        // OtherNode branch (Node.prototype); method calls on that
        // wrapper route through `entity_from_this`, which
        // short-circuits to a no-op while unbound.
        //
        // `Window` is NOT wrapped via this path — it gets an
        // independent `HostObject` allocated in `register_globals`
        // whose prototype chain skips `Node.prototype` so Node
        // members do not appear on `window` (WHATWG: Window is an
        // EventTarget but not a Node).
        let kind = self
            .host_data
            .as_deref()
            .map_or(super::super::host_data::PrototypeKind::OtherNode, |hd| {
                hd.prototype_kind_for(entity)
            });
        let proto = match kind {
            super::super::host_data::PrototypeKind::Element => {
                // Tag-specific secondary lookup.  Each known tag
                // routes through its own per-tag prototype; unknown
                // tags fall back to the shared
                // `HTMLElement.prototype` so `div instanceof
                // HTMLElement === true` (WHATWG §3.2.8).
                self.tag_specific_html_prototype(entity)
                    .or(self.html_element_prototype)
                    .or(self.element_prototype)
                    .expect("create_element_wrapper called before register_element_prototype")
            }
            super::super::host_data::PrototypeKind::Text => {
                // Text wrappers chain `Text.prototype →
                // CharacterData.prototype`; fall back to
                // `CharacterData.prototype` during the narrow
                // bootstrap window after CharacterData is registered
                // but before `register_text_prototype` runs.
                self.text_prototype
                    .or(self.character_data_prototype)
                    .expect(
                        "create_element_wrapper called before register_character_data_prototype",
                    )
            }
            super::super::host_data::PrototypeKind::OtherCharacterData => self
                .character_data_prototype
                .expect("create_element_wrapper called before register_character_data_prototype"),
            super::super::host_data::PrototypeKind::DocumentType => self
                .document_type_prototype
                .or(self.node_prototype)
                .expect("create_element_wrapper called before register_node_prototype"),
            super::super::host_data::PrototypeKind::DocumentFragment => self
                .document_fragment_prototype
                .or(self.node_prototype)
                .expect("create_element_wrapper called before register_node_prototype"),
            super::super::host_data::PrototypeKind::OtherNode => self
                .node_prototype
                .expect("create_element_wrapper called before register_node_prototype"),
        };
        let obj = self.alloc_object(Object {
            kind: ObjectKind::HostObject {
                entity_bits: entity.to_bits().get(),
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        });

        // Register in the wrapper cache so the next lookup for this
        // Entity returns the same ObjectId (and the object stays
        // rooted via `HostData::gc_root_object_ids`).
        self.host_data
            .as_deref_mut()
            .expect("create_element_wrapper requires installed HostData")
            .cache_wrapper(entity, obj);
        obj
    }

    /// Resolve a per-tag HTML element prototype from a tag string.
    /// Returns `None` for any tag without a registered tag-specific
    /// prototype, in which case `create_element_wrapper` falls back
    /// to `HTMLElement.prototype`.  Slot `#11-tags-T1-v2` extends
    /// the dispatch with the 10 form-control tags (Group α K-3
    /// fold); slot `#11-tags-T2a-url-bearing` adds 5 URL-bearing
    /// tags; slot `#11-tags-T2b-passive` adds the 7 head, 17
    /// grouping, and empty tags (h1-h6 and blockquote+q each route
    /// to a single shared prototype).
    fn tag_specific_html_prototype(&self, entity: elidex_ecs::Entity) -> Option<ObjectId> {
        let host = self.host_data.as_deref()?;
        // Linear chain of `tag_matches_ascii_case` checks.  Each
        // call walks the entity's `TagType` component without
        // allocating; for the ~40 tags in scope this is still well
        // below the cost of a per-call `to_ascii_lowercase`.  An
        // O(1) lookup table remains a separate, benchmark-driven
        // optimisation (defer slot in the master roadmap §H-7).
        if host.tag_matches_ascii_case(entity, "iframe") {
            return self.html_iframe_prototype;
        }
        if host.tag_matches_ascii_case(entity, "a") {
            return self.html_anchor_prototype;
        }
        if host.tag_matches_ascii_case(entity, "area") {
            return self.html_area_prototype;
        }
        if host.tag_matches_ascii_case(entity, "img") {
            return self.html_image_prototype;
        }
        if host.tag_matches_ascii_case(entity, "script") {
            return self.html_script_prototype;
        }
        if host.tag_matches_ascii_case(entity, "link") {
            return self.html_link_prototype;
        }
        if host.tag_matches_ascii_case(entity, "label") {
            return self.html_label_prototype;
        }
        if host.tag_matches_ascii_case(entity, "optgroup") {
            return self.html_optgroup_prototype;
        }
        if host.tag_matches_ascii_case(entity, "legend") {
            return self.html_legend_prototype;
        }
        if host.tag_matches_ascii_case(entity, "option") {
            return self.html_option_prototype;
        }
        if host.tag_matches_ascii_case(entity, "fieldset") {
            return self.html_fieldset_prototype;
        }
        if host.tag_matches_ascii_case(entity, "form") {
            return self.html_form_prototype;
        }
        if host.tag_matches_ascii_case(entity, "button") {
            return self.html_button_prototype;
        }
        if host.tag_matches_ascii_case(entity, "textarea") {
            return self.html_textarea_prototype;
        }
        if host.tag_matches_ascii_case(entity, "select") {
            return self.html_select_prototype;
        }
        if host.tag_matches_ascii_case(entity, "input") {
            return self.html_input_prototype;
        }
        self.tag_specific_t2b_prototype(entity)
    }

    /// T2b passive head + grouping/empty bundle dispatch.  Split out
    /// of `tag_specific_html_prototype` to keep each function under
    /// the 100-line cap; the two halves share the same linear-`if`
    /// structure and could be refactored into a `[(name, slot)]` table
    /// later (deferred to the bench-driven O(1) optimisation slot in
    /// the master roadmap §H-7 — that work touches **all** dispatch
    /// arms, not just the T2b half).
    fn tag_specific_t2b_prototype(&self, entity: elidex_ecs::Entity) -> Option<ObjectId> {
        let host = self.host_data.as_deref()?;
        // Head bundle.
        if host.tag_matches_ascii_case(entity, "html") {
            return self.html_html_prototype;
        }
        if host.tag_matches_ascii_case(entity, "head") {
            return self.html_head_prototype;
        }
        if host.tag_matches_ascii_case(entity, "body") {
            return self.html_body_prototype;
        }
        if host.tag_matches_ascii_case(entity, "title") {
            return self.html_title_prototype;
        }
        if host.tag_matches_ascii_case(entity, "base") {
            return self.html_base_prototype;
        }
        if host.tag_matches_ascii_case(entity, "meta") {
            return self.html_meta_prototype;
        }
        if host.tag_matches_ascii_case(entity, "style") {
            return self.html_style_prototype;
        }
        // Grouping/empty bundle.
        if host.tag_matches_ascii_case(entity, "div") {
            return self.html_div_prototype;
        }
        if host.tag_matches_ascii_case(entity, "span") {
            return self.html_span_prototype;
        }
        if host.tag_matches_ascii_case(entity, "br") {
            return self.html_br_prototype;
        }
        if host.tag_matches_ascii_case(entity, "hr") {
            return self.html_hr_prototype;
        }
        if host.tag_matches_ascii_case(entity, "pre") {
            return self.html_pre_prototype;
        }
        if host.tag_matches_ascii_case(entity, "p") {
            return self.html_p_prototype;
        }
        // h1-h6 share HTMLHeadingElement.prototype; six explicit
        // arms keep each `tag_matches_ascii_case` call monomorphic.
        if host.tag_matches_ascii_case(entity, "h1")
            || host.tag_matches_ascii_case(entity, "h2")
            || host.tag_matches_ascii_case(entity, "h3")
            || host.tag_matches_ascii_case(entity, "h4")
            || host.tag_matches_ascii_case(entity, "h5")
            || host.tag_matches_ascii_case(entity, "h6")
        {
            return self.html_heading_prototype;
        }
        // blockquote + q share HTMLQuoteElement.prototype.
        if host.tag_matches_ascii_case(entity, "blockquote")
            || host.tag_matches_ascii_case(entity, "q")
        {
            return self.html_quote_prototype;
        }
        if host.tag_matches_ascii_case(entity, "ol") {
            return self.html_olist_prototype;
        }
        if host.tag_matches_ascii_case(entity, "ul") {
            return self.html_ulist_prototype;
        }
        if host.tag_matches_ascii_case(entity, "li") {
            return self.html_li_prototype;
        }
        if host.tag_matches_ascii_case(entity, "dl") {
            return self.html_dlist_prototype;
        }
        if host.tag_matches_ascii_case(entity, "menu") {
            return self.html_menu_prototype;
        }
        if host.tag_matches_ascii_case(entity, "map") {
            return self.html_map_prototype;
        }
        if host.tag_matches_ascii_case(entity, "picture") {
            return self.html_picture_prototype;
        }
        if host.tag_matches_ascii_case(entity, "data") {
            return self.html_data_prototype;
        }
        if host.tag_matches_ascii_case(entity, "time") {
            return self.html_time_prototype;
        }
        // Pass `host` through so the T2c chain doesn't re-resolve
        // `host_data.as_deref()?` — the parent call already owns a
        // borrow of the same `HostData`.
        self.tag_specific_t2c_prototype(host, entity)
    }

    /// T2c HTMLTable family bundle dispatch (slot
    /// `#11-tags-T2c-table`).  Six prototypes routed across ten
    /// dispatch arms — `<thead>`/`<tbody>`/`<tfoot>` share section,
    /// `<td>`/`<th>` share cell, `<col>`/`<colgroup>` share col.
    /// Split out of `tag_specific_html_prototype` to keep each
    /// function under the 100-line cap; the same linear-`if`
    /// structure as the T2b helper.  Takes `host` by reference so
    /// the dispatch chain doesn't redundantly re-resolve
    /// `host_data.as_deref()` (caller already holds the borrow).
    fn tag_specific_t2c_prototype(
        &self,
        host: &super::super::host_data::HostData,
        entity: elidex_ecs::Entity,
    ) -> Option<ObjectId> {
        if host.tag_matches_ascii_case(entity, "table") {
            return self.html_table_prototype;
        }
        if host.tag_matches_ascii_case(entity, "thead")
            || host.tag_matches_ascii_case(entity, "tbody")
            || host.tag_matches_ascii_case(entity, "tfoot")
        {
            return self.html_table_section_prototype;
        }
        if host.tag_matches_ascii_case(entity, "tr") {
            return self.html_table_row_prototype;
        }
        if host.tag_matches_ascii_case(entity, "td") || host.tag_matches_ascii_case(entity, "th") {
            return self.html_table_cell_prototype;
        }
        if host.tag_matches_ascii_case(entity, "caption") {
            return self.html_table_caption_prototype;
        }
        if host.tag_matches_ascii_case(entity, "col")
            || host.tag_matches_ascii_case(entity, "colgroup")
        {
            return self.html_table_col_prototype;
        }
        self.tag_specific_t2d_prototype(host, entity)
    }

    /// T2d interactive bundle dispatch (slot
    /// `#11-tags-T2d-interactive`).  7 prototypes routed across 7
    /// dispatch arms — no shared prototypes (each tag has a distinct
    /// IDL surface).  Same chain shape as the T2b/T2c helpers; takes
    /// `host` by reference so the dispatch chain doesn't redundantly
    /// re-resolve `host_data.as_deref()`.
    fn tag_specific_t2d_prototype(
        &self,
        host: &super::super::host_data::HostData,
        entity: elidex_ecs::Entity,
    ) -> Option<ObjectId> {
        if host.tag_matches_ascii_case(entity, "dialog") {
            return self.html_dialog_prototype;
        }
        if host.tag_matches_ascii_case(entity, "details") {
            return self.html_details_prototype;
        }
        if host.tag_matches_ascii_case(entity, "template") {
            return self.html_template_prototype;
        }
        if host.tag_matches_ascii_case(entity, "datalist") {
            return self.html_datalist_prototype;
        }
        if host.tag_matches_ascii_case(entity, "output") {
            return self.html_output_prototype;
        }
        if host.tag_matches_ascii_case(entity, "progress") {
            return self.html_progress_prototype;
        }
        if host.tag_matches_ascii_case(entity, "meter") {
            return self.html_meter_prototype;
        }
        if host.tag_matches_ascii_case(entity, "slot") {
            return self.html_slot_prototype;
        }
        None
    }
}
