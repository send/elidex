//! Forced breaks, overflow detection, monolithic handling, and break token resume tests.

use super::*;
use crate::{BreakTokenData, FragmentainerContext, FragmentationType, LayoutInput};
use elidex_plugin::Overflow;

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
    assert!((lb.content.size.height - 200.0).abs() < 0.01);
}
