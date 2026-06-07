//! Tests for CSS Multi-column Layout.

#![allow(unused_must_use)]

use elidex_ecs::{Attributes, EcsDom, Entity, InlineFlow};
use elidex_plugin::{
    is_multicol, BoxSizing, ColumnFill, ColumnSpan, ComputedStyle, CssSize, Dimension, Display,
    EdgeSizes, Float, LayoutBox, MulticolInfo, Point, Position, Size, WritingMode,
};
use elidex_text::{measure_text, FontDatabase, FontStyle, TextMeasureParams};

use crate::layout_multicol;
use elidex_layout_block::LayoutInput;

mod box_fragment;
mod fill;
mod geometry;
mod inline_flow;
mod is_multicol;
mod prereqs;
mod spanner;

fn make_font_db() -> FontDatabase {
    FontDatabase::new()
}

fn make_input(font_db: &FontDatabase) -> LayoutInput<'_> {
    LayoutInput {
        containing: CssSize::definite(600.0, 800.0),
        containing_inline_size: 600.0,
        offset: Point::ZERO,
        font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(600.0, 800.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    }
}

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

fn add_block_child(dom: &mut EcsDom, parent: Entity, height: f32) -> Entity {
    let child = elem(dom, "div");
    dom.append_child(parent, child);
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(height),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(child, style);
    child
}

fn layout_child_fn(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    elidex_layout_block::block::layout_block_inner(dom, entity, input, layout_child_fn)
}

// --- I-multicol (slice 4) helpers: whole-IFC-in-column InlineFlow persistence ---

const TEST_FONTS: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Hiragino Sans",
];

/// Skip-guard: usable fonts present (headless CI may have none → no text geometry).
fn fonts_available(font_db: &FontDatabase) -> bool {
    let params = TextMeasureParams {
        families: TEST_FONTS,
        font_size: 16.0,
        weight: 400,
        style: FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    measure_text(font_db, &params, "x").is_some()
}

/// A block child carrying one text node, so its IFC persists an `InlineFlow`.
/// `height` fixes the block extent so column distribution is deterministic.
/// Returns `(div, text-node)` — the text node is the IFC run-start (`run[0]`) key.
fn add_text_block(dom: &mut EcsDom, parent: Entity, text: &str, height: f32) -> (Entity, Entity) {
    let div = elem(dom, "div");
    dom.append_child(parent, div);
    let style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(height),
        font_family: TEST_FONTS.iter().map(|&s| s.to_string()).collect(),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(div, style);
    let tnode = dom.create_text(text);
    dom.append_child(div, tnode);
    (div, tnode)
}

/// The absolute inline start of a run-start's single-fragment, single-line flow.
fn flow_inline_start(dom: &EcsDom, run_start: Entity) -> f32 {
    let flow = dom
        .world()
        .get::<&InlineFlow>(run_start)
        .expect("the whole-in-column IFC persists an InlineFlow");
    assert_eq!(flow.fragments.len(), 1, "whole-in-column = one fragment");
    let lines = &flow.fragments[0].lines;
    assert_eq!(lines.len(), 1, "single-word block lays out as one line");
    assert!(
        !lines[0].runs.is_empty(),
        "the line carries at least one run"
    );
    lines[0].runs[0].inline_start()
}
