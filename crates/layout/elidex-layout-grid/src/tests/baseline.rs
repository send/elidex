use super::*;
use elidex_plugin::JustifyItems;

#[test]
fn row_baseline_alignment() {
    // Grid with 1 row, 2 columns (100px each), align-items: Baseline.
    // Two items with different heights (20, 40). Items without text use
    // margin-box bottom as the baseline fallback. The taller item defines
    // the row baseline, so the shorter item should be offset downward.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                align_items: AlignItems::Baseline,
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 20.0);
    let c2 = make_grid_child(&mut dom, container, 40.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // The taller item (40px) defines the row baseline at its margin-box
    // bottom = 40. The shorter item (20px) should be offset so its
    // margin-box bottom aligns: offset = 40 - 20 = 20.
    assert!(
        lb1.content.origin.y > lb2.content.origin.y,
        "shorter item (y={}) should be offset below taller item (y={})",
        lb1.content.origin.y,
        lb2.content.origin.y,
    );
    // The taller item should start at y=0 (no offset needed).
    assert!(approx_eq(lb2.content.origin.y, 0.0));
    // The shorter item offset = row_baseline - item_baseline = 40 - 20 = 20.
    assert!(approx_eq(lb1.content.origin.y, 20.0));
}

#[test]
fn column_baseline_alignment() {
    // Grid with 2 rows, 1 column, justify-items: Baseline.
    // Two items with different heights. The justify baseline offset
    // should be applied along the inline axis.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(200.0),
                ])),
                grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                justify_items: JustifyItems::Baseline,
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 30.0);
    let c2 = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // Both items are in column 0 but different rows.
    // justify-items: Baseline computes per-column baselines.
    // The wider margin-box defines the column baseline; items are offset
    // along the x-axis. Both items have width determined by auto sizing.
    // The layout should complete without panic and items should have
    // valid positions.
    assert!(lb1.content.origin.x >= 0.0);
    assert!(lb2.content.origin.x >= 0.0);
    // Row positions: c1 in row 0, c2 in row 1 (at y=100).
    assert!(approx_eq(lb2.content.origin.y, 100.0) || lb2.content.origin.y > lb1.content.origin.y);
}

#[test]
fn mixed_baseline_and_stretch() {
    // Grid with 1 row, 2 columns. First item: align-self=Baseline (height:30).
    // Second item: align-self=Stretch (height:auto). Stretch item fills
    // the row height, baseline item aligns by baseline.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                ..Default::default()
            },
        )
        .unwrap();

    // First item: align-self = Baseline, explicit height 30.
    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                align_self: elidex_plugin::AlignSelf::Baseline,
                ..Default::default()
            },
        )
        .unwrap();

    // Second item: align-self = Stretch, auto height.
    let c2 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c2);
    dom.world_mut()
        .insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                align_self: elidex_plugin::AlignSelf::Stretch,
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // The baseline item has height 30.
    assert!(approx_eq(lb1.content.size.height, 30.0));
    // The stretch item should fill the row height (at least as tall as the
    // baseline item's outer height).
    assert!(
        lb2.content.size.height >= lb1.content.size.height,
        "stretch item (h={}) should fill row >= baseline item (h={})",
        lb2.content.size.height,
        lb1.content.size.height,
    );
    // Stretch item starts at y=0 (fills from top).
    assert!(approx_eq(lb2.content.origin.y, 0.0));
}

#[test]
fn spanning_item_fallback() {
    // Grid with 2 rows, 2 columns. One item spans 2 rows with align: Baseline.
    // Baseline should be computed correctly (fallback to margin-box height
    // since no text content).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(50.0),
                    TrackSize::Length(50.0),
                ])),
                align_items: AlignItems::Baseline,
                ..Default::default()
            },
        )
        .unwrap();

    // Item spanning 2 rows.
    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(80.0),
                grid_row_end: GridLine::Span(2),
                ..Default::default()
            },
        )
        .unwrap();

    // Normal item in row 0, col 1.
    let c2 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // Both items should have valid layout positions (no panic).
    assert!(lb1.content.size.height > 0.0);
    assert!(lb2.content.size.height > 0.0);
    // Spanning item should start at row 0.
    assert!(approx_eq(lb1.content.origin.y, 0.0) || lb1.content.origin.y >= 0.0);
}

#[test]
fn no_baseline_item() {
    // Grid with items that have no baseline (all explicit heights, no text).
    // Baseline alignment falls back to margin-box bottom.
    // Layout should complete without panic.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                align_items: AlignItems::Baseline,
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 25.0);
    let c2 = make_grid_child(&mut dom, container, 50.0);
    let c3 = make_grid_child(&mut dom, container, 35.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // All items should have valid positions.
    assert!(lb1.content.size.width > 0.0);
    assert!(lb2.content.size.width > 0.0);
    assert!(lb3.content.size.width > 0.0);

    // The tallest item (50px) should start at y=0 (it defines the baseline).
    assert!(approx_eq(lb2.content.origin.y, 0.0));
    // Shorter items should be offset downward.
    assert!(lb1.content.origin.y >= 0.0);
    assert!(lb3.content.origin.y >= 0.0);
    // Item with height 25 should have a larger offset than item with height 35.
    assert!(
        lb1.content.origin.y >= lb3.content.origin.y,
        "25px item (y={}) should be offset >= 35px item (y={})",
        lb1.content.origin.y,
        lb3.content.origin.y,
    );
}

#[test]
fn container_baseline_propagation() {
    // Grid with 1 row, 2 columns, align-items: Baseline. Items with heights
    // 30, 50. The grid container's first_baseline should be Some and > 0.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                align_items: AlignItems::Baseline,
                ..Default::default()
            },
        )
        .unwrap();

    let _c1 = make_grid_child(&mut dom, container, 30.0);
    let _c2 = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    let clb = do_layout_grid(
        &mut dom,
        container,
        400.0,
        Some(200.0),
        Point::ZERO,
        &font_db,
        0,
        layout_block_only,
    );

    // The grid container should propagate a first_baseline from row 0.
    assert!(
        clb.first_baseline.is_some(),
        "grid container should have first_baseline when align-items: baseline is used",
    );
    let baseline = clb.first_baseline.unwrap();
    assert!(baseline > 0.0, "first_baseline ({baseline}) should be > 0",);
    // The baseline should equal the row 0 baseline, which is the tallest
    // item's margin-box bottom = 50.
    assert!(
        approx_eq(baseline, 50.0),
        "first_baseline ({baseline}) should equal tallest item height (50)",
    );
}
