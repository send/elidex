use super::*;
use elidex_css::{parse_stylesheet, Origin};
use elidex_plugin::GridTrackList;

struct AreaProbe(Size);

fn record_area_probe(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    let result = dispatch_layout_child(dom, entity, input);
    if input.is_probe
        && matches!(
            input.item_context,
            elidex_layout_block::ItemContext::GridArea(_)
        )
    {
        dom.world_mut()
            .insert_one(entity, AreaProbe(result.layout_box.content.size))
            .unwrap();
    }
    result
}

#[test]
fn subgrid_area_probe_and_publication_agree() {
    for paged in [false, true] {
        for (col, row) in [(true, false), (false, true), (true, true), (false, false)] {
            let width = if col { 100. } else { 40. };
            let height = if row { 100. } else { 40. };
            let lb = geometry_with_root(
                ComputedStyle::default(),
                ComputedStyle {
                    width: Dimension::Length(40.),
                    height: Dimension::Length(40.),
                    max_width: Dimension::Length(50.),
                    max_height: Dimension::Length(50.),
                    justify_self: JustifySelf::End,
                    align_self: AlignSelf::End,
                    ..Default::default()
                },
                0.,
                paged,
                |dom, _, item| {
                    let mut style = dom.world_mut().get::<&mut ComputedStyle>(item).unwrap();
                    style.display = Display::Grid;
                    if col {
                        style.grid_template_columns = GridTrackList::Subgrid { line_names: vec![] };
                    }
                    if row {
                        style.grid_template_rows = GridTrackList::Subgrid { line_names: vec![] };
                    }
                },
                |dom, grid, item, input| {
                    elidex_layout_grid::layout_grid(dom, grid, input, record_area_probe);
                    let probe = dom
                        .world()
                        .get::<&AreaProbe>(item)
                        .expect("item received an area probe");
                    assert_eq!(probe.0, Size::new(width, height));
                },
            );
            assert_eq!(
                (lb.content.origin, lb.content.size),
                (
                    Point::new(140. - width, 140. - height),
                    Size::new(width, height)
                )
            );
        }
    }
}

fn layout_authored(dom: &mut EcsDom, root: Entity, viewport: Size, paged: bool) {
    if paged {
        let env = elidex_layout_block::LayoutEnv {
            font_db: &FONTS,
            layout_child: dispatch_layout_child,
            depth: 0,
            viewport: Some(viewport),
            scroll_limits: elidex_layout_block::ScrollLimits::default(),
            layout_generation: 0,
            is_probe: false,
        };
        let input = LayoutInput {
            depth: 0,
            containing: CssSize::definite(500., 500.),
            is_probe: false,
            ..LayoutInput::probe(&env, 500.)
        };
        let page = elidex_layout_block::FragmentainerContext {
            available_block_size: 500.,
            fragmentation_type: elidex_layout_block::FragmentationType::Page,
        };
        assert_eq!(layout_fragmented(dom, root, &input, page).len(), 1);
    } else {
        layout_tree(dom, viewport, &FONTS);
    }
}

