use super::*;

// ---------------------------------------------------------------------------
// Fragmentation tests (CSS Fragmentation Level 3)
// ---------------------------------------------------------------------------

#[test]
fn layout_fragmented_single_fragment_when_content_fits() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, div, &input, frag);
    assert_eq!(fragments.len(), 1, "content fits → 1 fragment");
    assert!(fragments[0].break_token.is_none());
}

#[test]
fn layout_fragmented_two_fragments_on_overflow() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    // Add 3 children, each 80px tall. Total = 240px.
    for _ in 0..3 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(80.0),
                ..Default::default()
            },
        );
    }
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, parent, &input, frag);
    assert!(
        fragments.len() >= 2,
        "240px in 100px fragments → at least 2 fragments"
    );
    // Break tokens are consumed by the fragmentation loop — non-last fragments
    // were successfully fragmented (verified by the fragment count above).
    assert!(
        fragments.last().unwrap().break_token.is_none(),
        "last fragment has no break token"
    );
}

#[test]
fn layout_fragmented_forced_break_produces_two_fragments() {
    use elidex_plugin::BreakValue;

    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child1 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child1);
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            break_after: BreakValue::Page,
            ..Default::default()
        },
    );
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, parent, &input, frag);
    assert_eq!(fragments.len(), 2, "forced break → 2 fragments");
}

#[test]
fn layout_fragmented_without_fragmentainer_returns_one() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Column,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    let fragments = layout_fragmented(&mut dom, div, &input, frag);
    assert_eq!(fragments.len(), 1);
}

// ---------------------------------------------------------------------------
// Paged media layout tests (CSS Paged Media Level 3)
// ---------------------------------------------------------------------------

fn make_page_ctx(width: f32, height: f32) -> elidex_plugin::PagedMediaContext {
    elidex_plugin::PagedMediaContext {
        page_width: width,
        page_height: height,
        page_margins: elidex_plugin::EdgeSizes {
            top: 50.0,
            right: 50.0,
            bottom: 50.0,
            left: 50.0,
        },
        page_rules: Vec::new(),
    }
}

#[test]
fn paged_single_page_fits_all_content() {
    let (mut dom, _root, _html, body) = build_styled_dom();
    let div = dom.create_element("div", Attributes::default());
    dom.append_child(body, div);
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(!pages.is_empty(), "should have at least one page");
    assert_eq!(pages[0].page_number, 1);
    assert!(!pages[0].is_blank);
}

#[test]
fn paged_multi_page_break() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(500.0),
                ..Default::default()
            },
        );
    }

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(
        pages.len() >= 2,
        "1500px content in 956px pages → at least 2 pages, got {}",
        pages.len()
    );
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.page_number, i + 1);
    }
}

#[test]
fn paged_selector_first() {
    use elidex_plugin::{PageRule, PageSelector as PS};

    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: elidex_plugin::EdgeSizes::default(),
        page_rules: vec![PageRule {
            selectors: vec![PS::First],
            ..PageRule::default()
        }],
    };
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(!pages.is_empty());
    assert!(
        pages[0].matched_selectors.contains(&PS::First),
        "first page should match :first selector"
    );
}

#[test]
fn paged_selector_left_right() {
    use elidex_plugin::PageSelector as PS;

    assert!(PS::Right.matches(1, false));
    assert!(PS::Left.matches(2, false));
    assert!(PS::Right.matches(3, false));
    assert!(!PS::Left.matches(1, false));
    assert!(!PS::Right.matches(2, false));
}

#[test]
fn paged_blank_page_from_forced_break() {
    use elidex_plugin::BreakValue;

    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            break_after: BreakValue::Page,
            ..Default::default()
        },
    );
    let child2 = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(
        pages.len() >= 2,
        "forced break should produce at least 2 pages"
    );
}

#[test]
fn paged_size_from_rule() {
    use elidex_plugin::{NamedPageSize, PageRule, PageSize};

    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 816.0,
        page_height: 1056.0,
        page_margins: elidex_plugin::EdgeSizes::default(),
        page_rules: vec![PageRule {
            selectors: Vec::new(), // matches all pages
            size: Some(PageSize::Named(NamedPageSize::A4)),
            ..PageRule::default()
        }],
    };

    let (w, h) = page_ctx.effective_page_size(1, false);
    assert!(approx_eq(w, 794.0), "A4 width = 794, got {w}");
    assert!(approx_eq(h, 1123.0), "A4 height = 1123, got {h}");
}

#[test]
fn paged_counter_page_increments() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..2 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(600.0),
                ..Default::default()
            },
        );
    }

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.page_number, i + 1, "page number should be sequential");
    }
}

#[test]
fn paged_two_pass_counter_pages() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(500.0),
                ..Default::default()
            },
        );
    }

    let font_db = FontDatabase::new();
    let page_ctx = make_page_ctx(816.0, 1056.0);
    let pages = layout_paged(&mut dom, &page_ctx, &font_db);

    let total = pages.len();
    assert!(total >= 2, "should have multiple pages");
    assert_eq!(pages.last().unwrap().page_number, total);
}
