//! Layout algorithms (block, inline, flexbox) for elidex.
//!
//! Implements CSS box layout including block formatting contexts,
//! inline layout, and the flexbox algorithm.

mod block;
mod inline;
mod layout;

pub use layout::layout_tree;

/// Maximum recursion depth for layout tree walking.
///
/// Prevents stack overflow on deeply nested DOMs. Shared between
/// block and inline layout modules.
const MAX_LAYOUT_DEPTH: u32 = 1000;
