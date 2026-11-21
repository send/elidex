//! Engine-independent `Selection` state machine
//! (Selection API Living Standard §3, formerly WHATWG HTML §7.5.5).
//!
//! The VM-side `dom_selection_proto.rs` is a thin marshalling layer over
//! this state machine — all spec algorithms (direction derivation, anchor /
//! focus computation, tree preconditions, alias dispatch) live here. The same
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
//! - The single-range disposition (Q1 (a), plan v3) is enforced here:
//!   [`SelectionState::add_range`] aborts when `rangeCount` is not 0
//!   (Selection API §3 `addRange()` step 2). `rangeCount` itself is 0 for a
//!   selection that is empty or whose anchor or focus is not in the
//!   document's tree, so such a selection is replaced by `addRange()`.
//!
//! - The tree preconditions of Selection API §3 are checked here, each in
//!   its spec step order and before any state change: the setters
//!   (`collapse` / `extend` / `setBaseAndExtent`) **abort** (`Ok(false)`, no
//!   exception) unless the associated document is a shadow-including
//!   inclusive ancestor of the node; `selectAllChildren` / `addRange` abort
//!   unless the root is the associated document; `deleteFromDocument`,
//!   `getRangeAt` and the `rangeCount` / `type` / `anchor*` / `focus*`
//!   attributes read "in the document tree" as: the root is the associated
//!   document (tree-model `tsel` plan §0.5).
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

