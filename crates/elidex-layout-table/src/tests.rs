//! Tests for the CSS table layout algorithm.

use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::{
    BorderCollapse, BorderStyle, ComputedStyle, Dimension, Display, LayoutBox, TableLayout,
    TextAlign,
};
use elidex_text::FontDatabase;

use crate::{algo, layout_table, CellInfo};

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0
}

/// Get the [`LayoutBox`] for an entity, panicking if none exists.
fn get_layout(dom: &EcsDom, entity: elidex_ecs::Entity) -> LayoutBox {
    dom.world()
        .get::<&LayoutBox>(entity)
        .map(|lb| (*lb).clone())
        .expect("LayoutBox not found")
}

/// A standalone child layout function for tests (block-only, no flex/grid dispatch).
#[allow(clippy::too_many_arguments)]
fn test_layout_child(
    dom: &mut EcsDom,
    entity: elidex_ecs::Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
) -> LayoutBox {
    elidex_layout_block::block::layout_block_inner(
        dom,
        entity,
        containing_width,
        containing_height,
        offset_x,
        offset_y,
        font_db,
        depth,
        test_layout_child,
    )
}

/// Helper: create a simple table with N rows and M cols.
/// Returns (dom, table, cells) where cells is a row-major vec of cell entities.
fn create_simple_table(
    rows: usize,
    cols: usize,
    table_style: ComputedStyle,
) -> (EcsDom, elidex_ecs::Entity, Vec<elidex_ecs::Entity>) {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, table_style);

    let mut cells = Vec::new();
    for _ in 0..rows {
        let tr = dom.create_element("tr", Attributes::default());
        dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        dom.append_child(table, tr);

        for _ in 0..cols {
            let td = dom.create_element("td", Attributes::default());
            dom.world_mut().insert_one(
                td,
                ComputedStyle {
                    display: Display::TableCell,
                    height: Dimension::Length(20.0),
                    ..Default::default()
                },
            );
            dom.append_child(tr, td);
            cells.push(td);
        }
    }

    (dom, table, cells)
}

fn default_table_style() -> ComputedStyle {
    ComputedStyle {
        display: Display::Table,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Basic table tests
// ---------------------------------------------------------------------------

#[test]
fn table_basic_two_columns() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
    // Both cells should exist and share the width equally.
    let lb0 = get_layout(&dom, cells[0]);
    let lb1 = get_layout(&dom, cells[1]);
    assert!(
        approx_eq(lb0.content.width, 200.0),
        "cell 0 width: {} expected 200",
        lb0.content.width
    );
    assert!(
        approx_eq(lb1.content.width, 200.0),
        "cell 1 width: {} expected 200",
        lb1.content.width
    );
}

#[test]
fn table_three_rows() {
    let (mut dom, table, _cells) = create_simple_table(3, 2, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    // Table should have some height from 3 rows.
    assert!(lb.content.height > 0.0);
}

#[test]
fn table_explicit_width() {
    let style = ComputedStyle {
        display: Display::Table,
        width: Dimension::Length(500.0),
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 500.0));
}

#[test]
fn table_auto_width() {
    let (mut dom, table, _) = create_simple_table(1, 2, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 600.0));
}

