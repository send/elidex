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
                grid_template_columns: GridTrackList::Explicit(vec![
                    TrackSize::Fr(1.0),
                    TrackSize::Fr(1.0),
                ]),
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
                    grid_template_columns: GridTrackList::Explicit(tracks.to_vec()),
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
                grid_template_columns: GridTrackList::Explicit(vec![
                    TrackSize::Fr(1.0),
                    TrackSize::Fr(1.0),
                ]),
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
                grid_template_columns: GridTrackList::Explicit(vec![TrackSize::Fr(1.0)]),
                grid_template_rows: GridTrackList::Explicit(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(200.0),
                ]),
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
                grid_template_columns: GridTrackList::Explicit(vec![TrackSize::Fr(1.0)]),
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
                grid_template_columns: GridTrackList::Explicit(vec![TrackSize::Fr(1.0)]),
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
                grid_template_columns: GridTrackList::Explicit(vec![
                    TrackSize::MinMax(
                        Box::new(TrackBreadth::Length(100.0)),
                        Box::new(TrackBreadth::Fr(1.0)),
                    ),
                    TrackSize::Length(200.0),
                ]),
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
                grid_template_columns: GridTrackList::Explicit(vec![TrackSize::Fr(1.0)]),
                grid_template_rows: GridTrackList::Explicit(vec![TrackSize::Percentage(50.0)]),
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

#[test]
fn grid_minmax_min_content_uses_narrow_size() {
    // minmax(min-content, 1fr) should use min-content for the base size,
    // not max-content. With a child that has a fixed width, min-content
    // and max-content are the same, so we test with a small fixed child
    // to verify the base is not inflated.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(vec![
                    TrackSize::MinMax(
                        Box::new(TrackBreadth::MinContent),
                        Box::new(TrackBreadth::Fr(1.0)),
                    ),
                    TrackSize::Length(200.0),
                ]),
                ..Default::default()
            },
        )
        .unwrap();

    // Child in first column has a small fixed width.
    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        )
        .unwrap();

    let _c2 = make_grid_child(&mut dom, container, 30.0);

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

    let _lb1 = get_layout(&dom, c1);

    // minmax(min-content, 1fr): base = min-content (50px child width),
    // max = 1fr gets remaining space (600 - 200 = 400px).
    // Since 1fr resolves to 400px and that's > base, track size = 400.
    // The child itself has explicit width 50px, so check via grid area position.
    // c1 is at x=0, c2 starts at x=400 (track1 position).
    let lb2_entity = dom.composed_children(container)[1];
    let lb2 = get_layout(&dom, lb2_entity);
    assert!(
        approx_eq(lb2.content.x, 400.0),
        "c2 should start at x=400 (track 0 = 400px), got {}",
        lb2.content.x
    );
}

#[test]
fn grid_minmax_max_content_in_max() {
    // minmax(100px, max-content) should use max-content for the limit.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(vec![
                    TrackSize::MinMax(
                        Box::new(TrackBreadth::Length(100.0)),
                        Box::new(TrackBreadth::MaxContent),
                    ),
                    TrackSize::Fr(1.0),
                ]),
                ..Default::default()
            },
        )
        .unwrap();

    // Child in first column has a medium fixed width (150px).
    let c1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(container, c1);
    dom.world_mut()
        .insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(150.0),
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        )
        .unwrap();

    let _c2 = make_grid_child(&mut dom, container, 30.0);

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

    // minmax(100px, max-content): base=100, limit=max-content(150).
    // Track size = max(base, min(limit, max(base, content))) = max(100, min(150, 150)) = 150.
    assert!(
        approx_eq(lb1.content.width, 150.0),
        "expected 150px (max-content limit), got {}",
        lb1.content.width
    );
}

// ---------------------------------------------------------------------------
// auto-fill / auto-fit tests
// ---------------------------------------------------------------------------

#[test]
fn auto_fill_200px_in_900px() {
    // repeat(auto-fill, 200px) in 900px container -> floor(900/200) = 4 tracks.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::AutoRepeat {
                    before: vec![],
                    pattern: vec![TrackSize::Length(200.0)],
                    mode: elidex_plugin::AutoRepeatMode::AutoFill,
                    after: vec![],
                },
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 50.0);
    let c2 = make_grid_child(&mut dom, container, 50.0);
    let c3 = make_grid_child(&mut dom, container, 50.0);
    let c4 = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        900.0,
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
    let lb4 = get_layout(&dom, c4);

    // 4 tracks of 200px each.
    assert!(approx_eq(lb1.content.x, 0.0));
    assert!(approx_eq(lb1.content.width, 200.0));
    assert!(approx_eq(lb2.content.x, 200.0));
    assert!(approx_eq(lb3.content.x, 400.0));
    assert!(approx_eq(lb4.content.x, 600.0));
}

