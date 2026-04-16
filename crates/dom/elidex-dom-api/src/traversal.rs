//! `TreeWalker` and `NodeIterator` implementations (WHATWG DOM §7).
//!
//! These provide filtered traversal of the DOM tree, matching the Web API
//! `TreeWalker` and `NodeIterator` interfaces.

use elidex_ecs::{EcsDom, Entity, NodeKind};

// ---------------------------------------------------------------------------
// whatToShow constants (WHATWG DOM §7.1)
// ---------------------------------------------------------------------------

/// Show all node types.
pub const SHOW_ALL: u32 = 0xFFFF_FFFF;
/// Show only Element nodes.
pub const SHOW_ELEMENT: u32 = 0x1;
/// Show only Text nodes.
pub const SHOW_TEXT: u32 = 0x4;
/// Show only Comment nodes.
pub const SHOW_COMMENT: u32 = 0x80;
/// Show only Document nodes.
pub const SHOW_DOCUMENT: u32 = 0x100;

/// Map a `NodeKind` to its `whatToShow` bitmask bit.
fn node_kind_bit(kind: NodeKind) -> u32 {
    match kind {
        NodeKind::Element => SHOW_ELEMENT,
        NodeKind::Attribute => 0x2,
        NodeKind::Text => SHOW_TEXT,
        NodeKind::CdataSection => 0x8,
        NodeKind::ProcessingInstruction => 0x40,
        NodeKind::Comment => SHOW_COMMENT,
        NodeKind::Document => SHOW_DOCUMENT,
        NodeKind::DocumentType => 0x200,
        NodeKind::DocumentFragment => 0x400,
        // Window is not a Node per WHATWG and is not exposed through
        // NodeIterator / TreeWalker `whatToShow`.
        NodeKind::Window => 0,
    }
}

/// Check if a node's kind is accepted by the given `what_to_show` mask.
fn accepts(entity: Entity, what_to_show: u32, dom: &EcsDom) -> bool {
    if what_to_show == SHOW_ALL {
        return true;
    }
    let Some(kind) = dom.node_kind(entity) else {
        return false;
    };
    (what_to_show & node_kind_bit(kind)) != 0
}

// ---------------------------------------------------------------------------
// Pre-order traversal helpers
// ---------------------------------------------------------------------------

/// Return the next node in pre-order traversal, confined within `root`.
fn next_in_preorder(current: Entity, root: Entity, dom: &EcsDom) -> Option<Entity> {
    // First child?
    if let Some(child) = dom.get_first_child(current) {
        return Some(child);
    }
    // Walk up to find next sibling.
    let mut node = current;
    loop {
        if node == root {
            return None;
        }
        if let Some(sib) = dom.get_next_sibling(node) {
            return Some(sib);
        }
        node = dom.get_parent(node)?;
    }
}

/// Return the previous node in pre-order traversal, confined within `root`.
fn prev_in_preorder(current: Entity, root: Entity, dom: &EcsDom) -> Option<Entity> {
    if current == root {
        return None;
    }
    // Previous sibling's deepest last descendant, or parent.
    if let Some(sib) = dom.get_prev_sibling(current) {
        return Some(last_descendant(sib, dom));
    }
    dom.get_parent(current)
}

/// Walk to the deepest last-child descendant of `node`.
fn last_descendant(node: Entity, dom: &EcsDom) -> Entity {
    let mut current = node;
    while let Some(last) = dom.get_last_child(current) {
        current = last;
    }
    current
}

// ===========================================================================
// TreeWalker
// ===========================================================================

/// `TreeWalker` — filtered tree traversal (WHATWG DOM §7.2).
///
/// `current_node` can be moved by the traversal methods. The walker never
/// moves outside the subtree rooted at `root`.
#[derive(Debug, Clone)]
pub struct TreeWalker {
    /// The root node of the traversal.
    pub root: Entity,
    /// The current position of the walker.
    pub current_node: Entity,
    /// Bitmask of node types to show.
    pub what_to_show: u32,
}

impl TreeWalker {
    /// Create a new `TreeWalker` with `current_node` set to `root`.
    #[must_use]
    pub fn new(root: Entity, what_to_show: u32) -> Self {
        Self {
            root,
            current_node: root,
            what_to_show,
        }
    }

