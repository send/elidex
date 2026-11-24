//! Z-1a: multicol mid-break **box-fragment** population into the standalone
//! fragment tree (§15.4.1). Dark data — render does not yet consume it (Z-1b),
//! so these assert the layout-side population only, plus the F1 definitive-only
//! invariant (probes leave no garbage) and the spanning-only scope (whole-in-
//! column children keep using the shifted `LayoutBox`, not a box fragment).

use super::*;
use elidex_ecs::FragmentContent;

/// A block child holding `parts` fixed-height block children, so it breaks at
/// each child boundary when a column fits only one part. `parts == 2` spans two
/// columns (col-0 → col-1); `parts == 3` spans three (exercising the middle
/// column, where a child both resumes-into AND breaks-out-of the column). Returns
/// the spanning div (the mid-break direct child of the multicol container).
pub(super) fn add_spanning_block(
    dom: &mut EcsDom,
    parent: Entity,
    parts: usize,
    part_height: f32,
) -> Entity {
    let div = elem(dom, "div");
    dom.append_child(parent, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        },
    );
    for _ in 0..parts {
        add_block_child(dom, div, part_height);
    }
    div
}

/// `(fragmentainer, content-origin-x)` for each of `entity`'s box fragments, in
/// insertion (column) order.
fn box_fragments(dom: &EcsDom, entity: Entity) -> Vec<(u32, f32)> {
    dom.fragment_tree()
        .fragments_for(entity)
        .map(|n| {
            let FragmentContent::Box(bf) = &n.content;
            (n.fragmentainer, bf.content.origin.x)
        })
        .collect()
}

#[test]
fn multicol_midbreak_block_populates_one_box_fragment_per_column() {
    // The div's content (two 50px blocks) is taller than the 50px column, so it
    // breaks at the child boundary: child-0 in column 0, child-1 in column 1. The
    // div is a mid-break direct child of the multicol → the standalone fragment
    // tree gets ONE box fragment per column it spans (replacing G11 last-column-
    // wins, which kept only the last column's `LayoutBox`).
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(50.0), // definite → sequential; column height 50
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);
    let span = add_spanning_block(&mut dom, container, 2, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Container 600 wide, 2 columns, gap 0 → column width 300. The div's col-0
    // fragment sits at x≈0, its col-1 fragment shifted to x≈300 (i × (width+gap)).
    let frags = box_fragments(&dom, span);
    assert_eq!(
        frags.len(),
        2,
        "spanning div gets one box fragment per column, got {frags:?}"
    );
    assert_eq!(frags[0].0, 0, "first fragment in column 0");
    assert_eq!(frags[1].0, 1, "second fragment in column 1");
    assert!(
        frags[0].1.abs() < 1.0,
        "column-0 fragment at x≈0, got {}",
        frags[0].1
    );
    assert!(
        (frags[1].1 - 300.0).abs() < 1.0,
        "column-1 fragment shifted to the exact column offset x≈300, got {}",
        frags[1].1
    );

    // Behavior-neutral (dark data): Z-1a does NOT remove the mid-break child's
    // `LayoutBox` (that is Z-1b, paired with the render store-consume). The box is
    // retained, so the child still paints via legacy/G11 exactly as before — the
    // store is purely additive. (The render-doesn't-read-the-store invariant, F7,
    // is verified structurally: `crates/core/elidex-render/` has zero `fragment_tree`
    // references, and the render + shell golden suites show zero churn.)
    assert!(
        dom.world().get::<&LayoutBox>(span).is_ok(),
        "Z-1a retains the LayoutBox (removal is Z-1b) — population is purely additive"
    );
}

#[test]
fn multicol_midbreak_block_spanning_three_columns_populates_three_fragments() {
    // A div whose content (three 50px blocks) spans THREE 50px columns exercises
    // the middle column (column 1), where the div both RESUMES into the column
    // (`carry_midbreak`) AND BREAKS OUT of it (`is_midbreak`) — the dedup path that
    // must snapshot the middle fragment exactly once. The 2-column tests never hit
    // this (the resume-finish column is also the last). Plan §8 "spans 3+ columns".
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(3),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(50.0), // column height 50 → one 50px part per column
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);
    let span = add_spanning_block(&mut dom, container, 3, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Container 600 wide, 3 columns, gap 0 → column width 200. One fragment per
    // column at x = i × 200, the middle one snapshotted exactly once (no dedup dup).
    let frags = box_fragments(&dom, span);
    assert_eq!(
        frags.len(),
        3,
        "spanning div gets exactly one box fragment per column (middle deduped once), got {frags:?}"
    );
    for (i, &(fragmentainer, x)) in frags.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let expected_col = i as u32;
        #[allow(clippy::cast_precision_loss)]
        let expected_x = i as f32 * 200.0;
        assert_eq!(fragmentainer, expected_col, "fragment {i} in column {i}");
        assert!(
            (x - expected_x).abs() < 1.0,
            "fragment {i} at column offset x≈{expected_x}, got {x}"
        );
    }
}

