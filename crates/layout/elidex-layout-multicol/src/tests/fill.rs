//! `column-fill` (auto / balance) and overflow-column tests.

use super::*;

#[test]
fn auto_vs_balance_fill() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // With auto fill and definite height=200, container uses 200.
    assert_eq!(lb.content.size.height, 200.0);
}

// --- Overflow columns ---
#[test]
fn overflow_columns_beyond_count() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(50.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Three children that won't fit in 2 columns at 50px each
    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // With definite height + auto fill, container height = definite height
    assert_eq!(lb.content.size.height, 50.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Overflow columns (CSS Multicol L1 §8.2): 3 children at 50px each into
    // 50px-tall columns → 3 realized columns, one beyond the requested count=2.
    // Asserting the total column count (not just non-empty) validates that
    // overflow columns were actually created.
    let total_columns: u32 = info.segments.iter().map(|&(count, _, _)| count).sum();
    assert!(
        total_columns > 2,
        "expected overflow columns beyond count=2, got {total_columns}"
    );
}

fn cursor_font_db() -> FontDatabase {
    FontDatabase::from_fixture(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../core/elidex-render/tests/fixtures/digits.ttf"
    )))
}

#[test]
fn direct_text_resumes_in_each_column_without_a_spanning_child_snapshot() {
    use crate::fill::{fill_columns_balanced, fill_columns_sequential, ColumnFillEnv};
    use elidex_layout_block::LayoutEnv;
    let font_db = cursor_font_db();
    for balanced in [false, true] {
        let mut dom = EcsDom::new();
        let owner = elem(&mut dom, "div");
        let style = ComputedStyle {
            display: Display::Block,
            orphans: 1,
            widows: 1,
            font_family: vec!["Elidex Test Digits".into()],
            ..Default::default()
        };
        let wm = elidex_plugin::WritingModeContext::new(style.writing_mode, style.direction);
        dom.world_mut().insert_one(owner, style).unwrap();
        let text = dom.create_text_ownerless_fixture("A B C D");
        dom.append_child(owner, text);
        let input = make_input(&font_db);
        let env = ColumnFillEnv {
            input: &input,
            geom: &crate::algo::ColumnGeometry {
                count: 2,
                width: 1.0,
                gap: 0.0,
            },
            env: &LayoutEnv::from_input(&input, layout_child_fn),
            parent_entity: owner,
            col_ctx: &crate::ColumnLayoutCtx {
                content_origin: Point::ZERO,
                block_offset: 0.0,
                wm,
            },
        };
        let columns = if balanced {
            fill_columns_balanced(&mut dom, &[text], &env, Some(1.0)).0
        } else {
            fill_columns_sequential(&mut dom, &[text], &env, 1.0, 10, false)
        };
        let columns = columns.fragments;
        assert_eq!(columns.len(), 4);
        assert!(columns.iter().all(|col| col.box_snapshots.is_empty()));
        let slice = dom
            .world()
            .get::<&elidex_ecs::ColumnFlowSlice>(owner)
            .unwrap();
        let suffix: String = slice
            .flow_groups
            .iter()
            .flat_map(|(_, lines)| lines)
            .flat_map(|line| &line.runs)
            .filter_map(|r| r.text())
            .collect();
        assert_eq!(suffix, "D");
    }
}

#[test]
fn nested_text_between_siblings_survives_auto_and_balanced_fill() {
    let font_db = cursor_font_db();
    let words = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima";
    for fill in [ColumnFill::Auto, ColumnFill::Balance] {
        let mut dom = EcsDom::new();
        let owner = elem(&mut dom, "div");
        dom.world_mut()
            .insert_one(
                owner,
                ComputedStyle {
                    display: Display::Block,
                    column_count: Some(2),
                    column_fill: fill,
                    height: Dimension::Length(45.0),
                    ..Default::default()
                },
            )
            .unwrap();
        let (first, a) = add_text_block(&mut dom, owner, "before", 16.0);
        let (split, b) = add_text_block(&mut dom, owner, words, 16.0);
        {
            let mut style = dom.world().get::<&mut ComputedStyle>(split).unwrap();
            style.height = Dimension::Auto;
            style.orphans = 1;
            style.widows = 1;
        }
        let (last, c) = add_text_block(&mut dom, owner, "after", 16.0);
        for child in [first, split, last] {
            dom.world()
                .get::<&mut ComputedStyle>(child)
                .unwrap()
                .font_family = vec!["Elidex Test Digits".into()];
        }
        let input = LayoutInput {
            containing: CssSize::definite(160.0, 800.0),
            containing_inline_size: 160.0,
            ..make_input(&font_db)
        };
        layout_multicol(&mut dom, owner, &input, layout_child_fn);
        for (key, expected) in [(a, "before"), (b, words), (c, "after")] {
            let flow = dom.world().get::<&InlineFlow>(key).unwrap();
            let actual: Vec<_> = flow
                .fragments
                .iter()
                .flat_map(|f| &f.lines)
                .flat_map(|line| &line.runs)
                .filter_map(|r| r.text())
                .flat_map(str::split_whitespace)
                .collect();
            assert_eq!(actual, expected.split_whitespace().collect::<Vec<_>>());
        }
        assert!(dom.fragment_tree().fragments_for(split).count() > 1);
        assert_eq!(dom.fragment_tree().fragments_for(first).count(), 0);
        assert_eq!(dom.fragment_tree().fragments_for(last).count(), 0);
    }
}

