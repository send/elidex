use super::*;
use elidex_layout_block::UsedAssignment;
use elidex_plugin::{BoxSizing, EdgeSizes, FlexBasis, GridTrackList, TrackSize};
use std::cell::RefCell;

struct Call {
    entity: Entity,
    probe: bool,
    returned_origin: Point,
    generation: u32,
    assigned: UsedAssignment,
    origin: Point,
    size: Size,
    style: ComputedStyle,
}

thread_local! {
    static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
}

fn observe(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    assert!(matches!(
        input.item_context,
        elidex_layout_block::ItemContext::Independent
    ));
    let style = elidex_layout_block::get_style(dom, entity);
    let result = dispatch_layout_child(dom, entity, input);
    CALLS.with_borrow_mut(|calls| {
        calls.push(Call {
            entity,
            probe: input.is_probe,
            returned_origin: result.layout_box.content.origin,
            generation: result.layout_box.layout_generation,
            assigned: input.assigned,
            origin: input.offset,
            size: result.layout_box.content.size,
            style,
        });
    });
    result
}

fn run(dom: &mut EcsDom, parent: Entity, width: f32) {
    run_at_generation(dom, parent, width, 0);
}

fn run_at_generation(dom: &mut EcsDom, parent: Entity, width: f32, generation: u32) {
    CALLS.with_borrow_mut(Vec::clear);
    let fonts = FontDatabase::new();
    let input = LayoutInput {
        available_height_if_indefinite: None,
        containing_width_definite: true,
        item_context: elidex_layout_block::ItemContext::NormalFlow,
        assigned: UsedAssignment::default(),
        containing: CssSize::width_only(width),
        containing_inline_size: width,
        offset: Point::ZERO,
        font_db: &fonts,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: generation,
        medium: elidex_plugin::Medium::Screen,
        placement: elidex_layout_block::FragmentPlacement::Normal,
        is_probe: false,
    };
    elidex_layout_flex::layout_flex(dom, parent, &input, observe);
}

fn node(dom: &mut EcsDom, parent: Option<Entity>, style: ComputedStyle) -> Entity {
    let entity = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.world_mut().insert_one(entity, style);
    if let Some(parent) = parent {
        dom.append_child(parent, entity);
    }
    entity
}

fn style(display: Display) -> ComputedStyle {
    ComputedStyle {
        display,
        ..Default::default()
    }
}

#[test]
fn repeated_flex_assignment_keeps_percentage_style_and_resizes_descendant() {
    let mut dom = EcsDom::new();
    let parent = node(&mut dom, None, style(Display::Flex));
    let item_style = ComputedStyle {
        width: Dimension::Percentage(50.0),
        margin_left: Dimension::Auto,
        flex_shrink: 0.0,
        ..style(Display::Block)
    };
    let item = node(&mut dom, Some(parent), item_style.clone());
    let child = node(
        &mut dom,
        Some(item),
        ComputedStyle {
            width: Dimension::Percentage(50.0),
            ..style(Display::Block)
        },
    );
    for width in [80.0, 120.0, 200.0, 300.0] {
        run(&mut dom, parent, width);
        let expected = width / 2.0;
        let lb = get_layout(&dom, item);
        assert_eq!(
            (lb.content.origin.x, lb.content.size.width, lb.margin.left),
            (expected, expected, expected)
        );
        assert_eq!(get_layout(&dom, child).content.size.width, expected / 2.0);
        assert_eq!(elidex_layout_block::get_style(&dom, item), item_style);
        CALLS.with_borrow(|calls| {
            assert!(calls.iter().all(|call| call.style == item_style));
            let call = calls.last().unwrap();
            assert!(!call.probe);
            assert_eq!(call.origin.x, 0.0);
            assert_eq!(call.assigned.content.width, Some(expected));
            assert_eq!(call.size.width, expected);
        });
    }
}