    /// Move to the parent node (stops at root).
    pub fn parent_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        let mut node = self.current_node;
        while node != self.root {
            let parent = dom.get_parent(node)?;
            if accepts(parent, self.what_to_show, dom) {
                self.current_node = parent;
                return Some(parent);
            }
            node = parent;
        }
        None
    }

    /// Move to the first accepted child.
    pub fn first_child(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_children(dom, true)
    }

    /// Move to the last accepted child.
    pub fn last_child(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_children(dom, false)
    }

    /// Move to the next accepted sibling.
    pub fn next_sibling(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_siblings(dom, true)
    }

    /// Move to the previous accepted sibling.
    pub fn previous_sibling(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.traverse_siblings(dom, false)
    }

    /// Move to the next node in pre-order traversal.
    pub fn next_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        let mut node = self.current_node;
        loop {
            node = next_in_preorder(node, self.root, dom)?;
            if accepts(node, self.what_to_show, dom) {
                self.current_node = node;
                return Some(node);
            }
        }
    }

    /// Move to the previous node in pre-order traversal.
    pub fn previous_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        let mut node = self.current_node;
        loop {
            if node == self.root {
                return None;
            }
            node = prev_in_preorder(node, self.root, dom)?;
            if accepts(node, self.what_to_show, dom) {
                self.current_node = node;
                return Some(node);
            }
        }
    }

    /// Helper: traverse to first or last accepted child of `current_node`.
    fn traverse_children(&mut self, dom: &EcsDom, first: bool) -> Option<Entity> {
        let child = if first {
            dom.get_first_child(self.current_node)?
        } else {
            dom.get_last_child(self.current_node)?
        };

        let mut node = child;
        loop {
            if accepts(node, self.what_to_show, dom) {
                self.current_node = node;
                return Some(node);
            }
            // Try children of this node (descend into filtered-out nodes).
            let sub = if first {
                dom.get_first_child(node)
            } else {
                dom.get_last_child(node)
            };
            if let Some(sub_node) = sub {
                node = sub_node;
                continue;
            }
            // Try siblings.
            loop {
                if node == self.current_node {
                    return None;
                }
                let sib = if first {
                    dom.get_next_sibling(node)
                } else {
                    dom.get_prev_sibling(node)
                };
                if let Some(sib_node) = sib {
                    node = sib_node;
                    break;
                }
                let parent = dom.get_parent(node)?;
                if parent == self.current_node {
                    return None;
                }
                node = parent;
            }
        }
    }

    /// Helper: traverse to next or previous accepted sibling.
    fn traverse_siblings(&mut self, dom: &EcsDom, next: bool) -> Option<Entity> {
        let mut node = self.current_node;
        loop {
            let sib = if next {
                dom.get_next_sibling(node)
            } else {
                dom.get_prev_sibling(node)
            };
            if let Some(sib_node) = sib {
                if accepts(sib_node, self.what_to_show, dom) {
                    self.current_node = sib_node;
                    return Some(sib_node);
                }
                // Descend into filtered-out sibling to find an accepted descendant.
                let sub = if next {
                    dom.get_first_child(sib_node)
                } else {
                    dom.get_last_child(sib_node)
                };
                if sub.is_some() {
                    node = sib_node;
                    continue;
                }
                node = sib_node;
                continue;
            }
            // Walk up to parent.
            let parent = dom.get_parent(node)?;
            if parent == self.root {
                return None;
            }
            node = parent;
        }
    }
}

// ===========================================================================
// NodeIterator
// ===========================================================================

/// `NodeIterator` — flat pre-order traversal with filtering (WHATWG DOM §7.1).
#[derive(Debug, Clone)]
pub struct NodeIterator {
    /// The root node of the iteration.
    pub root: Entity,
    /// The reference node for the iterator position.
    pub reference_node: Entity,
    /// Whether the pointer is before the reference node.
    pub pointer_before_reference: bool,
    /// Bitmask of node types to show.
    pub what_to_show: u32,
}

