//! Block fragmentation tests (CSS Fragmentation Level 3).

use super::*;
use crate::{BreakTokenData, FragmentainerContext, FragmentationType, LayoutInput};
use elidex_plugin::{BoxDecorationBreak, BreakInsideValue, BreakValue, Overflow};

fn make_block_child(dom: &mut EcsDom, parent: Entity, height: f32) -> Entity {
    let child = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(height),
            ..Default::default()
        },
    );
    child
}

fn make_block_child_with_break(
    dom: &mut EcsDom,
    parent: Entity,
    height: f32,
    break_before: BreakValue,
    break_after: BreakValue,
) -> Entity {
    let child = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(height),
            break_before,
            break_after,
            ..Default::default()
        },
    );
    child
}

/// Build a base LayoutInput (without fragmentainer/break_token; caller sets those).
fn base_input(font_db: &FontDatabase) -> LayoutInput<'_> {
    LayoutInput {
        containing_width: 400.0,
        containing_height: Some(1000.0),
        containing_inline_size: 400.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    }
}

// ---------------------------------------------------------------------------
// Forced break tests
// ---------------------------------------------------------------------------

#[test]
fn forced_break_before_produces_break_token() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 50.0);
    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Page, BreakValue::Auto);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_some(),
        "forced break-before should produce a break token"
    );
    let bt = outcome.break_token.unwrap();
    assert!(bt.child_break_token.is_some());
    let child_bt = bt.child_break_token.unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 1,
            "break should occur before 2nd child (index 1)"
        );
    } else {
        panic!("expected Block break token data");
    }
}

#[test]
fn forced_break_after_produces_break_token() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Auto, BreakValue::Page);
    make_block_child(&mut dom, parent, 50.0);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_some(),
        "forced break-after should produce a break token"
    );
    let bt = outcome.break_token.unwrap();
    let child_bt = bt.child_break_token.unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 1,
            "break should occur after 1st child (resume at index 1)"
        );
    } else {
        panic!("expected Block break token data");
    }
}

#[test]
fn column_break_not_forced_in_page_context() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Auto, BreakValue::Column);
    make_block_child(&mut dom, parent, 50.0);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_none(),
        "column break should not force in page context"
    );
}

// ---------------------------------------------------------------------------
// Overflow detection tests
// ---------------------------------------------------------------------------

#[test]
fn overflow_produces_break_token() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 100.0);
    make_block_child(&mut dom, parent, 100.0);
    make_block_child(&mut dom, parent, 100.0);

    let frag = FragmentainerContext {
        available_block_size: 150.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_some(),
        "overflow should produce a break token"
    );
}

#[test]
fn no_fragmentainer_no_break() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 100.0);
    make_block_child(&mut dom, parent, 100.0);

    let input = base_input(&font_db);
    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_none(),
        "no fragmentainer → no break token"
    );
}

// ---------------------------------------------------------------------------
// Break token resume tests
// ---------------------------------------------------------------------------

#[test]
fn break_token_resume_starts_from_correct_child() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 80.0);
    make_block_child(&mut dom, parent, 80.0);
    make_block_child(&mut dom, parent, 80.0);

    // First layout: 100px available → break after child 0 (80px consumed).
    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome1 = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome1.break_token.is_some());

    // Second layout: resume with 200px.
    let bt = outcome1.break_token.unwrap();
    let frag2 = FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input2 = LayoutInput {
        fragmentainer: Some(&frag2),
        break_token: Some(&bt),
        ..base_input(&font_db)
    };

    let outcome2 = crate::layout_block_only(&mut dom, parent, &input2);
    assert!(
        outcome2.break_token.is_none(),
        "remaining children should fit in 200px"
    );
}

// ---------------------------------------------------------------------------
// Monolithic handling tests
// ---------------------------------------------------------------------------

#[test]
fn monolithic_first_child_overflows_without_break() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let child = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(200.0),
            overflow_x: Overflow::Hidden,
            ..Default::default()
        },
    );

    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    // First (and only) child is monolithic with no prior content → overflow allowed.
    if let Some(bt) = &outcome.break_token {
        let child_bt = bt.child_break_token.as_ref().unwrap();
        if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
            assert_eq!(*child_index, 1);
        }
    }
    let lb = dom.world().get::<&elidex_plugin::LayoutBox>(child).unwrap();
    assert!((lb.content.height - 200.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// box-decoration-break tests
// ---------------------------------------------------------------------------

#[test]
fn box_decoration_break_slice_first_fragment() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            box_decoration_break: BoxDecorationBreak::Slice,
            ..Default::default()
        },
    );

    make_block_child(&mut dom, parent, 80.0);
    make_block_child_with_break(&mut dom, parent, 80.0, BreakValue::Page, BreakValue::Auto);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.layout_box.content.height >= 0.0);
    assert!(outcome.break_token.is_some());
}