#[test]
fn definitive_used_edges_are_signed_and_advance_the_next_item_once() {
    for (width, height, fixed, share) in [
        (200.0, 30.0, 0.0, 60.0),
        (80.0, 90.0, 0.0, 0.0),
        (90.0, 30.0, 10.0, 0.0),
    ] {
        let mut dom = EcsDom::new();
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                height: Dimension::Length(70.0),
                ..style(Display::Flex)
            },
        );
        let base = ComputedStyle {
            width: Dimension::Length(40.0),
            height: Dimension::Length(height),
            flex_shrink: 0.0,
            ..style(Display::Block)
        };
        let first = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                margin_left: if fixed == 0.0 {
                    Dimension::Auto
                } else {
                    Dimension::Length(fixed)
                },
                margin_top: Dimension::Auto,
                margin_bottom: Dimension::Auto,
                ..base.clone()
            },
        );
        let second = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                margin_right: Dimension::Auto,
                ..base
            },
        );
        run(&mut dom, parent, width);
        let first_box = get_layout(&dom, first);
        let top = if height < 70.0 { 20.0 } else { 0.0 };
        let bottom = 70.0 - height - top;
        assert_eq!(
            first_box.margin,
            EdgeSizes::new(top, 0.0, bottom, fixed + share)
        );
        assert_eq!(first_box.margin_box().size.height, 70.0);
        assert_eq!(first_box.content.origin, Point::new(fixed + share, top));
        let next = get_layout(&dom, second);
        assert_eq!(next.content.origin.x, fixed + share + 40.0);
        assert_eq!(next.margin, EdgeSizes::new(0.0, share, 0.0, 0.0));
        CALLS.with_borrow(|calls| {
            let first_call = calls
                .iter()
                .find(|c| c.entity == first && !c.probe)
                .unwrap();
            assert_eq!(first_call.origin, Point::ZERO);
            assert_eq!(first_call.assigned.margin.bottom, Some(bottom));
            let next_call = calls
                .iter()
                .find(|c| c.entity == second && !c.probe)
                .unwrap();
            assert_eq!(next_call.origin.x, next.content.origin.x);
        });
    }
}

#[test]
fn stretch_uses_content_height_for_descendants_and_replaced_final_pair() {
    for image in [false, true] {
        let mut dom = EcsDom::new();
        let height = if image { 70.0 } else { 60.0 };
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                height: Dimension::Length(height),
                ..style(Display::Flex)
            },
        );
        let padding = if image { 0.0 } else { 5.0 };
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                width: Dimension::Length(60.0),
                flex_basis: FlexBasis::Length(100.0),
                min_width: Dimension::ZERO,
                padding: EdgeSizes {
                    top: Dimension::Length(padding),
                    bottom: Dimension::Length(padding),
                    ..Default::default()
                },
                box_sizing: BoxSizing::BorderBox,
                ..style(Display::Block)
            },
        );
        if image {
            dom.world_mut().insert_one(
                item,
                elidex_ecs::ImageData {
                    width: 60,
                    height: 30,
                    pixels: std::sync::Arc::new(vec![0; 7200]),
                },
            );
        }
        let child = (!image).then(|| {
            node(
                &mut dom,
                Some(item),
                ComputedStyle {
                    height: Dimension::Percentage(50.0),
                    ..style(Display::Block)
                },
            )
        });
        run(&mut dom, parent, 100.0);
        let content = height - 2.0 * padding;
        assert_eq!(
            get_layout(&dom, item).content.size,
            Size::new(100.0, content)
        );
        if let Some(child) = child {
            assert_eq!(get_layout(&dom, child).content.size.height, 25.0);
        }
        CALLS.with_borrow(|calls| {
            let measured = calls
                .iter()
                .find(|call| {
                    call.probe
                        && call.assigned.content.width == Some(100.0)
                        && call.assigned.content.height.is_none()
                })
                .unwrap();
            if image {
                assert_eq!(measured.size, Size::new(100.0, 50.0));
            }
            assert_eq!(calls.last().unwrap().assigned.content.height, Some(content));
        });
    }
}

