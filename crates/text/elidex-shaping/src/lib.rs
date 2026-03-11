//! Font database, text shaping, and measurement for elidex.
//!
//! Provides font discovery via [`fontdb`], OpenType text shaping via
//! [`rustybuzz`], and high-level text measurement combining both.

mod database;
mod measurement;
mod shaping;

pub use database::{FontDatabase, FontMetrics};
pub use fontdb::ID as FontId;
pub use measurement::{measure_text, TextMetrics};
pub use shaping::{shape_text, shape_text_vertical, ShapedGlyph, ShapedText};

/// Cross-platform fallback font families used by tests that need a system font.
#[cfg(test)]
pub const TEST_FONT_FAMILIES: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Hiragino Sans",
];
