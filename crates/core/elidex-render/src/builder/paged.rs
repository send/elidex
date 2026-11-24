//! Paged display-list construction and margin-box painting.

use super::{find_roots, place_glyphs, walk, PaintContext};
use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;
use elidex_ecs::EcsDom;
use elidex_text::FontDatabase;

#[cfg(test)]
mod tests;

/// Build a multi-page display list from paged layout fragments.
///
/// CSS Paged Media Level 3: for each [`elidex_layout::PageFragment`], a display list is
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
    page_ctx: &elidex_plugin::PreparedPages,
) -> crate::display_list::PagedDisplayList {
    use crate::display_list::{DisplayList, PagedDisplayList};

    let total_pages = page_fragments.len();
    let mut pages = Vec::with_capacity(total_pages);

    for fragment in page_fragments {
        let page = page_ctx.select(fragment.page_number, fragment.is_blank);
        let page_width = page.geometry.sheet.width;
        let page_height = page.geometry.sheet.height;
        let margins = page.geometry.margins;

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
                paged: true,
                expected_generation: None,
                continuation_entities: None,
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
            &page.boxes,
            page_width,
            page_height,
            &margins,
            &counter_state,
        );

        pages.push(dl);
    }

    PagedDisplayList {
        pages,
        page_size: page_ctx.select(1, false).geometry.sheet,
    }
}

/// Build a multi-page display list with per-fragment layout interleaving.
///
/// Solves three problems with the simpler [`build_paged_display_lists`]:
///
/// 1. **Layout overwrites**: Each fragment's layout overwrites ECS `LayoutBox`
///    components. By interleaving layout and render per page, each page sees
///    the correct positions.
///
/// 2. **Fragment-scoped walk**: Only entities belonging to the current page
///    fragment are rendered, via `layout_generation` tagging.
///
/// 3. **Continuation detection**: Entities split across page breaks suppress
///    `counter-increment` on continuation fragments (CSS Fragmentation L3 §4).
///
/// # Two-phase approach
///
/// - **Phase 1**: `layout_fragmented_with_tokens()` determines page count and
///   captures the break token chain. Page count is needed for `counter(pages)`.
/// - **Phase 2**: For each page, re-layout with a unique generation, then
///   walk (skipping non-matching generations) and render margin boxes.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_paged_display_lists_interleaved(
    dom: &mut EcsDom,
    font_db: &FontDatabase,
    page_ctx: &elidex_plugin::PreparedPages,
) -> crate::display_list::PagedDisplayList {
    use crate::display_list::{DisplayList, PagedDisplayList};

    let roots = find_roots(dom);
    let Some(root) = roots.first().copied() else {
        return PagedDisplayList {
            pages: Vec::new(),
            page_size: page_ctx.select(1, false).geometry.sheet,
        };
    };

    let area = page_ctx.select(1, false).geometry.area();
    let content_width = area.size.width;
    let content_height = area.size.height;
    if content_height <= 0.0 || content_width <= 0.0 {
        return PagedDisplayList {
            pages: Vec::new(),
            page_size: page_ctx.select(1, false).geometry.sheet,
        };
    }

    let frag_ctx = elidex_layout_block::FragmentainerContext {
        available_block_size: content_height,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };

    let base_input = elidex_layout_block::LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        is_flex_item: false,
        assigned: elidex_layout_block::UsedAssignment::default(),
        containing: elidex_plugin::CssSize::definite(content_width, content_height),
        containing_inline_size: content_width,
        offset: area.origin,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(area.size),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        medium: elidex_plugin::Medium::Print,
        placement: elidex_layout_block::FragmentPlacement::Normal,
        is_probe: false,
    };

    // Phase 1: determine page count + break token chain.
    let (phase1_fragments, tokens) =
        elidex_layout::layout_fragmented_with_tokens(dom, root, &base_input, frag_ctx);
    let total_pages = phase1_fragments.len();

    // Phase 2: interleaved layout + render per page.
    let mut pages = Vec::with_capacity(total_pages);
    let mut counter_state = elidex_style::counter::CounterState::new();

    for page_num in 1..=total_pages {
        let page_index = page_num - 1;
        let prev_token = tokens.get(page_index).and_then(|t| t.as_ref());
        #[allow(clippy::cast_possible_truncation)]
        let generation = page_num as u32;

        // Re-layout this fragment with the correct generation stamp.
        let frag_input = elidex_layout_block::LayoutInput {
            fragmentainer: Some(&frag_ctx),
            break_token: prev_token,
            layout_generation: generation,
            is_probe: false,
            ..base_input
        };
        let outcome = elidex_layout::dispatch_layout_child(dom, root, &frag_input);
        let is_blank = outcome.layout_box.content.size.height < 0.5
            && outcome.layout_box.content.size.width < 0.5;

        let page = page_ctx.select(page_num, is_blank);
        let page_width = page.geometry.sheet.width;
        let page_height = page.geometry.sheet.height;
        let margins = page.geometry.margins;

        // Set page/pages counters.
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let page_i32 = page_num.min(i32::MAX as usize) as i32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let total_i32 = total_pages.min(i32::MAX as usize) as i32;
        counter_state.set_counter("page", page_i32);
        counter_state.set_counter("pages", total_i32);

        let mut dl = DisplayList::default();
        let mut font_cache = FontCache::new();

        if !is_blank {
            // Build continuation entity set from the previous break token.
            let continuations = collect_continuation_entities(prev_token);

            let mut ctx = PaintContext {
                dom: &*dom,
                font_db,
                font_cache: &mut font_cache,
                dl: &mut dl,
                caret_visible: false,
                scroll_offset: elidex_plugin::Vector::<f32>::ZERO,
                counter_state,
                paged: true,
                expected_generation: Some(generation),
                continuation_entities: if continuations.is_empty() {
                    None
                } else {
                    Some(continuations)
                },
            };

            let roots = find_roots(&*dom);
            for r in roots {
                walk(
                    &mut ctx,
                    r,
                    0,
                    &elidex_plugin::transform_math::Perspective::default(),
                    false,
                );
            }
            counter_state = ctx.counter_state;
        }

        // Render margin boxes.
        emit_margin_boxes(
            &mut dl,
            font_db,
            &mut font_cache,
            &page.boxes,
            page_width,
            page_height,
            &margins,
            &counter_state,
        );

        pages.push(dl);

        // Flatten counter state for cross-page persistence.
        counter_state.flatten();
    }

    PagedDisplayList {
        pages,
        page_size: page_ctx.select(1, false).geometry.sheet,
    }
}

