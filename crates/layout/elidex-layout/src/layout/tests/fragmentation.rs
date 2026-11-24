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
        medium: elidex_plugin::Medium::Screen,
        is_probe: false,
    };
    let fragments = layout_fragmented(&mut dom, div, &input, frag);
    assert_eq!(fragments.len(), 1, "content fits → 1 fragment");
    assert!(fragments[0].end.is_complete());
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
        medium: elidex_plugin::Medium::Screen,
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
        fragments.last().unwrap().end.is_complete(),
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
        medium: elidex_plugin::Medium::Screen,
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
        medium: elidex_plugin::Medium::Screen,
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

    for with_cells in [false, true] {
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
        if with_cells {
            node("td", Display::TableCell, Some(p), BreakValue::Auto);
        }
        let q = node("tr", Display::TableRow, Some(table), BreakValue::Page);
        if with_cells {
            node("td", Display::TableCell, Some(q), BreakValue::Auto);
        }
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
        let token = first.end.into_token().expect("Table interrupted after P");
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
            assert!(next.end.is_complete());
        }
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
        medium: elidex_plugin::Medium::Screen,
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
        let token = page.end.into_token().unwrap();
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
        assert!(next.end.is_complete());
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
        assert!(outcome.end.is_complete());
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
        assert!(outcome.end.is_complete());
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
    let flex_span = node("div", Display::Flex, Some(multi));
    let span_item = node("div", Display::Block, Some(flex_span));
    let flex_float = node("div", Display::Flex, Some(owner));
    let float_item = node("div", Display::Block, Some(flex_float));
    dom.world_mut()
        .get::<&mut ComputedStyle>(flex_float)
        .unwrap()
        .float = Float::Left;
    dom.world_mut()
        .get::<&mut ComputedStyle>(multi)
        .unwrap()
        .column_count = Some(2);
    for spanner in [table, block, flex_span] {
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
        (flex_span, span_item),
        (flex_float, float_item),
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

#[test]
fn nested_screen_multicol_keeps_overflow_columns_at_the_inner_pitch() {
    let mut dom = EcsDom::new();
    let mut multi = |parent| {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display: Display::Block,
                column_count: Some(2),
                column_gap: Dimension::Length(0.0),
                column_fill: elidex_plugin::ColumnFill::Auto,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let outer = multi(None);
    let inner = multi(Some(outer));
    let mut last = None;
    for i in 0..1001 {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.append_child(inner, id);
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(1.0),
                break_after: if i < 1000 {
                    elidex_plugin::BreakValue::Column
                } else {
                    elidex_plugin::BreakValue::Auto
                },
                ..Default::default()
            },
        );
        last = Some(id);
    }
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 40.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Column,
    };
    let input = elidex_layout_block::LayoutInput {
        medium: elidex_plugin::Medium::Screen,
        containing: CssSize::definite(600.0, 800.0),
        containing_inline_size: 600.0,
        fragmentainer: None,
        ..contribution_input(&fonts, &frag)
    };
    let outcome = dispatch_layout_child(&mut dom, outer, &input);
    let last = last.unwrap();
    assert!(outcome.contribution.is_finished(inner, last));
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::MulticolInfo>(inner)
            .unwrap()
            .segments[0]
            .0,
        1001
    );
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::LayoutBox>(last)
            .unwrap()
            .content
            .origin
            .x,
        150_000.0
    );
}

