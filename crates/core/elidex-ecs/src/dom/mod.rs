//! ECS DOM wrapper providing tree-manipulation API.
//!
//! # Tree invariants
//!
//! The DOM tree maintained by [`EcsDom`] guarantees:
//!
//! - **No cycles**: A node cannot be an ancestor of itself. All mutation
//!   methods (`append_child`, `insert_before`, `replace_child`) perform an
//!   ancestor walk to reject operations that would create cycles.
//! - **Consistent sibling links**: `prev_sibling` / `next_sibling` form a
//!   doubly-linked list among children of the same parent.
//! - **Parent <-> child consistency**: A child's `parent` field always matches
//!   the parent whose `first_child` / `last_child` chain reaches it.
//! - **Destroyed entity safety**: Operations on entities that have been
//!   removed from the world via `destroy_entity` return `false` and never
//!   mutate the tree.

pub(crate) mod equality;
mod mutation_event;
pub mod shadow;
mod tree;
mod tree_clone;

pub use mutation_event::{MutationDispatcher, MutationEvent};

use crate::components::{
    AssociatedDocument, AttrData, Attributes, CommentData, DocTypeData, DocumentBaseUrl,
    IframeData, InlineStyle, Namespace, NodeKind, ShadowRoot, TagType, TextContent, TreeRelation,
};
use hecs::{Entity, World};

/// Maximum ancestor walk depth before assuming tree corruption.
///
/// Also used by `elidex-css` selector matching to cap descendant walks.
pub const MAX_ANCESTOR_DEPTH: usize = 10_000;

/// ECS-backed DOM storage.
///
/// Each DOM node is an `Entity` in the `hecs::World`, with component data
/// describing its type, attributes, tree relationships, and content.
///
/// See the module-level documentation for tree invariant guarantees.
pub struct EcsDom {
    world: World,
    /// Cached document root entity, set by [`create_document_root()`](Self::create_document_root).
    document_root: Option<Entity>,
    /// Optional mutation event dispatcher.  Fires synchronously from
    /// `append_child` / `insert_before` / `remove_child` /
    /// `replace_child` / `destroy_entity` / `set_text_data` /
    /// `replace_text_data` / `set_attribute` / `attr_remove` /
    /// `fire_split_text` / `fire_normalize_merge`.  Single dispatcher
    /// per [`Self`] — multi-consumer composition is handled by a
    /// typed composer (`elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`) impl'ing
    /// [`MutationDispatcher`] with typed consumer fields.
    dispatcher: Option<Box<dyn MutationDispatcher + Send + Sync>>,
    /// Re-entry detection counter — incremented at the start of every
    /// [`Self::dispatch_event`] invocation, decremented in
    /// [`RestoreDispatcher::drop`].  Used by [`Self::dispatch_event`]
    /// to fire a `debug_assert!` when a mutation primitive is called
    /// recursively from within a consumer's `dispatch` body (violates
    /// the re-entry contract on [`MutationDispatcher::dispatch`]).
    dispatch_depth: u8,
    /// Standalone fragment tree — layout's output for N:M-fragmented content
    /// (§15.4.1), a sibling structure to `world` rather than a per-entity
    /// component. Populated by layout (multicol box fragments, Z-1a), read by
    /// render. Cleared + rebuilt each layout pass. See
    /// [`crate::FragmentTree`].
    fragment_tree: crate::FragmentTree,
    /// When set, [`rev_version`](Self::rev_version) is a no-op. Scoped on by
    /// [`despawn_subtree`](Self::despawn_subtree) so the per-node version bump
    /// (which walks all ancestors, O(depth)) does not turn a complete teardown
    /// into O(n²): every bump during the walk targets a node inside the doomed
    /// subtree, so the propagation is wasted. The single live-tree effect — a
    /// version bump on the root's *external* parent — is applied once after the
    /// walk.
    version_propagation_suppressed: bool,
}

/// Panic-safe Drop guard for [`EcsDom::dispatch_event`]: restores the
/// taken-out dispatcher back into `self.dispatcher` AND decrements the
/// re-entry depth counter on scope exit, even if the dispatch callback
/// panics.
///
/// # Safety
///
/// `target_ptr` and `depth_ptr` alias `EcsDom::dispatcher` /
/// `EcsDom::dispatch_depth` respectively.  Both pointers are derived
/// from a `&mut EcsDom` at the call site; the `RestoreDispatcher`
/// value is held in the SAME stack frame as the `&mut self` borrow,
/// so no other live borrow of either field can exist for the
/// duration of `guard`.  The raw pointers are dormant during the
/// re-borrowed `&mut *self` callback call (Rust's borrow stack
/// model allows raw pointers to alias actively-borrowed data so long
/// as they are not dereferenced).  Dereference happens only in
/// `Drop`, after the callback returns / unwinds and the `&mut self`
/// borrow has expired.
struct RestoreDispatcher {
    target_ptr: *mut Option<Box<dyn MutationDispatcher + Send + Sync>>,
    depth_ptr: *mut u8,
    pending: Option<Box<dyn MutationDispatcher + Send + Sync>>,
}

impl Drop for RestoreDispatcher {
    fn drop(&mut self) {
        #[allow(unsafe_code)]
        unsafe {
            *self.target_ptr = self.pending.take();
            *self.depth_ptr = (*self.depth_ptr).saturating_sub(1);
        }
    }
}

