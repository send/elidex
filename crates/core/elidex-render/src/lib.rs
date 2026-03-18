//! Rendering backend for elidex.
//!
//! Converts a laid-out DOM (with [`LayoutBox`](elidex_plugin::LayoutBox) components)
//! into a display list, then renders via Vello + wgpu.

mod builder;
mod display_list;
mod font_cache;
mod vello_backend;

pub use builder::{
    build_display_list, build_display_list_with_caret, build_display_list_with_scroll,
};
pub use display_list::{DisplayItem, DisplayList, GlyphEntry};
pub use vello_backend::VelloRenderer;
