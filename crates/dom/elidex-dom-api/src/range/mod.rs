//! `Range` implementation (WHATWG DOM §6).
//!
//! Represents a contiguous portion of the DOM tree. Provides methods for
//! manipulating the range boundaries and extracting/deleting content.

mod boundary;
pub mod live;
mod mutation;

pub use boundary::RangePointError;
pub use live::{LiveRangeBridge, LiveRangeRegistry, RangeId};

use elidex_ecs::{EcsDom, Entity, TextContent};

use crate::char_data::{utf16_len, utf16_to_byte_offset};

// ---------------------------------------------------------------------------
// Compare boundary points constants
// ---------------------------------------------------------------------------

/// Compare how constants for `compare_boundary_points`.
pub const START_TO_START: u16 = 0;
pub const START_TO_END: u16 = 1;
pub const END_TO_END: u16 = 2;
pub const END_TO_START: u16 = 3;

// ---------------------------------------------------------------------------
// Range
// ---------------------------------------------------------------------------

/// A DOM Range representing a contiguous portion of the document tree.
#[derive(Debug, Clone)]
pub struct Range {
    /// The node where the range starts.
    pub start_container: Entity,
    /// The offset within the start container.
    pub start_offset: usize,
    /// The node where the range ends.
    pub end_container: Entity,
    /// The offset within the end container.
    pub end_offset: usize,
    /// WHATWG DOM §4.4 — Range has a fixed associated document, set at
    /// construction. Used by `LiveRangeRegistry` as the dangling-collapse
    /// fallback target when both boundary containers become unreachable
    /// post-destroy (e.g. the entire boundary subtree was destroyed in a
    /// single mutation).
    ///
    /// Legacy two-arg [`Range::new`] sets `owner_document = node` for
    /// back-compat with tests that pre-date this field — callers that
    /// know which Document the range belongs to should prefer the new
    /// [`Range::new_with_owner`] / `Range::with_owner` factories.
    pub owner_document: Entity,
}

impl Range {
    /// Create a new range with both endpoints at offset 0 of the given node.
    ///
    /// Initialises `owner_document` to `node` — this is a back-compat
    /// shape used by in-crate tests / non-VM callers that do not have a
    /// distinct Document entity available. VM-side `document.createRange`
    /// and `new Range()` use [`Range::new_with_owner`] with the active
    /// document instead so dangling-collapse falls back to the real
    /// document.
    #[must_use]
    pub fn new(node: Entity) -> Self {
        Self {
            start_container: node,
            start_offset: 0,
            end_container: node,
            end_offset: 0,
            owner_document: node,
        }
    }

    /// Create a new range with both endpoints at `(node, 0)` and an
    /// explicit `owner_document` (WHATWG DOM §4.4 "node document" of the
    /// Range).
    #[must_use]
    pub fn new_with_owner(node: Entity, owner_document: Entity) -> Self {
        Self {
            start_container: node,
            start_offset: 0,
            end_container: node,
            end_offset: 0,
            owner_document,
        }
    }

    /// Returns `true` if start and end are the same container and offset.
    #[must_use]
    pub fn collapsed(&self) -> bool {
        self.start_container == self.end_container && self.start_offset == self.end_offset
    }