/// Collect entities that are continuations from a previous fragment.
///
/// Walks the break token chain and collects all entities mentioned.
/// These entities should suppress `counter-increment` on the current page
/// per CSS Fragmentation L3 §4.
fn collect_continuation_entities(
    break_token: Option<&elidex_layout_block::BreakToken>,
) -> std::collections::HashSet<elidex_ecs::Entity> {
    let mut set = std::collections::HashSet::new();
    let mut stack = Vec::new();
    if let Some(bt) = break_token {
        stack.push(bt);
    }
    while let Some(bt) = stack.pop() {
        set.insert(bt.entity);
        if let Some(elidex_layout_block::BreakTokenData::Block {
            continuation: elidex_layout_block::BlockContinuation::InterruptedChild(child_bt),
            ..
        }) = &bt.mode_data
        {
            stack.push(child_bt);
        }
    }
    set
}

/// Local renderer default font size for margin-box text.
const MARGIN_BOX_FONT_SIZE: f32 = 12.0;

/// Emit margin box content items (text strings, counter values) for a page.
///
/// Renders the selected margin boxes for this page. Each box produces
/// shaped text positioned in one of the 16 margin areas
/// defined by CSS Paged Media L3 §5, Table 1.
#[allow(clippy::too_many_arguments)]
fn emit_margin_boxes(
    dl: &mut crate::display_list::DisplayList,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    boxes: &[(String, elidex_plugin::MarginBoxContent)],
    page_width: f32,
    page_height: f32,
    margins: &elidex_plugin::EdgeSizes,
    counter_state: &elidex_style::counter::CounterState,
) {
    for (position, margin_box) in boxes {
        let text = evaluate_content_value(&margin_box.content, counter_state);
        if text.is_empty() {
            continue;
        }

        emit_margin_box_text(
            dl,
            font_db,
            font_cache,
            position,
            &text,
            margin_box,
            page_width,
            page_height,
            margins,
        );
    }
}

