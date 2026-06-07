//! Z-1a: multicol mid-break **box-fragment** population into the standalone
//! fragment tree (§15.4.1). Dark data — render does not yet consume it (Z-1b),
//! so these assert the layout-side population only, plus the F1 definitive-only
//! invariant (probes leave no garbage) and the spanning-only scope (whole-in-
//! column children keep using the shifted `LayoutBox`, not a box fragment).

use super::*;
use elidex_ecs::FragmentContent;

/// A block child holding two fixed-height block children, so it breaks at the
/// child boundary when a column is shorter than its total height. Returns the
/// spanning div (the mid-break direct child of the multicol container).
fn add_spanning_block(dom: &mut EcsDom, parent: Entity, part_height: f32) -> Entity {
    let div = elem(dom, "div");
    dom.append_child(parent, div);
    let _ = dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        },
    );
    add_block_child(dom, div, part_height);
    add_block_child(dom, div, part_height);
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
    let span = add_spanning_block(&mut dom, container, 50.0);

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
    let span = add_spanning_block(&mut dom, container, 50.0);

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
