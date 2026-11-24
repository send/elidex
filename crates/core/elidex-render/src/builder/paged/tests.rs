use super::{evaluate_content_value, margin_box_area, resolve_margin_box_style};
use elidex_plugin::EdgeSizes;

#[test]
fn page_counter_text_rendered() {
    use elidex_plugin::{ContentItem, ListStyleType};

    let mut cs = elidex_style::counter::CounterState::new();
    cs.set_counter("page", 3);
    cs.set_counter("pages", 10);

    // Evaluate content value directly to verify counter(page) resolves.
    let text = evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "page".to_string(),
            style: ListStyleType::Decimal,
        }]),
        &cs,
    );
    assert_eq!(text, "3", "counter(page) should resolve to page number");

    let text_pages = evaluate_content_value(
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

    let text = evaluate_content_value(
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
    let text_roman = evaluate_content_value(
        &elidex_plugin::ContentValue::Items(vec![ContentItem::Counter {
            name: "chapter".to_string(),
            style: ListStyleType::UpperRoman,
        }]),
        &cs,
    );
    assert_eq!(
        text_roman, "III",
        "upper-roman style should format correctly"
    );
}

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

    let (x, y, w, h) = margin_box_area("top-left", pw, ph, &margins);
    assert!((x - margins.left).abs() < f32::EPSILON);
    assert!(y.abs() < f32::EPSILON);
    assert!((w - third).abs() < 0.01);
    assert!((h - margins.top).abs() < f32::EPSILON);

    let (x, _y, w, _h) = margin_box_area("top-center", pw, ph, &margins);
    assert!((x - (margins.left + third)).abs() < 0.01);
    assert!((w - third).abs() < 0.01);

    let (x, _y, w, _h) = margin_box_area("top-right", pw, ph, &margins);
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

    let (_x, y, _w, h) = margin_box_area("bottom-center", pw, ph, &margins);
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
    let (x, y, w, h) = margin_box_area("left-middle", pw, ph, &margins);
    assert!(x.abs() < f32::EPSILON);
    assert!((y - (margins.top + third_h)).abs() < 0.01);
    assert!((w - margins.left).abs() < f32::EPSILON);
    assert!((h - third_h).abs() < 0.01);

    // right-top
    let (x, y, w, h) = margin_box_area("right-top", pw, ph, &margins);
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
    let (x, y, w, h) = margin_box_area("top-left-corner", pw, ph, &margins);
    assert!(x.abs() < f32::EPSILON);
    assert!(y.abs() < f32::EPSILON);
    assert!((w - margins.left).abs() < f32::EPSILON);
    assert!((h - margins.top).abs() < f32::EPSILON);

    // top-right-corner
    let (x, y, w, h) = margin_box_area("top-right-corner", pw, ph, &margins);
    assert!((x - (margins.left + content_w)).abs() < f32::EPSILON);
    assert!(y.abs() < f32::EPSILON);
    assert!((w - margins.right).abs() < f32::EPSILON);
    assert!((h - margins.top).abs() < f32::EPSILON);

    // bottom-left-corner
    let (x, y, w, h) = margin_box_area("bottom-left-corner", pw, ph, &margins);
    assert!(x.abs() < f32::EPSILON);
    assert!((y - (margins.top + content_h)).abs() < f32::EPSILON);
    assert!((w - margins.left).abs() < f32::EPSILON);
    assert!((h - margins.bottom).abs() < f32::EPSILON);

    // bottom-right-corner
    let (x, y, w, h) = margin_box_area("bottom-right-corner", pw, ph, &margins);
    assert!((x - (margins.left + content_w)).abs() < f32::EPSILON);
    assert!((y - (margins.top + content_h)).abs() < f32::EPSILON);
    assert!((w - margins.right).abs() < f32::EPSILON);
    assert!((h - margins.bottom).abs() < f32::EPSILON);
}

#[test]
fn resolve_margin_box_style_defaults() {
    use elidex_plugin::{ContentValue, MarginBoxContent};

    let mb = MarginBoxContent {
        content: ContentValue::Normal,
        properties: Vec::new(),
    };
    let style = resolve_margin_box_style(&mb);
    assert!((style.font_size - 12.0).abs() < f32::EPSILON);
    assert_eq!(style.weight, 400);
    assert!(style.background_color.is_none());
    assert!(style.explicit_width.is_none());
    assert!(style.explicit_height.is_none());
    assert!((style.padding.top).abs() < f32::EPSILON);
    assert!((style.border_widths.top).abs() < f32::EPSILON);
}