    /// Find the common ancestor container of start and end.
    #[must_use]
    pub fn common_ancestor_container(&self, dom: &EcsDom) -> Entity {
        // Collect ancestors of start_container.
        let mut ancestors = Vec::new();
        let mut node = self.start_container;
        ancestors.push(node);
        while let Some(parent) = dom.get_parent(node) {
            ancestors.push(parent);
            node = parent;
        }

        // Walk up from end_container to find first match.
        let mut node = self.end_container;
        loop {
            if ancestors.contains(&node) {
                return node;
            }
            match dom.get_parent(node) {
                Some(parent) => node = parent,
                None => return node,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Live Range updates (WHATWG DOM §5.5)
// ---------------------------------------------------------------------------

/// Adjust all live ranges when a child at `index` is removed from `parent`.
///
/// Per WHATWG DOM §5.5 "remove a node" steps 4-6, when a node is removed
/// any Range whose boundary point references the removed node OR an
/// **inclusive descendant** of it must be adjusted:
///
/// 1. If a boundary container is an inclusive descendant of `node`,
///    collapse that boundary to `(parent, index)`.
/// 2. If a boundary container is `parent` and its offset is greater than
///    `index`, decrement the offset.
///
/// `dom` is required to walk the descendant relationship for case (1).
///
/// **Calling order**: invoke either BEFORE `EcsDom::remove_child` /
/// `EcsDom::replace_child`, or from within a
/// [`MutationDispatcher`](elidex_ecs::MutationDispatcher) consumer
/// handling [`MutationEvent::Remove`](elidex_ecs::MutationEvent::Remove)
/// (which is the public-API dispatch point that fires once the node has
/// been detached but its subtree's parent links still point at it). The
/// descendant walk uses [`EcsDom::is_ancestor_or_self`], which walks
/// from `descendant`'s parent chain upward — that chain remains intact
/// at the `Remove` event because the engine only clears the removed
/// node's OWN parent / sibling links and leaves its `first_child` /
/// `last_child` (and the descendants' `parent`) alone.
///
/// `destroy_entity` differs: it orphans all children before firing the
/// hook, so the descendant walk does NOT find them — consumers must
/// lazy-collapse dangling boundaries on next access per the
/// `destroy_entity` contract.
pub fn adjust_ranges_for_removal(
    ranges: &mut [Range],
    node: Entity,
    parent: Entity,
    index: usize,
    dom: &EcsDom,
) {
    for range in ranges.iter_mut() {
        // Adjust start boundary. `is_ancestor_or_self` is inclusive, so the
        // direct-equality case (`container == node`) is subsumed.
        if dom.is_ancestor_or_self(node, range.start_container) {
            range.start_container = parent;
            range.start_offset = index;
        } else if range.start_container == parent && range.start_offset > index {
            range.start_offset -= 1;
        }

        // Adjust end boundary.
        if dom.is_ancestor_or_self(node, range.end_container) {
            range.end_container = parent;
            range.end_offset = index;
        } else if range.end_container == parent && range.end_offset > index {
            range.end_offset -= 1;
        }
    }
}

/// Snapshot-driven removal adjustment per WHATWG DOM §5.5 remove
/// step 4-6, identical semantics to [`adjust_ranges_for_removal`] but
/// driven by a caller-provided `descendants` set instead of an
/// `EcsDom` ancestor walk.
///
/// PR186 R2 #3: `destroy_entity` orphans children BEFORE firing the
/// `MutationEvent::Remove` event, so an `is_ancestor_or_self`-based
/// descendant walk at the consumer side would miss already-orphaned
/// descendants.  The engine pre-snapshots the light-tree inclusive-
/// descendant set before orphaning and hands it to the consumer; the
/// consumer ([`crate::LiveRangeBridge`], composed via
/// [`crate::ConsumerDispatcher`]) uses this snapshot membership check
/// instead of a tree walk.
///
/// `descendants` MUST include `node` itself (inclusive descendants) —
/// the engine's `collect_inclusive_descendants` helper guarantees
/// this.
///
/// `pub(crate)` because the only driver is the in-crate
/// [`crate::LiveRangeBridge`] consumer.
pub(crate) fn adjust_ranges_for_removal_snapshot(
    ranges: &mut [Range],
    descendants: &[Entity],
    parent: Entity,
    index: usize,
) {
    for range in ranges.iter_mut() {
        if descendants.contains(&range.start_container) {
            range.start_container = parent;
            range.start_offset = index;
        } else if range.start_container == parent && range.start_offset > index {
            range.start_offset -= 1;
        }

        if descendants.contains(&range.end_container) {
            range.end_container = parent;
            range.end_offset = index;
        } else if range.end_container == parent && range.end_offset > index {
            range.end_offset -= 1;
        }
    }
}

/// Adjust all live ranges when a child is inserted at `index` in `parent`.
///
/// Per WHATWG DOM §5.5 "insert a node" steps, Range boundaries at
/// `(parent, offset)` where `offset > index` need their offset
/// incremented by 1 (strict comparison; boundaries at exactly `index`
/// stay where they are so a Range collapsed at the insertion point
/// continues to bracket the inserted node from the start).
///
/// Live consumer (`LiveRangeRegistry` in `range/live.rs`) of the
/// `after_insert` hook. `pub(crate)` because the only external entry
/// point is the [`crate::range::live::LiveRangeRegistry`] consumer; no
/// other crate is expected to drive Range adjustment directly.
#[allow(dead_code)] // wired up by range::live::LiveRangeRegistry below in the same PR
pub(crate) fn adjust_ranges_for_insertion(ranges: &mut [Range], parent: Entity, index: usize) {
    for range in ranges.iter_mut() {
        if range.start_container == parent && range.start_offset > index {
            range.start_offset += 1;
        }
        if range.end_container == parent && range.end_offset > index {
            range.end_offset += 1;
        }
    }
}

/// Adjust all live ranges when text data changes in a character data node.
///
/// Per WHATWG DOM §5.5 step for "replace data", if a boundary container is
/// the modified node and its offset exceeds the new length, clamp it.
pub fn adjust_ranges_for_text_change(ranges: &mut [Range], node: Entity, new_utf16_len: usize) {
    for range in ranges.iter_mut() {
        if range.start_container == node && range.start_offset > new_utf16_len {
            range.start_offset = new_utf16_len;
        }
        if range.end_container == node && range.end_offset > new_utf16_len {
            range.end_offset = new_utf16_len;
        }
    }
}

/// Adjust all live ranges when a `CharacterData` middle-splice
/// (`appendData` / `insertData` / `deleteData` / `replaceData`) mutates
/// `node`'s data (WHATWG DOM §4.10 "replace data" steps 8-11).
///
/// For each boundary whose container is `node`:
/// - If `off ∈ [offset, offset + count]` → collapse boundary to `offset`
///   (boundary fell inside the spliced region; clamp to splice start).
/// - If `off > offset + count` → `off += new_data_len - count` (boundary
///   past the spliced region; shift by net length delta).
/// - Otherwise: boundary lies before the splice region; unchanged.
///
/// Boundaries on other containers are unaffected.
///
/// `pub(crate)` per the same rationale as
/// [`adjust_ranges_for_insertion`] — `LiveRangeRegistry` is the only
/// driver.
#[allow(dead_code)] // wired up by range::live::LiveRangeRegistry below in the same PR
pub(crate) fn adjust_ranges_for_replace_data(
    ranges: &mut [Range],
    node: Entity,
    offset: usize,
    count: usize,
    new_data_len: usize,
) {
    let end = offset.saturating_add(count);
    let adjust = |container: Entity, off: usize| -> usize {
        if container != node {
            return off;
        }
        if off >= offset && off <= end {
            offset
        } else if off > end {
            // Spec §4.10 step 9-11: pure arithmetic `off += new_data_len -
            // count`, no clamp. Split into grow / shrink branches to use
            // saturating ops on `usize`. Precondition `off > offset +
            // count` guarantees the shrink arm cannot underflow past
            // `offset + new_data_len`, so saturating_sub returning 0
            // would require `offset + count < count - new_data_len` i.e.
            // `offset < -new_data_len` which is impossible for unsigned
            // offsets — no clamp is needed to preserve the
            // past-the-spliced-region invariant.
            if new_data_len >= count {
                off.saturating_add(new_data_len - count)
            } else {
                off.saturating_sub(count - new_data_len)
            }
        } else {
            off
        }
    };
    for range in ranges.iter_mut() {
        range.start_offset = adjust(range.start_container, range.start_offset);
        range.end_offset = adjust(range.end_container, range.end_offset);
    }
}

/// Adjust all live ranges when a `Text.splitText(offset)` splits `node`
/// into `node` (head `[..offset]`) and `new_node` (tail `[offset..]`)
/// per WHATWG DOM §4.10 "split a Text node" step 8.
///
/// **Ordering invariant** (mirrors
/// [`MutationEvent::SplitText`](elidex_ecs::MutationEvent::SplitText)
/// doc): MUST run BEFORE the caller's subsequent
/// `set_text_data(node, head)` fires `MutationEvent::TextChange` —
/// otherwise the truncate-clamp on `node` destroys boundary offsets
/// needed for migration to `new_node`.
///
/// For each boundary:
/// - On `node` at `off > offset` → migrate to `(new_node, off - offset)`.
/// - On parent of `node` (i.e. `node`'s parent in the tree) at idx `≥
///   node_idx + 1` → `idx += 1` (parent now has one extra child between
///   `node` and `node_idx + 1`).
///
/// `node_index` is the pre-split index of `node` in its parent — caller
/// computes it via [`elidex_ecs::EcsDom::index_in_parent`] (or omits
/// the parent-side adjustment if `node` has no parent: orphan split
/// cannot affect a parent boundary).
#[allow(dead_code)] // wired up by range::live::LiveRangeRegistry below in the same PR
pub(crate) fn adjust_ranges_for_split_text(
    ranges: &mut [Range],
    node: Entity,
    new_node: Entity,
    offset: usize,
    parent: Option<Entity>,
    node_index: Option<usize>,
) {
    for range in ranges.iter_mut() {
        // WHATWG §4.10 step 7.2 / 7.3: strict `off > offset` migrates to
        // `(new_node, off - offset)`. The equality case (`off == offset`)
        // stays on `node` at its new end — a Range collapsed at the
        // split point is preserved on the original node, NOT migrated to
        // `(new_node, 0)`. This matches Chrome / Firefox observable
        // behaviour and the spec text as of 2026-05-14.
        if range.start_container == node && range.start_offset > offset {
            range.start_container = new_node;
            range.start_offset -= offset;
        }
        if range.end_container == node && range.end_offset > offset {
            range.end_container = new_node;
            range.end_offset -= offset;
        }
        if let (Some(parent), Some(node_idx)) = (parent, node_index) {
            if range.start_container == parent && range.start_offset > node_idx {
                range.start_offset += 1;
            }
            if range.end_container == parent && range.end_offset > node_idx {
                range.end_offset += 1;
            }
        }
    }
}

/// Adjust all live ranges when `Node.normalize()` merges `merged_child`
/// into `prev` (WHATWG DOM §4.5 "normalize" step 6.4).
///
/// **Ordering invariant** (mirrors
/// [`MutationEvent::NormalizeMerge`](elidex_ecs::MutationEvent::NormalizeMerge)
/// doc): MUST run BEFORE the caller's subsequent
/// `remove_child(parent, merged_child)` fires `MutationEvent::Remove` —
/// otherwise the boundary on `merged_child` is collapsed to
/// `(parent, child_idx)` by remove-step semantics and the migration to
/// `(prev, prev_old_len + off)` is lost.
///
/// `prev_old_len` is the UTF-16 length of `prev` BEFORE absorbing
/// `merged_child`'s data; `merged_child_index` is the pre-removal index
/// of `merged_child` in its parent (used to migrate boundaries on the
/// parent that point at `merged_child`'s slot).
///
/// For each boundary:
/// - On `merged_child` at `off` → migrate to
///   `(prev, prev_old_len + off)`.
/// - On parent of `merged_child` at `idx == merged_child_index` →
///   migrate to `(prev, prev_old_len)` (the merge splice point).
/// - On parent at `idx > merged_child_index` → `idx -= 1` (parent loses
///   one child).
#[allow(dead_code)] // wired up by range::live::LiveRangeRegistry below in the same PR
pub(crate) fn adjust_ranges_for_normalize_merge(
    ranges: &mut [Range],
    merged_child: Entity,
    prev: Entity,
    prev_old_len: usize,
    parent: Option<Entity>,
    merged_child_index: Option<usize>,
) {
    for range in ranges.iter_mut() {
        if range.start_container == merged_child {
            range.start_container = prev;
            range.start_offset += prev_old_len;
        }
        if range.end_container == merged_child {
            range.end_container = prev;
            range.end_offset += prev_old_len;
        }
        if let (Some(parent), Some(idx)) = (parent, merged_child_index) {
            if range.start_container == parent {
                match range.start_offset.cmp(&idx) {
                    std::cmp::Ordering::Equal => {
                        range.start_container = prev;
                        range.start_offset = prev_old_len;
                    }
                    std::cmp::Ordering::Greater => range.start_offset -= 1,
                    std::cmp::Ordering::Less => {}
                }
            }
            if range.end_container == parent {
                match range.end_offset.cmp(&idx) {
                    std::cmp::Ordering::Equal => {
                        range.end_container = prev;
                        range.end_offset = prev_old_len;
                    }
                    std::cmp::Ordering::Greater => range.end_offset -= 1,
                    std::cmp::Ordering::Less => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a UTF-16 offset to a byte offset, clamping to string length.
///
/// Range offsets are specified in UTF-16 code units per WHATWG DOM §5.
/// This helper converts to byte offsets for Rust string slicing, clamping
/// to string boundaries if the offset exceeds the string length.
fn utf16_offset_to_byte_clamped(s: &str, utf16_offset: usize) -> usize {
    let len16 = utf16_len(s);
    let clamped = utf16_offset.min(len16);
    utf16_to_byte_offset(s, clamped).unwrap_or(s.len())
}

/// Get the index of a child within its parent.
fn child_index(parent: Entity, child: Entity, dom: &EcsDom) -> usize {
    dom.children_iter(parent)
        .position(|c| c == child)
        .unwrap_or(0)
}

/// Get the "length" of a node (UTF-16 code unit count for text, child count otherwise).
///
/// Per WHATWG DOM §5, text node lengths are measured in UTF-16 code units.
pub fn node_length(node: Entity, dom: &EcsDom) -> usize {
    if let Ok(tc) = dom.world().get::<&TextContent>(node) {
        return utf16_len(&tc.0);
    }
    dom.children_iter(node).count()
}

/// Compare two boundary points per DOM spec §5.2.
/// Returns -1 (a before b), 0 (equal), or 1 (a after b).
pub(crate) fn compare_points(
    a_container: Entity,
    a_offset: usize,
    b_container: Entity,
    b_offset: usize,
    dom: &EcsDom,
) -> i8 {
    if a_container == b_container {
        return match a_offset.cmp(&b_offset) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
    }

    // If A is an ancestor of B, compare A's offset against the child index
    // of B's ancestor that is a direct child of A.
    if dom.is_ancestor_or_self(a_container, b_container) {
        let child_index = child_index_in_ancestor(b_container, a_container, dom);
        return if a_offset <= child_index { -1 } else { 1 };
    }

    // If B is an ancestor of A, compare the child index of A's ancestor
    // that is a direct child of B against B's offset.
    if dom.is_ancestor_or_self(b_container, a_container) {
        let child_index = child_index_in_ancestor(a_container, b_container, dom);
        return if child_index < b_offset { -1 } else { 1 };
    }

    // Neither is ancestor — use tree order of the containers.
    match dom.tree_order_cmp(a_container, b_container) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Find the child index of `descendant`'s ancestor that is a direct child of `ancestor`.
///
/// Walks from `descendant` up to `ancestor`, then counts preceding siblings of the
/// child of `ancestor` on that path.
fn child_index_in_ancestor(descendant: Entity, ancestor: Entity, dom: &EcsDom) -> usize {
    let mut current = descendant;
    loop {
        let Some(parent) = dom.get_parent(current) else {
            return 0;
        };
        if parent == ancestor {
            // Count siblings before `current`.
            let mut index = 0;
            let mut sib = dom.get_first_child(parent);
            while let Some(s) = sib {
                if s == current {
                    return index;
                }
                index += 1;
                sib = dom.get_next_sibling(s);
            }
            return index;
        }
        current = parent;
    }
}

/// Global pre-order next (not confined to a root).
fn next_in_preorder_global(current: Entity, dom: &EcsDom) -> Option<Entity> {
    if let Some(child) = dom.get_first_child(current) {
        return Some(child);
    }
    let mut node = current;
    loop {
        if let Some(sib) = dom.get_next_sibling(node) {
            return Some(sib);
        }
        node = dom.get_parent(node)?;
    }
}

#[cfg(test)]
mod tests;