#[test]
fn stalled_multicol_remains_incomplete_through_whole_children_and_page_counting() {
    use elidex_layout_block::{FragmentEnd, FragmentainerContext, FragmentationType};
    use elidex_plugin::{BreakValue, ColumnFill, ColumnSpan, Float, Medium};

    for (float, span) in [
        (Float::None, ColumnSpan::None),
        (Float::Left, ColumnSpan::None),
        (Float::None, ColumnSpan::All),
    ] {
        let mut dom = EcsDom::new();
        let mut node = |parent, style| {
            let id = dom.create_element_ownerless_fixture("div", Attributes::default());
            dom.world_mut().insert_one(id, style);
            if let Some(owner) = parent {
                dom.append_child(owner, id);
            }
            id
        };
        let block = ComputedStyle {
            display: Display::Block,
            ..Default::default()
        };
        let root = node(None, block.clone());
        let wrapper = node(
            Some(root),
            ComputedStyle {
                column_count: (span == ColumnSpan::All).then_some(2),
                ..block.clone()
            },
        );
        let inner = node(
            Some(wrapper),
            ComputedStyle {
                column_count: Some(2),
                height: Dimension::Length(40.0),
                column_fill: ColumnFill::Auto,
                float,
                column_span: span,
                ..block.clone()
            },
        );
        let pending = node(
            Some(inner),
            ComputedStyle {
                break_before: BreakValue::Column,
                height: Dimension::Length(1.0),
                ..block
            },
        );
        let fonts = FontDatabase::new();
        let page = FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: FragmentationType::Page,
        };
        let input = elidex_layout_block::LayoutInput {
            medium: Medium::Print,
            ..contribution_input(&fonts, &page)
        };
        for owner in [inner, wrapper, root] {
            let fragments = layout_fragmented(&mut dom, owner, &input, page);
            assert_eq!(fragments.len(), 1);
            let outcome = &fragments[0];
            assert!(matches!(outcome.end, FragmentEnd::Incomplete));
            for (parent, child) in [(inner, pending), (wrapper, inner), (root, wrapper)] {
                assert!(!outcome.contribution.is_finished(parent, child));
            }
        }
    }
}

#[test]
fn positive_room_for_an_unstarted_size_only_child_respects_class_a_avoid() {
    use elidex_layout_block::{
        BlockContinuation, BreakTokenData, FragmentainerContext, FragmentationType,
    };
    use elidex_plugin::BreakValue;
    let fonts = FontDatabase::new();
    let frag = FragmentainerContext {
        available_block_size: 15.0,
        fragmentation_type: FragmentationType::Page,
    };
    for avoid in [false, true] {
        let mut dom = EcsDom::new();
        let owner = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(
            owner,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let items = [10.0, 30.0].map(|height| {
            let child = dom.create_element_ownerless_fixture("div", Attributes::default());
            dom.append_child(owner, child);
            dom.world_mut().insert_one(
                child,
                ComputedStyle {
                    display: Display::Block,
                    height: Dimension::Length(height),
                    ..Default::default()
                },
            );
            child
        });
        if avoid {
            dom.world_mut()
                .get::<&mut ComputedStyle>(items[1])
                .unwrap()
                .break_before = BreakValue::Avoid;
        }
        let input = contribution_input(&fonts, &frag);
        let outcome = dispatch_layout_child(&mut dom, owner, &input);
        let token = outcome.end.into_token().unwrap();
        let Some(BreakTokenData::Block {
            child_index: 1,
            continuation,
            ..
        }) = token.mode_data
        else {
            panic!("expected second child route")
        };
        assert!(outcome.contribution.is_finished(owner, items[0]));
        if avoid {
            let BlockContinuation::InterruptedChild(child) = continuation else {
                panic!("only an avoided class A boundary")
            };
            assert_eq!(child.entity, items[1]);
            assert_eq!(child.specified_size_fraction, Some(5.0 / 30.0));
            assert_eq!(
                dom.world()
                    .get::<&elidex_plugin::LayoutBox>(items[1])
                    .unwrap()
                    .content
                    .size
                    .height,
                5.0
            );
        } else {
            assert!(matches!(continuation, BlockContinuation::ChildBoundary));
            assert!(!dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(items[1])
                .is_ok_and(|lb| lb.layout_generation == input.layout_generation));
        }
    }
}

fn make_parallel_grid() -> (EcsDom, Entity, Vec<(Entity, Entity, Entity)>, Entity) {
    use elidex_plugin::{BreakValue, GridTrackList, TrackSection, TrackSize};
    let mut dom = EcsDom::new();
    let mut node = |style, parent| {
        let entity = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(entity, style);
        if let Some(parent) = parent {
            dom.append_child(parent, entity);
        }
        entity
    };
    let grid = node(
        ComputedStyle {
            display: Display::Grid,
            grid_template_columns: GridTrackList::Explicit(TrackSection {
                tracks: vec![TrackSize::Fr(1.0); 3],
                line_names: Vec::new(),
            }),
            grid_template_rows: GridTrackList::Explicit(TrackSection {
                tracks: vec![TrackSize::Length(60.0)],
                line_names: Vec::new(),
            }),
            ..Default::default()
        },
        None,
    );
    let block = |height| ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(height),
        ..Default::default()
    };
    let mut interrupted = Vec::new();
    for height in [140.0, 210.0] {
        let child = node(block(height), Some(grid));
        let prefix = node(block(12.0), Some(child));
        let suffix = node(
            ComputedStyle {
                break_before: BreakValue::Page,
                ..block(14.0)
            },
            Some(child),
        );
        interrupted.push((child, prefix, suffix));
    }
    let whole = node(block(10.0), Some(grid));
    (dom, grid, interrupted, whole)
}