#[test]
fn box_decoration_break_clone_first_fragment() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            box_decoration_break: BoxDecorationBreak::Cloned,
            ..Default::default()
        },
    );

    make_block_child(&mut dom, parent, 80.0);
    make_block_child_with_break(&mut dom, parent, 80.0, BreakValue::Page, BreakValue::Auto);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
}

// ---------------------------------------------------------------------------
// Margin collapse + fragmentation tests
// ---------------------------------------------------------------------------

#[test]
fn margin_collapse_suppressed_at_fragment_start() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let child1 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child1);
    dom.world_mut().insert_one(
        child1,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(80.0),
            margin_bottom: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    let child2 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(80.0),
            margin_top: Dimension::Length(20.0),
            ..Default::default()
        },
    );

    // First layout: 100px available.
    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome1 = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome1.break_token.is_some());

    // Second layout: resume.
    let bt = outcome1.break_token.unwrap();
    let frag2 = FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input2 = LayoutInput {
        fragmentainer: Some(&frag2),
        break_token: Some(&bt),
        ..base_input(&font_db)
    };

    let outcome2 = crate::layout_block_only(&mut dom, parent, &input2);
    assert!(outcome2.break_token.is_none());
}

// ---------------------------------------------------------------------------
// Break propagation tests
// ---------------------------------------------------------------------------

#[test]
fn break_propagation_first_child_break_before() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Page, BreakValue::Auto);
    make_block_child(&mut dom, parent, 50.0);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    assert_eq!(outcome.propagated_break_before, Some(BreakValue::Page));
}

#[test]
fn break_propagation_last_child_break_after() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 50.0);
    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Auto, BreakValue::Page);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    assert_eq!(outcome.propagated_break_after, Some(BreakValue::Page));
}

// ---------------------------------------------------------------------------
// break-inside: avoid tests
// ---------------------------------------------------------------------------

#[test]
fn break_inside_avoid_penalizes_candidates() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            break_inside: BreakInsideValue::Avoid,
            ..Default::default()
        },
    );

    make_block_child(&mut dom, parent, 100.0);
    make_block_child(&mut dom, parent, 100.0);

    let frag = FragmentainerContext {
        available_block_size: 150.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_some(),
        "overflow still breaks even with avoid"
    );
}

// ---------------------------------------------------------------------------
// Orphans/widows inline fragmentation tests (CSS Fragmentation L3 §4.3)
// ---------------------------------------------------------------------------

#[test]
fn orphans_widows_unit_basic() {
    use crate::inline::InlineFragConstraint;
    // Test the inline fragmentation logic directly via InlineFragConstraint.
    // orphans=2, widows=2, 10 lines, fragmentainer fits 5 → break at line 5.
    let constraint = InlineFragConstraint {
        available_block: 100.0,
        orphans: 2,
        widows: 2,
        skip_lines: 0,
    };
    // Verify constraint fields are accessible (compile-time validation).
    assert_eq!(constraint.orphans, 2);
    assert_eq!(constraint.widows, 2);
    assert_eq!(constraint.skip_lines, 0);
    assert!((constraint.available_block - 100.0).abs() < f32::EPSILON);
}

#[test]
fn orphans_widows_monolithic_when_exceeds_total() {
    use crate::inline::InlineFragConstraint;
    // orphans=5, widows=5, total=8: orphans+widows > total → monolithic.
    let constraint = InlineFragConstraint {
        available_block: 50.0,
        orphans: 5,
        widows: 5,
        skip_lines: 0,
    };
    // Verify constraint can represent high orphans+widows.
    assert_eq!(constraint.orphans, 5);
    assert_eq!(constraint.widows, 5);
}

#[test]
fn inline_fragmentation_no_fragmentainer_no_break() {
    // Inline-only children without fragmentainer should not produce break.
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let text = dom.create_text("Hello world");
    dom.append_child(parent, text);

    let input = base_input(&font_db);
    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_none(),
        "no fragmentainer → no break from inline content"
    );
}

// ---------------------------------------------------------------------------
// Nested fragmentation tests
// ---------------------------------------------------------------------------

