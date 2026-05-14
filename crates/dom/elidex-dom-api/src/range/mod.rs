//! `Range` implementation (WHATWG DOM §6).
//!
//! Represents a contiguous portion of the DOM tree. Provides methods for
//! manipulating the range boundaries and extracting/deleting content.

mod boundary;
pub mod live;
mod mutation;

pub use live::{Bridge, LiveRangeRegistry, RangeId};

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
/// `EcsDom::replace_child`, or from within
/// [`MutationHook::after_remove`](elidex_ecs::MutationHook::after_remove)
/// (which is the public-API hook point that fires once the node has been
/// detached but its subtree's parent links still point at it). The
/// descendant walk uses [`EcsDom::is_ancestor_or_self`], which walks
/// from `descendant`'s parent chain upward — that chain remains intact
/// at the `after_remove` callback because the engine only clears the
/// removed node's OWN parent / sibling links and leaves its
/// `first_child` / `last_child` (and the descendants' `parent`) alone.
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
/// [`elidex_ecs::MutationHook::after_split_text`] doc): MUST run BEFORE
/// the caller's subsequent `set_text_data(node, head)` fires
/// `after_text_change` — otherwise the truncate-clamp on `node`
/// destroys boundary offsets needed for migration to `new_node`.
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
/// [`elidex_ecs::MutationHook::after_normalize_merge`] doc): MUST run
/// BEFORE the caller's subsequent `remove_child(parent, merged_child)`
/// fires `after_remove` — otherwise the boundary on `merged_child` is
/// collapsed to `(parent, child_idx)` by remove-step semantics and the
/// migration to `(prev, prev_old_len + off)` is lost.
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
fn node_length(node: Entity, dom: &EcsDom) -> usize {
    if let Ok(tc) = dom.world().get::<&TextContent>(node) {
        return utf16_len(&tc.0);
    }
    dom.children_iter(node).count()
}

