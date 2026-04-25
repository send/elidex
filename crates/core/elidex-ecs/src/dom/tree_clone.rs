//! Node cloning methods for [`EcsDom`] (WHATWG DOM §4.5 "clone a node").
//!
//! Split from [`super::tree`] to keep both files below the 1000-line
//! convention (cleanup tranche 2).  The cloning surface is a
//! self-contained subgroup of `EcsDom` methods:
//!
//! - [`EcsDom::clone_attributes`] — single-entity attribute copy
//! - [`EcsDom::clone_character_data`] — single-entity Text / Comment payload copy
//! - [`EcsDom::clone_subtree`] — recursive deep clone (`cloneNode(true)`)
//! - [`EcsDom::clone_node_shallow`] — shallow clone (`cloneNode(false)`)
//!
//! All clone helpers share the WHATWG §4.5 invariants: the returned
//! root is a fresh orphan (no parent / sibling links into the
//! source), `ShadowRoot` / `ShadowHost` components are not copied,
//! and `EventListeners` (which lives in `SessionCore`, outside this
//! crate) is preserved separately by the script-session layer.

use crate::components::{
    Attributes, CommentData, DocTypeData, NodeKind, TagType, TextContent, TreeRelation,
};
use hecs::Entity;

use super::EcsDom;

impl EcsDom {
    // ---- Node cloning (WHATWG DOM §4.5) ----

    /// Copy every attribute from `src` to `dst`, creating an
    /// [`Attributes`] component on `dst` if one does not exist.
    /// No-op if `src` has no attributes or if either entity has been
    /// destroyed.
    pub fn clone_attributes(&mut self, src: Entity, dst: Entity) {
        if !self.all_exist(&[src, dst]) {
            return;
        }
        let snapshot: Option<Attributes> = self
            .world
            .get::<&Attributes>(src)
            .ok()
            .map(|attrs| (*attrs).clone());
        let Some(attrs) = snapshot else { return };
        if self.world.get::<&Attributes>(dst).is_ok() {
            if let Ok(mut existing) = self.world.get::<&mut Attributes>(dst) {
                *existing = attrs;
            }
        } else {
            let _ = self.world.insert_one(dst, attrs);
        }
    }

    /// Copy character data from `src` to `dst`.  Only acts when both
    /// entities share the same character-data [`NodeKind`] —
    /// `Text` → `Text` copies [`TextContent`], `Comment` → `Comment`
    /// copies [`CommentData`].  Mismatched kinds (and non-character-data
    /// entities) are no-ops so the destination never ends up with
    /// both a `TextContent` and a `CommentData` component.
    ///
    /// Currently unused by [`Self::clone_subtree`] — which handles
    /// character data internally when allocating the cloned root —
    /// but kept as a public helper for ad-hoc cross-entity payload
    /// copies.
    pub fn clone_character_data(&mut self, src: Entity, dst: Entity) {
        if !self.all_exist(&[src, dst]) {
            return;
        }
        let src_kind = self.node_kind(src);
        let dst_kind = self.node_kind(dst);
        match (src_kind, dst_kind) {
            (Some(NodeKind::Text), Some(NodeKind::Text)) => {
                let Some(text) = self
                    .world
                    .get::<&TextContent>(src)
                    .ok()
                    .map(|t| t.0.clone())
                else {
                    return;
                };
                if self.world.get::<&TextContent>(dst).is_ok() {
                    if let Ok(mut existing) = self.world.get::<&mut TextContent>(dst) {
                        existing.0 = text;
                    }
                } else {
                    let _ = self.world.insert_one(dst, TextContent(text));
                }
            }
            (Some(NodeKind::Comment), Some(NodeKind::Comment)) => {
                let Some(data) = self
                    .world
                    .get::<&CommentData>(src)
                    .ok()
                    .map(|c| c.0.clone())
                else {
                    return;
                };
                if self.world.get::<&CommentData>(dst).is_ok() {
                    if let Ok(mut existing) = self.world.get::<&mut CommentData>(dst) {
                        existing.0 = data;
                    }
                } else {
                    let _ = self.world.insert_one(dst, CommentData(data));
                }
            }
            _ => {}
        }
    }