#[test]
fn table_border_spacing() {
    let style = ComputedStyle {
        display: Display::Table,
        border_spacing_h: 10.0,
        border_spacing_v: 10.0,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
}

#[test]
fn table_border_spacing_two_values() {
    let style = ComputedStyle {
        display: Display::Table,
        border_spacing_h: 5.0,
        border_spacing_v: 10.0,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(2, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    // Height should include vertical spacing.
    assert!(lb.content.height > 0.0);
}

#[test]
fn table_padding_border() {
    let style = ComputedStyle {
        display: Display::Table,
        padding_top: 5.0,
        padding_right: 5.0,
        padding_bottom: 5.0,
        padding_left: 5.0,
        border_top_width: 2.0,
        border_right_width: 2.0,
        border_bottom_width: 2.0,
        border_left_width: 2.0,
        border_top_style: BorderStyle::Solid,
        border_right_style: BorderStyle::Solid,
        border_bottom_style: BorderStyle::Solid,
        border_left_style: BorderStyle::Solid,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.padding.top, 5.0));
    assert!(approx_eq(lb.border.top, 2.0));
    // Content width = 400 - 2*5 - 2*2 = 386
    assert!(approx_eq(lb.content.width, 386.0));
}

#[test]
fn table_cell_padding() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            padding_top: 5.0,
            padding_right: 5.0,
            padding_bottom: 5.0,
            padding_left: 5.0,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let td_lb = get_layout(&dom, td);
    assert!(approx_eq(td_lb.padding.top, 5.0));
}

#[test]
fn table_colspan() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: one cell spanning 2 columns.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs = Attributes::default();
    attrs.set("colspan", "2");
    let td1 = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    // Row 2: two cells.
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let cell_r2c1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_r2c1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, cell_r2c1);
    let cell_r2c2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_r2c2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, cell_r2c2);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    // The spanning cell should exist and have a LayoutBox.
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    // The second row cells should also have layout boxes.
    assert!(dom.world().get::<&LayoutBox>(cell_r2c1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(cell_r2c2).is_ok());
}

#[test]
fn table_rowspan() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: cell with rowspan=2 + normal cell.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs = Attributes::default();
    attrs.set("rowspan", "2");
    let td1 = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td2);

    // Row 2: only one cell (col 0 occupied by rowspan).
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td3 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td3,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td3);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td3).is_ok());
}

#[test]
fn table_colspan_and_rowspan() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: cell(colspan=2, rowspan=2) + cell.
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut attrs = Attributes::default();
    attrs.set("colspan", "2");
    attrs.set("rowspan", "2");
    let td_span = dom.create_element("td", attrs);
    dom.world_mut().insert_one(
        td_span,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(40.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td_span);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    // Row 2: only one cell at col 2.
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td2);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    assert!(dom.world().get::<&LayoutBox>(td_span).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

#[test]
fn table_caption_top() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(table, caption);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let caption_lb = get_layout(&dom, caption);
    let td_lb = get_layout(&dom, td);
    // Caption should be above the table rows.
    assert!(caption_lb.content.y < td_lb.content.y);
}

#[test]
fn table_caption_bottom() {
    use elidex_plugin::CaptionSide;

    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let caption = dom.create_element("caption", Attributes::default());
    dom.world_mut().insert_one(
        caption,
        ComputedStyle {
            display: Display::TableCaption,
            caption_side: CaptionSide::Bottom,
            height: Dimension::Length(30.0),
            ..Default::default()
        },
    );
    dom.append_child(table, caption);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let caption_lb = get_layout(&dom, caption);
    let td_lb = get_layout(&dom, td);
    // Caption with caption-side: bottom should be below the table rows.
    assert!(
        caption_lb.content.y > td_lb.content.y,
        "caption y={} should be below td y={}",
        caption_lb.content.y,
        td_lb.content.y
    );
}

#[test]
fn table_header_body_footer() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // tbody
    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tbody);

    let tr_body = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_body,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr_body);

    let cell_body = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_body,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_body, cell_body);

    // thead (added after tbody in DOM, but should be rendered first).
    let thead = dom.create_element("thead", Attributes::default());
    dom.world_mut().insert_one(
        thead,
        ComputedStyle {
            display: Display::TableHeaderGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, thead);

    let tr_head = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_head,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(thead, tr_head);

    let cell_head = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_head,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_head, cell_head);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let head_lb = get_layout(&dom, cell_head);
    let body_lb = get_layout(&dom, cell_body);
    // Header row should come before body row.
    assert!(
        head_lb.content.y < body_lb.content.y,
        "thead should be before tbody: head_y={} body_y={}",
        head_lb.content.y,
        body_lb.content.y
    );
}

#[test]
fn table_empty_cells() {
    let (mut dom, table, _) = create_simple_table(1, 2, default_table_style());
    // Give cells zero height to simulate empty cells.
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic.
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_single_cell() {
    let (mut dom, table, cells) = create_simple_table(1, 1, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        300.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 300.0));
    assert!(dom.world().get::<&LayoutBox>(cells[0]).is_ok());
}

