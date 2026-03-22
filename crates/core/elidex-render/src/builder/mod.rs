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

        // Counter state is populated during the walk and then used by margin
        // box rendering, so it must outlive the PaintContext borrow.
        let mut counter_state = elidex_style::counter::CounterState::new();
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let page_num = fragment.page_number.min(i32::MAX as usize) as i32;
        counter_state.set_counter("page", page_num);
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let total = total_pages.min(i32::MAX as usize) as i32;
        counter_state.set_counter("pages", total);

        // Build display list from the fragment's layout box.
        if !fragment.is_blank {
            let mut ctx = PaintContext {
                dom,
                font_db,
                font_cache: &mut font_cache,
                dl: &mut dl,
                caret_visible: false,
                scroll_offset: elidex_plugin::Vector::<f32>::ZERO,
                counter_state,
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
            // Reclaim counter state after the walk completes.
            counter_state = ctx.counter_state;
        }

        // Render margin box content using the counter state populated by the
        // walk. Custom counters (counter-reset/increment in the document) are
        // now accessible alongside the built-in page/pages counters.
        emit_margin_boxes(
            &mut dl,
            font_db,
            &mut font_cache,
            &page_ctx.page_rules,
            fragment,
            page_width,
            page_height,
            &margins,
            &counter_state,
        );

        pages.push(dl);
    }

    PagedDisplayList {
        pages,
        page_size: elidex_plugin::Size::new(page_ctx.page_width, page_ctx.page_height),
    }
}

/// Default font size for margin box text (CSS Paged Media L3 §4.2).
const MARGIN_BOX_FONT_SIZE: f32 = 12.0;

/// Emit margin box content items (text strings, counter values) for a page.
///
/// Iterates over `@page` rules matching this page's selectors and renders
/// margin box content as shaped text items positioned in the 16 margin areas
/// defined by CSS Paged Media L3 §4.2.
#[allow(clippy::too_many_arguments)]
fn emit_margin_boxes(
    dl: &mut crate::display_list::DisplayList,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    page_rules: &[elidex_plugin::PageRule],
    fragment: &elidex_layout::PageFragment,
    page_width: f32,
    page_height: f32,
    margins: &elidex_plugin::EdgeSizes,
    counter_state: &elidex_style::counter::CounterState,
) {
    for rule in page_rules {
        // Check if this rule matches the current page.
        if !elidex_plugin::selectors_match(
            &rule.selectors,
            fragment.page_number,
            fragment.is_blank,
        ) {
            continue;
        }

        // All 16 margin box types (CSS Paged Media L3 §4.2).
        let margin_boxes: Vec<(&str, Option<&elidex_plugin::MarginBoxContent>)> = vec![
            // Top edge + corners
            ("top-left-corner", rule.margins.top_left_corner.as_ref()),
            ("top-left", rule.margins.top_left.as_ref()),
            ("top-center", rule.margins.top_center.as_ref()),
            ("top-right", rule.margins.top_right.as_ref()),
            ("top-right-corner", rule.margins.top_right_corner.as_ref()),
            // Right edge
            ("right-top", rule.margins.right_top.as_ref()),
            ("right-middle", rule.margins.right_middle.as_ref()),
            ("right-bottom", rule.margins.right_bottom.as_ref()),
            // Bottom edge + corners
            (
                "bottom-right-corner",
                rule.margins.bottom_right_corner.as_ref(),
            ),
            ("bottom-right", rule.margins.bottom_right.as_ref()),
            ("bottom-center", rule.margins.bottom_center.as_ref()),
            ("bottom-left", rule.margins.bottom_left.as_ref()),
            (
                "bottom-left-corner",
                rule.margins.bottom_left_corner.as_ref(),
            ),
            // Left edge
            ("left-bottom", rule.margins.left_bottom.as_ref()),
            ("left-middle", rule.margins.left_middle.as_ref()),
            ("left-top", rule.margins.left_top.as_ref()),
        ];

        for (position, maybe_content) in margin_boxes {
            let Some(margin_box) = maybe_content else {
                continue;
            };
            let text = evaluate_content_value(&margin_box.content, counter_state);
            if text.is_empty() {
                continue;
            }

            emit_margin_box_text(
                dl, font_db, font_cache, position, &text, margin_box, page_width, page_height,
                margins,
            );
        }
    }
}

