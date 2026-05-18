//! Mutation events fired synchronously from `EcsDom` mutation primitives.
//!
//! Replaces the 7-method `MutationHook` trait (D-31 PR) with a single
//! [`MutationEvent`] enum + single-method [`MutationDispatcher`] trait.
//!
//! ## Why event-as-data + single-method dispatch?
//!
//! - **Trait extension cascade (lesson #237) dissolved**: new mutation
//!   kinds are enum variant additions, not new trait methods. Adding
//!   a variant requires the variant declaration + an emit site; the
//!   single trait method's match in each dispatcher impl needs at
//!   most a new arm (or `_ => {}` to ignore).
//! - **ECS-native**: event-as-data, single dispatch method per consumer.
//!   Behavior-pluralization (7 trait methods) → data-pluralization (7
//!   enum variants).
//! - **Synchronous drain at chokepoint**: preserves "next IDL read sees
//!   adjusted state" semantics (LiveRangeBridge / NodeIterator
//!   pre-removing-steps correctness; Bevy-style deferred drain would
//!   break this).
//! - **Single `Box<dyn MutationDispatcher>` in `EcsDom`** (NOT a Vec):
//!   typed composer ([`elidex_dom_api::ConsumerDispatcher`]) holds all
//!   consumers as typed fields, dispatches in declaration order.
//!   Compile-time-visible ordering, no subscriber-list runtime registry,
//!   monomorphized internal dispatch.
//!
//! ## Re-entry contract
//!
//! Consumers MUST NOT invoke `EcsDom` mutation primitives from within
//! [`MutationDispatcher::dispatch`] — nested dispatch would observe an
//! empty `dispatcher` slot (take-and-restore borrow pattern) and
//! silently no-op. Queue mutations for after-drain via per-consumer
//! deferred-action state if needed.
//!
//! ## Lifetime contract
//!
//! [`MutationEvent`] carries borrowed strings (only relevant for
//! [`MutationEvent::AttributeChange`]); the dispatcher MUST NOT retain
//! these references past the dispatch call. Borrows are released when
//! the chokepoint that fired the event returns.

use hecs::Entity;

use crate::EcsDom;

