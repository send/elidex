use super::*;

// ---------------------------------------------------------------------------
// Fragmentation tests (CSS Fragmentation Level 3)
// ---------------------------------------------------------------------------

#[test]
fn layout_fragmented_single_fragment_when_content_fits() {
    let mut dom = EcsDom::new();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        available_height_if_indefinite: None,
        containing_width_definite: true,
        is_flex_item: false,
        assigned: elidex_layout_block::UsedAssignment::default(),
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
fn layout_fragmented_two_fragments_on_overflow() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    // Add 3 children, each 80px tall. Total = 240px.
    for _ in 0..3 {
        let child = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        available_height_if_indefinite: None,
        containing_width_definite: true,
        is_flex_item: false,
        assigned: elidex_layout_block::UsedAssignment::default(),
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child1 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
    let child2 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        available_height_if_indefinite: None,
        containing_width_definite: true,
        is_flex_item: false,
        assigned: elidex_layout_block::UsedAssignment::default(),
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
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        available_height_if_indefinite: None,
        containing_width_definite: true,
        is_flex_item: false,
        assigned: elidex_layout_block::UsedAssignment::default(),
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

fn make_page_ctx(width: f32, height: f32) -> elidex_plugin::PreparedPages {
    elidex_style::page::fixture(width, height, 50.0, "")
}

#[test]
fn paged_single_page_fits_all_content() {
    let (mut dom, _root, _html, body) = build_styled_dom();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        let child = dom.create_element_ownerless_fixture("div", Attributes::default());
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
    use elidex_plugin::PageSelector as PS;

    let mut dom = EcsDom::new();
    let div = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    let page_ctx = elidex_style::page::fixture(816., 1056., 0., "@page :first {}");
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child = dom.create_element_ownerless_fixture("div", Attributes::default());
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
    let child2 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
    let page_ctx = elidex_style::page::fixture(816., 1056., 0., "@page {size:A4}");

    let elidex_plugin::Size {
        width: w,
        height: h,
    } = page_ctx.select(1, false).geometry.sheet;
    assert!(approx_eq(w, 794.0), "A4 width = 794, got {w}");
    assert!(approx_eq(h, 1123.0), "A4 height = 1123, got {h}");
}

#[test]
fn paged_counter_page_increments() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..2 {
        let child = dom.create_element_ownerless_fixture("div", Attributes::default());
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
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        let child = dom.create_element_ownerless_fixture("div", Attributes::default());
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

#[test]
fn new_paged_run_starts_at_first_child_after_style_change() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
    let style = |height| ComputedStyle {
        display: Display::Block,
        height,
        ..Default::default()
    };
    dom.world_mut()
        .insert_one(parent, style(Dimension::Auto))
        .unwrap();
    let mut children = Vec::new();
    for _ in 0..3 {
        let child = dom.create_element_ownerless_fixture("div", Attributes::default());
        assert!(dom.append_child(parent, child));
        dom.world_mut()
            .insert_one(child, style(Dimension::Length(70.)))
            .unwrap();
        children.push(child);
    }
    let fonts = FontDatabase::new();
    let pages = make_page_ctx(400., 200.);
    assert_eq!(layout_paged(&mut dom, &pages, &fonts).len(), 3);
    for child in children {
        dom.world_mut()
            .insert_one(child, style(Dimension::Length(20.)))
            .unwrap();
    }
    let rerun = layout_paged(&mut dom, &pages, &fonts);
    assert_eq!(rerun.len(), 1);
    assert_eq!(rerun[0].page_number, 1);
    assert_eq!(rerun[0].layout_box.content.size.height, 60.);
}

#[test]
fn block_preserves_table_row_contributions_at_a_forced_break() {
    use elidex_layout_block::{
        BlockContinuation, BreakTokenData, FragmentainerContext, FragmentationType, LayoutInput,
    };
    use elidex_plugin::BreakValue;

    let mut dom = EcsDom::new();
    let mut node = |tag, display, parent, break_before| {
        let id = dom.create_element_ownerless_fixture(tag, Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display,
                break_before,
                height: if display == Display::TableCell {
                    Dimension::Length(10.0)
                } else {
                    Dimension::Auto
                },
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let block = node("div", Display::Block, None, BreakValue::Auto);
    let table = node("table", Display::Table, Some(block), BreakValue::Auto);
    let p = node("tr", Display::TableRow, Some(table), BreakValue::Auto);
    node("td", Display::TableCell, Some(p), BreakValue::Auto);
    let q = node("tr", Display::TableRow, Some(table), BreakValue::Page);
    node("td", Display::TableCell, Some(q), BreakValue::Auto);
    let font_db = FontDatabase::new();
    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = contribution_input(&font_db, &frag);
    let first = dispatch_layout_child(&mut dom, block, &input);
    assert!(first.contribution.is_finished(table, p));
    assert!(!first.contribution.is_finished(table, q));
    assert!(!first.contribution.is_finished(block, table));
    let token = first.break_token.expect("Table interrupted after P");
    let Some(BreakTokenData::Block {
        continuation: BlockContinuation::InterruptedChild(child),
        ..
    }) = &token.mode_data
    else {
        panic!("expected interrupted Table");
    };
    assert_eq!(child.entity, table);
    assert!(matches!(
        child.mode_data,
        Some(BreakTokenData::Table { row_index: 1, .. })
    ));
    let mut corrupt = token.clone();
    if let Some(BreakTokenData::Block {
        continuation: BlockContinuation::InterruptedChild(child),
        ..
    }) = &mut corrupt.mode_data
    {
        child.entity = q;
    }
    assert_rejected_before_publication(&mut dom, block, &input, &corrupt);
    let mut wrong_route = token.clone();
    if let Some(BreakTokenData::Block { continuation, .. }) = &mut wrong_route.mode_data {
        *continuation = BlockContinuation::ChildBoundary;
    }
    assert_rejected_before_publication(&mut dom, block, &input, &wrong_route);
    for width in [120.0, 260.0] {
        let next = dispatch_layout_child(
            &mut dom,
            block,
            &LayoutInput {
                containing: CssSize::definite(width, 100.0),
                containing_inline_size: width,
                break_token: Some(&token),
                layout_generation: 2,
                ..input
            },
        );
        assert!(!next.contribution.is_finished(table, p));
        assert!(next.contribution.is_finished(table, q));
        assert!(next.contribution.is_finished(block, table));
        assert!(next.break_token.is_none());
    }
}

fn contribution_input<'a>(
    font_db: &'a FontDatabase,
    frag: &'a elidex_layout_block::FragmentainerContext,
) -> elidex_layout_block::LayoutInput<'a> {
    elidex_layout_block::LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        is_flex_item: false,
        assigned: elidex_layout_block::UsedAssignment::default(),
        containing: CssSize::definite(200.0, 100.0),
        containing_inline_size: 200.0,
        offset: Point::ZERO,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: Some(frag),
        break_token: None,
        subgrid: None,
        layout_generation: 1,
        is_probe: false,
    }
}

#[test]
fn block_retreat_commits_only_the_winning_boundary() {
    use elidex_layout_block::{
        BlockContinuation, BreakTokenData, FragmentainerContext, FragmentationType, LayoutInput,
    };
    use elidex_plugin::BreakValue;
    let font_db = FontDatabase::new();
    let frag = FragmentainerContext {
        available_block_size: 20.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = contribution_input(&font_db, &frag);
    for (edge, expected_count, extent) in
        [(BreakValue::Avoid, 1, 10.0), (BreakValue::Auto, 2, 20.0)]
    {
        let mut dom = EcsDom::new();
        let block = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut()
            .insert_one(
                block,
                ComputedStyle {
                    display: Display::Block,
                    ..Default::default()
                },
            )
            .unwrap();
        let items = [BreakValue::Auto, edge, BreakValue::Auto].map(|break_after| {
            let child = dom.create_element_ownerless_fixture("div", Attributes::default());
            dom.append_child(block, child);
            dom.world_mut()
                .insert_one(
                    child,
                    ComputedStyle {
                        display: Display::Block,
                        height: Dimension::Length(10.0),
                        break_after,
                        ..Default::default()
                    },
                )
                .unwrap();
            child
        });
        let page = dispatch_layout_child(&mut dom, block, &input);
        let token = page.break_token.unwrap();
        let Some(BreakTokenData::Block {
            child_index,
            coverage,
            continuation: BlockContinuation::ChildBoundary,
        }) = &token.mode_data
        else {
            panic!("expected clean child boundary");
        };
        assert_eq!(
            (*child_index, token.consumed_block_size),
            (expected_count, extent)
        );
        for (i, child) in items.iter().enumerate() {
            assert_eq!(
                page.contribution.is_finished(block, *child),
                i < expected_count
            );
            let published = dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(*child)
                .is_ok_and(|lb| lb.layout_generation == input.layout_generation);
            assert_eq!(published, i < expected_count);
            assert_eq!(coverage.is_finished(block, *child), i < expected_count);
        }
        let next = dispatch_layout_child(
            &mut dom,
            block,
            &LayoutInput {
                break_token: Some(&token),
                layout_generation: 2,
                ..input
            },
        );
        assert!(next.break_token.is_none());
        for (i, child) in items.iter().enumerate() {
            assert_eq!(
                next.contribution.is_finished(block, *child),
                i >= expected_count
            );
        }
        let mut corrupt = token;
        if let Some(BreakTokenData::Block { child_index, .. }) = &mut corrupt.mode_data {
            *child_index = 0;
        }
        assert_rejected_before_publication(&mut dom, block, &input, &corrupt);
    }
}

fn assert_rejected_before_publication(
    dom: &mut EcsDom,
    block: Entity,
    input: &elidex_layout_block::LayoutInput<'_>,
    corrupt: &elidex_layout_block::BreakToken,
) {
    let before = dom
        .world()
        .get::<&elidex_plugin::LayoutBox>(block)
        .unwrap()
        .layout_generation;
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch_layout_child(
            dom,
            block,
            &elidex_layout_block::LayoutInput {
                break_token: Some(corrupt),
                layout_generation: before + 1,
                ..*input
            },
        )
    }));
    assert!(rejected.is_err());
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::LayoutBox>(block)
            .unwrap()
            .layout_generation,
        before
    );
}