/// Shape and emit text for a single margin box.
///
/// Resolves `font-size`, `color`, `font-family`, and `font-weight` from the
/// margin box's property declarations. Falls back to 12px serif black 400 when
/// a property is absent.
///
/// The text is positioned according to the margin box type per CSS Paged Media
/// L3 §4.2:
/// - Top/bottom edge boxes: centered vertically in the margin strip.
/// - Left/right edge boxes: centered horizontally in the margin strip.
/// - Corner boxes: centered in the corner rectangle.
#[allow(clippy::too_many_arguments)]
fn emit_margin_box_text(
    dl: &mut DisplayList,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    position: &str,
    text: &str,
    margin_box: &elidex_plugin::MarginBoxContent,
    page_width: f32,
    page_height: f32,
    margins: &elidex_plugin::EdgeSizes,
) {
    // Resolve properties from margin box declarations, falling back to defaults.
    let mut font_size = MARGIN_BOX_FONT_SIZE;
    let mut color = elidex_plugin::CssColor::BLACK;
    let mut family = String::from("serif");
    let mut weight: u16 = 400;
    let mut font_style = fontdb::Style::Normal;

    for decl in &margin_box.properties {
        match decl.property.as_str() {
            "font-size" => {
                let v = match &decl.value {
                    elidex_plugin::CssValue::Length(v, elidex_plugin::LengthUnit::Px)
                    | elidex_plugin::CssValue::Number(v) => Some(*v),
                    _ => None,
                };
                if let Some(v) = v {
                    if v.is_finite() && v > 0.0 {
                        font_size = v;
                    }
                }
            }
            "color" => {
                if let elidex_plugin::CssValue::Color(c) = &decl.value {
                    color = *c;
                }
            }
            "font-family" => {
                let resolved = match &decl.value {
                    elidex_plugin::CssValue::String(s) | elidex_plugin::CssValue::Keyword(s) => {
                        Some(s)
                    }
                    elidex_plugin::CssValue::List(items) => items.first().and_then(|f| match f {
                        elidex_plugin::CssValue::String(s)
                        | elidex_plugin::CssValue::Keyword(s) => Some(s),
                        _ => None,
                    }),
                    _ => None,
                };
                if let Some(s) = resolved {
                    family.clone_from(s);
                }
            }
            "font-weight" => match &decl.value {
                elidex_plugin::CssValue::Number(w) => {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss
                    )]
                    let w_int = w.clamp(1.0, 1000.0) as u16;
                    weight = w_int;
                }
                elidex_plugin::CssValue::Keyword(k) => match k.as_str() {
                    "bold" => weight = 700,
                    "normal" => weight = 400,
                    _ => {}
                },
                _ => {}
            },
            "font-style" => {
                if let elidex_plugin::CssValue::Keyword(k) = &decl.value {
                    match k.as_str() {
                        "italic" => font_style = fontdb::Style::Italic,
                        "oblique" => font_style = fontdb::Style::Oblique,
                        "normal" => font_style = fontdb::Style::Normal,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let families = [family.as_str()];
    let Some(font_id) = font_db.query(&families, weight, font_style) else {
        return;
    };
    let Some(shaped) = elidex_text::shape_text(font_db, font_id, font_size, text) else {
        return;
    };
    let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
        return;
    };

    let text_width: f32 = shaped.glyphs.iter().map(|g| g.x_advance).sum();
    if !text_width.is_finite() {
        return;
    }
    let text_height = font_size;

    // Compute the bounding box of the margin area for this position.
    let (area_x, area_y, area_w, area_h) =
        margin_box_area(position, page_width, page_height, margins);

    // Center text within the margin box area.
    let mut text_x = area_x + (area_w - text_width) * 0.5;
    let baseline_y = area_y + (area_h + text_height) * 0.5;

    let glyphs = place_glyphs(&shaped.glyphs, &mut text_x, baseline_y, 0.0, 0.0, text);
    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size,
        color,
    });
}