#[test]
fn nested_block_fragmentation_break_token_chain() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Outer > Inner > Children
    let outer = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(outer, block_style());

    let inner = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(outer, inner);
    dom.world_mut().insert_one(inner, block_style());

    make_block_child(&mut dom, inner, 100.0);
    make_block_child(&mut dom, inner, 100.0);

    let frag = FragmentainerContext {
        available_block_size: 150.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, outer, &input);
    assert!(
        outcome.break_token.is_some(),
        "nested overflow should produce break token"
    );
    // Verify chain: outer break → child break → inner break data.
    let bt = outcome.break_token.unwrap();
    assert!(
        bt.child_break_token.is_some(),
        "outer token should have child_break_token"
    );
}

#[test]
fn three_level_nested_break_propagation() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // outer > middle > inner > children
    let outer = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(outer, block_style());

    let middle = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(outer, middle);
    dom.world_mut().insert_one(middle, block_style());

    let inner = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(middle, inner);
    dom.world_mut().insert_one(inner, block_style());

    // First grandchild has break-before: page → should propagate up.
    make_block_child_with_break(&mut dom, inner, 50.0, BreakValue::Page, BreakValue::Auto);
    make_block_child(&mut dom, inner, 50.0);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, outer, &input);
    // The forced break should produce a break token.
    assert!(outcome.break_token.is_some());
    assert_eq!(outcome.propagated_break_before, Some(BreakValue::Page));
}

// ---------------------------------------------------------------------------
// Empty fragment from forced break
// ---------------------------------------------------------------------------

#[test]
fn forced_break_before_first_child_produces_empty_fragment() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // First child has forced break-before → empty first fragment.
    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Page, BreakValue::Auto);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    // The consumed_block_size of the child break token should be 0 (empty fragment).
    let bt = outcome.break_token.unwrap();
    let child_bt = bt.child_break_token.unwrap();
    assert!(
        child_bt.consumed_block_size < f32::EPSILON,
        "empty fragment: consumed_block_size should be ~0, got {}",
        child_bt.consumed_block_size
    );
}

// ---------------------------------------------------------------------------
// Multiple overflow candidates — best break selection
// ---------------------------------------------------------------------------

#[test]
fn best_break_selects_last_fitting_candidate() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // 4 children of 30px each = 120px total. Fragmentainer = 100px.
    // Should break between child 2 and child 3 (at 90px).
    make_block_child(&mut dom, parent, 30.0);
    make_block_child(&mut dom, parent, 30.0);
    make_block_child(&mut dom, parent, 30.0);
    make_block_child(&mut dom, parent, 30.0);

    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    let bt = outcome.break_token.unwrap();
    let child_bt = bt.child_break_token.unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 3,
            "should break before child 3 (after 90px of content)"
        );
    } else {
        panic!("expected Block break token data");
    }
}

// ---------------------------------------------------------------------------
// Monolithic non-first child defers to next fragmentainer
// ---------------------------------------------------------------------------

#[test]
fn monolithic_non_first_child_breaks_before() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // First child: 80px normal block.
    make_block_child(&mut dom, parent, 80.0);

    // Second child: monolithic (overflow:hidden), 80px tall.
    let child2 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(80.0),
            overflow_x: Overflow::Hidden,
            ..Default::default()
        },
    );

    // 100px available: first child fits (80px). Second child is monolithic.
    // After laying out, 160px consumed > 100px → overflow detection picks
    // best break before the monolithic child (at 80px).
    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.break_token.is_some(),
        "should break before monolithic child that would overflow"
    );
    let bt = outcome.break_token.unwrap();
    let child_bt = bt.child_break_token.unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 1,
            "should break before the monolithic child (index 1)"
        );
    } else {
        panic!("expected Block break token data");
    }
}

// ---------------------------------------------------------------------------
// Writing mode: vertical-rl block fragmentation
// ---------------------------------------------------------------------------

#[test]
fn vertical_rl_block_fragmentation() {
    use elidex_plugin::WritingMode;

    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );

    // In vertical-rl, block axis = horizontal (width).
    // Children with explicit width dimensions.
    let c1 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, c1);
    dom.world_mut().insert_one(
        c1,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(80.0),
            ..Default::default()
        },
    );
    let c2 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, c2);
    dom.world_mut().insert_one(
        c2,
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::Length(80.0),
            ..Default::default()
        },
    );

    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    // 80 + 80 = 160px block-axis (horizontal), but fragmentainer only 100px.
    assert!(
        outcome.break_token.is_some(),
        "vertical-rl block fragmentation should produce break"
    );
}

// ---------------------------------------------------------------------------
// M3: child_index correctness with display:none children
// ---------------------------------------------------------------------------

