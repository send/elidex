//! Mutation hook abstraction — fires from `EcsDom` tree + text-data primitives.
//!
//! First user: D-8 PR-A `LiveRangeRegistry` (Range live-tracking per
//! WHATWG DOM §5.5).
//!
//! Spec section coverage (D-8 PR-A complete set, 6 methods):
//! - §5.5 "Insert steps" → [`MutationHook::after_insert`] (pre-insertion index)
//! - §5.5 "Remove steps" → [`MutationHook::after_remove`] (pre-removal index,
//!   captured before detach)
//! - §5.5 "Set data steps" (whole-string write) →
//!   [`MutationHook::after_text_change`] via the
//!   [`crate::EcsDom::set_text_data`] chokepoint
//! - §4.10 "replace data" middle-splice (appendData / insertData /
//!   deleteData / replaceData) → [`MutationHook::after_replace_data`] via
//!   the [`crate::EcsDom::replace_text_data`] chokepoint
//! - §4.10 "split text" → [`MutationHook::after_split_text`] (caller-side
//!   fire via [`crate::EcsDom::fire_after_split_text`])
//! - §4.5 "normalize" step 6.4 → [`MutationHook::after_normalize_merge`]
//!   (caller-side fire via [`crate::EcsDom::fire_after_normalize_merge`])
//!
//! DEFERRED to future MutationObserver PR:
//! - `after_attribute_change` — needs `old_value` + `namespace` for §4.3.5;
//!   committing a wrong signature now is breaking-change risk
//!
//! `EcsDom` applies a **shallow** light-tree filter at fire sites:
//! callbacks are suppressed when EITHER `node` OR `parent` is itself a
//! [`ShadowRoot`](crate::ShadowRoot). Mutations DEEPER inside a shadow
//! tree (where `parent` is a normal element inside the shadow tree) DO
//! still fire — `EcsDom` does not walk the ancestor chain to find each
//! mutation's tree root, since that would add O(depth) cost to every
//! call. Consumers that need strict light-tree-only events
//! (e.g. `LiveRangeRegistry` per WHATWG §5.5) MUST filter by tree root
//! themselves on each callback.

use hecs::Entity;

/// Trait fired by [`crate::EcsDom`] mutation primitives after each mutation completes.
///
/// Each method has a default empty impl so that **existing** impls continue
/// to compile when new methods are added in the same crate. Adding a method
/// to this trait is still a breaking change for downstream impls under
/// strict semver, since adding the method changes the trait's vtable;
/// callers across crate boundaries must recompile when this trait gains
/// methods.
///
/// `Send + Sync` is required because some Worker-context impls (future) may
/// transfer `EcsDom` across threads. `hecs::World` is `Send + Sync`, so this is
/// not adding a constraint beyond what `EcsDom` already permits.
pub trait MutationHook: Send + Sync {
    /// Called AFTER a node has been removed from its parent (but, in the
    /// `destroy_entity` case, BEFORE the entity is despawned — so
    /// inspecting `node` via `dom.contains(node)` / component queries
    /// inside the callback still works).
    ///
    /// - `node`: the removed entity (alive when this fires; in the
    ///   `destroy_entity` path despawn happens immediately after the
    ///   callback returns).
    /// - `parent`: the former parent (still alive).
    /// - `removed_index`: the pre-removal index of `node` in `parent`'s child
    ///   list, captured BEFORE detach. Per WHATWG DOM §4.4 "remove" step 4,
    ///   this index is what Range live-tracking needs
    ///   (`adjust_ranges_for_removal`).
    ///
    /// Consumers (e.g. `LiveRangeRegistry`) MUST tolerate dangling boundary
    /// container references that surface across mutations and lazily
    /// collapse such Ranges on next access (e.g. by checking
    /// `dom.contains(boundary_container)` before use). Per the
    /// `destroy_entity` lazy-collapse contract, descendant entities
    /// orphaned by a destroy do NOT receive individual `after_remove` calls.
    ///
    /// This method does NOT receive the descendants snapshot — that
    /// information is delivered via the sibling
    /// [`Self::after_remove_with_descendants`] method, whose default
    /// impl delegates here. Consumers that need to reach the
    /// removed subtree (e.g. Range live-tracking) should override
    /// [`Self::after_remove_with_descendants`]; consumers that only
    /// care about the `(node, parent, removed_index)` shape (e.g.
    /// Mutation Observer's basic record) keep overriding this
    /// method.
    fn after_remove(&mut self, _node: Entity, _parent: Entity, _removed_index: usize) {}

