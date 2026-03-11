use super::*;

#[test]
fn grid_explicit_placement() {
    // grid-column: 2 / 4 places item in columns 1-2 (0-based).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(50.0),
                grid_column_start: GridLine::Line(2),
                grid_column_end: GridLine::Line(4),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // Should start at column 1 (x=100) and span 2 columns (width=200).
    assert!(approx_eq(lb.content.x, 100.0));
    assert!(approx_eq(lb.content.width, 200.0));
}

#[test]
fn grid_span_placement() {
    // grid-column: span 2 -> item spans 2 columns.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                grid_column_end: GridLine::Span(2),
                ..Default::default()
            },
        )
        .unwrap();

    let c2 = make_grid_child(&mut dom, container, 40.0);

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // c1 spans columns 0-1 (200px), c2 goes to column 2 (100px).
    assert!(approx_eq(lb1.content.width, 200.0));
    assert!(approx_eq(lb1.content.x, 0.0));
    assert!(approx_eq(lb2.content.x, 200.0));
    assert!(approx_eq(lb2.content.width, 100.0));
}

#[test]
fn grid_auto_placement_row() {
    // Default flow is row -- items fill columns left to right, then wrap to next row.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(100.0), TrackSize::Length(100.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 40.0);
    let c2 = make_grid_child(&mut dom, container, 40.0);
    let c3 = make_grid_child(&mut dom, container, 40.0);

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // Row 0: c1(0,0) c2(0,1), Row 1: c3(1,0)
    assert!(approx_eq(lb1.content.x, 0.0));
    assert!(approx_eq(lb1.content.y, 0.0));
    assert!(approx_eq(lb2.content.x, 100.0));
    assert!(approx_eq(lb2.content.y, 0.0));
    assert!(approx_eq(lb3.content.x, 0.0));
    assert!(approx_eq(lb3.content.y, 40.0));
}

#[test]
fn grid_auto_placement_column() {
    // column flow -- items fill rows top to bottom, then wrap to next column.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(100.0), TrackSize::Length(100.0)],
                grid_template_rows: vec![TrackSize::Length(40.0), TrackSize::Length(40.0)],
                grid_auto_flow: GridAutoFlow::Column,
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 30.0);
    let c2 = make_grid_child(&mut dom, container, 30.0);
    let c3 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // Column flow: c1(0,0) c2(1,0) c3(0,1)
    assert!(approx_eq(lb1.content.x, 0.0));
    assert!(approx_eq(lb1.content.y, 0.0));
    assert!(approx_eq(lb2.content.x, 0.0));
    assert!(approx_eq(lb2.content.y, 40.0));
    assert!(approx_eq(lb3.content.x, 100.0));
    assert!(approx_eq(lb3.content.y, 0.0));
}

#[test]
fn grid_dense_placement() {
    // Dense packing should fill gaps.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ],
                grid_auto_flow: GridAutoFlow::RowDense,
                ..Default::default()
            },
        )
        .unwrap();

    // Item 1: spans 2 columns.
    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                grid_column_end: GridLine::Span(2),
                ..Default::default()
            },
        )
        .unwrap();

    // Item 2: spans 2 columns (wraps to next row, leaving a gap at (0,2)).
    let c2 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c2);
    dom.world_mut()
        .insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                grid_column_end: GridLine::Span(2),
                ..Default::default()
            },
        )
        .unwrap();

    // Item 3: single column -- dense should fill the gap at (0,2).
    let c3 = make_grid_child(&mut dom, container, 40.0);

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb3 = get_layout(&dom, c3);

    // Dense placement: c3 should be placed at (0,2) to fill the gap.
    assert!(approx_eq(lb3.content.x, 200.0));
    assert!(approx_eq(lb3.content.y, 0.0));
}

#[test]
fn grid_order_property() {
    // The order property should affect visual placement order.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    // Create items with reversed order.
    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                order: 3,
                ..Default::default()
            },
        )
        .unwrap();

    let c2 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c2);
    dom.world_mut()
        .insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                order: 1,
                ..Default::default()
            },
        )
        .unwrap();

    let c3 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c3);
    dom.world_mut()
        .insert_one(
            c3,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                order: 2,
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // order: c2(1) -> col 0, c3(2) -> col 1, c1(3) -> col 2.
    assert!(approx_eq(lb2.content.x, 0.0));
    assert!(approx_eq(lb3.content.x, 100.0));
    assert!(approx_eq(lb1.content.x, 200.0));
}

#[test]
fn grid_negative_line_number() {
    // grid-column-start: -1 means the last grid line (after all explicit columns).
    // With 3 explicit columns, line -1 = line 4 (0-based index 3).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    // Place item at last column using negative line number.
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                grid_column_start: GridLine::Line(-1),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // Line -1 with 3 explicit cols = line 4 (0-based 3), so start at index 3
    // but there are only columns 0,1,2 -- so the item goes into an implicit col.
    // With the current algorithm: resolve_line(-1, 3) = 3 + (-1) + 1 = 3.
    // An item starting at index 3 spans 1 implicit column.
    // The explicit columns occupy x=0..300. Item starts at x=300.
    assert!(approx_eq(lb.content.x, 300.0));
}

#[test]
fn grid_negative_line_start_end() {
    // grid-column: -3 / -1 -> spans the last 2 explicit columns.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                grid_column_start: GridLine::Line(-3),
                grid_column_end: GridLine::Line(-1),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // -3 with 3 explicit cols: 3 + (-3) + 1 = 1 (0-based index 1)
    // -1 with 3 explicit cols: 3 + (-1) + 1 = 3 (0-based index 3)
    // Span = 3 - 1 = 2 columns -> width = 200px, starts at x=100.
    assert!(approx_eq(lb.content.x, 100.0));
    assert!(approx_eq(lb.content.width, 200.0));
}

#[test]
fn grid_extreme_line_number_capped() {
    // Extreme grid line numbers should be capped to prevent OOM.
    // grid-column-start: 1000000 should be capped to MAX_GRID_INDEX (10000).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(100.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                grid_column_start: GridLine::Line(1_000_000),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    // Should not OOM -- the line number is capped.
    let lb = layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    // Container should still produce a valid LayoutBox with finite dimensions.
    assert!(
        lb.content.height.is_finite() && lb.content.height >= 0.0,
        "extreme line: height={} should be finite non-negative",
        lb.content.height
    );
    assert!(
        approx_eq(lb.content.width, 400.0),
        "extreme line: width={} should match container width 400",
        lb.content.width
    );
}

#[test]
fn grid_extreme_span_capped() {
    // Extreme span values should be capped to prevent OOM.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(100.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                grid_column_end: GridLine::Span(1_000_000),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    // Should not OOM -- the span is capped.
    let lb = layout_grid(
        &mut dom,
        container,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    assert!(
        lb.content.height.is_finite() && lb.content.height >= 0.0,
        "extreme span: height={} should be finite non-negative",
        lb.content.height
    );
    assert!(
        approx_eq(lb.content.width, 400.0),
        "extreme span: width={} should match container width 400",
        lb.content.width
    );
}
