//! `TreeWalker`, `NodeIterator`, `Range`, and `Selection` methods for `HostBridge`.

use elidex_ecs::Entity;

use super::HostBridge;

impl HostBridge {
    /// Create a new `TreeWalker` and return its ID.
    pub fn create_tree_walker(&self, root: Entity, what_to_show: u32) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.traversal_next_id;
        inner.traversal_next_id += 1;
        inner
            .tree_walkers
            .insert(id, elidex_dom_api::TreeWalker::new(root, what_to_show));
        id
    }

    /// Access a `TreeWalker` by ID.
    pub fn with_tree_walker<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut elidex_dom_api::TreeWalker) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.tree_walkers.get_mut(&id).map(f)
    }

    /// Create a new `NodeIterator` and return its ID.
    pub fn create_node_iterator(&self, root: Entity, what_to_show: u32) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.traversal_next_id;
        inner.traversal_next_id += 1;
        inner
            .node_iterators
            .insert(id, elidex_dom_api::NodeIterator::new(root, what_to_show));
        id
    }

    /// Access a `NodeIterator` by ID.
    pub fn with_node_iterator<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut elidex_dom_api::NodeIterator) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.node_iterators.get_mut(&id).map(f)
    }

    /// Create a new `Range` and return its ID.
    pub fn create_range(&self, node: Entity) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.traversal_next_id;
        inner.traversal_next_id += 1;
        inner.ranges.insert(id, elidex_dom_api::Range::new(node));
        id
    }

    /// Access a `Range` by ID.
    pub fn with_range<R>(
        &self,
        id: u64,
        f: impl FnOnce(&mut elidex_dom_api::Range) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.ranges.get_mut(&id).map(f)
    }

    /// Get the current selection's Range ID, if any.
    pub fn selection_range_id(&self) -> Option<u64> {
        self.inner.borrow().selection_range_id
    }

    /// Set the selection's Range ID.
    pub fn set_selection_range_id(&self, id: Option<u64>) {
        self.inner.borrow_mut().selection_range_id = id;
    }
}