#[test]
fn multicol_splits_a_new_child_only_if_its_prefix_fits() {
    use crate::fill::{fill_columns_sequential, ColumnFillEnv};
    let fonts = make_font_db();
    for (first_height, splits) in [(30.0, true), (50.0, false)] {
        let mut dom = EcsDom::new();
        let owner = elem(&mut dom, "div");
        let style = ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..Default::default()
        };
        let wm = elidex_plugin::WritingModeContext::new(style.writing_mode, style.direction);
        dom.world_mut().insert_one(owner, style);
        let a = add_block_child(&mut dom, owner, first_height);
        let b = add_spanning_block(&mut dom, owner, 2, 15.0);
        let parts = dom.children(b);
        let input = make_input(&fonts);
        let env = ColumnFillEnv {
            input: &input,
            env: &elidex_layout_block::LayoutEnv::from_input(&input, layout_child_fn),
            geom: &crate::algo::ColumnGeometry {
                count: 2,
                width: 300.0,
                gap: 0.0,
            },
            parent_entity: owner,
            col_ctx: &crate::ColumnLayoutCtx {
                content_origin: Point::ZERO,
                block_offset: 0.0,
                wm,
            },
        };
        let batch = fill_columns_sequential(&mut dom, &[a, b], &env, 50.0, 10, false);
        assert!(batch.carry.is_none());
        assert_eq!(batch.fragments.len(), 2);
        for (i, fragment) in batch.fragments.iter().enumerate() {
            assert_eq!(
                fragment.contribution.is_finished(b, parts[0]),
                if splits { i == 0 } else { i == 1 }
            );
            assert_eq!(fragment.contribution.is_finished(b, parts[1]), i == 1);
        }
        layout_multicol(&mut dom, owner, &input, layout_child_fn);
        let stored = box_fragments(&dom, b);
        if splits {
            assert_eq!(stored, [(0, 0.0), (1, 300.0)]);
        } else {
            assert!(stored.is_empty());
        }
    }
}

#[test]
fn multicol_whole_in_column_block_has_no_box_fragment() {
    // A block that fits entirely in one column is NOT a mid-break child → it uses
    // the shifted `LayoutBox` (I-multicol), NOT a standalone box fragment. Only
    // spanning children populate the fragment tree — the spanning-only scope.
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
    let a = add_block_child(&mut dom, container, 50.0); // whole in column 0
    let b = add_block_child(&mut dom, container, 50.0); // whole in column 1

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    assert_eq!(
        box_fragments(&dom, a).len(),
        0,
        "whole-in-column A: no fragment"
    );
    assert_eq!(
        box_fragments(&dom, b).len(),
        0,
        "whole-in-column B: no fragment"
    );
    assert!(
        dom.fragment_tree().is_empty(),
        "no spanning child ⇒ empty fragment tree"
    );
}

#[test]
fn multicol_balanced_midbreak_no_probe_leftover() {
    // F1 (definitive-pass-only write): balanced fill re-lays via ≤12 probe passes
    // (`probe_total_height` + binary search), but box fragments are committed ONLY
    // in `position_column_fragments`, which runs ONLY on the definitive pass — so a
    // mid-break div ends with EXACTLY its 2 real per-column fragments, no probe
    // garbage accumulated in the append-only fragment tree. (The store analogue of
    // `multicol_balanced_persists_one_fragment_per_run`.)
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Balance, // → binary-search probes
        ..ComputedStyle::default()        // auto block-size → always balances
    };
    let _ = dom.world_mut().insert_one(container, style);
    // 100px content, 2 columns → balances to ~50px columns → div breaks col-0/col-1.
    let span = add_spanning_block(&mut dom, container, 2, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let frags = box_fragments(&dom, span);
    assert_eq!(
        frags.len(),
        2,
        "exactly the 2 real per-column fragments — no probe leftover, got {frags:?}"
    );
    assert_eq!(frags[0].0, 0);
    assert_eq!(frags[1].0, 1);
}

