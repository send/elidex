//! Range boundary manipulation methods (WHATWG DOM §6).

use elidex_ecs::{EcsDom, Entity};

use super::{child_index, compare_points, node_length, Range};
use super::{END_TO_END, END_TO_START, START_TO_END, START_TO_START};

impl Range {
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