impl EcsDom {
    /// Create a new, empty DOM.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            document_root: None,
            dispatcher: None,
            dispatch_depth: 0,
            fragment_tree: crate::FragmentTree::default(),
            version_propagation_suppressed: false,
        }
    }

    // ---- Fragment tree (standalone layout-output structure, §15.4.1) ----

    /// Read access to the standalone [`FragmentTree`](crate::FragmentTree)
    /// (render's consume entry; layout's read-back).
    #[must_use]
    pub fn fragment_tree(&self) -> &crate::FragmentTree {
        &self.fragment_tree
    }

    /// Mutable access to the standalone [`FragmentTree`](crate::FragmentTree)
    /// (layout populates box fragments here).
    pub fn fragment_tree_mut(&mut self) -> &mut crate::FragmentTree {
        &mut self.fragment_tree
    }

    // ---- Mutation event dispatcher ----

    /// Install the (single) mutation event dispatcher.  Returns the
    /// previously-installed dispatcher (if any) for replace-then-
    /// retrieve patterns.  Production caller: `Vm::bind` installs a
    /// `ConsumerDispatcher` composing the mutation consumers.
    pub fn set_mutation_dispatcher(
        &mut self,
        dispatcher: Box<dyn MutationDispatcher + Send + Sync>,
    ) -> Option<Box<dyn MutationDispatcher + Send + Sync>> {
        self.dispatcher.replace(dispatcher)
    }

    /// Drop the installed dispatcher, if any.
    pub fn clear_mutation_dispatcher(&mut self) {
        self.dispatcher = None;
    }

    /// Take the installed dispatcher out without dropping it.
    /// Returns `None` if none was installed.
    pub fn take_mutation_dispatcher(
        &mut self,
    ) -> Option<Box<dyn MutationDispatcher + Send + Sync>> {
        self.dispatcher.take()
    }

    /// Dispatch one [`MutationEvent`] to the installed dispatcher (if
    /// any), using a take-and-restore borrow pattern so the dispatch
    /// callback can receive `&mut EcsDom`.  Panic-safe via the inline
    /// [`RestoreDispatcher`] `Drop` guard — both the dispatcher slot
    /// AND the re-entry depth counter are restored even if the
    /// callback unwinds.
    ///
    /// **Re-entry contract**: the dispatcher's `dispatch` impl MUST NOT
    /// invoke mutation primitives on the same `EcsDom`.  A
    /// `debug_assert!` on the `dispatch_depth` counter fires in debug
    /// builds when this is violated; release builds silently no-op
    /// (nested dispatch sees an empty dispatcher slot and skips).
    fn dispatch_event(&mut self, event: &MutationEvent<'_>) {
        debug_assert_eq!(
            self.dispatch_depth, 0,
            "EcsDom mutation primitive called from inside \
             MutationDispatcher::dispatch — violates re-entry contract \
             (see `MutationDispatcher::dispatch` docstring).  Queue the \
             work via per-consumer deferred state instead."
        );
        if self.dispatcher.is_none() {
            return;
        }
        let target_ptr: *mut Option<Box<dyn MutationDispatcher + Send + Sync>> =
            &raw mut self.dispatcher;
        let depth_ptr: *mut u8 = &raw mut self.dispatch_depth;
        self.dispatch_depth = self.dispatch_depth.saturating_add(1);
        let mut guard = RestoreDispatcher {
            target_ptr,
            depth_ptr,
            pending: self.dispatcher.take(),
        };
        if let Some(d) = guard.pending.as_mut() {
            d.dispatch(event, self);
        }
        drop(guard);
    }

    /// HTML tag-name predicate: returns `true` iff `entity` is an
    /// `<base>` element (HTML §4.2.3).  Used by event consumers (e.g.
    /// `BaseUrlMaintainer` in `elidex-dom-api`) to filter
    /// `MutationEvent::AttributeChange` events to `<base>.href`
    /// writes only.
    ///
    /// HTML-spec knowledge co-located with `shadow.rs` `VALID_SHADOW_HOST_TAGS`
    /// (lines 65-84) + `is_valid_shadow_host` (lines 122-125) tag-whitelist
    /// precedent per layering mandate (small HTML predicates in
    /// elidex-ecs are OK; large algorithms hoist to elidex-dom-api).
    #[must_use]
    pub fn is_base_element(&self, entity: Entity) -> bool {
        // WHATWG HTML §4.2.3 restricts `<base>` to the HTML namespace, so
        // a foreign `<base>` (SVG / MathML) is not the document base.
        self.world
            .get::<&TagType>(entity)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("base"))
            && self.is_html_namespace(entity)
    }

    /// HTML tag-name predicate: returns `true` iff `entity` is a
    /// `<template>` element (HTML §4.12.3).  Tree walkers that
    /// implement spec algorithms requiring the "template contents"
    /// carve-out (HTML §2.4.3 "first base element in the document" —
    /// template contents form a separate document) use this to skip
    /// the `<template>` element's children.
    ///
    /// Tag-string compare matches the [`Self::is_base_element`]
    /// precedent.  The `TemplateContent` component is reserved for
    /// future plug-in use (e.g. signalling "this element owns a
    /// detached contents fragment"); a content-attach pass would
    /// piggyback on this predicate.
    #[must_use]
    pub fn is_template_element(&self, entity: Entity) -> bool {
        self.world
            .get::<&TagType>(entity)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("template"))
    }

    /// Fire [`MutationEvent::SplitText`] from a caller in another crate.
    /// WHATWG DOM §4.11 Interface Text "split a Text node" step 7.
    ///
    /// **Caller ordering contract**: MUST be invoked AFTER `new_node`
    /// has been inserted as a sibling of `node` but BEFORE `node`'s
    /// text is truncated via [`Self::set_text_data`] (which would
    /// otherwise fire `TextChange` and clamp Range boundaries on
    /// `node` to the truncated length, destroying offsets needed for
    /// migration).
    pub fn fire_split_text(
        &mut self,
        node: Entity,
        new_node: Entity,
        offset_utf16: usize,
        parent: Option<Entity>,
        node_index: Option<usize>,
    ) {
        let event = MutationEvent::SplitText {
            node,
            new_node,
            offset_utf16,
            parent,
            node_index,
        };
        self.dispatch_event(&event);
    }

    /// Fire [`MutationEvent::NormalizeMerge`] from a caller in another
    /// crate.  WHATWG DOM §4.4 Interface Node `normalize()` step 6.4.
    ///
    /// **Caller ordering contract**: MUST be invoked AFTER `prev` has
    /// absorbed `merged_child`'s data but BEFORE `merged_child` is
    /// detached.  Firing before detach lets consumers migrate
    /// boundaries on `merged_child` to `(prev, prev_old_len + off)`
    /// instead of collapsing via the subsequent
    /// [`MutationEvent::Remove`].
    pub fn fire_normalize_merge(
        &mut self,
        merged_child: Entity,
        prev: Entity,
        prev_old_len_utf16: usize,
        parent: Option<Entity>,
        merged_child_index: Option<usize>,
    ) {
        let event = MutationEvent::NormalizeMerge {
            merged_child,
            prev,
            prev_old_len_utf16,
            parent,
            merged_child_index,
        };
        self.dispatch_event(&event);
    }

    /// Replace the `TextContent` of an entity. Returns the new UTF-16 length
    /// on success, or `None` if the entity has no `TextContent` component.
    ///
    /// On success, bumps [`Self::rev_version`] for `entity` (the canonical
    /// cache-invalidation step per the version-tracking docs above) and
    /// fires `after_text_change` on the mutation hook (if installed). This
    /// makes `set_text_data` self-contained: callers do not need to
    /// `rev_version` themselves after.
    ///
    /// This is the canonical write path for **Text / CData** mutations.
    /// `CharacterData` handlers in `elidex-dom-api` route `TextContent`
    /// updates through this method to ensure Range live-tracking hook fire
    /// consistency.
    ///
    /// Takes `&str` and uses [`str::clone_into`] so the existing
    /// `TextContent` buffer's capacity is reused — frequent CharacterData
    /// updates do not re-allocate.
    ///
    /// **NOT for Comment nodes** — Comment uses a separate `CommentData`
    /// component which is NOT covered by Range live-tracking spec (§5.5
    /// covers Text only, not Comment). Comment writes continue to use the
    /// existing `set_char_data` Comment branch unchanged.
    pub fn set_text_data(&mut self, entity: Entity, text: &str) -> Option<usize> {
        let new_utf16_len = {
            let mut tc = self.world.get::<&mut TextContent>(entity).ok()?;
            let len = text.encode_utf16().count();
            text.clone_into(&mut tc.0);
            len
        };
        self.rev_version(entity);
        let event = MutationEvent::TextChange {
            node: entity,
            new_utf16_len,
        };
        self.dispatch_event(&event);
        Some(new_utf16_len)
    }

    /// Primitive UTF-16 splice on a Text / CData entity's `TextContent`
    /// (WHATWG DOM §4.10 "replace data" steps 1-7 storage mutation,
    /// step 8-11 boundary adjustment is the hook consumer's
    /// responsibility). Returns the new UTF-16 length on success, or
    /// `None` if the entity has no `TextContent` component.
    ///
    /// **Bounds validation is the CALLER's responsibility** — this is
    /// the engine-level splice primitive that `CharacterData` handlers
    /// in `elidex-dom-api` (`appendData` / `insertData` / `deleteData`
    /// / `replaceData`) route through after raising `IndexSizeError`
    /// for `offset > utf16_len`. `count` IS clamped to `len - offset`
    /// here to match the spec's silent clamp ("if offset+count is
    /// greater than length, end at length", §11.2 step 6).
    ///
    /// Splitting through a surrogate pair (offset / end mid-pair) is
    /// **spec-valid** — UTF-16 offsets ignore character boundaries —
    /// and produces lone surrogates in the intermediate `Vec<u16>`.
    /// Rust's `String` storage cannot represent lone surrogates, so the
    /// result is rendered through `from_utf16_lossy` which substitutes
    /// `U+FFFD` for each unpaired half. This matches the lossy-not-panic
    /// contract pinned by `tests_character_data::*surrogate_pair*` and
    /// mirrors `elidex-dom-api::char_data::splice_utf16`.
    ///
    /// On success:
    /// - splices the UTF-16 view of `TextContent` in place,
    /// - bumps [`Self::rev_version`] (cache invalidation),
    /// - fires [`MutationEvent::ReplaceData`] with
    ///   `(entity, offset, count, replacement_utf16_len)`.
    ///
    /// **NOT for Comment nodes** (Comment uses `CommentData`, not
    /// covered by WHATWG §5.5 Range live-tracking).
    pub fn replace_text_data(
        &mut self,
        entity: Entity,
        offset_utf16: usize,
        count_utf16: usize,
        replacement: &str,
    ) -> Option<usize> {
        let replacement_units: Vec<u16> = replacement.encode_utf16().collect();
        let replacement_len = replacement_units.len();
        let (new_utf16_len, clamped_count) = {
            let mut tc = self.world.get::<&mut TextContent>(entity).ok()?;
            let units: Vec<u16> = tc.0.encode_utf16().collect();
            let len = units.len();
            debug_assert!(
                offset_utf16 <= len,
                "replace_text_data: offset {offset_utf16} exceeds UTF-16 length {len}; \
                 caller must validate via `offset > utf16_len(&data)` before invocation"
            );
            let end = offset_utf16.saturating_add(count_utf16).min(len);
            let clamped_count = end - offset_utf16;
            let mut out: Vec<u16> = Vec::with_capacity(len - clamped_count + replacement_len);
            out.extend_from_slice(&units[..offset_utf16]);
            out.extend_from_slice(&replacement_units);
            out.extend_from_slice(&units[end..]);
            let new_len = out.len();
            let spliced = String::from_utf16_lossy(&out);
            spliced.clone_into(&mut tc.0);
            (new_len, clamped_count)
        };
        self.rev_version(entity);
        // WHATWG DOM §4.10 step 6 clamps the live-range adjustment to
        // the actual spliced span (`end - offset`), not the caller's
        // possibly-overflowing `count_utf16`. Passing the unclamped
        // value would make `adjust_ranges_for_replace_data` treat
        // boundaries near the OLD end as inside the splice region
        // and collapse them to `offset` instead of shifting by
        // `new_data_len - clamped_count` — PR186 R3 #1 fix.
        let event = MutationEvent::ReplaceData {
            node: entity,
            offset_utf16,
            count_utf16: clamped_count,
            new_data_len_utf16: replacement_len,
        };
        self.dispatch_event(&event);
        Some(new_utf16_len)
    }

    /// Provides read-only access to the underlying `hecs::World`.
    ///
    /// Use this for queries and component reads. Tree mutations **must** go
    /// through [`EcsDom`] methods to preserve invariants.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns `true` if the entity exists in this DOM world.
    #[must_use]
    pub fn contains(&self, entity: Entity) -> bool {
        self.world.contains(entity)
    }

    /// Check if an entity is editable via `contenteditable`, considering ancestor inheritance.
    ///
    /// Per HTML 6.6.1: `contenteditable` is inherited. An element with
    /// `contenteditable="true"` (or empty string) makes itself and its descendants
    /// editable. `contenteditable="false"` overrides the inherited state.
    #[must_use]
    pub fn is_contenteditable(&self, entity: Entity) -> bool {
        let mut current = Some(entity);
        for _ in 0..MAX_ANCESTOR_DEPTH {
            let Some(e) = current else { break };
            let attr = self
                .world
                .get::<&Attributes>(e)
                .ok()
                .and_then(|a| a.get("contenteditable").map(String::from));
            match attr.as_deref() {
                Some("true" | "") => return true,
                Some("false") => return false,
                _ => {}
            }
            current = self.get_parent(e);
        }
        false
    }

    /// Provides mutable access to the underlying `hecs::World`.
    ///
    /// **Warning:** Tree mutations (parent/child/sibling links) **must** go
    /// through [`EcsDom`] methods to preserve invariants. Use this only for
    /// adding or modifying non-tree components (e.g., [`crate::InlineStyle`]).
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Create an element node with the given tag and attributes.
    ///
    /// Shim over [`create_element_with_owner`](Self::create_element_with_owner)
    /// with `owner = None`.  New callers that know which Document the
    /// node belongs to should prefer the `_with_owner` variant so that
    /// [`owner_document`](Self::owner_document) can report the correct
    /// Document even for detached nodes (WHATWG §4.4 "node document").
    pub fn create_element(&mut self, tag: impl Into<String>, attrs: Attributes) -> Entity {
        self.create_element_with_owner(tag, attrs, None)
    }

    /// Create an element node with an explicit owner `Document` entity.
    ///
    /// `owner = Some(doc)` attaches an [`AssociatedDocument`] component so
    /// that [`owner_document`](Self::owner_document) returns `doc` before
    /// the node is inserted into any tree.  `owner = None` mirrors legacy
    /// [`create_element`](Self::create_element) behaviour.
    pub fn create_element_with_owner(
        &mut self,
        tag: impl Into<String>,
        attrs: Attributes,
        owner: Option<Entity>,
    ) -> Entity {
        let owner = self.validate_owner_document(owner);
        let entity = self.world.spawn((
            TagType(tag.into()),
            attrs,
            TreeRelation::default(),
            NodeKind::Element,
        ));
        if let Some(doc) = owner {
            let _ = self.world.insert_one(entity, AssociatedDocument(doc));
        }
        entity
    }

    /// Create a foreign-namespace element node (SVG / MathML).
    ///
    /// Like [`create_element_with_owner`](Self::create_element_with_owner)
    /// but tags the element with an explicit [`Namespace`].  The
    /// [`Namespace`] component is attached **only** for non-HTML
    /// namespaces — an absent component means [`Namespace::Html`] by
    /// construction — so HTML elements created via the plain
    /// [`create_element`](Self::create_element) stay component-free and
    /// unchanged.  Used by the strict HTML parser's foreign-content path
    /// (WHATWG HTML §13.2.6.5).
    pub fn create_element_ns(
        &mut self,
        tag: impl Into<String>,
        namespace: Namespace,
        attrs: Attributes,
        owner: Option<Entity>,
    ) -> Entity {
        let entity = self.create_element_with_owner(tag, attrs, owner);
        if namespace != Namespace::Html {
            let _ = self.world.insert_one(entity, namespace);
        }
        entity
    }

    /// The [`Namespace`] of an element entity.
    ///
    /// Returns the element's explicit [`Namespace`] component if present,
    /// otherwise [`Namespace::Html`] — the default-by-absence sentinel
    /// (most elements are HTML, so the component marks only the foreign
    /// exception).  For non-element entities the value is meaningless;
    /// callers gate on [`is_element`](Self::is_element) first, as
    /// [`is_html_namespace`](Self::is_html_namespace) does.
    #[must_use]
    pub fn namespace_of(&self, entity: Entity) -> Namespace {
        self.world
            .get::<&Namespace>(entity)
            .map_or(Namespace::Html, |n| *n)
    }

    /// Create a document root entity (no tag, only tree relations).
    ///
    /// The document root serves as the parent of the `<html>` element.
    /// The entity is cached for fast retrieval via [`document_root()`](Self::document_root).
    ///
    /// Eagerly attaches [`DocumentBaseUrl`] initialised to
    /// [`about_blank_url`](crate::about_blank_url) pending the
    /// `#11-document-url-real-navigation` slot landing.
    /// `elidex_dom_api::BaseUrlMaintainer` mutates it as `<base>`
    /// elements enter / leave the doc tree.
    pub fn create_document_root(&mut self) -> Entity {
        let entity = self.create_document_node();
        self.document_root = Some(entity);
        entity
    }

    /// Spawn a bare `Document` node ([`NodeKind::Document`]) **without**
    /// registering it as the dom's cached document root.
    ///
    /// For transient / throwaway documents that must not clobber the
    /// persistent [`document_root()`](Self::document_root) cache — e.g. the
    /// WHATWG HTML §13.4 fragment parsing algorithm's throwaway document
    /// (step 2), which exists only to own the fragment's nodes during the
    /// parse and is despawned afterwards. Sharing the spawn with
    /// [`create_document_root`](Self::create_document_root) keeps the
    /// `Document` node shape (`TreeRelation` + base URL) defined in one place.
    pub fn create_document_node(&mut self) -> Entity {
        self.world.spawn((
            TreeRelation::default(),
            NodeKind::Document,
            DocumentBaseUrl(crate::about_blank_url()),
        ))
    }

    /// Returns the document root entity created by [`create_document_root()`](Self::create_document_root).
    ///
    /// Returns `None` if no document root has been created yet.
    #[must_use]
    pub fn document_root(&self) -> Option<Entity> {
        self.document_root
    }

    /// Create a Window root entity (WHATWG HTML §7.2).
    ///
    /// The Window entity is **not** a Node and does **not** participate in
    /// the DOM tree: it carries only the [`NodeKind::Window`] component and
    /// has no `TreeRelation`. It exists purely as a stable ECS address so
    /// that the scripting layer can attach `EventListeners` / other
    /// window-scoped component data to a single entity per `Vm`.
    pub fn create_window_root(&mut self) -> Entity {
        self.world.spawn((NodeKind::Window,))
    }

    /// Locate the single [`NodeKind::Window`] entity, if one exists.
    ///
    /// One Window entity is created per `Vm` (via [`Self::create_window_root`]
    /// at bind time). Engine-independent consumers (e.g. the inline
    /// event-handler detector routing `<body>` WindowEventHandlers content
    /// attributes to the Window per WHATWG HTML §8.1.8.2) need it without a
    /// VM-side handle. Linear scan — only invoked off the hot path (a body
    /// element gaining a WindowEventHandlers `on*` attribute).
    #[must_use]
    pub fn window_entity(&self) -> Option<Entity> {
        self.world
            .query::<(Entity, &NodeKind)>()
            .iter()
            .find(|(_, kind)| matches!(**kind, NodeKind::Window))
            .map(|(entity, _)| entity)
    }

    /// Create a worker global scope root entity (WHATWG HTML §10.2.1.1).
    ///
    /// The worker realm's analog of [`create_window_root`](Self::create_window_root):
    /// the entity carries only the [`NodeKind::Worker`] component, has no
    /// `TreeRelation`, and exists purely as a stable ECS address for the
    /// worker scope's `EventListeners`. One per worker `Vm`.
    pub fn create_worker_global_scope_root(&mut self) -> Entity {
        self.world.spawn((NodeKind::Worker,))
    }

    /// Locate the worker-global-scope entity, if one exists **unambiguously**.
    ///
    /// **Worker-VM DOMs only**: a worker `Vm`'s `EcsDom` holds exactly one
    /// [`NodeKind::Worker`] entity (the worker scope, created at bind time), so
    /// the lookup is unambiguous and returns it. A *main*-VM DOM may hold many
    /// `NodeKind::Worker` entities (one per main-side `Worker` object) — this
    /// helper is not meant for that DOM, so rather than return an arbitrary one
    /// it returns `None` when zero **or more than one** `Worker` entity exists.
    /// Misuse on a main DOM therefore fails safe (no silent misrouting) instead
    /// of resolving to a random `Worker` handle. Linear scan (stops after the
    /// second hit) — only invoked off the hot path (worker bind / WEH routing).
    #[must_use]
    pub fn worker_scope_entity(&self) -> Option<Entity> {
        let mut query = self.world.query::<(Entity, &NodeKind)>();
        let mut workers = query
            .iter()
            .filter(|(_, kind)| matches!(**kind, NodeKind::Worker))
            .map(|(entity, _)| entity);
        match (workers.next(), workers.next()) {
            (Some(entity), None) => Some(entity),
            _ => None,
        }
    }

    /// Create a text node.
    ///
    /// Shim over [`create_text_with_owner`](Self::create_text_with_owner)
    /// with `owner = None`.
    pub fn create_text(&mut self, text: impl Into<String>) -> Entity {
        self.create_text_with_owner(text, None)
    }

    /// Create a text node with an explicit owner `Document` entity.
    pub fn create_text_with_owner(
        &mut self,
        text: impl Into<String>,
        owner: Option<Entity>,
    ) -> Entity {
        let owner = self.validate_owner_document(owner);
        let entity = self.world.spawn((
            TextContent(text.into()),
            TreeRelation::default(),
            NodeKind::Text,
        ));
        if let Some(doc) = owner {
            let _ = self.world.insert_one(entity, AssociatedDocument(doc));
        }
        entity
    }

    /// Create a document fragment node (WHATWG DOM 4.5).
    ///
    /// Shim over
    /// [`create_document_fragment_with_owner`](Self::create_document_fragment_with_owner)
    /// with `owner = None`.
    pub fn create_document_fragment(&mut self) -> Entity {
        self.create_document_fragment_with_owner(None)
    }

    /// Create a document fragment node with an explicit owner
    /// `Document` entity.
    pub fn create_document_fragment_with_owner(&mut self, owner: Option<Entity>) -> Entity {
        let owner = self.validate_owner_document(owner);
        let entity = self
            .world
            .spawn((TreeRelation::default(), NodeKind::DocumentFragment));
        if let Some(doc) = owner {
            let _ = self.world.insert_one(entity, AssociatedDocument(doc));
        }
        entity
    }

    /// Create a comment node.
    ///
    /// Shim over [`create_comment_with_owner`](Self::create_comment_with_owner)
    /// with `owner = None`.
    pub fn create_comment(&mut self, data: impl Into<String>) -> Entity {
        self.create_comment_with_owner(data, None)
    }

    /// Create a comment node with an explicit owner `Document` entity.
    pub fn create_comment_with_owner(
        &mut self,
        data: impl Into<String>,
        owner: Option<Entity>,
    ) -> Entity {
        let owner = self.validate_owner_document(owner);
        let entity = self.world.spawn((
            CommentData(data.into()),
            TreeRelation::default(),
            NodeKind::Comment,
        ));
        if let Some(doc) = owner {
            let _ = self.world.insert_one(entity, AssociatedDocument(doc));
        }
        entity
    }

    /// Create a document type node.
    pub fn create_document_type(
        &mut self,
        name: impl Into<String>,
        public_id: impl Into<String>,
        system_id: impl Into<String>,
    ) -> Entity {
        self.world.spawn((
            DocTypeData {
                name: name.into(),
                public_id: public_id.into(),
                system_id: system_id.into(),
            },
            TreeRelation::default(),
            NodeKind::DocumentType,
        ))
    }

    /// Create an attribute node (WHATWG DOM 4.9).
    pub fn create_attribute(&mut self, local_name: impl Into<String>) -> Entity {
        self.world.spawn((
            AttrData {
                local_name: local_name.into(),
                value: String::new(),
                owner_element: None,
            },
            NodeKind::Attribute,
        ))
    }

    // ---- Version tracking ----

    /// Bump the `inclusive_descendants_version` on `entity` and propagate to all ancestors.
    ///
    /// This is the Servo-style version cache invalidation mechanism: any tree
    /// mutation on `entity` (child add/remove, text content change, attribute change
    /// via `elidex-dom-api` handlers) sets the same new version counter on `entity`
    /// and all its ancestors up to the root.
    ///
    /// The new version is computed as `max(entity_version, doc_root_version) + 1`,
    /// ensuring a globally monotonic value across the entire tree.
    pub fn rev_version(&mut self, entity: Entity) {
        // Suppressed during a complete-subtree teardown (`despawn_subtree`),
        // where every bump would target a doomed node and walk its ancestors.
        if self.version_propagation_suppressed {
            return;
        }
        // Compute a single new version: max of entity and doc_root versions + 1.
        let entity_ver = self.read_rel(entity, |rel| rel.inclusive_descendants_version);
        let doc_root_ver = self.document_root.map_or(0, |dr| {
            self.read_rel(dr, |rel| rel.inclusive_descendants_version)
        });
        let new_version = entity_ver.max(doc_root_ver).wrapping_add(1);

        // Set the same version on entity and all ancestors.
        // When the parent chain ends at a ShadowRoot (no parent), jump to the
        // shadow host and continue propagating, so that LiveCollections rooted
        // at the host see the version change.
        let mut current = Some(entity);
        let mut depth = 0;
        while let Some(e) = current {
            self.update_rel(e, |rel| {
                rel.inclusive_descendants_version = new_version;
            });
            let parent = self
                .world
                .get::<&TreeRelation>(e)
                .ok()
                .and_then(|rel| rel.parent);
            current = if parent.is_some() {
                parent
            } else {
                // No parent -- if this is a ShadowRoot, jump to host.
                self.world.get::<&ShadowRoot>(e).ok().map(|sr| sr.host)
            };
            depth += 1;
            if depth > MAX_ANCESTOR_DEPTH {
                break;
            }
        }
    }

    /// Returns the `inclusive_descendants_version` for an entity.
    ///
    /// Returns 0 if the entity does not exist or has no `TreeRelation`.
    #[must_use]
    pub fn inclusive_descendants_version(&self, entity: Entity) -> u64 {
        self.read_rel(entity, |rel| rel.inclusive_descendants_version)
    }

    /// Returns `true` if the entity is an element node.
    ///
    /// Checks `NodeKind` first, falls back to `TagType` presence for
    /// backwards compatibility with entities created before `NodeKind` was added.
    #[must_use]
    pub fn is_element(&self, entity: Entity) -> bool {
        if let Ok(kind) = self.world.get::<&NodeKind>(entity) {
            return *kind == NodeKind::Element;
        }
        self.world.get::<&TagType>(entity).is_ok()
    }

    /// Returns the `NodeKind` of an entity, if it has one.
    #[must_use]
    pub fn node_kind(&self, entity: Entity) -> Option<NodeKind> {
        self.world.get::<&NodeKind>(entity).ok().map(|k| *k)
    }

    /// Whether `entity` is an element in the HTML namespace.
    ///
    /// `true` iff `entity` is an element AND its [`namespace_of`] is
    /// [`Namespace::Html`] (the default-by-absence case — most elements
    /// carry no [`Namespace`] component). Foreign (SVG / MathML) elements
    /// created via [`create_element_ns`](Self::create_element_ns) return
    /// `false`; non-elements return `false` (defensive).
    ///
    /// Used by `Range::create_contextual_fragment` in `elidex-dom-api`
    /// (HTML §8.5.7 `createContextualFragment()` step 6) to gate the
    /// `<html>`-as-context override (only HTML-namespace `<html>` is
    /// rewritten to `<body>` for parser-scope selection; SVG / MathML
    /// `<html>` must stay as-is), and by the `<base>` / `<datalist>`
    /// element predicates which are HTML-namespace-restricted (HTML
    /// §4.2.3 / §4.10.8).
    ///
    /// [`namespace_of`]: Self::namespace_of
    #[must_use]
    pub fn is_html_namespace(&self, entity: Entity) -> bool {
        self.is_element(entity) && self.namespace_of(entity) == Namespace::Html
    }

    /// Effective `NodeKind` — returns the explicit component when
    /// present, otherwise infers from payload components for legacy
    /// entities that predate the `NodeKind` component (same rules as
    /// [`clone_node_shallow`](Self::clone_node_shallow) and
    /// `HostData::prototype_kind_for`): `TagType` ⇒ `Element`,
    /// `TextContent` ⇒ `Text`, `CommentData` ⇒ `Comment`,
    /// `DocTypeData` ⇒ `DocumentType`.  Returns `None` only when no
    /// kind component and no DOM payload is present.
    ///
    /// Use this in any code path that has to treat legacy entities
    /// as real DOM nodes — e.g. `splitText` brand checks,
    /// `isEqualNode` equality, variadic argument normalisation.
    #[must_use]
    pub fn node_kind_inferred(&self, entity: Entity) -> Option<NodeKind> {
        if let Some(kind) = self.node_kind(entity) {
            return Some(kind);
        }
        if self.world.get::<&TagType>(entity).is_ok() {
            return Some(NodeKind::Element);
        }
        if self.world.get::<&TextContent>(entity).is_ok() {
            return Some(NodeKind::Text);
        }
        if self.world.get::<&CommentData>(entity).is_ok() {
            return Some(NodeKind::Comment);
        }
        if self.world.get::<&DocTypeData>(entity).is_ok() {
            return Some(NodeKind::DocumentType);
        }
        None
    }

    // ---- AssociatedDocument (WHATWG §4.4 "node document") ----

    /// Validate an incoming `owner` argument passed to
    /// `create_*_with_owner`.  Returns the entity unchanged when it
    /// still points at a live [`NodeKind::Document`]; returns `None`
    /// otherwise (destroyed / non-Document / never set).
    ///
    /// Write-time counterpart to the read-time validation in
    /// [`owner_document`](Self::owner_document): both layers together
    /// guarantee that an [`AssociatedDocument`] component in the
    /// world always points at a real Document as long as it persists.
    /// A `debug_assert!` fires when the caller passes a non-Document
    /// owner so misuse surfaces immediately in test builds while
    /// release builds keep the silent-skip safety net.
    fn validate_owner_document(&self, owner: Option<Entity>) -> Option<Entity> {
        let doc = owner?;
        if self.contains(doc) && matches!(self.node_kind(doc), Some(NodeKind::Document)) {
            Some(doc)
        } else {
            debug_assert!(
                false,
                "create_*_with_owner passed an owner that is not a live Document entity"
            );
            None
        }
    }

    /// Returns the [`AssociatedDocument`] component value for an
    /// entity, or `None` if absent.
    ///
    /// Low-level accessor — callers that need WHATWG-compliant
    /// `ownerDocument` semantics should use
    /// [`owner_document`](Self::owner_document), which layers tree-root
    /// fallback on top.
    #[must_use]
    pub fn get_associated_document(&self, entity: Entity) -> Option<Entity> {
        self.world
            .get::<&AssociatedDocument>(entity)
            .ok()
            .map(|a| a.0)
    }

    /// Attach or overwrite an entity's [`AssociatedDocument`] (WHATWG
    /// §4.4 "node document").
    ///
    /// Idempotent: inserts when absent, updates in place when present.
    /// Returns `false` if the entity has been destroyed.
    pub fn set_associated_document(&mut self, entity: Entity, doc: Entity) -> bool {
        if !self.contains(entity) {
            return false;
        }
        if let Ok(mut slot) = self.world.get::<&mut AssociatedDocument>(entity) {
            slot.0 = doc;
            return true;
        }
        self.world
            .insert_one(entity, AssociatedDocument(doc))
            .is_ok()
    }

    /// Resolve the owner `Document` entity for a node
    /// (WHATWG §4.4 `Node.ownerDocument`).
    ///
    /// Returns `None` when:
    /// - `entity` is itself a `Document` (per WHATWG, `Document.ownerDocument`
    ///   is `null`), **or**
    /// - no [`AssociatedDocument`] is set and the tree root is not a
    ///   Document (orphan node / detached fragment / Window).
    ///
    /// Otherwise, prefers the explicit [`AssociatedDocument`] component
    /// (set at node-creation time by `create_*_with_owner` and propagated
    /// through `clone_subtree`) and falls back to the tree-root walk so
    /// that legacy entities created without the component still resolve
    /// to the bound document when inserted into the main tree.
    #[must_use]
    pub fn owner_document(&self, entity: Entity) -> Option<Entity> {
        if !self.contains(entity) {
            return None;
        }
        if matches!(self.node_kind(entity), Some(NodeKind::Document)) {
            return None;
        }
        if let Some(doc) = self.get_associated_document(entity) {
            // Guard against a dangling pointer OR a wrongly-typed
            // component — callers expect an actual Document back.
            // A stale entity (destroyed / recycled as a non-Document)
            // must not leak through; fall through to the tree-root
            // walk in that case so `ownerDocument` never hands out a
            // ghost or off-kind receiver.
            if self.contains(doc) && matches!(self.node_kind(doc), Some(NodeKind::Document)) {
                return Some(doc);
            }
        }
        let root = self.find_tree_root(entity);
        if matches!(self.node_kind(root), Some(NodeKind::Document)) {
            return Some(root);
        }
        None
    }

    // ---- Attribute accessors ----

    /// Read attribute `name` on `entity`.
    ///
    /// Returns `None` when the value is not readable — covering the
    /// `Attributes` component absent / key not present cases AND any
    /// `World::get::<&Attributes>` failure (entity destroyed, hecs
    /// borrow conflict).  Callers cannot distinguish these from a
    /// genuinely-absent attribute; treat `None` as "no readable
    /// attribute" rather than "definitely no attribute".
    ///
    /// Allocates a fresh `String` for the present-value arm; prefer
    /// [`Self::with_attribute`] for borrow-only consumers (existence
    /// checks, equality comparisons, intern-on-Some) — that path
    /// keeps the value as `Option<&str>` and skips the `String::from`
    /// clone.
    #[must_use]
    pub fn get_attribute(&self, entity: Entity, name: &str) -> Option<String> {
        self.with_attribute(entity, name, |v| v.map(String::from))
    }

    /// Borrow attribute `name` on `entity` and project through `f`.
    ///
    /// `f` is called with `Some(value)` when the `Attributes`
    /// component is reachable and contains `name`, and `None`
    /// otherwise — covering not just absent-component / missing-key
    /// but every `World::get::<&Attributes>` failure (entity
    /// destroyed, borrow conflict).  Callers cannot distinguish
    /// these cases from `None`; treat it as "no readable attribute"
    /// rather than "definitely no attribute".  This is the
    /// zero-allocation sibling of [`Self::get_attribute`] —
    /// callers that only need to compare, parse, or hash the value
    /// can avoid the `String::from` clone the owned getter performs.
    /// Mirrors the closure-borrow `read_rel` pattern used internally
    /// for `TreeRelation` reads.
    ///
    /// The closure parameter is `for<'b> FnOnce(Option<&'b str>) -> R`
    /// so the borrowed `&str` cannot escape `f`'s scope: `hecs::World`
    /// supports interior-mutable borrows via `&World`, so leaking the
    /// `&str` past the internal `Ref<'_, Attributes>` guard could
    /// allow a later `&mut Attributes` borrow to alias it.
    pub fn with_attribute<R>(
        &self,
        entity: Entity,
        name: &str,
        f: impl for<'b> FnOnce(Option<&'b str>) -> R,
    ) -> R {
        match self.world.get::<&Attributes>(entity) {
            Ok(attrs) => f(attrs.get(name)),
            Err(_) => f(None),
        }
    }

    /// Returns `true` if `entity` has an `Attributes` component
    /// with `name` present.  Equivalent to
    /// `self.get_attribute(entity, name).is_some()` but skips the
    /// `String::from` clone.
    #[must_use]
    pub fn has_attribute(&self, entity: Entity, name: &str) -> bool {
        self.world
            .get::<&Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.contains(name))
    }

    /// Set attribute `name = value` on `entity`, inserting an
    /// `Attributes` component if one does not exist.
    ///
    /// On success, bumps [`rev_version`](Self::rev_version) so that
    /// live collections filtering on attribute state (e.g.
    /// `getElementsByClassName`, `getElementsByName`,
    /// `document.links`) invalidate any cached entity list at the
    /// next read.  Tag-only / topology-only filters (e.g.
    /// `getElementsByTagName`) over-invalidate harmlessly — the
    /// next read pays one walk and re-caches.  See the SP2 entity-
    /// list cache in `elidex-js::vm::host::dom_collection`.
    ///
    /// Returns `false` if the entity has been destroyed.
    pub fn set_attribute(&mut self, entity: Entity, name: &str, value: &str) -> bool {
        if !self.contains(entity) {
            return false;
        }
        // Engine-internal hardening (pre-D-31 `require_attrs_mut`
        // semantics): only Element entities carry `Attributes`.
        // Silently auto-attaching `Attributes` to Document / Text /
        // ShadowRoot / Comment entities would corrupt downstream
        // attribute readers; bail with `false` so caller sees the
        // mis-routed write the same way it sees a destroyed entity.
        if !matches!(self.node_kind(entity), Some(NodeKind::Element)) {
            return false;
        }
        // Single component lookup: capture old_value AND component
        // presence from one borrow; if absent, insert a fresh
        // Attributes default below.
        let (old_value, has_component) = match self.world.get::<&Attributes>(entity) {
            Ok(a) => (a.get(name).map(String::from), true),
            Err(_) => (None, false),
        };
        let did_set = if has_component {
            if let Ok(mut attrs) = self.world.get::<&mut Attributes>(entity) {
                attrs.set(name, value);
                true
            } else {
                false
            }
        } else {
            let mut attrs = Attributes::default();
            attrs.set(name, value);
            self.world.insert_one(entity, attrs).is_ok()
        };
        if did_set {
            self.reconcile_attribute_derived_components(entity, name);
            self.rev_version(entity);
            // Fire `MutationEvent::AttributeChange` per DOM §4.3.2 +
            // §4.3.3; same-value writes still fire because spec
            // requires same-value records be queued for
            // MutationObserver consumers.  Per-consumer suppression
            // (e.g. `BaseUrlMaintainer` idempotent bump) lives in
            // the dispatcher's handle, not here.
            let event = MutationEvent::AttributeChange {
                node: entity,
                name,
                old_value: old_value.as_deref(),
                new_value: Some(value),
            };
            self.dispatch_event(&event);
        }
        did_set
    }

    /// Remove attribute `name` from `entity` if present, then bump
    /// [`rev_version`](Self::rev_version) — both gated on the
    /// entity still being live AND being an Element.
    ///
    /// Destroyed entities short-circuit before either write,
    /// matching [`set_attribute`](Self::set_attribute)'s contract.
    /// Non-Element entities (Document / Text / Comment / ShadowRoot)
    /// also short-circuit — symmetric to `set_attribute`'s
    /// Element-only guard.  Without this, a stray
    /// `remove_attribute(non_element, ...)` would still bump
    /// `inclusive_descendants_version` and dispatch
    /// [`MutationEvent::AttributeChange`], cascading version bumps
    /// to attribute-filtered live collections and triggering
    /// downstream `MutationEvent` consumers (e.g. `BaseUrlMaintainer`,
    /// living in `elidex-dom-api`) on a receiver that cannot
    /// semantically own attributes.
    ///
    /// The attribute-storage write is itself a no-op when the
    /// `Attributes` component is absent or the key is missing,
    /// but the version bump still fires for live Element entities
    /// so attribute-filtered live collections invalidate cleanly
    /// even on spurious removals (the next read pays one walk and
    /// re-caches under the freshly bumped version).  See the SP2
    /// entity-list cache in `elidex-js::vm::host::dom_collection`;
    /// the `set_attribute` rationale on over-invalidation applies
    /// here too.
    pub fn remove_attribute(&mut self, entity: Entity, name: &str) {
        if !self.contains(entity) {
            return;
        }
        // Symmetric to `set_attribute`'s Element-only guard
        // (line ~939): non-Element entities never own `Attributes`,
        // so a remove on them is meaningless and must not cascade
        // version bumps / mutation events.
        if !matches!(self.node_kind(entity), Some(NodeKind::Element)) {
            return;
        }
        let old_value = self
            .world
            .get::<&mut Attributes>(entity)
            .ok()
            .and_then(|mut attrs| attrs.remove(name));
        self.reconcile_attribute_derived_components(entity, name);
        self.rev_version(entity);
        // Fire `MutationEvent::AttributeChange` ONLY when an attribute was
        // actually removed. DOM "remove an attribute by name" (§"remove an
        // attribute by name", step 2) removes — and thus queues a mutation
        // record via "handle attribute changes" — only when the attribute
        // is non-null; `removeAttribute("missing")` performs no mutation,
        // so MutationObserver consumers must not see a phantom removal.
        // (Unlike `set_attribute`, which always performs the mutation and
        // so queues even same-value writes.) The unconditional
        // `rev_version` above is a deliberate over-invalidation for
        // attribute-filtered live collections — distinct from the
        // observable mutation record gated here.
        if old_value.is_some() {
            let event = MutationEvent::AttributeChange {
                node: entity,
                name,
                old_value: old_value.as_deref(),
                new_value: None,
            };
            self.dispatch_event(&event);
        }
    }

    /// Re-make every **inline-reconciled** attribute-derived component
    /// consistent with the post-write `Attributes`, after the `name` content
    /// attribute is written or removed. The invariant is a single one —
    /// *component = f(`Attributes`)* — and the two arms below are just its two
    /// realizations (lazy-drop vs eager-rederive) given each component's
    /// materialization policy. Called by both
    /// [`set_attribute`](Self::set_attribute) /
    /// [`remove_attribute`](Self::remove_attribute) (after the `Attributes`
    /// mutation) and the deferred session-mutation flush in
    /// `elidex_script_session::mutation::apply_mutation` (which is `pub`'s
    /// reason — it writes `Attributes` without entering the chokepoints).
    ///
    /// **Why an inline seam in `elidex-ecs` core, not a `MutationEvent`
    /// consumer** — the mechanism higher layers use (e.g. `DocumentBaseUrl` via
    /// `elidex-dom-api`'s `BaseUrlMaintainer`, which subscribes to the
    /// `MutationEvent::AttributeChange` the chokepoint *dispatches* right after
    /// this call): these two components' consistency is a **core** invariant
    /// that must hold even when no consumer layer is composed, and two callers
    /// (`navigate_iframe`, the deferred flush) reconcile deliberately *without*
    /// dispatching an event (double-load avoidance) — neither is reducible to a
    /// consumer. So the split is a layering boundary, not a duplicate path.
    /// The two components:
    /// - **`InlineStyle`** (memoized parse of `attrs("style")`, materialized
    ///   lazily on first CSSOM access via `elidex_dom_api::ensure_inline_style`):
    ///   a `style` write changes the source of truth, so **drop the cache** —
    ///   the next `el.style.*` read re-hydrates. CSSOM mutators re-warm after
    ///   their own `set_attribute` (`sync_to_attribute`), so this is
    ///   perf-neutral for `el.style.*` sequences. (Closes the InlineStyle half
    ///   of slot `#11-derived-component-attr-maintenance`.)
    /// - **[`IframeData`]** (a pure projection of the iframe content attributes,
    ///   [`IframeData::from_attributes`], HTML §4.8.5): **re-derive eagerly**,
    ///   but only for entities that *already* carry it (i.e. `<iframe>`) — never
    ///   attach to a non-iframe that happens to get a `src`/`name`/… attribute.
    ///   Mirrors the clone-policy re-derive (`dom::tree_clone`). This closes the
    ///   IframeData half of the same slot: a generic `setAttribute("src", …)`
    ///   now keeps `IframeData` consistent with its attributes (the prior path
    ///   left the component stale, so the next load used the old URL).
    pub fn reconcile_attribute_derived_components(&mut self, entity: Entity, name: &str) {
        if name == "style" {
            let _ = self.world.remove_one::<InlineStyle>(entity);
        }
        // Presence-gated: `IframeData` exists ⇔ the entity is an `<iframe>`
        // (attached at parse / clone). Re-derive from the post-write attributes;
        // an iframe always has an `Attributes` component, but fall back to the
        // default projection if it somehow does not.
        if self.world.get::<&IframeData>(entity).is_ok() {
            let derived = self.world.get::<&Attributes>(entity).map_or_else(
                |_| IframeData::default(),
                |a| IframeData::from_attributes(&a),
            );
            let _ = self.world.insert_one(entity, derived);
        }
    }
}

