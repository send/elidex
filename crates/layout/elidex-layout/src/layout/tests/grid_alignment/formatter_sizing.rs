//! The formatter reconciles the proposed content size before Grid aligns its box.
use super::*;
use boundaries::layout_authored;
use elidex_plugin::{ColumnSpan, FlexWrap, GridTrackList, TrackSize};
use Dimension::Length;

fn layout(dom: &mut EcsDom, root: Entity, paged: bool) {
    layout_authored(dom, root, Size::new(500., 500.), paged);
}

fn container(dom: &mut EcsDom, height: f32) -> Entity {
    let track =
        |size| GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Length(size)]));
    add(
        dom,
        None,
        ComputedStyle {
            display: Display::Grid,
            grid_template_columns: track(100.),
            grid_template_rows: track(height),
            ..Default::default()
        },
    )
}

#[test]
fn inline_image_reconciles_its_auto_axis_with_the_authored_opposite_axis() {
    for paged in [false, true] {
        for (mode, width, height) in [
            (WritingMode::HorizontalTb, Dimension::Auto, Length(60.)),
            (WritingMode::VerticalRl, Length(120.), Dimension::Auto),
        ] {
            let mut dom = EcsDom::new();
            let image = dom.create_element_ownerless_fixture("img", Attributes::default());
            dom.world_mut()
                .insert(
                    image,
                    (
                        ComputedStyle {
                            display: Display::InlineBlock,
                            writing_mode: mode,
                            width,
                            height,
                            ..Default::default()
                        },
                        elidex_ecs::ImageData {
                            width: 60,
                            height: 30,
                            pixels: std::sync::Arc::new(vec![0; 60 * 30 * 4]),
                        },
                    ),
                )
                .unwrap();
            layout(&mut dom, image, paged);
            assert_eq!(get_layout(&dom, image).content.size, Size::new(120., 60.));
        }
    }
}

#[test]
fn table_rows_determine_used_height_before_grid_distributes_auto_margins() {
    for paged in [false, true] {
        for (area, margin, inset) in [(100., Length(0.), 0.), (160., Dimension::Auto, 30.)] {
            let mut dom = EcsDom::new();
            let grid = container(&mut dom, area);
            let table = add(
                &mut dom,
                Some(grid),
                ComputedStyle {
                    display: Display::Table,
                    height: Length(40.),
                    margin_top: margin,
                    margin_bottom: margin,
                    ..Default::default()
                },
            );
            let row = add(
                &mut dom,
                Some(table),
                ComputedStyle {
                    display: Display::TableRow,
                    ..Default::default()
                },
            );
            let cell = add(
                &mut dom,
                Some(row),
                ComputedStyle {
                    display: Display::TableCell,
                    height: Length(100.),
                    ..Default::default()
                },
            );
            layout(&mut dom, grid, paged);
            let lb = get_layout(&dom, table);
            assert_eq!(lb.content.size.height, 100.);
            assert_eq!(get_layout(&dom, cell).content.size.height, 100.);
            assert_eq!(
                (lb.content.origin.y, lb.margin.top, lb.margin.bottom),
                (inset, inset, inset)
            );
        }
    }
}

#[test]
fn flex_lines_multicol_spanners_and_grid_rows_supply_their_own_auto_height() {
    for paged in [false, true] {
        for display in [Display::Flex, Display::Block, Display::Grid] {
            let mut dom = EcsDom::new();
            let grid = container(&mut dom, 100.);
            let item = add(
                &mut dom,
                Some(grid),
                ComputedStyle {
                    display,
                    align_self: AlignSelf::End,
                    flex_wrap: FlexWrap::Wrap,
                    column_count: (display == Display::Block).then_some(2),
                    ..Default::default()
                },
            );
            for height in [20., 35.] {
                add(
                    &mut dom,
                    Some(item),
                    ComputedStyle {
                        display: Display::Block,
                        width: Length(80.),
                        height: Length(height),
                        column_span: ColumnSpan::All,
                        ..Default::default()
                    },
                );
            }
            layout(&mut dom, grid, paged);
            let lb = get_layout(&dom, item);
            assert_eq!(
                (lb.content.origin.y, lb.content.size.height),
                (45., 55.),
                "{display:?}, paged={paged}"
            );
        }
    }
}
