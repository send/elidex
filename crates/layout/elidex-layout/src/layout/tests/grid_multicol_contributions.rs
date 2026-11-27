mod placed_continuation;

use super::source_contributions::contribution_input;
use super::*;

#[test]
fn positive_room_for_an_unstarted_size_only_child_respects_class_a_avoid() {
    use elidex_layout_block::{
        BlockContinuation, BreakTokenData, FragmentainerContext, FragmentationType,
    };
    use elidex_plugin::BreakValue;
    let fonts = FontDatabase::new();
    let frag = FragmentainerContext {
        available_block_size: 15.0,
        fragmentation_type: FragmentationType::Page,
    };
    for avoid in [false, true] {
        let mut dom = EcsDom::new();
        let owner = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(
            owner,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let items = [10.0, 30.0].map(|height| {
            let child = dom.create_element_ownerless_fixture("div", Attributes::default());
            dom.append_child(owner, child);
            dom.world_mut().insert_one(
                child,
                ComputedStyle {
                    display: Display::Block,
                    height: Dimension::Length(height),
                    ..Default::default()
                },
            );
            child
        });
        if avoid {
            dom.world_mut()
                .get::<&mut ComputedStyle>(items[1])
                .unwrap()
                .break_before = BreakValue::Avoid;
        }
        let input = contribution_input(&fonts, &frag);
        let outcome = dispatch_layout_child(&mut dom, owner, &input);
        let token = outcome.end.into_token().unwrap();
        let Some(BreakTokenData::Block {
            child_index: 1,
            continuation,
            ..
        }) = token.mode_data
        else {
            panic!("expected second child route")
        };
        assert!(outcome.contribution.is_finished(owner, items[0]));
        if avoid {
            let BlockContinuation::InterruptedChild(child) = continuation else {
                panic!("only an avoided class A boundary")
            };
            assert_eq!(child.entity, items[1]);
            assert_eq!(child.specified_size_fraction, Some(5.0 / 30.0));
            assert_eq!(
                dom.world()
                    .get::<&elidex_plugin::LayoutBox>(items[1])
                    .unwrap()
                    .content
                    .size
                    .height,
                5.0
            );
        } else {
            assert!(matches!(continuation, BlockContinuation::ChildBoundary));
            assert!(!dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(items[1])
                .is_ok_and(|lb| lb.layout_generation == input.layout_generation));
        }
    }
}

#[test]
fn multicol_finishes_geometry_only_fragments_of_an_empty_grid() {
    let mut dom = EcsDom::new();
    let multi = dom.create_element_ownerless_fixture("div", Attributes::default());
    let grid = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.append_child(multi, grid);
    dom.world_mut().insert_one(
        multi,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            column_count: Some(2),
            column_fill: elidex_plugin::ColumnFill::Auto,
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        grid,
        ComputedStyle {
            display: Display::Grid,
            height: Dimension::Length(210.0),
            ..Default::default()
        },
    );
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let outcome = dispatch_layout_child(&mut dom, multi, &contribution_input(&fonts, &frag));
    assert!(outcome.end.is_complete());
    assert!(outcome.contribution.is_finished(multi, grid));
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::MulticolInfo>(multi)
            .unwrap()
            .segments[0]
            .0,
        6
    );
}

#[test]
fn multicol_admits_grid_rows_before_judging_progress() {
    use elidex_plugin::{ColumnFill, GridTrackList, TrackSection, TrackSize};
    let mut dom = EcsDom::new();
    let mut node = |parent, style| {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(id, style);
        if let Some(parent) = parent {
            dom.append_child(parent, id);
        }
        id
    };
    let block = ComputedStyle {
        display: Display::Block,
        ..Default::default()
    };
    let owner = node(
        None,
        ComputedStyle {
            height: Dimension::Length(100.0),
            column_count: Some(2),
            column_fill: ColumnFill::Auto,
            ..block.clone()
        },
    );
    let grid = node(
        Some(owner),
        ComputedStyle {
            display: Display::Grid,
            grid_template_columns: GridTrackList::Explicit(TrackSection {
                tracks: vec![TrackSize::Fr(1.0)],
                line_names: Vec::new(),
            }),
            ..Default::default()
        },
    );
    let items: Vec<_> = (0..3)
        .map(|_| {
            node(
                Some(grid),
                ComputedStyle {
                    height: Dimension::Length(60.0),
                    ..block.clone()
                },
            )
        })
        .collect();
    let sibling = node(Some(owner), block);
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Column,
    };
    let input = elidex_layout_block::LayoutInput {
        fragmentainer: None,
        ..contribution_input(&fonts, &frag)
    };
    let outcome = dispatch_layout_child(&mut dom, owner, &input);
    assert!(
        outcome.end.is_complete(),
        "column flow ended as {:?}",
        outcome.end
    );
    for item in items {
        assert!(outcome.contribution.is_finished(grid, item));
    }
    assert!(outcome.contribution.is_finished(owner, grid));
    assert!(outcome.contribution.is_finished(owner, sibling));
}