#[test]
fn table_nested_content() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    // Put a block element inside the cell.
    let inner_div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        inner_div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(40.0),
            ..Default::default()
        },
    );
    dom.append_child(td, inner_div);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let td_lb = get_layout(&dom, td);
    // Cell height should accommodate the inner div.
    assert!(td_lb.content.height >= 40.0);
}

#[test]
fn table_cell_align() {
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            text_align: TextAlign::Center,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic; text-align is inherited by cell content.
    assert!(dom.world().get::<&LayoutBox>(td).is_ok());
}

#[test]
fn table_row_background() {
    // Just ensure row background doesn't break layout.
    let (mut dom, table, _) = create_simple_table(2, 2, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn table_auto_column_sizing() {
    // Verify the auto column sizing produces reasonable widths.
    let (mut dom, table, cells) = create_simple_table(1, 3, default_table_style());
    // Give different heights to verify cells are properly laid out.
    for (i, &cell) in cells.iter().enumerate() {
        dom.world_mut().insert_one(
            cell,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(20.0 + i as f32 * 10.0),
                ..Default::default()
            },
        );
    }
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // All cells should have LayoutBoxes.
    for &cell in &cells {
        assert!(dom.world().get::<&LayoutBox>(cell).is_ok());
    }
}

#[test]
fn table_column_distribution() {
    let (mut dom, table, cells) = create_simple_table(1, 2, default_table_style());
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let c0 = get_layout(&dom, cells[0]);
    let c1 = get_layout(&dom, cells[1]);
    // In auto layout with equal content, columns should get roughly equal widths.
    assert!(approx_eq(c0.content.width, c1.content.width));
}

// ---------------------------------------------------------------------------
// border-collapse tests
// ---------------------------------------------------------------------------

#[test]
fn table_collapse_basic() {
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_spacing_h: 10.0, // should be ignored in collapse mode
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
}

#[test]
fn table_collapse_no_spacing() {
    // With border-collapse: collapse, border-spacing should be ignored.
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_spacing_h: 20.0,
        border_spacing_v: 20.0,
        ..Default::default()
    };
    let (mut dom, table, cells) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let c0 = get_layout(&dom, cells[0]);
    let c1 = get_layout(&dom, cells[1]);
    // In collapse mode, spacing should be 0, so cells should be adjacent.
    // c1 should start right after c0 (no spacing gap).
    let gap = c1.content.x - (c0.content.x + c0.content.width + c0.padding.right + c0.border.right);
    assert!(
        gap < 2.0,
        "gap between cells should be minimal in collapse mode, got {gap}"
    );
}

#[test]
fn table_collapse_border_merge() {
    // Test that wider border wins in collapse mode.
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_right_width: 4.0,
        border_right_style: BorderStyle::Solid,
        ..Default::default()
    };
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, style);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            border_right_width: 2.0,
            border_right_style: BorderStyle::Solid,
            ..Default::default()
        },
    );
    dom.append_child(tr, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            border_left_width: 6.0,
            border_left_style: BorderStyle::Solid,
            ..Default::default()
        },
    );
    dom.append_child(tr, td2);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic; border merge logic should work.
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

#[test]
fn table_collapse_outer_border() {
    let style = ComputedStyle {
        display: Display::Table,
        border_collapse: BorderCollapse::Collapse,
        border_top_width: 3.0,
        border_top_style: BorderStyle::Solid,
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 1, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_direct_cells_anonymous_row() {
    // Direct table-cell children of a table (without a tr) should be
    // wrapped in an anonymous row per CSS 2.1 §17.2.1.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Add cells directly to the table (no <tr>).
    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(table, td2);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    // Both cells should have been laid out (not silently dropped).
    assert!(
        dom.world().get::<&LayoutBox>(td1).is_ok(),
        "direct cell td1 should have LayoutBox"
    );
    assert!(
        dom.world().get::<&LayoutBox>(td2).is_ok(),
        "direct cell td2 should have LayoutBox"
    );
}

#[test]
fn table_rowspan_height_extension() {
    // When a rowspan cell is taller than the combined spanned rows,
    // the last spanned row should be extended.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // Row 1: td1 (rowspan=2, height=100) + td2 (height=20)
    let tr1 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr1,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr1);

    let mut td1_attrs = Attributes::default();
    td1_attrs.set("rowspan", "2");
    let td1 = dom.create_element("td", td1_attrs);
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr1, td2);

    // Row 2: td3 (height=20)
    let tr2 = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr2,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr2);

    let td3 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td3,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr2, td3);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        400.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let td1_lb = get_layout(&dom, td1);
    // The rowspan cell should span the full height of both rows.
    // Row 1 base height = 20, row 2 base height = 20 → total = 40.
    // But td1 needs 100, so row 2 should be extended by 80 → total = 100.
    assert!(
        td1_lb.content.height >= 100.0 - 1.0,
        "rowspan cell height should be at least 100, got {}",
        td1_lb.content.height
    );
}

