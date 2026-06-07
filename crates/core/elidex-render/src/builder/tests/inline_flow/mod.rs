//! Render-side tests for consuming a layout-produced `InlineFlow` (the converged
//! inline-text path) vs falling back to the legacy single-line emit.
//!
//! Split by concern (mirroring the layout-multicol `tests/` submodule house style):
//! - [`consume`] — basic per-line consume, atomic boxes, legacy fallback, no
//!   re-transform, interspersed abspos.
//! - [`vertical`] — vertical-writing-mode columns.
//! - [`relpos`] — `position:relative` inline sub-flow with in-flow gap.
//! - [`paged`] — per-page fragment consume (`expected_generation`).
//! - [`bidi`] — UAX #9 L2 paint-time visual reorder.
//!
//! Shared helpers live here; submodules `use super::*`.

use super::*;
use crate::display_list::DisplayItem;
use elidex_ecs::{InlineFlow, InlineFlowLine, InlineFlowRun, InlineFragment};

mod bidi;
mod consume;
mod justify;
mod paged;
mod relpos;
mod vertical;

/// Collect the glyph vectors of every `Text` display item, in order.
fn text_item_glyphs(
    dl: &crate::display_list::DisplayList,
) -> Vec<&Vec<crate::display_list::GlyphEntry>> {
    dl.0.iter()
        .filter_map(|i| match i {
            DisplayItem::Text { glyphs, .. } => Some(glyphs),
            _ => None,
        })
        .collect()
}

/// Per-Text-item glyph counts, in paint order. Shaping preserves one glyph per
/// cluster even when a font lacks the script (notdef), so the count sequence is a
/// font-independent fingerprint of the order runs were painted in — the signal the
/// bidi tests use to detect a UAX #9 L2 visual reorder.
fn text_item_glyph_counts(dl: &crate::display_list::DisplayList) -> Vec<usize> {
    text_item_glyphs(dl).iter().map(|g| g.len()).collect()
}
