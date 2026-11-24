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
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let admitted = make_block_child(&mut dom, parent, 0.0);
    let deferred =
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
        outcome.end.token().is_some(),
        "forced break-before should produce a break token"
    );
    assert!(outcome.contribution.is_finished(parent, admitted));
    assert!(!outcome.contribution.is_finished(parent, deferred));
    let bt = outcome.end.into_token().unwrap();

    if let Some(BreakTokenData::Block { child_index, .. }) = &bt.mode_data {
        assert_eq!(
            *child_index, 1,
            "break should occur before 2nd child (index 1)"
        );
    } else {
        panic!("expected Block break token data");
    }
    for (generation, width) in [(2, 120.0), (3, 260.0), (4, 120.0), (5, 260.0)] {
        let resumed = crate::layout_block_only(
            &mut dom,
            parent,
            &LayoutInput {
                layout_generation: generation,
                containing_inline_size: width,
                containing: elidex_plugin::CssSize::definite(width, 500.0),
                break_token: Some(&bt),
                ..input
            },
        );
        assert!(resumed.end.is_complete());
        assert_eq!(
            resumed.contribution.contributed_items().collect::<Vec<_>>(),
            [(parent, deferred)]
        );
        let child_box = dom
            .world()
            .get::<&elidex_plugin::LayoutBox>(deferred)
            .unwrap();
        assert_eq!(child_box.content.origin, input.offset);
        assert_eq!(child_box.content.size.width, width);
    }
}

#[test]
fn forced_break_after_produces_break_token() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let admitted =
        make_block_child_with_break(&mut dom, parent, 50.0, BreakValue::Auto, BreakValue::Page);
    let hidden = make_block_child(&mut dom, parent, 0.0);
    dom.world_mut().insert_one(
        hidden,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    let deferred =
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
        outcome.end.token().is_some(),
        "forced break-after should produce a break token"
    );
    assert!(outcome.contribution.is_finished(parent, admitted));
    assert!(!outcome.contribution.is_finished(parent, deferred));
    let bt = outcome.end.into_token().unwrap();

    if let Some(BreakTokenData::Block { child_index, .. }) = &bt.mode_data {
        assert_eq!(*child_index, 2, "resume at the first eligible child");
    } else {
        panic!("expected Block break token data");
    }
    let next = crate::layout_block_only(
        &mut dom,
        parent,
        &LayoutInput {
            break_token: Some(&bt),
            ..input
        },
    );
    assert!(next.end.is_complete());
    assert!(next.contribution.is_finished(parent, deferred));
}

#[test]
fn column_break_not_forced_in_page_context() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
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
        outcome.end.is_complete(),
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
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
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
        outcome.end.token().is_some(),
        "overflow should produce a break token"
    );
}

#[test]
fn no_fragmentainer_no_break() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    make_block_child(&mut dom, parent, 100.0);
    make_block_child(&mut dom, parent, 100.0);

    let input = base_input(&font_db);
    let outcome = crate::layout_block_only(&mut dom, parent, &input);
    assert!(
        outcome.end.is_complete(),
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
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
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
    assert!(outcome1.end.token().is_some());

    let bt = outcome1.end.into_token().unwrap();
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
        outcome2.end.is_complete(),
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
    let parent = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
    dom.world_mut().insert_one(parent, block_style());

    let child = dom.create_element_ownerless_fixture("div", elidex_ecs::Attributes::default());
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
    assert!(outcome.contribution.is_finished(parent, child));
    // First (and only) child is monolithic with no prior content → overflow allowed.
    if let Some(bt) = outcome.end.token() {
        if let Some(BreakTokenData::Block { child_index, .. }) = &bt.mode_data {
            assert_eq!(*child_index, 1);
        }
    }
    let lb = dom.world().get::<&elidex_plugin::LayoutBox>(child).unwrap();
    assert!((lb.content.size.height - 200.0).abs() < 0.01);
}

