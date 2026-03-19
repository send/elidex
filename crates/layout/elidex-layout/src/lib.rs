//! Layout orchestrator for elidex.
//!
//! Routes layout to the appropriate algorithm crate (block, flex, grid)
//! based on each element's `display` value. Re-exports shared types
//! from `elidex-layout-block`.

pub mod hit_test;
mod layout;

// Re-export shared types and utilities from subcrates.
pub use elidex_layout_block::{
    block, inline, resolve_padding, sanitize, sanitize_border, sanitize_padding, BreakToken,
    BreakTokenData, ChildLayoutFn, FragmentainerContext, FragmentationType, LayoutInput,
    LayoutOutcome, MAX_LAYOUT_DEPTH,
};
pub use hit_test::{hit_test, hit_test_with_scroll, HitTestQuery, HitTestResult};
pub use layout::layout_tree;