fn element(dom: &mut EcsDom, parent: Option<Entity>, style: ComputedStyle) -> Entity {
    let id = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(id, style);
    if let Some(parent) = parent {
        dom.append_child(parent, id);
    }
    id
}

fn sized(display: Display, height: f32) -> ComputedStyle {
    ComputedStyle {
        display,
        height: Dimension::Length(height),
        ..Default::default()
    }
}

fn tracks(size: f32) -> elidex_plugin::GridTrackList {
    elidex_plugin::GridTrackList::Explicit(elidex_plugin::TrackSection {
        tracks: vec![elidex_plugin::TrackSize::Length(size)],
        line_names: vec![],
    })
}

fn page(room: f32) -> elidex_layout_block::FragmentainerContext {
    elidex_layout_block::FragmentainerContext {
        available_block_size: room,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    }
}

#[test]
fn empty_child_after_clean_column_boundary_finishes() {
    let fonts = FontDatabase::new();
    for (room, height) in [(40.0, 95.0), (15.0, 30.0)] {
        let mut dom = EcsDom::new();
        let owner = element(
            &mut dom,
            None,
            ComputedStyle {
                column_count: Some(2),
                column_fill: elidex_plugin::ColumnFill::Auto,
                ..sized(Display::Block, room)
            },
        );
        let children =
            [10.0, height, 1.0].map(|h| element(&mut dom, Some(owner), sized(Display::Block, h)));
        let frag = page(room);
        let outcome = dispatch_layout_child(&mut dom, owner, &contribution_input(&fonts, &frag));
        assert!(outcome.end.is_complete());
        for child in children {
            assert!(outcome.contribution.is_finished(owner, child));
        }
    }
}

#[test]
fn whole_grid_keeps_ordinary_size_and_sibling_position() {
    let fonts = FontDatabase::new();
    let frag = page(100.0);
    for fragmented in [false, true] {
        let mut dom = EcsDom::new();
        let owner = element(
            &mut dom,
            None,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let grid = element(
            &mut dom,
            Some(owner),
            ComputedStyle {
                grid_template_rows: tracks(60.0),
                ..sized(Display::Grid, 20.0)
            },
        );
        element(&mut dom, Some(grid), sized(Display::Block, 10.0));
        let sibling = element(&mut dom, Some(owner), sized(Display::Block, 1.0));
        let input = elidex_layout_block::LayoutInput {
            fragmentainer: fragmented.then_some(&frag),
            ..contribution_input(&fonts, &frag)
        };
        assert!(dispatch_layout_child(&mut dom, owner, &input)
            .end
            .is_complete());
        assert_eq!(
            dom.world()
                .get::<&elidex_plugin::LayoutBox>(grid)
                .unwrap()
                .content
                .size
                .height,
            20.0
        );
        assert_eq!(
            dom.world()
                .get::<&elidex_plugin::LayoutBox>(sibling)
                .unwrap()
                .content
                .origin
                .y,
            20.0
        );
    }
}

#[test]
fn auto_vertical_grid_has_no_specified_size_obligation() {
    let mut dom = EcsDom::new();
    let grid = element(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            ..Default::default()
        },
    );
    let fonts = FontDatabase::new();
    let frag = page(40.0);
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(300.0, 300.0),
        ..contribution_input(&fonts, &frag)
    };
    assert!(dispatch_layout_child(&mut dom, grid, &input)
        .end
        .is_complete());
}

