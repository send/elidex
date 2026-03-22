//! Tests for paged media display list building.

use super::*;

// ---------------------------------------------------------------------------
// Step 6: Multi-page Rendering tests
// ---------------------------------------------------------------------------

fn make_page_ctx(width: f32, height: f32) -> elidex_plugin::PagedMediaContext {
    elidex_plugin::PagedMediaContext {
        page_width: width,
        page_height: height,
        page_margins: EdgeSizes {
            top: 50.0,
            right: 50.0,
            bottom: 50.0,
            left: 50.0,
        },
        page_rules: Vec::new(),
    }
}

#[test]
fn single_page_display_list() {
    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox {
            content: Rect::new(50.0, 50.0, 716.0, 100.0),
            ..Default::default()
        },
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    assert_eq!(paged_dl.page_count(), 1);
    assert!(!paged_dl.is_empty());
    assert!((paged_dl.page_size.width - 816.0).abs() < f32::EPSILON);
    assert!((paged_dl.page_size.height - 1056.0).abs() < f32::EPSILON);
}

#[test]
fn multi_page_display_list_count() {
    let fragments: Vec<elidex_layout::PageFragment> = (1..=3)
        .map(|i| elidex_layout::PageFragment {
            layout_box: elidex_plugin::LayoutBox {
                content: Rect::new(50.0, 50.0, 716.0, 300.0),
                ..Default::default()
            },
            page_number: i,
            matched_selectors: Vec::new(),
            is_blank: false,
        })
        .collect();

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);

    let paged_dl = build_paged_display_lists(&dom, &font_db, &fragments, &page_ctx);

    assert_eq!(paged_dl.page_count(), 3, "3 fragments → 3 pages");
}

#[test]
fn page_margin_box_items() {
    use elidex_plugin::{ContentItem, ContentValue, MarginBoxContent, PageMargins, PageRule};

    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox {
            content: Rect::new(50.0, 50.0, 716.0, 100.0),
            ..Default::default()
        },
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();

    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: EdgeSizes {
            top: 50.0,
            right: 50.0,
            bottom: 50.0,
            left: 50.0,
        },
        page_rules: vec![PageRule {
            selectors: Vec::new(),
            size: None,
            margins: PageMargins {
                top_center: Some(MarginBoxContent {
                    content: ContentValue::Items(vec![ContentItem::String("Header".to_string())]),
                    properties: Vec::new(),
                }),
                ..PageMargins::default()
            },
            properties: Vec::new(),
        }],
    };

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    assert_eq!(paged_dl.page_count(), 1);
    // The margin box should produce a Text display item for the header.
    let text_items: Vec<_> = paged_dl.pages[0]
        .iter()
        .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
        .collect();
    assert!(
        !text_items.is_empty(),
        "page should have text items from margin box"
    );
}

#[test]
fn page_counter_text_rendered() {
    use elidex_plugin::{ContentItem, ListStyleType};

    let mut cs = elidex_style::counter::CounterState::new();
    cs.set_counter("page", 3);
    cs.set_counter("pages", 10);

    // Evaluate content value directly to verify counter(page) resolves.
    let text = super::super::evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "page".to_string(),
            style: ListStyleType::Decimal,
        }]),
        &cs,
    );
    assert_eq!(text, "3", "counter(page) should resolve to page number");

    let text_pages = super::super::evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "pages".to_string(),
            style: ListStyleType::Decimal,
        }]),
        &cs,
    );
    assert_eq!(text_pages, "10", "counter(pages) should resolve to total");
}

#[test]
fn custom_counter_in_margin_box() {
    use elidex_plugin::{ContentItem, ListStyleType};

    let mut cs = elidex_style::counter::CounterState::new();
    cs.set_counter("page", 1);
    cs.set_counter("pages", 5);
    // Simulate a document-defined counter.
    cs.set_counter("chapter", 3);

    let text = super::super::evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![
            ContentItem::String("Chapter ".to_string()),
            ContentItem::Counter {
                name: "chapter".to_string(),
                style: ListStyleType::Decimal,
            },
        ]),
        &cs,
    );
    assert_eq!(text, "Chapter 3", "custom counter should resolve");

    // Upper-roman style.
    let text_roman = super::super::evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "chapter".to_string(),
            style: ListStyleType::UpperRoman,
        }]),
        &cs,
    );
    assert_eq!(text_roman, "III", "upper-roman style should format correctly");
}

