//! Positioned inline-level convergence tests (the Layer-6 paint path):
//! - **Slice 3p-b** — `position:relative`/`sticky` inline *subtrees* converged as
//!   per-positioned-subtree `InlineFlow` sub-flows (the `walk(span)` path).
//! - **Slice 3p-b-2** — `position:relative`/`sticky` *atomic* inlines
//!   (`inline-block` etc.) converged by an offset-preserving `LayoutBox` reposition
//!   (NOT a flow member — render Layer 6 paints the box, so a member would
//!   double-paint). Offset *preservation* needs the real `dispatch_layout_child`
//!   (which bakes `apply_relative_offset`); `layout_block_only` here does not, so
//!   these tests cover the gate-drop / non-membership / reposition, and the
//!   offset-preservation crux lives in the `elidex-layout` integration test.
//!
//! Split out of `inline_flow.rs` (which exceeded the repo's ~1000-line convention)
//! as a sibling topic module, mirroring `baseline` / `text_height`. Reuses `env`
//! from `inline_flow` and `setup_inline_test` from the parent test module.

use super::inline_flow::env;
use super::*;
use elidex_ecs::{InlineFlow, InlineFlowRun};
use elidex_plugin::{Position, TextTransform, WritingMode};

/// Append a `position:relative` inline `<span>` containing `text` to `parent`, then
/// a trailing text node `tail`. Returns `(span, span's text node, tail text node)`.
fn append_relpos_span(
    dom: &mut EcsDom,
    parent: Entity,
    families: &[String],
    inner: &str,
    tail: &str,
) -> (Entity, Entity, Entity) {
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        position: Position::Relative,
        font_family: families.to_vec(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    let inner_t = dom.create_text(inner);
    dom.append_child(span, inner_t);
    dom.append_child(parent, span);
    let tail_t = dom.create_text(tail);
    dom.append_child(parent, tail_t);
    (span, inner_t, tail_t)
}

#[test]
fn persists_relative_positioned_inline_as_subflow() {
    // `<p>a<span rel>b</span>c</p>`: slice 3p-b converges the relpos inline as a
    // per-positioned-subtree SUB-FLOW. `a`/`c` go into the top-level flow (keyed on
    // the `a` text node = render's Layer-5 run[0]) with the in-flow GAP where the
    // span sits; `b` goes into a SEPARATE flow keyed on the span's first child (= the
    // `b` text node = render's `walk(span)` run[0]). The span entity itself carries
    // no flow.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let a_text = dom.composed_children(parent)[0];
    let (span, b_text, _c_text) =
        append_relpos_span(&mut dom, parent, &style.font_family, "b", "c");

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // Top-level flow on the `a` text node: members for `a` and `c` (both styled by
    // the <p>), NOT `b` (it's in the sub-flow). `c` is past the span — the in-flow
    // gap (CSS 2 §9.4.3) is preserved.
    let top = dom
        .world()
        .get::<&InlineFlow>(a_text)
        .expect("top-level flow persists on the first non-positioned child (slice 3p-b)");
    let top_members: Vec<_> = top.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert_eq!(
        top_members.len(),
        2,
        "top-level flow has `a` and `c`, not `b`"
    );
    assert!(
        top_members.iter().all(|m| m.entity() == parent),
        "`a` and `c` are styled by the <p>"
    );
    let a_start = top_members[0].inline_start();
    let c_start = top_members[1].inline_start();

    // Sub-flow on the span's first child (`b` text node), NOT on the span entity.
    let sub = dom
        .world()
        .get::<&InlineFlow>(b_text)
        .expect("relpos inline sub-flow persists on its first eligible child");
    let sub_members: Vec<_> = sub.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert_eq!(sub_members.len(), 1, "sub-flow holds only `b`");
    assert_eq!(sub_members[0].text(), Some("b"));
    assert_eq!(sub_members[0].entity(), span, "`b` is styled by the span");
    let b_start = sub_members[0].inline_start();

    assert!(
        dom.world().get::<&InlineFlow>(span).is_err(),
        "the span entity is not a run-start key — no flow on it"
    );

    // In-flow gap: a < b < c (b sits between a and c; c is past the span, not at a's end).
    assert!(
        a_start < b_start && b_start < c_start,
        "in-flow gap preserved: a({a_start}) < b({b_start}) < c({c_start})"
    );
}

#[test]
fn realigns_run_start_past_leading_positioned() {
    // `<p><span rel>b</span>c</p>`: the first child is the positioned span, but
    // render's Layer-5 run[0] is `c` (the span is skipped). The top-level key must
    // realign to `c` (NOT `children.first()` = span), else render reads no flow.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let (span, b_text, c_text) = append_relpos_span(&mut dom, parent, &style.font_family, "b", "c");
    assert_eq!(
        dom.composed_children(parent)[0],
        span,
        "the span is the first child (children.first())"
    );

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // Realigned: the top-level flow is keyed on `c`, not the leading span.
    let top = dom
        .world()
        .get::<&InlineFlow>(c_text)
        .expect("top-level flow realigned onto the first non-positioned child `c`");
    let top_members: Vec<_> = top.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert_eq!(top_members.len(), 1);
    assert_eq!(top_members[0].text(), Some("c"));
    assert!(
        dom.world().get::<&InlineFlow>(span).is_err(),
        "no flow on the leading positioned span (children.first())"
    );
    // The sub-flow still persists on the span's first child.
    assert!(
        dom.world().get::<&InlineFlow>(b_text).is_ok(),
        "relpos sub-flow persists on `b`"
    );
}

#[test]
fn nested_relpos_in_static_inline_subflow_no_double_paint() {
    // `<p>a<em>x<span rel>b</span>y</em>c</p>`: the relpos span is nested inside a
    // STATIC inline <em>. `b` must be ONLY in the span sub-flow (Layer 6), NOT also
    // flattened into the top-level run — today's legacy path double-paints it
    // (`collect_styled_inline_text` recurses the span). The sub-flow routes `b`
    // once.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let a_text = dom.composed_children(parent)[0];
    let em = dom.create_element("em", Attributes::default());
    let em_style = ComputedStyle {
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(em, em_style);
    let x_text = dom.create_text("x");
    dom.append_child(em, x_text);
    let span = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Relative,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let b_text = dom.create_text("b");
    dom.append_child(span, b_text);
    dom.append_child(em, span);
    let y_text = dom.create_text("y");
    dom.append_child(em, y_text);
    dom.append_child(parent, em);
    let c_text = dom.create_text("c");
    dom.append_child(parent, c_text);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let top = dom
        .world()
        .get::<&InlineFlow>(a_text)
        .expect("top-level flow persists");
    let top_members: Vec<_> = top.lines.iter().flat_map(|l| l.runs.iter()).collect();
    // `a`(p), `x`(em), `y`(em), `c`(p) — NEVER the span (b is in the sub-flow).
    assert!(
        top_members.iter().all(|m| m.entity() != span),
        "`b` must NOT be flattened into the top-level flow (no double-paint)"
    );
    assert_eq!(top_members.len(), 4, "a, x, y, c — b excluded");

    let sub = dom
        .world()
        .get::<&InlineFlow>(b_text)
        .expect("sub-flow on the nested relpos span's first child");
    let sub_members: Vec<_> = sub.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert_eq!(sub_members.len(), 1);
    assert_eq!(sub_members[0].entity(), span);
    assert_eq!(sub_members[0].text(), Some("b"));
}

#[test]
fn static_atomic_inside_relpos_subflow_repositions() {
    // `<p><span rel>a<ib/>b</span></p>`: a static inline-block inside a relpos span.
    // The span sub-flow (keyed on its first child `a`) holds Text(a), AtomicBox(ib),
    // Text(b). The per-group reposition must move the inline-block's LayoutBox to its
    // on-line position INSIDE the sub-flow (not only the top-level group).
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let span = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Relative,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let a_text = dom.create_text("a");
    dom.append_child(span, a_text);
    let ib = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        ib,
        ComputedStyle {
            display: Display::InlineBlock,
            width: Dimension::Length(20.0),
            height: Dimension::Length(20.0),
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    dom.append_child(span, ib);
    let b_text = dom.create_text("b");
    dom.append_child(span, b_text);
    dom.append_child(parent, span);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let sub = dom
        .world()
        .get::<&InlineFlow>(a_text)
        .expect("relpos sub-flow persists on its first child `a`");
    let (atomic_inline, atomic_block) = sub
        .lines
        .iter()
        .flat_map(|line| line.runs.iter().map(move |r| (r, line.block_start)))
        .find_map(|(r, block_start)| match r {
            InlineFlowRun::AtomicBox {
                entity,
                inline_start,
            } if *entity == ib => Some((*inline_start, block_start)),
            _ => None,
        })
        .expect("the inline-block is an AtomicBox member of the SUB-flow");
    // The atomic is placed away from the sub-flow IFC origin — either after `a` on
    // the same line or (as `layout_block_only` sizes it to the full container width)
    // wrapped onto its own line below `a`. Either makes the per-group reposition
    // observable.
    assert!(
        atomic_block > 0.0 || atomic_inline > 0.0,
        "atomic placed away from the sub-flow origin, got ({atomic_inline}, {atomic_block})"
    );
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the inline-block has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_inline).abs() < 0.5
            && (lb.content.origin.y - atomic_block).abs() < 0.5,
        "the sub-flow atomic's LayoutBox was repositioned to its member position \
         ({atomic_inline}, {atomic_block}), got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

#[test]
fn subflow_cleared_when_relpos_made_static() {
    // Staleness (F9): persist a relpos sub-flow, then make the span `position:static`
    // and re-lay. The sub-flow key (`b` text node) must be CLEARED (its members
    // rejoin the top-level flow); render must not consume the stale sub-flow.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let a_text = dom.composed_children(parent)[0];
    let (span, b_text, _c_text) =
        append_relpos_span(&mut dom, parent, &style.font_family, "b", "c");
    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    assert!(
        dom.world().get::<&InlineFlow>(b_text).is_ok(),
        "precondition: relpos sub-flow persisted on `b`"
    );

    // Make the span static and re-lay.
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Static,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(b_text).is_err(),
        "the stale sub-flow on `b` must be cleared when the span becomes static"
    );
    let top = dom
        .world()
        .get::<&InlineFlow>(a_text)
        .expect("top-level flow persists");
    let top_members: Vec<_> = top.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert!(
        top_members.iter().any(|m| m.entity() == span),
        "`b` is now an in-flow member of the top-level flow (entity = the static span)"
    );
}

#[test]
fn top_level_key_matches_render_for_leading_display_none() {
    // Render's run[0] is the first non-positioned, non-block child — INCLUDING a
    // leading `display:none` element (render pushes it into the inline run; it just
    // paints nothing). The persist key must therefore key on that `display:none`
    // element too, NOT skip it — else render reads no flow and falls to legacy. (The
    // realigned `first_eligible_child` skips only positioned children, mirroring
    // render's `is_positioned`.)
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let hidden = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        hidden,
        ComputedStyle {
            display: Display::None,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let hx = dom.create_text("x");
    dom.append_child(hidden, hx);
    dom.append_child(parent, hidden);
    let visible = dom.create_text("visible");
    dom.append_child(parent, visible);

    let children = dom.composed_children(parent);
    assert_eq!(
        children[0], hidden,
        "the display:none span is the first child"
    );
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // The flow is keyed on the display:none element (render's run[0]), not realigned
    // past it onto the visible text.
    let flow = dom
        .world()
        .get::<&InlineFlow>(hidden)
        .expect("InlineFlow keyed on the leading display:none child = render's run[0]");
    let members: Vec<_> = flow.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert!(
        members.iter().any(|m| m.text() == Some("visible")),
        "the visible text is a member of the flow keyed on the display:none child"
    );
    assert!(
        dom.world().get::<&InlineFlow>(visible).is_err(),
        "the flow is NOT realigned onto the visible text node (that would mismatch render's run[0])"
    );
}

#[test]
fn relpos_inline_with_writing_mode_override_gated() {
    // A relpos inline that overrides `writing-mode` away from the IFC root gets NO
    // sub-flow: layout projects every group with the root's axis, but render reads a
    // sub-flow's writing mode off the span (its run-parent), so a mismatch would
    // transpose it. Gate to legacy (no transposition). The top-level still converges.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let a_text = dom.composed_children(parent)[0];
    let span = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Relative,
            // IFC root defaults to horizontal-tb; override the span to vertical-rl.
            writing_mode: WritingMode::VerticalRl,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let b_text = dom.create_text("b");
    dom.append_child(span, b_text);
    dom.append_child(parent, span);
    let c_text = dom.create_text("c");
    dom.append_child(parent, c_text);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(b_text).is_err(),
        "a relpos inline overriding writing-mode gets no sub-flow (would transpose) → legacy"
    );
    // The top-level run still converges (the span is excluded from it as a gap).
    assert!(
        dom.world().get::<&InlineFlow>(a_text).is_ok(),
        "the top-level flow still persists (only the WM-mismatched sub-flow is gated)"
    );
}

#[test]
fn persists_relative_positioned_inline_subflow_vertical() {
    // A relpos inline in a VERTICAL IFC whose writing mode MATCHES the root (the
    // common inherited case) persists a sub-flow, projected with the root's vertical
    // axis like every other group (slice 2). Sub-flow's `block_start` is the
    // physical x (column block-start); `inline_start` is physical y.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("a") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let a_text = dom.composed_children(parent)[0];
    let span = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Relative,
            // Matches the IFC root's vertical-rl (writing-mode is inherited in real
            // cascades; set explicitly here since the test builds ComputedStyle).
            writing_mode: WritingMode::VerticalRl,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let b_text = dom.create_text("b");
    dom.append_child(span, b_text);
    dom.append_child(parent, span);

    let children = dom.composed_children(parent);
    // Vertical IFC: containing inline-axis extent = height.
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(a_text).is_ok(),
        "top-level flow persists in the vertical IFC"
    );
    let sub = dom
        .world()
        .get::<&InlineFlow>(b_text)
        .expect("matching-WM relpos sub-flow persists in a vertical IFC");
    let members: Vec<_> = sub.lines.iter().flat_map(|l| l.runs.iter()).collect();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].text(), Some("b"));
}

