use super::*;
use elidex_layout_block::UsedAssignment;
use elidex_plugin::{BoxSizing, EdgeSizes, FlexBasis, GridTrackList, TrackSize};
use std::cell::RefCell;

struct Call {
    entity: Entity,
    probe: bool,
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
    let style = elidex_layout_block::get_style(dom, entity);
    let result = dispatch_layout_child(dom, entity, input);
    CALLS.with_borrow_mut(|calls| {
        calls.push(Call {
            entity,
            probe: input.is_probe,
            assigned: input.assigned,
            origin: input.offset,
            size: result.layout_box.content.size,
            style,
        });
    });
    result
}

fn run(dom: &mut EcsDom, parent: Entity, width: f32) {
    CALLS.with_borrow_mut(Vec::clear);
    let fonts = FontDatabase::new();
    let input = LayoutInput {
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
        layout_generation: 0,
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
                assigned: UsedAssignment {
                    content: Size {
                        width: Some(0.0),
                        height: Some(20.0),
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