// ---------------------------------------------------------------------------
// table-layout: fixed tests
// ---------------------------------------------------------------------------

#[test]
fn table_fixed_layout_basic() {
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(approx_eq(lb.content.width, 400.0));
}

#[test]
fn table_fixed_layout_explicit_widths() {
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, style);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            width: Dimension::Length(100.0),
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td1);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td2);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    assert!(dom.world().get::<&LayoutBox>(td1).is_ok());
    assert!(dom.world().get::<&LayoutBox>(td2).is_ok());
}

#[test]
fn table_fixed_layout_equal_distribution() {
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(400.0),
        ..Default::default()
    };
    let (mut dom, table, cells) = create_simple_table(1, 2, style);
    // No explicit widths on cells — should distribute equally.
    for &cell in &cells {
        dom.world_mut().insert_one(
            cell,
            ComputedStyle {
                display: Display::TableCell,
                height: Dimension::Length(20.0),
                ..Default::default()
            },
        );
    }
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let c0 = get_layout(&dom, cells[0]);
    let c1 = get_layout(&dom, cells[1]);
    assert!(approx_eq(c0.content.width, c1.content.width));
}

#[test]
fn table_fixed_overflow() {
    // Fixed layout where content exceeds column width — should not panic.
    let style = ComputedStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        width: Dimension::Length(100.0),
        ..Default::default()
    };
    let (mut dom, table, _) = create_simple_table(1, 2, style);
    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

// ---------------------------------------------------------------------------
// Safety / limits tests (R4-8)
// ---------------------------------------------------------------------------

#[test]
fn table_colspan_clamped_to_max() {
    // colspan="5000" should be clamped to remaining columns (max 1000 total).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    let _ = dom.append_child(table, tbody);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    let _ = dom.append_child(tbody, tr);

    // One cell with colspan=5000 (exceeds MAX_TABLE_COLS=1000)
    let mut td_attrs = Attributes::default();
    td_attrs.set("colspan", "5000");
    let td = dom.create_element("td", td_attrs);
    dom.world_mut().insert_one(
        td,
        ComputedStyle {
            display: Display::TableCell,
            ..Default::default()
        },
    );
    let _ = dom.append_child(tr, td);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic; layout succeeds with clamped colspan.
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_column_limit_overflow() {
    // 10 cells each with colspan=200 → 2000 columns requested, capped at 1000.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    let _ = dom.append_child(table, tbody);

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    let _ = dom.append_child(tbody, tr);

    for _ in 0..10 {
        let mut td_attrs = Attributes::default();
        td_attrs.set("colspan", "200");
        let td = dom.create_element("td", td_attrs);
        dom.world_mut().insert_one(
            td,
            ComputedStyle {
                display: Display::TableCell,
                ..Default::default()
            },
        );
        let _ = dom.append_child(tr, td);
    }

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        800.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    // Should not panic; columns capped at MAX_TABLE_COLS.
    assert!(dom.world().get::<&LayoutBox>(table).is_ok());
}

#[test]
fn table_collapse_grid_budget_exceeded() {
    // 1500 cols × 1500 rows exceeds MAX_COLLAPSE_GRID_CELLS (1M).
    // resolve_collapsed_borders should return default (zero) borders.
    let cells: Vec<CellInfo> = (0..4)
        .map(|i| CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: i % 2,
            row: i / 2,
            colspan: 1,
            rowspan: 1,
        })
        .collect();
    let styles: Vec<ComputedStyle> = vec![ComputedStyle::default(); 4];
    let table_style = ComputedStyle::default();

    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 1500, 1500);
    // Falls back to default borders (all zeros).
    assert_eq!(result.len(), 4);
    for cb in &result {
        assert!(
            cb.top == 0.0 && cb.right == 0.0 && cb.bottom == 0.0 && cb.left == 0.0,
            "Expected zero borders for budget-exceeded fallback"
        );
    }
}

