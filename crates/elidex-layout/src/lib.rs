//! Layout algorithms (block, inline, flexbox) for elidex.
//!
//! Implements CSS box layout including block formatting contexts,
//! inline layout, and the flexbox algorithm.

mod block;
mod inline;
mod layout;

pub use layout::layout_tree;
