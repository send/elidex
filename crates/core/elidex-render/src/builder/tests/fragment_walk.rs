//! Render fragment-walk of the multicol box store (terminal-Z C-1).
//!
//! These exercise the unified chrome+clip+content loop in [`super::super::walk`]:
//! a **consumable** mid-break IFC entity (store-flagged, the direct-child IFC
//! category) paints per-column chrome + per-column overflow clip + per-column
//! clipped content, while every other entity keeps the single-`LayoutBox` path
//! byte-for-byte. The store-recorded `consumable` flag — NOT box-fragment presence —
//! is the router signal (the scope guard, test
//! `nonconsumable_midbreak_uses_single_clip`).

use elidex_ecs::{BoxFragment, InlineFlow, InlineFlowLine, InlineFlowRun};
use elidex_plugin::{
    BorderSide, BorderStyle, BoxDecorationBreak, ComputedStyle, CssColor, Display, EdgeSizes,
    LayoutBox, Overflow, Rect,
};

use super::*;
use crate::display_list::DisplayItem;

/// A zero-edges box fragment at column inline-offset `x` (so padding-box == content).
fn col_fragment(x: f32) -> BoxFragment {
    BoxFragment {
        content: Rect::new(x, 0.0, 100.0, 40.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    }
}

/// All `PushClip` rects in display-list order.
fn push_clip_rects(dl: &crate::display_list::DisplayList) -> Vec<Rect> {
    dl.0.iter()
        .filter_map(|i| match i {
            DisplayItem::PushClip { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

fn count<F: Fn(&DisplayItem) -> bool>(dl: &crate::display_list::DisplayList, pred: F) -> usize {
    dl.0.iter().filter(|i| pred(i)).count()
}

/// Build a `clip`ped (or not) mid-break IFC `div` spanning two columns, with the
/// two box-store fragments flagged `consumable`, plus the converged two-column
/// `InlineFlow` on its text run-start. Returns `(dom, div)`.
fn make_consumable_midbreak(
    clip: bool,
    consumable: bool,
) -> (elidex_ecs::EcsDom, elidex_ecs::Entity) {
    let mut style = ComputedStyle {
        display: Display::Block,
        font_family: test_font_family_strings(),
        ..Default::default()
    };
    if clip {
        style.overflow_x = Overflow::Hidden;
    }
    // The entity's single (G11, last-column) LayoutBox — drives per-entity concerns;
    // the per-column store fragments drive the chrome+clip+content loop.
    let (mut dom, div) = setup_block_element(
        style,
        LayoutBox {
            content: Rect::new(150.0, 0.0, 100.0, 40.0),
            ..Default::default()
        },
    );

    // The two per-column box fragments (column 0 at x=0, column 1 at x=150).
    dom.fragment_tree_mut()
        .push_box(div, 0, col_fragment(0.0), consumable);
    dom.fragment_tree_mut()
        .push_box(div, 1, col_fragment(150.0), consumable);

    // The converged InlineFlow on the run-start text child: one line per column at
    // the column's baked inline-start (x≈0 and x≈150).
    let node = dom.create_text("ab cd");
    let _ = dom.append_child(div, node);
    let flow = InlineFlow::single(
        0,
        vec![
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "ab".to_string(),
                    inline_start: 0.0,
                }],
            },
            InlineFlowLine {
                block_start: 0.0,
                block_size: 20.0,
                justify_word_spacing: 0.0,
                runs: vec![InlineFlowRun::Text {
                    entity: div,
                    text: "cd".to_string(),
                    inline_start: 150.0,
                }],
            },
        ],
    );
    let _ = dom.world_mut().insert_one(node, flow);
    (dom, div)
}

#[test]
fn consumable_midbreak_clipping_emits_a_clip_per_column() {
    // The #316 regression fixture: an `overflow:hidden` mid-break IFC must clip
    // PER COLUMN (css-multicol-1 §8.1), not once from the last-column box — else the
    // col-0 lines fall left of the last-column clip and vanish. C-1 pushes one clip
    // per store fragment, at that column's padding box.
    let (dom, _div) = make_consumable_midbreak(true, true);
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    let clips = push_clip_rects(&dl);
    assert_eq!(
        clips.len(),
        2,
        "a clipping consumable mid-break entity pushes one clip per column, not one \
         last-column clip (got {clips:?})"
    );
    let xs: Vec<f32> = clips.iter().map(|r| r.origin.x).collect();
    assert!(
        xs.contains(&0.0) && xs.contains(&150.0),
        "the two clips are the two columns' padding boxes (x=0 and x=150), got {xs:?}"
    );
    // Clips are balanced.
    assert_eq!(
        count(&dl, |i| matches!(i, DisplayItem::PushClip { .. })),
        count(&dl, |i| matches!(i, DisplayItem::PopClip)),
        "every per-column PushClip is matched by a PopClip"
    );
    // Content survives: the InlineFlow is re-emitted under each column clip (so col-0
    // text is not clipped away). Disjoint clips make each line visible exactly once.
    assert!(
        count(&dl, |i| matches!(i, DisplayItem::Text { .. })) >= 2,
        "the converged InlineFlow is emitted under each column clip"
    );
}

#[test]
fn consumable_midbreak_nonclipping_emits_content_once_no_clip() {
    // Without `overflow:hidden` the per-column chrome still loops, but the content is
    // emitted ONCE (no clip), at the lines' baked absolute offsets — re-emitting it
    // per fragment with no clip would over-paint. (css-break-3 §5.4 row 3.)
    let (dom, _div) = make_consumable_midbreak(false, true);
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    assert_eq!(
        push_clip_rects(&dl).len(),
        0,
        "a non-clipping mid-break entity pushes no clip"
    );
    // Two lines, emitted once total (not once-per-fragment).
    assert_eq!(
        count(&dl, |i| matches!(i, DisplayItem::Text { .. })),
        2,
        "the InlineFlow's two lines are emitted exactly once (no per-fragment re-emit \
         without a clip to disjoin them)"
    );
}

#[test]
fn nonconsumable_midbreak_uses_single_clip() {
    // The scope guard (§2.2): box-store fragments that are NOT flagged consumable
    // (a nested-block / deeper-IFC mid-break — box geometry, no per-column carrier)
    // must ride the single-`LayoutBox` arm, exactly as today — NOT the per-fragment
    // path. Box-fragment presence alone is not the router signal.
    let (dom, _div) = make_consumable_midbreak(true, false);
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    let clips = push_clip_rects(&dl);
    assert_eq!(
        clips.len(),
        1,
        "a non-consumable entity clips once from its single LayoutBox (got {clips:?})"
    );
    assert_eq!(
        clips[0].origin.x, 150.0,
        "the single clip is the entity's LayoutBox padding box (last-column G11 box)"
    );
}

#[test]
fn single_box_overflow_hidden_unchanged_one_clip() {
    // N=1 byte-identity: a plain `overflow:hidden` block with NO store fragments uses
    // the single-iteration loop — exactly one clip at its LayoutBox padding box, as
    // before C-1.
    let (mut dom, div) = setup_block_element(
        ComputedStyle {
            display: Display::Block,
            overflow_x: Overflow::Hidden,
            font_family: test_font_family_strings(),
            ..Default::default()
        },
        LayoutBox {
            content: Rect::new(0.0, 0.0, 100.0, 40.0),
            ..Default::default()
        },
    );
    let node = dom.create_text("hi");
    let _ = dom.append_child(div, node);
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    assert_eq!(
        push_clip_rects(&dl).len(),
        1,
        "a non-fragmented overflow:hidden block pushes exactly one clip (N=1 path)"
    );
}

/// A 2px-border box fragment at column inline-offset `x`.
fn bordered_col_fragment(x: f32) -> BoxFragment {
    BoxFragment {
        content: Rect::new(x, 0.0, 100.0, 40.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::uniform(2.0),
        margin: EdgeSizes::default(),
        first_baseline: None,
    }
}

/// Build a non-clipping, fully-bordered consumable mid-break `div` spanning two
/// columns with the given `box-decoration-break`, paint it, and return the count of
/// border `SolidRect`s (transparent background ⇒ the only SolidRects are borders).
fn bordered_midbreak_border_rects(decoration: BoxDecorationBreak) -> usize {
    let side = BorderSide {
        width: 2.0,
        style: BorderStyle::Solid,
        color: CssColor::BLACK,
    };
    let (mut dom, div) = setup_block_element(
        ComputedStyle {
            display: Display::Block,
            font_family: test_font_family_strings(),
            border_top: side,
            border_right: side,
            border_bottom: side,
            border_left: side,
            box_decoration_break: decoration,
            ..Default::default()
        },
        LayoutBox {
            content: Rect::new(150.0, 0.0, 100.0, 40.0),
            border: EdgeSizes::uniform(2.0),
            ..Default::default()
        },
    );
    dom.fragment_tree_mut()
        .push_box(div, 0, bordered_col_fragment(0.0), true);
    dom.fragment_tree_mut()
        .push_box(div, 1, bordered_col_fragment(150.0), true);
    let node = dom.create_text("ab");
    let _ = dom.append_child(div, node);
    let flow = InlineFlow::single(
        0,
        vec![InlineFlowLine {
            block_start: 0.0,
            block_size: 20.0,
            justify_word_spacing: 0.0,
            runs: vec![InlineFlowRun::Text {
                entity: div,
                text: "ab".to_string(),
                inline_start: 0.0,
            }],
        }],
    );
    let _ = dom.world_mut().insert_one(node, flow);
    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    dl.0.iter()
        .filter(|i| matches!(i, DisplayItem::SolidRect { .. }))
        .count()
}

#[test]
fn box_decoration_break_clone_vs_slice_per_column_chrome() {
    // css-break-3 §5.4: `slice` (default) inserts no border AT a break (the block-axis
    // edge of each column fragment is omitted), while `clone` paints the FULL border on
    // every fragment. For a 2-column mid-break box with 4 solid borders:
    //   - clone: 4 sides × 2 columns = 8 border rects
    //   - slice: each column omits one block-axis edge (col 0 the block-end, col 1 the
    //     block-start) ⇒ 3 sides × 2 = 6 border rects
    let clone_rects = bordered_midbreak_border_rects(BoxDecorationBreak::Cloned);
    let slice_rects = bordered_midbreak_border_rects(BoxDecorationBreak::Slice);
    assert_eq!(
        clone_rects, 8,
        "box-decoration-break: clone paints the full 4-side border on every column"
    );
    assert_eq!(
        slice_rects, 6,
        "box-decoration-break: slice omits the block-axis edge at each column break"
    );
    assert!(
        clone_rects > slice_rects,
        "clone (full per-fragment chrome) emits more border than slice (break edges omitted)"
    );
}

#[test]
fn paged_consumable_clipping_uses_the_all_column_union_clip() {
    // Codex PR#321 R4-F3 (regression) + R6-F2 + R8-F2: on the PAGED path (`ctx.paged`,
    // both renderers) the store fragments are not consumed per-fragment (§2.8), so a
    // consumable clipping mid-break must NOT clip to the single last-column `LayoutBox`
    // (it would lose the earlier columns — the #316 loss on the paged path). It clips
    // to the UNION of the per-column padding boxes: overflow clipped to the element's
    // overall extent, every column survives. The discriminator is `ctx.paged`, NOT
    // `expected_generation` — this test drives the LEGACY paged renderer
    // (`paged: true, expected_generation: None`), the path R8-F2 was wrongly consuming
    // the store on.
    use super::super::walk::{walk, PaintContext};
    let (dom, div) = make_consumable_midbreak(true, true);
    let font_db = elidex_text::FontDatabase::new();
    let mut dl = crate::display_list::DisplayList::default();
    let mut font_cache = crate::font_cache::FontCache::new();
    let mut ctx = PaintContext {
        dom: &dom,
        font_db: &font_db,
        font_cache: &mut font_cache,
        dl: &mut dl,
        caret_visible: false,
        scroll_offset: elidex_plugin::Vector::<f32>::ZERO,
        counter_state: elidex_style::counter::CounterState::new(),
        paged: true,
        // Legacy paged renderer: paged with NO generation filter (the R8-F2 path).
        expected_generation: None,
        continuation_entities: None,
    };
    walk(
        &mut ctx,
        div,
        0,
        &elidex_plugin::transform_math::Perspective::default(),
        false,
    );
    let clips = push_clip_rects(&dl);
    assert_eq!(
        clips.len(),
        1,
        "exactly one clip — the all-column union (not a per-column or single-box clip)"
    );
    // Union of col-0 (x=0,w=100) and col-1 (x=150,w=100) padding boxes = x∈[0,250].
    assert_eq!(
        (clips[0].origin.x, clips[0].right()),
        (0.0, 250.0),
        "the clip spans both columns (no last-column-only loss), clipping overflow to \
         the element's overall extent (no bleed)"
    );
    assert!(
        count(&dl, |i| matches!(i, DisplayItem::Text { .. })) >= 2,
        "both columns' converged lines survive under the union clip"
    );
}