#[test]
fn relpos_subflow_key_flattens_display_contents() {
    // Render's `walk(span)` iterates `composed_children_flat` (display:contents
    // flattened), so the sub-flow key must too: a `display:contents` first child of
    // the relpos span keys the sub-flow on the CONTENTS element's flattened first
    // child (= render's run[0]), NOT the contents wrapper — else render reads no
    // flow and drops to legacy.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let span = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        span,
        ComputedStyle {
            position: Position::Relative,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let contents = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        contents,
        ComputedStyle {
            display: Display::Contents,
            font_family: style.font_family.clone(),
            ..Default::default()
        },
    );
    let b_text = dom.create_text("b");
    dom.append_child(contents, b_text);
    dom.append_child(span, contents);
    dom.append_child(parent, span);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(b_text).is_ok(),
        "sub-flow keyed on the display:contents-flattened first child (render's run[0])"
    );
    assert!(
        dom.world().get::<&InlineFlow>(contents).is_err(),
        "NOT keyed on the display:contents wrapper (render flattens it away)"
    );
}

// ---------------------------------------------------------------------------
// Slice 3p-b-2 — relpos/sticky ATOMIC inlines (offset-preserving box reposition)
// ---------------------------------------------------------------------------

/// Append a 20×20 `inline-block` with `position` `pos` to `parent`; returns it.
fn append_inline_block(
    dom: &mut EcsDom,
    parent: Entity,
    families: &[String],
    pos: Position,
) -> Entity {
    let ib = dom.create_element("span", Attributes::default());
    let _ = dom.world_mut().insert_one(
        ib,
        ComputedStyle {
            display: Display::InlineBlock,
            position: pos,
            width: Dimension::Length(20.0),
            height: Dimension::Length(20.0),
            font_family: families.to_vec(),
            ..Default::default()
        },
    );
    dom.append_child(parent, ib);
    ib
}