#[test]
fn auto_column_widths_nan_infinity_sanitized() {
    // NaN, Infinity, and negative values in cell content widths should be sanitized.
    let cells = vec![
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 0,
            row: 0,
            colspan: 1,
            rowspan: 1,
        },
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 1,
            row: 0,
            colspan: 1,
            rowspan: 1,
        },
    ];
    let content_widths = vec![
        (f32::NAN, f32::INFINITY),  // col 0: both non-finite
        (-10.0, f32::NEG_INFINITY), // col 1: negative + neg-infinity
    ];
    let widths = algo::auto_column_widths(2, &cells, &content_widths, 200.0);
    assert_eq!(widths.len(), 2);
    for w in &widths {
        assert!(w.is_finite(), "Width should be finite, got {w}");
        assert!(*w >= 0.0, "Width should be non-negative, got {w}");
    }
    // Total should equal available since all intrinsic sizes are zero.
    let total: f32 = widths.iter().sum();
    assert!(approx_eq(total, 200.0));
}

// ---------------------------------------------------------------------------
// T-1: auto_column_widths branch coverage (min overflow / max+remainder / proportional)
// ---------------------------------------------------------------------------

fn make_cell(col: usize, row: usize) -> CellInfo {
    CellInfo {
        entity: elidex_ecs::Entity::DANGLING,
        col,
        row,
        colspan: 1,
        rowspan: 1,
    }
}

#[test]
fn auto_col_widths_branches() {
    // Each case: (description, content_widths_per_col, available_width, expected_widths)
    for (desc, content_widths, available, expected) in [
        (
            "min overflow: total_min (60+80=140) >= available (100) returns min widths",
            vec![(60.0, 200.0), (80.0, 300.0)],
            100.0,
            vec![60.0, 80.0],
        ),
        (
            "max with remainder: total_max (200) <= available (400), each gets 100 + (400-200)/2 = 200",
            vec![(50.0, 100.0), (50.0, 100.0)],
            400.0,
            vec![200.0, 200.0],
        ),
        (
            "proportional: total_min=60 < available=180 < total_max=300, fraction=0.5",
            vec![(20.0, 100.0), (40.0, 200.0)],
            180.0,
            vec![60.0, 120.0],
        ),
    ] {
        let num_cols = content_widths.len();
        let cells: Vec<_> = (0..num_cols).map(|c| make_cell(c, 0)).collect();
        let widths = algo::auto_column_widths(num_cols, &cells, &content_widths, available);
        assert_eq!(
            widths.len(),
            expected.len(),
            "{desc}: column count mismatch"
        );
        for (col, (got, want)) in widths.iter().zip(expected.iter()).enumerate() {
            assert!(
                approx_eq(*got, *want),
                "{desc}: col {col} width: got {got}, expected {want}"
            );
        }
    }
}

#[test]
fn auto_col_widths_spanning_cell() {
    // colspan=2 cell distributes evenly across 2 columns.
    let cells = vec![CellInfo {
        entity: elidex_ecs::Entity::DANGLING,
        col: 0,
        row: 0,
        colspan: 2,
        rowspan: 1,
    }];
    let content_widths = vec![(100.0, 200.0)];
    let widths = algo::auto_column_widths(2, &cells, &content_widths, 300.0);
    // min per col = 50, max per col = 100. total_max=200 <= 300.
    // Each gets 100 + (300-200)/2 = 150.
    assert_eq!(widths.len(), 2);
    assert!(approx_eq(widths[0], 150.0));
    assert!(approx_eq(widths[1], 150.0));
}

