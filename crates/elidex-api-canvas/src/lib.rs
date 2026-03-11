//! Canvas 2D rendering context for elidex.
//!
//! Provides a CPU-based Canvas 2D API implementation using tiny-skia.
//! The rendered output is an RGBA8 pixel buffer that integrates with
//! elidex's existing `ImageData` component and rendering pipeline.

mod context;
mod path;
mod style;

pub use context::{Canvas2dContext, DEFAULT_HEIGHT, DEFAULT_WIDTH};
pub use style::serialize_canvas_color;