#[test]
fn split_child_positions_its_two_fragments_in_nonzero_columns() {
    use crate::fill::ColumnFillEnv;
    use elidex_layout_block::LayoutEnv;
    use elidex_plugin::BreakValue;

    let font_db = make_font_db();
    for column_fill in [ColumnFill::Auto, ColumnFill::Balance] {
        let mut dom = EcsDom::new();
        let owner = elem(&mut dom, "div");
        let style = ComputedStyle {
            display: Display::Block,
            column_fill,
            ..Default::default()
        };
        let wm = elidex_plugin::WritingModeContext::new(style.writing_mode, style.direction);
        dom.world_mut().insert_one(owner, style.clone()).unwrap();
        let first = add_block_child(&mut dom, owner, 20.0);
        let before_split = add_block_child(&mut dom, owner, 10.0);
        let span = elem(&mut dom, "div");
        dom.append_child(owner, span);
        dom.world_mut()
            .insert_one(
                span,
                ComputedStyle {
                    display: Display::Block,
                    ..Default::default()
                },
            )
            .unwrap();
        let prefix = add_block_child(&mut dom, span, 10.0);
        let suffix = add_block_child(&mut dom, span, 10.0);
        let last = add_block_child(&mut dom, owner, 10.0);
        for entity in [first, prefix] {
            dom.world_mut()
                .get::<&mut ComputedStyle>(entity)
                .unwrap()
                .break_after = BreakValue::Column;
        }
        let saved_input = make_input(&font_db);
        for generation in [1, 2] {
            let input = elidex_layout_block::LayoutInput {
                layout_generation: generation,
                ..saved_input
            };
            let env = ColumnFillEnv {
                input: &input,
                geom: &crate::algo::ColumnGeometry {
                    count: 3,
                    width: 80.0,
                    gap: 20.0,
                },
                env: &LayoutEnv::from_input(&input, layout_child_fn),
                parent_entity: owner,
                col_ctx: &crate::ColumnLayoutCtx {
                    content_origin: Point::ZERO,
                    block_offset: 0.0,
                    wm,
                },
            };
            let (batch, _) = crate::layout_normal_segment(
                &mut dom,
                &[first, before_split, span, last],
                &env,
                Some(20.0),
                Some(20.0),
                &style,
            );
            let columns = batch.fragments;
            assert_eq!(
                columns
                    .iter()
                    .map(|col| col.children.as_slice())
                    .collect::<Vec<_>>(),
                [&[first][..], &[before_split, span], &[span, last]]
            );
            assert_eq!(box_fragments(&dom, span), [(1, 100.0), (2, 200.0)]);
            for (entity, x) in [
                (first, 0.0),
                (before_split, 100.0),
                (prefix, 100.0),
                (span, 200.0),
                (suffix, 200.0),
                (last, 200.0),
            ] {
                assert_eq!(
                    dom.world()
                        .get::<&LayoutBox>(entity)
                        .unwrap()
                        .content
                        .origin
                        .x,
                    x
                );
            }
            assert!(columns[1].contribution.is_finished(span, prefix));
            assert!(!columns[1].contribution.is_finished(owner, span));
            assert!(columns[2].contribution.is_finished(span, suffix));
            assert!(columns[2].contribution.is_finished(owner, span));
            assert!(!columns[2].contribution.is_finished(span, prefix));
        }
    }
}

#[test]
fn slotted_descendant_receives_its_columns_offset_once() {
    let fonts = make_font_db();
    let mut dom = EcsDom::new();
    let owner = elem(&mut dom, "div");
    dom.world_mut().insert_one(
        owner,
        ComputedStyle {
            display: Display::Block,
            column_fill: ColumnFill::Auto,
            column_count: Some(2),
            column_gap: Dimension::Length(0.0),
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    let shadow = dom
        .attach_shadow(owner, elidex_ecs::ShadowRootMode::Open)
        .unwrap();
    let first = add_block_child(&mut dom, shadow, 20.0);
    dom.world_mut()
        .get::<&mut ComputedStyle>(first)
        .unwrap()
        .break_after = elidex_plugin::BreakValue::Column;
    let whole = add_block_child(&mut dom, shadow, 10.0);
    let slot = elem(&mut dom, "slot");
    dom.append_child(whole, slot);
    let slotted = add_block_child(&mut dom, owner, 10.0);
    dom.world_mut().insert(
        slot,
        (
            ComputedStyle {
                display: Display::Contents,
                ..Default::default()
            },
            elidex_ecs::SlotAssignment {
                assigned_nodes: vec![slotted],
            },
        ),
    );
    let outcome = layout_multicol(&mut dom, owner, &make_input(&fonts), layout_child_fn);
    assert!(outcome.contribution.is_finished(whole, slotted));
    for entity in [whole, slotted] {
        assert_eq!(
            dom.world()
                .get::<&LayoutBox>(entity)
                .unwrap()
                .content
                .origin
                .x,
            300.0
        );
    }
}
