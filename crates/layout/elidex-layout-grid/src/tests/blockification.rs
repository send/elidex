//! Tests for Grid §6.1 item blockification.

use super::*;

fn grid_container() -> ComputedStyle {
    ComputedStyle {
        display: Display::Grid,
        grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(200.0),
        ])),
        ..Default::default()
    }
}

#[test]
fn grid_item_inline_blockified() {
    let container = grid_container();
    let mut dom = EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(grid, container);
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(grid, child);
    let _ = dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::Inline,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        grid,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let style = dom
        .world()
        .get::<&ComputedStyle>(child)
        .map(|s| s.display)
        .unwrap();
    assert_eq!(
        style,
        Display::Block,
        "inline should be blockified to block in grid"
    );
}

#[test]
fn grid_item_inline_flex_to_flex() {
    let container = grid_container();
    let mut dom = EcsDom::new();
    let grid = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(grid, container);
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(grid, child);
    let _ = dom.world_mut().insert_one(
        child,
        ComputedStyle {
            display: Display::InlineFlex,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        grid,
        400.0,
        None,
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let style = dom
        .world()
        .get::<&ComputedStyle>(child)
        .map(|s| s.display)
        .unwrap();
    assert_eq!(
        style,
        Display::Flex,
        "inline-flex should be blockified to flex in grid"
    );
}