/// Resolved paint inputs for a margin box.
struct MarginBoxStyle {
    font_size: f32,
    color: elidex_plugin::CssColor,
    family: String,
    weight: u16,
    font_style: fontdb::Style,
    text_align: elidex_plugin::TextAlign,
    padding: elidex_plugin::EdgeSizes,
    border_widths: elidex_plugin::EdgeSizes,
    border_color: elidex_plugin::CssColor,
    background_color: Option<elidex_plugin::CssColor>,
    explicit_width: Option<f32>,
    explicit_height: Option<f32>,
}

impl Default for MarginBoxStyle {
    fn default() -> Self {
        Self {
            font_size: MARGIN_BOX_FONT_SIZE,
            color: elidex_plugin::CssColor::BLACK,
            family: String::from("serif"),
            weight: 400,
            font_style: fontdb::Style::Normal,
            text_align: elidex_plugin::TextAlign::Center,
            padding: elidex_plugin::EdgeSizes::default(),
            border_widths: elidex_plugin::EdgeSizes::default(),
            border_color: elidex_plugin::CssColor::BLACK,
            background_color: None,
            explicit_width: None,
            explicit_height: None,
        }
    }
}

/// Resolve all margin box properties from declarations.
#[allow(clippy::too_many_lines)]
fn resolve_margin_box_style(margin_box: &elidex_plugin::MarginBoxContent) -> MarginBoxStyle {
    use elidex_plugin::{CssValue, LengthUnit};

    let mut s = MarginBoxStyle::default();
    for decl in &margin_box.properties {
        match decl.property.as_str() {
            "font-size" => {
                let v = match &decl.value {
                    CssValue::Length(v, LengthUnit::Px) | CssValue::Number(v) => Some(*v),
                    _ => None,
                };
                if let Some(v) = v {
                    if v.is_finite() && v >= 0.0 {
                        s.font_size = v;
                    }
                }
            }
            "color" => {
                if let CssValue::Color(c) = &decl.value {
                    s.color = *c;
                }
            }
            "font-family" => {
                let resolved = match &decl.value {
                    CssValue::String(v) | CssValue::Keyword(v) => Some(v),
                    CssValue::List(items) => items.first().and_then(|f| match f {
                        CssValue::String(v) | CssValue::Keyword(v) => Some(v),
                        _ => None,
                    }),
                    _ => None,
                };
                if let Some(v) = resolved {
                    s.family.clone_from(v);
                }
            }
            "font-weight" => match &decl.value {
                CssValue::Number(w) => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let w_int = w.clamp(1.0, 1000.0) as u16;
                    s.weight = w_int;
                }
                CssValue::Keyword(k) => match k.as_str() {
                    "bold" => s.weight = 700,
                    "normal" => s.weight = 400,
                    _ => {}
                },
                _ => {}
            },
            "font-style" => {
                if let CssValue::Keyword(k) = &decl.value {
                    match k.as_str() {
                        "italic" => s.font_style = fontdb::Style::Italic,
                        "oblique" => s.font_style = fontdb::Style::Oblique,
                        "normal" => s.font_style = fontdb::Style::Normal,
                        _ => {}
                    }
                }
            }
            "text-align" => {
                if let CssValue::Keyword(k) = &decl.value {
                    match k.as_str() {
                        "left" => s.text_align = elidex_plugin::TextAlign::Left,
                        "center" => s.text_align = elidex_plugin::TextAlign::Center,
                        "right" => s.text_align = elidex_plugin::TextAlign::Right,
                        _ => {}
                    }
                }
            }
            "padding-top" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    s.padding.top = v.max(0.0);
                }
            }
            "padding-right" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    s.padding.right = v.max(0.0);
                }
            }
            "padding-bottom" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    s.padding.bottom = v.max(0.0);
                }
            }
            "padding-left" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    s.padding.left = v.max(0.0);
                }
            }
            "border-width"
            | "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    let w = v.max(0.0);
                    match decl.property.as_str() {
                        "border-width" => {
                            s.border_widths = elidex_plugin::EdgeSizes::new(w, w, w, w);
                        }
                        "border-top-width" => s.border_widths.top = w,
                        "border-right-width" => s.border_widths.right = w,
                        "border-bottom-width" => s.border_widths.bottom = w,
                        "border-left-width" => s.border_widths.left = w,
                        _ => {}
                    }
                }
            }
            "border-color" => {
                if let CssValue::Color(c) = &decl.value {
                    s.border_color = *c;
                }
            }
            "background-color" => {
                if let CssValue::Color(c) = &decl.value {
                    s.background_color = Some(*c);
                }
            }
            "width" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    if v.is_finite() && *v >= 0.0 {
                        s.explicit_width = Some(*v);
                    }
                }
            }
            "height" => {
                if let CssValue::Length(v, LengthUnit::Px) = &decl.value {
                    if v.is_finite() && *v >= 0.0 {
                        s.explicit_height = Some(*v);
                    }
                }
            }
            _ => {}
        }
    }
    s
}

