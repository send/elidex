//! Fragmentation tests for CSS Flexbox Level 1 §12.

use super::*;
use elidex_layout_block::{BreakTokenData, FragmentainerContext, FragmentationType, LayoutOutcome};
use elidex_plugin::{BreakInsideValue, BreakValue, FlexDirection, FlexWrap, Overflow};

/// Layout a flex container with fragmentation context and return the full outcome.
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn do_layout_flex_fragmented(
    dom: &mut EcsDom,
    entity: Entity,
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
    layout_flex(dom, entity, &input, layout_child)
}

fn wrap_container() -> ComputedStyle {
    ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::Wrap,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// 1. No fragmentainer → break_token is None
// ---------------------------------------------------------------------------

#[test]
fn no_fragmentation_returns_none_break_token() {
    let container = wrap_container();
    let items = vec![flex_item(100.0, 50.0), flex_item(100.0, 50.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
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
// 2. All lines fit within available block size
// ---------------------------------------------------------------------------

#[test]
fn lines_fit_no_break() {
    // 2 items at 50px height each, wrap at 200px → 2 lines of 50px each = 100px.
    // Available: 200px → everything fits.
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let items = vec![flex_item(100.0, 50.0), flex_item(100.0, 50.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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
// 3. Overflow at line boundary → break between lines
// ---------------------------------------------------------------------------

#[test]
fn overflow_at_line_boundary() {
    // Container width: 100px, 3 items each 100x60, wrap → 3 lines of 60px.
    // Total = 180px, available = 130px → break after line 2 (at 120px consumed).
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let items = vec![
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
    ];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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
        Some(BreakTokenData::Flex { line_index, .. }) => {
            assert_eq!(*line_index, 2, "should break before line 2");
        }
        other => panic!("expected Flex break token data, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Resume from break token
// ---------------------------------------------------------------------------

#[test]
fn resume_from_break_token() {
    // Same setup as overflow_at_line_boundary.
    // First pass breaks at line 2. Second pass resumes from line 2.
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let items = vec![
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
    ];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();
    let frag = Some(FragmentainerContext {
        available_block_size: 130.0,
        fragmentation_type: FragmentationType::Column,
    });

    // First pass: should break.
    let outcome1 = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        frag,
        None,
    );
    let bt1 = outcome1.break_token.expect("first pass should break");

    // Second pass with break token: line 2 only (60px) fits in 130px.
    let outcome2 = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        frag,
        Some(bt1),
    );
    // Only 1 remaining line (60px) fits within 130px → no further break.
    assert!(
        outcome2.break_token.is_none(),
        "second pass should complete without break"
    );
}

// ---------------------------------------------------------------------------
// 5. Forced break-before on first item of a line
// ---------------------------------------------------------------------------

#[test]
fn forced_break_before_item() {
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let mut item2 = flex_item(100.0, 40.0);
    item2.break_before = BreakValue::Column;

    let items = vec![flex_item(100.0, 40.0), item2];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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
        Some(BreakTokenData::Flex { line_index, .. }) => {
            assert_eq!(*line_index, 1, "forced break before line 1");
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 6. Forced break-after on last item of a line
// ---------------------------------------------------------------------------

#[test]
fn forced_break_after_item() {
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let mut item1 = flex_item(100.0, 40.0);
    item1.break_after = BreakValue::Column;

    let items = vec![item1, flex_item(100.0, 40.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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
        Some(BreakTokenData::Flex { line_index, .. }) => {
            assert_eq!(
                *line_index, 1,
                "forced break after line 0 → resume at line 1"
            );
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 7. break-inside: avoid penalizes break candidates
// ---------------------------------------------------------------------------

#[test]
fn break_inside_avoid() {
    // 3 lines, second item has break-inside: avoid.
    // Available 130px with 60px lines → needs to break.
    // The break candidate after line 1 (where break-inside:avoid item lives) should
    // be penalized; prefer breaking after line 0 instead if possible... but since
    // break-inside applies to the item itself (not the gap between lines), the
    // candidate between line 0 and line 1 (before the avoid item's line) is fine.
    // The avoid-item is on line 1, and break-inside:avoid makes the candidate
    // AFTER line 1 penalized (items in prev line are checked).
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let mut item2 = flex_item(100.0, 60.0);
    item2.break_inside = BreakInsideValue::Avoid;

    let items = vec![flex_item(100.0, 60.0), item2, flex_item(100.0, 60.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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
    // find_best_break should prefer the non-avoid candidate (between line 0 and 1)
    // over the avoid candidate (between line 1 and 2).
    match &bt.mode_data {
        Some(BreakTokenData::Flex { line_index, .. }) => {
            // Break at line 1 (after line 0) is non-avoid, so preferred
            // even though consumed (60) < available (130).
            // Break at line 2 (after line 1) is avoid.
            // Both fit, but line 1 is non-avoid so find_best_break picks the
            // later non-avoid candidate... actually find_best_break picks
            // non-avoid AND later position. Line 1 candidate (60px) is
            // non-avoid, line 2 candidate (120px) is avoid.
            // non-avoid < avoid in preference, so line 1 wins.
            assert_eq!(*line_index, 1, "should prefer non-avoid break point");
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 8. Single line (nowrap) → monolithic, no break possible
// ---------------------------------------------------------------------------

#[test]
fn single_line_no_break() {
    // Nowrap: all items on one line. Even if it overflows, no break.
    let container = ComputedStyle {
        display: Display::Flex,
        flex_wrap: FlexWrap::Nowrap,
        ..Default::default()
    };
    let items = vec![flex_item(100.0, 200.0), flex_item(100.0, 200.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        300.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    // Single line → no break candidates → no break token.
    assert!(
        outcome.break_token.is_none(),
        "single line (nowrap) should not fragment"
    );
}

// ---------------------------------------------------------------------------
// 9. Multi-line wrap produces break
// ---------------------------------------------------------------------------

#[test]
fn multi_line_wrap_break() {
    // 4 items, each 100x50, container 100px wide → 4 lines.
    // Available 120px → should break after line 2 (at 100px consumed).
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let items = vec![
        flex_item(100.0, 50.0),
        flex_item(100.0, 50.0),
        flex_item(100.0, 50.0),
        flex_item(100.0, 50.0),
    ];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 120.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Flex { line_index, .. }) => {
            // 2 lines of 50px each = 100px fits in 120px.
            // 3 lines = 150px > 120px → break at line 2.
            assert_eq!(*line_index, 2);
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 10. Propagated break-before from first item
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_before() {
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let mut item1 = flex_item(100.0, 50.0);
    item1.break_before = BreakValue::Page;

    let items = vec![item1, flex_item(100.0, 50.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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
// 11. Propagated break-after from last item
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_after() {
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let mut item2 = flex_item(100.0, 50.0);
    item2.break_after = BreakValue::Page;

    let items = vec![flex_item(100.0, 50.0), item2];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
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

// ---------------------------------------------------------------------------
// 12. Item with fragmentable content (nested break token)
// ---------------------------------------------------------------------------

#[test]
fn item_with_fragmentable_content() {
    // A flex item with visible overflow is NOT monolithic and could be fragmented
    // within. For now, we fragment at line boundaries only, so this test verifies
    // that line-level fragmentation works even when items are fragmentable.
    let mut container = wrap_container();
    container.width = Dimension::Length(100.0);
    let mut tall_item = flex_item(100.0, 80.0);
    tall_item.overflow_x = Overflow::Visible;
    tall_item.overflow_y = Overflow::Visible;

    let items = vec![tall_item, flex_item(100.0, 80.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        100.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    // Two lines of 80px each. First line (80px) fits in 100px.
    // Second line would push to 160px > 100px → break at line 1.
    let bt = outcome.break_token.expect("should break");
    match &bt.mode_data {
        Some(BreakTokenData::Flex { line_index, .. }) => {
            assert_eq!(*line_index, 1, "break before second line");
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 13. Column flex breaks between items along main axis
// ---------------------------------------------------------------------------

fn column_container() -> ComputedStyle {
    ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        ..Default::default()
    }
}

#[test]
fn column_flex_breaks_between_items() {
    // Column flex with 3 items, each 60px tall. Container width 200px.
    // Available block size 130px → 2 items (120px) fit, break before item 2.
    let container = column_container();
    let items = vec![
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
    ];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        200.0,
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
    let bt = outcome
        .break_token
        .expect("column flex should produce break token");
    match &bt.mode_data {
        Some(BreakTokenData::Flex {
            line_index,
            item_index,
            ..
        }) => {
            assert_eq!(*line_index, 0, "break within the first (only) line");
            assert_eq!(*item_index, 2, "break before item 2");
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 14. Column flex resume from break token
// ---------------------------------------------------------------------------

#[test]
fn column_flex_resume_from_break_token() {
    // Same setup as column_flex_breaks_between_items.
    // First pass breaks at item 2. Second pass resumes from item 2.
    let container = column_container();
    let items = vec![
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
        flex_item(100.0, 60.0),
    ];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();
    let frag = Some(FragmentainerContext {
        available_block_size: 130.0,
        fragmentation_type: FragmentationType::Column,
    });

    // First pass: should break.
    let outcome1 = do_layout_flex_fragmented(
        &mut dom,
        entity,
        200.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        frag,
        None,
    );
    let bt1 = outcome1.break_token.expect("first pass should break");

    // Second pass with break token: item 2 only (60px) fits in 130px.
    let outcome2 = do_layout_flex_fragmented(
        &mut dom,
        entity,
        200.0,
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
// 15. Column flex forced break-before on item
// ---------------------------------------------------------------------------

#[test]
fn column_flex_forced_break() {
    let container = column_container();
    let mut item2 = flex_item(100.0, 40.0);
    item2.break_before = BreakValue::Column;

    let items = vec![flex_item(100.0, 40.0), item2, flex_item(100.0, 40.0)];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        200.0,
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
        Some(BreakTokenData::Flex {
            line_index,
            item_index,
            ..
        }) => {
            assert_eq!(*line_index, 0, "break within line 0");
            assert_eq!(*item_index, 1, "forced break before item 1");
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 16. Column nowrap (single line) fragments between items
// ---------------------------------------------------------------------------

#[test]
fn column_no_wrap_breaks_within_single_line() {
    // Column nowrap: all items in one line. Unlike row nowrap (monolithic single line),
    // column flex CAN break between items since they are along the block axis.
    let container = ComputedStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_wrap: FlexWrap::Nowrap,
        ..Default::default()
    };
    let items = vec![
        flex_item(100.0, 80.0),
        flex_item(100.0, 80.0),
        flex_item(100.0, 80.0),
    ];
    let (mut dom, entity, _) = make_flex_dom(container, &items);
    let fdb = FontDatabase::new();

    let outcome = do_layout_flex_fragmented(
        &mut dom,
        entity,
        200.0,
        None,
        Point::ZERO,
        &fdb,
        0,
        layout_block_only,
        Some(FragmentainerContext {
            available_block_size: 170.0,
            fragmentation_type: FragmentationType::Column,
        }),
        None,
    );
    let bt = outcome
        .break_token
        .expect("column nowrap should still fragment between items");
    match &bt.mode_data {
        Some(BreakTokenData::Flex {
            line_index,
            item_index,
            ..
        }) => {
            assert_eq!(*line_index, 0, "single line");
            assert_eq!(
                *item_index, 2,
                "break before item 2 (160px fits, 240px doesn't)"
            );
        }
        other => panic!("expected Flex break token, got {other:?}"),
    }
}
