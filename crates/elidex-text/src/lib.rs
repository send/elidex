//! Text shaping, measurement, and line breaking for elidex.
//!
//! Provides text layout primitives including Unicode line breaking,
//! font metric queries, and glyph shaping integration.

mod database;
mod linebreak;
mod measurement;
mod shaping;

pub use database::{FontDatabase, FontMetrics};
pub use fontdb::ID as FontId;
pub use linebreak::{find_break_opportunities, BreakOpportunity};
pub use measurement::{measure_text, TextMetrics};
pub use shaping::{shape_text, ShapedGlyph, ShapedText};

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
