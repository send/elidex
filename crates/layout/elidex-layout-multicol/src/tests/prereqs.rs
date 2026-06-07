//! Z-1b-0 prerequisites for the render store-consume (still dark data): the three
//! nested-multicol invariants the consumer (Z-1b) will depend on.
//!
//! - **P1** probe-write suppression: an ANCESTOR multicol's balanced probe must
//!   not leave garbage fragments in the store (the F1 "definitive-pass-only"
//!   structure protects a multicol's OWN probes but not an ancestor's;
//!   `LayoutInput.is_probe` closes it).
//! - **P2** fragment-tree ancestor-shift: a subtree shift (`shift_descendants`)
//!   moves a standalone box fragment too — else it would paint at its pre-shift
//!   position once consumed.
//! - **P3** precise mid-break signal: `col_children` membership (which drives the
//!   column shift) uses the precise `child_break_token`, not the coarse positional
//!   proxy, so a `break-before: column` deferral is not misclassified as mid-break.

use super::box_fragment::add_spanning_block;
use super::*;
use elidex_ecs::FragmentContent;
use elidex_plugin::{is_multicol, LayoutBox, Vector};

/// A `layout_child` that dispatches a multicol child to [`layout_multicol`]
/// (recursively), so a NESTED multicol is exercised (the plain
/// [`layout_child_fn`] only routes block layout).
fn layout_child_nested(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    let is_mc = dom
        .world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .is_some_and(|s| is_multicol(&s));
    if is_mc {
        crate::layout_multicol(dom, entity, input, layout_child_nested)
    } else {
        elidex_layout_block::block::layout_block_inner(dom, entity, input, layout_child_nested)
    }
}

fn box_fragment_cols(dom: &EcsDom, entity: Entity) -> Vec<(u32, f32, f32)> {
    dom.fragment_tree()
        .fragments_for(entity)
        .map(|n| {
            let FragmentContent::Box(bf) = &n.content;
            (n.fragmentainer, bf.content.origin.x, bf.content.origin.y)
        })
        .collect()
}

/// Add a multicol container child of `parent` with `count` columns and an
/// explicit block-size (definite → sequential fill, deterministic mid-break).
fn add_multicol(dom: &mut EcsDom, parent: Entity, count: u32, height: f32) -> Entity {
    let mc = elem(dom, "div");
    dom.append_child(parent, mc);
    let _ = dom.world_mut().insert_one(
        mc,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(count),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(height),
            ..ComputedStyle::default()
        },
    );
    mc
}

#[test]
fn p1_nested_multicol_in_balanced_probe_leaves_no_store_garbage() {
    // OUTER multicol balances (auto block-size → binary-search probes). Its child
    // is an INNER multicol whose spanning div breaks across the inner's two
    // columns. Each outer probe re-lays the inner → the inner's
    // `position_column_fragments` runs each time. Without P1 the inner would append
    // its 2 box fragments on EVERY probe pass (≤12), accumulating garbage in the
    // append-only store; with P1 the inner inherits `is_probe` during the outer's
    // probes and writes its fragments ONLY on the outer's definitive pass — exactly
    // 2. (The store analogue of F1, but for an ANCESTOR's probes.)
    let mut dom = EcsDom::new();
    let outer = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        outer,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Balance, // auto block-size ⇒ binary-search probes
            ..ComputedStyle::default()
        },
    );
    // Inner multicol: 50px tall, 2 columns; its spanning div (two 50px parts)
    // breaks col-0 → col-1 inside the inner.
    let inner = add_multicol(&mut dom, outer, 2, 50.0);
    let span = add_spanning_block(&mut dom, inner, 2, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, outer, &input, layout_child_nested);

    let frags = box_fragment_cols(&dom, span);
    assert_eq!(
        frags.len(),
        2,
        "inner spanning div has EXACTLY 2 fragments — no ancestor-probe garbage (P1), got {frags:?}"
    );
    assert_eq!(frags[0].0, 0, "inner column 0");
    assert_eq!(frags[1].0, 1, "inner column 1");
}

#[test]
fn p1_plain_multicol_unaffected_by_is_probe() {
    // A non-nested multicol's definitive pass still writes (is_probe=false) — P1 is
    // a no-op for the common case. (Guards against the suppression over-firing.)
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    let span = add_spanning_block(&mut dom, container, 2, 50.0);
    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);
    assert_eq!(
        box_fragment_cols(&dom, span).len(),
        2,
        "plain multicol writes its 2 fragments"
    );
}