#[test]
fn block_reports_only_rows_admitted_by_table_grid() {
    use elidex_layout_block::{FragmentainerContext, FragmentationType};
    for with_cells in [false, true] {
        let mut dom = EcsDom::new();
        let mut node = |tag, display, parent| {
            let id = dom.create_element_ownerless_fixture(tag, Attributes::default());
            dom.world_mut().insert_one(
                id,
                ComputedStyle {
                    display,
                    ..Default::default()
                },
            );
            if let Some(owner) = parent {
                dom.append_child(owner, id);
            }
            id
        };
        let block = node("div", Display::Block, None);
        let table = node("table", Display::Table, Some(block));
        let count = if with_cells { 4001 } else { 1 };
        let mut rows = Vec::new();
        for _ in 0..count {
            let row = node("tr", Display::TableRow, Some(table));
            if with_cells {
                node("td", Display::TableCell, Some(row));
            }
            rows.push(row);
        }
        let fonts = FontDatabase::new();
        let frag = FragmentainerContext {
            available_block_size: 1_000_000.0,
            fragmentation_type: FragmentationType::Page,
        };
        let outcome = dispatch_layout_child(&mut dom, block, &contribution_input(&fonts, &frag));
        assert!(outcome.break_token.is_none());
        assert!(outcome.contribution.is_finished(block, table));
        for (index, row) in rows.into_iter().enumerate() {
            assert_eq!(
                outcome.contribution.is_finished(table, row),
                index < 4000,
                "row {index}"
            );
        }
    }
}

