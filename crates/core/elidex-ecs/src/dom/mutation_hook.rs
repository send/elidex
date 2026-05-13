//! Mutation hook abstraction — fires from `EcsDom` tree + text-data primitives.
//!
//! First user: D-8 PR-A `LiveRangeRegistry` (Range live-tracking per
//! WHATWG DOM §5.5).
//!
//! Spec section coverage in this prereq PR:
//! - §5.5 "Insert steps" → [`MutationHook::after_insert`] (pre-insertion index)
//! - §5.5 "Remove steps" → [`MutationHook::after_remove`] (pre-removal index,
//!   captured before detach)
//! - §5.5 "Set/Replace data steps" → [`MutationHook::after_text_change`] via
//!   the [`crate::EcsDom::set_text_data`] chokepoint
//!
//! DEFERRED to D-8 PR-A (LiveRangeRegistry will handle inline):
//! - §5.5 "Split text steps" — boundary re-targeting from old to new text node
//!   (bespoke per-method spec algorithm, NOT derivable from current 3 hooks)
//!
//! DEFERRED to future MutationObserver PR:
//! - `after_attribute_change` — needs `old_value` + `namespace` for §4.3.5;
//!   committing a wrong signature now is breaking-change risk
//!
//! `MutationHook` operates on light-tree mutations only. Shadow root
//! boundaries are tracked by consumers as needed (e.g. `LiveRangeRegistry`
//! per WHATWG §5.5).

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
    fn after_remove(&mut self, _node: Entity, _parent: Entity, _removed_index: usize) {}

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

    /// Called AFTER a Text / CData entity's `TextContent` changes.
    ///
    /// - `node`: the entity whose `TextContent` was rewritten.
    /// - `new_utf16_len`: the new UTF-16 length of the `TextContent`. WHATWG
    ///   DOM §5.5 "Set/Replace data steps" clamps Range boundaries on `node`
    ///   to `min(offset, new_utf16_len)`.
    ///
    /// Note: `splitText` boundary re-targeting from old to new text node is
    /// NOT covered by this hook — D-8 PR-A `LiveRangeRegistry` handles it
    /// inline within `Text.splitText` impl. Comment nodes are NOT covered by
    /// WHATWG §5.5 Range live-tracking, so `CommentData` writes do not fire
    /// this hook.
    fn after_text_change(&mut self, _node: Entity, _new_utf16_len: usize) {}
}
