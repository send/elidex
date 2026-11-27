use super::*;
use elidex_plugin::{AlignSelf, JustifySelf};

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

#[test]
fn block_preserves_an_orthogonal_child_on_a_fitting_page() {
    use elidex_plugin::WritingMode;
    let fonts = FontDatabase::new();
    let frag = page(100.);
    let mut dom = EcsDom::new();
    let parent = element(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(210.),
            ..Default::default()
        },
    );
    let child = element(
        &mut dom,
        Some(parent),
        ComputedStyle {
            writing_mode: WritingMode::VerticalLr,
            width: Dimension::Length(210.),
            ..sized(Display::Block, 40.)
        },
    );
    for (generation, room) in [(1, None), (2, Some(&frag)), (3, Some(&frag))] {
        let input = LayoutInput {
            layout_generation: generation,
            fragmentainer: room,
            ..contribution_input(&fonts, &frag)
        };
        let out = dispatch_layout_child(&mut dom, parent, &input);
        assert!(
            out.end.is_complete(),
            "{out:?} child {:?}",
            get_layout(&dom, child)
        );
        assert!(out.contribution.is_finished(parent, child));
        assert_eq!(get_layout(&dom, child).content.size, Size::new(210., 40.));
    }
}

#[test]
fn reversed_grid_publishes_descendants_against_the_absolute_scroll_edge() {
    use elidex_plugin::WritingMode;
    let fonts = FontDatabase::new();
    let frag = page(100.);
    let mut dom = EcsDom::new();
    let outer = element(
        &mut dom,
        None,
        ComputedStyle {
            writing_mode: WritingMode::VerticalRl,
            display: Display::Grid,
            width: Dimension::Length(50.),
            grid_template_rows: tracks(50.),
            grid_template_columns: tracks(30.),
            ..Default::default()
        },
    );
    let inner = element(
        &mut dom,
        Some(outer),
        ComputedStyle {
            width: Dimension::Length(50.),
            display: Display::Grid,
            grid_template_columns: tracks(50.),
            ..Default::default()
        },
    );
    let leaf = element(
        &mut dom,
        Some(inner),
        ComputedStyle {
            justify_self: JustifySelf::End,
            width: Dimension::Length(100.),
            ..sized(Display::Block, 10.)
        },
    );
    for room in [None, Some(&frag)] {
        let mut input = contribution_input(&fonts, &frag);
        input.fragmentainer = room;
        input.scroll_limits.x = Some(elidex_layout_block::ScrollEdge::Low(0.));
        let out = dispatch_layout_child(&mut dom, outer, &input);
        assert!(out.end.is_complete());
        assert_eq!(get_layout(&dom, inner).content.origin.x, 0.);
        assert_eq!(get_layout(&dom, leaf).content.origin.x, 0.);
    }
}

#[test]
fn reversed_grid_keeps_pixel_precision_in_a_large_fragmentainer() {
    use elidex_plugin::WritingMode;
    let fonts = FontDatabase::new();
    let frag = page(268_435_456.);
    let mut dom = EcsDom::new();
    let grid = element(
        &mut dom,
        None,
        ComputedStyle {
            writing_mode: WritingMode::VerticalRl,
            display: Display::Grid,
            width: Dimension::Length(50.),
            grid_template_rows: tracks(50.),
            grid_template_columns: tracks(30.),
            ..Default::default()
        },
    );
    let leaf = element(
        &mut dom,
        Some(grid),
        ComputedStyle {
            align_self: AlignSelf::FlexStart,
            width: Dimension::Length(10.),
            ..sized(Display::Block, 10.)
        },
    );
    for room in [None, Some(&frag)] {
        let input = LayoutInput {
            fragmentainer: room,
            ..contribution_input(&fonts, &frag)
        };
        let out = dispatch_layout_child(&mut dom, grid, &input);
        assert!(out.end.is_complete());
        assert_eq!(get_layout(&dom, leaf).content.origin.x, 40.);
    }
}

#[test]
fn reversed_grid_breaks_after_the_whole_oversized_row() {
    use elidex_plugin::WritingMode;
    let fonts = FontDatabase::new();
    let frag = page(100.);
    let mut dom = EcsDom::new();
    let grid = element(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: WritingMode::VerticalRl,
            grid_template_rows: elidex_plugin::GridTrackList::Explicit(
                elidex_plugin::TrackSection {
                    tracks: vec![
                        elidex_plugin::TrackSize::Length(240.),
                        elidex_plugin::TrackSize::Length(20.),
                    ],
                    line_names: vec![],
                },
            ),
            grid_template_columns: tracks(30.),
            ..Default::default()
        },
    );
    let item = element(
        &mut dom,
        Some(grid),
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            align_self: AlignSelf::Stretch,
            ..Default::default()
        },
    );
    let next = element(
        &mut dom,
        Some(grid),
        ComputedStyle {
            display: Display::Block,
            writing_mode: WritingMode::VerticalRl,
            ..Default::default()
        },
    );
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
    assert_eq!(continuous.layout_box.content.size.width, 260.);
    assert_eq!(get_layout(&dom, item).content.origin.x, 20.);
    let first = dispatch_layout_child(&mut dom, grid, &initial);
    assert_eq!(first.layout_box.content.size.width, 240.);
    assert_eq!(get_layout(&dom, item).content.origin.x, 0.);
    assert_eq!(get_layout(&dom, item).content.size.width, 240.);
    assert!(first.contribution.is_finished(grid, item));
    assert!(!first.contribution.is_finished(grid, next));
    let token = first.end.into_token().unwrap();
    assert_eq!(token.consumed_block_size, 240.);
    let last = dispatch_layout_child(
        &mut dom,
        grid,
        &LayoutInput {
            break_token: Some(&token),
            ..initial
        },
    );
    assert!(last.end.is_complete());
    assert_eq!(last.layout_box.content.size.width, 20.);
    assert_eq!(get_layout(&dom, next).content.origin.x, 0.);
    assert!(last.contribution.is_finished(grid, next));
}
