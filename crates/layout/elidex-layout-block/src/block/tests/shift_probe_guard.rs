//! Z-1b-0.5 (P1 completion): the `is_probe` guard on the canonical subtree shifter
//! must leave a persisted `InlineFlow` (render-consumed, *visible* — this is the
//! R3d/R4 half) untouched during a throwaway probe, while still moving it on a
//! definitive shift. Self-contained: an `InlineFlow` is inserted directly (no IFC
//! fixture needed — `InlineFlow`/`InlineFlowLine`/`InlineFlowRun` are public, and
//! the arm's writing-mode projection defaults to horizontal when the run-start has
//! no styled parent).

use elidex_ecs::{EcsDom, InlineFlow, InlineFlowLine, InlineFlowRun};
use elidex_plugin::Vector;

use crate::block::children::shift_descendants;

/// Build a single-line, single-text-run `InlineFlow` at the given absolute coords.
fn flow_at(block_start: f32, inline_start: f32, text_entity: elidex_ecs::Entity) -> InlineFlow {
    InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start,
            block_size: 16.0,
            runs: vec![InlineFlowRun::Text {
                entity: text_entity,
                text: "x".to_string(),
                inline_start,
            }],
            justify_word_spacing: 0.0,
        }],
    )
}

fn flow_coords(dom: &EcsDom, run_start: elidex_ecs::Entity) -> (f32, f32) {
    let flow = dom.world().get::<&InlineFlow>(run_start).unwrap();
    let line = &flow.fragments[0].lines[0];
    let inline_start = match &line.runs[0] {
        InlineFlowRun::Text { inline_start, .. }
        | InlineFlowRun::AtomicBox { inline_start, .. } => *inline_start,
    };
    (line.block_start, inline_start)
}

#[test]
fn probe_shift_does_not_move_persisted_inline_flow() {
    let mut dom = EcsDom::new();
    let run_start = dom.world_mut().spawn(());
    let _ = dom
        .world_mut()
        .insert_one(run_start, flow_at(100.0, 200.0, run_start));

    // Probe: the persisted (definitive) InlineFlow must NOT move — the probe does
    // not rebuild it, so shifting it would corrupt the render-consumed coords.
    shift_descendants(&mut dom, &[run_start], Vector::new(40.0, 70.0), true);
    let (block_start, inline_start) = flow_coords(&dom, run_start);
    assert!(
        (block_start - 100.0).abs() < 0.01 && (inline_start - 200.0).abs() < 0.01,
        "a probe leaves the persisted InlineFlow coords put (block={block_start}, inline={inline_start})"
    );
}

#[test]
fn definitive_shift_does_move_persisted_inline_flow() {
    let mut dom = EcsDom::new();
    let run_start = dom.world_mut().spawn(());
    let _ = dom
        .world_mut()
        .insert_one(run_start, flow_at(100.0, 200.0, run_start));

    // Definitive (not a probe): the InlineFlow shifts with the subtree. Horizontal
    // default (no styled parent): block_start += delta.y, inline_start += delta.x.
    shift_descendants(&mut dom, &[run_start], Vector::new(40.0, 70.0), false);
    let (block_start, inline_start) = flow_coords(&dom, run_start);
    assert!(
        (block_start - 170.0).abs() < 0.01 && (inline_start - 240.0).abs() < 0.01,
        "a definitive shift moves the InlineFlow by the delta (block={block_start}, inline={inline_start})"
    );
}
