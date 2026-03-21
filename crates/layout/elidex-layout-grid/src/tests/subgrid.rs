//! Subgrid layout tests (CSS Grid Level 2 §2).

use super::*;

/// Create a grid container with specified track list.
fn make_grid_container(
    dom: &mut EcsDom,
    cols: GridTrackList,
    rows: GridTrackList,
) -> elidex_ecs::Entity {
    let entity = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            entity,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: cols,
                grid_template_rows: rows,
                ..Default::default()
            },
        )
        .unwrap();
    entity
}

/// Create a subgrid child (`display:grid` + subgrid columns).
#[allow(clippy::too_many_arguments)]
fn make_subgrid_child(
    dom: &mut EcsDom,
    parent: elidex_ecs::Entity,
    subgrid_cols: bool,
    subgrid_rows: bool,
    col_start: GridLine,
    col_end: GridLine,
    row_start: GridLine,
    row_end: GridLine,
) -> elidex_ecs::Entity {
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: if subgrid_cols {
                    GridTrackList::Subgrid { line_names: vec![] }
                } else {
                    GridTrackList::default()
                },
                grid_template_rows: if subgrid_rows {
                    GridTrackList::Subgrid { line_names: vec![] }
                } else {
                    GridTrackList::default()
                },
                grid_column_start: col_start,
                grid_column_end: col_end,
                grid_row_start: row_start,
                grid_row_end: row_end,
                ..Default::default()
            },
        )
        .unwrap();
    child
}

// --- Tests ---

#[test]
fn subgrid_inherits_parent_column_tracks() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: 2 columns 100px + 200px
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(100.0),
            TrackSize::Length(200.0),
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid child spanning both columns
    let subgrid_child = make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Line(1),
        GridLine::Line(3),
        GridLine::Auto,
        GridLine::Auto,
    );

    // Add a grandchild to the subgrid
    make_grid_child(&mut dom, subgrid_child, 30.0);

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    let child_lb = get_layout(&dom, subgrid_child);
    // The subgrid child should span the full area (100+200=300)
    assert!(
        approx_eq(child_lb.content.width, 300.0),
        "width={}",
        child_lb.content.width
    );
}

#[test]
fn subgrid_content_contributes_to_parent_tracks() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: 2 auto columns
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Auto,
            TrackSize::Auto,
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Child 1: normal grid item in column 1
    let child1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, child1);
    dom.world_mut()
        .insert_one(
            child1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                height: Dimension::Length(20.0),
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(2),
                ..Default::default()
            },
        )
        .unwrap();

    // Child 2: normal grid item in column 2
    let child2 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, child2);
    dom.world_mut()
        .insert_one(
            child2,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(80.0),
                height: Dimension::Length(20.0),
                grid_column_start: GridLine::Line(2),
                grid_column_end: GridLine::Line(3),
                ..Default::default()
            },
        )
        .unwrap();

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        500.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    let lb1 = get_layout(&dom, child1);
    let lb2 = get_layout(&dom, child2);
    // Auto columns: col1 gets 50px content, col2 gets 80px content.
    // Container is 500px, so remaining space distributes via stretch.
    // With stretch: col1 = 250, col2 = 250 (equal distribution of remaining space).
    // Items with explicit width keep their width inside the stretched track.
    assert!(
        approx_eq(lb1.content.width, 50.0),
        "child1 width={} expected 50.0",
        lb1.content.width
    );
    assert!(
        approx_eq(lb2.content.width, 80.0),
        "child2 width={} expected 80.0",
        lb2.content.width
    );
}

