//! Text shaping, measurement, line breaking, and bidi analysis for elidex.
//!
//! This is a facade crate that re-exports from [`elidex_shaping`],
//! [`elidex_linebreak`], and [`elidex_bidi`].

pub use elidex_bidi::{analyze_bidi, reorder_by_levels, reorder_line, BidiRun, ParagraphLevel};
pub use elidex_linebreak::{find_break_opportunities, BreakOpportunity};
pub use elidex_shaping::{
    is_word_separator, measure_text, shape_text, shape_text_vertical, shape_text_with_fallback,
    to_fontdb_style, FontDatabase, FontId, FontMetrics, FontStyle, ShapedGlyph, ShapedRun,
    ShapedText, ShapedTextWithFonts, TextMeasureParams, TextMetrics,
};