/// Zero-allocation iterator over direct children of a DOM node.
///
/// Created by [`EcsDom::children_iter()`].
pub struct ChildrenIter<'a> {
    pub(crate) dom: &'a EcsDom,
    pub(crate) next: Option<Entity>,
    pub(crate) remaining: usize,
}

impl Iterator for ChildrenIter<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        loop {
            let entity = self.next?;
            if self.remaining == 0 {
                self.next = None;
                return None;
            }
            self.remaining -= 1;
            self.next = self.dom.read_rel(entity, |rel| rel.next_sibling);
            // M1: Skip ShadowRoot entities -- not exposed as children.
            if self.dom.world.get::<&ShadowRoot>(entity).is_err() {
                return Some(entity);
            }
        }
    }
}

/// Zero-allocation reverse iterator over direct children of a DOM node.
///
/// Walks from `last_child` via `prev_sibling` — yields children in
/// reverse sibling order.  Like [`ChildrenIter`], internal
/// [`ShadowRoot`] entities are skipped so the yielded items are the
/// externally visible direct children.  Stops after
/// [`MAX_ANCESTOR_DEPTH`] iterations to guard against corrupted
/// sibling chains.  Created by [`EcsDom::children_iter_rev()`].
pub struct ChildrenIterRev<'a> {
    pub(crate) dom: &'a EcsDom,
    pub(crate) next: Option<Entity>,
    pub(crate) remaining: usize,
}

impl Iterator for ChildrenIterRev<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        loop {
            let entity = self.next?;
            if self.remaining == 0 {
                self.next = None;
                return None;
            }
            self.remaining -= 1;
            self.next = self.dom.read_rel(entity, |rel| rel.prev_sibling);
            if self.dom.world.get::<&ShadowRoot>(entity).is_err() {
                return Some(entity);
            }
        }
    }
}

impl Default for EcsDom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