// ---------------------------------------------------------------------------
// T-2: resolve_collapsed_borders numeric validation
// ---------------------------------------------------------------------------

#[test]
fn collapse_border_numeric_top_bottom() {
    // 2-row, 1-col table. Top cell border-bottom=2, bottom cell border-top=6.
    // Top edge of cell0 resolves against table border-top (4px).
    // Bottom edge of cell0 resolves against cell1's border-top (6px > 2px).
    let cells = vec![make_cell(0, 0), make_cell(0, 1)];
    let styles = vec![
        ComputedStyle {
            border_bottom_width: 2.0,
            border_bottom_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_top_width: 6.0,
            border_top_style: BorderStyle::Solid,
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle {
        border_top_width: 4.0,
        border_top_style: BorderStyle::Solid,
        border_bottom_width: 1.0,
        border_bottom_style: BorderStyle::Solid,
        ..Default::default()
    };
    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 1, 2);
    assert_eq!(result.len(), 2);
    // cell0 top: max(cell0-top=0 none, table-top=4 solid) → 4.
    assert!(approx_eq(result[0].top, 4.0));
    // cell0 bottom: resolve(cell0-bottom=2 solid, cell1-top=6 solid) → 6 (wider wins).
    assert!(approx_eq(result[0].bottom, 6.0));
    // cell1 top: resolve(cell1-top=6 solid, cell0-bottom=2 solid) → 6.
    assert!(approx_eq(result[1].top, 6.0));
    // cell1 bottom: resolve(cell1-bottom=0 none, table-bottom=1 solid) → 1.
    assert!(approx_eq(result[1].bottom, 1.0));
}

#[test]
fn collapse_border_numeric_left_right() {
    // 1-row, 2-col table. Cell0 border-right=3, Cell1 border-left=5.
    let cells = vec![make_cell(0, 0), make_cell(1, 0)];
    let styles = vec![
        ComputedStyle {
            border_right_width: 3.0,
            border_right_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_left_width: 5.0,
            border_left_style: BorderStyle::Solid,
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle {
        border_left_width: 2.0,
        border_left_style: BorderStyle::Solid,
        border_right_width: 7.0,
        border_right_style: BorderStyle::Solid,
        ..Default::default()
    };
    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 2, 1);
    // cell0 left: resolve(cell0-left=0 none, table-left=2 solid) → 2.
    assert!(approx_eq(result[0].left, 2.0));
    // cell0 right: resolve(cell0-right=3, cell1-left=5) → 5 (wider wins).
    assert!(approx_eq(result[0].right, 5.0));
    // cell1 left: resolve(cell1-left=5, cell0-right=3) → 5.
    assert!(approx_eq(result[1].left, 5.0));
    // cell1 right: resolve(cell1-right=0 none, table-right=7 solid) → 7.
    assert!(approx_eq(result[1].right, 7.0));
}

#[test]
fn collapse_border_spanning_cell_checks_all_neighbors() {
    // 2-row, 2-col table. Row 0: one colspan=2 cell. Row 1: two cells with
    // different border-top widths. The spanning cell's bottom edge should
    // reflect the maximum across both neighbors.
    //
    // | cell0 (colspan=2)     |
    // | cell1 (top=3) | cell2 (top=8) |
    let cells = vec![
        CellInfo {
            entity: elidex_ecs::Entity::DANGLING,
            col: 0,
            row: 0,
            colspan: 2,
            rowspan: 1,
        },
        make_cell(0, 1),
        make_cell(1, 1),
    ];
    let styles = vec![
        ComputedStyle {
            border_bottom_width: 1.0,
            border_bottom_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_top_width: 3.0,
            border_top_style: BorderStyle::Solid,
            ..Default::default()
        },
        ComputedStyle {
            border_top_width: 8.0,
            border_top_style: BorderStyle::Solid,
            ..Default::default()
        },
    ];
    let table_style = ComputedStyle::default();
    let result = algo::resolve_collapsed_borders(&cells, &styles, &table_style, 2, 2);
    // cell0 bottom should resolve against both cell1 (top=3) and cell2 (top=8),
    // taking the max: resolve(1,3)=3, resolve(1,8)=8 → max=8.
    assert!(approx_eq(result[0].bottom, 8.0));
}

// ---------------------------------------------------------------------------
// T-3: tfoot ordering
// ---------------------------------------------------------------------------

#[test]
fn table_tfoot_after_tbody() {
    // tfoot rows should appear after tbody rows even if tfoot is first in DOM.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    // tfoot (first in DOM)
    let tfoot = dom.create_element("tfoot", Attributes::default());
    dom.world_mut().insert_one(
        tfoot,
        ComputedStyle {
            display: Display::TableFooterGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tfoot);
    let tr_foot = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_foot,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tfoot, tr_foot);
    let cell_foot = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_foot,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_foot, cell_foot);

    // tbody (second in DOM)
    let tbody = dom.create_element("tbody", Attributes::default());
    dom.world_mut().insert_one(
        tbody,
        ComputedStyle {
            display: Display::TableRowGroup,
            ..Default::default()
        },
    );
    dom.append_child(table, tbody);
    let tr_body = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr_body,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(tbody, tr_body);
    let cell_body = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        cell_body,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr_body, cell_body);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let body_lb = get_layout(&dom, cell_body);
    let foot_lb = get_layout(&dom, cell_foot);
    // tbody rows should come before tfoot rows.
    assert!(
        body_lb.content.y < foot_lb.content.y,
        "tbody cell y ({}) should be < tfoot cell y ({})",
        body_lb.content.y,
        foot_lb.content.y,
    );
}

