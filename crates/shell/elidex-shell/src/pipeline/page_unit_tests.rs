use crate::{build_pipeline_interactive, pipeline::build_paged_pipeline};
use elidex_plugin::{CssColor, EdgeSizes, PageGeometry, Point, Rect};
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
    let device = PageGeometry {
        sheet: elidex_plugin::Size::new(600., 400.),
        margins: EdgeSizes::uniform(100.),
    };
    let authored = || {
        result
            .stylesheets
            .iter()
            .flat_map(|s| &s.page_rules)
            .flat_map(|rule| &rule.boxes)
            .flat_map(|(_, decls)| decls.iter().map(|d| (d.property.clone(), d.value.clone())))
            .collect::<Vec<_>>()
    };
    let original = authored();
    let (interleaved, context) = build_paged_pipeline(
        &mut result.dom,
        &result.stylesheets,
        &result.font_db,
        device,
        result.engine_mode,
    );
    let fragments = elidex_layout::layout_paged(&mut result.dom, &context, &result.font_db);
    let precomputed = elidex_render::build_paged_display_lists(
        &result.dom,
        &result.font_db,
        &fragments,
        &context,
    );
    assert_eq!(authored(), original);
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

#[test]
fn first_page_area_drives_document_units_while_print_media_keeps_the_device_sheet() {
    use elidex_plugin::{ComputedStyle, CssValue, LengthUnit, Size};
    let css="html{font-size:2vw;--base:calc(1em + 1rem + 10lvw)} main{color:black;background-color:white}
        @media print and (min-width:900px){main{color:red}}
        @media print and (min-width:1001px){main{background-color:red}}
        @page{font-size:1.5em;margin:0;margin-left:1vw;@top-left{content:'1';padding-left:1rem;padding-right:10vw}}
        @page :first{size:var(--base)}";
    let mut result = build_pipeline_interactive("<main></main>", css);
    let device = PageGeometry {
        sheet: Size::new(1000., 1000.),
        margins: EdgeSizes::uniform(100.),
    };
    for (extra, sheet, area, font) in [
        ("", 120., 112., 2.24),
        (
            "@page :first {--base:calc(1em + 1rem + 10lvw + 100px)}",
            220.,
            212.,
            4.24,
        ),
    ] {
        let overrides = elidex_css::parse_stylesheet(extra, elidex_css::Origin::Author);
        let mut sheets = result.stylesheets.clone();
        sheets.push(overrides);
        let (_, pages) = build_paged_pipeline(
            &mut result.dom,
            &sheets,
            &result.font_db,
            device,
            result.engine_mode,
        );
        let first = pages.select(1, false);
        assert_eq!(first.geometry.sheet.width, sheet);
        assert_eq!(first.geometry.margins.left, 8.);
        assert_eq!(first.geometry.area().size.width, area);
        let html = result.dom.query_by_tag("html")[0];
        let main = result.dom.query_by_tag("main")[0];
        let root = result.dom.world().get::<&ComputedStyle>(html).unwrap();
        assert!((root.font_size - font).abs() < 0.001);
        let child = result.dom.world().get::<&ComputedStyle>(main).unwrap();
        assert_eq!(child.color, CssColor::RED);
        assert_eq!(child.background_color, CssColor::WHITE);
        for (prop, expected) in [("padding-left", font), ("padding-right", area * 0.1)] {
            let d = first.boxes[0]
                .1
                .properties
                .iter()
                .find(|d| d.property == prop)
                .unwrap();
            let CssValue::Length(px, LengthUnit::Px) = d.value else {
                panic!("{d:?}")
            };
            assert!((px - expected).abs() < 0.001);
        }
        let decls = &sheets
            .iter()
            .flat_map(|s| &s.page_rules)
            .find(|r| !r.boxes.is_empty())
            .unwrap()
            .boxes[0]
            .1;
        for (prop, value) in [
            ("padding-left", CssValue::Length(1., LengthUnit::Rem)),
            ("padding-right", CssValue::Length(10., LengthUnit::Vw)),
        ] {
            assert_eq!(
                decls.iter().find(|d| d.property == prop).unwrap().value,
                value
            );
        }
    }
}

#[test]
fn descendant_viewport_units_follow_each_new_first_page() {
    use elidex_plugin::{ComputedStyle, Size};
    let mut result = build_pipeline_interactive(
        "<main></main>",
        "main{font-size:10vw;color:black} @media print and (min-width:700px){main{color:red}}",
    );
    let device = PageGeometry {
        sheet: Size::new(800., 900.),
        margins: EdgeSizes::default(),
    };
    for (width, font) in [(200, 18.), (300, 28.)] {
        let mut sheets = result.stylesheets.clone();
        sheets.push(elidex_css::parse_stylesheet(
            &format!("@page {{size:600px}} @page :first {{size:{width}px; margin:0 10px}}"),
            elidex_css::Origin::Author,
        ));
        let (_, pages) = build_paged_pipeline(
            &mut result.dom,
            &sheets,
            &result.font_db,
            device,
            result.engine_mode,
        );
        assert_eq!(pages.select(2, false).geometry.sheet.width, 600.);
        let main = result.dom.query_by_tag("main")[0];
        let published = result.dom.world().get::<&ComputedStyle>(main).unwrap();
        assert_eq!(published.color, CssColor::RED);
        assert_eq!(published.font_size, font);
    }
}
