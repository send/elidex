//! Typed arena allocator for AST nodes.
//!
//! All AST nodes are stored in `Arena<T>` and referenced via `NodeId<T>`,
//! providing stable indices suitable for future incremental parsing (Phase 5).

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Typed index into an `Arena<T>`. Copy + Eq + Hash.
pub struct NodeId<T> {
    index: u32,
    _marker: PhantomData<T>,
}

impl<T> NodeId<T> {
    /// Raw index (for serialization / debug).
    #[must_use]
    pub fn index(self) -> u32 {
        self.index
    }
}

impl<T> Clone for NodeId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for NodeId<T> {}

impl<T> PartialEq for NodeId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<T> Eq for NodeId<T> {}

impl<T> Hash for NodeId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> fmt::Debug for NodeId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.index)
    }
}

/// Append-only typed arena backed by a `Vec<T>`.
#[derive(Debug)]
pub struct Arena<T> {
    nodes: Vec<T>,
    /// Set when the arena exceeds `MAX_NODES`. Callers should check `has_overflowed()`.
    overflowed: bool,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    /// Create an empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            overflowed: false,
        }
    }

    /// Practical limit to prevent runaway allocation before OOM.
    /// 16M nodes ≈ ~256 MiB worst case; well below `u32::MAX` but catches pathological input.
    pub const MAX_NODES: usize = 16 * 1024 * 1024;

    /// Allocate a node, returning its stable id.
    ///
    /// If the arena exceeds `MAX_NODES`, the node is still pushed (to avoid
    /// aliased `NodeId`s) but the `overflowed` flag is set. Callers should
    /// check `is_full()` or `has_overflowed()` to detect this.
    #[inline]
    pub fn alloc(&mut self, value: T) -> NodeId<T> {
        if self.nodes.len() >= Self::MAX_NODES {
            self.overflowed = true;
        }
        let index = self.nodes.len() as u32;
        self.nodes.push(value);
        NodeId {
            index,
            _marker: PhantomData,
        }
    }

    /// Whether the arena has exceeded `MAX_NODES`.
    #[must_use]
    pub fn has_overflowed(&self) -> bool {
        self.overflowed
    }

    /// Get a shared reference to a node.
    ///
    /// # Panics
    /// Panics if `id` is out of bounds. In practice all `NodeId` values are
    /// created by this arena's `alloc()`, so bounds are guaranteed. The
    /// `debug_assert` catches cross-arena misuse during development.
    #[inline]
    #[must_use]
    pub fn get(&self, id: NodeId<T>) -> &T {
        debug_assert!(
            (id.index as usize) < self.nodes.len(),
            "Arena::get: NodeId({}) out of bounds (len={})",
            id.index,
            self.nodes.len()
        );
        // SAFETY-NOTE: bounds guaranteed by alloc(); use get_unchecked in hot path
        // with debug_assert guard above.
        &self.nodes[id.index as usize]
    }

    /// Get a mutable reference to a node.
    ///
    /// # Panics
    /// Same contract as `get()`.
    pub fn get_mut(&mut self, id: NodeId<T>) -> &mut T {
        debug_assert!(
            (id.index as usize) < self.nodes.len(),
            "Arena::get_mut: NodeId({}) out of bounds (len={})",
            id.index,
            self.nodes.len()
        );
        &mut self.nodes[id.index as usize]
    }

    /// Number of allocated nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Whether the arena is near capacity or has already overflowed.
    /// L3: Uses a margin of 1024 nodes so the parser's pre-check catches overflow
    /// before individual `alloc()` calls within a single statement could panic.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.overflowed || self.nodes.len() + 1024 >= Self::MAX_NODES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_get() {
        let mut arena: Arena<i32> = Arena::new();
        let a = arena.alloc(10);
        let b = arena.alloc(20);
        assert_eq!(*arena.get(a), 10);
        assert_eq!(*arena.get(b), 20);
    }

    #[test]
    fn get_mut() {
        let mut arena: Arena<String> = Arena::new();
        let id = arena.alloc("hello".to_string());
        arena.get_mut(id).push_str(" world");
        assert_eq!(arena.get(id), "hello world");
    }

    #[test]
    fn node_id_copy_eq_hash() {
        use std::collections::HashSet;

        let mut arena: Arena<u8> = Arena::new();
        let a = arena.alloc(1);
        let b = a; // Copy
        assert_eq!(a, b);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn len_and_is_empty() {
        let mut arena: Arena<u8> = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
        arena.alloc(1);
        assert!(!arena.is_empty());
        assert_eq!(arena.len(), 1);
    }
}
