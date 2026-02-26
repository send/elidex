//! Text shaping, measurement, and line breaking for elidex.
//!
//! Provides text layout primitives including Unicode line breaking,
//! font metric queries, and glyph shaping integration.

pub mod database;
pub mod linebreak;
pub mod measurement;
pub mod shaping;

pub use database::{FontDatabase, FontMetrics};
pub use linebreak::{find_break_opportunities, BreakOpportunity};
pub use measurement::{measure_text, TextMetrics};
pub use shaping::{shape_text, ShapedGlyph, ShapedText};
