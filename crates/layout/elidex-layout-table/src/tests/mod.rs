//! Tests for the CSS table layout algorithm.

use elidex_ecs::{Attributes, EcsDom};
use elidex_layout_block::LayoutInput;
use elidex_plugin::{
    BorderCollapse, BorderStyle, ComputedStyle, Dimension, Display, LayoutBox, TableLayout,
    TextAlign,
};
use elidex_text::FontDatabase;

use crate::{algo, layout_table, CellInfo};

mod box_model;
mod col_span;
mod sizing;
mod structure;

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
fn test_layout_child(
    dom: &mut EcsDom,
    entity: elidex_ecs::Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    elidex_layout_block::block::layout_block_inner(dom, entity, input, test_layout_child).into()
}

/// Helper to call `layout_table` with the old positional-argument pattern used by tests.
#[allow(clippy::too_many_arguments)]
fn do_layout_table(
    dom: &mut EcsDom,
    entity: elidex_ecs::Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
    layout_child: elidex_layout_block::ChildLayoutFn,
) -> LayoutBox {
    let input = LayoutInput {
        containing_width,
        containing_height,
        offset_x,
        offset_y,
        font_db,
        depth,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
    };
    layout_table(dom, entity, &input, layout_child)
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

fn make_cell(col: usize, row: usize) -> CellInfo {
    CellInfo {
        entity: elidex_ecs::Entity::DANGLING,
        col,
        row,
        colspan: 1,
        rowspan: 1,
    }
}
