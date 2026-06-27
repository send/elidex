//! Range content manipulation methods (WHATWG DOM §5.5 "Interface Range").

use elidex_ecs::{EcsDom, Entity, NodeKind, TextContent};
use elidex_script_session::{
    apply_append_child, apply_insert_before, apply_remove_child, MutationRecord,
};

use super::{
    compare_points, next_in_preorder_global, node_length, utf16_offset_to_byte_clamped, Range,
};
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

    /// Collect the **top-level** nodes contained in this range, in tree
    /// order, omitting any node whose parent is also contained — i.e. WHATWG
    /// DOM §5.5 `deleteContents`/`extractContents` `nodesToRemove` = "all the
    /// nodes contained in this, in tree order, omitting any node whose parent
    /// is also contained". Shared by [`Self::delete_contents`] and
    /// [`Self::extract_contents`] so the two cannot diverge.
    ///
    /// A node is **contained** (DOM §5.5 [`#contained`]) iff `(node, 0)` is
    /// after the range start AND `(node, node's length)` is before the range
    /// end. The preorder candidate walk is only an over-approximation — it
    /// includes nodes *outside* the range (e.g. children before an element
    /// `start_offset`, Codex PR418 R2) and merely-partially-contained nodes —
    /// so each candidate is filtered through the canonical `compare_points`
    /// containment test before the omit-nested dominance scan. (Codex PR418
    /// R1: the dominance scan alone let a nested descendant emit a spurious
    /// record; R2: the missing containment test let a pre-start sibling do
    /// the same.)
    ///
    /// [`#contained`]: https://dom.spec.whatwg.org/#contained
    fn contained_top_level_nodes(&self, dom: &EcsDom) -> Vec<Entity> {
        let mut collected = Vec::new();
        let mut current = self.start_container;
        while let Some(next) = next_in_preorder_global(current, dom) {
            if next == self.end_container {
                break;
            }
            collected.push(next);
            current = next;
        }
        let mut top_level: Vec<Entity> = Vec::new();
        for &node in &collected {
            // DOM §5.5 "contained": (node, 0) strictly after range start and
            // (node, length) strictly before range end. Drops the preorder
            // walk's over-collection (pre-start siblings, partial overlaps).
            let contained = compare_points(self.start_container, self.start_offset, node, 0, dom)
                < 0
                && compare_points(
                    node,
                    node_length(node, dom),
                    self.end_container,
                    self.end_offset,
                    dom,
                ) < 0;
            if !contained {
                continue;
            }
            // §5.5 omit-nested: skip any node dominated by an already-kept
            // node. Preorder guarantees an ancestor is kept before its
            // descendants, so a single forward dominance scan suffices (the
            // spec's "parent is also contained" holds transitively — a kept
            // ancestor is itself contained).
            if top_level
                .iter()
                .any(|&tl| dom.is_ancestor_or_self(tl, node))
            {
                continue;
            }
            top_level.push(node);
        }
        top_level
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
    pub fn delete_contents(&mut self, dom: &mut EcsDom) -> Vec<MutationRecord> {
        let mut records = Vec::new();
        if self.collapsed() {
            return records;
        }

        // Same container, text node: splice [start_offset..end_offset]
        // → empty via replace_text_data (fires after_replace_data).
        // characterData facet — produces no MutationRecord (deferred B1.3).
        if self.start_container == self.end_container {
            let is_text = dom
                .world()
                .get::<&TextContent>(self.start_container)
                .is_ok();
            if is_text {
                let count = self.end_offset.saturating_sub(self.start_offset);
                let _ = dom.replace_text_data(self.start_container, self.start_offset, count, "");
                self.end_offset = self.start_offset;
                return records;
            }
            // Non-text container: remove children in range via the
            // record-producing `apply_remove_child` — one childList removal
            // record per child (§5.5 deleteContents step 10).
            let children: Vec<_> = dom.children_iter(self.start_container).collect();
            let end = self.end_offset.min(children.len());
            for &child in &children[self.start_offset..end] {
                records.extend(apply_remove_child(dom, self.start_container, child));
            }
            self.end_offset = self.start_offset;
            return records;
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

        // 3. Remove fully-contained nodes between start and end via the
        //    record-producing `apply_remove_child` (§5.5 deleteContents
        //    step 10) — one childList removal record per top-level node
        //    (`contained_top_level_nodes` applies the §5.5 omit-nested rule
        //    so a nested descendant does not emit its own record).
        for node in self.contained_top_level_nodes(dom) {
            if let Some(parent) = dom.get_parent(node) {
                records.extend(apply_remove_child(dom, parent, node));
            }
        }

        self.end_container = self.start_container;
        self.end_offset = self.start_offset;
        records
    }

    /// Extract contents into a document fragment.
    ///
    /// Handles element and text nodes:
    /// - Same container text node: splits and extracts the middle portion.
    /// - Same container element: detaches children in `[start_offset..end_offset]`.
    /// - Different containers: splits boundary text nodes, detaches fully-contained
    ///   nodes, and clones partially-contained element ancestors.
    #[allow(clippy::too_many_lines)]
    pub fn extract_contents(&mut self, dom: &mut EcsDom) -> (Entity, Vec<MutationRecord>) {
        let frag = dom.create_document_fragment();
        let mut records = Vec::new();

        if self.collapsed() {
            return (frag, records);
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
                    // Fresh clone appended to the (unobserved) fragment —
                    // childList on the fragment, no observed record.
                    let _ = dom.append_child(frag, text_node);
                }
                self.end_offset = self.start_offset;
                return (frag, records);
            }

            // Non-text container: MOVE children in range into the fragment via
            // a single `apply_append_child` per child (§5.5 extractContents
            // step 19 "append contained child to fragment" = a move). This
            // emits the B1.2a 2-record move: an OBSERVED source-removal (target
            // = original parent) + an UNOBSERVED fragment-insertion. F2: replace
            // the remove+append pair with ONE apply_append_child so
            // `capture_move_source` reads the old parent pre-move.
            let children: Vec<_> = dom.children_iter(self.start_container).collect();
            let end = self.end_offset.min(children.len());
            let to_move: Vec<_> = children[self.start_offset..end].to_vec();
            for child in to_move {
                records.extend(apply_append_child(dom, frag, child));
            }
            self.end_offset = self.start_offset;
            return (frag, records);
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

        // 2b. Move fully-contained top-level nodes into the fragment
        //     (`contained_top_level_nodes` applies the §5.5 omit-nested rule,
        //     shared with `delete_contents`).
        for node in self.contained_top_level_nodes(dom) {
            // §5.5 extractContents step 19 — MOVE the contained child to the
            // fragment via a single `apply_append_child` (F2: NOT a
            // remove_child + append_child pair, so the move source is read
            // pre-move). Emits the observed source-removal + unobserved
            // fragment-insertion.
            records.extend(apply_append_child(dom, frag, node));
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
                // Fresh clone appended to the (unobserved) fragment.
                let _ = dom.append_child(frag, text_node);
            }
        }

        // Collapse range to start.
        self.end_container = self.start_container;
        self.end_offset = self.start_offset;
        (frag, records)
    }

    /// WHATWG DOM §5.5 `Range.insertNode` core (steps 2-12).
    ///
    /// On success returns `Some((parent, new_offset))` matching
    /// WHATWG §5.5 step 10-11 (referenceNode's index after step 12's
    /// pre-insert, or parent's length when referenceNode is null) so
    /// the caller can apply step 13 to the registered live range
    /// when it was collapsed pre-call.  On rejection (cycle, orphan
    /// parent) returns `None` and the DOM is NOT mutated.
    ///
    /// Copilot R13 (#1): the pre-insertion validity check (cycle)
    /// runs BEFORE the text-node split — the previous impl split
    /// first and rejected later, leaving a dangling tail in the DOM
    /// on insertion failure.
    ///
    /// Copilot R13 (#2): `&self`, not `&mut self`.  Earlier impls
    /// drove a snapshot+commit pattern in VM-side
    /// `native_range_insert_node` that committed stale boundary
    /// deltas over hook-adjusted live-range entries, losing the
    /// §5.10 splitText migration and §4.2.3 insert adjustments.
    /// The caller is responsible for applying step 13 to the
    /// **registered** range (not a clone) when the pre-call range
    /// was collapsed; start and end migrations are handled by the
    /// engine's `after_split_text` / `after_insert` mutation hooks.
    pub fn insert_node(
        &self,
        dom: &mut EcsDom,
        node: Entity,
    ) -> Option<(Entity, usize, Vec<MutationRecord>)> {
        let start_container = self.start_container;
        let start_offset = self.start_offset;
        let is_text = dom.node_kind(start_container) == Some(NodeKind::Text);

        // Spec step 2-5: compute referenceNode + parent.
        let (mut reference_node, parent) = if is_text {
            let parent = dom.get_parent(start_container)?;
            (Some(start_container), parent)
        } else {
            let children: Vec<_> = dom.children_iter(start_container).collect();
            let r = children.get(start_offset).copied();
            (r, start_container)
        };

        // Copilot R14: WHATWG §5.5 step 11 — "node's length" for a
        // DocumentFragment is its child count; for any other node it
        // is 1.  WHATWG §4.2.3 `insert` fans the fragment's children
        // out into `parent` and empties the fragment, so materialise
        // the children list up front and treat it as the canonical
        // pre-insert node list for steps 6, 10-11, and 12.
        //
        // `child_list_uncapped` (NOT `children_iter`, which truncates at
        // `MAX_ANCESTOR_DEPTH`): `apply_*` expands the fragment via the
        // uncapped fragment machinery, so the cycle check + length here MUST
        // see the same full child set — otherwise a fragment wider than the
        // cap moves all children but the bookkeeping (and the new_offset
        // below) under-counts, placing the range end short of the tail
        // (Codex PR418 R2).
        let is_fragment = dom.node_kind(node) == Some(NodeKind::DocumentFragment);
        let nodes: Vec<Entity> = if is_fragment {
            dom.child_list_uncapped(node)
        } else {
            vec![node]
        };

        // Spec step 6: pre-insertion validity (cycle / self-as-parent).
        // Per WHATWG §4.2.3 the host-including-inclusive-ancestor
        // check runs against the ORIGINAL `node` argument too — not
        // just the fanned-out child list.  Without this, an empty
        // DocumentFragment inserted into itself (or a fragment whose
        // children don't reach back up to the parent) would bypass
        // the cycle check and silently succeed as a no-op (Copilot
        // R16).  Run BEFORE step 7's split so a rejection never
        // leaves a dangling tail node.
        if dom.is_ancestor_or_self(node, parent) {
            return None;
        }
        for &n in &nodes {
            if dom.is_ancestor_or_self(n, parent) {
                return None;
            }
        }
        // A `ShadowRoot` is not an insertable node (DOM §4.8 shadow
        // encapsulation): `apply_insert_before` / `apply_append_child` reject
        // it (`rejects_shadow_root_insertion`). Mirror that rejection HERE,
        // before step 7's split — otherwise `Range.insertNode(shadowRoot)` at
        // a Text boundary splits the text node, *then* gets rejected by the
        // `apply_*` layer, leaving the DOM mutated while the native reports
        // `HierarchyRequestError` (violates the rejection guarantee above).
        // Codex PR418 R1.
        if dom.is_shadow_root(node) {
            return None;
        }

        // Spec step 7: split if start is Text.  Safe to mutate DOM now —
        // all rejection paths above have returned.  If split fails
        // (orphan / missing TextContent), keep reference_node at
        // start_container so the insert lands at the original slot.
        if is_text {
            if let Ok(tail) = crate::char_data::split_text::split_text_at_offset(
                start_container,
                start_offset,
                dom,
            ) {
                reference_node = Some(tail);
            }
        }

        // Spec step 8: if node == referenceNode, advance to its next
        // sibling.  Step 8 references the original argument `node`
        // (not a fragment member); a DocumentFragment cannot itself
        // be a child of `parent` (would already be a cycle, rejected
        // above), so the comparison is only meaningful for the
        // single-node case.
        if !is_fragment && reference_node == Some(node) {
            reference_node = dom.get_next_sibling(node);
        }

        // Spec step 12: pre-insert `node` into `parent` before
        // `reference_node` (or append when null) through the
        // record-producing `apply_*` primitive — passing the ORIGINAL
        // `node` (fragment or single) so `apply_*`'s own §4.2.3
        // fragment-expand record shape is reused (the §4.2.3 step-4.2
        // fragment record + the destination record), rather than
        // re-fanning the children here and emitting one move-record per
        // child.  The step-6 cycle pre-check above already excluded the
        // cycle-reject `apply_*` performs internally
        // (`mutation/mod.rs:418`/`:506`), so this does not double-reject;
        // an empty fragment is a legitimate §4.2.3 step-3 no-op, NOT a
        // failure (distinguished below via `is_empty_fragment`).
        let is_empty_fragment = is_fragment && nodes.is_empty();
        let records = match reference_node {
            Some(rn) => apply_insert_before(dom, parent, node, rn),
            None => apply_append_child(dom, parent, node),
        };
        // Genuine insertion failure (orphan parent / invalid reference
        // child) yields an empty record list for a non-empty input — the
        // DOM was NOT mutated; surface as rejection.  An empty fragment
        // also yields an empty list but is a successful no-op.
        if records.is_empty() && !is_empty_fragment {
            return None;
        }

        // Spec step 10-11: newOffset = referenceNode's pre-step-12
        // index + nodes.len() (= spec's pre-bump value + step-11
        // bump).  Read AFTER step 12 so the result is correct even
        // when the inserted node (or a fragment child) was already
        // an earlier sibling of `reference_node` in the same parent
        // — `insert_before` implicitly removes such siblings via
        // `detach_with_hook`, which would have shifted
        // `reference_node` left by one before insertion.  Computing
        // pre-step-12 + nodes.len() over-counts by exactly that
        // shift (Copilot R21).  Post-step-12 refNode.position is
        // unambiguous regardless of same-parent moves: it equals
        // ref_idx_pre_step_9 + nodes.len() - (same-parent-move
        // count), which matches spec's step-10/11 numbering.
        // `child_list_uncapped` (NOT `children_iter`): after expanding a
        // fragment wider than `MAX_ANCESTOR_DEPTH`, `parent` has more children
        // than the cap, so a capped `position`/`count` would under-count and
        // step 13 would set the range end short of the inserted tail (Codex
        // PR418 R2). Match the uncapped `apply_*` expansion.
        let parent_children = dom.child_list_uncapped(parent);
        let new_offset = match reference_node {
            Some(rn) => parent_children.iter().position(|&c| c == rn).unwrap_or(0),
            None => parent_children.len(),
        };

        Some((parent, new_offset, records))
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
    /// `InvalidStateError` per WHATWG DOM §5.5 — same defer slot as
    /// [`Self::clone_contents`].
    pub fn surround_contents(&mut self, _dom: &mut EcsDom, _new_parent: Entity) -> Option<()> {
        None
    }

    /// Parse `markup` in the context of this range and return the
    /// resulting fragment (HTML §8.5.7 createContextualFragment()).
    ///
    /// Stub for PR-A: returns `None` so VM-side bindings can throw a
    /// well-defined error. Full impl requires:
    /// 1. Resolve context element (start_container if Element, else
    ///    its parent — Text / Comment / CData boundary contexts).
    /// 2. Apply the `<html>` → `<body>` rewrite per HTML §8.5.7 step 6 GATED
    ///    on `dom.is_html_namespace(context)` (see
    ///    [`elidex_ecs::EcsDom::is_html_namespace`] forward-stub).
    /// 3. Call `elidex_html_parser::parse_fragment_progressive(markup,
    ///    context, dom, opts)` (the §11.3 strict-first dispatcher, detached
    ///    return) — requires `elidex-dom-api` to take an `elidex-html-parser`
    ///    dependency, which is currently avoided to keep the handler crate
    ///    parser-independent. The sibling `insertAdjacentHTML` routes parsing
    ///    through `elidex-script-session::mutation` (the canonical parser
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
