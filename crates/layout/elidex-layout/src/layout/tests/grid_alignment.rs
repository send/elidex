use super::*;
use elidex_plugin::{AlignSelf, JustifySelf, WritingMode};

static FONTS: std::sync::LazyLock<FontDatabase> = std::sync::LazyLock::new(FontDatabase::new);

fn add(dom: &mut EcsDom, parent: Option<Entity>, style: ComputedStyle) -> Entity {
    let id = dom.create_element_ownerless_fixture("div", Attributes::default());
    if let Some(p) = parent {
        assert!(dom.append_child(p, id));
    }
    dom.world_mut().insert_one(id, style).unwrap();
    id
}

fn geometry(
    grid_style: ComputedStyle,
    item_style: ComputedStyle,
    child_width: f32,
    paged: bool,
) -> LayoutBox {
    geometry_with(grid_style, item_style, child_width, paged, |_, _, _| {})
}

fn geometry_with(
    grid_style: ComputedStyle,
    item_style: ComputedStyle,
    child_width: f32,
    paged: bool,
    setup: impl FnOnce(&mut EcsDom, Entity, Entity),
) -> LayoutBox {
    use elidex_layout_block::{FragmentainerContext, FragmentationType};
    let mut dom = EcsDom::new();
    let track = elidex_plugin::GridTrackList::Explicit(TrackSection::from_tracks(vec![
        elidex_plugin::TrackSize::Length(100.0),
    ]));
    let grid = add(
        &mut dom,
        None,
        ComputedStyle {
            display: Display::Grid,
            grid_template_columns: track.clone(),
            grid_template_rows: track,
            ..grid_style
        },
    );
    let item = add(
        &mut dom,
        Some(grid),
        ComputedStyle {
            display: Display::Block,
            ..item_style
        },
    );
    add(
        &mut dom,
        Some(item),
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(child_width),
            height: Dimension::Length(10.0),
            margin_bottom: Dimension::Length(12.0),
            ..Default::default()
        },
    );
    setup(&mut dom, grid, item);
    let room = FragmentainerContext {
        available_block_size: 500.0,
        fragmentation_type: FragmentationType::Page,
    };
    let input = LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        item_context: elidex_layout_block::ItemContext::NormalFlow,
        assigned: elidex_layout_block::UsedAssignment::default(),
        containing: CssSize::definite(300.0, 500.0),
        containing_inline_size: 300.0,
        offset: Point::new(40.0, 40.0),
        font_db: &FONTS,
        depth: 0,
        float_ctx: None,
        scroll_limits: elidex_layout_block::ScrollLimits::default(),
        viewport: Some(Size::new(500.0, 500.0)),
        fragmentainer: paged.then_some(&room),
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    dispatch_layout_child(&mut dom, grid, &input);
    get_layout(&dom, item)
}

#[test]
fn grid_fits_intrinsic_content_and_charges_decoration_once() {
    for paged in [false, true] {
        for (child, max, padding, expected, x) in [
            (100., 300., 0., 100., 40.),
            (80., 40., 0., 40., 70.),
            (0., 40., 0., 0., 90.),
            (40., 100., 10., 40., 70.),
        ] {
            let mut style = ComputedStyle {
                justify_self: JustifySelf::Start,
                margin_left: Dimension::Auto,
                margin_right: Dimension::Auto,
                max_width: Dimension::Length(max),
                ..Default::default()
            };
            style.padding.left = Dimension::Length(padding);
            style.padding.right = Dimension::Length(padding);
            let lb = geometry(ComputedStyle::default(), style, child, paged);
            assert_eq!(
                (lb.content.size.width, lb.content.origin.x, lb.margin.bottom),
                (expected, x, 0.)
            );
        }
    }
}

