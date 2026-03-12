//! Display list builder: converts a laid-out DOM into paint commands.
//!
//! Walks the DOM tree in pre-order (painter's order) and emits
//! [`DisplayItem`]s for background rectangles and text content.
//!
//! Text processing follows CSS `white-space: normal` rules: newlines
//! and tabs are replaced with spaces, and runs of spaces are collapsed
//! to a single space. Whitespace-only text is discarded.

mod bidi;
mod glyph;
mod inline;
mod paint;
mod text;
mod walk;
mod whitespace;

#[cfg(test)]
mod tests;

use elidex_ecs::EcsDom;
use elidex_text::FontDatabase;

use crate::display_list::DisplayList;
use crate::font_cache::FontCache;

// Re-export sub-module items used across sibling modules.
pub(super) use bidi::bidi_visual_order;
pub(super) use glyph::{families_as_refs, place_glyphs, place_glyphs_vertical};
pub(super) use inline::{emit_inline_run, StyledTextSegment};
pub(super) use paint::{
    apply_opacity, emit_background, emit_borders, emit_image, emit_list_marker_with_counter,
    find_nearest_layout_box,
};
pub(super) use text::{compute_text_align_offset, query_segment_font, resolve_text_align};
pub(super) use walk::walk;
pub(super) use whitespace::collapse_segments;

// ---------------------------------------------------------------------------
// Named constants for list marker layout (R4)
// ---------------------------------------------------------------------------

/// List marker size as a fraction of `font_size`.
pub(super) const MARKER_SIZE_FACTOR: f32 = 0.35;

/// Horizontal offset of the list marker from the content box left edge,
/// as a fraction of `font_size`.
pub(super) const MARKER_X_OFFSET_FACTOR: f32 = 0.75;

/// Vertical center of the marker relative to the font ascent.
pub(super) const MARKER_Y_CENTER_FACTOR: f32 = 0.5;

/// Gap between a decimal marker's trailing edge and the content box,
/// as a fraction of `font_size`.
pub(super) const DECIMAL_MARKER_GAP_FACTOR: f32 = 0.3;

// ---------------------------------------------------------------------------
// Named constants for text metrics (R5)
// ---------------------------------------------------------------------------

/// Default descent as a fraction of `font_size` when font metrics are
/// unavailable (negative direction).
pub(super) const DEFAULT_DESCENT_FACTOR: f32 = 0.25;

/// Underline position as a fraction of the descent below the baseline.
///
/// CSS Text Decoration Level 3 section 2.2 leaves the exact position UA-defined when
/// font metrics are unavailable. We place the underline at 50% of the descent.
pub(super) const UNDERLINE_POSITION_FACTOR: f32 = 0.5;

/// Line-through position as a fraction of the ascent above the baseline.
///
/// CSS Text Decoration Level 3 section 2.2 leaves the exact position UA-defined.
/// We place the line-through at 40% of the ascent (roughly the vertical center of
/// lowercase glyphs).
pub(super) const LINE_THROUGH_POSITION_FACTOR: f32 = 0.4;

/// Minimum text decoration thickness divisor: `font_size / DECORATION_THICKNESS_DIVISOR`.
pub(super) const DECORATION_THICKNESS_DIVISOR: f32 = 16.0;

/// Overline position as a fraction of the ascent above the baseline.
///
/// CSS Text Decoration Level 3 section 2.2 leaves the exact position UA-defined.
/// We place the overline at 100% of the ascent (top of the em square).
pub(super) const OVERLINE_POSITION_FACTOR: f32 = 1.0;

/// Build a display list from a laid-out DOM tree.
///
/// Each element with a [`LayoutBox`](elidex_plugin::LayoutBox) component is visited in pre-order.
/// Background colors produce [`SolidRect`](crate::DisplayItem::SolidRect) entries; text
/// nodes produce [`Text`](crate::DisplayItem::Text) entries via re-shaping.
///
/// Children of each element are processed in "inline runs": consecutive
/// non-block children (text nodes and inline elements) have their text
/// collected, whitespace-collapsed, and rendered as a single text item.
/// This avoids position overlap when multiple text nodes share the same
/// block ancestor.
///
/// # Prerequisites
///
/// `elidex_layout::layout_tree()` must have been called first so that
/// every visible element has a [`LayoutBox`](elidex_plugin::LayoutBox) component.
#[must_use]
pub fn build_display_list(dom: &EcsDom, font_db: &FontDatabase) -> DisplayList {
    let mut dl = DisplayList::default();
    let mut font_cache = FontCache::new();

    let roots = find_roots(dom);
    for root in roots {
        walk(dom, root, font_db, &mut font_cache, &mut dl, 0);
    }

    dl
}

/// Find root entities for rendering: parentless entities with layout or children.
fn find_roots(dom: &EcsDom) -> Vec<elidex_ecs::Entity> {
    dom.root_entities()
        .into_iter()
        .filter(|&e| {
            dom.world().get::<&elidex_plugin::LayoutBox>(e).is_ok()
                || dom.get_first_child(e).is_some()
        })
        .collect()
}