    /// Snapshot-aware variant of [`Self::after_remove`] fired by
    /// [`crate::EcsDom`] mutation primitives (`remove_child` /
    /// `replace_child` / `destroy_entity`). The default impl
    /// delegates to [`Self::after_remove`] for back-compat — existing
    /// hooks that only override the 3-arg [`Self::after_remove`]
    /// continue to receive the basic event shape.
    ///
    /// `descendants` is a snapshot of `node` plus every light-tree
    /// inclusive descendant captured by the engine BEFORE any
    /// `destroy_entity`-style orphaning (PR186 R2 #3 / R4 #1
    /// additive-trait fix): the snapshot lets the consumer decide
    /// whether a Range boundary container falls inside the
    /// about-to-be-removed subtree without having to walk a parent
    /// chain that may have been cleared by the time the hook
    /// returns. For plain `remove_child` / `replace_child` the
    /// subtree is still tree-linked at call time so the snapshot
    /// duplicates what `is_ancestor_or_self` would find; for
    /// `destroy_entity` the snapshot is the only way for the
    /// consumer to reach orphaned descendants.
    ///
    /// Overriding this method effectively replaces the basic
    /// `after_remove` call — the engine fires
    /// `after_remove_with_descendants` only, never both.
    fn after_remove_with_descendants(
        &mut self,
        node: Entity,
        parent: Entity,
        removed_index: usize,
        _descendants: &[Entity],
    ) {
        self.after_remove(node, parent, removed_index);
    }

    /// Called AFTER a node has been inserted into a parent.
    ///
    /// - `node`: the newly-attached entity.
    /// - `parent`: the parent that received `node`.
    /// - `index`: the position `node` occupies after insertion — measured
    ///   over light-tree exposed siblings (shadow roots excluded).
    ///   Equivalently, the insertion index computed AFTER any implicit
    ///   detach of `node` from a prior parent and AFTER linking. For
    ///   `append_child`, this equals the post-detach child count of
    ///   `parent`. For `insert_before(parent, new, ref)`, this equals
    ///   `ref`'s post-detach index in parent (the slot `new` now
    ///   occupies). Per WHATWG DOM §5.5 "Insert steps", Range boundaries
    ///   at `(parent, offset)` where `offset > index` need `+=1` (strict
    ///   comparison).
    fn after_insert(&mut self, _node: Entity, _parent: Entity, _index: usize) {}

    /// Called AFTER a Text / CData entity's `TextContent` is rewritten as
    /// a single whole-string assignment (e.g. `set_text_data` /
    /// `textContent` setter / `Normalize` whole-text replacement).
    ///
    /// - `node`: the entity whose `TextContent` was rewritten.
    /// - `new_utf16_len`: the new UTF-16 length of the `TextContent`. WHATWG
    ///   DOM §5.5 "Set data steps" clamps Range boundaries on `node` to
    ///   `min(offset, new_utf16_len)`.
    ///
    /// Comment nodes are NOT covered by WHATWG §5.5 Range live-tracking, so
    /// `CommentData` writes do not fire this hook.
    ///
    /// Middle-splice operations (appendData / insertData / deleteData /
    /// replaceData) fire [`Self::after_replace_data`] instead, not this
    /// method — the spec boundary-adjustment math is different.
    fn after_text_change(&mut self, _node: Entity, _new_utf16_len: usize) {}

    /// Called AFTER an `appendData` / `insertData` / `deleteData` /
    /// `replaceData` splice on a Text / CData entity (WHATWG DOM §4.10
    /// "replace data" steps 8-11). Fires from the
    /// [`crate::EcsDom::replace_text_data`] chokepoint.
    ///
    /// - `node`: the entity whose `TextContent` was spliced.
    /// - `offset_utf16`: the UTF-16 offset where the splice started.
    /// - `count_utf16`: the UTF-16 count that was removed at `offset`.
    /// - `new_data_len_utf16`: the UTF-16 length of the inserted/replacement
    ///   string (0 for `deleteData`, replacement-string length for the
    ///   other three).
    ///
    /// Range live-tracking boundary adjustment per §4.10 step 8-11:
    /// - Boundary on `node` with `off ∈ [offset, offset + count]` →
    ///   collapse to `offset`.
    /// - Boundary on `node` with `off > offset + count` →
    ///   `off += new_data_len - count`.
    /// - Other boundaries unchanged.
    fn after_replace_data(
        &mut self,
        _node: Entity,
        _offset_utf16: usize,
        _count_utf16: usize,
        _new_data_len_utf16: usize,
    ) {
    }

