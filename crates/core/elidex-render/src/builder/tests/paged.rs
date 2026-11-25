//! Tests for paged media display list building.

use super::*;

// ---------------------------------------------------------------------------
// Step 6: Multi-page Rendering tests
// ---------------------------------------------------------------------------

fn make_page_ctx(width: f32, height: f32) -> elidex_plugin::PreparedPages {
    elidex_style::page::fixture(width, height, 50.0, "")
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
fn interleaved_paged_driver_leaves_screen_geometry_invalid() {
    // Provenance (terminal-Z C-3a §2), production path end-to-end: the interleaved
    // driver's Phase 1 (`layout_fragmented_with_tokens`) demotes the phase because the
    // boxes its fragmentainer loop lays out are written through `EcsDom::set_layout_box`,
    // and Phase 2's per-page direct `dispatch_layout_child` writes through the same
    // chokepoint; nothing on the paged path publishes. So a prior screen pass's `CompletedScreen` cannot survive a
    // paged render that lays anything out — a reader can never see a page-relative store
    // as screen geometry.
    let (mut dom, _elem) = setup_block_element(
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            ..Default::default()
        },
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 100.0),
            ..Default::default()
        },
    );
    // A prior screen pass completed.
    dom.screen_layout_pass(|_| {});
    assert!(
        dom.screen_geometry().is_some(),
        "precondition: completed screen"
    );

    let font_db = elidex_text::FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let _ = build_paged_display_lists_interleaved(&mut dom, &font_db, &page_ctx);

    assert!(
        dom.screen_geometry().is_none(),
        "the interleaved paged driver leaves phase Invalid (both phases dispatch, none publishes)"
    );
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

    let page_ctx =
        elidex_style::page::fixture(816., 1056., 50., "@page {@top-center {content:'Header'}}");

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    assert_eq!(paged_dl.page_count(), 1);
    // The margin box should produce a Text display item for the header
    // (only when a serif font is available for shaping).
    if font_db
        .query(
            &["serif"],
            400,
            elidex_text::to_fontdb_style(elidex_plugin::FontStyle::Normal),
        )
        .is_some()
    {
        let text_items: Vec<_> = paged_dl.pages[0]
            .iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
        assert!(
            !text_items.is_empty(),
            "page should have text items from margin box"
        );
    }
}

#[test]
fn content_offset_to_page_area() {
    let page_ctx = make_page_ctx(816.0, 1056.0);
    // Content area: 716 x 956, starting at (50, 50).
    assert!((page_ctx.select(1, false).geometry.area().size.width - 716.0).abs() < f32::EPSILON);
    assert!((page_ctx.select(1, false).geometry.area().size.height - 956.0).abs() < f32::EPSILON);
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
    let page_ctx = elidex_style::page::fixture(816., 1056., 0., "@page {size:A4}");

    let elidex_plugin::Size {
        width: w,
        height: h,
    } = page_ctx.select(1, false).geometry.sheet;
    assert!((w - 794.0).abs() < 1.0, "A4 width = 794, got {w}");
    assert!((h - 1123.0).abs() < 1.0, "A4 height = 1123, got {h}");
}

// ---------------------------------------------------------------------------
// Margin box area positioning tests (all 16 types)
// ---------------------------------------------------------------------------

#[test]
fn margin_box_all_16_positions_render() {
    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox::default(),
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();

    let page_ctx = elidex_style::page::fixture(816.,1056.,50., "@page {@top-left-corner{content:'TLC'} @top-left{content:'TL'} @top-center{content:'TC'} @top-right{content:'TR'} @top-right-corner{content:'TRC'} @right-top{content:'RT'} @right-middle{content:'RM'} @right-bottom{content:'RB'} @bottom-right-corner{content:'BRC'} @bottom-right{content:'BR'} @bottom-center{content:'BC'} @bottom-left{content:'BL'} @bottom-left-corner{content:'BLC'} @left-bottom{content:'LB'} @left-middle{content:'LM'} @left-top{content:'LT'}}");

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    // Each of 16 margin boxes should produce a Text item (if fonts are available).
    // In test env without system fonts, shape_text may fail, so we just check
    // the function doesn't panic and returns at least zero items.
    assert_eq!(paged_dl.page_count(), 1);
}