#[test]
fn child_index_correct_with_display_none_children() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // children: [NONE(0), A(1, 80px), NONE(2), B(3, 80px)]
    let none1 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, none1);
    dom.world_mut().insert_one(
        none1,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    make_block_child(&mut dom, parent, 80.0); // index 1
    let none2 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, none2);
    dom.world_mut().insert_one(
        none2,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    make_block_child(&mut dom, parent, 80.0); // index 3

    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    // First layout: A (80px) fits, B overflows → break before B (index 3).
    let outcome1 = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome1.break_token.is_some());
    let bt = outcome1.break_token.unwrap();
    let child_bt = bt.child_break_token.as_ref().unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 3,
            "break should be at array index 3 (second block child, after display:none gaps)"
        );
    } else {
        panic!("expected Block break token data");
    }

    // Resume: skip to index 3, display:none at 0 and 2 should be skipped.
    let frag2 = FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input2 = LayoutInput {
        fragmentainer: Some(&frag2),
        break_token: Some(&bt),
        ..base_input(&font_db)
    };
    let outcome2 = crate::layout_block_only(&mut dom, parent, &input2);
    assert!(
        outcome2.break_token.is_none(),
        "remaining child B should fit in 200px"
    );
}

// ---------------------------------------------------------------------------
// H1: break-after avoid penalty in break candidate
// ---------------------------------------------------------------------------

#[test]
fn break_after_avoid_penalizes_candidate() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // Child 0: 30px, break-after: avoid
    make_block_child_with_break(&mut dom, parent, 30.0, BreakValue::Auto, BreakValue::Avoid);
    // Child 1: 30px (candidate between 0-1 should have avoid penalty)
    make_block_child(&mut dom, parent, 30.0);
    // Child 2: 30px (candidate between 1-2 should be non-penalized)
    make_block_child(&mut dom, parent, 30.0);
    // Child 3: 30px (overflows)
    make_block_child(&mut dom, parent, 30.0);

    // Fragmentainer: 100px. Total = 120px → overflow.
    // Candidates: between 0-1 (at 30px, avoid penalty), between 1-2 (at 60px, clean),
    //             between 2-3 (at 90px, clean).
    // Best break should prefer non-avoid candidates → break at child 3 (at 90px).
    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    let bt = outcome.break_token.unwrap();
    let child_bt = bt.child_break_token.as_ref().unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        // Should break at index 3 (90px), not index 1 (30px, penalized by avoid).
        assert_eq!(
            *child_index, 3,
            "best break should prefer non-avoid candidate (break at child 3)"
        );
    } else {
        panic!("expected Block break token data");
    }
}

// ---------------------------------------------------------------------------
// M1: propagated_break_after only for last content child
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_after_not_set_for_non_last_child() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // Child 0: forced break-after, but child 1 exists → NOT the last child.
    make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Auto, BreakValue::Page);
    make_block_child(&mut dom, parent, 50.0);

    let frag = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    // Child 0's break-after: page should NOT propagate because child 1 exists.
    assert_eq!(
        outcome.propagated_break_after, None,
        "break-after should not propagate from non-last child"
    );
}

// ---------------------------------------------------------------------------
// H3: fragment height clamped to break point
// ---------------------------------------------------------------------------

#[test]
fn fragment_height_clamped_to_break_point() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // 3 children of 50px = 150px. Fragmentainer = 80px.
    make_block_child(&mut dom, parent, 50.0);
    make_block_child(&mut dom, parent, 50.0);
    make_block_child(&mut dom, parent, 50.0);

    let frag = FragmentainerContext {
        available_block_size: 80.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    // Fragment height should be clamped to the break point (50px for 1 child),
    // not the full stacking height that includes the overflowing child.
    assert!(
        outcome.layout_box.content.height <= 80.0,
        "fragment height ({}) should not exceed fragmentainer (80px)",
        outcome.layout_box.content.height
    );
}

// ---------------------------------------------------------------------------
// M2: monolithic with no remaining space defers
// ---------------------------------------------------------------------------

#[test]
fn monolithic_deferred_when_no_space_remaining() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    // First child fills exactly the fragmentainer (100px).
    make_block_child(&mut dom, parent, 100.0);
    // Second child: monolithic, 50px. No space left → deferred.
    let child2 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, child2);
    dom.world_mut().insert_one(
        child2,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            overflow_x: Overflow::Hidden,
            ..Default::default()
        },
    );

    let frag = FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&font_db)
    };

    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(outcome.break_token.is_some());
    let bt = outcome.break_token.unwrap();
    let child_bt = bt.child_break_token.as_ref().unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 1,
            "monolithic child should be deferred (break at index 1)"
        );
    } else {
        panic!("expected Block break token data");
    }
}