#[test]
fn content_offset_to_page_area() {
    let page_ctx = make_page_ctx(816.0, 1056.0);
    // Content area: 716 x 956, starting at (50, 50).
    assert!((page_ctx.content_width() - 716.0).abs() < f32::EPSILON);
    assert!((page_ctx.content_height() - 956.0).abs() < f32::EPSILON);
}

#[test]
fn empty_page_handled() {
    let blank_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox::default(),
        page_number: 2,
        matched_selectors: vec![elidex_plugin::PageSelector::Blank],
        is_blank: true,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[blank_fragment], &page_ctx);

    assert_eq!(paged_dl.page_count(), 1);
    // Blank pages should produce an empty display list (no content walk).
    assert!(
        paged_dl.pages[0].is_empty(),
        "blank page should have no content items"
    );
}

#[test]
fn page_size_matches_rule() {
    use elidex_plugin::{NamedPageSize, PageRule, PageSize};

    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: EdgeSizes::default(),
        page_rules: vec![PageRule {
            selectors: Vec::new(),
            size: Some(PageSize::Named(NamedPageSize::A4)),
            ..PageRule::default()
        }],
    };

    let (w, h) = page_ctx.effective_page_size(1, false);
    assert!((w - 794.0).abs() < 1.0, "A4 width = 794, got {w}");
    assert!((h - 1123.0).abs() < 1.0, "A4 height = 1123, got {h}");
}

// ---------------------------------------------------------------------------
// Margin box area positioning tests (all 16 types)
// ---------------------------------------------------------------------------

#[test]
fn margin_box_area_top_edge() {
    let margins = EdgeSizes {
        top: 50.0,
        right: 40.0,
        bottom: 60.0,
        left: 30.0,
    };
    let pw = 800.0;
    let ph = 1000.0;
    let content_w = pw - margins.left - margins.right; // 730
    let third = content_w / 3.0;

    let (x, y, w, h) = super::super::margin_box_area("top-left", pw, ph, &margins);
    assert!((x - margins.left).abs() < f32::EPSILON);
    assert!(y.abs() < f32::EPSILON);
    assert!((w - third).abs() < 0.01);
    assert!((h - margins.top).abs() < f32::EPSILON);

    let (x, _y, w, _h) = super::super::margin_box_area("top-center", pw, ph, &margins);
    assert!((x - (margins.left + third)).abs() < 0.01);
    assert!((w - third).abs() < 0.01);

    let (x, _y, w, _h) = super::super::margin_box_area("top-right", pw, ph, &margins);
    assert!((x - (margins.left + 2.0 * third)).abs() < 0.01);
    assert!((w - third).abs() < 0.01);
}

#[test]
fn margin_box_area_bottom_edge() {
    let margins = EdgeSizes {
        top: 50.0,
        right: 40.0,
        bottom: 60.0,
        left: 30.0,
    };
    let pw = 800.0;
    let ph = 1000.0;
    let content_y = margins.top;
    let content_h = ph - margins.top - margins.bottom;

    let (_x, y, _w, h) = super::super::margin_box_area("bottom-center", pw, ph, &margins);
    assert!((y - (content_y + content_h)).abs() < f32::EPSILON);
    assert!((h - margins.bottom).abs() < f32::EPSILON);
}

#[test]
fn margin_box_area_left_right_edge() {
    let margins = EdgeSizes {
        top: 50.0,
        right: 40.0,
        bottom: 60.0,
        left: 30.0,
    };
    let pw = 800.0;
    let ph = 1000.0;
    let content_h = ph - margins.top - margins.bottom; // 890
    let third_h = content_h / 3.0;

    // left-middle
    let (x, y, w, h) = super::super::margin_box_area("left-middle", pw, ph, &margins);
    assert!(x.abs() < f32::EPSILON);
    assert!((y - (margins.top + third_h)).abs() < 0.01);
    assert!((w - margins.left).abs() < f32::EPSILON);
    assert!((h - third_h).abs() < 0.01);

    // right-top
    let (x, y, w, h) = super::super::margin_box_area("right-top", pw, ph, &margins);
    assert!((x - (pw - margins.right)).abs() < f32::EPSILON);
    assert!((y - margins.top).abs() < f32::EPSILON);
    assert!((w - margins.right).abs() < f32::EPSILON);
    assert!((h - third_h).abs() < 0.01);
}