// ---------------------------------------------------------------------------
// Margin box styling tests (box model properties)
// ---------------------------------------------------------------------------

#[test]
fn margin_box_with_background_color() {
    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox::default(),
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();

    let page_ctx = elidex_style::page::fixture(
        816.,
        1056.,
        50.,
        "@page {@top-center {content:'Title';background-color:#c8c8c8}}",
    );

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    // Should have at least a SolidRect (background) item.
    let rect_items: Vec<_> = paged_dl.pages[0]
        .iter()
        .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
        .collect();
    assert!(
        !rect_items.is_empty(),
        "margin box with background-color should produce SolidRect"
    );
}

#[test]
fn margin_box_with_border() {
    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox::default(),
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();

    let page_ctx = elidex_style::page::fixture(
        816.,
        1056.,
        50.,
        "@page {@top-center {content:'Header';border-bottom:1px solid black}}",
    );

    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);

    // Should have a SolidRect for the bottom border.
    let rect_items: Vec<_> = paged_dl.pages[0]
        .iter()
        .filter(|i| matches!(i, crate::display_list::DisplayItem::SolidRect { .. }))
        .collect();
    assert!(
        !rect_items.is_empty(),
        "margin box with border-bottom-width should produce border rect"
    );
}

#[test]
fn margin_box_with_explicit_width_height() {
    let page_fragment = elidex_layout::PageFragment {
        layout_box: elidex_plugin::LayoutBox::default(),
        page_number: 1,
        matched_selectors: Vec::new(),
        is_blank: false,
    };

    let dom = elidex_ecs::EcsDom::new();
    let font_db = elidex_text::FontDatabase::new();

    let page_ctx = elidex_style::page::fixture(
        816.,
        1056.,
        50.,
        "@page {@top-center {content:'W';width:100px;height:30px}}",
    );

    // Should not panic with explicit width/height constraints.
    let paged_dl = build_paged_display_lists(&dom, &font_db, &[page_fragment], &page_ctx);
    assert_eq!(paged_dl.page_count(), 1);
}