#[test]
fn nested_formatters_size_descendants_at_the_assigned_width() {
    for (display, multicol) in [
        (Display::Flex, false),
        (Display::Grid, false),
        (Display::Table, false),
        (Display::Block, true),
    ] {
        for width in [100.0, 160.0] {
            let mut dom = EcsDom::new();
            let parent = node(&mut dom, None, style(Display::Flex));
            let item = node(
                &mut dom,
                Some(parent),
                ComputedStyle {
                    width: Dimension::Length(40.0),
                    min_width: Dimension::ZERO,
                    flex_grow: 1.0,
                    grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(
                        vec![TrackSize::Percentage(100.0)],
                    )),
                    column_count: multicol.then_some(2),
                    column_gap: Dimension::ZERO,
                    ..style(display)
                },
            );
            let row = if display == Display::Table {
                node(&mut dom, Some(item), style(Display::TableRow))
            } else {
                item
            };
            let child = node(
                &mut dom,
                Some(row),
                ComputedStyle {
                    height: Dimension::Length(10.0),
                    flex_grow: 1.0,
                    ..style(if display == Display::Table {
                        Display::TableCell
                    } else {
                        Display::Block
                    })
                },
            );
            TRACK_CHILD.set((display == Display::Grid).then_some(child));
            TRACK_WIDTH.set(None);
            run(&mut dom, parent, width);
            if display == Display::Grid {
                assert_eq!(TRACK_WIDTH.get(), Some(width));
                CALLS.with_borrow(|calls| {
                    assert_eq!(calls.last().unwrap().assigned.content.width, Some(width));
                });
            }
            TRACK_CHILD.set(None);
            assert_eq!(get_layout(&dom, item).content.size.width, width);
            assert_eq!(
                get_layout(&dom, child).content.size.width,
                if multicol { width / 2.0 } else { width },
                "{display:?}, multicol={multicol}"
            );
        }
    }
}

#[test]
fn empty_and_depth_limited_boxes_keep_assigned_zero_and_signed_edges() {
    for display in [Display::Block, Display::Flex, Display::Grid, Display::Table] {
        let mut dom = EcsDom::new();
        let item = node(
            &mut dom,
            None,
            ComputedStyle {
                width: Dimension::Length(80.0),
                height: Dimension::Length(90.0),
                ..style(display)
            },
        );
        let fonts = FontDatabase::new();
        for depth in [0, elidex_layout_block::MAX_LAYOUT_DEPTH] {
            let input = LayoutInput {
                available_height_if_indefinite: None,
                containing_width_definite: true,
                item_context: elidex_layout_block::ItemContext::NormalFlow,
                assigned: UsedAssignment {
                    content: Size {
                        width: Some(0.0),
                        height: Some(20.0),
                    },
                    definite_for_percentages: Size {
                        width: true,
                        height: true,
                    },
                    margin: EdgeSizes {
                        left: Some(5.0),
                        right: Some(0.0),
                        top: Some(2.0),
                        bottom: Some(-3.0),
                    },
                },
                containing: CssSize::width_only(100.0),
                containing_inline_size: 100.0,
                offset: Point::new(10.0, 20.0),
                font_db: &fonts,
                depth,
                float_ctx: None,
                viewport: None,
                fragmentainer: None,
                break_token: None,
                subgrid: None,
                layout_generation: 3,
                medium: elidex_plugin::Medium::Screen,
                placement: elidex_layout_block::FragmentPlacement::Normal,
                is_probe: false,
            };
            let result = dispatch_layout_child(&mut dom, item, &input);
            assert_eq!(result.layout_box.content.size, Size::new(0.0, 20.0));
            let lb = get_layout(&dom, item);
            assert_eq!(lb.content.origin, Point::new(15.0, 22.0));
            assert_eq!(lb.margin, EdgeSizes::new(2.0, 0.0, -3.0, 5.0));
            assert_eq!(lb.layout_generation, 3);
        }
    }
}

