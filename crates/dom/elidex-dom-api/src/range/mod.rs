//! `Range` implementation (WHATWG DOM §6).
//!
//! Represents a contiguous portion of the DOM tree. Provides methods for
//! manipulating the range boundaries and extracting/deleting content.

mod boundary;
mod mutation;

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
}

impl Range {
    /// Create a new range with both endpoints at offset 0 of the given node.
    #[must_use]
    pub fn new(node: Entity) -> Self {
        Self {
            start_container: node,
            start_offset: 0,
            end_container: node,
            end_offset: 0,
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
/// Per WHATWG DOM §5.5, when a node is removed, any Range whose boundary
/// point references the removed node must be adjusted. This simplified
/// implementation handles the most common cases:
///
/// 1. If a boundary container is the removed node, collapse that boundary
///    to `(parent, index)`.
/// 2. If a boundary container is `parent` and its offset is greater than
///    `index`, decrement the offset.
///
/// This function should be called before the actual removal from the DOM.
pub fn adjust_ranges_for_removal(ranges: &mut [Range], node: Entity, parent: Entity, index: usize) {
    for range in ranges.iter_mut() {
        // Adjust start boundary.
        if range.start_container == node {
            range.start_container = parent;
            range.start_offset = index;
        } else if range.start_container == parent && range.start_offset > index {
            range.start_offset -= 1;
        }

        // Adjust end boundary.
        if range.end_container == node {
            range.end_container = parent;
            range.end_offset = index;
        } else if range.end_container == parent && range.end_offset > index {
            range.end_offset -= 1;
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
        super::adjust_ranges_for_removal(&mut ranges, c1, parent, 1);

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
        super::adjust_ranges_for_removal(&mut ranges, child, parent, 0);

        // Both boundaries should collapse to (parent, 0).
        assert_eq!(ranges[0].start_container, parent);
        assert_eq!(ranges[0].start_offset, 0);
        assert_eq!(ranges[0].end_container, parent);
        assert_eq!(ranges[0].end_offset, 0);
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