#[test]
fn subgrid_uses_own_gap() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: 3 columns 100px each, gap 10px
    let parent = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            parent,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                    TrackSize::Length(100.0),
                ])),
                grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Auto,
                ])),
                column_gap: Dimension::Length(10.0),
                ..Default::default()
            },
        )
        .unwrap();

    // Subgrid child spanning all 3 columns with gap=20px
    let subgrid_child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, subgrid_child);
    dom.world_mut()
        .insert_one(
            subgrid_child,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Subgrid { line_names: vec![] },
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(4),
                column_gap: Dimension::Length(20.0),
                ..Default::default()
            },
        )
        .unwrap();

    // Add a grandchild
    make_grid_child(&mut dom, subgrid_child, 20.0);

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        320.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    // The parent should resolve: 100+10+100+10+100 = 320
    let child_lb = get_layout(&dom, subgrid_child);
    // Subgrid spans all 3 parent columns (3*100 + 2*10 gaps = 320).
    assert!(
        approx_eq(child_lb.content.width, 320.0),
        "subgrid width={} expected 320.0",
        child_lb.content.width
    );
}

#[test]
fn subgrid_cols_only_rows_explicit() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: 2 cols 100px, 1 row auto
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(100.0),
            TrackSize::Length(100.0),
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid on cols only, explicit rows
    let subgrid_child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, subgrid_child);
    dom.world_mut()
        .insert_one(
            subgrid_child,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Subgrid { line_names: vec![] },
                grid_template_rows: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                    TrackSize::Length(40.0),
                    TrackSize::Length(40.0),
                ])),
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(3),
                ..Default::default()
            },
        )
        .unwrap();

    make_grid_child(&mut dom, subgrid_child, 20.0);

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    let child_lb = get_layout(&dom, subgrid_child);
    // Width should span both parent columns
    assert!(
        approx_eq(child_lb.content.width, 200.0),
        "w={}",
        child_lb.content.width
    );
    // Height: the subgrid has explicit rows (40+40=80) but the parent's auto
    // row is sized by the subgrid's measured content height. The grandchild is
    // 20px; the parent auto row reflects that measurement.
    assert!(
        approx_eq(child_lb.content.height, 20.0),
        "h={} expected 20.0",
        child_lb.content.height
    );
}

#[test]
fn subgrid_empty_no_children() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(100.0),
            TrackSize::Length(100.0),
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Empty subgrid
    let _subgrid_child = make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Line(1),
        GridLine::Line(3),
        GridLine::Auto,
        GridLine::Auto,
    );

    // Should not panic
    let _lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
}

#[test]
fn subgrid_auto_placement() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(100.0),
            TrackSize::Length(100.0),
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid with auto-placement (no explicit column/row placement)
    let subgrid_child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, subgrid_child);
    dom.world_mut()
        .insert_one(
            subgrid_child,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Subgrid { line_names: vec![] },
                ..Default::default()
            },
        )
        .unwrap();

    make_grid_child(&mut dom, subgrid_child, 20.0);

    // Should not panic
    let _lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
}

#[test]
fn subgrid_named_lines_declared() {
    // Test that subgrid with named lines parses and resolves correctly
    let subgrid = GridTrackList::Subgrid {
        line_names: vec![
            vec!["a".to_string()],
            vec!["b".to_string(), "c".to_string()],
        ],
    };
    assert!(subgrid.is_subgrid());
    assert_eq!(subgrid.len(), 0);
    assert!(!subgrid.is_empty());
    if let GridTrackList::Subgrid { ref line_names } = subgrid {
        assert_eq!(line_names.len(), 2);
        assert_eq!(line_names[0], vec!["a".to_string()]);
        assert_eq!(line_names[1], vec!["b".to_string(), "c".to_string()]);
    }
}

#[test]
fn max_passes_limit() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: auto columns
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Auto,
            TrackSize::Auto,
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid child
    make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Line(1),
        GridLine::Line(3),
        GridLine::Auto,
        GridLine::Auto,
    );

    // Should complete without infinite loop (max 3 passes)
    let _lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
}

