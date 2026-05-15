//! Range boundary manipulation methods (WHATWG DOM §6).

use elidex_ecs::{EcsDom, Entity};

use super::{child_index, compare_points, node_length, Range};
use super::{END_TO_END, END_TO_START, START_TO_END, START_TO_START};

/// Error returned by [`Range::compare_point`] / [`Range::is_point_in_range`]
/// for spec-defined exception cases.  The VM-side wrapper maps each
/// variant to the WebIDL DOMException with the matching name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangePointError {
    /// WHATWG DOM §4.4 step 1 — `node`'s root is not the range's root.
    WrongDocument,
    /// WHATWG DOM §4.4 step 2 — `node` is a `DocumentType`.
    InvalidNodeType,
    /// WHATWG DOM §4.4 step 3 — `offset` exceeds `node`'s length.
    IndexSize,
}

impl Range {
    /// Set the start boundary WITHOUT spec collapse-on-cross-root /
    /// after-end logic.  Engine-indep primitive used by in-crate
    /// callers (live-range hooks, tests).  VM-side spec algorithm
    /// uses [`Self::set_start_to_boundary`] instead.
    pub fn set_start(&mut self, node: Entity, offset: usize) {
        self.start_container = node;
        self.start_offset = offset;
    }

    /// Set the end boundary WITHOUT spec collapse-on-cross-root /
    /// before-start logic.  See [`Self::set_start`] for rationale.
    pub fn set_end(&mut self, node: Entity, offset: usize) {
        self.end_container = node;
        self.end_offset = offset;
    }

    /// WHATWG §4.4 "set the start of a Range to a boundary point"
    /// step 4-5: if the new start is after end OR in a different root,
    /// collapse end to (node, offset); then write start.  Assumes
    /// caller has run spec steps 1-2 (DocumentType / IndexSize
    /// validation).
    pub fn set_start_to_boundary(&mut self, node: Entity, offset: usize, dom: &EcsDom) {
        let new_root = dom.find_tree_root(node);
        let after_end = dom.find_tree_root(self.end_container) != new_root
            || compare_points(node, offset, self.end_container, self.end_offset, dom) > 0;
        if after_end {
            self.end_container = node;
            self.end_offset = offset;
        }
        self.start_container = node;
        self.start_offset = offset;
    }

    /// WHATWG §4.4 "set the end of a Range to a boundary point"
    /// step 4-5: mirror of [`Self::set_start_to_boundary`].
    pub fn set_end_to_boundary(&mut self, node: Entity, offset: usize, dom: &EcsDom) {
        let new_root = dom.find_tree_root(node);
        let before_start = dom.find_tree_root(self.start_container) != new_root
            || compare_points(node, offset, self.start_container, self.start_offset, dom) < 0;
        if before_start {
            self.start_container = node;
            self.start_offset = offset;
        }
        self.end_container = node;
        self.end_offset = offset;
    }