#[test]
fn p1_abspos_multicol_under_ancestor_probe_leaves_no_store_garbage() {
    // The OTHER ancestor-probe path (not column-content nesting): an
    // absolutely-positioned multicol reached via `layout_positioned_children`,
    // whose `pos_env` inherits `is_probe`. An OUTER balanced multicol probes; its
    // in-flow child establishes a containing block (`position: relative`) holding
    // an ABSPOS multicol whose spanning div breaks across its two columns. During
    // each outer probe the abspos multicol is laid via the positioned path — its
    // definitive-child input must inherit `env.is_probe` (NOT hardcode false), else
    // it writes box fragments on every probe pass. Asserts exactly the 2 definitive
    // fragments.
    let mut dom = EcsDom::new();
    let outer = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        outer,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Balance, // auto block-size ⇒ probes
            ..ComputedStyle::default()
        },
    );
    // In-flow CB child (position:relative) holding the abspos multicol.
    let cb = elem(&mut dom, "div");
    dom.append_child(outer, cb);
    let _ = dom.world_mut().insert_one(
        cb,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    let mc = elem(&mut dom, "div");
    dom.append_child(cb, mc);
    let _ = dom.world_mut().insert_one(
        mc,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    let span = add_spanning_block(&mut dom, mc, 2, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, outer, &input, layout_child_nested);

    assert_eq!(
        box_fragment_cols(&dom, span).len(),
        2,
        "abspos multicol under ancestor probe writes EXACTLY 2 fragments — no garbage (P1 abspos path)"
    );
}

#[test]
fn p2_ancestor_shift_moves_box_fragments() {
    // Lay a mid-break multicol, then shift the container subtree like an ancestor
    // (relpos / margin-collapse / outer-multicol) would: `shift_descendants` moves
    // the standalone box fragments too (P2), keeping them absolute-correct.
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    let span = add_spanning_block(&mut dom, container, 2, 50.0);
    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let before = box_fragment_cols(&dom, span);
    assert_eq!(before.len(), 2);
    let (c0x, c0y) = (before[0].1, before[0].2);
    let (c1x, c1y) = (before[1].1, before[1].2);

    // Ancestor reposition (e.g. relpos top/left, or an outer multicol's column
    // shift of THIS whole multicol): fragments move with the subtree.
    elidex_layout_block::block::shift_descendants(&mut dom, &[container], Vector::new(40.0, 70.0));

    let after = box_fragment_cols(&dom, span);
    assert_eq!(after.len(), 2, "shift does not add/remove fragments");
    assert!((after[0].1 - (c0x + 40.0)).abs() < 0.01, "col0 x +40");
    assert!((after[0].2 - (c0y + 70.0)).abs() < 0.01, "col0 y +70");
    assert!((after[1].1 - (c1x + 40.0)).abs() < 0.01, "col1 x +40");
    assert!((after[1].2 - (c1y + 70.0)).abs() < 0.01, "col1 y +70");
}

#[test]
fn p2_excluding_fragments_variant_does_not_move_box_fragments() {
    // The multicol's OWN column-positioning shifter
    // (`shift_descendants_excluding_fragments`) must NOT move the born-absolute box
    // fragments (their column offset is baked at commit) — else the column shift
    // would double-apply it. This is exactly why `position_column_fragments` uses
    // the excluding variant.
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    let span = add_spanning_block(&mut dom, container, 2, 50.0);
    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let before = box_fragment_cols(&dom, span);
    elidex_layout_block::block::shift_descendants_excluding_fragments(
        &mut dom,
        &[container],
        Vector::new(40.0, 70.0),
    );
    let after = box_fragment_cols(&dom, span);
    assert_eq!(
        before, after,
        "excluding-fragments shift leaves box fragments put"
    );
}

#[test]
fn p3_deferred_whole_child_is_positioned_once_not_double_shifted() {
    // P3 converges `col_children` (which drives the per-column LayoutBox/InlineFlow
    // shift) onto the precise `child_break_token` signal the box capture already
    // uses. The coarse `break_token.is_some()` positional proxy it replaces could,
    // at a `next == prev` resume point with no actual child split, misclassify a
    // deferred-WHOLE child as mid-break and additively double-shift it. That exact
    // double-shift is currently UNREACHABLE (this engine defers a child to a clean
    // column boundary before ever splitting it, so coarse and precise coincide —
    // the Z-1a `child_break_token` investigation), but the precise signal removes
    // the latent risk and is the single source for both the shift membership and
    // the box snapshot. This pins the reachable guarantee: a whole sibling that
    // does not fit in the remaining space is deferred whole to the NEXT column and
    // positioned there exactly ONCE — its `LayoutBox` lands at that column's offset
    // (no double-shift) and it gets NO box fragment (it is not a mid-break child).
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            height: Dimension::Length(50.0),
            ..ComputedStyle::default()
        },
    );
    let _a = add_block_child(&mut dom, container, 30.0); // whole in column 0 (30 < 50)
    let b = add_block_child(&mut dom, container, 40.0); // 40 > remaining 20 → deferred whole to col 1

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    // Container 600 wide, 2 columns, gap 0 → column width 300. B is deferred whole
    // to column 1, so its single LayoutBox sits at x≈300 — shifted to the column
    // exactly once (a double-shift would land it at 600).
    let b_x = dom
        .world()
        .get::<&LayoutBox>(b)
        .expect("B is laid out (whole) in column 1")
        .content
        .origin
        .x;
    assert!(
        (b_x - 300.0).abs() < 1.0,
        "deferred-whole B at column-1 offset x≈300 (positioned once, not double-shifted), got {b_x}"
    );
    assert_eq!(
        box_fragment_cols(&dom, b).len(),
        0,
        "deferred-whole B is not a mid-break child ⇒ no box fragment (precise signal, no false-positive)"
    );
}
