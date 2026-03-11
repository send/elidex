use super::*;

#[test]
fn grid_empty_container() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
                height: Dimension::Length(100.0),
                ..Default::default()
            },
        )
        .unwrap();

    let font_db = FontDatabase::new();
    let lb = do_layout_grid(
        &mut dom,
        container,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    assert!(approx_eq(lb.content.width, 800.0));
    assert!(approx_eq(lb.content.height, 100.0));
}

#[test]
fn column_track_sizing() {
    let font_db = FontDatabase::new();
    let cases: &[TrackSizingCase] = &[
        (
            "two fixed-px columns (200+300)",
            &[TrackSize::Length(200.0), TrackSize::Length(300.0)],
            800.0,
            &[(0.0, 200.0), (200.0, 300.0)],
        ),
        (
            "three equal 1fr columns in 900px",
            &[TrackSize::Fr(1.0), TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
            900.0,
            &[(0.0, 300.0), (300.0, 300.0), (600.0, 300.0)],
        ),
        (
            "100px + 1fr + 200px in 600px",
            &[
                TrackSize::Length(100.0),
                TrackSize::Fr(1.0),
                TrackSize::Length(200.0),
            ],
            600.0,
            &[(0.0, 100.0), (100.0, 300.0), (400.0, 200.0)],
        ),
        (
            "1fr + 2fr in 900px",
            &[TrackSize::Fr(1.0), TrackSize::Fr(2.0)],
            900.0,
            &[(0.0, 300.0), (300.0, 600.0)],
        ),
        (
            "100px + 1fr + 1fr in 800px",
            &[
                TrackSize::Length(100.0),
                TrackSize::Fr(1.0),
                TrackSize::Fr(1.0),
            ],
            800.0,
            &[(0.0, 100.0), (100.0, 350.0), (450.0, 350.0)],
        ),
        (
            "50% + 50% in 600px",
            &[TrackSize::Percentage(50.0), TrackSize::Percentage(50.0)],
            600.0,
            &[(0.0, 300.0), (300.0, 300.0)],
        ),
        (
            "0.25fr + 0.25fr (sum < 1) in 400px",
            &[TrackSize::Fr(0.25), TrackSize::Fr(0.25)],
            400.0,
            &[(0.0, 100.0), (100.0, 100.0)],
        ),
    ];

    for (desc, tracks, container_w, expected) in cases {
        let mut dom = EcsDom::new();
        let container = dom.create_element("div", Attributes::default());
        dom.world_mut()
            .insert_one(
                container,
                ComputedStyle {
                    display: Display::Grid,
                    grid_template_columns: tracks.to_vec(),
                    ..Default::default()
                },
            )
            .unwrap();

        let children: Vec<_> = (0..expected.len())
            .map(|_| make_grid_child(&mut dom, container, 50.0))
            .collect();

        do_layout_grid(
            &mut dom,
            container,
            *container_w,
            None,
            0.0,
            0.0,
            &font_db,
            0,
            layout_block_only,
        );

        for (i, &(expected_x, expected_w)) in expected.iter().enumerate() {
            let child_lb = get_layout(&dom, children[i]);
            assert!(
                approx_eq(child_lb.content.x, expected_x),
                "{desc}: child[{i}] x={} expected {expected_x}",
                child_lb.content.x,
            );
            assert!(
                approx_eq(child_lb.content.width, expected_w),
                "{desc}: child[{i}] width={} expected {expected_w}",
                child_lb.content.width,
            );
        }
    }
}

#[test]
fn grid_auto_rows() {
    // 2-column grid with 4 items -> 2 auto rows.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 50.0);
    let _c2 = make_grid_child(&mut dom, container, 50.0);
    let c3 = make_grid_child(&mut dom, container, 30.0);
    let _c4 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    let lb = do_layout_grid(
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
    let lb3 = get_layout(&dom, c3);

    // Row 0: y=0, Row 1: y=50.
    assert!(approx_eq(lb1.content.y, 0.0));
    assert!(approx_eq(lb3.content.y, 50.0));
    // Container height = 50 + 30 = 80.
    assert!(approx_eq(lb.content.height, 80.0));
}

#[test]
fn grid_explicit_rows() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_template_rows: vec![TrackSize::Length(100.0), TrackSize::Length(200.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 30.0);
    let c2 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    let lb = do_layout_grid(
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

    // Row 0: 100px, Row 1: 200px.
    assert!(approx_eq(lb1.content.y, 0.0));
    assert!(approx_eq(lb2.content.y, 100.0));
    assert!(approx_eq(lb.content.height, 300.0));
}

#[test]
fn grid_auto_track_size() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_auto_rows: TrackSize::Length(50.0),
                ..Default::default()
            },
        )
        .unwrap();

    let _c1 = make_grid_child(&mut dom, container, 20.0);
    let c2 = make_grid_child(&mut dom, container, 20.0);
    let c3 = make_grid_child(&mut dom, container, 20.0);

    let font_db = FontDatabase::new();
    let lb = do_layout_grid(
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

    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // Auto rows should be 50px each.
    assert!(approx_eq(lb2.content.y, 50.0));
    assert!(approx_eq(lb3.content.y, 100.0));
    assert!(approx_eq(lb.content.height, 150.0));
}

#[test]
fn grid_container_auto_height() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let _c1 = make_grid_child(&mut dom, container, 100.0);
    let _c2 = make_grid_child(&mut dom, container, 200.0);

    let font_db = FontDatabase::new();
    let lb = do_layout_grid(
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

    // Auto height = sum of row heights = 100 + 200 = 300.
    assert!(approx_eq(lb.content.height, 300.0));
}

#[test]
fn grid_minmax_track() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![
                    TrackSize::MinMax(
                        Box::new(TrackBreadth::Length(100.0)),
                        Box::new(TrackBreadth::Fr(1.0)),
                    ),
                    TrackSize::Length(200.0),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 30.0);
    let c2 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // minmax(100px, 1fr): gets remaining space after 200px -> 400px.
    // (But must be at least 100px.)
    assert!(approx_eq(lb1.content.width, 400.0));
    assert!(approx_eq(lb2.content.width, 200.0));
}

#[test]
fn grid_percentage_row_indefinite_height() {
    // CSS Grid 7.2.1: percentage row tracks with indefinite container height
    // should behave like auto (use content size).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_template_rows: vec![TrackSize::Percentage(50.0)],
                // No explicit height -> indefinite.
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 80.0);

    let font_db = FontDatabase::new();
    let clb = do_layout_grid(
        &mut dom,
        container,
        400.0,
        None, // Indefinite containing height.
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb = get_layout(&dom, child);

    // With indefinite height, 50% row should behave like auto -> use content height (80px).
    assert!(approx_eq(lb.content.height, 80.0));
    assert!(approx_eq(clb.content.height, 80.0));
}
