use crate::{build_pipeline_interactive, pipeline::build_paged_pipeline};
use elidex_plugin::{CssColor, EdgeSizes, PagedMediaContext, Point, Rect};
use elidex_render::{DisplayItem, PagedDisplayList};

#[derive(Debug, PartialEq)]
struct BoxPaint {
    rectangles: Vec<(Rect, CssColor)>,
    text: Vec<(Point, f32)>,
}

fn paint(list: &PagedDisplayList) -> BoxPaint {
    let page = list.pages.first().expect("first printed page");
    let rectangles = page
        .iter()
        .filter_map(|item| match item {
            DisplayItem::SolidRect { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let text = page
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text {
                glyphs, font_size, ..
            } => Some((glyphs.first().expect("painted glyph").position, *font_size)),
            _ => None,
        })
        .collect();
    assert!(!rectangles.is_empty());
    BoxPaint { rectangles, text }
}

fn printed_box(properties: &str) -> BoxPaint {
    // Margin boxes resolve a single family. Choose an installed face so the
    // geometry assertions exercise glyph painting on every CI platform.
    let fonts = elidex_text::FontDatabase::new();
    let normal = elidex_text::to_fontdb_style(elidex_plugin::FontStyle::Normal);
    let family = [
        "Arial",
        "Liberation Sans",
        "DejaVu Sans",
        "Helvetica",
        "Noto Sans",
    ]
    .into_iter()
    .find(|name| fonts.query(&[name], 400, normal).is_some())
    .expect("an installed test font");
    let css = format!("html {{font-size:16px}} @page {{font-size:20px; @top-left {{content:'X';font-family:'{family}';background-color:red; border-style:solid; {properties} }} }}");
    let mut result = build_pipeline_interactive("<main></main>", &css);
    let context = PagedMediaContext {
        page_width: 600.,
        page_height: 400.,
        page_margins: EdgeSizes::uniform(100.),
        page_rules: result
            .stylesheets
            .iter()
            .flat_map(|s| s.page_rules.iter().cloned())
            .collect(),
    };
    let original = context.page_rules[0]
        .margins
        .top_left
        .as_ref()
        .unwrap()
        .properties
        .clone();
    let interleaved = build_paged_pipeline(
        &mut result.dom,
        &result.stylesheets,
        &result.font_db,
        &context,
        &result.registry,
        result.engine_mode,
    );
    let fragments = elidex_layout::layout_paged(&mut result.dom, &context, &result.font_db);
    let precomputed = elidex_render::build_paged_display_lists(
        &result.dom,
        &result.font_db,
        &fragments,
        &context,
    );
    assert_eq!(
        context.page_rules[0]
            .margins
            .top_left
            .as_ref()
            .unwrap()
            .properties,
        original
    );
    let painted = paint(&interleaved);
    assert_eq!(paint(&precomputed), painted);
    painted
}

#[test]
fn stylesheet_lengths_reach_printed_margin_box_geometry() {
    let fixed = printed_box(
        "font-size:30px; width:96px; height:80px; border-width:1px; padding:20px 16px 20px 30px",
    );
    assert_eq!(fixed.text[0].1, 30.);
    for (width_axis, height_axis) in [
        ("vw", "vh"),
        ("lvw", "lvh"),
        ("vmax", "vmin"),
        ("lvmax", "lvmin"),
    ] {
        let actual = printed_box(&format!("font-size:1.5em; width:1in; height:20{width_axis}; border-width:0.75pt; padding:10{height_axis} 1rem 10{height_axis} 1em"));
        assert_eq!(actual, fixed, "{width_axis}/{height_axis}");
    }
}

#[test]
fn inch_padding_moves_the_glyph_by_the_same_distance_as_pixels() {
    let base = printed_box("text-align:left;padding-left:0px");
    let inch = printed_box("text-align:left;padding-left:1in");
    assert_eq!(inch, printed_box("text-align:left;padding-left:96px"));
    assert_eq!(inch.text[0].0.x - base.text[0].0.x, 96.);
}

#[test]
fn zero_margin_font_preserves_background_and_borders_without_text() {
    let sample = |size| {
        printed_box(&format!(
            "font-size:{size};width:90px;height:60px;border:2px solid black"
        ))
    };
    let positive = sample("12px");
    assert_eq!(positive.text.len(), 1);
    assert_eq!(positive.rectangles.len(), 5);
    assert_eq!(positive.rectangles[0].1, CssColor::RED);
    let background = positive.rectangles[0].0;
    let Point { x, y } = background.origin;
    let width = background.size.width;
    let height = background.size.height;
    for edge in [
        Rect::new(x, y, width, 2.0),
        Rect::new(x, y + height - 2.0, width, 2.0),
        Rect::new(x, y + 2.0, 2.0, height - 4.0),
        Rect::new(x + width - 2.0, y + 2.0, 2.0, height - 4.0),
    ] {
        assert!(positive.rectangles.contains(&(edge, CssColor::BLACK)));
    }
    for size in ["0in", "0px"] {
        let zero = sample(size);
        assert!(zero.text.is_empty());
        assert_eq!(zero.rectangles, positive.rectangles);
    }
}