    /// Recursively clone `src` and its descendants, returning the
    /// new root entity (WHATWG DOM §4.5 "clone a node").
    ///
    /// Invariants:
    /// - The returned root has default tree links
    ///   (`parent: None`, `prev_sibling: None`, `next_sibling: None`)
    ///   — i.e. the clone is a fresh orphan.
    /// - Descendant tree links mirror the source structure between
    ///   the newly-allocated entities (parent and sibling pointers
    ///   point inside the clone, never into the source subtree).
    /// - `ShadowRoot` / `ShadowHost` components are **not** copied —
    ///   closed shadow trees clone only the host's light tree.
    /// - `EventListeners` (and any other script-session-owned state
    ///   that lives outside the ECS) is **not** copied — those live
    ///   in `SessionCore`, which is outside this crate, and WHATWG
    ///   §4.5 specifies listeners are not cloned.
    ///
    /// Returns `None` when `src` does not exist, so a missing source
    /// can never alias the original via the returned handle.
    ///
    /// # WHATWG §4.5 "clone a node" — AssociatedDocument propagation
    ///
    /// Per the spec the `document` threaded through the recursion is
    /// the clone's *node document* and differs between branches:
    ///
    /// - **`src` is a Document** — allocate the copy, then replace
    ///   `document` with `copy` (a Document's node document is itself).
    ///   Descendants therefore report the *clone* as their
    ///   `ownerDocument`, not the source document.
    /// - **`src` is not a Document** — keep `document` at the caller's
    ///   initial value (the src node's own `ownerDocument`), so the
    ///   copy and its descendants inherit the src subtree's associated
    ///   document.
    ///
    /// Orphan nodes whose `ownerDocument` is `None` intentionally
    /// produce a clone with no [`crate::AssociatedDocument`] component —
    /// callers relying on the tree-root fallback behaviour remain
    /// unchanged.
    #[must_use = "returns None when src does not exist"]
    pub fn clone_subtree(&mut self, src: Entity) -> Option<Entity> {
        if !self.contains(src) {
            return None;
        }
        let src_kind = self.node_kind(src);
        // Pick the "document" that gets threaded through the spec's
        // recursion.  For Document clones we defer until after the
        // root has been allocated (it becomes self-referential); for
        // everything else we read the src node document up front.
        let document_for_children: Option<Entity> = if matches!(src_kind, Some(NodeKind::Document))
        {
            None
        } else {
            self.owner_document(src)
        };
        // Shallow-clone the root.  The root's own AssociatedDocument is
        // set below so we can handle the Document self-ref and the
        // non-Document propagation uniformly.
        let root = self.clone_node_shallow_unchecked(src);
        let root_document = if matches!(src_kind, Some(NodeKind::Document)) {
            root
        } else if let Some(doc) = document_for_children {
            self.set_associated_document(root, doc);
            doc
        } else {
            // Orphan clone — no AssociatedDocument anywhere in the
            // new subtree.  Skip the descendant propagation.
            self.clone_children_recursive(src, root, None);
            return Some(root);
        };
        if matches!(src_kind, Some(NodeKind::Document)) {
            self.set_associated_document(root, root_document);
        }
        self.clone_children_recursive(src, root, Some(root_document));
        Some(root)
    }

    /// Allocate a new entity carrying the same [`NodeKind`] and core
    /// component data as `src` (attributes for Elements, text/comment
    /// data for CharacterData, doctype fields for DocumentType) with
    /// no descendants and no parent/sibling links.
    ///
    /// Returns `None` when `src` does not exist so a missing source
    /// can never alias the original via the returned handle.  Tree
    /// relations on the clone are left at defaults — the caller
    /// threads the clone into place.  Does **not** copy `ShadowRoot`
    /// or `EventListeners`.
    ///
    /// Use this for `Node.cloneNode(false)` — it avoids the O(size)
    /// cost of cloning the full subtree only to destroy descendants
    /// that `cloneNode(false)` never wanted.
    #[must_use = "returns None when src does not exist"]
    pub fn clone_node_shallow(&mut self, src: Entity) -> Option<Entity> {
        if !self.contains(src) {
            return None;
        }
        Some(self.clone_node_shallow_unchecked(src))
    }