#[test]
fn authored_grid_uses_nearest_scroll_origin_after_relayout() {
    for paged in [false, true] {
        for (parent_value, child_value, positions) in [
            ("end", "auto", [20., 40., 65.]),
            ("safe end", "auto", [40., 40., 65.]),
            ("safe end", "unsafe end", [20., 20., 45.]),
        ] {
            let mut dom = EcsDom::new();
            let root = add(&mut dom, None, ComputedStyle::default());
            let scroller = add(&mut dom, Some(root), ComputedStyle::default());
            let grid = add(&mut dom, Some(scroller), ComputedStyle::default());
            let item = add(&mut dom, Some(grid), ComputedStyle::default());
            let leaf = add(&mut dom, Some(item), ComputedStyle::default());
            for (node, name) in [(scroller, "scroller"), (grid, "grid"), (item, "item")] {
                dom.set_attribute(node, "id", name);
            }
            for (overflow, left, expected) in [
                ("visible", 40, positions[0]),
                ("hidden", 40, positions[1]),
                ("hidden", 65, positions[2]),
            ] {
                let css=format!("div {{ display:block }}
                    #scroller {{ overflow:{overflow}; margin-left:{left}px; width:100px }}
                    #grid {{ display:grid; grid-template-columns:100px; grid-template-rows:100px; justify-items:{parent_value} }}
                    #item {{ width:120px; margin-left:auto; justify-self:{child_value} }}");
                let sheet = parse_stylesheet(&css, Origin::Author);
                let viewport = Size::new(500., 500.);
                elidex_style::resolve_styles(&mut dom, &[&sheet], viewport);
                layout_authored(&mut dom, root, viewport, paged);
                assert_eq!(get_layout(&dom, item).margin.left, 0.);
                assert_eq!(get_layout(&dom, item).content.origin.x, expected);
                assert_eq!(get_layout(&dom, leaf).content.origin.x, expected);
            }
        }
    }
}

#[test]
fn authored_self_values_preserve_serialization_through_placement() {
    for paged in [false, true] {
        for (child_value, size, coordinate) in [
            ("auto", 40, 60.),
            ("auto", 120, 0.),
            ("unsafe end", 120, -20.),
        ] {
            let mut dom = EcsDom::new();
            let grid = add(&mut dom, None, ComputedStyle::default());
            let item = add(&mut dom, Some(grid), ComputedStyle::default());
            dom.set_attribute(grid, "id", "grid");
            dom.set_attribute(item, "id", "item");
            let css=format!("#grid {{display:grid;grid-template-columns:100px;grid-template-rows:100px;justify-items:safe end;align-items:safe end}}
                #item {{width:{size}px;height:{size}px;justify-self:{child_value};align-self:{child_value}}}");
            let sheet = parse_stylesheet(&css, Origin::Author);
            let viewport = Size::new(500., 500.);
            elidex_style::resolve_styles(&mut dom, &[&sheet], viewport);
            layout_authored(&mut dom, grid, viewport, paged);
            for (entity, property, expected) in [
                (grid, "justify-items", "safe end"),
                (grid, "align-items", "safe end"),
                (item, "justify-self", child_value),
                (item, "align-self", child_value),
            ] {
                assert_eq!(
                    elidex_style::serialize_resolved_value(
                        property,
                        &elidex_layout_block::get_style(&dom, entity)
                    ),
                    expected
                );
            }
            assert_eq!(
                get_layout(&dom, item).content.origin,
                Point::new(coordinate, coordinate)
            );
        }
    }
}

#[test]
fn fit_content_grid_counts_empty_explicit_tracks() {
    use elidex_plugin::TrackSize;
    let mut sizes = Vec::new();
    for paged in [false, true] {
        for hidden in [false, true] {
            let lb = geometry_with(
                ComputedStyle::default(),
                ComputedStyle {
                    justify_self: JustifySelf::Start,
                    align_self: AlignSelf::Start,
                    grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                        TrackSize::Length(43.),
                    ])),
                    grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(
                        vec![TrackSize::Length(30.), TrackSize::Length(20.)],
                    )),
                    column_gap: Dimension::Length(7.),
                    ..Default::default()
                },
                0.,
                paged,
                |dom, _, item| {
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(item)
                        .unwrap()
                        .display = Display::Grid;
                    let leaf = dom.children(item)[0];
                    if hidden {
                        dom.world_mut()
                            .get::<&mut ComputedStyle>(leaf)
                            .unwrap()
                            .display = Display::None;
                    } else {
                        assert!(dom.remove_child(item, leaf));
                    }
                },
            );
            sizes.push(lb.content.size);
        }
    }
    assert_eq!(sizes, [Size::new(57., 43.); 4]);
}

#[test]
fn fit_content_flex_follows_logical_main_axis() {
    use elidex_plugin::FlexDirection;
    let mut heights = Vec::new();
    for paged in [false, true] {
        for direction in [FlexDirection::Row, FlexDirection::Column] {
            let vertical = ComputedStyle {
                writing_mode: WritingMode::VerticalRl,
                ..Default::default()
            };
            let lb = geometry_with(
                vertical.clone(),
                ComputedStyle {
                    justify_self: JustifySelf::Start,
                    flex_direction: direction,
                    column_gap: Dimension::Length(7.),
                    ..vertical.clone()
                },
                0.,
                paged,
                |dom, _, item| {
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(item)
                        .unwrap()
                        .display = Display::Flex;
                    let leaf = dom.children(item)[0];
                    for (child, height) in
                        [(leaf, 20.), (add(dom, Some(item), vertical.clone()), 30.)]
                    {
                        let mut style = dom.world_mut().get::<&mut ComputedStyle>(child).unwrap();
                        style.display = Display::Block;
                        style.writing_mode = WritingMode::VerticalRl;
                        style.height = Dimension::Length(height);
                        style.margin_bottom = Dimension::Length(5.);
                    }
                },
            );
            heights.push(lb.content.size.height);
        }
    }
    assert_eq!(heights, [67., 35., 67., 35.]);
}

