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
    // The margin box should have produced at least one display item
    // (the transparent placeholder rect for the header text).
    assert!(
        !paged_dl.pages[0].is_empty(),
        "page should have margin box items"
    );
}

#[test]
fn page_counter_text_rendered() {
    use elidex_plugin::{ContentItem, ListStyleType};

    // Evaluate content value directly to verify counter(page) resolves.
    let text = super::super::evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "page".to_string(),
            style: ListStyleType::Decimal,
        }]),
        3,
        10,
    );
    assert_eq!(text, "3", "counter(page) should resolve to page number");

    let text_pages = super::super::evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "pages".to_string(),
            style: ListStyleType::Decimal,
        }]),
        3,
        10,
    );
    assert_eq!(text_pages, "10", "counter(pages) should resolve to total");
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