#[test]
fn relative_flex_item_preserves_paged_generation_and_flow_origin() {
    for left in [0.0, 5.0] {
        let mut dom = EcsDom::new();
        let parent = node(&mut dom, None, style(Display::Flex));
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                position: Position::Relative,
                left: Dimension::Length(left),
                width: Dimension::Length(40.0),
                ..style(Display::Block)
            },
        );
        run_at_generation(&mut dom, parent, 100.0, 3);
        let published = get_layout(&dom, item);
        assert_eq!(
            (published.content.origin.x, published.layout_generation),
            (left, 3)
        );
        CALLS.with_borrow(|calls| {
            let last = calls.last().unwrap();
            assert_eq!((last.returned_origin.x, last.generation), (0.0, 3));
        });
    }
}

#[test]
fn rtl_rows_preserve_the_physical_auto_margin_side() {
    use elidex_plugin::{Direction, FlexDirection};
    for direction in [FlexDirection::Row, FlexDirection::RowReverse] {
        for left in [false, true] {
            let mut dom = EcsDom::new();
            let parent = node(
                &mut dom,
                None,
                ComputedStyle {
                    flex_direction: direction,
                    direction: Direction::Rtl,
                    ..style(Display::Flex)
                },
            );
            let item = node(
                &mut dom,
                Some(parent),
                ComputedStyle {
                    width: Dimension::Length(40.0),
                    margin_left: if left {
                        Dimension::Auto
                    } else {
                        Dimension::ZERO
                    },
                    margin_right: if left {
                        Dimension::ZERO
                    } else {
                        Dimension::Auto
                    },
                    ..style(Display::Block)
                },
            );
            run(&mut dom, parent, 200.0);
            let lb = get_layout(&dom, item);
            let expected = if left { 160.0 } else { 0.0 };
            assert_eq!(
                (lb.margin.left, lb.margin.right, lb.content.origin.x),
                (expected, 160.0 - expected, expected)
            );
        }
    }
}

#[test]
fn stretch_respects_cross_margins_and_clamps_used_content() {
    for (child_height, auto, max, min, expected, share) in [
        (30.0, true, Dimension::Auto, 0.0, 30.0, 20.0),
        (30.0, false, Dimension::Length(40.0), 0.0, 40.0, 0.0),
        (90.0, false, Dimension::Auto, 0.0, 70.0, 0.0),
        (30.0, false, Dimension::Auto, 80.0, 80.0, 0.0),
    ] {
        let mut dom = EcsDom::new();
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                height: Dimension::Length(70.0),
                ..style(Display::Flex)
            },
        );
        let edge = if auto {
            Dimension::Auto
        } else {
            Dimension::ZERO
        };
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                width: Dimension::Length(40.0),
                min_height: Dimension::Length(min),
                max_height: max,
                margin_top: edge,
                margin_bottom: edge,
                ..style(Display::Block)
            },
        );
        node(
            &mut dom,
            Some(item),
            ComputedStyle {
                height: Dimension::Length(child_height),
                ..style(Display::Block)
            },
        );
        let percentage_child = node(
            &mut dom,
            Some(item),
            ComputedStyle {
                height: Dimension::Percentage(50.0),
                ..style(Display::Block)
            },
        );
        run(&mut dom, parent, 100.0);
        let lb = get_layout(&dom, item);
        assert_eq!(
            get_layout(&dom, percentage_child).content.size.height,
            if auto { 0.0 } else { expected / 2.0 }
        );
        assert_eq!(
            (lb.content.size.height, lb.margin.top, lb.margin.bottom),
            (expected, share, share)
        );
    }
}

#[test]
fn fully_assigned_replaced_flex_item_skips_its_dom_children() {
    for position in [Position::Static, Position::Relative] {
        let mut dom = EcsDom::new();
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                position: Position::Relative,
                ..style(Display::Flex)
            },
        );
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                position,
                width: Dimension::Length(60.0),
                height: Dimension::Length(30.0),
                ..style(Display::Block)
            },
        );
        dom.world_mut().insert_one(
            item,
            elidex_ecs::ImageData {
                width: 60,
                height: 30,
                pixels: std::sync::Arc::new(vec![0; 7200]),
            },
        );
        let child = node(&mut dom, Some(item), style(Display::Block));
        let abs = node(
            &mut dom,
            Some(item),
            ComputedStyle {
                position: Position::Absolute,
                width: Dimension::Length(10.0),
                height: Dimension::Length(10.0),
                ..style(Display::Block)
            },
        );
        run(&mut dom, parent, 100.0);
        for descendant in [child, abs] {
            assert!(dom.world().get::<&LayoutBox>(descendant).is_err());
        }
        assert_eq!(get_layout(&dom, item).content.size, Size::new(60.0, 30.0));
    }
}

