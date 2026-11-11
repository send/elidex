//! Relinking tree mutations for [`EcsDom`] plus their `MutationEvent` fire
//! sites: `append_child` / `remove_child` / `insert_before` / `replace_child`,
//! DOM §4.5 *adopt* (the only writer of a node document after creation), the
//! low-level `link_node` / `detach` primitives, and the centralized
//! insert/remove dispatch helpers.

use std::collections::HashMap;

use super::super::mutation_event::{AdoptedContents, MutationEvent};
use super::super::EcsDom;
use super::TemplateContentsWalk;
use crate::components::{AttrEntityCache, ShadowRoot};
use crate::{ElementState, NodeKind};
use hecs::Entity;

impl EcsDom {
    // ---- Tree mutation ----

    /// Append `child` as the last child of `parent`.
    ///
    /// `child` is first adopted into `parent`'s node document (DOM §4.2.3
    /// *insert* step 7.1; see [`Self::adopt`]): removed from its parent, if
    /// any, then re-homed when the documents differ.
    /// Returns `false` if:
    /// - `parent == child` (self-append),
    /// - either entity has been destroyed,
    /// - `child` is an ancestor of `parent` (would create a cycle).
    #[must_use = "returns false if the operation failed"]
    pub fn append_child(&mut self, parent: Entity, child: Entity) -> bool {
        if parent == child {
            return false;
        }
        if !self.all_exist(&[parent, child]) {
            return false;
        }
        if self.is_ancestor(child, parent) {
            return false;
        }

        // Capture pre-mutation connectedness of `child` for
        // `MutationEvent::Insert.was_connected` (HTML §4.13.6 Custom
        // Element `connectedCallback` transition gate). MUST be read
        // BEFORE `detach_with_hook` runs, which itself fires an
        // implicit Remove whose `was_connected` reflects the same
        // pre-mutation state.
        let child_was_connected = self.is_connected(child);
        // DOM §4.2.3 *insert* step 7.1: adopt `child` into parent's node
        // document (its removal from the old parent, then the walk), before
        // the link and every insertion consumer.
        self.adopt_for_insert(child, parent);

        let last_child = self.read_rel(parent, |rel| rel.last_child);
        self.link_node(parent, child, last_child, None);
        self.rev_version(parent);

        if let Some(new_index) = self.index_in_parent(child) {
            self.fire_after_insert(child, parent, new_index, child_was_connected);
        }

        true
    }

    /// Remove `child` from `parent`.
    ///
    /// Returns `false` if either entity is destroyed or `child` is not a
    /// child of `parent`.
    #[must_use = "returns false if the operation failed"]
    pub fn remove_child(&mut self, parent: Entity, child: Entity) -> bool {
        if !self.all_exist(&[parent, child]) {
            return false;
        }
        if !self.is_child_of(child, parent) {
            return false;
        }
        let removed_index = self.index_in_parent(child);
        // Pre-detach connectedness for `MutationEvent::Remove.
        // was_connected` (HTML §4.13.6 Custom Element
        // `disconnectedCallback` transition gate).
        let was_connected = self.is_connected(child);
        self.detach(child);
        self.rev_version(parent);
        if let Some(idx) = removed_index {
            self.fire_after_remove(child, parent, idx, was_connected);
        } else if was_connected {
            // `index_in_parent` capped out — a focused child with more than
            // MAX_ANCESTOR_DEPTH previous siblings in a wide-but-valid DOM — so
            // `fire_after_remove`, and the §2.1.4 focused-area reset it carries,
            // were skipped. The reset must still run: removing the focused element
            // has to clear `ElementState::FOCUS` regardless of sibling count, else
            // reattaching the node resurrects `activeElement` + keyboard routing.
            // The reset is connectedness-driven (needs no index); mirrors
            // `destroy_entity`'s `defer_focus_clear`. (`clear_focus_if_disconnected`
            // is idempotent — a no-op when the focus holder stays connected.)
            self.clear_focus_if_disconnected();
        }
        true
    }