/// Evaluate a `ContentValue` to a string using the counter state.
///
/// Resolves `counter(name)` and `counters(name, sep)` via the document's
/// [`CounterState`](elidex_style::counter::CounterState), which includes
/// the built-in `page`/`pages` counters as well as any document-defined
/// custom counters (e.g. `counter(chapter)`).
fn evaluate_content_value(
    content: &elidex_plugin::ContentValue,
    counter_state: &elidex_style::counter::CounterState,
) -> String {
    use elidex_plugin::{ContentItem, ContentValue};

    match content {
        ContentValue::Normal | ContentValue::None => String::new(),
        ContentValue::Items(items) => {
            let mut result = String::new();
            for item in items {
                match item {
                    ContentItem::String(s) => result.push_str(s),
                    ContentItem::Counter { name, style } => {
                        result.push_str(&counter_state.evaluate_counter(name, *style));
                    }
                    ContentItem::Counters {
                        name,
                        separator,
                        style,
                    } => {
                        if !result.is_empty() {
                            result.push_str(separator);
                        }
                        result
                            .push_str(&counter_state.evaluate_counters(name, separator, *style));
                    }
                    ContentItem::Attr(_) => {} // Not applicable in margin boxes.
                }
            }
            result
        }
    }
}

/// Compute the bounding area `(x, y, width, height)` for a margin box.
///
/// CSS Paged Media L3 §4.2 defines 16 margin box positions:
///
/// ```text
/// ┌──────────┬──────────┬──────────┬──────────┬──────────┐
/// │ TL corner│ top-left │top-center│top-right │ TR corner│
/// ├──────────┼──────────┴──────────┴──────────┼──────────┤
/// │ left-top │                                │ right-top│
/// ├──────────┤                                ├──────────┤
/// │left-mid  │       content area             │ right-mid│
/// ├──────────┤                                ├──────────┤
/// │left-btm  │                                │right-btm │
/// ├──────────┼──────────┬──────────┬──────────┼──────────┤
/// │ BL corner│ bot-left │bot-center│bot-right │ BR corner│
/// └──────────┴──────────┴──────────┴──────────┴──────────┘
/// ```
fn margin_box_area(
    position: &str,
    page_width: f32,
    page_height: f32,
    margins: &elidex_plugin::EdgeSizes,
) -> (f32, f32, f32, f32) {
    let content_x = margins.left;
    let content_y = margins.top;
    let content_w = (page_width - margins.left - margins.right).max(0.0);
    let content_h = (page_height - margins.top - margins.bottom).max(0.0);
    let third_w = content_w / 3.0;
    let third_h = content_h / 3.0;

    match position {
        // Top edge (3 boxes split content width into thirds)
        "top-left" => (content_x, 0.0, third_w, margins.top),
        "top-center" => (content_x + third_w, 0.0, third_w, margins.top),
        "top-right" => (content_x + 2.0 * third_w, 0.0, third_w, margins.top),

        // Bottom edge (3 boxes split content width into thirds)
        "bottom-left" => (content_x, content_y + content_h, third_w, margins.bottom),
        "bottom-center" => (
            content_x + third_w,
            content_y + content_h,
            third_w,
            margins.bottom,
        ),
        "bottom-right" => (
            content_x + 2.0 * third_w,
            content_y + content_h,
            third_w,
            margins.bottom,
        ),

        // Left edge (3 boxes split content height into thirds)
        "left-top" => (0.0, content_y, margins.left, third_h),
        "left-middle" => (0.0, content_y + third_h, margins.left, third_h),
        "left-bottom" => (0.0, content_y + 2.0 * third_h, margins.left, third_h),

        // Right edge (3 boxes split content height into thirds)
        "right-top" => (content_x + content_w, content_y, margins.right, third_h),
        "right-middle" => (
            content_x + content_w,
            content_y + third_h,
            margins.right,
            third_h,
        ),
        "right-bottom" => (
            content_x + content_w,
            content_y + 2.0 * third_h,
            margins.right,
            third_h,
        ),

        // 4 corner boxes (intersection of margin strips)
        "top-left-corner" => (0.0, 0.0, margins.left, margins.top),
        "top-right-corner" => (content_x + content_w, 0.0, margins.right, margins.top),
        "bottom-left-corner" => (0.0, content_y + content_h, margins.left, margins.bottom),
        "bottom-right-corner" => (
            content_x + content_w,
            content_y + content_h,
            margins.right,
            margins.bottom,
        ),

        _ => (0.0, 0.0, 0.0, 0.0),
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