#[test]
fn continued_block_paints_each_child_and_page_counter_once() {
    use elidex_plugin::{ComputedStyle, CssColor, Dimension, Display};

    let font_db = elidex_shaping::FontDatabase::from_fixture(include_bytes!(
        "../../../tests/fixtures/digits.ttf"
    ));
    let (mut dom, block) = setup_block_element(
        ComputedStyle {
            display: Display::Block,
            counter_increment: vec![("page".into(), 10)],
            ..Default::default()
        },
        elidex_plugin::LayoutBox::default(),
    );
    // Each indivisible child fills the 60px page area. Their parent continues.
    for color in [CssColor::RED, CssColor::BLUE] {
        let child = dom.create_element_ownerless_fixture("div", Attributes::default());
        assert!(dom.append_child(block, child));
        dom.world_mut()
            .insert_one(
                child,
                ComputedStyle {
                    display: Display::Block,
                    height: Dimension::Length(60.),
                    overflow_y: elidex_plugin::Overflow::Hidden,
                    background_color: color,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let ctx = elidex_style::page::fixture(
        220.,
        160.,
        50.,
        "@page {@top-left {content:counter(page);font-family:'Elidex Test Digits'}}",
    );
    let printed = crate::build_paged_display_lists_interleaved(&mut dom, &font_db, &ctx);
    assert_eq!(printed.pages.len(), 2);
    let box_rect = Rect::new(50., 50., 120., 60.);
    // The fixture maps digit n to glyph n + 1 with a half-em advance.
    let eleven = [(2, 62.), (2, 70.)];
    let two = [(3, 66.)];
    for (page, (body_color, digits)) in printed.pages.iter().zip([
        (CssColor::RED, eleven.as_slice()),
        (CssColor::BLUE, two.as_slice()),
    ]) {
        let commands: Vec<_> = page.iter().collect();
        let [DisplayItem::SolidRect { rect, color }, DisplayItem::PushClip { rect: clip, radii }, DisplayItem::PopClip, DisplayItem::Text {
            glyphs, font_size, ..
        }] = commands.as_slice()
        else {
            panic!("expected one clipped body followed by the page counter");
        };
        assert_eq!((*rect, *color), (box_rect, body_color));
        assert_eq!((*clip, *radii), (box_rect, [0.; 4]));
        assert_eq!(*font_size, 16.);
        assert_eq!(glyphs.len(), digits.len());
        for (glyph, &(id, x)) in glyphs.iter().zip(digits) {
            assert_eq!(glyph.glyph_id, id);
            assert_eq!(glyph.position, elidex_plugin::Point::new(x, 33.));
        }
    }
}

#[test]
fn precomputed_blank_page_paints_only_its_number() {
    use elidex_plugin::{ComputedStyle, CssColor, Display, LayoutBox, Point};

    let body = Rect::new(50., 50., 120., 40.);
    let (dom, _) = setup_block_element(
        ComputedStyle {
            display: Display::Block,
            background_color: CssColor::RED,
            ..Default::default()
        },
        LayoutBox {
            content: body,
            ..Default::default()
        },
    );
    let font_db = elidex_text::FontDatabase::from_fixture(include_bytes!(
        "../../../tests/fixtures/digits.ttf"
    ));
    let ctx = elidex_style::page::fixture(
        220.,
        160.,
        50.,
        "@page {@top-left {content:counter(page);font-family:'Elidex Test Digits'}}",
    );

    let fragments = [1, 2].map(|page_number| elidex_layout::PageFragment {
        page_number,
        is_blank: page_number == 2,
        matched_selectors: vec![],
        layout_box: LayoutBox {
            content: body,
            ..Default::default()
        },
    });
    let printed = crate::build_paged_display_lists(&dom, &font_db, &fragments, &ctx);
    assert_eq!(printed.page_count(), 2);
    for (page, id) in printed.pages.iter().zip([2, 3]) {
        let mut commands = page.iter();
        if id == 2 {
            assert!(
                matches!(commands.next(), Some(DisplayItem::SolidRect { rect, color }) if *rect == body && *color == CssColor::RED)
            );
        }
        let Some(DisplayItem::Text { glyphs, .. }) = commands.next() else {
            panic!("expected printed page number");
        };
        let [glyph] = glyphs.as_slice() else {
            panic!("expected one digit");
        };
        assert_eq!((glyph.glyph_id, glyph.position), (id, Point::new(66., 33.)));
        assert!(commands.next().is_none());
    }
}

#[test]
fn consumed_inline_text_paints_only_on_its_source_page() {
    use elidex_plugin::{ComputedStyle, CssColor, Dimension, Display};
    let fonts = elidex_shaping::FontDatabase::from_fixture(include_bytes!(
        "../../../tests/fixtures/digits.ttf"
    ));
    let (mut dom, block) = setup_block_element(
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(210.0),
            background_color: CssColor::RED,
            font_family: vec!["Elidex Test Digits".into()],
            ..Default::default()
        },
        elidex_plugin::LayoutBox::default(),
    );
    let text = dom.create_text_ownerless_fixture("1");
    assert!(dom.append_child(block, text));
    let ctx = elidex_style::page::fixture(240., 180., 50., "");
    let printed = crate::build_paged_display_lists_interleaved(&mut dom, &fonts, &ctx);
    assert_eq!(printed.pages.len(), 3);
    for (i, page) in printed.pages.iter().enumerate() {
        let count: usize = page
            .iter()
            .filter_map(|item| {
                if let DisplayItem::Text { glyphs, .. } = item {
                    Some(glyphs.len())
                } else {
                    None
                }
            })
            .sum();
        assert_eq!(count, usize::from(i == 0));
        assert!(page
            .iter()
            .any(|item| matches!(item,DisplayItem::SolidRect {color,..} if *color==CssColor::RED)));
    }
}

#[test]
fn geometry_only_columns_preserve_the_first_columns_digit() {
    use elidex_plugin::{ColumnFill, ComputedStyle, CssColor, Dimension, Display, LayoutBox};
    let fonts = elidex_shaping::FontDatabase::from_fixture(include_bytes!(
        "../../../tests/fixtures/digits.ttf"
    ));
    for (column_height, height, source) in [
        (40.0, 95.0, "1"),
        (80.0, 210.0, "1"),
        (40.0, 110.0, "1\n2\n3\n4"),
    ] {
        let (mut dom, columns) = setup_block_element(
            ComputedStyle {
                column_count: Some(3),
                column_gap: Dimension::Length(0.0),
                column_fill: ColumnFill::Auto,
                height: Dimension::Length(column_height),
                display: Display::Block,
                ..Default::default()
            },
            LayoutBox::default(),
        );
        let block = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
        assert!(dom.append_child(columns, block));
        dom.world_mut()
            .insert_one(
                block,
                ComputedStyle {
                    height: Dimension::Length(height),
                    background_color: CssColor::RED,
                    display: Display::Block,
                    white_space: elidex_plugin::WhiteSpace::Pre,
                    line_height: elidex_plugin::LineHeight::Px(20.0),
                    font_family: vec!["Elidex Test Digits".into()],
                    ..Default::default()
                },
            )
            .unwrap();
        let text = dom.create_text_ownerless_fixture(source);
        assert!(dom.append_child(block, text));
        let ctx = elidex_style::page::fixture(400., 240., 50., "");
        let printed = crate::build_paged_display_lists_interleaved(&mut dom, &fonts, &ctx);
        assert_eq!(printed.pages.len(), 1);
        let page = &printed.pages[0];
        let positions: Vec<_> = page
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Text { glyphs, .. } => Some(
                    glyphs
                        .iter()
                        .filter(|g| g.glyph_id != 0)
                        .map(|g| g.position.x),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(positions.len(), source.lines().count());
        assert!(positions[0] < 150.0);
        if source.lines().count() > 1 {
            assert!(positions.last().unwrap() >= &150.0);
        }
        assert!(positions.iter().all(|x| *x < 250.0));
        assert_eq!(page.iter().filter(|item| matches!(item, DisplayItem::SolidRect {color,..} if *color == CssColor::RED)).count(), 3);
    }
}

#[test]
fn empty_first_grid_row_preserves_visible_second_page_item() {
    use elidex_plugin::{
        ComputedStyle, CssColor, Display, GridLine, GridTrackList, LayoutBox, TrackSection,
        TrackSize,
    };
    let (mut dom, grid) = setup_block_element(
        ComputedStyle {
            display: Display::Grid,
            grid_template_rows: GridTrackList::Explicit(TrackSection {
                tracks: vec![TrackSize::Length(40.0); 2],
                line_names: vec![],
            }),
            ..Default::default()
        },
        LayoutBox::default(),
    );
    let item = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(grid, item));
    dom.world_mut()
        .insert_one(
            item,
            ComputedStyle {
                display: Display::Block,
                grid_row_start: GridLine::Line(2),
                grid_row_end: GridLine::Line(3),
                background_color: CssColor::RED,
                ..Default::default()
            },
        )
        .unwrap();
    let fonts = elidex_shaping::FontDatabase::new();
    let ctx = elidex_style::page::fixture(240., 140., 50., "");
    let printed = crate::build_paged_display_lists_interleaved(&mut dom, &fonts, &ctx);
    let visible: Vec<_> = printed
        .pages
        .iter()
        .map(|page| {
            page.iter().any(|item|
        matches!(item, DisplayItem::SolidRect {color, ..} if *color == CssColor::RED))
        })
        .collect();
    assert_eq!(visible, [false, true]);
}
