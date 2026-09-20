//! Inline text-height tests, split into scenario submodules on the section seams
//! the single file already carried, per CLAUDE.md's 1000-line touch-time rule
//! (`text_height.rs` had reached 1125 lines). Bodies moved verbatim; the shared
//! imports stay here and reach the submodules through their `use super::*`, the
//! same shape as the sibling `inline_flow/` split.

use super::*;

mod anonymous_block;
mod atomic;
mod basic;
mod collapse;
mod layout_box;
mod multi_style;
mod vertical;