impl NodeIterator {
    /// Create a new `NodeIterator`.
    #[must_use]
    pub fn new(root: Entity, what_to_show: u32) -> Self {
        Self {
            root,
            reference_node: root,
            pointer_before_reference: true,
            what_to_show,
        }
    }

    /// Validate that `reference_node` still exists in the DOM tree rooted at
    /// `root`. If it has been removed (e.g. by a DOM mutation), reset the
    /// iterator to `root`.
    ///
    /// Per WHATWG DOM §7.1, when a node is removed, any `NodeIterator` whose
    /// `reference_node` is that node must update its reference. This safety
    /// check is a simplified version: instead of tracking all mutations via
    /// hooks, we validate on each traversal step.
    fn validate_reference(&mut self, dom: &EcsDom) {
        // Check if reference_node is still a descendant of (or equal to) root.
        if self.reference_node == self.root {
            return;
        }
        if !dom.is_ancestor_or_self(self.root, self.reference_node) {
            // The reference node is no longer in our subtree; reset to root.
            self.reference_node = self.root;
            self.pointer_before_reference = true;
        }
    }

    /// Handle a node removal: if `reference_node` is the removed node, advance
    /// to an adjacent node per WHATWG DOM §7.1.
    ///
    /// Call this before actually removing `removed` from the DOM.
    pub fn pre_remove_check(&mut self, removed: Entity, dom: &EcsDom) {
        if self.reference_node != removed {
            return;
        }
        // Try to advance to next accepted node.
        if let Some(next) = next_in_preorder(removed, self.root, dom) {
            self.reference_node = next;
            self.pointer_before_reference = true;
        } else if let Some(prev) = prev_in_preorder(removed, self.root, dom) {
            // Fall back to previous node.
            self.reference_node = prev;
            self.pointer_before_reference = false;
        } else {
            // Only node was root; reset.
            self.reference_node = self.root;
            self.pointer_before_reference = true;
        }
    }