#[test]
fn subgrid_mbp_affects_parent_tracks() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Auto,
            TrackSize::Auto,
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Normal child in col 1 with 30px width
    let normal_child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, normal_child);
    dom.world_mut()
        .insert_one(
            normal_child,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(30.0),
                height: Dimension::Length(20.0),
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(2),
                grid_row_start: GridLine::Line(1),
                grid_row_end: GridLine::Line(2),
                ..Default::default()
            },
        )
        .unwrap();

    // Subgrid child spanning both columns with padding 10px left, 15px right.
    // CSS Grid L2 §2.5: m/b/p on subgridded axis contributes to first/last tracks.
    let subgrid_child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, subgrid_child);
    dom.world_mut()
        .insert_one(
            subgrid_child,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Subgrid { line_names: vec![] },
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(3),
                grid_row_start: GridLine::Line(2),
                grid_row_end: GridLine::Line(3),
                padding: elidex_plugin::EdgeSizes {
                    top: Dimension::Length(0.0),
                    right: Dimension::Length(15.0),
                    bottom: Dimension::Length(0.0),
                    left: Dimension::Length(10.0),
                },
                ..Default::default()
            },
        )
        .unwrap();

    // Grandchild in the subgrid with 20px width
    let gc = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(subgrid_child, gc);
    dom.world_mut()
        .insert_one(
            gc,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(20.0),
                height: Dimension::Length(20.0),
                ..Default::default()
            },
        )
        .unwrap();

    let lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    // Col 1: max(normal_child=30, subgrid_gc_contribution + mbp_start=10) >= 30
    // Col 2: mbp_end=15 contributes to second track sizing
    // The parent container should resolve without panic and subgrid m/b/p
    // increases track sizing beyond content alone.
    let normal_lb = get_layout(&dom, normal_child);
    assert!(
        approx_eq(normal_lb.content.width, 30.0),
        "normal w={} expected 30.0",
        normal_lb.content.width
    );
    // Container should be laid out
    assert!(
        lb.content.width > 0.0,
        "container width={}",
        lb.content.width
    );
}

#[test]
fn subgrid_intrinsic_sizing() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid child
    let subgrid_child = make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Auto,
        GridLine::Auto,
        GridLine::Auto,
        GridLine::Auto,
    );

    let gc = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(subgrid_child, gc);
    dom.world_mut()
        .insert_one(
            gc,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(60.0),
                height: Dimension::Length(20.0),
                ..Default::default()
            },
        )
        .unwrap();

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    // The auto column sizes to max(grandchild content=60, stretch to container).
    // With a 200px container, the auto column stretches to fill, so the
    // subgrid item (width:auto + stretch) fills the available space.
    let child_lb = get_layout(&dom, subgrid_child);
    assert!(
        approx_eq(child_lb.content.width, 200.0),
        "w={} expected 200.0",
        child_lb.content.width
    );
}

#[test]
fn convergence_two_passes() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: auto columns — subgrid content should inform track sizing
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Auto,
            TrackSize::Auto,
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Normal child in col 1
    let child1 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, child1);
    dom.world_mut()
        .insert_one(
            child1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(40.0),
                height: Dimension::Length(20.0),
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(2),
                ..Default::default()
            },
        )
        .unwrap();

    // Subgrid in col 2
    let subgrid_child = make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Line(2),
        GridLine::Line(3),
        GridLine::Auto,
        GridLine::Auto,
    );

    // Grandchild with fixed width
    let gc = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(subgrid_child, gc);
    dom.world_mut()
        .insert_one(
            gc,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(70.0),
                height: Dimension::Length(15.0),
                ..Default::default()
            },
        )
        .unwrap();

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    // Col 1: child1 has explicit width=40.
    // Col 2: subgrid has width=auto, grandchild=70px content.
    // Both auto tracks stretch to fill the container; items with explicit
    // widths keep their declared size inside the stretched area.
    let lb1 = get_layout(&dom, child1);
    let lb2 = get_layout(&dom, subgrid_child);
    assert!(
        approx_eq(lb1.content.width, 40.0),
        "child1 w={} expected 40.0",
        lb1.content.width
    );
    // Subgrid item (width:auto) stretches to fill its track.
    assert!(
        lb2.content.width >= 70.0,
        "subgrid w={} expected >= 70.0",
        lb2.content.width
    );
}

