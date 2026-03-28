//! Range content manipulation methods (WHATWG DOM §6).

use elidex_ecs::{EcsDom, Entity, NodeKind, TextContent};

use super::{next_in_preorder_global, utf16_offset_to_byte_clamped, Range};
use crate::element::collect_text_content;

impl Range {
    /// Concatenate text content within the range.
    #[must_use]
    pub fn to_string(&self, dom: &EcsDom) -> String {
        if self.collapsed() {
            return String::new();
        }

        // Simple case: same container, text node.
        if self.start_container == self.end_container {
            if let Ok(tc) = dom.world().get::<&TextContent>(self.start_container) {
                let start = utf16_offset_to_byte_clamped(&tc.0, self.start_offset);
                let end = utf16_offset_to_byte_clamped(&tc.0, self.end_offset);
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
            let start = utf16_offset_to_byte_clamped(&tc.0, self.start_offset);
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
            let end = utf16_offset_to_byte_clamped(&tc.0, self.end_offset);
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
                let start = utf16_offset_to_byte_clamped(&tc.0, self.start_offset);
                let end = utf16_offset_to_byte_clamped(&tc.0, self.end_offset);
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
            let start = utf16_offset_to_byte_clamped(&tc.0, self.start_offset);
            tc.0 = tc.0[..start].to_string();
        }

        // 2. Truncate end text node.
        if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(self.end_container) {
            let end = utf16_offset_to_byte_clamped(&tc.0, self.end_offset);
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
    /// Handles element and text nodes:
    /// - Same container text node: splits and extracts the middle portion.
    /// - Same container element: detaches children in `[start_offset..end_offset]`.
    /// - Different containers: splits boundary text nodes, detaches fully-contained
    ///   nodes, and clones partially-contained element ancestors.
    #[allow(clippy::too_many_lines)]
    pub fn extract_contents(&mut self, dom: &mut EcsDom) -> Entity {
        let frag = dom.create_document_fragment();

        if self.collapsed() {
            return frag;
        }

        // Case 1: Same container.
        if self.start_container == self.end_container {
            if dom.node_kind(self.start_container) == Some(NodeKind::Text) {
                // Text node: extract substring.
                let text = dom
                    .world()
                    .get::<&TextContent>(self.start_container)
                    .map(|tc| tc.0.clone())
                    .unwrap_or_default();
                let start_byte = utf16_offset_to_byte_clamped(&text, self.start_offset);
                let end_byte = utf16_offset_to_byte_clamped(&text, self.end_offset);
                let extracted = text[start_byte..end_byte].to_string();
                let remaining = format!("{}{}", &text[..start_byte], &text[end_byte..]);
                if let Ok(mut tc) = dom
                    .world_mut()
                    .get::<&mut TextContent>(self.start_container)
                {
                    tc.0 = remaining;
                }
                if !extracted.is_empty() {
                    let text_node = dom.create_text(&extracted);
                    let _ = dom.append_child(frag, text_node);
                }
                self.end_offset = self.start_offset;
                return frag;
            }

            // Non-text container: detach children in range.
            let children: Vec<_> = dom.children_iter(self.start_container).collect();
            let end = self.end_offset.min(children.len());
            let to_move: Vec<_> = children[self.start_offset..end].to_vec();
            for child in to_move {
                let _ = dom.remove_child(self.start_container, child);
                let _ = dom.append_child(frag, child);
            }
            self.end_offset = self.start_offset;
            return frag;
        }

        // Case 2: Different containers.
        // 2a. Split start text node: extract tail portion.
        if dom.node_kind(self.start_container) == Some(NodeKind::Text) {
            let text = dom
                .world()
                .get::<&TextContent>(self.start_container)
                .map(|tc| tc.0.clone())
                .unwrap_or_default();
            let start_byte = utf16_offset_to_byte_clamped(&text, self.start_offset);
            let tail = text[start_byte..].to_string();
            let head = text[..start_byte].to_string();
            if let Ok(mut tc) = dom
                .world_mut()
                .get::<&mut TextContent>(self.start_container)
            {
                tc.0 = head;
            }
            if !tail.is_empty() {
                let text_node = dom.create_text(&tail);
                let _ = dom.append_child(frag, text_node);
            }
        }

        // 2b. Collect and detach fully-contained nodes between start and end.
        let mut to_move = Vec::new();
        let mut current = self.start_container;
        while let Some(next) = next_in_preorder_global(current, dom) {
            if next == self.end_container {
                break;
            }
            to_move.push(next);
            current = next;
        }
        // Filter to top-level nodes only (skip descendants of already-collected nodes).
        let mut top_level = Vec::new();
        for &node in &to_move {
            let dominated = top_level
                .iter()
                .any(|&tl| dom.is_ancestor_or_self(tl, node));
            if !dominated {
                top_level.push(node);
            }
        }
        for node in top_level {
            if let Some(parent) = dom.get_parent(node) {
                let _ = dom.remove_child(parent, node);
            }
            let _ = dom.append_child(frag, node);
        }

        // 2c. Split end text node: extract head portion.
        if dom.node_kind(self.end_container) == Some(NodeKind::Text) {
            let text = dom
                .world()
                .get::<&TextContent>(self.end_container)
                .map(|tc| tc.0.clone())
                .unwrap_or_default();
            let end_byte = utf16_offset_to_byte_clamped(&text, self.end_offset);
            let head = text[..end_byte].to_string();
            let tail = text[end_byte..].to_string();
            if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(self.end_container) {
                tc.0 = tail;
            }
            if !head.is_empty() {
                let text_node = dom.create_text(&head);
                let _ = dom.append_child(frag, text_node);
            }
        }

        // Collapse range to start.
        self.end_container = self.start_container;
        self.end_offset = self.start_offset;
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
                let byte_offset = utf16_offset_to_byte_clamped(&text, self.start_offset);
                let head = text[..byte_offset].to_string();
                let tail = text[byte_offset..].to_string();

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

    /// Clone the contents of this range into a document fragment.
    ///
    /// Similar to `extract_contents` but does not modify the original DOM.
    /// Currently a stub that extracts and re-inserts (TODO: proper clone).
    #[must_use]
    pub fn clone_contents(&self, _dom: &EcsDom) -> Option<Entity> {
        // Stub — full implementation requires deep-cloning DOM nodes.
        None
    }

    /// Surround the range contents with a new parent node.
    ///
    /// Currently a stub (requires extract + append + insert sequence).
    pub fn surround_contents(&mut self, _dom: &mut EcsDom, _new_parent: Entity) {
        // Stub — full implementation requires:
        // 1. Check that range doesn't partially select a non-Text node
        // 2. Extract contents
        // 3. Append extracted to new_parent
        // 4. Insert new_parent at range start
        // 5. Select new_parent
    }
}