    /// Return the next accepted node.
    pub fn next_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.validate_reference(dom);
        if self.pointer_before_reference {
            self.pointer_before_reference = false;
            if accepts(self.reference_node, self.what_to_show, dom) {
                return Some(self.reference_node);
            }
        }
        let mut node = self.reference_node;
        loop {
            node = next_in_preorder(node, self.root, dom)?;
            self.reference_node = node;
            if accepts(node, self.what_to_show, dom) {
                return Some(node);
            }
        }
    }

    /// Return the previous accepted node.
    pub fn previous_node(&mut self, dom: &EcsDom) -> Option<Entity> {
        self.validate_reference(dom);
        if !self.pointer_before_reference {
            self.pointer_before_reference = true;
            if accepts(self.reference_node, self.what_to_show, dom) {
                return Some(self.reference_node);
            }
        }
        let mut node = self.reference_node;
        loop {
            node = prev_in_preorder(node, self.root, dom)?;
            if node == self.root {
                // root is included.
                self.reference_node = node;
                if accepts(node, self.what_to_show, dom) {
                    return Some(node);
                }
                return None;
            }
            self.reference_node = node;
            if accepts(node, self.what_to_show, dom) {
                return Some(node);
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    /// Build: root(div) -> [span, text("hello"), p -> [text("world")], comment]
    fn build_tree() -> (EcsDom, Entity, Entity, Entity, Entity, Entity, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let text1 = dom.create_text("hello");
        let p = dom.create_element("p", Attributes::default());
        let text2 = dom.create_text("world");
        let comment = dom.create_comment("a comment");

        dom.append_child(root, span);
        dom.append_child(root, text1);
        dom.append_child(root, p);
        dom.append_child(p, text2);
        dom.append_child(root, comment);

        (dom, root, span, text1, p, text2, comment)
    }

    // --- TreeWalker tests ---

    #[test]
    fn treewalker_next_node_walks_elements() {
        let (dom, root, span, _text1, p, _text2, _comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_ELEMENT);

        assert_eq!(tw.next_node(&dom), Some(span));
        assert_eq!(tw.next_node(&dom), Some(p));
        assert_eq!(tw.next_node(&dom), None);
    }

    #[test]
    fn treewalker_show_text_filters() {
        let (dom, root, _span, text1, _p, text2, _comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_TEXT);

        assert_eq!(tw.next_node(&dom), Some(text1));
        assert_eq!(tw.next_node(&dom), Some(text2));
        assert_eq!(tw.next_node(&dom), None);
    }

    #[test]
    fn treewalker_parent_node_stops_at_root() {
        let (dom, root, _span, _text1, p, text2, _comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_ALL);
        tw.current_node = text2;

        assert_eq!(tw.parent_node(&dom), Some(p));
        assert_eq!(tw.parent_node(&dom), Some(root));
        // At root, should not go further.
        assert_eq!(tw.parent_node(&dom), None);
    }

    #[test]
    fn treewalker_first_child_last_child() {
        let (dom, root, span, _text1, _p, _text2, comment) = build_tree();
        let mut tw = TreeWalker::new(root, SHOW_ALL);

        assert_eq!(tw.first_child(&dom), Some(span));
        tw.current_node = root;
        assert_eq!(tw.last_child(&dom), Some(comment));
    }

    // --- NodeIterator tests ---

    #[test]
    fn nodeiterator_next_previous_roundtrip() {
        let (dom, root, span, text1, p, text2, comment) = build_tree();
        let mut ni = NodeIterator::new(root, SHOW_ALL);

        // Forward
        assert_eq!(ni.next_node(&dom), Some(root));
        assert_eq!(ni.next_node(&dom), Some(span));
        assert_eq!(ni.next_node(&dom), Some(text1));
        assert_eq!(ni.next_node(&dom), Some(p));
        assert_eq!(ni.next_node(&dom), Some(text2));
        assert_eq!(ni.next_node(&dom), Some(comment));
        assert_eq!(ni.next_node(&dom), None);

        // Backward
        assert_eq!(ni.previous_node(&dom), Some(comment));
        assert_eq!(ni.previous_node(&dom), Some(text2));
        assert_eq!(ni.previous_node(&dom), Some(p));
        assert_eq!(ni.previous_node(&dom), Some(text1));
        assert_eq!(ni.previous_node(&dom), Some(span));
        assert_eq!(ni.previous_node(&dom), Some(root));
        assert_eq!(ni.previous_node(&dom), None);
    }

    #[test]
    fn nodeiterator_pre_remove_check_advances() {
        let (dom, root, span, text1, _p, _text2, _comment) = build_tree();
        let mut ni = NodeIterator::new(root, SHOW_ALL);

        // Advance to span.
        ni.next_node(&dom); // root
        ni.next_node(&dom); // span
        assert_eq!(ni.reference_node, span);

        // Pre-remove span: should advance to text1.
        ni.pre_remove_check(span, &dom);
        assert_eq!(ni.reference_node, text1);
    }

    #[test]
    fn nodeiterator_validate_reference_resets_on_removal() {
        let (mut dom, root, span, _text1, _p, _text2, _comment) = build_tree();
        let mut ni = NodeIterator::new(root, SHOW_ALL);

        // Advance to span.
        ni.next_node(&dom); // root
        ni.next_node(&dom); // span
        assert_eq!(ni.reference_node, span);

        // Actually remove span from the tree.
        dom.remove_child(root, span);

        // Next traversal should reset to root since span is no longer in tree.
        let next = ni.next_node(&dom);
        // After reset, pointer_before_reference is true, so returns root first.
        assert_eq!(next, Some(root));
    }

    // --- Normalize full-tree test ---

    #[test]
    fn normalize_merges_adjacent_text_full_tree() {
        use elidex_ecs::TextContent;

        let mut dom = EcsDom::new();
        let root = dom.create_element("div", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        let t1 = dom.create_text("hello");
        let t2 = dom.create_text(" ");
        let t3 = dom.create_text("world");

        dom.append_child(root, p);
        dom.append_child(p, t1);
        dom.append_child(p, t2);
        dom.append_child(p, t3);

        // normalize via the handler
        crate::node_methods::Normalize::normalize_entity(root, &mut dom);

        // p should have one text child: "hello world"
        let children: Vec<_> = dom.children_iter(p).collect();
        assert_eq!(children.len(), 1);
        let text = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(text.0, "hello world");
    }
}
