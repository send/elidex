//! Engine-independent `Selection` state machine
//! (Selection API Living Standard §3, formerly WHATWG HTML §7.5.5).
//!
//! The VM-side `dom_selection_proto.rs` is a thin marshalling layer over
//! this state machine — all spec algorithms (direction derivation, anchor /
//! focus computation, validity gates, alias dispatch) live here. The same
//! layering mandate as for [`crate::range::live::LiveRangeRegistry`] applies
//! (CLAUDE.md "VM host/ は engine-bound 責務のみ" / lesson #226).
//!
//! # Architecture
//!
//! - [`SelectionState`] is the per-document singleton: a `RangeId` reference
//!   (Selection always backs onto a registered `Range` from
//!   [`crate::range::live::LiveRangeRegistry`]) + a `direction` bias that
//!   records the most recent direction-setting op (`extend` /
//!   `setBaseAndExtent`). All other surface state (anchor / focus / type /
//!   isCollapsed / rangeCount) is **derived at read time** from the
//!   registered Range + bias — never cached — so that live-range adjustments
//!   under the selection are visible immediately.
//!
//! - The Chrome single-range disposition (Q1 (a), plan v3) is enforced here:
//!   [`SelectionState::add_range`] is a spec-correct no-op when a range is
//!   already set.
//!
//! - `direction` derivation per Selection API §3.1 is tri-state:
//!   `Forward` / `Backward` / `Directionless`. A collapsed range is
//!   ALWAYS `Directionless` regardless of the stored bias; the bias kicks
//!   in only once the range becomes non-collapsed via a direction-setting
//!   op. `addRange` / `collapse` / `selectAllChildren` clear the bias to
//!   `Directionless` (their algorithms don't establish a forward/backward
//!   intent).
//!
//! # GC interaction
//!
//! `SelectionState` holds `Option<RangeId>` — a u64 — so it has no Entity
//! references of its own to root. The VM-side wrapper layer
//! (`vm/gc/trace.rs`) fans out from a marked `Selection` wrapper to the
//! cached `Range` wrapper at `range_instances[active_range_id.bits()]`,
//! which keeps the registry entry alive across sweeps. If no Range wrapper
//! has been materialised yet, `getRangeAt(0)` builds one on demand —
//! `RangeId` is the source of truth.

use elidex_ecs::{DocTypeData, EcsDom, Entity, NodeKind};
use elidex_script_session::MutationRecord;

use crate::range::{
    live::{LiveRangeRegistry, RangeId},
    node_length, Range, END_TO_END, START_TO_START,
};

// ---------------------------------------------------------------------------
// SelectionDirection (Selection API §3.1)
// ---------------------------------------------------------------------------

/// Per Selection API §3.1, `Selection.direction` returns one of three
/// strings. The collapsed-range case overrides the stored bias to
/// `Directionless` at read time (see [`SelectionState::current_direction`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionDirection {
    /// Anchor precedes focus in tree order (range start = anchor).
    Forward,
    /// Focus precedes anchor in tree order (range start = focus).
    Backward,
    /// No anchor / focus orientation set — collapsed range, or range set
    /// by `addRange` / `collapse` / `selectAllChildren` (which don't
    /// establish a direction per spec).
    #[default]
    Directionless,
}

impl SelectionDirection {
    /// Stable spec string for `Selection.direction`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Backward => "backward",
            Self::Directionless => "directionless",
        }
    }
}

// ---------------------------------------------------------------------------
// SelectionType (Selection API §3.1)
// ---------------------------------------------------------------------------

/// Per Selection API §3.1, `Selection.type` returns one of three strings
/// derived from the current range state. NOT stored; computed at read
/// time by [`SelectionState::selection_type`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionType {
    /// `rangeCount == 0`.
    None,
    /// A range is set AND it is collapsed.
    Caret,
    /// A range is set AND it is non-collapsed.
    Range,
}

impl SelectionType {
    /// Stable spec string for `Selection.type`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Caret => "Caret",
            Self::Range => "Range",
        }
    }
}

// ---------------------------------------------------------------------------
// SelectionError (spec exceptions)
// ---------------------------------------------------------------------------