    /// Called AFTER a `Text.splitText(offset)` operation (WHATWG DOM §4.10
    /// "split a Text node" step 7).
    ///
    /// **Ordering invariant**: this hook fires AFTER `new_node` has been
    /// inserted as a sibling of `node` but **BEFORE** `node`'s text is
    /// truncated. Boundary-on-`node` boundaries with `off > offset` must be
    /// MIGRATED to `(new_node, off - offset)` BEFORE the subsequent
    /// `set_text_data(node, head)` fires [`Self::after_text_change`] (which
    /// would otherwise clamp those boundaries on `node` to `head_len` and
    /// destroy the offset needed for migration). Boundaries with `off ==
    /// offset` stay on `node` at the truncated end per spec.
    ///
    /// - `node`: the original Text node (still holds the full pre-split
    ///   text at the moment this fires).
    /// - `new_node`: the newly-inserted sibling Text node holding the tail
    ///   `[offset..]`.
    /// - `offset_utf16`: the UTF-16 split point.
    /// - `parent`: the parent of `node`, or `None` if `node` was orphan
    ///   pre-split (in which case no parent-side adjustment applies).
    /// - `node_index`: the pre-split index of `node` in `parent`'s child
    ///   list, or `None` matched with `parent: None`.
    ///
    /// Range live-tracking boundary adjustment per §4.10 step 7:
    /// - Boundary on `node` with `off > offset` →
    ///   migrate to `(new_node, off - offset)` (spec §4.10 step 7.2 /
    ///   7.3 strict-greater). Boundaries at `off == offset` stay on
    ///   `node` (collapsed-at-split-point ranges are preserved on the
    ///   original node, per Chrome / Firefox observable behaviour).
    /// - Boundary on `parent` with `idx > node_idx` → `idx += 1`
    ///   (spec §4.10 step 7.2; the [`Self::after_insert`] hook fired by
    ///   the prior `insert_before` already handles `idx > node_idx + 1`
    ///   via strict-greater compare against the inserted-at index — the
    ///   consumer here must ONLY apply the missing `idx == node_idx + 1`
    ///   increment to avoid double-shifting).
    fn after_split_text(
        &mut self,
        _node: Entity,
        _new_node: Entity,
        _offset_utf16: usize,
        _parent: Option<Entity>,
        _node_index: Option<usize>,
    ) {
    }

    /// Called BEFORE the remove-merged-child step of `Node.normalize()`
    /// (WHATWG DOM §4.5 "normalize" step 6.4) on adjacent Text-node merge.
    ///
    /// **Ordering invariant**: this hook fires AFTER `prev` has absorbed
    /// `merged_child`'s data but BEFORE `merged_child` is detached from its
    /// parent. Firing before detach lets the consumer compute the migration
    /// without the subsequent [`Self::after_remove`] callback collapsing the
    /// boundary to `(parent, child_idx)` instead.
    ///
    /// - `merged_child`: the empty/redundant Text node about to be removed.
    /// - `prev`: the Text node that absorbed `merged_child`'s data
    ///   (`prev`'s `TextContent` already reflects the merged string when
    ///   this fires).
    /// - `prev_old_len_utf16`: the UTF-16 length of `prev`'s data BEFORE
    ///   the merge (the migration offset shift).
    /// - `parent`: the parent of `merged_child`, or `None` if no parent
    ///   was set (vacuous case).
    /// - `merged_child_index`: the pre-removal index of `merged_child` in
    ///   `parent`'s child list, or `None` matched with `parent: None`.
    ///
    /// Range live-tracking boundary adjustment per §4.5 step 6.4:
    /// - Boundary on `merged_child` at `off` →
    ///   migrate to `(prev, prev_old_len + off)`.
    /// - Boundary on `parent` at exactly `child_idx` of `merged_child` →
    ///   migrate to `(prev, prev_old_len)` (the merged splice point).
    ///   The subsequent [`Self::after_remove`] handles boundaries at
    ///   `off > child_idx` via the standard `-= 1` decrement — the
    ///   consumer here must NOT double-decrement those.
    fn after_normalize_merge(
        &mut self,
        _merged_child: Entity,
        _prev: Entity,
        _prev_old_len_utf16: usize,
        _parent: Option<Entity>,
        _merged_child_index: Option<usize>,
    ) {
    }
}