/// Compare two boundary points per DOM spec §5.2.
/// Returns -1 (a before b), 0 (equal), or 1 (a after b).
fn compare_points(
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom, TextContent};

    fn build_range_tree() -> (EcsDom, Entity, Entity, Entity, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("Hello");
        let span = dom.create_element("span", Attributes::default());
        let t2 = dom.create_text(" World");

        dom.append_child(root, t1);
        dom.append_child(root, span);
        dom.append_child(span, t2);

        (dom, root, t1, span, t2)
    }

    #[test]
    fn range_defaults_collapsed() {
        let (dom, root, _, _, _) = build_range_tree();
        let range = Range::new(root);
        assert!(range.collapsed());
        assert_eq!(range.common_ancestor_container(&dom), root);
    }

    #[test]
    fn range_set_start_end() {
        let (dom, _root, t1, _span, t2) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 2);
        range.set_end(t2, 3);
        assert!(!range.collapsed());
        assert_eq!(range.start_offset, 2);
        assert_eq!(range.end_offset, 3);
        // Common ancestor should be root (div).
        let _ca = range.common_ancestor_container(&dom);
    }

    #[test]
    fn range_collapsed() {
        let (_dom, root, _, _, _) = build_range_tree();
        let mut range = Range::new(root);
        range.set_start(root, 0);
        range.set_end(root, 0);
        assert!(range.collapsed());
    }

    #[test]
    fn range_common_ancestor() {
        let (dom, root, t1, _span, t2) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 0);
        range.set_end(t2, 3);
        assert_eq!(range.common_ancestor_container(&dom), root);
    }

    #[test]
    fn range_select_node_contents() {
        let (dom, _root, t1, _, _) = build_range_tree();
        let mut range = Range::new(t1);
        range.select_node_contents(t1, &dom);
        assert_eq!(range.start_offset, 0);
        assert_eq!(range.end_offset, 5); // "Hello" length
    }

    #[test]
    fn range_clone() {
        let (_dom, root, _, _, _) = build_range_tree();
        let mut range = Range::new(root);
        range.set_start(root, 1);
        range.set_end(root, 2);
        let cloned = range.clone_range();
        assert_eq!(cloned.start_offset, 1);
        assert_eq!(cloned.end_offset, 2);
    }

    #[test]
    fn range_compare_boundary_points() {
        let (dom, root, _t1, _span, _t2) = build_range_tree();
        let mut r1 = Range::new(root);
        r1.set_start(root, 0);
        r1.set_end(root, 2);

        let mut r2 = Range::new(root);
        r2.set_start(root, 1);
        r2.set_end(root, 3);

        assert_eq!(r1.compare_boundary_points(START_TO_START, &r2, &dom), -1);
    }

    #[test]
    fn range_to_string_same_text_node() {
        let (dom, _root, t1, _, _) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 1);
        range.set_end(t1, 4);
        assert_eq!(range.to_string(&dom), "ell");
    }

    #[test]
    fn range_delete_contents_same_text() {
        let (mut dom, _root, t1, _, _) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 1);
        range.set_end(t1, 4);
        range.delete_contents(&mut dom);

        let tc = dom.world().get::<&TextContent>(t1).unwrap();
        assert_eq!(tc.0, "Ho");
        assert!(range.collapsed());
    }

    #[test]
    fn range_delete_contents_splits_text() {
        let (mut dom, _root, t1, _, _) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 2);
        range.set_end(t1, 4);
        range.delete_contents(&mut dom);

        let tc = dom.world().get::<&TextContent>(t1).unwrap();
        assert_eq!(tc.0, "Heo");
    }

    #[test]
    fn range_extract_contents() {
        let (mut dom, _root, t1, _, _) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 1);
        range.set_end(t1, 4);
        let frag = range.extract_contents(&mut dom);

        // Fragment should contain "ell".
        let children: Vec<_> = dom.children_iter(frag).collect();
        assert_eq!(children.len(), 1);
        let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(tc.0, "ell");

        // Original text should be "Ho".
        let tc = dom.world().get::<&TextContent>(t1).unwrap();
        assert_eq!(tc.0, "Ho");
    }

    #[test]
    fn range_to_string_utf16_offsets() {
        // Test that Range offsets are treated as UTF-16 code units.
        // U+1F600 is 4 bytes in UTF-8 but 2 UTF-16 code units.
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let t = dom.create_text("A\u{1F600}B"); // "A<emoji>B" = 3 chars, 4 UTF-16 units
        dom.append_child(root, t);

        let mut range = Range::new(t);
        // UTF-16: A(1) + surrogate pair(2) + B(1) = 4 units
        // offset 1..3 should extract the emoji (surrogate pair)
        range.set_start(t, 1);
        range.set_end(t, 3);
        assert_eq!(range.to_string(&dom), "\u{1F600}");

        // offset 3..4 should extract "B"
        range.set_start(t, 3);
        range.set_end(t, 4);
        assert_eq!(range.to_string(&dom), "B");
    }

    #[test]
    fn range_delete_utf16_offsets() {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let t = dom.create_text("A\u{1F600}B");
        dom.append_child(root, t);

        let mut range = Range::new(t);
        range.set_start(t, 1);
        range.set_end(t, 3);
        range.delete_contents(&mut dom);

        let tc = dom.world().get::<&TextContent>(t).unwrap();
        assert_eq!(tc.0, "AB");
    }

    #[test]
    fn range_select_node_contents_utf16() {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let t = dom.create_text("A\u{1F600}B");
        dom.append_child(root, t);

        let mut range = Range::new(t);
        range.select_node_contents(t, &dom);
        assert_eq!(range.start_offset, 0);
        // UTF-16 length: A(1) + surrogate(2) + B(1) = 4
        assert_eq!(range.end_offset, 4);
    }

    #[test]
    fn adjust_ranges_for_removal_basic() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let c0 = dom.create_element("a", Attributes::default());
        let c1 = dom.create_element("b", Attributes::default());
        let c2 = dom.create_element("c", Attributes::default());
        dom.append_child(parent, c0);
        dom.append_child(parent, c1);
        dom.append_child(parent, c2);

        let mut r = Range::new(parent);
        r.set_start(parent, 1);
        r.set_end(parent, 3);

        let mut ranges = [r];
        // Remove child at index 1 (c1).
        super::adjust_ranges_for_removal(&mut ranges, c1, parent, 1, &dom);

        // start_offset was 1 (== index), not > index, so unchanged.
        assert_eq!(ranges[0].start_offset, 1);
        // end_offset was 3 (> index 1), so decremented to 2.
        assert_eq!(ranges[0].end_offset, 2);
    }

    #[test]
    fn adjust_ranges_for_removal_container_is_removed() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_text("hello");
        dom.append_child(parent, child);

        let mut r = Range::new(child);
        r.set_start(child, 2);
        r.set_end(child, 4);

        let mut ranges = [r];
        super::adjust_ranges_for_removal(&mut ranges, child, parent, 0, &dom);

        // Both boundaries should collapse to (parent, 0).
        assert_eq!(ranges[0].start_container, parent);
        assert_eq!(ranges[0].start_offset, 0);
        assert_eq!(ranges[0].end_container, parent);
        assert_eq!(ranges[0].end_offset, 0);
    }

    #[test]
    fn adjust_ranges_for_removal_descendant_container_collapses() {
        // WHATWG DOM §5.5 "remove a node" steps 4-6: boundaries whose
        // container is an inclusive descendant of the removed node must
        // collapse to (parent, index), not just direct-equality containers.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let section = dom.create_element("section", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        let inner_text = dom.create_text("hello");
        dom.append_child(parent, section);
        dom.append_child(section, p);
        dom.append_child(p, inner_text);

        // Range boundaries sit on inner_text (a descendant of `section`).
        let mut r = Range::new(inner_text);
        r.set_start(inner_text, 2);
        r.set_end(inner_text, 4);

        let mut ranges = [r];
        super::adjust_ranges_for_removal(&mut ranges, section, parent, 0, &dom);

        // Both boundaries must collapse to (parent, 0).
        assert_eq!(ranges[0].start_container, parent);
        assert_eq!(ranges[0].start_offset, 0);
        assert_eq!(ranges[0].end_container, parent);
        assert_eq!(ranges[0].end_offset, 0);
    }

    #[test]
    fn adjust_ranges_for_insertion_increments_strict_greater() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let c0 = dom.create_element("a", Attributes::default());
        let c1 = dom.create_element("b", Attributes::default());
        dom.append_child(parent, c0);
        dom.append_child(parent, c1);

        // Boundaries at offset 1 and 2 in `parent`. Inserting at index 1
        // shifts only the offset > 1, leaving offset == 1 in place.
        let mut r = Range::new(parent);
        r.set_start(parent, 1);
        r.set_end(parent, 2);
        let mut ranges = [r];
        super::adjust_ranges_for_insertion(&mut ranges, parent, 1);

        assert_eq!(ranges[0].start_offset, 1);
        assert_eq!(ranges[0].end_offset, 3);
    }

    #[test]
    fn adjust_ranges_for_insertion_leaves_other_containers_alone() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let other = dom.create_element("section", Attributes::default());
        let mut r = Range::new(parent);
        r.set_start(other, 0);
        r.set_end(other, 5);

        let mut ranges = [r];
        super::adjust_ranges_for_insertion(&mut ranges, parent, 0);

        assert_eq!(ranges[0].start_container, other);
        assert_eq!(ranges[0].start_offset, 0);
        assert_eq!(ranges[0].end_container, other);
        assert_eq!(ranges[0].end_offset, 5);
    }

    #[test]
    fn adjust_ranges_for_replace_data_collapse_inside_splice() {
        // WHATWG §4.10 "replace data" step 8: boundary `off ∈ [offset,
        // offset+count]` collapses to `offset`. Test: text="hello", replace
        // (offset=1, count=3) with "XYZ"; boundary at off=2 (inside region)
        // → collapses to 1.
        let mut dom = EcsDom::new();
        let t = dom.create_text("hello");

        let mut r = Range::new(t);
        r.set_start(t, 2);
        r.set_end(t, 3);
        let mut ranges = [r];
        super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 3);

        // Both boundaries fall inside [1, 4] → collapse to 1.
        assert_eq!(ranges[0].start_offset, 1);
        assert_eq!(ranges[0].end_offset, 1);
    }

    #[test]
    fn adjust_ranges_for_replace_data_shift_past_splice() {
        // WHATWG §4.10 step 9: boundary `off > offset+count` shifts by
        // `new_data_len - count`. Replace (offset=1, count=3, new_data=3)
        // → boundary at off=5 stays at 5 (delta=0).
        // Replace (offset=1, count=3, new_data=5) → boundary at off=5
        // shifts to 7 (delta=+2).
        let mut dom = EcsDom::new();
        let t = dom.create_text("aaaaa");

        let mut r = Range::new(t);
        r.set_start(t, 5);
        r.set_end(t, 5);
        let mut ranges = [r.clone()];
        super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 3);
        assert_eq!(ranges[0].start_offset, 5);
        assert_eq!(ranges[0].end_offset, 5);

        let mut ranges = [r.clone()];
        super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 5);
        assert_eq!(ranges[0].start_offset, 7);
        assert_eq!(ranges[0].end_offset, 7);

        // Net-deletion: replace (offset=1, count=3, new_data=0) →
        // boundary at off=5 shifts to 2 (delta=-3).
        let mut ranges = [r];
        super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 0);
        assert_eq!(ranges[0].start_offset, 2);
        assert_eq!(ranges[0].end_offset, 2);
    }

    #[test]
    fn adjust_ranges_for_replace_data_before_splice_unchanged() {
        let mut dom = EcsDom::new();
        let t = dom.create_text("hello");

        let mut r = Range::new(t);
        r.set_start(t, 0);
        r.set_end(t, 0);
        let mut ranges = [r];
        super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 3);

        assert_eq!(ranges[0].start_offset, 0);
        assert_eq!(ranges[0].end_offset, 0);
    }

    #[test]
    fn adjust_ranges_for_replace_data_other_container_unchanged() {
        let mut dom = EcsDom::new();
        let t = dom.create_text("hello");
        let other = dom.create_text("world");

        let mut r = Range::new(other);
        r.set_start(other, 3);
        r.set_end(other, 3);
        let mut ranges = [r];
        super::adjust_ranges_for_replace_data(&mut ranges, t, 1, 3, 5);

        assert_eq!(ranges[0].start_container, other);
        assert_eq!(ranges[0].start_offset, 3);
    }

    #[test]
    fn adjust_ranges_for_split_text_migrates_past_offset() {
        // WHATWG §4.10 "split text" step 8: boundary on node at off >
        // offset migrates to (new_node, off - offset).
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let node = dom.create_text("hello world");
        let new_node = dom.create_text("");
        dom.append_child(parent, node);
        dom.append_child(parent, new_node);

        let mut r = Range::new(node);
        r.set_start(node, 3); // "hel|lo world"
        r.set_end(node, 8); // "hello wo|rld"
        let mut ranges = [r];
        // split at offset 5 ("hello" | " world").
        super::adjust_ranges_for_split_text(&mut ranges, node, new_node, 5, Some(parent), Some(0));

        // start_offset 3 ≤ 5 → stays on `node`.
        assert_eq!(ranges[0].start_container, node);
        assert_eq!(ranges[0].start_offset, 3);
        // end_offset 8 > 5 → migrates to (new_node, 3).
        assert_eq!(ranges[0].end_container, new_node);
        assert_eq!(ranges[0].end_offset, 3);
    }

    #[test]
    fn adjust_ranges_for_split_text_parent_boundary_increments() {
        // splitText adds one child between node and node_idx+1; parent
        // boundaries at idx > node_idx → +1.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let n0 = dom.create_text("hello");
        let n1 = dom.create_text("world");
        dom.append_child(parent, n0);
        dom.append_child(parent, n1);
        let new_node = dom.create_text("tail");
        dom.append_child(parent, new_node);

        // Splitting n0 (at index 0) inserts new_node at index 1; boundary
        // on parent at offset > 0 increments.
        let mut r = Range::new(parent);
        r.set_start(parent, 1);
        r.set_end(parent, 2);
        let mut ranges = [r];
        super::adjust_ranges_for_split_text(&mut ranges, n0, new_node, 3, Some(parent), Some(0));

        assert_eq!(ranges[0].start_offset, 2);
        assert_eq!(ranges[0].end_offset, 3);
    }

    #[test]
    fn adjust_ranges_for_split_text_orphan_node_skips_parent() {
        // No parent → only the node-side migration runs.
        let mut dom = EcsDom::new();
        let node = dom.create_text("hello");
        let new_node = dom.create_text("");

        let mut r = Range::new(node);
        r.set_start(node, 4);
        r.set_end(node, 4);
        let mut ranges = [r];
        super::adjust_ranges_for_split_text(&mut ranges, node, new_node, 2, None, None);

        assert_eq!(ranges[0].start_container, new_node);
        assert_eq!(ranges[0].start_offset, 2);
    }

    #[test]
    fn adjust_ranges_for_normalize_merge_migrates_merged_child() {
        // WHATWG §4.5 step 6.4: boundary on merged_child at off migrates to
        // (prev, prev_old_len + off). prev_old_len = 5 ("hello").
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let prev = dom.create_text("helloworld"); // post-merge state
        let merged = dom.create_text(""); // post-merge empty, pre-detach
        dom.append_child(parent, prev);
        dom.append_child(parent, merged);

        let mut r = Range::new(merged);
        r.set_start(merged, 2);
        r.set_end(merged, 4);
        let mut ranges = [r];
        super::adjust_ranges_for_normalize_merge(
            &mut ranges,
            merged,
            prev,
            5,
            Some(parent),
            Some(1),
        );

        // Boundary migrated to (prev, 5 + off).
        assert_eq!(ranges[0].start_container, prev);
        assert_eq!(ranges[0].start_offset, 7);
        assert_eq!(ranges[0].end_container, prev);
        assert_eq!(ranges[0].end_offset, 9);
    }

    #[test]
    fn adjust_ranges_for_normalize_merge_parent_boundary_at_merged_idx() {
        // Parent boundary AT merged_child's index migrates to (prev,
        // prev_old_len) — the merge splice point.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let prev = dom.create_text("helloworld");
        let merged = dom.create_text("");
        dom.append_child(parent, prev);
        dom.append_child(parent, merged);

        let mut r = Range::new(parent);
        r.set_start(parent, 1);
        r.set_end(parent, 1);
        let mut ranges = [r];
        super::adjust_ranges_for_normalize_merge(
            &mut ranges,
            merged,
            prev,
            5,
            Some(parent),
            Some(1),
        );

        assert_eq!(ranges[0].start_container, prev);
        assert_eq!(ranges[0].start_offset, 5);
        assert_eq!(ranges[0].end_container, prev);
        assert_eq!(ranges[0].end_offset, 5);
    }

    #[test]
    fn adjust_ranges_for_normalize_merge_parent_boundary_past_merged_idx() {
        // Parent boundary AT idx > merged_child_index → decrement (parent
        // loses one child).
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let prev = dom.create_text("helloworld");
        let merged = dom.create_text("");
        let trailing = dom.create_element("span", Attributes::default());
        dom.append_child(parent, prev);
        dom.append_child(parent, merged);
        dom.append_child(parent, trailing);

        let mut r = Range::new(parent);
        r.set_start(parent, 3);
        r.set_end(parent, 3);
        let mut ranges = [r];
        super::adjust_ranges_for_normalize_merge(
            &mut ranges,
            merged,
            prev,
            5,
            Some(parent),
            Some(1),
        );

        assert_eq!(ranges[0].start_offset, 2);
        assert_eq!(ranges[0].end_offset, 2);
    }

    #[test]
    fn adjust_ranges_for_text_change() {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let t = dom.create_text("hello");
        dom.append_child(root, t);

        let mut r = Range::new(t);
        r.set_start(t, 2);
        r.set_end(t, 5);

        let mut ranges = [r];
        // Shorten text to 3 UTF-16 units.
        super::adjust_ranges_for_text_change(&mut ranges, t, 3);

        assert_eq!(ranges[0].start_offset, 2); // still valid
        assert_eq!(ranges[0].end_offset, 3); // clamped from 5 to 3
    }

    #[test]
    fn range_insert_node() {
        let (mut dom, root, t1, _, _) = build_range_tree();
        let mut range = Range::new(t1);
        range.set_start(t1, 2);
        range.set_end(t1, 2);

        let new_elem = dom.create_element("b", Attributes::default());
        range.insert_node(&mut dom, new_elem);

        // t1 should be "He", then <b>, then "llo".
        let children: Vec<_> = dom.children_iter(root).collect();
        assert!(children.len() >= 3);
        let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(tc.0, "He");
        assert_eq!(children[1], new_elem);
    }

    #[test]
    fn range_extract_contents_element_children() {
        // Test extracting element nodes (not just text).
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let a = dom.create_element("a", Attributes::default());
        let b = dom.create_element("b", Attributes::default());
        let c = dom.create_element("c", Attributes::default());
        dom.append_child(div, a);
        dom.append_child(div, b);
        dom.append_child(div, c);

        // Range: div children [1..2] -> should extract <b>.
        let mut range = Range::new(div);
        range.set_start(div, 1);
        range.set_end(div, 2);
        let frag = range.extract_contents(&mut dom);

        // Fragment should contain <b>.
        let frag_children: Vec<_> = dom.children_iter(frag).collect();
        assert_eq!(frag_children.len(), 1);
        assert_eq!(frag_children[0], b);

        // Original div should have <a> and <c>.
        let div_children: Vec<_> = dom.children_iter(div).collect();
        assert_eq!(div_children.len(), 2);
        assert_eq!(div_children[0], a);
        assert_eq!(div_children[1], c);
    }

    #[test]
    fn range_extract_contents_cross_container() {
        // Range spanning from text node t1 to text node t2 across containers.
        let (mut dom, _root, t1, _span, t2) = build_range_tree();
        // Tree: root -> [t1("Hello"), span -> [t2(" World")]]

        let mut range = Range::new(t1);
        range.set_start(t1, 3); // "Hel|lo" -> extract "lo"
        range.set_end(t2, 3); // " Wo|rld" -> extract " Wo"
        let frag = range.extract_contents(&mut dom);

        // t1 should be "Hel".
        let tc1 = dom.world().get::<&TextContent>(t1).unwrap();
        assert_eq!(tc1.0, "Hel");

        // t2 should be "rld".
        let tc2 = dom.world().get::<&TextContent>(t2).unwrap();
        assert_eq!(tc2.0, "rld");

        // Fragment should contain extracted text nodes.
        let frag_children: Vec<_> = dom.children_iter(frag).collect();
        assert!(frag_children.len() >= 2);
    }
}