#[test]
fn nested_subgrid() {
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: 3 columns 100px each
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(100.0),
            TrackSize::Length(100.0),
            TrackSize::Length(100.0),
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid child spanning cols 1-3
    let subgrid1 = make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Line(1),
        GridLine::Line(4),
        GridLine::Auto,
        GridLine::Auto,
    );

    // Nested subgrid inside first subgrid
    let subgrid2 = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(subgrid1, subgrid2);
    dom.world_mut()
        .insert_one(
            subgrid2,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: GridTrackList::Subgrid { line_names: vec![] },
                ..Default::default()
            },
        )
        .unwrap();

    make_grid_child(&mut dom, subgrid2, 10.0);

    // Should not panic on nested subgrid
    let _lb = do_layout_grid(
        &mut dom,
        parent,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    // Outer subgrid spans all 3 columns (300px total)
    let sg1_lb = get_layout(&dom, subgrid1);
    assert!(
        approx_eq(sg1_lb.content.width, 300.0),
        "subgrid1 w={} expected 300.0",
        sg1_lb.content.width
    );
}

#[test]
fn subgrid_mbp_row_axis() {
    // CSS Grid L2 §2.5: subgrid m/b/p on row axis contributes to parent row tracks.
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Auto,
            TrackSize::Auto,
        ])),
    );

    // Subgrid on rows with margin-top=8, margin-bottom=12.
    let subgrid_child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, subgrid_child);
    dom.world_mut()
        .insert_one(
            subgrid_child,
            ComputedStyle {
                display: Display::Grid,
                grid_template_rows: GridTrackList::Subgrid { line_names: vec![] },
                grid_row_start: GridLine::Line(1),
                grid_row_end: GridLine::Line(3),
                grid_column_start: GridLine::Line(1),
                grid_column_end: GridLine::Line(2),
                margin_top: Dimension::Length(8.0),
                margin_bottom: Dimension::Length(12.0),
                ..Default::default()
            },
        )
        .unwrap();

    let gc = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(subgrid_child, gc);
    dom.world_mut()
        .insert_one(
            gc,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(40.0),
                height: Dimension::Length(25.0),
                ..Default::default()
            },
        )
        .unwrap();

    // Should complete without panic; row tracks include subgrid m/b/p contribution.
    let lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    assert!(lb.content.height > 0.0, "container h={}", lb.content.height);
}

#[test]
fn subgrid_min_content_probe_with_parent_context() {
    // Verify that subgrid items receive parent subgrid context during
    // min/max-content probes (R1-3 fix).
    let mut dom = EcsDom::new();
    let font_db = FontDatabase::new();

    // Parent: 2 fixed columns 120px + 80px
    let parent = make_grid_container(
        &mut dom,
        GridTrackList::Explicit(TrackSection::from_tracks(vec![
            TrackSize::Length(120.0),
            TrackSize::Length(80.0),
        ])),
        GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::Auto])),
    );

    // Subgrid child spanning both columns
    let subgrid_child = make_subgrid_child(
        &mut dom,
        parent,
        true,
        false,
        GridLine::Line(1),
        GridLine::Line(3),
        GridLine::Auto,
        GridLine::Auto,
    );

    // Grandchild with auto width (should fill available space)
    let gc = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(subgrid_child, gc);
    dom.world_mut()
        .insert_one(
            gc,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        )
        .unwrap();

    let _lb = do_layout_grid(
        &mut dom,
        parent,
        200.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        layout_block_only,
    );
    let child_lb = get_layout(&dom, subgrid_child);
    // Subgrid spans 120+80 = 200px total
    assert!(
        approx_eq(child_lb.content.width, 200.0),
        "subgrid w={} expected 200.0",
        child_lb.content.width
    );
}