#[test]
fn floated_child_contributes_to_flex_item_cross_measurement() {
    let mut dom = EcsDom::new();
    let parent = node(&mut dom, None, style(Display::Flex));
    let item = node(&mut dom, Some(parent), style(Display::Block));
    node(
        &mut dom,
        Some(item),
        ComputedStyle {
            float: elidex_plugin::Float::Left,
            width: Dimension::Length(10.0),
            height: Dimension::Length(30.0),
            ..style(Display::Block)
        },
    );
    run(&mut dom, parent, 100.0);
    assert_eq!(get_layout(&dom, item).content.size.height, 30.0);
    assert_eq!(get_layout(&dom, parent).content.size.height, 30.0);
}

#[test]
fn vertical_rl_receiver_uses_physical_left_and_top_once() {
    for multicol in [false, true] {
        let mut dom = EcsDom::new();
        let parent = node(&mut dom, None, style(Display::Flex));
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                writing_mode: elidex_plugin::WritingMode::VerticalRl,
                width: Dimension::Length(40.0),
                height: Dimension::Length(70.0),
                column_count: multicol.then_some(1),
                margin_left: Dimension::Auto,
                margin_top: Dimension::Length(3.0),
                padding: EdgeSizes {
                    left: Dimension::Length(5.0),
                    right: Dimension::Length(1.0),
                    ..Default::default()
                },
                ..style(Display::Block)
            },
        );
        let child = node(
            &mut dom,
            Some(item),
            ComputedStyle {
                width: Dimension::Length(10.0),
                height: Dimension::Length(10.0),
                ..style(Display::Block)
            },
        );
        run(&mut dom, parent, 100.0);
        let lb = get_layout(&dom, item);
        assert_eq!(lb.margin_box().origin.x, 0.0);
        assert_eq!(lb.content.origin, Point::new(59.0, 3.0));
        assert!(get_layout(&dom, child).content.origin.x >= lb.content.origin.x);
    }
}

#[test]
fn empty_vertical_table_keeps_each_main_only_assignment_on_its_axis() {
    use elidex_plugin::FlexDirection;
    for (direction, width, height, minimum) in [
        (FlexDirection::Row, 100.0, 0.0, 0.0),
        (FlexDirection::Column, 0.0, 80.0, 0.0),
        (FlexDirection::Column, 40.0, 80.0, 40.0),
    ] {
        let mut dom = EcsDom::new();
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                flex_direction: direction,
                align_items: elidex_plugin::AlignItems::FlexStart,
                height: Dimension::Length(80.0),
                ..style(Display::Flex)
            },
        );
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                writing_mode: elidex_plugin::WritingMode::VerticalRl,
                flex_basis: FlexBasis::Length(if direction == FlexDirection::Column {
                    height
                } else {
                    width
                }),
                min_width: Dimension::Length(minimum),
                min_height: Dimension::ZERO,
                ..style(Display::Table)
            },
        );
        run(&mut dom, parent, 100.0);
        CALLS.with_borrow(|calls| {
            let cross = calls
                .iter()
                .find(|c| {
                    c.probe
                        && c.assigned.content.width.is_some() != c.assigned.content.height.is_some()
                })
                .unwrap();
            assert_eq!(cross.size, Size::new(width, height));
        });
        assert_eq!(
            get_layout(&dom, item).content.size,
            Size::new(width, height)
        );
    }
}