#[test]
fn relpos_atomic_persists_and_repositions_not_a_flow_member() {
    // Slice 3p-b-2: a `position:relative` *atomic* (inline-block) no longer gates the
    // run. `<p>a<ib rel></ib>c</p>`: the top-level flow persists with Text members
    // for `a` and `c`, the relpos atomic is NOT an `AtomicBox` flow member (render
    // Layer 6 paints it — a member would double-paint with Layer 5), and its
    // `LayoutBox` is repositioned off the IFC origin to its on-line position.
    // (`layout_block_only` sizes the inline-block to the full container width — so it
    // wraps onto its own line below `a`, like `static_atomic_inside_relpos_subflow_*`
    // — and bakes no relative offset; the precise in-flow gap + offset preservation
    // need the real dispatcher and live in the `elidex-layout` integration test.)
    // Inverts the old `gate_excludes_relative_positioned_atomic`.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let a_text = dom.composed_children(parent)[0];
    let ib = append_inline_block(&mut dom, parent, &style.font_family, Position::Relative);
    let c_text = dom.create_text("c");
    dom.append_child(parent, c_text);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // The run now persists (gate dropped). Top-level flow on the `a` text node.
    let flow = dom
        .world()
        .get::<&InlineFlow>(a_text)
        .expect("slice 3p-b-2: a run containing a relpos atomic now persists (gate dropped)");
    let members: Vec<_> = flow.lines.iter().flat_map(|l| l.runs.iter()).collect();
    // `a` and `c` both persist as Text members (the atomic interrupts coalescing).
    assert!(
        members.iter().any(|m| m.text() == Some("a"))
            && members.iter().any(|m| m.text() == Some("c")),
        "both `a` and `c` are top-level Text members — got {members:?}"
    );
    // The relpos atomic is NOT a flow member (no AtomicBox, no `ib` entity).
    assert!(
        members
            .iter()
            .all(|m| matches!(m, InlineFlowRun::Text { .. }) && m.entity() != ib),
        "the relpos atomic must NOT be an AtomicBox flow member (Layer 6 paints it; a \
         member would double-paint) — members: {members:?}"
    );
    // The relpos atomic's box was repositioned away from the IFC origin where
    // `layout_atomic_items` placed it (on-line after `a`, or — full-width under
    // `layout_block_only` — wrapped onto its own line below `a`).
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the relpos atomic has a LayoutBox");
    assert!(
        lb.content.origin.x > 0.0 || lb.content.origin.y > 0.0,
        "the relpos atomic was repositioned off the IFC origin, got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

#[test]
fn relpos_atomic_with_text_transform_stays_on_legacy() {
    // D6 co-occurrence: a run that ALSO has `text-transform` stays gated (a
    // transform-slice concern) and falls to render legacy ENTIRELY — the relpos
    // atomic is then NOT repositioned (it stays at the IFC origin, as today). The
    // relpos-atomic convergence only applies on the persistable path.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("a") else {
        return;
    };
    style.text_transform = TextTransform::Uppercase;
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let a_text = dom.composed_children(parent)[0];
    let ib = append_inline_block(&mut dom, parent, &style.font_family, Position::Relative);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(a_text).is_err(),
        "text-transform gates the whole run → no InlineFlow persists"
    );
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic is still laid out (at the IFC origin)");
    assert_eq!(
        lb.content.origin,
        Point::ZERO,
        "a gated run does NOT reposition its relpos atomic — it stays where \
         layout_atomic_items placed it (the IFC content origin = ZERO here)"
    );
}

