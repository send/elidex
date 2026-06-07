use super::*;
use elidex_ecs::{InlineFlow, InlineFlowRun};

#[test]
fn persists_atomic_inline_as_box_member() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    // Append a static inline-block after the text → the run contains an atomic.
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(20.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // Slice 3p-a: a static atomic now persists as an `AtomicBox` member (was gated).
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("a run with a static atomic inline now persists an InlineFlow (slice 3p-a)");
    // Find the AtomicBox member and the block_start of the line it landed on. (The
    // test's `layout_block_only` child lays the inline-block out at full container
    // width, so it wraps below the text onto its own line — which makes the D7
    // reposition's block-axis move observable: the box moves from content_origin
    // y=0 down to the line.)
    let (atomic_inline, atomic_block) = flow.fragments[0]
        .lines
        .iter()
        .flat_map(|line| line.runs.iter().map(move |r| (r, line.block_start)))
        .find_map(|(r, block_start)| match r {
            InlineFlowRun::AtomicBox {
                entity,
                inline_start,
            } if *entity == ib => Some((*inline_start, block_start)),
            _ => None,
        })
        .expect("the inline-block must be recorded as an AtomicBox member of the flow");
    assert!(
        atomic_block > 0.0 || atomic_inline > 0.0,
        "the atomic is placed away from the IFC origin on its line, got ({atomic_inline}, {atomic_block})"
    );

    // D7: layout repositioned the atomic's LayoutBox from content_origin (0,0) to its
    // member position (margins are 0, so margin-box origin == content origin).
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic was laid out and has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_inline).abs() < 0.5
            && (lb.content.origin.y - atomic_block).abs() < 0.5,
        "atomic LayoutBox repositioned to its member position ({atomic_inline}, {atomic_block}), \
         got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

// NOTE: the slice-3p-b-2 inversion of the old `gate_excludes_relative_positioned_atomic`
// test (a relpos/sticky atomic now persists + repositions instead of gating) lives in
// the sibling `relpos_subflow` module alongside the other positioned-inline tests.

#[test]
fn vertical_atomic_repositions_with_axis_swap() {
    // Vertical WM (slice 2 persists it): the D7 reposition projects inline-axis →
    // physical y and block-axis → physical x (the `is_vertical` swap). Assert the
    // static atomic persists as an `AtomicBox` and its `LayoutBox` lands at the
    // swapped member position.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("a") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(20.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("vertical static atomic persists (slice 2 + 3p-a)");
    let (atomic_inline, atomic_block) = flow.fragments[0]
        .lines
        .iter()
        .flat_map(|line| line.runs.iter().map(move |r| (r, line.block_start)))
        .find_map(|(r, block_start)| match r {
            InlineFlowRun::AtomicBox {
                entity,
                inline_start,
            } if *entity == ib => Some((*inline_start, block_start)),
            _ => None,
        })
        .expect("the inline-block is an AtomicBox member in vertical mode");

    // is_vertical projection: inline-axis → physical y, block-axis → physical x.
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic was laid out and has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_block).abs() < 0.5
            && (lb.content.origin.y - atomic_inline).abs() < 0.5,
        "vertical atomic box at the swapped position (x=block {atomic_block}, \
         y=inline {atomic_inline}), got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

#[test]
fn atomic_inner_text_inline_flow_shifts_with_box() {
    // An inline-block CONTAINING converged text: when the atomic is repositioned to
    // its line, the inner text's persisted `InlineFlow` (absolute coords, consumed
    // directly by render) must shift with the box — else render repaints the inner
    // text at the pre-reposition (content_origin) position. Root fix in
    // `shift_descendants` (covers relpos/abspos subtree shifts too).
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(40.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    let inner = dom.create_text("hi");
    dom.append_child(ib, inner);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // The atomic was repositioned to its line (the stub lays it out full-width, so it
    // wraps below "a") → its box is off content_origin.
    let box_y = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic has a LayoutBox")
        .content
        .origin
        .y;
    // The inner text persisted its own InlineFlow (the atomic's inner IFC); its
    // block_start must have tracked the box, not stayed at the pre-shift origin.
    let inner_block = dom
        .world()
        .get::<&InlineFlow>(inner)
        .expect("the inner text persists an InlineFlow")
        .fragments[0]
        .lines[0]
        .block_start;
    assert!(
        inner_block > 0.5,
        "the inner text's InlineFlow moved off content_origin with the box, got {inner_block}"
    );
    assert!(
        (inner_block - box_y).abs() < 0.5,
        "inner text InlineFlow tracks the repositioned box (box y {box_y}, inner block_start {inner_block})"
    );
}