    /// Insert `new_child` before `ref_child` under `parent`, adopting it into
    /// `parent`'s node document first (DOM §4.2.3 *insert* step 7.1).
    ///
    /// Returns `false` if any entity is destroyed, `ref_child` is not a child
    /// of `parent`, `new_child == parent`, `new_child == ref_child`, or
    /// `new_child` is an ancestor of `parent` (would create a cycle).
    #[must_use = "returns false if the operation failed"]
    pub fn insert_before(&mut self, parent: Entity, new_child: Entity, ref_child: Entity) -> bool {
        if new_child == parent || new_child == ref_child {
            return false;
        }
        if !self.all_exist(&[parent, new_child, ref_child]) {
            return false;
        }
        if self.is_ancestor(new_child, parent) {
            return false;
        }
        if !self.is_child_of(ref_child, parent) {
            return false;
        }

        // Pre-mutation connectedness for `was_connected` — same
        // gating rationale as `append_child`.
        let new_child_was_connected = self.is_connected(new_child);
        // DOM §4.2.3 *insert* step 7.1: adopt `new_child` into parent's node
        // document — its removal from the current position (fires
        // after_remove on the implicit old-parent removal, *adopt* step 2),
        // then the walk.
        self.adopt_for_insert(new_child, parent);

        // Re-read ref_child's prev_sibling AFTER detach (it may have changed
        // if new_child was an adjacent sibling).
        let ref_prev = self.read_rel(ref_child, |rel| rel.prev_sibling);
        self.link_node(parent, new_child, ref_prev, Some(ref_child));
        self.rev_version(parent);

        if let Some(new_index) = self.index_in_parent(new_child) {
            self.fire_after_insert(new_child, parent, new_index, new_child_was_connected);
        }

        true
    }

    /// Replace `old_child` with `new_child` under `parent`.
    ///
    /// `new_child` is adopted into `parent`'s node document (DOM §4.2.3
    /// *replace* step 6) before `old_child` is detached from the tree (step
    /// 7). Returns `false` if any entity
    /// is destroyed, `old_child` is not a child of `parent`, or `new_child`
    /// is an ancestor of `parent` (would create a cycle).
    ///
    /// Validation is performed **before** detaching `new_child`, so the tree
    /// is never left in a corrupted state on failure.
    #[must_use = "returns false if the operation failed"]
    pub fn replace_child(&mut self, parent: Entity, new_child: Entity, old_child: Entity) -> bool {
        if new_child == parent || new_child == old_child {
            return false;
        }
        if !self.all_exist(&[parent, new_child, old_child]) {
            return false;
        }
        if self.is_ancestor(new_child, parent) {
            return false;
        }

        // Verify old_child is a child of parent BEFORE detaching new_child.
        if !self.is_child_of(old_child, parent) {
            return false;
        }

        // Pre-mutation connectedness for both subjects. `old_child` is
        // connected via `parent` (which is unaffected by the
        // forthcoming removal of `new_child` in `adopt_for_insert` — per the
        // `is_ancestor(new_child, parent)` rejection above, `new_child`
        // cannot be on `old_child`'s ancestor chain).
        let new_child_was_connected = self.is_connected(new_child);
        let old_child_was_connected = self.is_connected(old_child);
        // DOM §4.2.3 *replace* step 6: adopt `new_child` into parent's node
        // document — its removal from the current position (fires
        // after_remove, *adopt* step 2 "If node's parent is non-null, then
        // remove node"), then the walk — before step 7 removes `old_child`.
        self.adopt_for_insert(new_child, parent);

        // Capture old_child's index AFTER detach so that — when new_child was
        // an earlier sibling of old_child in the same parent — the index
        // reflects old_child's actual position at the moment of removal
        // (the WHATWG DOM *remove* algorithm's live-range pre-remove steps, which
        // use node's index).
        let old_index = self.index_in_parent(old_child);

        // Re-read old_child's siblings AFTER detach (they may have changed
        // if new_child was an adjacent sibling).
        let (old_prev, old_next) =
            self.read_rel(old_child, |rel| (rel.prev_sibling, rel.next_sibling));

        // Copilot R20: fully detach `old_child` from parent's sibling chain
        // BEFORE firing the after_remove hook + linking the replacement.
        // The mutation-hook post-detach invariant promises that, at fire
        // time, `parent.children[removed_index]` is the first follower of
        // the removed node — earlier impl linked `new_child` into that
        // slot first, so `LiveRangeRegistry` / `MutationBridge` consumers
        // (NodeIterator pre-removing-steps especially) would pick the
        // replacement as the follower instead of `old_child`'s real next
        // sibling.
        self.detach(old_child);

        if let Some(idx) = old_index {
            self.fire_after_remove(old_child, parent, idx, old_child_was_connected);
        } else if old_child_was_connected {
            // index_in_parent capped out (wide DOM) → fire_after_remove skipped;
            // run the §2.1.4 focused-area reset directly (see `remove_child`).
            self.clear_focus_if_disconnected();
        }

        // Now link `new_child` into the slot `old_child` used to occupy.
        self.link_node(parent, new_child, old_prev, old_next);
        self.rev_version(parent);

        if let Some(idx) = old_index {
            self.fire_after_insert(new_child, parent, idx, new_child_was_connected);
        }

        true
    }

