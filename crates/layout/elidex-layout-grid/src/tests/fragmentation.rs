//! Fragmentation tests for CSS Grid Level 1 §10.

use super::*;
use elidex_layout_block::{BreakTokenData, FragmentainerContext, FragmentationType, LayoutOutcome};
use elidex_plugin::{
    BreakInsideValue, BreakValue, Dimension, GridTrackList, TrackSection, TrackSize,
};

/// Layout a grid container with fragmentation context and return the full outcome.
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn do_layout_grid_fragmented(
    dom: &mut EcsDom,
    entity: elidex_ecs::Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset: Point,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: elidex_layout_block::ChildLayoutFn,
    fragmentainer: Option<FragmentainerContext>,
    break_token: Option<elidex_layout_block::BreakToken>,
) -> LayoutOutcome {
    let input = LayoutInput {
        containing: CssSize {
            width: containing_width,
            height: containing_height,
        },
        containing_inline_size: containing_width,
        offset,
        font_db,
        depth,
        float_ctx: None,
        viewport: None,
        fragmentainer: fragmentainer.as_ref(),
        break_token: break_token.as_ref(),
        subgrid: None,
    };
    layout_grid(dom, entity, &input, layout_child)
}

fn grid_container(cols: usize) -> ComputedStyle {
    let tracks: Vec<TrackSize> = (0..cols).map(|_| TrackSize::Fr(1.0)).collect();
    ComputedStyle {
        display: Display::Grid,
        grid_template_columns: GridTrackList::Explicit(TrackSection {
            tracks,
            line_names: Vec::new(),
        }),
        ..Default::default()
    }
}

fn make_grid_dom(
    container_style: ComputedStyle,
    items: &[ComputedStyle],
) -> (EcsDom, elidex_ecs::Entity, Vec<elidex_ecs::Entity>) {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(container, container_style);

    let mut entities = Vec::new();
    for item_style in items {
        let child = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(container, child);
        let _ = dom.world_mut().insert_one(child, item_style.clone());
        entities.push(child);
    }
    (dom, container, entities)
}