/// One mutation event delivered to the dispatcher.
///
/// ## Spec coverage (variant → WHATWG section)
///
/// - [`Self::Insert`] — fired AFTER a node is linked into its parent.
///   Source spec: **WHATWG DOM §4.2.3** "Mutation algorithms" (insert
///   algorithm); the synchronous fire corresponds to the §4.2
///   "insertion steps" extension hook defined in the §4.2 / §4.2.3
///   preamble paragraphs.
///
/// - [`Self::Remove`] — fired AFTER a node is detached from its
///   parent. Source spec: **WHATWG DOM §4.2.3** "Mutation algorithms"
///   (remove algorithm); the synchronous fire corresponds to the §4.2
///   "removing steps" extension hook. The inclusive descendant
///   snapshot is captured BEFORE detach (PR186 R2 #3 / R4 #1
///   additive-snapshot pattern); `dom: &EcsDom` access enables WHATWG
///   DOM §6.1 NodeIterator pre-removing-steps walk-forward through
///   the about-to-be-removed subtree.
///
/// - [`Self::TextChange`] — fired AFTER a Text / CData entity's
///   `TextContent` is rewritten as a single whole-string assignment
///   via [`EcsDom::set_text_data`]. Source spec: **WHATWG DOM §4.10**
///   Interface CharacterData "replace data" algorithm (whole-string
///   write path). Comment nodes use `CommentData` (NOT §4.10 covered)
///   and do NOT fire this event.
///
/// - [`Self::ReplaceData`] — fired AFTER an `appendData` / `insertData`
///   / `deleteData` / `replaceData` middle-splice via
///   [`EcsDom::replace_text_data`]. Source spec: **WHATWG DOM §4.10**
///   Interface CharacterData "replace data" algorithm middle-splice
///   path (steps 8-11 boundary-adjustment math).
///
/// - [`Self::SplitText`] — fired by `EcsDom::fire_split_text` AFTER
///   `new_node` is inserted as a sibling of `node` but BEFORE `node`'s
///   text is truncated. Source spec: **WHATWG DOM §4.11** Interface
///   Text "split a Text node" step 7. Boundary migration ordering
///   contract: consumer MUST migrate boundaries on `node` with
///   `off > offset` to `(new_node, off - offset)` BEFORE the
///   subsequent `set_text_data(node, head)` fires
///   [`Self::TextChange`].
///
/// - [`Self::NormalizeMerge`] — fired by `EcsDom::fire_normalize_merge`
///   AFTER `prev` has absorbed `merged_child`'s data but BEFORE
///   `merged_child` is detached. Source spec: **WHATWG DOM §4.4**
///   Interface Node `normalize()` step 6.4. Firing before detach lets
///   consumers migrate boundaries on `merged_child` to
///   `(prev, prev_old_len + off)` instead of collapsing via the
///   subsequent [`Self::Remove`].
///
/// - [`Self::AttributeChange`] — fired AFTER an attribute write at the
///   `EcsDom::set_attribute` / `attr_remove` chokepoints. Source spec:
///   **WHATWG DOM §4.3.2** "Queue a mutation record" step 5.1 +
///   MutationRecord shape per **WHATWG DOM §4.3.3** Interface
///   MutationRecord (`attributeName` / `attributeNamespace` /
///   `oldValue`). Attribute name ASCII case-insensitivity per **WHATWG
///   DOM §4.9** Interface Element `getAttribute` (case-insens match
///   for HTML documents).
pub enum MutationEvent<'a> {
    /// A node has been linked into a parent.
    ///
    /// - `node`: the newly-attached entity.
    /// - `parent`: the parent that received `node`.
    /// - `index`: light-tree exposed-sibling index, post-detach +
    ///   post-link. For `append_child` this equals the post-detach
    ///   child count of `parent`. For `insert_before(parent, new, ref)`
    ///   this equals `ref`'s post-detach index in parent. Per WHATWG
    ///   DOM §4.2.3 (insertion-steps boundary-adjustment), Range
    ///   boundaries at `(parent, offset)` where `offset > index` need
    ///   `+= 1` (strict comparison).
    Insert {
        node: Entity,
        parent: Entity,
        index: usize,
    },

    /// A node has been removed from its parent.
    ///
    /// - `node`: the removed entity (alive when fired; in the
    ///   `destroy_entity` path despawn happens immediately after this
    ///   variant returns, so component queries on `node` still work
    ///   inside the dispatch).
    /// - `parent`: the former parent (still alive).
    /// - `removed_index`: pre-removal index of `node` in `parent`'s
    ///   child list, captured BEFORE detach. Per WHATWG DOM §4.2.3
    ///   (remove algorithm) this is what Range live-tracking needs.
    /// - `descendants`: inclusive snapshot of `node` plus every
    ///   light-tree descendant, captured BEFORE any orphaning. Lets
    ///   the dispatcher decide whether a Range boundary container
    ///   falls inside the about-to-be-removed subtree without walking
    ///   parent chains that may be cleared by the time the dispatch
    ///   returns (PR186 R2 #3 / R4 #1 additive pattern).
    ///
    /// ## Tree shape at fire time
    ///
    /// Fire happens AFTER engine has detached `node` from `parent`:
    /// - `parent.children` no longer contains `node`.
    /// - `node`'s own parent / sibling links are cleared, but
    ///   `node.first_child` / `last_child` and descendant
    ///   parent-pointers remain intact (lesson #238 fire-before-
    ///   despawn invariant — only ECS despawn happens later).
    /// - `dom` reflects this post-detach, pre-despawn shape.
    Remove {
        node: Entity,
        parent: Entity,
        removed_index: usize,
        descendants: &'a [Entity],
    },

    /// A Text / CData entity's `TextContent` was rewritten as a
    /// single whole-string assignment.
    ///
    /// - `node`: the entity whose `TextContent` was rewritten.
    /// - `new_utf16_len`: new UTF-16 length of `TextContent`. WHATWG
    ///   DOM §4.10 CharacterData "replace data" clamps Range
    ///   boundaries on `node` to `min(offset, new_utf16_len)`.
    ///
    /// Comment nodes use `CommentData` (NOT covered by §4.10 Range
    /// live-tracking) and do NOT fire this event. Middle-splice
    /// operations fire [`Self::ReplaceData`] instead.
    TextChange { node: Entity, new_utf16_len: usize },

    /// An `appendData` / `insertData` / `deleteData` / `replaceData`
    /// splice on a Text / CData entity (WHATWG DOM §4.10 Interface
    /// CharacterData "replace data" steps 8-11 boundary adjustment).
    ///
    /// - `node`: the entity whose `TextContent` was spliced.
    /// - `offset_utf16`: the UTF-16 offset where the splice started.
    /// - `count_utf16`: the UTF-16 count removed at `offset`.
    /// - `new_data_len_utf16`: UTF-16 length of the inserted /
    ///   replacement string (0 for `deleteData`).
    ///
    /// Range live-tracking boundary adjustment per §4.10 step 8-11:
    /// - Boundary on `node` with `off ∈ [offset, offset + count]` →
    ///   collapse to `offset`.
    /// - Boundary on `node` with `off > offset + count` →
    ///   `off += new_data_len - count`.
    /// - Other boundaries unchanged.
    ReplaceData {
        node: Entity,
        offset_utf16: usize,
        count_utf16: usize,
        new_data_len_utf16: usize,
    },

    /// A `Text.splitText(offset)` operation (WHATWG DOM §4.11
    /// Interface Text "split a Text node" step 7).
    ///
    /// **Ordering invariant**: fired AFTER `new_node` is inserted as a
    /// sibling of `node` but BEFORE `node`'s text is truncated.
    /// Boundary-on-`node` boundaries with `off > offset` MUST be
    /// migrated to `(new_node, off - offset)` BEFORE the subsequent
    /// `set_text_data(node, head)` fires [`Self::TextChange`] (which
    /// would otherwise clamp those boundaries on `node` to `head_len`
    /// and destroy the offset needed for migration). Boundaries with
    /// `off == offset` stay on `node` at the truncated end per spec.
    ///
    /// Per §4.11 step 7:
    /// - Boundary on `node` with `off > offset` →
    ///   migrate to `(new_node, off - offset)` (spec strict-greater).
    /// - Boundary on `parent` with `idx > node_idx` → `idx += 1`
    ///   (spec §4.11 step 7.2; the [`Self::Insert`] fired by the
    ///   prior `insert_before` already handles `idx > node_idx + 1`
    ///   via strict-greater compare against the inserted-at index —
    ///   consumer MUST ONLY apply the missing `idx == node_idx + 1`
    ///   increment to avoid double-shifting).
    SplitText {
        node: Entity,
        new_node: Entity,
        offset_utf16: usize,
        parent: Option<Entity>,
        node_index: Option<usize>,
    },

    /// BEFORE the remove-merged-child step of `Node.normalize()`
    /// (WHATWG DOM §4.4 Interface Node `normalize()` step 6.4) on
    /// adjacent Text-node merge.
    ///
    /// **Ordering invariant**: fired AFTER `prev` has absorbed
    /// `merged_child`'s data but BEFORE `merged_child` is detached
    /// from its parent. Firing before detach lets consumers compute
    /// the migration without the subsequent [`Self::Remove`]
    /// collapsing the boundary to `(parent, child_idx)` instead.
    ///
    /// - `merged_child`: empty/redundant Text node about to be removed.
    /// - `prev`: Text node that absorbed `merged_child`'s data
    ///   (`prev`'s `TextContent` already reflects the merged string).
    /// - `prev_old_len_utf16`: UTF-16 length of `prev`'s data BEFORE
    ///   the merge (the migration offset shift).
    /// - `parent`: parent of `merged_child`, or `None` if no parent
    ///   was set (vacuous case).
    /// - `merged_child_index`: pre-removal index of `merged_child` in
    ///   `parent`'s child list, or `None` matched with `parent: None`.
    ///
    /// Range live-tracking boundary adjustment per §4.4 step 6.4:
    /// - Boundary on `merged_child` at `off` →
    ///   migrate to `(prev, prev_old_len + off)`.
    /// - Boundary on `parent` at exactly `child_idx` of `merged_child`
    ///   → migrate to `(prev, prev_old_len)` (the merged splice point).
    ///   The subsequent [`Self::Remove`] handles boundaries at
    ///   `off > child_idx` via the standard `-= 1` decrement —
    ///   consumer MUST NOT double-decrement those.
    NormalizeMerge {
        merged_child: Entity,
        prev: Entity,
        prev_old_len_utf16: usize,
        parent: Option<Entity>,
        merged_child_index: Option<usize>,
    },

    /// An attribute write at the `EcsDom::set_attribute` / `attr_remove`
    /// chokepoint (lesson #181 canonical write path).
    ///
    /// - `node`: element whose attribute changed.
    /// - `name`: local attribute name (case-preserving as stored).
    /// - `namespace`: attribute namespace URI, or `None` for
    ///   no-namespace attributes (HTML attribute namespace handling per
    ///   WHATWG DOM §4.9 Interface Element).
    /// - `old_value`: previous attribute value, or `None` if the
    ///   attribute was absent (insert case).
    /// - `new_value`: post-mutation attribute value, or `None` if the
    ///   attribute was just removed.
    ///
    /// Source spec: WHATWG DOM §4.3.2 "Queue a mutation record" step
    /// 5.1 + MutationRecord shape per WHATWG DOM §4.3.3 Interface
    /// MutationRecord.
    ///
    /// **Suppression contract** (per-consumer, NOT at EcsDom fire
    /// path): same-value `set_attribute` writes still fire this event
    /// because WHATWG DOM §4.3.2 requires same-value records be
    /// queued for MutationObserver consumers. Consumers that want to
    /// skip same-value processing (e.g., `BaseUrlMaintainer`
    /// idempotent bump suppression) apply the diff check inside their
    /// own `handle` body, NOT at the engine fire path.
    AttributeChange {
        node: Entity,
        name: &'a str,
        namespace: Option<&'a str>,
        old_value: Option<&'a str>,
        new_value: Option<&'a str>,
    },
}

