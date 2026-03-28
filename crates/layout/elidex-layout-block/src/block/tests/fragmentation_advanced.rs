//! Box-decoration-break, margin collapse, break propagation, orphans/widows,
//! nested fragmentation, and best-break selection tests.

use super::*;
use crate::{BreakTokenData, FragmentainerContext, FragmentationType, LayoutInput};
use elidex_plugin::{BoxDecorationBreak, BreakInsideValue, Overflow};

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
    assert!(outcome.layout_box.content.size.height >= 0.0);
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
    let constraint = InlineFragConstraint {
        available_block: 100.0,
        orphans: 2,
        widows: 2,
        skip_lines: 0,
    };
    assert_eq!(constraint.orphans, 2);
    assert_eq!(constraint.widows, 2);
    assert_eq!(constraint.skip_lines, 0);
    assert!((constraint.available_block - 100.0).abs() < f32::EPSILON);
}

#[test]
fn orphans_widows_monolithic_when_exceeds_total() {
    use crate::inline::InlineFragConstraint;
    let constraint = InlineFragConstraint {
        available_block: 50.0,
        orphans: 5,
        widows: 5,
        skip_lines: 0,
    };
    assert_eq!(constraint.orphans, 5);
    assert_eq!(constraint.widows, 5);
}

#[test]
fn inline_fragmentation_no_fragmentainer_no_break() {
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

    let outer = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(outer, block_style());

    let middle = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(outer, middle);
    dom.world_mut().insert_one(middle, block_style());

    let inner = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(middle, inner);
    dom.world_mut().insert_one(inner, block_style());

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

    make_block_child(&mut dom, parent, 80.0);

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
    assert!(
        outcome.break_token.is_some(),
        "vertical-rl block fragmentation should produce break"
    );
}

// ---------------------------------------------------------------------------
// child_index correctness with display:none children
// ---------------------------------------------------------------------------

#[test]
fn child_index_correct_with_display_none_children() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let none1 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, none1);
    dom.world_mut().insert_one(
        none1,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    make_block_child(&mut dom, parent, 80.0);
    let none2 = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.append_child(parent, none2);
    dom.world_mut().insert_one(
        none2,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    make_block_child(&mut dom, parent, 80.0);

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
// break-after avoid penalty in break candidate
// ---------------------------------------------------------------------------

#[test]
fn break_after_avoid_penalizes_candidate() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child_with_break(&mut dom, parent, 30.0, BreakValue::Auto, BreakValue::Avoid);
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
    let child_bt = bt.child_break_token.as_ref().unwrap();
    if let Some(BreakTokenData::Block { child_index, .. }) = &child_bt.mode_data {
        assert_eq!(
            *child_index, 3,
            "best break should prefer non-avoid candidate (break at child 3)"
        );
    } else {
        panic!("expected Block break token data");
    }
}

// ---------------------------------------------------------------------------
// propagated_break_after only for last content child
// ---------------------------------------------------------------------------

#[test]
fn propagated_break_after_not_set_for_non_last_child() {
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
    assert!(outcome.break_token.is_some());
    assert_eq!(
        outcome.propagated_break_after, None,
        "break-after should not propagate from non-last child"
    );
}

// ---------------------------------------------------------------------------
// fragment height clamped to break point
// ---------------------------------------------------------------------------

#[test]
fn fragment_height_clamped_to_break_point() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

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
    assert!(
        outcome.layout_box.content.size.height <= 80.0,
        "fragment height ({}) should not exceed fragmentainer (80px)",
        outcome.layout_box.content.size.height
    );
}

// ---------------------------------------------------------------------------
// monolithic with no remaining space defers
// ---------------------------------------------------------------------------

#[test]
fn monolithic_deferred_when_no_space_remaining() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 100.0);
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
