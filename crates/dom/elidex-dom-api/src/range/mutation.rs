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
    /// text nodes at boundaries.  Copilot R8: text-node truncations
    /// route through [`EcsDom::replace_text_data`] (which fires
    /// `after_replace_data` for live-range adjust) rather than
    /// `set_text_data` (which only fires the truncate-clamp hook).
    /// The replace-data hook is required so OTHER live ranges
    /// anchored in the same text node collapse their boundaries to
    /// the deletion start per WHATWG §5.5 replaceData rule, not
    /// merely clamp to the new length.
    pub fn delete_contents(&mut self, dom: &mut EcsDom) {
        if self.collapsed() {
            return;
        }

        // Same container, text node: splice [start_offset..end_offset]
        // → empty via replace_text_data (fires after_replace_data).
        if self.start_container == self.end_container {
            let is_text = dom
                .world()
                .get::<&TextContent>(self.start_container)
                .is_ok();
            if is_text {
                let count = self.end_offset.saturating_sub(self.start_offset);
                let _ = dom.replace_text_data(self.start_container, self.start_offset, count, "");
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
        // 1. Truncate start text node — splice [start_offset..] → empty
        //    via replace_text_data so live ranges in start_container
        //    get the right adjustment.
        let start_len = dom
            .world()
            .get::<&TextContent>(self.start_container)
            .ok()
            .map(|tc| crate::char_data::utf16_len(&tc.0));
        if let Some(len) = start_len {
            let count = len.saturating_sub(self.start_offset);
            let _ = dom.replace_text_data(self.start_container, self.start_offset, count, "");
        }

        // 2. Truncate end text node — splice [..end_offset] → empty
        //    via replace_text_data.
        let end_len = dom
            .world()
            .get::<&TextContent>(self.end_container)
            .ok()
            .map(|_| self.end_offset);
        if let Some(end_off) = end_len {
            let _ = dom.replace_text_data(self.end_container, 0, end_off, "");
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
                // Text node: extract substring + splice via
                // replace_text_data so live ranges in this node
                // get the correct boundary adjustment (Copilot R8).
                let text = dom
                    .world()
                    .get::<&TextContent>(self.start_container)
                    .map(|tc| tc.0.clone())
                    .unwrap_or_default();
                let start_byte = utf16_offset_to_byte_clamped(&text, self.start_offset);
                let end_byte = utf16_offset_to_byte_clamped(&text, self.end_offset);
                let extracted = text[start_byte..end_byte].to_string();
                let count = self.end_offset.saturating_sub(self.start_offset);
                let _ = dom.replace_text_data(self.start_container, self.start_offset, count, "");
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
        // 2a. Split start text node: extract tail portion via
        //     replace_text_data so live-range boundaries inside it
        //     collapse to start_offset (Copilot R8).
        if dom.node_kind(self.start_container) == Some(NodeKind::Text) {
            let text = dom
                .world()
                .get::<&TextContent>(self.start_container)
                .map(|tc| tc.0.clone())
                .unwrap_or_default();
            let start_byte = utf16_offset_to_byte_clamped(&text, self.start_offset);
            let tail = text[start_byte..].to_string();
            let total = crate::char_data::utf16_len(&text);
            let count = total.saturating_sub(self.start_offset);
            let _ = dom.replace_text_data(self.start_container, self.start_offset, count, "");
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

        // 2c. Split end text node: extract head portion via
        //     replace_text_data (Copilot R8 — same rationale as 2a).
        if dom.node_kind(self.end_container) == Some(NodeKind::Text) {
            let text = dom
                .world()
                .get::<&TextContent>(self.end_container)
                .map(|tc| tc.0.clone())
                .unwrap_or_default();
            let end_byte = utf16_offset_to_byte_clamped(&text, self.end_offset);
            let head = text[..end_byte].to_string();
            let _ = dom.replace_text_data(self.end_container, 0, self.end_offset, "");
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
    /// If start container is a text node, splits it at the offset
    /// first via [`crate::char_data::split_text::split_text_at_offset`]
    /// which fires the spec-correct `after_split_text` hook so live
    /// ranges anchored AFTER the split offset migrate to the new
    /// tail node rather than getting clamp-truncated (Copilot R10).
    pub fn insert_node(&mut self, dom: &mut EcsDom, node: Entity) {
        if dom.node_kind(self.start_container) == Some(NodeKind::Text) {
            let parent = dom.get_parent(self.start_container);
            if let Some(parent) = parent {
                // Copilot R10: split via canonical helper so
                // `after_split_text` fires + live-range boundaries
                // past the split point migrate to the new tail.
                let tail_node = crate::char_data::split_text::split_text_at_offset(
                    self.start_container,
                    self.start_offset,
                    dom,
                )
                .ok();
                if let Some(tail_node) = tail_node {
                    // Insert node before the new tail.
                    let _ = dom.insert_before(parent, node, tail_node);
                } else {
                    // Split failed (orphan, missing TextContent,
                    // etc.) — fall back to plain insert at the
                    // start_container's slot.
                    let _ = dom.insert_before(parent, node, self.start_container);
                }
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
    /// Currently a stub that returns `None` so VM-side bindings can
    /// throw `NotSupportedError` (WebIDL convention for unimplemented
    /// methods on shipped interfaces). Full impl requires deep-cloning
    /// DOM nodes (`clone_node_deep` already exists on EcsDom) PLUS the
    /// partial-selection cases at the start / end of the range that
    /// `extract_contents` handles for cross-container ranges. Tracked
    /// at `#11-range-clone-and-surround-contents` defer slot — re-eval
    /// when first WPT failure cites the absence.
    #[must_use]
    pub fn clone_contents(&self, _dom: &EcsDom) -> Option<Entity> {
        None
    }

    /// Surround the range contents with a new parent node.
    ///
    /// Currently a stub returning `None` so VM-side bindings can throw
    /// `InvalidStateError` per WHATWG DOM §4.4 — same defer slot as
    /// [`Self::clone_contents`].
    pub fn surround_contents(&mut self, _dom: &mut EcsDom, _new_parent: Entity) -> Option<()> {
        None
    }

    /// Parse `markup` in the context of this range and return the
    /// resulting fragment (DOM Parsing §3.2 "createContextualFragment").
    ///
    /// Stub for PR-A: returns `None` so VM-side bindings can throw a
    /// well-defined error. Full impl requires:
    /// 1. Resolve context element (start_container if Element, else
    ///    its parent — Text / Comment / CData boundary contexts).
    /// 2. Apply the `<html>` → `<body>` rewrite per §3.2 step 2 GATED
    ///    on `dom.is_html_namespace(context)` (see
    ///    [`elidex_ecs::EcsDom::is_html_namespace`] forward-stub).
    /// 3. Call `elidex_html_parser::parse_html_fragment(markup,
    ///    context_tag, fragment, dom)` — requires `elidex-dom-api` to
    ///    take an `elidex-html-parser` dependency, which is currently
    ///    avoided to keep the handler crate parser-independent. The
    ///    sibling `insertAdjacentHTML` routes parsing through
    ///    `elidex-script-session::mutation` (the canonical parser
    ///    boundary); replicating that path here is a clean follow-up.
    ///
    /// Tracked at `#11-range-create-contextual-fragment` defer slot —
    /// the `is_html_namespace` stub + `Range.owner_document` field are
    /// already in place so the follow-up is purely the parser-call
    /// wiring.
    #[must_use]
    pub fn create_contextual_fragment(&self, _markup: &str, _dom: &mut EcsDom) -> Option<Entity> {
        None
    }
}