    // ---- DOM §4.5 adopt ----

    /// WHATWG DOM §4.5 *adopt* (`#concept-node-adopt`) `node` into `document`.
    ///
    /// Step 1: oldDocument = `node`'s node document. Step 2: if `node` has a
    /// parent, remove it (firing [`MutationEvent::Remove`] on the old parent).
    /// Step 3, when `document` is not oldDocument, for each
    /// shadow-including inclusive descendant: 3.1 its node document becomes
    /// `document`; 3.3.1 so does that of each of its attributes (the Attr
    /// entities materialized in its [`AttrEntityCache`]); 3.4 the adopting
    /// steps — a `<template>`'s contents are adopted into the *appropriate
    /// template contents owner document* of `document` (HTML §4.12.3 template
    /// *adopting steps* 1–2: the walk visits them,
    /// [`TemplateContentsWalk::Include`], each fragment a nested adopt with
    /// its own pair, or skipped when its documents already agree). Steps 3.2
    /// and 3.3.2 (custom element registries) are not modelled; for 3.3.3
    /// (`adoptedCallback`) the walk fires one [`MutationEvent::Adopt`]
    /// carrying the root's pair and the contents segments, which the
    /// custom-element consumer turns into the reactions.
    ///
    /// A node without a node document (oldDocument `None`, no spec
    /// counterpart) is re-homed silently. A destroyed `node` or a `document`
    /// that is not a live Document is a no-op.
    ///
    /// `node` must be a Node and not a Document: every caller has already
    /// rejected a Document (DOM *ensure pre-insert validity* step 4 before an
    /// insertion; `adoptNode`'s step-1 `NotSupportedError` is its caller's),
    /// and only Nodes carry a node document (a Window / Worker entity is never
    /// stamped), so this is a debug assertion, not a check.
    pub fn adopt(&mut self, node: Entity, document: Entity) {
        debug_assert!(
            self.node_kind(node)
                .is_none_or(|kind| kind.is_node() && kind != NodeKind::Document),
            "adopt: only a non-Document Node can be adopted (callers reject the rest first)"
        );
        if !self.contains(node) || !self.is_live_document(document) {
            return;
        }
        self.adopt_into(node, Some(document), None);
    }

    /// *Adopt* inside the primitives (DOM §4.2.3 *insert* step 7.1, *replace*
    /// step 6): the target is parent's node document through
    /// [`Self::node_document_for_new_nodes`] (the HTML §13.4 fragment-parse
    /// root redirects to the context's document). The removal always runs;
    /// the walk runs only when the target is `Some` and differs from the
    /// child's node document, so a parent without a node document leaves the
    /// child's stamp as it is.
    fn adopt_for_insert(&mut self, child: Entity, parent: Entity) {
        let target = self.node_document_for_new_nodes(parent).document();
        self.adopt_into(child, target, Some(parent));
    }