#[test]
fn row_subgrid_clamps_item_to_inherited_tracks_in_both_flows() {
    use elidex_plugin::{GridLine, GridTrackList};
    let fonts = FontDatabase::new();
    let frag = page(100.0);
    for fragmented in [false, true] {
        let mut dom = EcsDom::new();
        let grid = element(
            &mut dom,
            None,
            ComputedStyle {
                display: Display::Grid,
                grid_template_rows: GridTrackList::Subgrid { line_names: vec![] },
                ..Default::default()
            },
        );
        let child = element(
            &mut dom,
            Some(grid),
            ComputedStyle {
                grid_row_start: GridLine::Line(2),
                grid_row_end: GridLine::Line(3),
                ..sized(Display::Block, 10.0)
            },
        );
        let inherited = elidex_layout_block::SubgridContext {
            row_sizes: Some(vec![30.0]),
            col_sizes: None,
            col_line_names: vec![],
            row_line_names: vec![],
            col_gap: None,
            row_gap: None,
        };
        let input = elidex_layout_block::LayoutInput {
            subgrid: Some(&inherited),
            fragmentainer: fragmented.then_some(&frag),
            ..contribution_input(&fonts, &frag)
        };
        let outcome = dispatch_layout_child(&mut dom, grid, &input);
        assert!(outcome.end.is_complete());
        assert!(outcome.contribution.is_finished(grid, child));
        assert_eq!(
            dom.world()
                .get::<&elidex_plugin::LayoutBox>(child)
                .unwrap()
                .content
                .origin
                .y,
            0.0
        );
    }
}

#[test]
fn cloned_edges_cannot_repeat_the_same_page_forever() {
    let fonts = FontDatabase::new();
    for display in [Display::Block, Display::Grid] {
        let mut dom = EcsDom::new();
        let mut style = sized(display, 100.0);
        style.box_decoration_break = elidex_plugin::BoxDecorationBreak::Cloned;
        style.padding.top = Dimension::Length(20.0);
        style.padding.bottom = Dimension::Length(20.0);
        let root = element(&mut dom, None, style);
        let frag = page(40.0);
        let fragments = layout_fragmented(&mut dom, root, &contribution_input(&fonts, &frag), frag);
        assert_eq!(fragments.len(), 5);
        assert!(fragments.last().unwrap().end.is_complete());
        for fragment in fragments {
            assert_eq!(fragment.layout_box.content.size.height, 20.0);
            assert_eq!(fragment.layout_box.padding.bottom, 0.0);
        }
        let zero = page(0.0);
        let stalled = layout_fragmented(&mut dom, root, &contribution_input(&fonts, &zero), zero);
        assert_eq!(stalled.len(), 1);
        assert!(matches!(
            stalled[0].end,
            elidex_layout_block::FragmentEnd::Incomplete
        ));
    }
}

#[test]
fn pages_advance_across_empty_grid_rows_and_new_size_only_children() {
    use elidex_plugin::{AlignSelf, GridLine, GridTrackList, TrackSection, TrackSize};
    let fonts = FontDatabase::new();
    let frag = page(40.0);
    for empty_first_row in [true, false] {
        let mut dom = EcsDom::new();
        let grid = element(
            &mut dom,
            None,
            ComputedStyle {
                display: Display::Grid,
                grid_template_rows: GridTrackList::Explicit(TrackSection {
                    tracks: vec![TrackSize::Length(40.0); 2],
                    line_names: vec![],
                }),
                ..Default::default()
            },
        );
        if !empty_first_row {
            element(
                &mut dom,
                Some(grid),
                ComputedStyle {
                    grid_row_start: GridLine::Line(1),
                    grid_row_end: GridLine::Line(2),
                    ..sized(Display::Block, 10.0)
                },
            );
        }
        let item = element(
            &mut dom,
            Some(grid),
            ComputedStyle {
                grid_row_start: GridLine::Line(2),
                grid_row_end: GridLine::Line(3),
                align_self: AlignSelf::FlexStart,
                ..sized(Display::Block, if empty_first_row { 10.0 } else { 95.0 })
            },
        );
        let pages = layout_fragmented(&mut dom, grid, &contribution_input(&fonts, &frag), frag);
        assert_eq!(pages.len(), 2);
        assert!(!pages[0].contribution.is_finished(grid, item));
        let boundary = pages[0].end.token().unwrap();
        assert!(!elidex_layout_block::source_progress::fragment_advanced(
            &dom,
            Some(boundary),
            boundary,
            &pages[0].contribution,
            elidex_layout_block::FragmentationType::Page,
            pages[0].propagated_break_before,
            None
        ));
        let last = pages.last().unwrap();
        assert!(last.contribution.is_finished(grid, item));
        assert!(last.end.is_complete());
        if !empty_first_row {
            assert_eq!(
                dom.world()
                    .get::<&elidex_plugin::LayoutBox>(item)
                    .unwrap()
                    .content
                    .size
                    .height,
                95.0
            );
        }
    }
}