#[test]
fn auto_fill_multi_pattern_in_900px() {
    // repeat(auto-fill, 100px 200px) in 900px -> floor(900/300) = 3 reps = 6 tracks.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::AutoRepeat {
                    before: vec![],
                    pattern: vec![TrackSize::Length(100.0), TrackSize::Length(200.0)],
                    mode: elidex_plugin::AutoRepeatMode::AutoFill,
                    after: vec![],
                },
                ..Default::default()
            },
        )
        .unwrap();

    let children: Vec<_> = (0..6)
        .map(|_| make_grid_child(&mut dom, container, 50.0))
        .collect();

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        900.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    // 3 repetitions: 100 200 100 200 100 200
    let lb0 = get_layout(&dom, children[0]);
    let lb1 = get_layout(&dom, children[1]);
    let lb2 = get_layout(&dom, children[2]);

    assert!(
        approx_eq(lb0.content.width, 100.0),
        "child0 width={}",
        lb0.content.width
    );
    assert!(
        approx_eq(lb1.content.width, 200.0),
        "child1 width={}",
        lb1.content.width
    );
    assert!(
        approx_eq(lb2.content.x, 300.0),
        "child2 x={}",
        lb2.content.x
    );
}

#[test]
fn auto_fit_collapses_empty_tracks() {
    // repeat(auto-fit, 200px) with only 2 items in 900px -> 4 tracks,
    // but empty ones collapse their size to 0.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::AutoRepeat {
                    before: vec![],
                    pattern: vec![TrackSize::Length(200.0)],
                    mode: elidex_plugin::AutoRepeatMode::AutoFit,
                    after: vec![],
                },
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 50.0);
    let c2 = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    let _clb = do_layout_grid(
        &mut dom,
        container,
        900.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);
    let lb2 = get_layout(&dom, c2);

    // Items should be in first 2 tracks (200px each).
    assert!(approx_eq(lb1.content.x, 0.0));
    assert!(approx_eq(lb1.content.width, 200.0));
    assert!(approx_eq(lb2.content.x, 200.0));
    assert!(approx_eq(lb2.content.width, 200.0));
}

#[test]
fn auto_fill_minimum_one_repetition() {
    // repeat(auto-fill, 300px) in 250px container -> minimum 1 track.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::AutoRepeat {
                    before: vec![],
                    pattern: vec![TrackSize::Length(300.0)],
                    mode: elidex_plugin::AutoRepeatMode::AutoFill,
                    after: vec![],
                },
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        250.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb1 = get_layout(&dom, c1);

    // Even though 300px > 250px, minimum 1 track.
    assert!(approx_eq(lb1.content.width, 300.0));
}

#[test]
fn auto_fill_with_fixed_before_after() {
    // 100px repeat(auto-fill, 200px) 100px in 900px
    // Fixed: 100 + 100 = 200. Remaining: 700.
    // Each 200px pattern with gap=0 -> floor(700/200) = 3 repetitions.
    // Total: 100 + 3*200 + 100 = 800.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::AutoRepeat {
                    before: vec![TrackSize::Length(100.0)],
                    pattern: vec![TrackSize::Length(200.0)],
                    mode: elidex_plugin::AutoRepeatMode::AutoFill,
                    after: vec![TrackSize::Length(100.0)],
                },
                ..Default::default()
            },
        )
        .unwrap();

    // 5 items to fill: 100 + 200 + 200 + 200 + 100 = 5 tracks
    let children: Vec<_> = (0..5)
        .map(|_| make_grid_child(&mut dom, container, 50.0))
        .collect();

    let font_db = FontDatabase::new();
    do_layout_grid(
        &mut dom,
        container,
        900.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );

    let lb0 = get_layout(&dom, children[0]);
    let lb1 = get_layout(&dom, children[1]);
    let lb4 = get_layout(&dom, children[4]);

    // First track: 100px
    assert!(
        approx_eq(lb0.content.width, 100.0),
        "first track width={}",
        lb0.content.width
    );
    // Second track: 200px, starts at x=100
    assert!(
        approx_eq(lb1.content.x, 100.0),
        "second track x={}",
        lb1.content.x
    );
    assert!(
        approx_eq(lb1.content.width, 200.0),
        "second track width={}",
        lb1.content.width
    );
    // Last track: 100px, starts at x=100+600=700
    assert!(
        approx_eq(lb4.content.x, 700.0),
        "last track x={}",
        lb4.content.x
    );
    assert!(
        approx_eq(lb4.content.width, 100.0),
        "last track width={}",
        lb4.content.width
    );
}