    /// *Adopt* steps 1–3 shared by [`Self::adopt`] and the primitives: read
    /// oldDocument, remove `node` (`new_parent` is the hook's insertion
    /// target, if any), then walk iff `target` is `Some` and differs, and
    /// fire [`MutationEvent::Adopt`] iff the root's pair is `Some` — after
    /// the walk and before returning to the primitive, so before its link
    /// and [`MutationEvent::Insert`].
    ///
    /// `target` is a live Document when `Some`, so comparing the raw stamp is
    /// the same test as comparing `node_document` (a dead stamp differs too);
    /// removal does not touch the stamp. The root's pair is
    /// `(node_document(node), document)`: a node without a live node document
    /// is re-homed silently (no event; every segment under it is `None`).
    fn adopt_into(&mut self, node: Entity, target: Option<Entity>, new_parent: Option<Entity>) {
        self.detach_with_hook(node, new_parent);
        let Some(document) = target else {
            return;
        };
        if self.get_associated_document(node) == Some(document) {
            return;
        }
        let documents = self.node_document(node).map(|old| (old, document));
        let contents = self.adopt_walk(node, document, documents.is_some());
        if documents.is_some() {
            self.dispatch_event(&MutationEvent::Adopt {
                node,
                documents,
                contents: &contents,
            });
        }
    }