#[test]
fn relpos_atomic_repositioned_in_vertical_ifc() {
    // The relpos-atomic reposition folds with the same is_vertical swap the flow
    // members get (inline-axis → physical y, block-axis → physical x).
    // `<p vertical-rl><ib1/><ib2 rel/></p>`: ib1 (static) advances the inline (y) axis
    // by its 20px height, so ib2 (relpos) lands at inline-y = 20 and is repositioned
    // there — NOT a flow member. Two inline-blocks → no text/font dependency.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let ib1 = append_inline_block(&mut dom, parent, &style.font_family, Position::Static);
    let ib2 = append_inline_block(&mut dom, parent, &style.font_family, Position::Relative);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // ib2 (relpos) repositioned along the inline (y) axis past ib1's 20px advance.
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib2)
        .expect("the relpos atomic has a LayoutBox");
    assert!(
        lb.content.origin.y > 0.0,
        "vertical relpos atomic repositioned along the inline (y) axis, got y={}",
        lb.content.origin.y
    );
    // It is NOT a flow member (the flow keyed on ib1 holds only ib1's AtomicBox).
    let ib2_is_member = dom.world().get::<&InlineFlow>(ib1).is_ok_and(|flow| {
        flow.lines
            .iter()
            .flat_map(|l| l.runs.iter())
            .any(|m| m.entity() == ib2)
    });
    assert!(
        !ib2_is_member,
        "the relpos atomic ib2 must not be a flow member"
    );
}
