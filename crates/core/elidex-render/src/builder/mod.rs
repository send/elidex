//! Display list builder: converts a laid-out DOM into paint commands.
//!
//! Walks the DOM tree in pre-order (painter's order) and emits
//! [`DisplayItem`]s for background rectangles and text content.
//!
//! Text processing follows CSS `white-space: normal` rules: newlines
//! and tabs are replaced with spaces, and runs of spaces are collapsed
//! to a single space. Whitespace-only text is discarded.

mod bidi;
mod form;
mod glyph;
mod inline;
mod paint;
mod text;
pub(crate) mod transform;
mod walk;
mod whitespace;

#[cfg(test)]
mod tests;

use elidex_ecs::EcsDom;
use elidex_plugin::Vector;
use elidex_style::counter::CounterState;
use elidex_text::FontDatabase;

use crate::display_list::DisplayList;
use crate::font_cache::FontCache;

// Re-export sub-module items used across sibling modules.
use crate::display_list::DisplayItem;
pub(super) use bidi::bidi_visual_order;
pub(super) use glyph::{families_as_refs, place_glyphs, place_glyphs_vertical};
pub(super) use inline::{emit_inline_run, StyledTextSegment};
pub(super) use paint::{
    apply_opacity, emit_background, emit_borders, emit_list_marker_with_counter,
    find_nearest_layout_box,
};
pub(super) use text::{compute_text_align_offset, query_segment_font, resolve_text_align};
pub(super) use walk::{walk, PaintContext};
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
    build_display_list_with_caret(dom, font_db, true)
}

/// Build a display list with explicit caret visibility control.
///
/// Like [`build_display_list`], but allows the caller to control whether
/// the text input caret is rendered. Used by the content thread to
/// implement caret blink (toggling visibility every 500ms).
#[must_use]
pub fn build_display_list_with_caret(
    dom: &EcsDom,
    font_db: &FontDatabase,
    caret_visible: bool,
) -> DisplayList {
    build_display_list_with_scroll(dom, font_db, caret_visible, Vector::<f32>::ZERO)
}

/// Build a display list with viewport scroll offset.
///
/// Wraps the content in `PushScrollOffset`/`PopScrollOffset` when the scroll
/// offset is non-zero. `position: fixed` elements are excluded from the scroll
/// translation (they remain viewport-attached).
#[must_use]
pub fn build_display_list_with_scroll(
    dom: &EcsDom,
    font_db: &FontDatabase,
    caret_visible: bool,
    scroll_offset: Vector,
) -> DisplayList {
    let mut dl = DisplayList::default();
    let mut font_cache = FontCache::new();
    let has_scroll = scroll_offset.x.abs() > f32::EPSILON || scroll_offset.y.abs() > f32::EPSILON;

    if has_scroll {
        dl.push(DisplayItem::PushScrollOffset { scroll_offset });
    }

    let mut ctx = PaintContext {
        dom,
        font_db,
        font_cache: &mut font_cache,
        dl: &mut dl,
        caret_visible,
        scroll_offset,
        counter_state: CounterState::new(),
    };
    let roots = find_roots(dom);
    for root in roots {
        walk(
            &mut ctx,
            root,
            0,
            &elidex_plugin::transform_math::Perspective::default(),
            false,
        );
    }

    if has_scroll {
        dl.push(DisplayItem::PopScrollOffset);
    }

    dl
}

/// Build a multi-page display list from paged layout fragments.
///
/// CSS Paged Media Level 3: for each [`PageFragment`], a display list is
/// built with content offset to the page content area. Margin box content
/// (e.g. page counter text) is rendered into the margin area.
///
/// # Arguments
///
/// * `dom` — The ECS DOM (with layout boxes already assigned per fragment).
/// * `font_db` — Font database for text rendering.
/// * `page_fragments` — Layout results from [`layout_paged`](elidex_layout::layout_paged).
/// * `page_ctx` — Paged media context with page size and margins.
#[must_use]
pub fn build_paged_display_lists(
    dom: &EcsDom,
    font_db: &FontDatabase,
    page_fragments: &[elidex_layout::PageFragment],
    page_ctx: &elidex_plugin::PagedMediaContext,
) -> crate::display_list::PagedDisplayList {
    use crate::display_list::{DisplayList, PagedDisplayList};

    let total_pages = page_fragments.len();
    let mut pages = Vec::with_capacity(total_pages);

    for fragment in page_fragments {
        let (page_width, page_height) =
            page_ctx.effective_page_size(fragment.page_number, fragment.is_blank);
        let margins = page_ctx.effective_margins(fragment.page_number, fragment.is_blank);

        let mut dl = DisplayList::default();
        let mut font_cache = FontCache::new();

        // Build display list from the fragment's layout box.
        // The layout boxes in the DOM already have correct positions from the
        // paged layout pass, so we walk normally.
        if !fragment.is_blank {
            let mut ctx = PaintContext {
                dom,
                font_db,
                font_cache: &mut font_cache,
                dl: &mut dl,
                caret_visible: false,
                scroll_offset: elidex_plugin::Vector::<f32>::ZERO,
                counter_state: elidex_style::counter::CounterState::new(),
            };
            // Set the `page` counter for this page.
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let page_num = fragment.page_number as i32;
            ctx.counter_state.set_counter("page", page_num);
            // Set `pages` counter to total count (known from first pass).
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let total = total_pages as i32;
            ctx.counter_state.set_counter("pages", total);

            let roots = find_roots(dom);
            for root in roots {
                walk(
                    &mut ctx,
                    root,
                    0,
                    &elidex_plugin::transform_math::Perspective::default(),
                    false,
                );
            }
        }

        // Render margin box content (page counters, static strings).
        emit_margin_boxes(
            &mut dl,
            &page_ctx.page_rules,
            fragment,
            page_width,
            page_height,
            &margins,
            total_pages,
        );

        pages.push(dl);
    }

    PagedDisplayList {
        pages,
        page_size: elidex_plugin::Size::new(page_ctx.page_width, page_ctx.page_height),
    }
}