#[test]
fn vertical_intrinsic_contributions_use_the_parent_inline_axis() {
    use elidex_plugin::FlexDirection;
    for paged in [false, true] {
        for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
            for (direction, height) in [
                (FlexDirection::RowReverse, 67.),
                (FlexDirection::ColumnReverse, 35.),
            ] {
                let vertical = ComputedStyle {
                    writing_mode: mode,
                    ..Default::default()
                };
                let lb = geometry_with(
                    vertical.clone(),
                    ComputedStyle {
                        justify_self: JustifySelf::Start,
                        flex_direction: direction,
                        column_gap: Dimension::Length(7.),
                        row_gap: Dimension::Length(19.),
                        ..vertical
                    },
                    0.,
                    paged,
                    |dom, _, item| {
                        dom.world_mut()
                            .get::<&mut ComputedStyle>(item)
                            .unwrap()
                            .display = Display::Flex;
                        let first = dom.children(item)[0];
                        for (child, h) in [
                            (first, 20.),
                            (add(dom, Some(item), ComputedStyle::default()), 30.),
                        ] {
                            let mut style =
                                dom.world_mut().get::<&mut ComputedStyle>(child).unwrap();
                            style.display = Display::Block;
                            style.width = Dimension::Length(11.);
                            style.height = Dimension::Length(h);
                            style.margin_bottom = Dimension::Length(5.);
                        }
                    },
                );
                assert_eq!(lb.content.size.height, height, "{mode:?} {direction:?}");
            }
        }
    }
}

#[test]
fn empty_vertical_grid_preserves_its_own_rows_and_columns() {
    use elidex_plugin::{Position, TrackSize};
    for paged in [false, true] {
        for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
            let vertical = ComputedStyle {
                writing_mode: mode,
                ..Default::default()
            };
            let lb = geometry_with(
                vertical.clone(),
                ComputedStyle {
                    align_self: AlignSelf::Start,
                    justify_self: JustifySelf::Start,
                    grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                        TrackSize::Length(43.),
                    ])),
                    grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(
                        vec![TrackSize::Length(30.), TrackSize::Length(20.)],
                    )),
                    column_gap: Dimension::Length(7.),
                    ..vertical
                },
                200.,
                paged,
                |dom, _, item| {
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(item)
                        .unwrap()
                        .display = Display::Grid;
                    let child = dom.children(item)[0];
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(child)
                        .unwrap()
                        .position = Position::Absolute;
                },
            );
            assert_eq!(lb.content.size, Size::new(43., 57.));
        }
    }
}

#[test]
fn empty_auto_fit_tracks_collapse_their_gutters_beside_an_authored_track() {
    use elidex_plugin::{AutoRepeatMode, TrackSize};
    for paged in [false, true] {
        for (mode, width) in [
            (AutoRepeatMode::AutoFit, 30.),
            (AutoRepeatMode::AutoFill, 57.),
        ] {
            let lb = geometry_with(
                ComputedStyle::default(),
                ComputedStyle {
                    justify_self: JustifySelf::Start,
                    grid_template_columns: GridTrackList::AutoRepeat {
                        before: TrackSection::from_tracks(vec![TrackSize::Length(30.)]),
                        pattern: TrackSection::from_tracks(vec![TrackSize::Length(20.)]),
                        mode,
                        after: TrackSection::from_tracks(vec![]),
                    },
                    column_gap: Dimension::Length(7.),
                    ..Default::default()
                },
                200.,
                paged,
                |dom, _, item| {
                    let child = dom.children(item)[0];
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(child)
                        .unwrap()
                        .display = Display::None;
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(item)
                        .unwrap()
                        .display = Display::Grid;
                },
            );
            assert_eq!(lb.content.size.width, width);
        }
    }
}

#[test]
fn vertical_inline_grid_shrinks_its_auto_height_with_definite_width() {
    use elidex_plugin::TrackSize;
    for paged in [false, true] {
        let mut dom = EcsDom::new();
        let grid = add(
            &mut dom,
            None,
            ComputedStyle {
                display: Display::InlineGrid,
                writing_mode: WritingMode::VerticalRl,
                width: Dimension::Length(90.),
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(57.),
                ])),
                ..Default::default()
            },
        );
        layout_authored(&mut dom, grid, Size::new(300., 500.), paged);
        assert_eq!(get_layout(&dom, grid).content.size, Size::new(90., 57.));
    }
}

#[test]
fn nodes_without_in_flow_boxes_do_not_add_intrinsic_gaps() {
    for paged in [false, true] {
        let lb = geometry_with(
            ComputedStyle::default(),
            ComputedStyle {
                justify_self: JustifySelf::Start,
                column_gap: Dimension::Length(17.),
                ..Default::default()
            },
            40.,
            paged,
            |dom, _, item| {
                dom.world_mut()
                    .get::<&mut ComputedStyle>(item)
                    .unwrap()
                    .display = Display::Flex;
                let comment = dom.create_comment_ownerless_fixture("unused");
                let whitespace = dom.create_text_ownerless_fixture(" \t ");
                assert!(dom.append_child(item, comment));
                assert!(dom.append_child(item, whitespace));
            },
        );
        assert_eq!(lb.content.size.width, 40.);
    }
}