use elidex_ecs::{EcsDom, Entity};
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
    /// `rangeCount == 0` (this is empty, or its range is not in the
    /// document tree).
    None,
    /// A range is set in the document tree AND it is collapsed
    /// (`selection_type` returns `None` otherwise).
    Caret,
    /// A range is set in the document tree AND it is non-collapsed
    /// (`selection_type` returns `None` otherwise).
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
    /// `offset` exceeds `node`'s length per WHATWG DOM "length of a node".
    IndexSize,
    /// This is empty (or the given range is not this's range).
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

    /// `Selection.rangeCount` per Selection API §3: "0 if this is empty or
    /// either focus or anchor is not in the document tree, and 1
    /// otherwise" (single-range ship per plan-v3 Q1 (a)). One definition,
    /// shared with `type`, `getRangeAt()`, `deleteFromDocument()` and
    /// `addRange()` step 2.
    #[must_use]
    pub fn range_count(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
    ) -> u32 {
        u32::from(
            self.range_in_document_tree(registry, dom, owner_document)
                .is_some(),
        )
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

    /// `Selection.type` per Selection API §3: `None` if this is empty or
    /// either focus or anchor is not in the document tree, `Caret` if the
    /// range is collapsed, `Range` otherwise.
    pub fn selection_type(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
    ) -> SelectionType {
        let Some(id) = self.range_in_document_tree(registry, dom, owner_document) else {
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

    /// The Selection API §2 *anchor* concept: `(node, offset)` derived from
    /// the current range + direction. `None` when this is empty. It need not
    /// be in the document tree (it "could be in a shadow tree of the same
    /// document", §2); the `anchorNode` / `anchorOffset` attributes read
    /// [`Self::anchor_in_document_tree`] instead.
    ///
    /// Per Selection API §2 Definition (anchor / focus):
    /// - `Forward` direction: anchor = range start, focus = range end.
    /// - `Backward` direction: anchor = range end, focus = range start.
    /// - `Directionless`: spec doesn't pin a side; Chrome / Firefox both
    ///   return range start for anchor in this case, so we follow suit.
    ///
    /// Directionless anchor/focus: see `#11-selection-select-all-children-steps`.
    pub fn anchor(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> Option<(Entity, usize)> {
        let id = self.active_range_id?;
        registry.with_range(id, dom, |range, _| self.anchor_of(range))
    }

    /// The anchor end of `range` under the current direction: the one
    /// home of the direction → anchor mapping, shared by [`Self::anchor`]
    /// and [`Self::extend`] (which reads it together with the range's
    /// root in one registry lookup).
    fn anchor_of(&self, range: &Range) -> (Entity, usize) {
        if matches!(self.direction, SelectionDirection::Backward) {
            (range.end_container, range.end_offset)
        } else {
            (range.start_container, range.start_offset)
        }
    }

    /// The Selection API §2 *focus* concept, `(node, offset)`. Inverse of
    /// [`Self::anchor`]; see that doc-comment for the per-direction
    /// mapping and for why it need not be in the document tree.
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

    /// Selection API §3 (`anchorNode`, `anchorOffset`): the anchor, or
    /// `None` if it is null or not in the document tree.
    pub fn anchor_in_document_tree(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
    ) -> Option<(Entity, usize)> {
        self.anchor(registry, dom)
            .filter(|&(node, _)| root_is_document(node, owner_document, dom))
    }

    /// Selection API §3 (`focusNode`, `focusOffset`): the focus, or `None`
    /// if it is null or not in the document tree.
    pub fn focus_in_document_tree(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
    ) -> Option<(Entity, usize)> {
        self.focus(registry, dom)
            .filter(|&(node, _)| root_is_document(node, owner_document, dom))
    }

    /// Selection API §3 (`rangeCount`, `type`, `getRangeAt()`,
    /// `deleteFromDocument()`; paraphrased): this is not empty and both
    /// anchor and focus are in the document tree, read as the associated
    /// document's tree (tsel plan §0.5, rung 2). The one home of that
    /// condition; returns the current range's id when it holds, else `None`.
    fn range_in_document_tree(
        &self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
    ) -> Option<RangeId> {
        let id = self.active_range_id?;
        registry
            .with_range(id, dom, |range, dom| {
                range_roots_are_document(range, owner_document, dom)
            })
            .unwrap_or(false)
            .then_some(id)
    }

    /// `Selection.collapse(node, offset)` per Selection API §3 (`setPosition`
    /// is an alias — the VM-side proto installs both names pointing at this
    /// impl). Steps, each before any state change: DocumentType → throw
    /// (step 2); offset beyond the node's length → throw (step 3); the
    /// associated document is not a shadow-including inclusive ancestor of
    /// `node` → abort (step 4); else set the range (step 6, through DOM
    /// *set the start or end of a range to a boundary point*).
    ///
    /// Returns `Ok(true)` when a range was set and `Ok(false)` on the
    /// abort (no state change, no exception), so the VM-side caller
    /// schedules `selectionchange` only when something happened. On an
    /// exception the Selection state is left bit-identical.
    pub fn collapse(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        node: Entity,
        offset: usize,
    ) -> Result<bool, SelectionError> {
        if dom.is_document_type(node) {
            return Err(SelectionError::InvalidNodeType);
        }
        if offset > node_length(node, dom) {
            return Err(SelectionError::IndexSize);
        }
        if !crate::focus::is_in_document(dom, node, owner_document) {
            return Ok(false);
        }
        let mut range = Range::new_with_owner(node, owner_document);
        range.set_start_to_boundary(node, offset, dom);
        range.set_end_to_boundary(node, offset, dom);
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = SelectionDirection::Directionless;
        Ok(true)
    }

    /// `Selection.collapseToStart()` per Selection API §3 — collapses the
    /// existing range to its start boundary. Throws `InvalidStateError`
    /// when this is empty. Returns `Ok(true)` on every non-error path (the
    /// existing range is collapsed in place), so the VM-side caller keeps
    /// its unconditional `selectionchange`.
    ///
    /// Known deviation: the Selection API creates a new range and leaves
    /// the old `Range` object unchanged; this mutates it in place. Slot
    /// `#11-selection-api-algorithm-conformance`.
    pub fn collapse_to_start(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> Result<bool, SelectionError> {
        let id = self.active_range_id.ok_or(SelectionError::InvalidState)?;
        registry
            .with_range_mut(id, dom, |range, _| {
                range.collapse(true);
            })
            .ok_or(SelectionError::InvalidState)?;
        self.direction = SelectionDirection::Directionless;
        Ok(true)
    }

    /// `Selection.collapseToEnd()` per Selection API §3 — collapses the
    /// existing range to its end boundary. Throws `InvalidStateError` when
    /// this is empty. Returns `Ok(true)` on every non-error path, as
    /// [`Self::collapse_to_start`] (including the in-place collapse and its
    /// known deviation from the spec's new range, slot
    /// `#11-selection-api-algorithm-conformance`).
    pub fn collapse_to_end(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
    ) -> Result<bool, SelectionError> {
        let id = self.active_range_id.ok_or(SelectionError::InvalidState)?;
        registry
            .with_range_mut(id, dom, |range, _| {
                range.collapse(false);
            })
            .ok_or(SelectionError::InvalidState)?;
        self.direction = SelectionDirection::Directionless;
        Ok(true)
    }

    /// `Selection.extend(node, offset)` per Selection API §3. Moves the
    /// focus to `(node, offset)` while keeping the anchor. Steps, each
    /// before any state change: the associated document is not a
    /// shadow-including inclusive ancestor of `node` → abort (step 1);
    /// empty → `InvalidStateError` (step 2); DocumentType → throw and
    /// offset beyond the length → throw (DOM *set the start or end*
    /// steps 1-2, run by the writes of steps 5-7); `node`'s root is not the
    /// range's root → both points to `(node, offset)` (step 5); otherwise
    /// forward / backward through DOM *set the start* / *set the end*
    /// (steps 6-7). Direction is `Forward` when the new focus follows the
    /// anchor, `Backward` when it precedes it, and `Directionless` when the
    /// two points are equal (`direction_between`; the spec's step 8 differs,
    /// tracked by slot `#11-selection-select-all-children-steps`).
    ///
    /// Returns `Ok(true)` when the range was replaced, `Ok(false)` on the
    /// step 1 abort.
    pub fn extend(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        node: Entity,
        offset: usize,
    ) -> Result<bool, SelectionError> {
        if !crate::focus::is_in_document(dom, node, owner_document) {
            return Ok(false);
        }
        let id = self.active_range_id.ok_or(SelectionError::InvalidState)?;
        if dom.is_document_type(node) {
            return Err(SelectionError::InvalidNodeType);
        }
        if offset > node_length(node, dom) {
            return Err(SelectionError::IndexSize);
        }
        // Capture the current anchor BEFORE allocating any new range so
        // we keep the pre-extend boundary semantics.
        let ((anchor_node, anchor_offset), range_root) = registry
            .with_range(id, dom, |range, dom| {
                (
                    self.anchor_of(range),
                    dom.find_tree_root(range.start_container),
                )
            })
            .ok_or(SelectionError::InvalidState)?;
        let new_direction = direction_between(anchor_node, anchor_offset, node, offset, dom);
        let mut range = Range::new_with_owner(anchor_node, owner_document);
        if dom.find_tree_root(node) != range_root {
            // Step 5: set newRange's start and end to newFocus. The
            // boundary-point setters alone would collapse a backward
            // cross-root extend at oldAnchor.
            range.set_start(node, offset);
            range.set_end(node, offset);
        } else if new_direction == SelectionDirection::Backward {
            range.set_start_to_boundary(node, offset, dom);
            range.set_end_to_boundary(anchor_node, anchor_offset, dom);
        } else {
            range.set_start_to_boundary(anchor_node, anchor_offset, dom);
            range.set_end_to_boundary(node, offset, dom);
        }
        // Old RangeId is left in the registry — `mutate_selection`
        // unregisters it when no wrapper holds it.
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = new_direction;
        Ok(true)
    }

    /// `Selection.setBaseAndExtent(anchor, anchorOffset, focus, focusOffset)`
    /// per Selection API §3. Sets anchor and focus simultaneously;
    /// direction is derived from the tree-order relationship (collapsed →
    /// `Directionless`).
    ///
    /// Per WebIDL §3.7.7 Operations, the VM-side wrapper coerces all four
    /// args in declared order (overload resolution converts left to right)
    /// before any spec step runs. Steps, each before any state change:
    /// an offset beyond its node's length → `IndexSizeError` (step 1);
    /// the associated document is not a shadow-including inclusive
    /// ancestor of either node → abort (step 2); DocumentType for either
    /// node → throw (DOM *set the start or end* step 1, inside step 5);
    /// then forward / backward through the DOM setters (step 5).
    ///
    /// Returns `Ok(true)` when the range was replaced, `Ok(false)` on the
    /// step 2 abort.
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
    ) -> Result<bool, SelectionError> {
        if anchor_offset > node_length(anchor_node, dom)
            || focus_offset > node_length(focus_node, dom)
        {
            return Err(SelectionError::IndexSize);
        }
        if !crate::focus::is_in_document(dom, anchor_node, owner_document)
            || !crate::focus::is_in_document(dom, focus_node, owner_document)
        {
            return Ok(false);
        }
        if dom.is_document_type(anchor_node) || dom.is_document_type(focus_node) {
            return Err(SelectionError::InvalidNodeType);
        }
        let new_direction =
            direction_between(anchor_node, anchor_offset, focus_node, focus_offset, dom);
        let mut range = Range::new_with_owner(anchor_node, owner_document);
        if new_direction == SelectionDirection::Backward {
            range.set_start_to_boundary(focus_node, focus_offset, dom);
            range.set_end_to_boundary(anchor_node, anchor_offset, dom);
        } else {
            range.set_start_to_boundary(anchor_node, anchor_offset, dom);
            range.set_end_to_boundary(focus_node, focus_offset, dom);
        }
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = new_direction;
        Ok(true)
    }

    /// `Selection.selectAllChildren(parentNode)` per Selection API §3.
    /// Steps: DocumentType → throw (step 1); `parentNode`'s root is not
    /// the associated document → abort (step 2); then the range is set to
    /// `(parentNode, 0) → (parentNode, parentNode.length)` (steps 4-5,
    /// plain "set", not "set the start") and direction cleared to
    /// `Directionless` (the direction step is the slot
    /// `#11-selection-select-all-children-steps`'s).
    ///
    /// Returns `Ok(true)` when the range was replaced, `Ok(false)` on the
    /// step 2 abort.
    pub fn select_all_children(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        parent: Entity,
    ) -> Result<bool, SelectionError> {
        if dom.is_document_type(parent) {
            return Err(SelectionError::InvalidNodeType);
        }
        if !root_is_document(parent, owner_document, dom) {
            return Ok(false);
        }
        let len = node_length(parent, dom);
        let mut range = Range::new_with_owner(parent, owner_document);
        range.set_start(parent, 0);
        range.set_end(parent, len);
        let id = registry.register(range);
        self.active_range_id = Some(id);
        self.direction = SelectionDirection::Directionless;
        Ok(true)
    }

    /// `Selection.addRange(range)` per Selection API §3. Steps: the root of
    /// the range's boundary points is not the associated document → abort
    /// (step 1); `rangeCount` is not 0 → abort (step 2; `rangeCount` the
    /// attribute's value, so a selection outside the document tree is
    /// replaced); else set this's range to `range` (step 3).
    ///
    /// Returns `true` when the range was set, `false` on an abort (so
    /// VM-side can decide whether to schedule `selectionchange`). The
    /// displaced range, if any, is freed by the VM-side caller.
    pub fn add_range(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
        range_id: RangeId,
    ) -> bool {
        let rooted = registry
            .with_range(range_id, dom, |range, dom| {
                range_roots_are_document(range, owner_document, dom)
            })
            .unwrap_or(false);
        if !rooted {
            return false;
        }
        if self
            .range_in_document_tree(registry, dom, owner_document)
            .is_some()
        {
            return false;
        }
        self.active_range_id = Some(range_id);
        self.direction = SelectionDirection::Directionless;
        true
    }

    /// `Selection.removeRange(range)` per Selection API §3.
    /// `InvalidStateError` when `range` is not the current range. Known
    /// deviation: the spec throws `NotFoundError` there; slot
    /// `#11-selection-api-algorithm-conformance`. Returns
    /// `Ok(true)` on every non-error path (the range is dissociated).
    pub fn remove_range(&mut self, range_id: RangeId) -> Result<bool, SelectionError> {
        match self.active_range_id {
            Some(current) if current == range_id => {
                self.active_range_id = None;
                self.direction = SelectionDirection::Directionless;
                Ok(true)
            }
            _ => Err(SelectionError::InvalidState),
        }
    }

    /// `Selection.removeAllRanges()` per Selection API §3 — drops the
    /// current range. Old RangeId is left in the registry;
    /// `mutate_selection` unregisters it when no wrapper holds it.
    ///
    /// Returns `true` unconditionally, also on an empty selection where
    /// nothing is dissociated; that is the slot
    /// `#11-selection-change-on-noop-range-mutation`'s (facet (a)), whose
    /// fix is one edit here.
    pub fn remove_all_ranges(&mut self) -> bool {
        self.active_range_id = None;
        self.direction = SelectionDirection::Directionless;
        true
    }

    /// `Selection.empty()` per Selection API §3 — alias of
    /// [`Self::remove_all_ranges`], with the same unconditional return
    /// value (slot `#11-selection-change-on-noop-range-mutation`,
    /// facet (a)).
    pub fn empty(&mut self) -> bool {
        self.remove_all_ranges()
    }

    /// `Selection.deleteFromDocument()` per Selection API §3: invoke
    /// `deleteContents()` on this's range "if this is not empty and both
    /// focus and anchor are in the document tree. Otherwise the method must
    /// do nothing." The document tree is the associated document's.
    ///
    /// Returns `None` for the "do nothing" branch, and `Some(records)` when
    /// `deleteContents()` ran (a collapsed range included:
    /// `Some(vec![])`): the `MutationRecord`s produced by
    /// `Range::delete_contents` (childList removals + characterData
    /// text-splice records) so the VM-side caller routes them through the
    /// same `commit_notify_records` chokepoint as the Range natives
    /// (One-issue-one-way: a record-producing primitive's records are never
    /// silently dropped).
    pub fn delete_from_document(
        &mut self,
        registry: &mut LiveRangeRegistry,
        dom: &mut EcsDom,
        owner_document: Entity,
    ) -> Option<Vec<MutationRecord>> {
        let id = self.range_in_document_tree(registry, dom, owner_document)?;
        // `with_range_mut` would need `&mut EcsDom` to call
        // `delete_contents` from inside the closure, but `with_range_mut`'s
        // closure receives `&EcsDom`. Clone the boundary state out under
        // a shared borrow, run `delete_contents(&mut dom)` outside the
        // closure, then write the post-deletion boundaries back via
        // `with_range_mut`.
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
        // Direction reset: slot `#11-selection-api-algorithm-conformance`.
        self.direction = SelectionDirection::Directionless;
        Some(records)
    }

    /// `Selection.getRangeAt(index)` per Selection API §3: throw
    /// `IndexSizeError` (here [`SelectionError::OutOfRange`]) "if index is
    /// not 0, or if this is empty or either focus or anchor is not in the
    /// document tree"; otherwise the current range (a reference, not a
    /// copy: the caller wraps the same `RangeId`).
    pub fn range_at(
        &self,
        index: usize,
        registry: &mut LiveRangeRegistry,
        dom: &EcsDom,
        owner_document: Entity,
    ) -> Result<RangeId, SelectionError> {
        if index != 0 {
            return Err(SelectionError::OutOfRange);
        }
        self.range_in_document_tree(registry, dom, owner_document)
            .ok_or(SelectionError::OutOfRange)
    }

    /// `Selection.containsNode(node, allowPartialContainment)` per
    /// Selection API §3: `false` if this is empty or `node`'s root is not
    /// the associated document (no throw); else full-contain (default) or
    /// partial-overlap containment per `allowPartialContainment`.
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
        if !root_is_document(node, owner_document, dom) {
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

/// DOM §1.1 / §4.2.1: `node`'s root is the associated document (`node` is
/// in its document tree; the associated document, not any document:
/// tree-model tsel plan §0.5).
/// Depth bound differs from `focus::is_in_document` (`find_tree_root` here,
/// `MAX_ANCESTOR_DEPTH` there): `#11-capped-walker-deep-element-processing`.
fn root_is_document(node: Entity, owner_document: Entity, dom: &EcsDom) -> bool {
    dom.find_tree_root(node) == owner_document
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

/// The one home of "both boundary roots of `range` are the associated
/// document" (Selection API `addRange()` step 1; the document-tree gate of
/// [`SelectionState::range_in_document_tree`]).
fn range_roots_are_document(range: &Range, owner_document: Entity, dom: &EcsDom) -> bool {
    root_is_document(range.start_container, owner_document, dom)
        && root_is_document(range.end_container, owner_document, dom)
}

/// `true` when `node`'s `(parent, child_index)..(parent, child_index+1)`
/// boundary points both lie within `range` — i.e. the entire node is
/// covered. Used by `containsNode(node, allowPartialContainment=false)`.
///
/// A node with no parent (only the Document, after the root gate) is
/// treated as contained iff both of the range's containers are that node.
/// Only the containers are compared. Known gap, slot
/// `#11-selection-api-algorithm-conformance`: containsNode compares boundary
/// points, (node, 0) and (node, node's length).
fn node_fully_contained(range: &Range, node: Entity, dom: &EcsDom) -> bool {
    let Some(parent) = dom.get_parent(node) else {
        // Parentless: after the root gate only the Document reaches
        // here. Compares only the containers (both are `node`); it does
        // not check that the range covers the whole root. Known gap, slot
        // `#11-selection-api-algorithm-conformance`.
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
#[path = "selection_tests.rs"]
mod tests;
