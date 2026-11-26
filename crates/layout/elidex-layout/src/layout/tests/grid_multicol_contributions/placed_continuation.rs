use super::*;
use elidex_layout_block::{BreakTokenData, LayoutOutcome};
use elidex_plugin::{AlignSelf, AlignmentSafety, BreakValue, JustifySelf};

thread_local! {
    static CALLS: std::cell::RefCell<Vec<(Entity, f32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn record_dispatch(dom: &mut EcsDom, child: Entity, input: &LayoutInput<'_>) -> LayoutOutcome {
    if !input.is_probe {
        let room = input
            .fragmentainer
            .expect("paged definitive child")
            .available_block_size;
        CALLS.with_borrow_mut(|calls| calls.push((child, room)));
    }
    dispatch_layout_child(dom, child, input)
}

#[test]
fn aligned_item_preserves_inline_placement_and_debits_block_entry_once() {
    let mut dom = EcsDom::new();
    let grid = element(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Grid,
            grid_template_columns: tracks(100.),
            grid_template_rows: tracks(240.),
            ..Default::default()
        },
    );
    let item = element(
        &mut dom,
        Some(grid),
        ComputedStyle {
            width: Dimension::Length(120.),
            margin_left: Dimension::Auto,
            justify_self: JustifySelf::End,
            justify_self_safety: Some(AlignmentSafety::Unsafe),
            align_self: AlignSelf::End,
            ..sized(Display::Block, 210.)
        },
    );
    let p = element(&mut dom, Some(item), sized(Display::Block, 16.));
    let q = element(
        &mut dom,
        Some(item),
        ComputedStyle {
            break_before: BreakValue::Page,
            ..sized(Display::Block, 16.)
        },
    );
    let fonts = FontDatabase::new();
    let mut saved = None;
    for replay in [false, true] {
        for first in [true, false] {
            let frag = page(if first { 125. } else { 150. });
            let generation = 1 + 2 * u32::from(replay) + u32::from(!first);
            let input = LayoutInput {
                offset: Point::new(40., 0.),
                break_token: if first { None } else { saved.as_ref() },
                layout_generation: generation,
                medium: elidex_plugin::Medium::Print,
                ..contribution_input(&fonts, &frag)
            };
            let out = elidex_layout_grid::layout_grid(&mut dom, grid, &input, record_dispatch);
            assert_eq!(CALLS.take(), vec![(item, if first { 95. } else { 150. })]);
            let lb = get_layout(&dom, item);
            let y = if first { 30. } else { 0. };
            assert_eq!(lb.content.origin, Point::new(20., y));
            assert_eq!(
                lb.content.size,
                Size::new(120., if first { 95. } else { 115. })
            );
            let (present, absent) = if first { (p, q) } else { (q, p) };
            let child = get_layout(&dom, present);
            assert_eq!(child.layout_generation, generation);
            assert_eq!(child.content.origin, Point::new(20., y));
            assert!(!dom
                .world()
                .get::<&LayoutBox>(absent)
                .is_ok_and(|b| b.layout_generation == generation));
            assert!(out.contribution.is_finished(item, present));
            assert!(!out.contribution.is_finished(item, absent));
            if first {
                let token = out.end.into_token().expect("grid continuation");
                let Some(BreakTokenData::Grid {
                    child_break_tokens, ..
                }) = &token.mode_data
                else {
                    panic!("grid route")
                };
                let [(id, nested)] = child_break_tokens.as_slice() else {
                    panic!("one unfinished item")
                };
                assert_eq!((*id, nested.entity), (item, item));
                assert_eq!(nested.specified_size_fraction, Some(95. / 210.));
                assert!(matches!(
                    nested.mode_data,
                    Some(BreakTokenData::Block { child_index: 1, .. })
                ));
                if !replay {
                    saved = Some(token);
                }
            } else {
                assert!(out.end.is_complete());
                assert!(out.contribution.is_finished(grid, item));
            }
        }
    }
}

#[test]
fn admitted_orthogonal_item_keeps_its_physical_extent() {
    use elidex_plugin::WritingMode;
    let fonts = FontDatabase::new();
    let frag = page(100.);
    for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
        let mut dom = EcsDom::new();
        let grid = element(
            &mut dom,
            None,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: tracks(210.),
                grid_template_rows: tracks(40.),
                ..Default::default()
            },
        );
        let item = element(
            &mut dom,
            Some(grid),
            ComputedStyle {
                writing_mode: mode,
                width: Dimension::Length(210.),
                ..sized(Display::Block, 40.)
            },
        );
        // Continuous control, page count, then paint from the same initial state.
        for (generation, room) in [(1, None), (2, Some(&frag)), (3, Some(&frag))] {
            let input = LayoutInput {
                fragmentainer: room,
                layout_generation: generation,
                ..contribution_input(&fonts, &frag)
            };
            let out = dispatch_layout_child(&mut dom, grid, &input);
            assert!(out.end.is_complete());
            assert!(out.contribution.is_finished(grid, item));
            let lb = get_layout(&dom, item);
            assert_eq!(lb.layout_generation, generation);
            assert_eq!(lb.content.size, Size::new(210., 40.));
        }
    }
}

#[test]
fn vertical_rows_map_to_the_current_fragment_edges() {
    use elidex_plugin::{GridTrackList, TrackSection, TrackSize, WritingMode};
    let fonts = FontDatabase::new();
    let frag = page(50.);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let mut dom = EcsDom::new();
        let grid = element(
            &mut dom,
            None,
            ComputedStyle {
                writing_mode: mode,
                display: Display::Grid,
                grid_template_columns: tracks(30.),
                grid_template_rows: GridTrackList::Explicit(TrackSection {
                    tracks: vec![TrackSize::Length(50.); 2],
                    line_names: vec![],
                }),
                ..Default::default()
            },
        );
        let items = [(); 2].map(|()| {
            element(
                &mut dom,
                Some(grid),
                ComputedStyle {
                    writing_mode: mode,
                    display: Display::Block,
                    ..Default::default()
                },
            )
        });
        let initial = contribution_input(&fonts, &frag);
        let continuous = dispatch_layout_child(
            &mut dom,
            grid,
            &LayoutInput {
                fragmentainer: None,
                ..initial
            },
        );
        assert!(continuous.end.is_complete());
        assert_eq!(continuous.layout_box.content.size.width, 100.);
        let positions = items.map(|item| get_layout(&dom, item).content.origin.x);
        assert_eq!(
            positions,
            if mode == WritingMode::VerticalRl {
                [50., 0.]
            } else {
                [0., 50.]
            }
        );
        let last_page = page(80.);
        let mut saved = None;
        for generation in [2, 4] {
            for (i, &item) in items.iter().enumerate() {
                let input = LayoutInput {
                    fragmentainer: Some(if i == 0 { &frag } else { &last_page }),
                    layout_generation: generation + u32::from(i == 1),
                    break_token: if i == 0 { None } else { saved.as_ref() },
                    ..initial
                };
                let out = dispatch_layout_child(&mut dom, grid, &input);
                assert_eq!(out.layout_box.content.size.width, 50.);
                let lb = get_layout(&dom, item);
                assert_eq!(lb.layout_generation, input.layout_generation);
                assert_eq!(lb.content.origin.x, 0.);
                assert_eq!(lb.content.size.width, 50.);
                assert!(out.contribution.is_finished(grid, item));
                assert_eq!(out.end.is_complete(), i == 1);
                if generation == 2 && i == 0 {
                    saved = out.end.into_token();
                }
            }
        }
    }
}