/// Shape and emit a single margin box with full box model support.
///
/// Resolves all CSS properties from the margin box's declarations and renders:
/// - Background color rectangle
/// - Border rectangles (4 sides)
/// - Shaped text positioned by `text-align` within the content area
///
/// The content area is the margin box area minus padding and border.
/// If `width`/`height` are explicitly set, the box is constrained and
/// centered within the allocated margin area.
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
    use elidex_plugin::Rect;

    let s = resolve_margin_box_style(margin_box);

    // Compute the allocated area for this margin box position.
    let (alloc_x, alloc_y, alloc_w, alloc_h) =
        margin_box_area(position, page_width, page_height, margins);

    // Apply explicit width/height constraints, centering within the allocated area.
    let box_w = s.explicit_width.unwrap_or(alloc_w).min(alloc_w);
    let box_h = s.explicit_height.unwrap_or(alloc_h).min(alloc_h);
    let box_x = alloc_x + (alloc_w - box_w) * 0.5;
    let box_y = alloc_y + (alloc_h - box_h) * 0.5;

    // Background (covers border box = full box area).
    if let Some(bg) = s.background_color {
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(box_x, box_y, box_w, box_h),
            color: bg,
        });
    }

    // Borders (4 sides as thin rectangles).
    let bw = &s.border_widths;
    if bw.top > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(box_x, box_y, box_w, bw.top),
            color: s.border_color,
        });
    }
    if bw.bottom > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(box_x, box_y + box_h - bw.bottom, box_w, bw.bottom),
            color: s.border_color,
        });
    }
    if bw.left > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(box_x, box_y + bw.top, bw.left, box_h - bw.top - bw.bottom),
            color: s.border_color,
        });
    }
    if bw.right > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(
                box_x + box_w - bw.right,
                box_y + bw.top,
                bw.right,
                box_h - bw.top - bw.bottom,
            ),
            color: s.border_color,
        });
    }

    // Content area = box minus border minus padding.
    let content_x = box_x + bw.left + s.padding.left;
    let content_y = box_y + bw.top + s.padding.top;
    let content_w = (box_w - bw.left - bw.right - s.padding.left - s.padding.right).max(0.0);
    let content_h = (box_h - bw.top - bw.bottom - s.padding.top - s.padding.bottom).max(0.0);

    if content_w <= 0.0 || content_h <= 0.0 || s.font_size == 0.0 {
        return;
    }

    // Shape text.
    let families = [s.family.as_str()];
    let Some(font_id) = font_db.query(&families, s.weight, s.font_style) else {
        return;
    };
    let Some(shaped) = elidex_text::shape_text(font_db, font_id, s.font_size, text) else {
        return;
    };
    let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
        return;
    };

    let text_width: f32 = shaped.glyphs.iter().map(|g| g.x_advance).sum();
    if !text_width.is_finite() {
        return;
    }

    // Horizontal alignment within content area.
    let text_offset_x = match s.text_align {
        elidex_plugin::TextAlign::Left | elidex_plugin::TextAlign::Start => 0.0,
        elidex_plugin::TextAlign::Center | elidex_plugin::TextAlign::Justify => {
            (content_w - text_width) * 0.5
        }
        elidex_plugin::TextAlign::Right | elidex_plugin::TextAlign::End => content_w - text_width,
    };
    let mut text_x = content_x + text_offset_x.max(0.0);
    let baseline_y = content_y + f32::midpoint(content_h, s.font_size);

    let glyphs = place_glyphs(&shaped.glyphs, &mut text_x, baseline_y, 0.0, 0.0, text);
    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size: s.font_size,
        color: s.color,
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
                        result.push_str(&counter_state.evaluate_counters(name, separator, *style));
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
/// CSS Paged Media L3 §5, Table 1 defines 16 margin box positions:
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