/// Spec-defined exception cases surfaced by mutating Selection methods.
/// The VM-side wrapper maps each variant to the matching `DOMException`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    /// `node` is a `DocumentType` (Selection API §3.2 collapse / extend
    /// step 1).
    InvalidNodeType,
    /// `node`'s document differs from the Selection's owner document
    /// (Selection API §3.2 collapse step 2).
    WrongDocument,
    /// `offset` exceeds `node`'s length per WHATWG DOM "length of a node".
    IndexSize,
    /// Operation requires `rangeCount > 0` (Selection API §3.2 extend
    /// step 1 / `getRangeAt(0)`).
    InvalidState,
    /// `getRangeAt(index)` with `index >= rangeCount`.
    OutOfRange,
}

// ---------------------------------------------------------------------------
// SelectionState
// ---------------------------------------------------------------------------

/// Per-document Selection state. Singleton: there is exactly one
/// `SelectionState` per Document. Holds the registry id of the current
/// range (if any) plus the direction bias from the most recent
/// direction-setting op.
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    active_range_id: Option<RangeId>,
    direction: SelectionDirection,
}

impl SelectionState {
    /// Fresh empty Selection: no range, directionless.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Currently-set `RangeId`, if any.
    #[must_use]
    pub fn current_range_id(&self) -> Option<RangeId> {
        self.active_range_id
    }

    /// `Selection.rangeCount` per spec §3.1 — 0 or 1 (Chrome single-range
    /// ship per plan-v3 Q1 (a)).
    #[must_use]
    pub fn range_count(&self) -> u32 {
        u32::from(self.active_range_id.is_some())
    }