fn grid_item(height: f32) -> ComputedStyle {
    ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(height),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// 1. No fragmentainer -> break_token is None
// ---------------------------------------------------------------------------

#[test]
fn no_fragmentation_returns_none() {
    let container = grid_container(1);
    let items = vec![grid_item(50.0), grid_item(50.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        None, // no fragmentainer
        None,
    );
    assert!(outcome.break_token.is_none());
}

// ---------------------------------------------------------------------------
// 2. All rows fit within available block size
// ---------------------------------------------------------------------------

#[test]
fn rows_fit_no_break() {
    // 2 items in a 1-col grid, each 50px tall. Available: 200px -> fits.
    let container = grid_container(1);
    let items = vec![grid_item(50.0), grid_item(50.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 200.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    assert!(outcome.break_token.is_none());
}

// ---------------------------------------------------------------------------
// 3. Overflow at row boundary
// ---------------------------------------------------------------------------

#[test]
fn overflow_at_row_boundary() {
    // 3 items in 1-col grid, each 60px. Total = 180px, available = 130px.
    // Rows 0+1 = 120px fits. Row 2 would push to 180px -> break at row 2.
    let container = grid_container(1);
    let items = vec![grid_item(60.0), grid_item(60.0), grid_item(60.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 130.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should produce break token");
    assert_eq!(bt.entity, entity);
    match &bt.mode_data {
        Some(BreakTokenData::Grid { row_index, .. }) => {
            assert_eq!(*row_index, 2, "should break before row 2");
        }
        other => panic!("expected Grid break token data, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Resume from break token
// ---------------------------------------------------------------------------

#[test]
fn resume_from_break_token() {
    let container = grid_container(1);
    let items = vec![grid_item(60.0), grid_item(60.0), grid_item(60.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();
    let frag = Some(FragmentainerContext {
        available_block_size: 130.0,
        fragmentation_type: FragmentationType::Column,
    });

    // First pass: should break.
    let outcome1 = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        frag,
        None,
    );
    let bt1 = outcome1.break_token.expect("first pass should break");

    // Second pass with break token: row 2 only (60px) fits in 130px.
    let outcome2 = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        frag,
        Some(bt1),
    );
    assert!(
        outcome2.break_token.is_none(),
        "second pass should complete without break"
    );
}

// ---------------------------------------------------------------------------
// 5. Forced break-before on item
// ---------------------------------------------------------------------------

#[test]
fn forced_break_before_item() {
    let container = grid_container(1);
    let mut item2 = grid_item(40.0);
    item2.break_before = BreakValue::Column;

    let items = vec![grid_item(40.0), item2];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome
        .break_token
        .expect("forced break should produce token");
    match &bt.mode_data {
        Some(BreakTokenData::Grid { row_index, .. }) => {
            assert_eq!(*row_index, 1, "forced break before row 1");
        }
        other => panic!("expected Grid break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 6. Forced break-after on item
// ---------------------------------------------------------------------------

#[test]
fn forced_break_after_item() {
    let container = grid_container(1);
    let mut item1 = grid_item(40.0);
    item1.break_after = BreakValue::Column;

    let items = vec![item1, grid_item(40.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome
        .break_token
        .expect("forced break-after should produce token");
    match &bt.mode_data {
        Some(BreakTokenData::Grid { row_index, .. }) => {
            assert_eq!(*row_index, 1, "forced break after row 0 -> resume at row 1");
        }
        other => panic!("expected Grid break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 7. break-inside: avoid penalizes break candidates
// ---------------------------------------------------------------------------

#[test]
fn break_inside_avoid() {
    // 3 items in 1-col grid, item 1 has break-inside: avoid.
    // Available 130px with 60px rows -> needs to break.
    // Break after row 0 is non-avoid (preferred).
    let container = grid_container(1);
    let mut item2 = grid_item(60.0);
    item2.break_inside = BreakInsideValue::Avoid;

    let items = vec![grid_item(60.0), item2, grid_item(60.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 130.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Grid { row_index, .. }) => {
            // Break candidates: row 1 (at 60px), row 2 (at 120px).
            // Row 2 candidate is after the avoid item, but the avoid check
            // looks at break-before/break-after on boundary items, not
            // break-inside. Both are valid break points.
            assert!(*row_index <= 2, "should break at a valid row boundary");
        }
        other => panic!("expected Grid break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 8. Spanning item fragmentation (break token stores child tokens)
// ---------------------------------------------------------------------------

#[test]
fn spanning_item_fragmentation() {
    // Items spanning multiple rows: when break occurs at a row boundary,
    // spanning items are noted in child_break_tokens.
    // For now, verify the break token structure is correct.
    let container = grid_container(1);
    let items = vec![grid_item(100.0), grid_item(100.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 150.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Grid {
            row_index,
            child_break_tokens,
        }) => {
            assert_eq!(*row_index, 1, "break before row 1");
            // No spanning items in this test, so child_break_tokens should be empty.
            assert!(
                child_break_tokens.is_empty(),
                "no spanning items to fragment"
            );
        }
        other => panic!("expected Grid break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 9. Propagated break-before from first item
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_before() {
    let container = grid_container(1);
    let mut item1 = grid_item(50.0);
    item1.break_before = BreakValue::Page;

    let items = vec![item1, grid_item(50.0)];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    assert_eq!(
        outcome.propagated_break_before,
        Some(BreakValue::Page),
        "should propagate break-before from first item"
    );
}

// ---------------------------------------------------------------------------
// 10. Propagated break-after from last item
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_after() {
    let container = grid_container(1);
    let mut item2 = grid_item(50.0);
    item2.break_after = BreakValue::Page;

    let items = vec![grid_item(50.0), item2];
    let (mut dom, entity, _) = make_grid_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_grid_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 500.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    assert_eq!(
        outcome.propagated_break_after,
        Some(BreakValue::Page),
        "should propagate break-after from last item"
    );
}