    /// Unchecked shallow-clone helper: spawns `dst` carrying `src`'s
    /// payload without validating that `src` exists.  Callers must
    /// verify existence first (the public [`clone_node_shallow`]
    /// does exactly that).
    ///
    /// [`clone_node_shallow`]: Self::clone_node_shallow
    fn clone_node_shallow_unchecked(&mut self, src: Entity) -> Entity {
        // Snapshot the payload under a read borrow, then spawn under
        // a mutable borrow.  Spawning a bare tuple first and then
        // inserting components one at a time keeps the hecs API
        // happy (each component needs its own `insert_one` call).
        // `.map(|t| (*t).clone())` — `*` dereferences the hecs `Ref`
        // so `clone()` clones the *data* rather than the reference
        // handle (which would keep `self.world` borrowed and block
        // the subsequent `spawn`).
        let tag = self.world.get::<&TagType>(src).ok().map(|t| (*t).clone());
        let text = self
            .world
            .get::<&TextContent>(src)
            .ok()
            .map(|t| (*t).clone());
        let comment = self
            .world
            .get::<&CommentData>(src)
            .ok()
            .map(|c| (*c).clone());
        let doc_type = self
            .world
            .get::<&DocTypeData>(src)
            .ok()
            .map(|d| (*d).clone());
        let attrs = self
            .world
            .get::<&Attributes>(src)
            .ok()
            .map(|a| (*a).clone());
        // Determine the destination `NodeKind`: prefer the source
        // component if present, otherwise infer from the payload
        // (defensive for entities that predate the `NodeKind`
        // component — e.g. anywhere `is_element` would fall back to
        // `TagType`).  Falling back to `Element` unconditionally
        // misclassifies Text/Comment entities that lack `NodeKind`
        // but carry a `TextContent` / `CommentData` payload.
        let kind = self.node_kind(src).unwrap_or_else(|| {
            if tag.is_some() {
                NodeKind::Element
            } else if text.is_some() {
                NodeKind::Text
            } else if comment.is_some() {
                NodeKind::Comment
            } else if doc_type.is_some() {
                NodeKind::DocumentType
            } else {
                NodeKind::Element
            }
        });

        let dst = self.world.spawn((TreeRelation::default(), kind));
        if let Some(tag) = tag {
            let _ = self.world.insert_one(dst, tag);
        }
        if let Some(text) = text {
            let _ = self.world.insert_one(dst, text);
        }
        if let Some(comment) = comment {
            let _ = self.world.insert_one(dst, comment);
        }
        if let Some(doc_type) = doc_type {
            let _ = self.world.insert_one(dst, doc_type);
        }
        if let Some(attrs) = attrs {
            let _ = self.world.insert_one(dst, attrs);
        }
        dst
    }

    /// Clone each child of `src` under `dst`, then every descendant
    /// depth-first.  Uses an explicit `(src_parent, dst_parent)` stack
    /// rather than Rust recursion so trees up to `MAX_ANCESTOR_DEPTH`
    /// (10 000) deep can be cloned without risk of a thread-stack
    /// overflow.  [`ShadowRoot`] children are skipped (matches
    /// `children_iter` semantics — the shadow root is not part of
    /// the light tree).
    fn clone_children_recursive(
        &mut self,
        src: Entity,
        dst: Entity,
        propagate_doc: Option<Entity>,
    ) {
        let mut stack: Vec<(Entity, Entity)> = vec![(src, dst)];
        while let Some((src_parent, dst_parent)) = stack.pop() {
            // Snapshot children before spawning new entities — tree
            // mutations during clone would invalidate
            // `children_iter`'s `next_sibling` reads.
            let kids: Vec<Entity> = self.children(src_parent);
            // Clone each child into the destination, recording the
            // `(src, dst)` pair for the descendant walk.  Push in
            // reverse so the explicit-stack traversal preserves the
            // same left-to-right depth-first ordering the recursive
            // version produced.
            let mut pending: Vec<(Entity, Entity)> = Vec::with_capacity(kids.len());
            for child_src in kids {
                // `kids` was snapshotted from the live tree, so each
                // entry is guaranteed to exist; the unchecked variant
                // skips the redundant `contains` lookup.
                let child_dst = self.clone_node_shallow_unchecked(child_src);
                if let Some(doc) = propagate_doc {
                    self.set_associated_document(child_dst, doc);
                }
                let _ = self.append_child(dst_parent, child_dst);
                pending.push((child_src, child_dst));
            }
            stack.extend(pending.into_iter().rev());
        }
    }
}