// ---------------------------------------------------------------------------
// T-4: empty table (0 children, 0 rows/cols)
// ---------------------------------------------------------------------------

#[test]
fn table_empty_zero_children() {
    // Table with no children at all should produce a valid LayoutBox.
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(table, default_table_style());

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );
    let lb = get_layout(&dom, table);
    assert!(lb.content.width >= 0.0);
    assert!(lb.content.height >= 0.0);
}

// ---------------------------------------------------------------------------
// T-5: display:none cell skipping
// ---------------------------------------------------------------------------

#[test]
fn table_display_none_cell_skipped() {
    // A cell with display:none should not occupy a grid position.
    // Row: [visible_cell] [display:none] [visible_cell]
    // Expected: 2 columns (not 3).
    let mut dom = EcsDom::new();
    let table = dom.create_element("table", Attributes::default());
    dom.world_mut().insert_one(
        table,
        ComputedStyle {
            display: Display::Table,
            width: Dimension::Length(300.0),
            ..Default::default()
        },
    );

    let tr = dom.create_element("tr", Attributes::default());
    dom.world_mut().insert_one(
        tr,
        ComputedStyle {
            display: Display::TableRow,
            ..Default::default()
        },
    );
    dom.append_child(table, tr);

    let td1 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td1,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td1);

    let td_hidden = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td_hidden,
        ComputedStyle {
            display: Display::None,
            ..Default::default()
        },
    );
    dom.append_child(tr, td_hidden);

    let td2 = dom.create_element("td", Attributes::default());
    dom.world_mut().insert_one(
        td2,
        ComputedStyle {
            display: Display::TableCell,
            height: Dimension::Length(20.0),
            ..Default::default()
        },
    );
    dom.append_child(tr, td2);

    let font_db = FontDatabase::new();
    layout_table(
        &mut dom,
        table,
        600.0,
        None,
        0.0,
        0.0,
        &font_db,
        0,
        test_layout_child,
    );

    let lb1 = get_layout(&dom, td1);
    let lb2 = get_layout(&dom, td2);
    // Two visible cells in a 300px table should each get ~150px (minus spacing).
    // The hidden cell should not create a third column.
    assert!(
        lb1.content.width > 100.0,
        "td1 width {} should be > 100px (2-col, not 3-col)",
        lb1.content.width
    );
    assert!(
        lb2.content.width > 100.0,
        "td2 width {} should be > 100px (2-col, not 3-col)",
        lb2.content.width
    );
    // td_hidden should not have a LayoutBox.
    assert!(
        dom.world().get::<&LayoutBox>(td_hidden).is_err(),
        "display:none cell should not have a LayoutBox"
    );
}