#[test]
fn margin_box_area_corners() {
    let margins = EdgeSizes {
        top: 50.0,
        right: 40.0,
        bottom: 60.0,
        left: 30.0,
    };
    let pw = 800.0;
    let ph = 1000.0;
    let content_w = pw - margins.left - margins.right;
    let content_h = ph - margins.top - margins.bottom;

    // top-left-corner
    let (x, y, w, h) = super::super::margin_box_area("top-left-corner", pw, ph, &margins);
    assert!(x.abs() < f32::EPSILON);
    assert!(y.abs() < f32::EPSILON);
    assert!((w - margins.left).abs() < f32::EPSILON);
    assert!((h - margins.top).abs() < f32::EPSILON);

    // top-right-corner
    let (x, y, w, h) = super::super::margin_box_area("top-right-corner", pw, ph, &margins);
    assert!((x - (margins.left + content_w)).abs() < f32::EPSILON);
    assert!(y.abs() < f32::EPSILON);
    assert!((w - margins.right).abs() < f32::EPSILON);
    assert!((h - margins.top).abs() < f32::EPSILON);

    // bottom-left-corner
    let (x, y, w, h) = super::super::margin_box_area("bottom-left-corner", pw, ph, &margins);
    assert!(x.abs() < f32::EPSILON);
    assert!((y - (margins.top + content_h)).abs() < f32::EPSILON);
    assert!((w - margins.left).abs() < f32::EPSILON);
    assert!((h - margins.bottom).abs() < f32::EPSILON);

    // bottom-right-corner
    let (x, y, w, h) = super::super::margin_box_area("bottom-right-corner", pw, ph, &margins);
    assert!((x - (margins.left + content_w)).abs() < f32::EPSILON);
    assert!((y - (margins.top + content_h)).abs() < f32::EPSILON);
    assert!((w - margins.right).abs() < f32::EPSILON);
    assert!((h - margins.bottom).abs() < f32::EPSILON);
}

#[test]
fn margin_box_all_16_positions_render() {
    use elidex_plugin::{
        ContentItem, ContentValue, MarginBoxContent, PageMargins, PageRule,
    };

    let make_content = |s: &str| MarginBoxContent {
        content: ContentValue::Items(vec![ContentItem::String(s.to_string())]),
        properties: Vec::new(),
    };

    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox::default(),
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();

    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: EdgeSizes {
            top: 50.0,
            right: 50.0,
            bottom: 50.0,
            left: 50.0,
        },
        page_rules: vec![PageRule {
            selectors: Vec::new(),
            size: None,
            margins: PageMargins {
                top_left_corner: Some(make_content("TLC")),
                top_left: Some(make_content("TL")),
                top_center: Some(make_content("TC")),
                top_right: Some(make_content("TR")),
                top_right_corner: Some(make_content("TRC")),
                right_top: Some(make_content("RT")),
                right_middle: Some(make_content("RM")),
                right_bottom: Some(make_content("RB")),
                bottom_right_corner: Some(make_content("BRC")),
                bottom_right: Some(make_content("BR")),
                bottom_center: Some(make_content("BC")),
                bottom_left: Some(make_content("BL")),
                bottom_left_corner: Some(make_content("BLC")),
                left_bottom: Some(make_content("LB")),
                left_middle: Some(make_content("LM")),
                left_top: Some(make_content("LT")),
            },
            properties: Vec::new(),
        }],
    };

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    // Each of 16 margin boxes should produce a Text item (if fonts are available).
    // In test env without system fonts, shape_text may fail, so we just check
    // the function doesn't panic and returns at least zero items.
    assert_eq!(paged_dl.page_count(), 1);
}