    /// `true` when there is no current range.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active_range_id.is_none()
    }

    /// `Selection.isCollapsed` per spec §3.1 — `true` when there is no
    /// range OR the range is collapsed. Read from the live registry to
    /// reflect any mutation-hook adjustments.
    pub fn is_collapsed(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom) -> bool {
        let Some(id) = self.active_range_id else {
            return true;
        };
        registry
            .with_range(id, dom, |range, _| range.collapsed())
            .unwrap_or(true)
    }

    /// `Selection.type` per spec §3.1 — derived from current range state.
    pub fn selection_type(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom) -> SelectionType {
        let Some(id) = self.active_range_id else {
            return SelectionType::None;
        };
        let collapsed = registry
            .with_range(id, dom, |range, _| range.collapsed())
            .unwrap_or(true);
        if collapsed {
            SelectionType::Caret
        } else {
            SelectionType::Range
        }
    }

    /// `Selection.direction` per spec §3.1 — collapsed range always
    /// yields `Directionless`; otherwise the stored bias is returned.
    pub fn current_direction(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> SelectionDirection {
        if self.is_collapsed(registry, dom) {
            return SelectionDirection::Directionless;
        }
        self.direction
    }

    /// `(Selection.anchorNode, Selection.anchorOffset)` derived from the
    /// current range + direction. `None` when `rangeCount == 0`.
    ///
    /// Per Selection API §3.1:
    /// - `Forward` direction: anchor = range start, focus = range end.
    /// - `Backward` direction: anchor = range end, focus = range start.
    /// - `Directionless`: spec doesn't pin a side; Chrome / Firefox both
    ///   return range start for anchor in this case, so we follow suit.
    pub fn anchor(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> Option<(Entity, usize)> {
        let id = self.active_range_id?;
        registry.with_range(id, dom, |range, _| {
            if matches!(self.direction, SelectionDirection::Backward) {
                (range.end_container, range.end_offset)
            } else {
                (range.start_container, range.start_offset)
            }
        })
    }

    /// `(Selection.focusNode, Selection.focusOffset)`. Inverse of
    /// [`Self::anchor`]; see that doc-comment for the per-direction
    /// mapping.
    pub fn focus(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom) -> Option<(Entity, usize)> {
        let id = self.active_range_id?;
        registry.with_range(id, dom, |range, _| {
            if matches!(self.direction, SelectionDirection::Backward) {
                (range.start_container, range.start_offset)
            } else {
                (range.end_container, range.end_offset)
            }
        })
    }

    /// `Selection.collapse(node, offset)` per spec §3.2. Spec aliases
    /// `setPosition` to this — the VM-side proto installs both names
    /// pointing at this impl.
    ///
    /// Validity gates (run in this order, all before any state mutation
    /// per lesson #249): DocumentType rejection / cross-document rejection
    /// / offset bound. On any rejection, the Selection state is left
    /// bit-identical.
    pub fn collapse(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        node: Entity,
        offset: usize,
    ) -> Result<(), SelectionError> {
        if is_doctype(node, dom) {
            return Err(SelectionError::InvalidNodeType);
        }
        if !same_document(node, owner_document, dom) {
            return Err(SelectionError::WrongDocument);
        }
        if offset > node_length(node, dom) {
            return Err(SelectionError::IndexSize);
        }
        let mut range = Range::new_with_owner(node, owner_document);
        range.set_start(node, offset);
        range.set_end(node, offset);
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = SelectionDirection::Directionless;
        Ok(())
    }

    /// `Selection.collapseToStart()` per spec §3.2 — collapses the
    /// existing range to its start boundary. Throws `InvalidStateError`
    /// when `rangeCount == 0`.
    pub fn collapse_to_start(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> Result<(), SelectionError> {
        let id = self.active_range_id.ok_or(SelectionError::InvalidState)?;
        registry
            .with_range_mut(id, dom, |range, _| {
                range.collapse(true);
            })
            .ok_or(SelectionError::InvalidState)?;
        self.direction = SelectionDirection::Directionless;
        Ok(())
    }

    /// `Selection.collapseToEnd()` per spec §3.2 — collapses the existing
    /// range to its end boundary. Throws `InvalidStateError` when
    /// `rangeCount == 0`.
    pub fn collapse_to_end(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> Result<(), SelectionError> {
        let id = self.active_range_id.ok_or(SelectionError::InvalidState)?;
        registry
            .with_range_mut(id, dom, |range, _| {
                range.collapse(false);
            })
            .ok_or(SelectionError::InvalidState)?;
        self.direction = SelectionDirection::Directionless;
        Ok(())
    }

    /// `Selection.extend(node, offset)` per spec §3.2. Moves the focus
    /// to `(node, offset)` while keeping the anchor; the range is set to
    /// `(anchor → focus)` in tree order. Direction is set to `Forward`
    /// when the new focus follows the anchor, `Backward` otherwise.
    ///
    /// `InvalidStateError` when `rangeCount == 0`. Standard validity gates
    /// otherwise.
    pub fn extend(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        node: Entity,
        offset: usize,
    ) -> Result<(), SelectionError> {
        if self.active_range_id.is_none() {
            return Err(SelectionError::InvalidState);
        }
        if is_doctype(node, dom) {
            return Err(SelectionError::InvalidNodeType);
        }
        if !same_document(node, owner_document, dom) {
            return Err(SelectionError::WrongDocument);
        }
        if offset > node_length(node, dom) {
            return Err(SelectionError::IndexSize);
        }
        // Capture the current anchor BEFORE allocating any new range so
        // we keep the pre-extend boundary semantics.
        let (anchor_node, anchor_offset) = self
            .anchor(registry, dom)
            .ok_or(SelectionError::InvalidState)?;
        let new_direction = direction_between(anchor_node, anchor_offset, node, offset, dom);
        let mut range = Range::new_with_owner(anchor_node, owner_document);
        if new_direction == SelectionDirection::Backward {
            range.set_start(node, offset);
            range.set_end(anchor_node, anchor_offset);
        } else {
            range.set_start(anchor_node, anchor_offset);
            range.set_end(node, offset);
        }
        // Old RangeId is left in the registry — GC sweep tail will
        // unregister it when the Range wrapper becomes unreachable.
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = new_direction;
        Ok(())
    }

    /// `Selection.setBaseAndExtent(anchor, anchorOffset, focus, focusOffset)`
    /// per spec §3.2. Sets anchor and focus simultaneously; direction is
    /// derived from the tree-order relationship (collapsed →
    /// `Directionless`).
    ///
    /// Per WebIDL §3.7.6 (lesson #245), the VM-side wrapper coerces all
    /// four args in declared order before any spec step runs. This impl
    /// runs validity gates serially: DocumentType / WrongDocument / offset
    /// bound check on both `anchor` and `focus` before any mutation.
    #[allow(clippy::too_many_arguments)]
    pub fn set_base_and_extent(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        anchor_node: Entity,
        anchor_offset: usize,
        focus_node: Entity,
        focus_offset: usize,
    ) -> Result<(), SelectionError> {
        if is_doctype(anchor_node, dom) || is_doctype(focus_node, dom) {
            return Err(SelectionError::InvalidNodeType);
        }
        if !same_document(anchor_node, owner_document, dom)
            || !same_document(focus_node, owner_document, dom)
        {
            return Err(SelectionError::WrongDocument);
        }
        if anchor_offset > node_length(anchor_node, dom)
            || focus_offset > node_length(focus_node, dom)
        {
            return Err(SelectionError::IndexSize);
        }
        let new_direction =
            direction_between(anchor_node, anchor_offset, focus_node, focus_offset, dom);
        let mut range = Range::new_with_owner(anchor_node, owner_document);
        if new_direction == SelectionDirection::Backward {
            range.set_start(focus_node, focus_offset);
            range.set_end(anchor_node, anchor_offset);
        } else {
            range.set_start(anchor_node, anchor_offset);
            range.set_end(focus_node, focus_offset);
        }
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = new_direction;
        Ok(())
    }

    /// `Selection.selectAllChildren(parentNode)` per spec §3.2. Sets the
    /// range to `(parentNode, 0) → (parentNode, parentNode.length)` and
    /// clears direction to `Directionless` (this op does not establish
    /// a forward/backward intent).
    pub fn select_all_children(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        parent: Entity,
    ) -> Result<(), SelectionError> {
        if is_doctype(parent, dom) {
            return Err(SelectionError::InvalidNodeType);
        }
        if !same_document(parent, owner_document, dom) {
            return Err(SelectionError::WrongDocument);
        }
        let len = node_length(parent, dom);
        let mut range = Range::new_with_owner(parent, owner_document);
        range.set_start(parent, 0);
        range.set_end(parent, len);
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = SelectionDirection::Directionless;
        Ok(())
    }

    /// `Selection.addRange(range)` per Selection API §3.2 step 3.
    /// Chrome single-range disposition (plan-v3 Q1 (a)): no-op when
    /// `rangeCount > 0`. Cross-document range is rejected as a no-op
    /// per spec §3.2 step 2 ("If range's root is not the document
    /// associated with this, abort").
    ///
    /// Returns `true` when the range was actually set, `false` on no-op
    /// (so VM-side can decide whether to fire `selectionchange`).
    pub fn add_range(
        &mut self,
        range_owner_document: Entity,
        self_owner_document: Entity,
        range_id: RangeId,
    ) -> bool {
        if self.active_range_id.is_some() {
            return false;
        }
        if range_owner_document != self_owner_document {
            return false;
        }
        self.active_range_id = Some(range_id);
        self.direction = SelectionDirection::Directionless;
        true
    }

    /// `Selection.removeRange(range)` per spec §3.2. `InvalidStateError`
    /// when `range` is not the current range.
    pub fn remove_range(&mut self, range_id: RangeId) -> Result<(), SelectionError> {
        match self.active_range_id {
            Some(current) if current == range_id => {
                self.active_range_id = None;
                self.direction = SelectionDirection::Directionless;
                Ok(())
            }
            _ => Err(SelectionError::InvalidState),
        }
    }

    /// `Selection.removeAllRanges()` per spec §3.2 — drops the current
    /// range. Old RangeId is left in the registry; GC sweep tail
    /// unregisters when no wrapper is reachable.
    pub fn remove_all_ranges(&mut self) {
        self.active_range_id = None;
        self.direction = SelectionDirection::Directionless;
    }

    /// `Selection.empty()` per spec §3.2 — alias of
    /// [`Self::remove_all_ranges`].
    pub fn empty(&mut self) {
        self.remove_all_ranges();
    }

    /// `Selection.deleteFromDocument()` per spec §3.3. Early-return when
    /// `rangeCount == 0` per spec step 1. Otherwise delegates to
    /// `Range::delete_contents`, which collapses the range to the
    /// deletion start (Selection state is consistent because we hold a
    /// `RangeId` reference, not a snapshot).
    ///
    /// Returns the `MutationRecord`s produced by the underlying
    /// `Range::delete_contents` (childList removals + characterData
    /// text-splice records) so the VM-side caller routes them through
    /// the same `commit_range_mutation_records` chokepoint as the Range
    /// natives (One-issue-one-way: a record-producing primitive's records
    /// are never silently dropped). The `characterData` text-splice
    /// records flow with zero change here — `delete_contents` produces
    /// them (B1.3-ii) and this path inherits them for free.
    pub fn delete_from_document(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &mut EcsDom,
    ) -> Vec<MutationRecord> {
        let Some(id) = self.active_range_id else {
            return Vec::new();
        };
        // `with_range_mut` would need `&mut EcsDom` to call
        // `delete_contents` from inside the closure, but `with_range_mut`'s
        // closure receives `&EcsDom`. Clone the boundary state out under
        // a shared borrow, run `delete_contents(&mut dom)` outside the
        // closure, then write the post-deletion boundaries back via
        // `with_range_mut` (Copilot R2 MIN cleanup: removed an earlier
        // no-op `with_range_mut` that bound `range` to `_` and returned
        // nothing — confusing because it read as if some mutation
        // happened there).
        let snapshot = registry.with_range(id, dom, |range, _| range.clone());
        let mut records = Vec::new();
        if let Some(mut range) = snapshot {
            records = range.delete_contents(dom);
            // Live-range mutation hooks have already updated registered
            // boundaries for any text-splice / removal; the deletion
            // collapses our range to its start, so write that back.
            registry.with_range_mut(id, dom, |reg_range, _| {
                reg_range.start_container = range.start_container;
                reg_range.start_offset = range.start_offset;
                reg_range.end_container = range.start_container;
                reg_range.end_offset = range.start_offset;
            });
        }
        self.direction = SelectionDirection::Directionless;
        records
    }

    /// `Selection.containsNode(node, allowPartialContainment)` per spec
    /// §3.2. Cross-document `node` returns `false` (no throw). The
    /// `allowPartialContainment` arg switches between full-contain
    /// (default) and partial-overlap semantics.
    pub fn contains_node(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        node: Entity,
        allow_partial: bool,
    ) -> bool {
        let Some(id) = self.active_range_id else {
            return false;
        };
        // Cross-document → false.
        if !same_document(node, owner_document, dom) {
            return false;
        }
        registry
            .with_range(id, dom, |range, dom| {
                if allow_partial {
                    range.intersects_node(node, dom)
                } else {
                    node_fully_contained(range, node, dom)
                }
            })
            .unwrap_or(false)
    }

    /// `Selection.toString()` per spec §3.3 — empty string when empty,
    /// otherwise the contained Range's stringification.
    pub fn to_string(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom) -> String {
        let Some(id) = self.active_range_id else {
            return String::new();
        };
        registry
            .with_range(id, dom, Range::to_string)
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Helpers (engine-indep, not exposed)
// ---------------------------------------------------------------------------

/// `true` when `node` is a `DocumentType` per WHATWG DOM `length of a
/// node` exception list. Mirrors the private helper in
/// [`crate::range::boundary`]; duplicated here to keep that one
/// file-scoped (lesson #228 — write primitives self-contained).
fn is_doctype(node: Entity, dom: &EcsDom) -> bool {
    matches!(dom.node_kind_inferred(node), Some(NodeKind::DocumentType))
        || dom.world().get::<&DocTypeData>(node).is_ok()
}

/// `true` when `node`'s owner document is `owner_document`. Used by
/// Selection's cross-document gates (spec §3.2 collapse step 4 etc.).
///
/// Uses [`EcsDom::owner_document`] (which prefers the explicit
/// `AssociatedDocument` component) rather than `find_tree_root` so
/// that detached-but-document-associated nodes (the typical
/// `document.createElement('div')` then `appendChild` sequence)
/// match correctly even before they are linked into the main tree.
/// A node that IS the Document itself returns `None`; we treat that
/// as a self-match against `owner_document` so Selection ops on
/// `document` directly are accepted.
fn same_document(node: Entity, owner_document: Entity, dom: &EcsDom) -> bool {
    if node == owner_document {
        return true;
    }
    dom.owner_document(node) == Some(owner_document)
}

/// Tree-order comparison wrapper around the private `compare_points`
/// in [`crate::range`]. Returns the relevant Selection direction
/// directly: when (a, ao) precedes (b, bo) → `Forward`; equal →
/// `Directionless` (collapsed); after → `Backward`.
fn direction_between(
    a_node: Entity,
    a_offset: usize,
    b_node: Entity,
    b_offset: usize,
    dom: &EcsDom,
) -> SelectionDirection {
    let mut a = Range::new(a_node);
    a.set_start(a_node, a_offset);
    a.set_end(a_node, a_offset);
    let mut b = Range::new(b_node);
    b.set_start(b_node, b_offset);
    b.set_end(b_node, b_offset);
    match a.compare_boundary_points(START_TO_START, &b, dom) {
        x if x < 0 => SelectionDirection::Forward,
        x if x > 0 => SelectionDirection::Backward,
        _ => SelectionDirection::Directionless,
    }
}

/// `true` when `node`'s `(parent, child_index)..(parent, child_index+1)`
/// boundary points both lie within `range` — i.e. the entire node is
/// covered. Used by `containsNode(node, allowPartialContainment=false)`.
///
/// A node with no parent has only one boundary point (the document root)
/// — we treat it as contained iff the range covers the whole root,
/// matching the conservative interpretation Chrome / Firefox use.
fn node_fully_contained(range: &Range, node: Entity, dom: &EcsDom) -> bool {
    let Some(parent) = dom.get_parent(node) else {
        // Detached node: contained only if it IS the range's
        // start/end container (degenerate case).
        return range.start_container == node && range.end_container == node;
    };
    // Build a temporary range covering exactly `node` and compare both
    // boundaries against `range`. node_range fully contained iff:
    //   range.start ≤ node_range.start AND node_range.end ≤ range.end
    let children: Vec<Entity> = dom.children_iter(parent).collect();
    let Some(idx) = children.iter().position(|c| *c == node) else {
        return false;
    };
    let mut node_range = Range::new(parent);
    node_range.set_start(parent, idx);
    node_range.set_end(parent, idx + 1);
    range.compare_boundary_points(START_TO_START, &node_range, dom) <= 0
        && range.compare_boundary_points(END_TO_END, &node_range, dom) >= 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    /// Create an element AND set its `AssociatedDocument` to `doc` so
    /// the engine-indep `EcsDom::owner_document(node) == Some(doc)`
    /// holds (matches the VM-side `document.createElement` path which
    /// always sets the association via `create_element_with_owner`).
    /// Tests without a Document anchor use the raw [`Self::elem`].
    fn elem_in(dom: &mut EcsDom, tag: &str, doc: Entity) -> Entity {
        dom.create_element_with_owner(tag, Attributes::default(), Some(doc))
    }

    #[allow(dead_code)] // kept for future cross-doc / no-association tests
    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    fn fresh() -> (EcsDom, LiveRangeRegistry, Entity) {
        let dom = EcsDom::new();
        let (reg, _bridge) = LiveRangeRegistry::new_pair();
        let mut dom = dom;
        // Build a Document anchor entity that nodes can `owner_document` to.
        let doc = dom.create_document_root();
        (dom, reg, doc)
    }

    #[test]
    fn new_state_is_empty_directionless() {
        let sel = SelectionState::new();
        assert_eq!(sel.range_count(), 0);
        assert!(sel.is_empty());
        assert_eq!(sel.direction, SelectionDirection::Directionless);
        assert!(sel.current_range_id().is_none());
    }

    #[test]
    fn empty_selection_type_and_collapsed() {
        let (dom, mut reg, _doc) = fresh();
        let sel = SelectionState::new();
        assert_eq!(sel.selection_type(&mut reg, &dom), SelectionType::None);
        assert!(sel.is_collapsed(&mut reg, &dom));
        assert!(sel.anchor(&mut reg, &dom).is_none());
        assert!(sel.focus(&mut reg, &dom).is_none());
    }

    #[test]
    fn collapse_sets_caret_directionless() {
        let (mut dom, mut reg, doc) = fresh();
        let child = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, child);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, child, 0).unwrap();
        assert_eq!(sel.range_count(), 1);
        assert_eq!(sel.selection_type(&mut reg, &dom), SelectionType::Caret);
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Directionless
        );
        assert_eq!(sel.anchor(&mut reg, &dom), Some((child, 0)));
        assert_eq!(sel.focus(&mut reg, &dom), Some((child, 0)));
    }

    #[test]
    fn collapse_rejects_doctype() {
        let (mut dom, mut reg, doc) = fresh();
        let doctype = dom.create_document_type("html", "", "");
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.collapse(&mut reg, &dom, doc, doctype, 0),
            Err(SelectionError::InvalidNodeType)
        );
        assert!(sel.is_empty(), "state untouched on reject");
    }

    #[test]
    fn collapse_rejects_cross_document() {
        let (mut dom, mut reg, doc) = fresh();
        let other_doc = dom.create_document_root();
        let foreign = elem_in(&mut dom, "p", other_doc);
        let _ = dom.append_child(other_doc, foreign);
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.collapse(&mut reg, &dom, doc, foreign, 0),
            Err(SelectionError::WrongDocument)
        );
    }

    #[test]
    fn collapse_rejects_out_of_bounds_offset() {
        let (mut dom, mut reg, doc) = fresh();
        let child = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, child);
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.collapse(&mut reg, &dom, doc, child, 99),
            Err(SelectionError::IndexSize)
        );
    }

    #[test]
    fn select_all_children_makes_range() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let c0 = elem_in(&mut dom, "span", doc);
        let c1 = elem_in(&mut dom, "span", doc);
        let _ = dom.append_child(p, c0);
        let _ = dom.append_child(p, c1);
        let mut sel = SelectionState::new();
        sel.select_all_children(&mut reg, &dom, doc, p).unwrap();
        assert_eq!(sel.range_count(), 1);
        assert_eq!(sel.selection_type(&mut reg, &dom), SelectionType::Range);
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Directionless,
            "selectAllChildren does NOT establish direction"
        );
        assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 0)));
        assert_eq!(sel.focus(&mut reg, &dom), Some((p, 2)));
    }

    #[test]
    fn extend_forward_when_focus_after_anchor() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let c0 = elem_in(&mut dom, "span", doc);
        let c1 = elem_in(&mut dom, "span", doc);
        let _ = dom.append_child(p, c0);
        let _ = dom.append_child(p, c1);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
        sel.extend(&mut reg, &dom, doc, p, 2).unwrap();
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Forward
        );
        assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 0)));
        assert_eq!(sel.focus(&mut reg, &dom), Some((p, 2)));
    }

    #[test]
    fn extend_backward_when_focus_before_anchor() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let c0 = elem_in(&mut dom, "span", doc);
        let c1 = elem_in(&mut dom, "span", doc);
        let _ = dom.append_child(p, c0);
        let _ = dom.append_child(p, c1);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 2).unwrap();
        sel.extend(&mut reg, &dom, doc, p, 0).unwrap();
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Backward
        );
        assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 2)));
        assert_eq!(sel.focus(&mut reg, &dom), Some((p, 0)));
    }

    #[test]
    fn extend_rejects_without_initial_range() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.extend(&mut reg, &dom, doc, p, 0),
            Err(SelectionError::InvalidState)
        );
    }

    #[test]
    fn set_base_and_extent_collapsed_is_directionless() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        sel.set_base_and_extent(&mut reg, &dom, doc, p, 0, p, 0)
            .unwrap();
        assert!(sel.is_collapsed(&mut reg, &dom));
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Directionless,
            "collapsed setBaseAndExtent should NOT report a direction"
        );
    }

    #[test]
    fn set_base_and_extent_backward_reverses_range() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let c0 = elem_in(&mut dom, "span", doc);
        let c1 = elem_in(&mut dom, "span", doc);
        let _ = dom.append_child(p, c0);
        let _ = dom.append_child(p, c1);
        let mut sel = SelectionState::new();
        sel.set_base_and_extent(&mut reg, &dom, doc, p, 2, p, 0)
            .unwrap();
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Backward
        );
        assert_eq!(sel.anchor(&mut reg, &dom), Some((p, 2)));
        assert_eq!(sel.focus(&mut reg, &dom), Some((p, 0)));
    }

    #[test]
    fn add_range_no_op_when_range_set() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
        let original = sel.current_range_id().unwrap();
        let mut r = Range::new_with_owner(p, doc);
        r.set_start(p, 0);
        r.set_end(p, 0);
        let new_id = reg.register(r);
        assert!(
            !sel.add_range(doc, doc, new_id),
            "must no-op per Chrome single-range"
        );
        assert_eq!(sel.current_range_id(), Some(original));
    }

    #[test]
    fn add_range_rejected_cross_document() {
        let (mut dom, mut reg, doc) = fresh();
        let other = dom.create_document_root();
        let p = elem_in(&mut dom, "p", other);
        let _ = dom.append_child(other, p);
        let mut sel = SelectionState::new();
        let mut r = Range::new_with_owner(p, other);
        r.set_start(p, 0);
        r.set_end(p, 0);
        let new_id = reg.register(r);
        assert!(!sel.add_range(other, doc, new_id));
        assert!(sel.is_empty());
    }

    #[test]
    fn add_range_sets_when_empty() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        let mut r = Range::new_with_owner(p, doc);
        r.set_start(p, 0);
        r.set_end(p, 0);
        let id = reg.register(r);
        assert!(sel.add_range(doc, doc, id));
        assert_eq!(sel.current_range_id(), Some(id));
        assert_eq!(
            sel.current_direction(&mut reg, &dom),
            SelectionDirection::Directionless
        );
    }

    #[test]
    fn remove_range_clears_when_matching() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
        let id = sel.current_range_id().unwrap();
        sel.remove_range(id).unwrap();
        assert!(sel.is_empty());
    }

    #[test]
    fn remove_range_rejects_non_current() {
        let mut sel = SelectionState::new();
        assert_eq!(
            sel.remove_range(RangeId(999)),
            Err(SelectionError::InvalidState)
        );
    }

    #[test]
    fn remove_all_ranges_empties() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
        sel.remove_all_ranges();
        assert!(sel.is_empty());
    }

    #[test]
    fn empty_is_alias_of_remove_all_ranges() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
        sel.empty();
        assert!(sel.is_empty());
    }

    #[test]
    fn contains_node_full_contain_basic() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let c0 = elem_in(&mut dom, "span", doc);
        let c1 = elem_in(&mut dom, "span", doc);
        let _ = dom.append_child(p, c0);
        let _ = dom.append_child(p, c1);
        let mut sel = SelectionState::new();
        sel.select_all_children(&mut reg, &dom, doc, p).unwrap();
        assert!(sel.contains_node(&mut reg, &dom, doc, c0, false));
        assert!(sel.contains_node(&mut reg, &dom, doc, c1, false));
    }

    #[test]
    fn contains_node_cross_document_false() {
        let (mut dom, mut reg, doc) = fresh();
        let p = elem_in(&mut dom, "p", doc);
        let _ = dom.append_child(doc, p);
        let other_doc = dom.create_document_root();
        let foreign = elem_in(&mut dom, "span", other_doc);
        let _ = dom.append_child(other_doc, foreign);
        let mut sel = SelectionState::new();
        sel.collapse(&mut reg, &dom, doc, p, 0).unwrap();
        assert!(
            !sel.contains_node(&mut reg, &dom, doc, foreign, false),
            "cross-doc must return false, not throw"
        );
        assert!(
            !sel.contains_node(&mut reg, &dom, doc, foreign, true),
            "cross-doc with allowPartial=true must still return false"
        );
    }

    #[test]
    fn empty_selection_to_string_is_empty() {
        let (dom, mut reg, _doc) = fresh();
        let sel = SelectionState::new();
        assert_eq!(sel.to_string(&mut reg, &dom), "");
    }

    #[test]
    fn direction_str_round_trip() {
        for d in [
            SelectionDirection::Forward,
            SelectionDirection::Backward,
            SelectionDirection::Directionless,
        ] {
            // Just exercise the helper; the actual values are spec
            // verbatim and asserted in tests_selection.rs.
            assert!(!d.as_str().is_empty());
        }
    }
}