#[test]
fn pages_resume_multicol_after_one_thousand_size_only_columns() {
    let fonts = FontDatabase::new();
    let frag = page(40.0);
    let mut dom = EcsDom::new();
    let owner = element(
        &mut dom,
        None,
        ComputedStyle {
            column_count: Some(2),
            column_fill: elidex_plugin::ColumnFill::Auto,
            ..sized(Display::Block, 1.0)
        },
    );
    let child = element(&mut dom, Some(owner), sized(Display::Block, 1001.0));
    let input = elidex_layout_block::LayoutInput {
        medium: elidex_plugin::Medium::Print,
        ..contribution_input(&fonts, &frag)
    };
    let pages = layout_fragmented(&mut dom, owner, &input, frag);
    assert_eq!(pages.len(), 2);
    assert!(pages[0].end.token().is_some());
    assert!(pages[1].end.is_complete());
    assert!(pages[1].contribution.is_finished(owner, child));
}

#[test]
fn cloned_border_box_uses_ordinary_content_target_on_every_page() {
    let fonts = FontDatabase::new();
    let frag = page(40.0);
    for display in [Display::Grid, Display::Block] {
        for vertical in [false, true] {
            let mut dom = EcsDom::new();
            let mut style = sized(display, 100.0);
            style.box_sizing = elidex_plugin::BoxSizing::BorderBox;
            style.box_decoration_break = elidex_plugin::BoxDecorationBreak::Cloned;
            if vertical {
                style.writing_mode = elidex_plugin::WritingMode::VerticalRl;
                style.width = Dimension::Length(100.0);
                style.padding.left = Dimension::Length(20.0);
                style.padding.right = Dimension::Length(20.0);
            } else {
                style.padding.top = Dimension::Length(20.0);
                style.padding.bottom = Dimension::Length(20.0);
            }
            let root = element(&mut dom, None, style);
            let pages = layout_fragmented(&mut dom, root, &contribution_input(&fonts, &frag), frag);
            assert_eq!(pages.len(), 3, "{display:?}, vertical={vertical}");
            let extent: f32 = pages
                .iter()
                .map(|p| {
                    let size = p.layout_box.content.size;
                    if vertical {
                        size.width
                    } else {
                        size.height
                    }
                })
                .sum();
            assert_eq!(extent, 60.0);
            assert!(pages.last().unwrap().end.is_complete());
        }
    }
}

#[test]
fn non_layout_nodes_before_a_split_block_do_not_require_coverage() {
    let fonts = FontDatabase::new();
    let frag = page(100.0);
    for container in ["block", "grid", "multicol"] {
        for comment_count in [1, 2] {
            let mut dom = EcsDom::new();
            let owner = element(
                &mut dom,
                None,
                match container {
                    "grid" => ComputedStyle {
                        display: Display::Grid,
                        grid_template_rows: tracks(200.0),
                        ..Default::default()
                    },
                    "multicol" => ComputedStyle {
                        column_count: Some(2),
                        column_fill: elidex_plugin::ColumnFill::Auto,
                        ..sized(Display::Block, 100.0)
                    },
                    _ => ComputedStyle {
                        display: Display::Block,
                        ..Default::default()
                    },
                },
            );
            let mut ignored = Vec::new();
            if container != "block" {
                let id = dom.create_comment_ownerless_fixture("outer");
                assert!(dom.append_child(owner, id));
                ignored.push(id);
            }
            let block = if container == "block" {
                owner
            } else {
                element(
                    &mut dom,
                    Some(owner),
                    ComputedStyle {
                        display: Display::Block,
                        ..Default::default()
                    },
                )
            };
            for _ in 0..comment_count {
                let id = dom.create_comment_ownerless_fixture("x");
                assert!(dom.append_child(block, id));
                ignored.push(id);
            }
            if comment_count == 2 {
                let id = dom.create_element_ownerless_fixture("template", Attributes::default());
                assert!(dom.append_child(block, id));
                ignored.push(id);
            }
            let content = element(&mut dom, Some(block), sized(Display::Block, 200.0));
            let fragments =
                layout_fragmented(&mut dom, owner, &contribution_input(&fonts, &frag), frag);
            assert_eq!(fragments.len(), if container == "block" { 2 } else { 1 });
            let last = fragments.last().unwrap();
            assert!(last.end.is_complete());
            assert!(last.contribution.is_finished(block, content));
            for fragment in fragments {
                assert!(!fragment
                    .contribution
                    .contributed_items()
                    .any(|(_, id)| ignored.contains(&id)));
            }
        }
    }
}

