//! Grid layout tests.

use elidex_ecs::{Attributes, EcsDom};
use elidex_layout_block::layout_block_only;
use elidex_plugin::{
    AlignItems, ComputedStyle, Dimension, Display, GridAutoFlow, GridLine, LayoutBox, TrackBreadth,
    TrackSize,
};
use elidex_text::FontDatabase;

use crate::layout_grid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_layout(dom: &EcsDom, entity: elidex_ecs::Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}

fn make_grid_child(
    dom: &mut EcsDom,
    parent: elidex_ecs::Entity,
    height: f32,
) -> elidex_ecs::Entity {
    let child = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(parent, child);
    dom.world_mut()
        .insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(height),
                ..Default::default()
            },
        )
        .unwrap();
    child
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

// (description, tracks, container_width, expected (x, width) per child)
type TrackSizingCase = (
    &'static str,
    &'static [TrackSize],
    f32,
    &'static [(f32, f32)],
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    let lb = layout_grid(
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

        layout_grid(
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
    // 2-column grid with 4 items → 2 auto rows.
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

    let lb2 = get_layout(&dom, c2);
    let lb3 = get_layout(&dom, c3);

    // Auto rows should be 50px each.
    assert!(approx_eq(lb2.content.y, 50.0));
    assert!(approx_eq(lb3.content.y, 100.0));
    assert!(approx_eq(lb.content.height, 150.0));
}

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
    // grid-column: span 2 → item spans 2 columns.
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
    // Default flow is row — items fill columns left to right, then wrap to next row.
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
    // column flow — items fill rows top to bottom, then wrap to next column.
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
fn grid_gap_between_items() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
                column_gap: 20.0,
                row_gap: 10.0,
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 50.0);
    let c2 = make_grid_child(&mut dom, container, 50.0);
    let c3 = make_grid_child(&mut dom, container, 30.0);

    let font_db = FontDatabase::new();
    let clb = layout_grid(
        &mut dom,
        container,
        420.0,
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

    // 420 - 20 (gap) = 400 / 2 = 200 each.
    assert!(approx_eq(lb1.content.width, 200.0));
    assert!(approx_eq(lb2.content.width, 200.0));
    // Column gap: c2 starts at 200 + 20 = 220.
    assert!(approx_eq(lb2.content.x, 220.0));
    // Row gap: c3 at y = 50 + 10 = 60.
    assert!(approx_eq(lb3.content.y, 60.0));
    // Container height: 50 + 10 + 30 = 90.
    assert!(approx_eq(clb.content.height, 90.0));
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

    // Auto height = sum of row heights = 100 + 200 = 300.
    assert!(approx_eq(lb.content.height, 300.0));
}

#[test]
fn grid_align_items_center() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_template_rows: vec![TrackSize::Length(100.0)],
                align_items: AlignItems::Center,
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 40.0);

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

    // Centered in 100px row: (100 - 40) / 2 = 30.
    assert!(approx_eq(lb.content.y, 30.0));
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
    layout_grid(
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

    // minmax(100px, 1fr): gets remaining space after 200px → 400px.
    // (But must be at least 100px.)
    assert!(approx_eq(lb1.content.width, 400.0));
    assert!(approx_eq(lb2.content.width, 200.0));
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

    // Item 3: single column — dense should fill the gap at (0,2).
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
fn grid_with_padding_border() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                padding_top: 10.0,
                padding_right: 10.0,
                padding_bottom: 10.0,
                padding_left: 10.0,
                border_top_width: 5.0,
                border_right_width: 5.0,
                border_bottom_width: 5.0,
                border_left_width: 5.0,
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    let clb = layout_grid(
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

    // Content area starts after padding+border: 10+5=15.
    assert!(approx_eq(clb.content.x, 15.0));
    assert!(approx_eq(clb.content.y, 15.0));
    // Content width: 400 - 2*(10+5) = 370.
    assert!(approx_eq(clb.content.width, 370.0));
    // Child should fill the grid.
    assert!(approx_eq(lb.content.width, 370.0));
}

#[test]
fn grid_item_margin() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(200.0)],
                grid_template_rows: vec![TrackSize::Length(100.0)],
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
                margin_top: Dimension::Length(10.0),
                margin_left: Dimension::Length(20.0),
                margin_right: Dimension::Length(20.0),
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

    // Item starts at margin offset: x=20, y=10.
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 10.0));
    // Width: 200 - 20 - 20 = 160.
    assert!(approx_eq(lb.content.width, 160.0));
}