    /// *Adopt* step 3 (3.1, 3.3.1, 3.4): stamp `root`, every
    /// shadow-including descendant and every template's contents, and the
    /// Attr entities each visited element has materialized. Uncapped in depth
    /// and breadth. Returns the contents segments in walk order.
    ///
    /// Each contents fragment the walk reaches is the root of a nested
    /// *adopt* (HTML §4.12.3 template *adopting steps* 1–2): its `old` is its
    /// own node document, read here before the walk writes it, and `new` is
    /// the *appropriate template contents owner document* of `document`
    /// ([`Self::appropriate_template_contents_owner_document`], created on
    /// first use; computed once per walk, and only by a walk that reaches a
    /// contents fragment). Every node whose innermost contents fragment is
    /// live is stamped with that `new`; the root's own segment (the root, its
    /// shadow-including descendants outside contents) keeps `document`, even
    /// when the root is itself a contents fragment (`X.adoptNode(tpl.content)`
    /// adopts it into X). The fragment is **skipped** — no stamp on it or on
    /// any node whose innermost contents fragment is skipped, no Attr
    /// re-home, no segment — when `old == new` (the nested adopt's step 3
    /// guard) or its template's innermost contents fragment is skipped.
    /// Otherwise it records a segment whose pair is `None` when `old` is
    /// `None` or the template lies in a `None` segment (`root_pair` false
    /// for the root's own segment).
    ///
    /// A node's innermost segment is carried down the walk: the walk visits a
    /// node after its parent (a shadow root after its host, a contents
    /// fragment right after its template), so each node reads its parent's
    /// segment, with no capped ancestor climb.
    fn adopt_walk(
        &mut self,
        root: Entity,
        document: Entity,
        root_pair: bool,
    ) -> Vec<AdoptedContents> {
        #[cfg(any(test, feature = "fixtures"))]
        {
            self.adopt_walks += 1;
        }
        let mut nodes: Vec<Entity> = Vec::new();
        self.for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, &mut |e| {
            nodes.push(e);
        });
        // The contents fragments the walk reaches, each with its template.
        // The root is excluded: it is never a contents segment, so no
        // fragment overwrites it. This guards against a contents cycle
        // (`tpl.content.appendChild(tpl)`), which the pre-insert validator
        // accepts today: the template contents' host is not modelled
        // (`#11-template-contents-host-link`), so
        // `is_host_including_ancestor_or_self` does not climb from a contents
        // fragment to its template. Without the guard, adopting such a
        // fragment would record the root as its own template's segment.
        let templates: HashMap<Entity, Entity> = nodes
            .iter()
            .filter_map(|&n| {
                self.template_contents_fragment(n)
                    .filter(|&f| f != root)
                    .map(|f| (f, n))
            })
            .collect();
        // No template reached: every node is in the root's segment, so stamp
        // them all with no segment bookkeeping (an empty `collect` does not
        // allocate).
        if templates.is_empty() {
            for node in nodes {
                self.stamp_node_and_attrs(node, document);
            }
            return Vec::new();
        }
        // HTML §4.12.3 template *adopting steps* 1: the contents' document.
        let contents_document = self.appropriate_template_contents_owner_document(document);
        let root_segment = Segment::Live {
            pair: root_pair,
            document,
        };
        let mut segment_of: HashMap<Entity, Segment> = HashMap::with_capacity(nodes.len());
        let mut contents: Vec<AdoptedContents> = Vec::new();
        for node in nodes {
            let segment = if node == root {
                root_segment
            } else if let Some(&template) = templates.get(&node) {
                let template_segment = segment_of.get(&template).copied();
                let old = self.node_document(node);
                match template_segment {
                    Some(Segment::Live { pair, .. }) if old != Some(contents_document) => {
                        let documents = old.filter(|_| pair).map(|old| (old, contents_document));
                        contents.push(AdoptedContents {
                            fragment: node,
                            documents,
                        });
                        Segment::Live {
                            pair: documents.is_some(),
                            document: contents_document,
                        }
                    }
                    _ => Segment::Skipped,
                }
            } else {
                self.walk_parent(node)
                    .and_then(|parent| segment_of.get(&parent).copied())
                    .unwrap_or(root_segment)
            };
            segment_of.insert(node, segment);
            if let Segment::Live { document, .. } = segment {
                self.stamp_node_and_attrs(node, document);
            }
        }
        contents
    }

    /// The node the adopt walk reached `node` from: its parent, or a shadow
    /// root's host.
    fn walk_parent(&self, node: Entity) -> Option<Entity> {
        self.get_parent(node)
            .or_else(|| self.world.get::<&ShadowRoot>(node).ok().map(|sr| sr.host))
    }

    /// Stamp `node` and the Attr entities it has materialized with
    /// `document` (*adopt* steps 3.1 and 3.3.1).
    fn stamp_node_and_attrs(&mut self, node: Entity, document: Entity) {
        let _ = self.set_associated_document(node, document);
        let attrs: Vec<Entity> = self
            .world
            .get::<&AttrEntityCache>(node)
            .map(|cache| cache.entries.values().copied().collect())
            .unwrap_or_default();
        for attr in attrs {
            let _ = self.set_associated_document(attr, document);
        }
    }

    /// How many times the *adopt* walk has run on this DOM (C0b's walk
    /// counter): a link whose child already carries the target document does
    /// not walk. Test-only, like the fixture creators.
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    pub fn adopt_walk_count(&self) -> u64 {
        self.adopt_walks
    }

    /// Place `node` into `parent`'s child list between `prev` and `next`.
    ///
    /// Updates all affected sibling pointers and parent's first/last child.
    pub(super) fn link_node(
        &mut self,
        parent: Entity,
        node: Entity,
        prev: Option<Entity>,
        next: Option<Entity>,
    ) {
        self.update_rel(node, |rel| {
            rel.parent = Some(parent);
            rel.prev_sibling = prev;
            rel.next_sibling = next;
        });

        if let Some(prev_entity) = prev {
            self.update_rel(prev_entity, |rel| rel.next_sibling = Some(node));
        } else {
            self.update_rel(parent, |rel| rel.first_child = Some(node));
        }

        if let Some(next_entity) = next {
            self.update_rel(next_entity, |rel| rel.prev_sibling = Some(node));
        } else {
            self.update_rel(parent, |rel| rel.last_child = Some(node));
        }
    }

    /// Detach an entity from its parent and siblings.
    pub(super) fn detach(&mut self, entity: Entity) {
        let (parent, prev, next) = self.read_rel(entity, |rel| {
            (rel.parent, rel.prev_sibling, rel.next_sibling)
        });
        if parent.is_none() {
            return;
        }

        if let Some(prev_entity) = prev {
            self.update_rel(prev_entity, |rel| rel.next_sibling = next);
        }

        if let Some(next_entity) = next {
            self.update_rel(next_entity, |rel| rel.prev_sibling = prev);
        }

        if let Some(parent_entity) = parent {
            self.update_rel(parent_entity, |rel| {
                if rel.first_child == Some(entity) {
                    rel.first_child = next;
                }
                if rel.last_child == Some(entity) {
                    rel.last_child = prev;
                }
            });
        }

        self.clear_rel(entity);
    }

    /// Centralized `MutationEvent::Insert` fire site for tree mutations.
    ///
    /// Per the light-tree-only contract, the event is suppressed when
    /// **either** `node` or `parent` is itself a shadow root —
    /// shadow roots are internal to the engine and shadow-tree
    /// mutations under the root must not surface to light-tree
    /// consumers (e.g. Range live-tracking). Deeper mutations within
    /// the shadow tree (where `parent` is a normal element inside the
    /// shadow tree) are NOT filtered here — those consumers should
    /// filter by tree root if they want light-tree-only events.
    ///
    /// `was_connected` is the connectedness of `node` BEFORE this
    /// mutation began (captured by callers via [`Self::is_connected`]
    /// prior to any implicit detach). Required by Custom Elements
    /// `connectedCallback` gating (HTML §4.13.6).
    fn fire_after_insert(
        &mut self,
        node: Entity,
        parent: Entity,
        index: usize,
        was_connected: bool,
    ) {
        if self.is_shadow_root(node) || self.is_shadow_root(parent) {
            return;
        }
        let event = MutationEvent::Insert {
            node,
            parent,
            index,
            was_connected,
        };
        self.dispatch_event(&event);
    }

    /// Centralized `MutationEvent::Remove` fire site for tree mutations.
    /// Suppression rules match [`Self::fire_after_insert`].
    ///
    /// `was_connected` is the connectedness of `node` BEFORE detach
    /// (captured by callers via [`Self::is_connected`] prior to
    /// `self.detach(node)`). Required by Custom Elements
    /// `disconnectedCallback` gating (HTML §4.13.6).
    pub(super) fn fire_after_remove(
        &mut self,
        node: Entity,
        parent: Entity,
        removed_index: usize,
        was_connected: bool,
    ) {
        // WHATWG HTML §2.1.4 "removing steps for the HTML Standard" step 2:
        // when the document's focused area leaves the tree, reset it to the
        // viewport — clear `ElementState::FOCUS`. The spec is explicit it
        // "does not perform the unfocusing steps, focusing steps, or focus
        // update steps, and thus no blur or change events are fired" (focusout
        // / focusin too, as those only fire via the steps that don't run) — so
        // this is a silent component mutation (the event-dispatching focusing
        // steps stay engine-bound in the shell). Run it BEFORE the light-tree
        // event suppression below
        // and via the SHADOW-INCLUSIVE `is_connected` (not the light-tree
        // `descendants` snapshot), so focus held inside a removed host's shadow
        // tree — or a light child of a removed shadow root — is still reset.
        // Only the removal of a previously-connected node can disconnect the
        // (connected-by-construction) holder, so gate on `was_connected`.
        //
        // This INTENTIONALLY also clears focus on a same-document *move*
        // (`append_child` / `insert_before` / `replace_child` of an already-
        // parented node): DOM `insert` adopts the node, and `adopt` removes it
        // from its old parent before the re-insert (DOM §"adopt" step 2), so
        // these removing steps run on the transient detach. Only `Node.moveBefore()`'s
        // separate *moving steps* preserve the focused area (HTML §2.1.4 defines
        // them distinctly), and elidex does not implement `moveBefore`; focus
        // loss across a plain reparent matches browsers (it is exactly what
        // `moveBefore` was added to avoid). Pinned by the `move_clears_focus`
        // dom-api test. (Codex S2 R6/R9 flagged this as a bug twice — it is not.)
        if was_connected {
            self.clear_focus_if_disconnected();
        }
        if self.is_shadow_root(node) || self.is_shadow_root(parent) {
            return;
        }
        // PR186 R2 #3: snapshot the light-tree inclusive-descendant
        // set BEFORE the caller orphans children / despawns the
        // subtree (the `destroy_entity` path clears descendant parent
        // links after this fire). The dispatcher consumer uses this
        // snapshot to collapse Range boundaries on any inclusive
        // descendant per WHATWG DOM §4.2.3 remove algorithm —
        // `is_ancestor_or_self` walking up from the boundary container
        // would miss orphaned descendants whose parent link is about
        // to be cleared.
        let descendants = self.collect_inclusive_descendants(node);
        let event = MutationEvent::Remove {
            node,
            parent,
            removed_index,
            descendants: &descendants,
            was_connected,
        };
        self.dispatch_event(&event);
    }

    /// Clear [`ElementState::FOCUS`] if its (single) holder is no longer
    /// connected (WHATWG HTML §2.1.4 removing steps step 2 — focused-area reset
    /// on removal, *silent*: no events). Single-focus ⇒ at most one holder; the
    /// `is_connected` check is **shadow-inclusive** (it walks the shadow-
    /// including ancestor chain), so this uniformly covers light-tree removal,
    /// a removed shadow host whose shadow tree held focus, and a removed shadow
    /// root's light children — none of which the light-tree event path reaches.
    /// Despawn-based removal needs no hook (hecs drops the component with the
    /// entity); this covers detach-without-despawn — including
    /// [`EcsDom::destroy_entity`]'s no-dispatcher path, which orphans (does not
    /// despawn) descendants and skips [`Self::fire_after_remove`], so it calls
    /// this directly.
    pub(super) fn clear_focus_if_disconnected(&mut self) {
        let holder = self
            .world
            .query::<(Entity, &ElementState)>()
            .iter()
            .find(|(_, state)| state.contains(ElementState::FOCUS))
            .map(|(e, _)| e);
        let Some(holder) = holder else { return };
        if self.is_connected(holder) {
            return;
        }
        let cleared = match self.world.get::<&ElementState>(holder) {
            Ok(state) => {
                let mut next = *state;
                next.remove(ElementState::FOCUS);
                Some(next)
            }
            Err(_) => None,
        };
        if let Some(next) = cleared {
            let _ = self.world.insert_one(holder, next);
        }
    }

    /// Light-tree inclusive-descendant walker — collects `node` plus
    /// every descendant reachable via [`Self::children_iter`] (which
    /// suppresses shadow roots). Used by [`Self::fire_after_remove`]
    /// to snapshot the subtree pre-orphan so `MutationDispatcher`
    /// consumers can apply boundary adjustments without depending on
    /// intact parent links.
    fn collect_inclusive_descendants(&self, node: Entity) -> Vec<Entity> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            out.push(n);
            for child in self.children_iter(n) {
                stack.push(child);
            }
        }
        out
    }

    /// Detach `child` from its current parent and fire
    /// [`MutationEvent::Remove`] on the
    /// installed [`MutationDispatcher`](super::super::MutationDispatcher), if
    /// any.
    ///
    /// Used by `append_child` / `insert_before` / `replace_child` to
    /// notify consumers of the **implicit** removal that happens when a
    /// node is moved between parents (WHATWG DOM "insert" algorithm step
    /// "If node has a parent, then remove node"). Returns `false` when
    /// `child` has no parent (nothing to detach).
    ///
    /// `new_parent` is the parent the caller is about to re-link `child`
    /// into (`None` for a standalone [`Self::adopt`]). When `old_parent ==
    /// new_parent` the caller's post-link
    /// `rev_version(new_parent)` already covers the bump, so the
    /// version-tracking call here is skipped to avoid a redundant
    /// ancestor-walk per same-parent move.
    ///
    /// Per the light-tree-only contract of `MutationDispatcher`, the
    /// `Remove` event is suppressed when `child` is itself a shadow
    /// root — shadow roots have a `TreeRelation.parent` set to the host
    /// (via the internal `attach_shadow → append_child` plumbing), but
    /// are not exposed as DOM children.
    fn detach_with_hook(&mut self, child: Entity, new_parent: Option<Entity>) -> bool {
        let Some(old_parent) = self.get_parent(child) else {
            return false;
        };
        let old_index = self.index_in_parent(child);
        // Pre-detach connectedness for `was_connected` — same gating
        // rationale as the public `remove_child` path.
        let was_connected = self.is_connected(child);
        self.detach(child);
        // Only bump the old parent's version when it differs from the
        // new parent. Same-parent moves rely on the caller's
        // post-link `rev_version(new_parent)`.
        if Some(old_parent) != new_parent {
            self.rev_version(old_parent);
        }
        if let Some(idx) = old_index {
            self.fire_after_remove(child, old_parent, idx, was_connected);
        } else if was_connected {
            // index_in_parent capped out (wide DOM) → fire_after_remove skipped;
            // run the §2.1.4 focused-area reset directly so a move of the focused
            // element clears FOCUS regardless of sibling count (see `remove_child`).
            self.clear_focus_if_disconnected();
        }
        true
    }
}

/// A node's innermost adopt segment, as the walk sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    /// The root's segment or a recorded one: `pair` is whether its pair is
    /// `Some`, `document` the document its nodes are stamped with (the
    /// root's: the adopt's target; a contents segment's: the target's inert
    /// template document).
    Live { pair: bool, document: Entity },
    /// A skipped contents fragment: its nodes are left as they are.
    Skipped,
}