    /// Set start to just before `node`.  Per WHATWG §4.4
    /// `setStartBefore`, runs the spec set-start algorithm including
    /// the collapse-on-cross-root branch.
    pub fn set_start_before(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom);
            self.set_start_to_boundary(parent, offset, dom);
        }
    }

    /// Set start to just after `node`.  See [`Self::set_start_before`].
    pub fn set_start_after(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom) + 1;
            self.set_start_to_boundary(parent, offset, dom);
        }
    }

    /// Set end to just before `node`.  See [`Self::set_start_before`].
    pub fn set_end_before(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom);
            self.set_end_to_boundary(parent, offset, dom);
        }
    }

    /// Set end to just after `node`.  See [`Self::set_start_before`].
    pub fn set_end_after(&mut self, node: Entity, dom: &EcsDom) {
        if let Some(parent) = dom.get_parent(node) {
            let offset = child_index(parent, node, dom) + 1;
            self.set_end_to_boundary(parent, offset, dom);
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

    /// WHATWG DOM §4.4 `isPointInRange(node, offset)` algorithm.
    ///
    /// Returns:
    /// - `Ok(true)` if `(node, offset)` lies within `[start, end]` (inclusive).
    /// - `Ok(false)` if the point is outside the range OR `node`'s root
    ///   differs from this range's root (spec step 1 returns false).
    /// - `Err(InvalidNodeType)` if `node` is a `DocumentType`
    ///   (spec step 2 throws `InvalidNodeTypeError`).
    /// - `Err(IndexSize)` if `offset > length(node)`
    ///   (spec step 3 throws `IndexSizeError`).
    pub fn is_point_in_range(
        &self,
        node: Entity,
        offset: usize,
        dom: &EcsDom,
    ) -> Result<bool, RangePointError> {
        // Step 1: roots must match.  Use start_container as the range's
        // root anchor; both boundaries share a root by construction.
        if dom.find_tree_root(node) != dom.find_tree_root(self.start_container) {
            return Ok(false);
        }
        // Step 2: DocumentType rejection delegated to caller via node-kind
        // probe — boundary.rs has no NodeKind access, so the VM-side
        // performs the eager type check before calling.  We still
        // validate offset here for defence-in-depth.
        if offset > node_length(node, dom) {
            return Err(RangePointError::IndexSize);
        }
        // Spec step 4: if (node, offset) is before start or after end, false.
        if compare_points(node, offset, self.start_container, self.start_offset, dom) < 0 {
            return Ok(false);
        }
        if compare_points(node, offset, self.end_container, self.end_offset, dom) > 0 {
            return Ok(false);
        }
        Ok(true)
    }

    /// WHATWG DOM §4.4 `comparePoint(node, offset)` algorithm.
    ///
    /// Returns -1 / 0 / 1 when the point precedes / equals / follows
    /// this range's start (or follows its end).  Per spec:
    /// - `-1` if `(node, offset)` is before the range start.
    /// - `0` if the point lies in `[start, end]`.
    /// - `1` if it is after the range end.
    /// - `Err(WrongDocument)` if `node`'s root differs from the range's root.
    /// - `Err(InvalidNodeType)` if `node` is a `DocumentType` (VM-side check).
    /// - `Err(IndexSize)` if `offset > length(node)`.
    pub fn compare_point(
        &self,
        node: Entity,
        offset: usize,
        dom: &EcsDom,
    ) -> Result<i8, RangePointError> {
        if dom.find_tree_root(node) != dom.find_tree_root(self.start_container) {
            return Err(RangePointError::WrongDocument);
        }
        if offset > node_length(node, dom) {
            return Err(RangePointError::IndexSize);
        }
        if compare_points(node, offset, self.start_container, self.start_offset, dom) < 0 {
            return Ok(-1);
        }
        if compare_points(node, offset, self.end_container, self.end_offset, dom) > 0 {
            return Ok(1);
        }
        Ok(0)
    }

    /// WHATWG DOM §4.4 `intersectsNode(node)` algorithm.
    ///
    /// Returns true when `node` overlaps any part of this range:
    /// - If `node`'s root differs from the range's root → false.
    /// - If `node` has no parent → true (root-of-tree case per spec
    ///   step 2 — a Range whose root contains `node` always intersects
    ///   when `node` is the root itself).
    /// - Otherwise compare `(parent, child_index)` and
    ///   `(parent, child_index + 1)` against the range boundaries.
    #[must_use]
    pub fn intersects_node(&self, node: Entity, dom: &EcsDom) -> bool {
        if dom.find_tree_root(node) != dom.find_tree_root(self.start_container) {
            return false;
        }
        let Some(parent) = dom.get_parent(node) else {
            // Spec step 2: if `node`'s parent is null, return true.
            return true;
        };
        let offset = child_index(parent, node, dom);
        // Spec step 5: (parent, offset) before range end AND
        // (parent, offset + 1) after range start.
        let before_end =
            compare_points(parent, offset, self.end_container, self.end_offset, dom) < 0;
        let after_start = compare_points(
            parent,
            offset + 1,
            self.start_container,
            self.start_offset,
            dom,
        ) > 0;
        before_end && after_start
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
                self.start_container,
                self.start_offset,
                other.end_container,
                other.end_offset,
            ),
            END_TO_END => (
                self.end_container,
                self.end_offset,
                other.end_container,
                other.end_offset,
            ),
            END_TO_START => (
                self.end_container,
                self.end_offset,
                other.start_container,
                other.start_offset,
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
}