#[test]
fn stretched_grid_item_uses_same_content_size_with_or_without_page_room() {
    let fonts = FontDatabase::new();
    let frag = page(500.0);
    for sizing in [
        elidex_plugin::BoxSizing::BorderBox,
        elidex_plugin::BoxSizing::ContentBox,
    ] {
        let mut dom = EcsDom::new();
        let grid = element(
            &mut dom,
            None,
            ComputedStyle {
                display: Display::Grid,
                grid_template_rows: tracks(100.0),
                ..Default::default()
            },
        );
        let mut style = ComputedStyle {
            display: Display::Block,
            box_sizing: sizing,
            ..Default::default()
        };
        style.padding.top = Dimension::Length(10.0);
        style.padding.bottom = Dimension::Length(10.0);
        let item = element(&mut dom, Some(grid), style);
        for room in [Some(&frag), None] {
            let input = elidex_layout_block::LayoutInput {
                fragmentainer: room,
                ..contribution_input(&fonts, &frag)
            };
            assert!(dispatch_layout_child(&mut dom, grid, &input)
                .end
                .is_complete());
            assert_eq!(
                dom.world()
                    .get::<&elidex_plugin::LayoutBox>(item)
                    .unwrap()
                    .content
                    .size
                    .height,
                80.0
            );
            assert_eq!(
                dom.world().get::<&ComputedStyle>(item).unwrap().height,
                Dimension::Auto
            );
        }
    }
}

fn grid_item_box(
    style: ComputedStyle,
    vertical: bool,
    fragmented: bool,
) -> elidex_plugin::LayoutBox {
    let fonts = FontDatabase::new();
    let frag = page(500.0);
    let mut dom = EcsDom::new();
    let grid = element(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Grid,
            writing_mode: if vertical {
                elidex_plugin::WritingMode::VerticalLr
            } else {
                elidex_plugin::WritingMode::HorizontalTb
            },
            width: Dimension::Length(100.0),
            height: Dimension::Length(100.0),
            grid_template_rows: tracks(100.0),
            grid_template_columns: tracks(100.0),
            ..Default::default()
        },
    );
    let item = element(
        &mut dom,
        Some(grid),
        ComputedStyle {
            display: Display::Block,
            ..style
        },
    );
    element(
        &mut dom,
        Some(item),
        ComputedStyle {
            width: Dimension::Length(80.0),
            margin_bottom: Dimension::Length(12.0),
            ..sized(Display::Block, 10.0)
        },
    );
    let input = elidex_layout_block::LayoutInput {
        fragmentainer: fragmented.then_some(&frag),
        ..contribution_input(&fonts, &frag)
    };
    assert!(dispatch_layout_child(&mut dom, grid, &input)
        .end
        .is_complete());
    let lb = (*dom.world().get::<&elidex_plugin::LayoutBox>(item).unwrap()).clone();
    lb
}

#[test]
fn grid_item_stretch_obeys_constraints() {
    for fragmented in [false, true] {
        for (min, max, width) in [(0.0, 40.0, 40.0), (120.0, 200.0, 120.0)] {
            let lb = grid_item_box(
                ComputedStyle {
                    min_width: Dimension::Length(min),
                    max_width: Dimension::Length(max),
                    ..Default::default()
                },
                false,
                fragmented,
            );
            assert_eq!(lb.content.size.width, width);
        }
    }
}