#[test]
fn empty_vertical_table_direct_receiver_clamps_only_the_missing_axis() {
    for (preferred, min, max, expected) in [
        (Dimension::Auto, 40.0, 90.0, 40.0),
        (Dimension::Length(100.0), 10.0, 60.0, 60.0),
    ] {
        let mut dom = EcsDom::new();
        let table = node(
            &mut dom,
            None,
            ComputedStyle {
                writing_mode: elidex_plugin::WritingMode::VerticalRl,
                width: preferred,
                min_width: Dimension::Length(min),
                max_width: Dimension::Length(max),
                max_height: Dimension::Length(10.0),
                ..style(Display::Table)
            },
        );
        let fonts = FontDatabase::new();
        let input = LayoutInput {
            available_height_if_indefinite: None,
            containing_width_definite: true,
            item_context: elidex_layout_block::ItemContext::NormalFlow,
            assigned: UsedAssignment {
                content: Size {
                    width: None,
                    height: Some(80.0),
                },
                ..UsedAssignment::default()
            },
            containing: CssSize::width_only(200.0),
            containing_inline_size: 200.0,
            offset: Point::ZERO,
            font_db: &fonts,
            depth: 0,
            float_ctx: None,
            viewport: None,
            fragmentainer: None,
            break_token: None,
            subgrid: None,
            layout_generation: 0,
            medium: elidex_plugin::Medium::Screen,
            placement: elidex_layout_block::FragmentPlacement::Normal,
            is_probe: false,
        };
        let result =
            elidex_layout_table::layout_table(&mut dom, table, &input, dispatch_layout_child);
        assert_eq!(result.layout_box.content.size, Size::new(expected, 80.0));
    }
}

#[test]
fn vertical_block_percentage_width_requires_a_definite_flex_cross_size() {
    use elidex_plugin::{FlexDirection, WritingMode};
    for (auto_margin, expected_width) in [(true, 0.0), (false, 50.0)] {
        let mut dom = EcsDom::new();
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                flex_direction: FlexDirection::Column,
                height: Dimension::Length(80.0),
                width: Dimension::Length(100.0),
                ..style(Display::Flex)
            },
        );
        let edge = if auto_margin {
            Dimension::Auto
        } else {
            Dimension::ZERO
        };
        let vertical = ComputedStyle {
            writing_mode: WritingMode::VerticalRl,
            ..style(Display::Block)
        };
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                height: Dimension::Length(80.0),
                margin_left: edge,
                margin_right: edge,
                ..vertical.clone()
            },
        );
        node(
            &mut dom,
            Some(item),
            ComputedStyle {
                width: Dimension::Length(30.0),
                ..vertical.clone()
            },
        );
        let child = node(
            &mut dom,
            Some(item),
            ComputedStyle {
                width: Dimension::Percentage(50.0),
                ..vertical
            },
        );
        run(&mut dom, parent, 100.0);
        assert_eq!(get_layout(&dom, child).content.size.width, expected_width);
        let lb = get_layout(&dom, item);
        assert_eq!(
            lb.content.size.width,
            if auto_margin { 30.0 } else { 100.0 }
        );
        assert_eq!(lb.margin.left, if auto_margin { 35.0 } else { 0.0 });
    }
}

#[test]
fn vertical_auto_height_uses_available_space_with_indefinite_percentage_reference() {
    use elidex_plugin::{AlignItems, WritingMode};
    for child_height in [Dimension::Auto, Dimension::Percentage(50.0)] {
        let mut dom = EcsDom::new();
        let parent = node(
            &mut dom,
            None,
            ComputedStyle {
                align_items: AlignItems::FlexStart,
                ..style(Display::Flex)
            },
        );
        let vertical = ComputedStyle {
            writing_mode: WritingMode::VerticalRl,
            ..style(Display::Block)
        };
        let item = node(
            &mut dom,
            Some(parent),
            ComputedStyle {
                width: Dimension::Length(100.0),
                max_height: Dimension::Length(20.0),
                ..vertical.clone()
            },
        );
        let child = node(
            &mut dom,
            Some(item),
            ComputedStyle {
                height: child_height,
                ..vertical
            },
        );
        run(&mut dom, parent, 100.0);
        assert_eq!(get_layout(&dom, item).content.size.height, 20.0);
        assert_eq!(get_layout(&dom, child).content.size.height, 20.0);
    }
}
