//! `Range` implementation (WHATWG DOM §6).
//!
//! Represents a contiguous portion of the DOM tree. Provides methods for
//! manipulating the range boundaries and extracting/deleting content.

use elidex_ecs::{EcsDom, Entity, NodeKind, TextContent};

use crate::element::collect_text_content;

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

    /// Set the start boundary.
    pub fn set_start(&mut self, node: Entity, offset: usize) {
        self.start_container = node;
        self.start_offset = offset;
    }

    /// Set the end boundary.
    pub fn set_end(&mut self, node: Entity, offset: usize) {
        self.end_container = node;
        self.end_offset = offset;
    }

    /// Set start to just before `node`.
    pub fn set_start_before(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom);
            self.set_start(parent, offset);
        }
    }

    /// Set start to just after `node`.
    pub fn set_start_after(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom) + 1;
            self.set_start(parent, offset);
        }
    }

    /// Set end to just before `node`.
    pub fn set_end_before(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom);
            self.set_end(parent, offset);
        }
    }

    /// Set end to just after `node`.
    pub fn set_end_after(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom) + 1;
            self.set_end(parent, offset);
        }
    }

    /// Collapse the range to one of its boundary points.
    pub fn collapse(&mut self, to_start: bool) {
        if to_start {
            self.end_container = self.start_container;
            self.end_offset = self.start_offset;
        } else {
            self.start_container = self.end_container;
            self.start_offset = self.end_offset;
        }
    }

    /// Select the given node (start before, end after).
    pub fn select_node(&mut self, node: Entity, dom: &EcsDom) {
        self.set_start_before(node, dom);
        self.set_end_after(node, dom);
    }

    /// Select the contents of a node (start at 0, end at child/char count).
    pub fn select_node_contents(&mut self, node: Entity, dom: &EcsDom) {
        self.start_container = node;
        self.start_offset = 0;
        self.end_container = node;
        self.end_offset = node_length(node, dom);
    }

    /// Clone this range (independent copy).
    #[must_use]
    pub fn clone_range(&self) -> Self {
        self.clone()
    }

    /// Compare boundary points.
    ///
    /// Returns -1, 0, or 1 based on the relative position of the boundary
    /// points specified by `how`.
    #[must_use]
    pub fn compare_boundary_points(&self, how: u16, other: &Range, dom: &EcsDom) -> i8 {
        let (this_container, this_offset, other_container, other_offset) = match how {
            START_TO_START => (
                self.start_container,
                self.start_offset,
                other.start_container,
                other.start_offset,
            ),
            START_TO_END => (
                self.end_container,
                self.end_offset,
                other.start_container,
                other.start_offset,
            ),
            END_TO_END => (
                self.end_container,
                self.end_offset,
                other.end_container,
                other.end_offset,
            ),
            END_TO_START => (
                self.start_container,
                self.start_offset,
                other.end_container,
                other.end_offset,
            ),
            _ => return 0,
        };
        compare_points(
            this_container,
            this_offset,
            other_container,
            other_offset,
            dom,
        )
    }

    /// Concatenate text content within the range.
    #[must_use]
    pub fn to_string(&self, dom: &EcsDom) -> String {
        if self.collapsed() {
            return String::new();
        }

        // Simple case: same container, text node.
        if self.start_container == self.end_container {
            if let Ok(tc) = dom.world().get::<&TextContent>(self.start_container) {
                let start = self.start_offset.min(tc.0.len());
                let end = self.end_offset.min(tc.0.len());
                return tc.0[start..end].to_string();
            }
            // Non-text container: collect text from children in range.
            let mut result = String::new();
            let children: Vec<_> = dom.children_iter(self.start_container).collect();
            for &child in &children[self.start_offset..self.end_offset.min(children.len())] {
                result.push_str(&collect_text_content(child, dom));
            }
            return result;
        }

        // Different containers: collect partial start, full middle, partial end.
        // Simplified: walk pre-order from start to end collecting text.
        let mut result = String::new();

        // Partial start text.
        if let Ok(tc) = dom.world().get::<&TextContent>(self.start_container) {
            let start = self.start_offset.min(tc.0.len());
            result.push_str(&tc.0[start..]);
        }

        // Walk from start_container to end_container in pre-order.
        let mut current = self.start_container;
        while let Some(next) = next_in_preorder_global(current, dom) {
            if next == self.end_container {
                break;
            }
            if dom.node_kind(next) == Some(NodeKind::Text) {
                if let Ok(tc) = dom.world().get::<&TextContent>(next) {
                    result.push_str(&tc.0);
                }
            }
            current = next;
        }

        // Partial end text.
        if let Ok(tc) = dom.world().get::<&TextContent>(self.end_container) {
            let end = self.end_offset.min(tc.0.len());
            result.push_str(&tc.0[..end]);
        }

        result
    }

    /// Delete the contents of this range.
    ///
    /// Simplified implementation: removes fully-contained nodes, splits
    /// text nodes at boundaries.
    pub fn delete_contents(&mut self, dom: &mut EcsDom) {
        if self.collapsed() {
            return;
        }

        // Same container, text node: just delete the substring.
        if self.start_container == self.end_container {
            if let Ok(mut tc) = dom
                .world_mut()
                .get::<&mut TextContent>(self.start_container)
            {
                let start = self.start_offset.min(tc.0.len());
                let end = self.end_offset.min(tc.0.len());
                tc.0 = format!("{}{}", &tc.0[..start], &tc.0[end..]);
                self.end_offset = self.start_offset;
                return;
            }
            // Non-text container: remove children in range.
            let children: Vec<_> = dom.children_iter(self.start_container).collect();
            let end = self.end_offset.min(children.len());
            for &child in &children[self.start_offset..end] {
                let _ = dom.remove_child(self.start_container, child);
            }
            self.end_offset = self.start_offset;
            return;
        }

        // Different containers: simplified approach.
        // 1. Truncate start text node.
        if let Ok(mut tc) = dom
            .world_mut()
            .get::<&mut TextContent>(self.start_container)
        {
            let start = self.start_offset.min(tc.0.len());
            tc.0 = tc.0[..start].to_string();
        }

        // 2. Truncate end text node.
        if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(self.end_container) {
            let end = self.end_offset.min(tc.0.len());
            tc.0 = tc.0[end..].to_string();
        }

        // 3. Remove fully-contained nodes between start and end.
        let mut to_remove = Vec::new();
        let mut current = self.start_container;
        while let Some(next) = next_in_preorder_global(current, dom) {
            if next == self.end_container {
                break;
            }
            to_remove.push(next);
            current = next;
        }
        for node in to_remove {
            if let Some(parent) = dom.get_parent(node) {
                let _ = dom.remove_child(parent, node);
            }
        }

        self.end_container = self.start_container;
        self.end_offset = self.start_offset;
    }

    /// Extract contents into a document fragment.
    ///
    /// Simplified: collects text from the range, clears the range, and
    /// returns a fragment containing the extracted text.
    pub fn extract_contents(&mut self, dom: &mut EcsDom) -> Entity {
        let text = self.to_string(dom);
        self.delete_contents(dom);

        let frag = dom.create_document_fragment();
        if !text.is_empty() {
            let text_node = dom.create_text(&text);
            let _ = dom.append_child(frag, text_node);
        }
        frag
    }

    /// Insert a node at the start boundary.
    ///
    /// If start container is a text node, splits it at the offset first.
    pub fn insert_node(&mut self, dom: &mut EcsDom, node: Entity) {
        if dom.node_kind(self.start_container) == Some(NodeKind::Text) {
            // Read text and parent first, then do mutations.
            let text = dom
                .world()
                .get::<&TextContent>(self.start_container)
                .map(|tc| tc.0.clone())
                .unwrap_or_default();
            let parent = dom.get_parent(self.start_container);
            let next_sib = dom.get_next_sibling(self.start_container);

            if let Some(parent) = parent {
                let offset = self.start_offset.min(text.len());
                let head = text[..offset].to_string();
                let tail = text[offset..].to_string();

                if let Ok(mut tc) = dom
                    .world_mut()
                    .get::<&mut TextContent>(self.start_container)
                {
                    tc.0 = head;
                }

                let tail_node = dom.create_text(&tail);

                // Insert tail after start_container.
                if let Some(next) = next_sib {
                    let _ = dom.insert_before(parent, tail_node, next);
                } else {
                    let _ = dom.append_child(parent, tail_node);
                }

                // Insert node before tail.
                let _ = dom.insert_before(parent, node, tail_node);
            }
        } else {
            // Non-text container: insert at offset.
            let children: Vec<_> = dom.children_iter(self.start_container).collect();
            if self.start_offset < children.len() {
                let _ = dom.insert_before(self.start_container, node, children[self.start_offset]);
            } else {
                let _ = dom.append_child(self.start_container, node);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the index of a child within its parent.
fn child_index(parent: Entity, child: Entity, dom: &EcsDom) -> usize {
    dom.children_iter(parent)
        .position(|c| c == child)
        .unwrap_or(0)
}

/// Get the "length" of a node (character count for text, child count otherwise).
fn node_length(node: Entity, dom: &EcsDom) -> usize {
    if let Ok(tc) = dom.world().get::<&TextContent>(node) {
        return tc.0.len();
    }
    dom.children_iter(node).count()
}

/// Compare two boundary points. Returns -1, 0, or 1.
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

    // Use tree_order_cmp.
    match dom.tree_order_cmp(a_container, b_container) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
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
}