#[test]
fn grid_end_placement_uses_signed_offsets_and_reverses_vertical_rows() {
    for paged in [false, true] {
        let lb = geometry(
            ComputedStyle::default(),
            ComputedStyle {
                width: Dimension::Length(120.),
                justify_self: JustifySelf::End,
                margin_left: Dimension::Auto,
                ..Default::default()
            },
            0.,
            paged,
        );
        assert_eq!((lb.margin.left, lb.content.origin.x), (0., 20.));
        for (wm, x) in [
            (WritingMode::VerticalRl, 40.),
            (WritingMode::VerticalLr, 100.),
        ] {
            let lb = geometry(
                ComputedStyle {
                    width: Dimension::Length(100.),
                    height: Dimension::Length(100.),
                    writing_mode: wm,
                    ..Default::default()
                },
                ComputedStyle {
                    width: Dimension::Length(40.),
                    writing_mode: wm,
                    align_self: AlignSelf::FlexEnd,
                    ..Default::default()
                },
                0.,
                paged,
            );
            assert_eq!(lb.content.origin.x, x);
        }
    }
}

/// Physical low-edge displacement and used margins for the ratified area matrix.
#[test]
#[allow(clippy::too_many_lines)] // Factor the six independent fixture axes around one geometry assertion.
fn published_grid_area_matrix() {
    use elidex_plugin::AlignmentSafety::{Safe, Unsafe};
    let positions = [
        (JustifySelf::Start, AlignSelf::Start, 0., 0.),
        (JustifySelf::End, AlignSelf::End, 40., -20.),
        (JustifySelf::Center, AlignSelf::Center, 20., -10.),
        (JustifySelf::Stretch, AlignSelf::Stretch, 0., 0.),
    ];
    for wm in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ] {
        let grid = ComputedStyle {
            writing_mode: wm,
            width: Dimension::Length(100.),
            height: Dimension::Length(100.),
            ..Default::default()
        };
        for inline in [true, false] {
            let x = inline == wm.is_horizontal();
            let reverse = !inline && wm == WritingMode::VerticalRl;
            for (auto_start, auto_end) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                for used in [60_f32, 100., 120.] {
                    let surplus = (100. - used).max(0.);
                    let share = if auto_start && auto_end {
                        surplus / 2.
                    } else {
                        surplus
                    };
                    let start = if auto_start { share } else { 0. };
                    let end = if auto_end { share } else { 0. };
                    let auto = |yes| {
                        if yes {
                            Dimension::Auto
                        } else {
                            Dimension::Length(0.)
                        }
                    };
                    for (preferred, min, max) in [
                        (used, 0., 200.),
                        (used - 20., used, 200.),
                        (used + 20., 0., used),
                    ] {
                        for (justify, align, positive, negative) in positions {
                            for safety in [None, Some(Safe), Some(Unsafe)] {
                                // Invalid modified stretch leaves the seeded start declaration in force.
                                let invalid = justify == JustifySelf::Stretch && safety.is_some();
                                let mut item = ComputedStyle {
                                    writing_mode: wm,
                                    justify_self: JustifySelf::Start,
                                    align_self: AlignSelf::Start,
                                    ..Default::default()
                                };
                                if inline && !invalid {
                                    item.justify_self = justify;
                                    item.justify_self_safety = safety;
                                } else if !inline && !invalid {
                                    item.align_self = align;
                                    item.align_self_safety = safety;
                                }
                                let (size, lo, hi, m1, m2) = if x {
                                    (
                                        &mut item.width,
                                        &mut item.min_width,
                                        &mut item.max_width,
                                        &mut item.margin_left,
                                        &mut item.margin_right,
                                    )
                                } else {
                                    (
                                        &mut item.height,
                                        &mut item.min_height,
                                        &mut item.max_height,
                                        &mut item.margin_top,
                                        &mut item.margin_bottom,
                                    )
                                };
                                *size = Dimension::Length(preferred);
                                *lo = Dimension::Length(min);
                                *hi = Dimension::Length(max);
                                *m1 = auto(if reverse { auto_end } else { auto_start });
                                *m2 = auto(if reverse { auto_start } else { auto_end });
                                let logical = if used == 60. {
                                    if auto_start || auto_end {
                                        start
                                    } else {
                                        positive
                                    }
                                } else if used == 120. && safety != Some(Safe) {
                                    negative
                                } else {
                                    0.
                                };
                                let expected_pos = 40.
                                    + if reverse {
                                        100. - used - logical
                                    } else {
                                        logical
                                    };
                                for paged in [false, true] {
                                    let lb = geometry(grid.clone(), item.clone(), 0., paged);
                                    let observed = if x {
                                        (
                                            lb.content.origin.x,
                                            lb.content.size.width,
                                            lb.margin.left,
                                            lb.margin.right,
                                        )
                                    } else {
                                        (
                                            lb.content.origin.y,
                                            lb.content.size.height,
                                            lb.margin.top,
                                            lb.margin.bottom,
                                        )
                                    };
                                    assert_eq!(observed,(expected_pos,used,if reverse {end}else{start},if reverse {start}else{end}),
                                        "{wm:?} inline={inline} mask={auto_start}/{auto_end} {justify:?} {safety:?} preferred={preferred} paged={paged}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn normal_replaced_subgrid_and_auto_block_margin() {
    use elidex_plugin::{BoxSizing, GridTrackList};
    for paged in [false, true] {
        for (align, height, width) in [
            (JustifySelf::Normal, 30., 60.),
            (JustifySelf::Normal, 60., 120.),
            (JustifySelf::Stretch, 30., 100.),
        ] {
            let lb = geometry_with(
                ComputedStyle::default(),
                ComputedStyle {
                    justify_self: align,
                    height: Dimension::Length(height),
                    ..Default::default()
                },
                0.,
                paged,
                |dom, _, item| {
                    dom.world_mut()
                        .insert_one(
                            item,
                            elidex_ecs::ImageData {
                                width: 60,
                                height: 30,
                                pixels: std::sync::Arc::new(vec![0; 7200]),
                            },
                        )
                        .unwrap();
                },
            );
            assert_eq!(lb.content.size, Size::new(width, height));
        }
        for (height, margin) in [(Dimension::Auto, 39.), (Dimension::Length(50.), 25.)] {
            let lb = geometry(
                ComputedStyle::default(),
                ComputedStyle {
                    height,
                    margin_top: Dimension::Auto,
                    margin_bottom: Dimension::Auto,
                    align_self: AlignSelf::End,
                    ..Default::default()
                },
                0.,
                paged,
            );
            assert_eq!(
                (lb.margin.top, lb.margin.bottom, lb.content.origin.y),
                (margin, margin, 40. + margin)
            );
        }
        for sizing in [BoxSizing::BorderBox, BoxSizing::ContentBox] {
            let mut style = ComputedStyle {
                box_sizing: sizing,
                ..Default::default()
            };
            style.padding.top = Dimension::Length(10.);
            style.padding.bottom = Dimension::Length(10.);
            let lb = geometry(ComputedStyle::default(), style, 0., paged);
            assert_eq!(
                (lb.content.size.height, lb.border_box().size.height),
                (80., 100.)
            );
        }
        for (col, row) in [(true, false), (false, true), (true, true), (false, false)] {
            let lb = geometry_with(
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
            );
            let width = if col { 100. } else { 40. };
            let height = if row { 100. } else { 40. };
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

#[test]
fn used_auto_inherits_parent_safety_and_scrollport_limits_omitted_end() {
    use elidex_plugin::{AlignItems, AlignmentSafety, JustifyItems, Overflow};
    for paged in [false, true] {
        for size in [40., 120.] {
            let parent = ComputedStyle {
                justify_items: JustifyItems::End,
                justify_items_safety: Some(AlignmentSafety::Safe),
                align_items: AlignItems::End,
                align_items_safety: Some(AlignmentSafety::Safe),
                ..Default::default()
            };
            let child = ComputedStyle {
                width: Dimension::Length(size),
                height: Dimension::Length(size),
                ..Default::default()
            };
            let pos = if size == 40. { 100. } else { 40. };
            assert_eq!(
                geometry(parent.clone(), child.clone(), 0., paged)
                    .content
                    .origin,
                Point::new(pos, pos)
            );
            for overflow in [Overflow::Visible, Overflow::Hidden] {
                for safety in [
                    None,
                    Some(AlignmentSafety::Safe),
                    Some(AlignmentSafety::Unsafe),
                ] {
                    let lb = geometry(
                        ComputedStyle {
                            overflow_x: overflow,
                            overflow_y: overflow,
                            ..parent.clone()
                        },
                        ComputedStyle {
                            justify_self: JustifySelf::End,
                            align_self: AlignSelf::End,
                            justify_self_safety: safety,
                            align_self_safety: safety,
                            ..child.clone()
                        },
                        0.,
                        paged,
                    );
                    let pos = if size == 40. {
                        100.
                    } else if safety == Some(AlignmentSafety::Safe)
                        || (safety.is_none() && overflow == Overflow::Hidden)
                    {
                        40.
                    } else {
                        20.
                    };
                    assert_eq!(lb.content.origin, Point::new(pos, pos));
                }
            }
        }
    }
}

#[test]
fn authored_grid_uses_nearest_scroll_origin_after_relayout() {
    use elidex_css::{parse_stylesheet, Origin};
    let fonts = FontDatabase::new();
    for paged in [false, true] {
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
            ("visible", 40, 20.),
            ("hidden", 40, 40.),
            ("hidden", 65, 65.),
        ] {
            let sheet=parse_stylesheet(&format!("div {{display:block}} #scroller {{overflow:{overflow};margin-left:{left}px;width:100px}} #grid {{display:grid;grid-template-columns:100px;grid-template-rows:100px;justify-items:end}} #item {{width:120px;justify-self:auto}}"),Origin::Author);
            let viewport = Size::new(500., 500.);
            elidex_style::resolve_styles(&mut dom, &[&sheet], viewport);
            if paged {
                let env = elidex_layout_block::LayoutEnv {
                    font_db: &fonts,
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
                assert_eq!(layout_fragmented(&mut dom, root, &input, page).len(), 1);
            } else {
                layout_tree(&mut dom, viewport, &fonts);
            }
            assert_eq!(get_layout(&dom, item).content.origin.x, expected);
            assert_eq!(get_layout(&dom, leaf).content.origin.x, expected);
        }
    }
}

#[test]
fn logical_area_in_second_row_and_orthogonal_item() {
    use elidex_plugin::{Direction, GridLine, GridTrackList, TrackSize};
    for paged in [false, true] {
        for (wm, expected) in [
            (WritingMode::VerticalLr, 200.),
            (WritingMode::VerticalRl, 40.),
        ] {
            let lb =
                geometry_with(
                    ComputedStyle {
                        width: Dimension::Length(200.),
                        height: Dimension::Length(100.),
                        writing_mode: wm,
                        ..Default::default()
                    },
                    ComputedStyle {
                        width: Dimension::Length(40.),
                        height: Dimension::Length(60.),
                        grid_row_start: GridLine::Line(2),
                        align_self: AlignSelf::End,
                        ..Default::default()
                    },
                    0.,
                    paged,
                    |dom, grid, _| {
                        dom.world_mut()
                            .get::<&mut ComputedStyle>(grid)
                            .unwrap()
                            .grid_template_rows = GridTrackList::Explicit(
                            TrackSection::from_tracks(vec![TrackSize::Length(100.); 2]),
                        );
                    },
                );
            assert_eq!(lb.content.origin.x, expected);
        }
        for direction in [Direction::Ltr, Direction::Rtl] {
            for child_mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
                let lb = geometry(
                    ComputedStyle {
                        writing_mode: WritingMode::VerticalRl,
                        direction,
                        width: Dimension::Length(100.),
                        height: Dimension::Length(100.),
                        ..Default::default()
                    },
                    ComputedStyle {
                        writing_mode: child_mode,
                        width: Dimension::Length(40.),
                        height: Dimension::Length(60.),
                        margin_left: Dimension::Auto,
                        margin_right: Dimension::Auto,
                        justify_self: JustifySelf::End,
                        ..Default::default()
                    },
                    0.,
                    paged,
                );
                assert_eq!(
                    lb.content.origin,
                    Point::new(
                        70.,
                        if direction == Direction::Rtl {
                            40.
                        } else {
                            80.
                        }
                    )
                );
            }
        }
    }
}

#[test]
fn constrained_area_budgets_and_formatter_dispatch() {
    for paged in [false, true] {
        for (display, columns) in [
            (Display::Block, None),
            (Display::Flex, None),
            (Display::Grid, None),
            (Display::Table, None),
            (Display::Block, Some(2)),
        ] {
            let lb = geometry_with(
                ComputedStyle::default(),
                ComputedStyle {
                    width: Dimension::Percentage(50.),
                    column_count: columns,
                    height: Dimension::Percentage(50.),
                    margin_top: Dimension::Auto,
                    margin_bottom: Dimension::Auto,
                    justify_self: JustifySelf::End,
                    ..Default::default()
                },
                0.,
                paged,
                |dom, _, item| {
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(item)
                        .unwrap()
                        .display = display;
                },
            );
            assert_eq!(
                (lb.content.size, lb.content.origin),
                (Size::new(50., 50.), Point::new(90., 65.)),
                "{display:?}"
            );
        }
        for (align, width, outer) in [
            (JustifySelf::Start, 40., 80.),
            (JustifySelf::Stretch, 60., 100.),
        ] {
            let mut item = ComputedStyle {
                justify_self: align,
                margin_left: Dimension::Length(5.),
                margin_right: Dimension::Length(15.),
                ..Default::default()
            };
            item.padding.left = Dimension::Length(10.);
            item.padding.right = Dimension::Length(10.);
            let lb = geometry(ComputedStyle::default(), item, 40., paged);
            assert_eq!(
                (lb.content.size.width, lb.margin_box().size.width),
                (width, outer)
            );
        }
    }
}

#[test]
fn fit_content_uses_the_inner_intrinsic_range() {
    use elidex_plugin::{FlexWrap, GridTrackList, TrackSize};
    for paged in [false, true] {
        for (budget, expected) in [(20., 40.), (80., 80.), (180., 140.)] {
            let lb = geometry_with(
                ComputedStyle::default(),
                ComputedStyle {
                    justify_self: JustifySelf::Start,
                    flex_wrap: FlexWrap::Wrap,
                    ..Default::default()
                },
                40.,
                paged,
                |dom, grid, item| {
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(item)
                        .unwrap()
                        .display = Display::Flex;
                    dom.world_mut()
                        .get::<&mut ComputedStyle>(grid)
                        .unwrap()
                        .grid_template_columns =
                        GridTrackList::Explicit(TrackSection::from_tracks(vec![
                            TrackSize::Length(budget),
                        ]));
                    for width in [40., 40., 20.] {
                        add(
                            dom,
                            Some(item),
                            ComputedStyle {
                                display: Display::Block,
                                width: Dimension::Length(width),
                                ..Default::default()
                            },
                        );
                    }
                },
            );
            assert_eq!(lb.content.size.width, expected);
        }
    }
}

#[test]
fn reversed_scroll_origin_protects_the_physical_high_edge() {
    use elidex_plugin::{AlignmentSafety, Overflow};
    for paged in [false, true] {
        for (overflow, safety, x) in [
            (Overflow::Visible, None, 40.),
            (Overflow::Hidden, None, 20.),
            (Overflow::Hidden, Some(AlignmentSafety::Unsafe), 40.),
            (Overflow::Visible, Some(AlignmentSafety::Safe), 20.),
        ] {
            let lb = geometry(
                ComputedStyle {
                    writing_mode: WritingMode::VerticalRl,
                    width: Dimension::Length(100.),
                    height: Dimension::Length(100.),
                    overflow_x: overflow,
                    ..Default::default()
                },
                ComputedStyle {
                    width: Dimension::Length(120.),
                    height: Dimension::Length(50.),
                    align_self: AlignSelf::End,
                    align_self_safety: safety,
                    ..Default::default()
                },
                0.,
                paged,
            );
            assert_eq!(lb.content.origin.x, x);
        }
    }
}
