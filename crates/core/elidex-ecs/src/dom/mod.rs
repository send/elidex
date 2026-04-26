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

mod shadow;
mod tree;
mod tree_clone;

use crate::components::{
    AssociatedDocument, AttrData, Attributes, CommentData, DocTypeData, NodeKind, ShadowRoot,
    TagType, TextContent, TreeRelation,
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
}

impl EcsDom {
    /// Create a new, empty DOM.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            document_root: None,
        }
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

    /// Create a document root entity (no tag, only tree relations).
    ///
    /// The document root serves as the parent of the `<html>` element.
    /// The entity is cached for fast retrieval via [`document_root()`](Self::document_root).
    pub fn create_document_root(&mut self) -> Entity {
        let entity = self
            .world
            .spawn((TreeRelation::default(), NodeKind::Document));
        self.document_root = Some(entity);
        entity
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

    /// Read attribute `name` on `entity`, returning `None` if the
    /// `Attributes` component is absent or the key is not present.
    ///
    /// Allocates a fresh `String`; prefer [`Self::with_attribute`]
    /// for borrow-only consumers (existence checks, equality
    /// comparisons, intern-on-Some) — that path keeps the value as
    /// `Option<&str>` and skips the `String::from` clone.
    #[must_use]
    pub fn get_attribute(&self, entity: Entity, name: &str) -> Option<String> {
        self.with_attribute(entity, name, |v| v.map(String::from))
    }

    /// Borrow attribute `name` on `entity` and project through `f`.
    ///
    /// `f` is called with `Some(value)` when both the `Attributes`
    /// component and key are present, and `None` otherwise.  This
    /// is the zero-allocation sibling of [`Self::get_attribute`] —
    /// callers that only need to compare, parse, or hash the value
    /// can avoid the `String::from` clone the owned getter performs.
    /// Mirror of [`Self::read_rel`] for attribute reads.
    pub fn with_attribute<R>(
        &self,
        entity: Entity,
        name: &str,
        f: impl FnOnce(Option<&str>) -> R,
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
    /// Returns `false` if the entity has been destroyed.
    pub fn set_attribute(&mut self, entity: Entity, name: &str, value: String) -> bool {
        if !self.contains(entity) {
            return false;
        }
        let has_component = self.world.get::<&Attributes>(entity).is_ok();
        if has_component {
            if let Ok(mut attrs) = self.world.get::<&mut Attributes>(entity) {
                attrs.set(name, value);
                return true;
            }
            return false;
        }
        let mut attrs = Attributes::default();
        attrs.set(name, value);
        self.world.insert_one(entity, attrs).is_ok()
    }

    /// Remove attribute `name` from `entity`.  No-op if the entity is
    /// destroyed, the `Attributes` component is absent, or the key is
    /// missing.
    pub fn remove_attribute(&mut self, entity: Entity, name: &str) {
        if let Ok(mut attrs) = self.world.get::<&mut Attributes>(entity) {
            attrs.remove(name);
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