#[test]
fn grid_replays_two_parallel_child_routes_by_identity() {
    use elidex_layout_block::{BreakTokenData, FragmentainerContext, FragmentationType};
    let (mut dom, grid, interrupted, whole) = make_parallel_grid();
    let fonts = FontDatabase::new();
    let mut saved = vec![None];
    for replay in [false, true] {
        for (page, room) in [95.0, 60.0, 30.0, 30.0].into_iter().enumerate() {
            let frag = FragmentainerContext {
                available_block_size: room,
                fragmentation_type: FragmentationType::Page,
            };
            let generation = 1 + u32::from(replay) * 4 + u32::try_from(page).unwrap();
            let input = elidex_layout_block::LayoutInput {
                break_token: saved[page].as_ref(),
                layout_generation: generation,
                medium: elidex_plugin::Medium::Print,
                ..contribution_input(&fonts, &frag)
            };
            let outcome = dispatch_layout_child(&mut dom, grid, &input);
            let visible = |id| {
                dom.world()
                    .get::<&elidex_plugin::LayoutBox>(id)
                    .is_ok_and(|lb| lb.layout_generation == generation)
            };
            assert_eq!(visible(whole), page == 0);
            assert_eq!(outcome.contribution.is_finished(grid, whole), page == 0);
            for (i, &(child, prefix, suffix)) in interrupted.iter().enumerate() {
                let last = if i == 0 { 1 } else { 3 };
                assert_eq!(visible(child), page <= last);
                if page <= last {
                    let expected = [[95.0, 45.0, 0.0, 0.0], [95.0, 60.0, 30.0, 25.0]][i][page];
                    assert_eq!(
                        dom.world()
                            .get::<&elidex_plugin::LayoutBox>(child)
                            .unwrap()
                            .content
                            .size
                            .height,
                        expected
                    );
                }
                assert_eq!(visible(prefix), page == 0);
                assert_eq!(visible(suffix), page == 1);
                assert_eq!(outcome.contribution.is_finished(child, prefix), page == 0);
                assert_eq!(outcome.contribution.is_finished(child, suffix), page == 1);
                assert_eq!(outcome.contribution.is_finished(grid, child), page == last);
            }
            if page == 3 {
                assert!(outcome.end.is_complete());
            } else {
                let token = outcome.end.into_token().unwrap();
                let Some(BreakTokenData::Grid {
                    child_break_tokens, ..
                }) = &token.mode_data
                else {
                    panic!("expected Grid continuation")
                };
                let unresolved = if page == 0 {
                    &interrupted[..]
                } else {
                    &interrupted[1..]
                };
                assert_eq!(child_break_tokens.len(), unresolved.len());
                for ((child, token), (expected, _, _)) in child_break_tokens.iter().zip(unresolved)
                {
                    assert_eq!(child, expected);
                    assert_eq!(token.entity, *expected);
                }
                if !replay {
                    saved.push(Some(token));
                }
            }
        }
    }
}

#[test]
fn multicol_finishes_geometry_only_fragments_of_an_empty_grid() {
    let mut dom = EcsDom::new();
    let multi = dom.create_element_ownerless_fixture("div", Attributes::default());
    let grid = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.append_child(multi, grid);
    dom.world_mut().insert_one(
        multi,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            column_count: Some(2),
            column_fill: elidex_plugin::ColumnFill::Auto,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            height: Dimension::Length(210.0),
            ..Default::default()
        },
    );
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let outcome = dispatch_layout_child(&mut dom, multi, &contribution_input(&fonts, &frag));
    assert!(outcome.end.is_complete());
    assert!(outcome.contribution.is_finished(multi, grid));
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::MulticolInfo>(multi)
            .unwrap()
            .segments[0]
            .0,
        6
    );
}