#[test]
fn balanced_multicol_reports_empty_block_to_its_parent() {
    let mut dom = EcsDom::new();
    let mut node = |parent, columns| {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display: Display::Block,
                column_count: columns,
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let owner = node(None, None);
    let multi = node(Some(owner), Some(2));
    let empty = node(Some(multi), None);
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = contribution_input(&fonts, &frag);
    for entry in [multi, owner] {
        let outcome = dispatch_layout_child(&mut dom, entry, &input);
        assert!(outcome.break_token.is_none());
        assert!(outcome.contribution.is_finished(multi, empty));
        if entry == owner {
            assert!(outcome.contribution.is_finished(owner, multi));
        }
    }
}

#[test]
fn whole_spanners_and_floats_forward_nested_source() {
    use elidex_plugin::{ColumnSpan, Float};
    let mut dom = EcsDom::new();
    let mut node = |tag, display, parent| {
        let id = dom.create_element_ownerless_fixture(tag, Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display,
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let owner = node("div", Display::Block, None);
    let multi = node("div", Display::Block, Some(owner));
    let table = node("table", Display::Table, Some(multi));
    let row = node("tr", Display::TableRow, Some(table));
    node("td", Display::TableCell, Some(row));
    let block = node("div", Display::Block, Some(multi));
    let inner = node("div", Display::Block, Some(block));
    let float = node("div", Display::Block, Some(owner));
    let floated = node("div", Display::Block, Some(float));
    dom.world_mut()
        .get::<&mut ComputedStyle>(multi)
        .unwrap()
        .column_count = Some(2);
    for spanner in [table, block] {
        dom.world_mut()
            .get::<&mut ComputedStyle>(spanner)
            .unwrap()
            .column_span = ColumnSpan::All;
    }
    dom.world_mut()
        .get::<&mut ComputedStyle>(float)
        .unwrap()
        .float = Float::Left;
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let outcome = dispatch_layout_child(&mut dom, owner, &contribution_input(&fonts, &frag));
    for (parent, child) in [
        (owner, multi),
        (multi, table),
        (table, row),
        (multi, block),
        (block, inner),
        (owner, float),
        (float, floated),
    ] {
        assert!(
            outcome.contribution.is_finished(parent, child),
            "missing {parent:?}/{child:?}"
        );
    }
}