#[test]
fn mixed_child_keeps_both_run_keys_in_the_resumed_column() {
    let font_db = cursor_font_db();
    for fill in [ColumnFill::Auto, ColumnFill::Balance] {
        for tail in ["D", "D E F"] {
            let mut dom = EcsDom::new();
            let owner = elem(&mut dom, "div");
            dom.world_mut()
                .insert_one(
                    owner,
                    ComputedStyle {
                        display: Display::Block,
                        column_count: Some(2),
                        column_fill: fill,
                        height: Dimension::Length(32.0),
                        ..Default::default()
                    },
                )
                .unwrap();
            let child = elem(&mut dom, "div");
            dom.append_child(owner, child);
            dom.world_mut()
                .insert_one(
                    child,
                    ComputedStyle {
                        display: Display::Block,
                        font_family: vec!["Elidex Test Digits".into()],
                        line_height: elidex_plugin::LineHeight::Px(16.0),
                        orphans: 1,
                        widows: 1,
                        ..Default::default()
                    },
                )
                .unwrap();
            let first = dom.create_text_ownerless_fixture("A B C");
            dom.append_child(child, first);
            add_block_child(&mut dom, child, 0.0);
            let last = dom.create_text_ownerless_fixture(tail);
            dom.append_child(child, last);
            let input = LayoutInput {
                containing: CssSize::definite(2.0, 100.0),
                containing_inline_size: 2.0,
                ..make_input(&font_db)
            };
            layout_multicol(&mut dom, owner, &input, layout_child_fn);
            for (key, expected) in [(first, "A B C"), (last, tail)] {
                let flow = dom.world().get::<&InlineFlow>(key).unwrap();
                let words: Vec<_> = flow
                    .fragments
                    .iter()
                    .flat_map(|f| &f.lines)
                    .flat_map(|l| &l.runs)
                    .filter_map(|r| r.text())
                    .flat_map(str::split_whitespace)
                    .collect();
                assert_eq!(words, expected.split_whitespace().collect::<Vec<_>>());
            }
        }
    }
}