#[test]
fn grid_box_sizing_border_box() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                width: Dimension::Length(400.0),
                box_sizing: elidex_plugin::BoxSizing::BorderBox,
                padding_left: 20.0,
                padding_right: 20.0,
                border_left_width: 5.0,
                border_right_width: 5.0,
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 50.0);

    let font_db = FontDatabase::new();
    let clb = layout_grid(
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

    let lb = get_layout(&dom, child);

    // border-box: content = 400 - 2*(20+5) = 350.
    assert!(approx_eq(clb.content.width, 350.0));
    assert!(approx_eq(lb.content.width, 350.0));
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
    // but there are only columns 0,1,2 — so the item goes into an implicit col.
    // With the current algorithm: resolve_line(-1, 3) = 3 + (-1) + 1 = 3.
    // An item starting at index 3 spans 1 implicit column.
    // The explicit columns occupy x=0..300. Item starts at x=300.
    assert!(approx_eq(lb.content.x, 300.0));
}

#[test]
fn grid_negative_line_start_end() {
    // grid-column: -3 / -1 → spans the last 2 explicit columns.
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
    // Span = 3 - 1 = 2 columns → width = 200px, starts at x=100.
    assert!(approx_eq(lb.content.x, 100.0));
    assert!(approx_eq(lb.content.width, 200.0));
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

    // order: c2(1) → col 0, c3(2) → col 1, c1(3) → col 2.
    assert!(approx_eq(lb2.content.x, 0.0));
    assert!(approx_eq(lb3.content.x, 100.0));
    assert!(approx_eq(lb1.content.x, 200.0));
}

#[test]
fn grid_align_self_stretch_with_center_container() {
    // align-self: stretch should stretch the item even when container has align-items: center.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Fr(1.0)],
                grid_template_rows: vec![TrackSize::Length(100.0)],
                align_items: AlignItems::Center,
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
                // height is auto — eligible for stretch.
                align_self: elidex_plugin::AlignSelf::Stretch,
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

    // align-self: stretch should override align-items: center.
    // Item should fill the 100px row (starts at y=0, height=100).
    assert!(approx_eq(lb.content.y, 0.0));
    assert!(approx_eq(lb.content.height, 100.0));
}

#[test]
fn grid_percentage_row_indefinite_height() {
    // CSS Grid §7.2.1: percentage row tracks with indefinite container height
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
                // No explicit height → indefinite.
                ..Default::default()
            },
        )
        .unwrap();

    let child = make_grid_child(&mut dom, container, 80.0);

    let font_db = FontDatabase::new();
    let clb = layout_grid(
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

    // With indefinite height, 50% row should behave like auto → use content height (80px).
    assert!(approx_eq(lb.content.height, 80.0));
    assert!(approx_eq(clb.content.height, 80.0));
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
    // Should not OOM — the line number is capped.
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
    // Should not OOM — the span is capped.
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

#[test]
fn grid_negative_track_size_clamped() {
    // Negative track sizes (from malformed CSS) should be clamped to 0.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    dom.world_mut()
        .insert_one(
            container,
            ComputedStyle {
                display: Display::Grid,
                grid_template_columns: vec![TrackSize::Length(-50.0), TrackSize::Length(200.0)],
                ..Default::default()
            },
        )
        .unwrap();

    let c1 = make_grid_child(&mut dom, container, 30.0);
    let c2 = make_grid_child(&mut dom, container, 30.0);

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

    // Negative track should be clamped to 0.
    assert!(lb1.content.width >= 0.0);
    assert!(lb2.content.width >= 0.0);
    // The second column (200px) should still work correctly.
    assert!(approx_eq(lb2.content.width, 200.0));
}