/// Synchronous mutation event dispatcher.
///
/// Installed via [`EcsDom::set_mutation_dispatcher`]; called once per
/// fired event in registration order.
///
/// Production impl: `elidex_dom_api::ConsumerDispatcher` (typed
/// composer of `LiveRangeBridge` + `NodeIteratorAdjuster` +
/// `BaseUrlMaintainer`). Dispatch order = composer field declaration
/// order = compile-time-visible.
///
/// New mutation kinds are added as [`MutationEvent`] variant additions;
/// existing dispatcher / consumer code compile-time-ignores via the
/// `_` arm. This is the ECS-native replacement for the OO 7-method
/// `MutationHook` trait (D-31 PR; lesson #237 cascade structurally
/// dissolved).
///
/// `Send + Sync` is required because some Worker-context impls (future)
/// may transfer `EcsDom` across threads. `hecs::World` is `Send + Sync`,
/// so this adds no constraint beyond what `EcsDom` already permits.
pub trait MutationDispatcher: Send + Sync {
    /// Dispatch ONE event synchronously. Consumers (called by the
    /// dispatcher's impl) pattern-match on variants of interest;
    /// unmatched variants are silently ignored.
    ///
    /// `dom` is passed `&mut` so consumers (e.g. `BaseUrlMaintainer`
    /// in D-31) can mutate ECS components (e.g. `BaseFrozenUrl`,
    /// `DocumentBaseUrl`) at dispatch time.  Read-only consumers
    /// (e.g. `LiveRangeBridge`, `NodeIteratorAdjuster`) ignore the
    /// `&mut` and use it as `&` via auto-deref.
    ///
    /// Re-entry: invoking an `EcsDom` mutation primitive (which would
    /// fire a nested event) from within `dispatch` would observe an
    /// empty dispatcher slot (take-and-restore pattern) and silently
    /// no-op — consumers MUST queue such work for after-drain via
    /// per-consumer deferred-action state instead.
    fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom);
}