/// Emit margin box content items (text strings, counter values) for a page.
///
/// Iterates over `@page` rules matching this page's selectors and renders
/// margin box content as text items positioned in the margin areas.
fn emit_margin_boxes(
    dl: &mut crate::display_list::DisplayList,
    page_rules: &[elidex_plugin::PageRule],
    fragment: &elidex_layout::PageFragment,
    page_width: f32,
    page_height: f32,
    margins: &elidex_plugin::EdgeSizes,
    total_pages: usize,
) {
    use elidex_plugin::Rect;

    for rule in page_rules {
        // Check if this rule matches the current page.
        let matches = if rule.selectors.is_empty() {
            true
        } else {
            rule.selectors
                .iter()
                .all(|s| s.matches(fragment.page_number, fragment.is_blank))
        };
        if !matches {
            continue;
        }

        // Collect margin box content strings and render them.
        // For now, we handle top-center and bottom-center as the most common.
        let margin_boxes: Vec<(&str, Option<&elidex_plugin::MarginBoxContent>)> = vec![
            ("top-center", rule.margins.top_center.as_ref()),
            ("bottom-center", rule.margins.bottom_center.as_ref()),
            ("top-left", rule.margins.top_left.as_ref()),
            ("top-right", rule.margins.top_right.as_ref()),
            ("bottom-left", rule.margins.bottom_left.as_ref()),
            ("bottom-right", rule.margins.bottom_right.as_ref()),
        ];

        for (position, maybe_content) in margin_boxes {
            let Some(margin_box) = maybe_content else {
                continue;
            };
            let text =
                evaluate_content_value(&margin_box.content, fragment.page_number, total_pages);
            if text.is_empty() {
                continue;
            }

            // Compute position based on margin box name.
            let (x, y) = margin_box_position(position, page_width, page_height, margins, &text);

            // Emit as a SolidRect placeholder for now (text rendering in margin
            // boxes requires font shaping which is deferred to a future phase).
            // We record the text content as metadata via a zero-size rect at the
            // computed position.
            dl.push(DisplayItem::SolidRect {
                rect: Rect::new(x, y, 0.0, 0.0),
                color: elidex_plugin::CssColor::TRANSPARENT,
            });
        }
    }
}

/// Evaluate a `ContentValue` to a string, resolving `counter(page)` and
/// `counter(pages)`.
fn evaluate_content_value(
    content: &elidex_plugin::ContentValue,
    page_number: usize,
    total_pages: usize,
) -> String {
    use elidex_plugin::{ContentItem, ContentValue};

    match content {
        ContentValue::Normal | ContentValue::None => String::new(),
        ContentValue::Items(items) => {
            let mut result = String::new();
            for item in items {
                match item {
                    ContentItem::String(s) => result.push_str(s),
                    ContentItem::Counter { name, .. } => {
                        let value = match name.as_str() {
                            "page" => page_number,
                            "pages" => total_pages,
                            _ => 0,
                        };
                        result.push_str(&value.to_string());
                    }
                    ContentItem::Counters {
                        name, separator, ..
                    } => {
                        let value = match name.as_str() {
                            "page" => page_number,
                            "pages" => total_pages,
                            _ => 0,
                        };
                        if !result.is_empty() {
                            result.push_str(separator);
                        }
                        result.push_str(&value.to_string());
                    }
                    ContentItem::Attr(_) => {} // Not applicable in margin boxes.
                }
            }
            result
        }
    }
}

/// Compute the position for a margin box by name.
fn margin_box_position(
    position: &str,
    page_width: f32,
    page_height: f32,
    margins: &elidex_plugin::EdgeSizes,
    _text: &str,
) -> (f32, f32) {
    match position {
        "top-left" => (margins.left, margins.top * 0.5),
        "top-center" => (page_width * 0.5, margins.top * 0.5),
        "top-right" => (page_width - margins.right, margins.top * 0.5),
        "bottom-left" => (margins.left, page_height - margins.bottom * 0.5),
        "bottom-center" => (page_width * 0.5, page_height - margins.bottom * 0.5),
        "bottom-right" => (
            page_width - margins.right,
            page_height - margins.bottom * 0.5,
        ),
        _ => (0.0, 0.0),
    }
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