#[test]
fn grid_item_auto_margins_absorb_free_space_before_alignment() {
    for fragmented in [false, true] {
        let lb = grid_item_box(
            ComputedStyle {
                max_width: Dimension::Length(40.0),
                justify_self: elidex_plugin::JustifySelf::Start,
                margin_left: Dimension::Auto,
                margin_right: Dimension::Auto,
                height: Dimension::Length(50.0),
                align_self: elidex_plugin::AlignSelf::FlexEnd,
                margin_top: Dimension::Auto,
                margin_bottom: Dimension::Auto,
                ..Default::default()
            },
            false,
            fragmented,
        );
        assert_eq!(lb.content.origin.x, 30.0);
        assert_eq!(lb.content.origin.y, 25.0);
        let auto = grid_item_box(
            ComputedStyle {
                align_self: elidex_plugin::AlignSelf::FlexEnd,
                margin_top: Dimension::Auto,
                margin_bottom: Dimension::Auto,
                ..Default::default()
            },
            false,
            fragmented,
        );
        assert_eq!(
            auto.content.origin.y,
            (100.0 - auto.content.size.height) / 2.0
        );
    }
}

#[test]
fn grid_item_contains_child_margin() {
    for fragmented in [false, true] {
        let lb = grid_item_box(ComputedStyle::default(), false, fragmented);
        assert_eq!(lb.margin.bottom, 0.0);
    }
}

#[test]
fn vertical_grid_item_aligns_on_track_axes() {
    for fragmented in [false, true] {
        let lb = grid_item_box(
            ComputedStyle {
                writing_mode: elidex_plugin::WritingMode::VerticalLr,
                align_self: elidex_plugin::AlignSelf::FlexEnd,
                width: Dimension::Length(40.0),
                max_height: Dimension::Length(70.0),
                ..Default::default()
            },
            true,
            fragmented,
        );
        assert_eq!(lb.content.origin.x, 60.0);
        assert_eq!(lb.content.size.height, 70.0);
    }
}

#[test]
fn forced_break_before_first_child_survives_paged_driver() {
    let mut dom = EcsDom::new();
    let owner = element(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let child = element(
        &mut dom,
        Some(owner),
        ComputedStyle {
            break_before: elidex_plugin::BreakValue::Page,
            ..sized(Display::Block, 50.)
        },
    );
    let fonts = FontDatabase::new();
    let frag = page(500.);
    let (pages, tokens) =
        layout_fragmented_with_tokens(&mut dom, owner, &contribution_input(&fonts, &frag), frag);
    assert_eq!(pages.len(), 2);
    assert!(tokens[0].is_none());
    assert_eq!(tokens[1].as_ref().unwrap().consumed_block_size, 0.);
    assert!(pages[0].end.token().is_some());
    assert!(!pages[0].contribution.is_finished(owner, child));
    assert!(pages[1].end.is_complete());
    assert!(pages[1].contribution.is_finished(owner, child));
    assert_eq!(pages[1].layout_box.content.size.height, 50.);
}

#[test]
fn forced_break_before_first_child_continues_in_second_column() {
    let fonts = FontDatabase::new();
    let frag = page(500.);
    let mut dom = EcsDom::new();
    let owner = element(
        &mut dom,
        None,
        ComputedStyle {
            column_count: Some(2),
            column_gap: Dimension::Length(0.),
            column_fill: elidex_plugin::ColumnFill::Auto,
            ..sized(Display::Block, 100.)
        },
    );
    let child = element(
        &mut dom,
        Some(owner),
        ComputedStyle {
            break_before: elidex_plugin::BreakValue::Column,
            ..sized(Display::Block, 20.)
        },
    );
    let input = LayoutInput {
        fragmentainer: None,
        ..contribution_input(&fonts, &frag)
    };
    let out = dispatch_layout_child(&mut dom, owner, &input);
    assert!(out.end.is_complete());
    assert!(out.contribution.is_finished(owner, child));
    let lb = get_layout(&dom, child);
    assert_eq!(lb.content.origin, Point::new(100., 0.));
    assert_eq!(lb.content.size.height, 20.);
}
