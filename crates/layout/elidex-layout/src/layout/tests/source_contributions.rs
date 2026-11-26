use super::*;

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

pub(super) fn contribution_input<'a>(
    font_db: &'a FontDatabase,
    frag: &'a elidex_layout_block::FragmentainerContext,
) -> elidex_layout_block::LayoutInput<'a> {
    elidex_layout_block::LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        item_context: elidex_layout_block::ItemContext::NormalFlow,
        assigned: elidex_layout_block::UsedAssignment::default(),
        containing: CssSize::definite(200.0, 100.0),
        containing_inline_size: 200.0,
        offset: Point::ZERO,
        font_db,
        depth: 0,
        float_ctx: None,
        scroll_limits: elidex_layout_block::ScrollLimits::default(),
        viewport: None,
        fragmentainer: Some(frag),
        break_token: None,
        subgrid: None,
        layout_generation: 1,
        medium: elidex_plugin::Medium::Screen,
        placement: elidex_layout_block::FragmentPlacement::Normal,
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
