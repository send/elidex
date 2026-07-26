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
        is_probe: false,
    };
    let fragments = layout_fragmented(&mut dom, div, &input, frag);
    assert_eq!(fragments.len(), 1, "content fits → 1 fragment");
    assert!(fragments[0].break_token.is_none());
}

#[test]
fn layout_fragmented_invalidates_screen_geometry_provenance() {
    // Provenance (terminal-Z C-3a §2): a paged pass that lays anything out must leave a
    // prior screen pass's `CompletedScreen` demoted — else `screen_geometry()` would read
    // a page-relative store as screen geometry (soundness hole 1). It holds without any
    // entry mark: the fragmentainer loop reaches `dispatch_layout_child`, whose bracket
    // invalidates. (A paged attempt that lays out NOTHING is the opposite case and must
    // NOT demote — `a_zero_write_paged_early_return_leaves_the_phase_alone` below.)
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
    // A prior screen pass completed.
    dom.fragment_tree_mut().publish_completed_screen();
    assert!(
        dom.screen_geometry().is_some(),
        "precondition: completed screen"
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
        is_probe: false,
    };
    let _ = layout_fragmented(&mut dom, div, &input, frag);

    assert!(
        dom.screen_geometry().is_none(),
        "the paged pass's dispatch bracket invalidated the stale CompletedScreen"
    );
}

#[test]
fn layout_paged_invalidates_screen_geometry_provenance() {
    // The OTHER layout-crate paged public fn: `layout_paged` reaches
    // `layout_fragmented_with_tokens` via `layout_fragmented`, so it too demotes a prior
    // screen pass once it lays anything out. Guards the path a future
    // non-interleaved-driver caller of `layout_paged` would take.
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
    dom.fragment_tree_mut().publish_completed_screen();
    assert!(
        dom.screen_geometry().is_some(),
        "precondition: completed screen"
    );

    let font_db = FontDatabase::new();
    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 400.0,
        page_height: 600.0,
        page_margins: elidex_plugin::EdgeSizes::uniform(10.0),
        page_rules: Vec::new(),
    };
    let _ = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(
        dom.screen_geometry().is_none(),
        "layout_paged laid content out, so the dispatch bracket cleared CompletedScreen"
    );
}

#[test]
fn a_zero_write_paged_early_return_leaves_the_phase_alone() {
    // Codex PR#488 R2 asked for an `invalidate()` *before* the paged fns'
    // validation returns, so that entering paged mode always clears
    // `CompletedScreen`. Deliberately NOT done — the invariant is about the store's
    // CONTENTS, not about which mode is executing. `CompletedScreen` means "the store
    // reflects a completed screen pass", and a paged attempt that bails on
    // `roots.is_empty()` or a non-positive content area writes NOTHING (`find_roots` /
    // `find_roots_mut` both take `&EcsDom`), so the store still holds exactly that and
    // `Some` is the truthful answer.
    //
    // Invalidating there would be a spurious demotion — the same defect this branch
    // removed from `remove_entity` one round earlier (a mutator that writes nothing must
    // not demote a published store, pinned by
    // `every_content_mutator_invalidates_a_published_store`). Codex's patch would have
    // reintroduced it on the paged side and forced a full relayout before any
    // screen-geometry read whenever a print attempt no-ops.
    //
    // The plan's §2 wording ("BOTH paged entries leave phase `Invalid`") was the real
    // defect here — it stated the guarantee at MODE granularity ("a paged fn was
    // called"), whereas what actually demotes is the pass reaching a store writer or a
    // dispatch bracket; an attempt that returns before both reaches neither. §2 is
    // corrected there. The correction is NOT "the mechanism is uniformly write-granular"
    // — the component half demotes unconditionally at `dispatch_layout_child`'s bracket,
    // write or no write. It is that ENTERING a mode is not itself a demotion event.
    // This test pins the decision so a future reader of that section cannot "fix" the
    // code back.
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
    dom.fragment_tree_mut().publish_completed_screen();

    let font_db = FontDatabase::new();
    // Margins exceed the page box ⇒ content_width/height <= 0 ⇒ the early return.
    let degenerate = elidex_plugin::PagedMediaContext {
        page_width: 10.0,
        page_height: 10.0,
        page_margins: elidex_plugin::EdgeSizes::uniform(20.0),
        page_rules: Vec::new(),
    };
    let pages = layout_paged(&mut dom, &degenerate, &font_db);

    assert!(pages.is_empty(), "degenerate page box lays nothing out");
    assert!(
        dom.screen_geometry().is_some(),
        "a zero-write paged early return must not demote the published screen pass"
    );

    // The SECOND zero-write arm of the same fn — `roots.is_empty()`. Pinning only the
    // non-positive-area arm would leave the decision half-covered, and a "fix" that
    // added the invalidate to this arm alone would pass.
    let mut empty_dom = EcsDom::new();
    empty_dom.fragment_tree_mut().publish_completed_screen();
    let sane = elidex_plugin::PagedMediaContext {
        page_width: 400.0,
        page_height: 600.0,
        page_margins: elidex_plugin::EdgeSizes::uniform(10.0),
        page_rules: Vec::new(),
    };
    let pages = layout_paged(&mut empty_dom, &sane, &font_db);

    assert!(pages.is_empty(), "no layout roots ⇒ nothing laid out");
    assert!(
        empty_dom.screen_geometry().is_some(),
        "the roots.is_empty() early return is a zero-write path too — no demotion"
    );
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
        is_probe: false,
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
        is_probe: false,
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
        is_probe: false,
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