#[test]
fn overflow_column_cap_preserves_remaining_source_in_both_media() {
    use elidex_layout_block::{BreakTokenData, FragmentainerContext, FragmentationType};
    let fonts = make_font_db();
    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    for (outer, count) in [
        (Some(&frag), 1000),
        (None, 1000),
        (Some(&frag), 1001),
        (None, 1001),
    ] {
        let mut dom = EcsDom::new();
        let owner = elem(&mut dom, "div");
        dom.world_mut().insert_one(
            owner,
            ComputedStyle {
                display: Display::Block,
                column_count: Some(2),
                column_gap: Dimension::Length(0.0),
                column_fill: ColumnFill::Auto,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );
        let children: Vec<_> = (0..count)
            .map(|i| {
                let id = add_block_child(&mut dom, owner, 1.0);
                if i < 1000 {
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(id)
                        .unwrap()
                        .break_after = elidex_plugin::BreakValue::Column;
                }
                id
            })
            .collect();
        let input = LayoutInput {
            medium: if outer.is_some() {
                elidex_plugin::Medium::Print
            } else {
                elidex_plugin::Medium::Screen
            },
            fragmentainer: outer,
            ..make_input(&fonts)
        };
        let first = layout_multicol(&mut dom, owner, &input, layout_child_fn);
        assert!(first.contribution.is_finished(owner, children[999]));
        if count == 1000 {
            assert!(first.end.is_complete());
            assert_eq!(
                dom.world().get::<&MulticolInfo>(owner).unwrap().segments[0].0,
                1000
            );
        } else if outer.is_some() {
            assert!(!first.contribution.is_finished(owner, children[1000]));
            let token = first.end.into_token().unwrap();
            assert!(
                matches!(&token.mode_data, Some(BreakTokenData::Multicol {segment_index: 0, column}) if matches!(&column.mode_data, Some(BreakTokenData::Block { child_index: 1000, .. })))
            );
            assert_invalid_segment_rejected(&mut dom, owner, &input, &token);
            let tail = layout_multicol(
                &mut dom,
                owner,
                &LayoutInput {
                    break_token: Some(&token),
                    ..input
                },
                layout_child_fn,
            );
            assert_eq!(
                tail.contribution.contributed_items().collect::<Vec<_>>(),
                [(owner, children[1000])]
            );
            assert!(tail.end.is_complete());
        } else {
            assert!(first.contribution.is_finished(owner, children[1000]));
            assert!(first.end.is_complete());
            assert_eq!(
                dom.world()
                    .get::<&LayoutBox>(children[1000])
                    .unwrap()
                    .content
                    .origin
                    .x,
                300_000.0
            );
        }
    }
}

fn assert_invalid_segment_rejected(
    dom: &mut EcsDom,
    owner: Entity,
    input: &LayoutInput<'_>,
    token: &elidex_layout_block::BreakToken,
) {
    let mut bad = token.clone();
    if let Some(elidex_layout_block::BreakTokenData::Multicol { segment_index, .. }) =
        &mut bad.mode_data
    {
        *segment_index = 1;
    }
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        layout_multicol(
            dom,
            owner,
            &LayoutInput {
                break_token: Some(&bad),
                layout_generation: 5,
                ..*input
            },
            layout_child_fn,
        )
    }))
    .is_err());
    assert_ne!(
        dom.world()
            .get::<&LayoutBox>(owner)
            .unwrap()
            .layout_generation,
        5
    );
}

#[test]
fn replay_columns_with_end_of_source_size_progress() {
    use crate::fill::{fill_columns_sequential, ColumnFillEnv};
    let fonts = make_font_db();
    let mut dom = EcsDom::new();
    let owner = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..Default::default()
    };
    let wm = elidex_plugin::WritingModeContext::new(style.writing_mode, style.direction);
    dom.world_mut().insert_one(owner, style);
    let child = add_block_child(&mut dom, owner, 210.0);
    let prefix = add_block_child(&mut dom, child, 16.0);
    let suffix = add_block_child(&mut dom, child, 16.0);
    dom.world_mut()
        .get::<&mut ComputedStyle>(suffix)
        .unwrap()
        .break_before = elidex_plugin::BreakValue::Column;
    let geom = crate::algo::ColumnGeometry {
        count: 2,
        width: 100.0,
        gap: 0.0,
    };
    let ctx = crate::ColumnLayoutCtx {
        content_origin: Point::ZERO,
        block_offset: 0.0,
        wm,
    };
    let mut saved = Vec::new();
    for replay in [false, true] {
        let mut carry = None;
        for (i, (room, expected)) in [
            (95.0, 95.0),
            (60.0, 60.0),
            (0.0, 0.0),
            (30.0, 30.0),
            (30.0, 25.0),
        ]
        .into_iter()
        .enumerate()
        {
            if !replay {
                saved.push(carry.clone());
            }
            let input = LayoutInput {
                break_token: saved[i].as_ref(),
                ..make_input(&fonts)
            };
            let env = ColumnFillEnv {
                input: &input,
                geom: &geom,
                env: &elidex_layout_block::LayoutEnv::from_input(&input, layout_child_fn),
                parent_entity: owner,
                col_ctx: &ctx,
            };
            let batch = fill_columns_sequential(&mut dom, &[child], &env, room, 1, false);
            assert_eq!(batch.fragments.len(), 1);
            assert_eq!(batch.stalled, i == 2, "column {i}");
            let source = &batch.fragments[0].contribution;
            assert_eq!(source.is_finished(child, prefix), i == 0);
            assert_eq!(source.is_finished(child, suffix), i == 1);
            assert_eq!(
                dom.world()
                    .get::<&LayoutBox>(child)
                    .unwrap()
                    .content
                    .size
                    .height,
                expected
            );
            carry = batch.carry;
            assert_eq!(carry.is_none(), i == 4);
        }
    }
}