#[test]
fn hidden_inline_descendant_leaves_only_its_wrapper_terminal() {
    let fonts = FontDatabase::new();
    let (mut dom, owner) = make_dom_with_block_div(block_style());
    let wrapper = make_block_child(&mut dom, owner, 0.0);
    dom.world_mut().insert_one(
        wrapper,
        ComputedStyle {
            display: Display::Inline,
            ..Default::default()
        },
    );
    let hidden = make_block_child(&mut dom, wrapper, 0.0);
    dom.world_mut().insert_one(
        hidden,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    let deferred =
        make_block_child_with_break(&mut dom, owner, 10.0, BreakValue::Page, BreakValue::Auto);
    let frag = FragmentainerContext {
        available_block_size: 50.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&fonts)
    };
    let first = crate::layout_block_only(&mut dom, owner, &input);
    assert!(first.contribution.is_finished(owner, wrapper));
    assert!(!first
        .contribution
        .contributed_items()
        .any(|(_, id)| id == hidden));
    let token = first.end.into_token().unwrap();
    let next = crate::layout_block_only(
        &mut dom,
        owner,
        &LayoutInput {
            break_token: Some(&token),
            ..input
        },
    );
    assert!(next.contribution.is_finished(owner, deferred));
    assert!(next.end.is_complete());
}

#[test]
fn specified_block_size_counts_filled_fragment_room_through_replay() {
    let fonts = FontDatabase::new();
    for (rooms, heights, suffix_height) in [
        (vec![95.0, 120.0], vec![95.0, 115.0], 16.0),
        (vec![95.0, 160.0], vec![95.0, 130.0], 130.0),
        (
            vec![95.0, 60.0, 30.0, 30.0],
            vec![95.0, 60.0, 30.0, 25.0],
            16.0,
        ),
    ] {
        let (mut dom, owner) = make_dom_with_block_div(ComputedStyle {
            height: Dimension::Length(210.0),
            ..block_style()
        });
        make_block_child_with_break(&mut dom, owner, 16.0, BreakValue::Auto, BreakValue::Page);
        make_block_child(&mut dom, owner, suffix_height);
        let mut saved = Vec::new();
        for replay in [false, true] {
            let mut carry = None;
            let mut total = 0.0;
            for (index, (&room, &expected)) in rooms.iter().zip(&heights).enumerate() {
                let frag = FragmentainerContext {
                    available_block_size: room,
                    fragmentation_type: FragmentationType::Page,
                };
                if !replay {
                    saved.push(carry.clone());
                }
                let input = LayoutInput {
                    break_token: saved[index].as_ref(),
                    fragmentainer: Some(&frag),
                    ..base_input(&fonts)
                };
                let outcome = crate::layout_block_only(&mut dom, owner, &input);
                assert_eq!(outcome.layout_box.content.size.height, expected);
                total += expected;
                carry = outcome.end.into_token();
                if index + 1 == rooms.len() {
                    assert!(carry.is_none());
                } else {
                    let token = carry.as_ref().unwrap();
                    assert!(
                        (token.specified_size_fraction.unwrap() - f64::from(total) / 210.0).abs()
                            < 1e-10
                    );
                    assert!(
                        matches!(&token.mode_data, Some(BreakTokenData::Block { child_index, .. }) if *child_index == if index == 0 { 1 } else { 2 })
                    );
                }
            }
        }
    }
}

#[test]
fn interrupted_child_uses_room_left_by_preceding_sibling() {
    let fonts = FontDatabase::new();
    let (mut dom, owner) = make_dom_with_block_div(block_style());
    make_block_child(&mut dom, owner, 20.0);
    let split = make_block_child(&mut dom, owner, 210.0);
    make_block_child_with_break(&mut dom, split, 16.0, BreakValue::Auto, BreakValue::Page);
    make_block_child(&mut dom, split, 16.0);
    let frag = FragmentainerContext {
        available_block_size: 95.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        fragmentainer: Some(&frag),
        ..base_input(&fonts)
    };
    let first = crate::layout_block_only(&mut dom, owner, &input);
    assert!(first.end.token().is_some());
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::LayoutBox>(split)
            .unwrap()
            .content
            .size
            .height,
        75.0
    );
}
