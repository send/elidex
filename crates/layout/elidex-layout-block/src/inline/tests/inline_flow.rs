//! Tests for `InlineFlow` persistence — the converged inline-text geometry that
//! render consumes (slice 1 of the render↔layout inline-pipeline convergence).

use super::*;
use elidex_ecs::{InlineFlow, InlineFlowRun, PseudoElementMarker, TextContent};
use elidex_plugin::{Direction, Position, TextAlign, TextTransform, WritingMode};

/// Build a `LayoutEnv` for the test font db.
fn env(font_db: &FontDatabase) -> crate::LayoutEnv<'_> {
    crate::LayoutEnv {
        font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
    }
}

/// The run-start key = the first composed child of the parent (render's run[0]).
fn run_start(dom: &EcsDom, parent: Entity) -> Entity {
    dom.composed_children(parent)[0]
}

/// Lay out `text` under writing mode `wm` and return `(dom, parent, run-start key)`.
/// `containing_inline_size` is the inline-axis extent (height for vertical modes).
fn layout_vertical(
    text: &str,
    wm: WritingMode,
    containing_inline_size: f32,
    origin: Point,
) -> Option<(EcsDom, Entity, Entity)> {
    let (mut dom, parent, mut style, font_db) = setup_inline_test(text)?;
    style.writing_mode = wm;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        containing_inline_size,
        parent,
        origin,
        &env(&font_db),
    );
    Some((dom, parent, key))
}

#[test]
fn persists_single_line_flow() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("simple horizontal text run should persist an InlineFlow");
    assert_eq!(flow.lines.len(), 1, "single short word → one line");
    let line = &flow.lines[0];
    assert_eq!(line.block_start, 0.0, "content origin is ZERO");
    assert_eq!(line.runs.len(), 1);
    assert_eq!(line.runs[0].text(), Some("Hello"));
    assert_eq!(line.runs[0].inline_start(), 0.0, "left-aligned start");
    // The run's entity is the style source (the <p>), not the text-node key.
    assert_eq!(line.runs[0].entity(), parent);
}

#[test]
fn coalesces_break_pieces_on_one_line() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Wide container: both words fit on one line.
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.lines.len(), 1);
    assert_eq!(
        flow.lines[0].runs.len(),
        1,
        "contiguous same-entity break pieces coalesce into one run"
    );
    assert_eq!(flow.lines[0].runs[0].text(), Some("hello world"));
}

#[test]
fn multi_line_wrap_has_increasing_block_start() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    // Tiny container forces a wrap at the space.
    layout_inline_context(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.lines.len(), 2, "wraps into two lines");
    assert_eq!(flow.lines[0].block_start, 0.0);
    assert!(
        flow.lines[1].block_start > flow.lines[0].block_start,
        "second line below the first (block_start {} > {})",
        flow.lines[1].block_start,
        flow.lines[0].block_start
    );
    assert!(flow.lines[0].runs[0]
        .text()
        .is_some_and(|t| t.starts_with("hello")));
    assert_eq!(flow.lines[1].runs[0].text(), Some("world"));
}

#[test]
fn absolute_coordinates_offset_by_content_origin() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    let origin = Point::new(10.0, 20.0);
    layout_inline_context(&mut dom, &children, 800.0, parent, origin, &env(&font_db));

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.lines[0].block_start, 20.0, "block_start = origin.y");
    assert_eq!(
        flow.lines[0].runs[0].inline_start(),
        10.0,
        "inline_start = origin.x (left-aligned)"
    );
}

#[test]
fn text_align_center_bakes_offset_into_inline_start() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    style.text_align = TextAlign::Center;
    style.direction = Direction::Ltr;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert!(
        flow.lines[0].runs[0].inline_start() > 0.0,
        "centered text is offset from the line start, got {}",
        flow.lines[0].runs[0].inline_start()
    );
}

// --- gate: runs that diverge between layout IFC and render are NOT persisted
// (relpos/sticky still gated — slice 3p-b; static atomics now persist — 3p-a) ---

#[test]
fn persists_atomic_inline_as_box_member() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    // Append a static inline-block after the text → the run contains an atomic.
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(20.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // Slice 3p-a: a static atomic now persists as an `AtomicBox` member (was gated).
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("a run with a static atomic inline now persists an InlineFlow (slice 3p-a)");
    // Find the AtomicBox member and the block_start of the line it landed on. (The
    // test's `layout_block_only` child lays the inline-block out at full container
    // width, so it wraps below the text onto its own line — which makes the D7
    // reposition's block-axis move observable: the box moves from content_origin
    // y=0 down to the line.)
    let (atomic_inline, atomic_block) = flow
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
        .expect("the inline-block must be recorded as an AtomicBox member of the flow");
    assert!(
        atomic_block > 0.0 || atomic_inline > 0.0,
        "the atomic is placed away from the IFC origin on its line, got ({atomic_inline}, {atomic_block})"
    );

    // D7: layout repositioned the atomic's LayoutBox from content_origin (0,0) to its
    // member position (margins are 0, so margin-box origin == content origin).
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic was laid out and has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_inline).abs() < 0.5
            && (lb.content.origin.y - atomic_block).abs() < 0.5,
        "atomic LayoutBox repositioned to its member position ({atomic_inline}, {atomic_block}), \
         got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

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
fn gate_excludes_relative_positioned_atomic() {
    // A relpos/sticky *atomic* (inline-block) takes the atomic collect arm, which
    // sets `has_relpos_sticky_atomic` so the run stays gated — render paints it in
    // Layer 6 from its own `LayoutBox`. Converging it needs an offset-preserving box
    // reposition (a *different* mechanism than the slice-3p-b sub-flow), deferred to
    // slice 3p-b-2. Without the arm's `position` check it would persist an
    // `AtomicBox` member and double-paint with Layer 6.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        position: Position::Relative,
        width: Dimension::Length(20.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "a relative-positioned atomic inline must NOT persist (has_relpos_sticky gates \
         it via the atomic arm; it paints in render Layer 6) — else it double-paints"
    );
}

#[test]
fn vertical_atomic_repositions_with_axis_swap() {
    // Vertical WM (slice 2 persists it): the D7 reposition projects inline-axis →
    // physical y and block-axis → physical x (the `is_vertical` swap). Assert the
    // static atomic persists as an `AtomicBox` and its `LayoutBox` lands at the
    // swapped member position.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("a") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(20.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("vertical static atomic persists (slice 2 + 3p-a)");
    let (atomic_inline, atomic_block) = flow
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
        .expect("the inline-block is an AtomicBox member in vertical mode");

    // is_vertical projection: inline-axis → physical y, block-axis → physical x.
    let lb = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic was laid out and has a LayoutBox");
    assert!(
        (lb.content.origin.x - atomic_block).abs() < 0.5
            && (lb.content.origin.y - atomic_inline).abs() < 0.5,
        "vertical atomic box at the swapped position (x=block {atomic_block}, \
         y=inline {atomic_inline}), got ({}, {})",
        lb.content.origin.x,
        lb.content.origin.y
    );
}

#[test]
fn atomic_inner_text_inline_flow_shifts_with_box() {
    // An inline-block CONTAINING converged text: when the atomic is repositioned to
    // its line, the inner text's persisted `InlineFlow` (absolute coords, consumed
    // directly by render) must shift with the box — else render repaints the inner
    // text at the pre-reposition (content_origin) position. Root fix in
    // `shift_descendants` (covers relpos/abspos subtree shifts too).
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("a") else {
        return;
    };
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(40.0),
        height: Dimension::Length(20.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    let inner = dom.create_text("hi");
    dom.append_child(ib, inner);
    dom.append_child(parent, ib);

    let children = dom.composed_children(parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    // The atomic was repositioned to its line (the stub lays it out full-width, so it
    // wraps below "a") → its box is off content_origin.
    let box_y = dom
        .world()
        .get::<&LayoutBox>(ib)
        .expect("the atomic has a LayoutBox")
        .content
        .origin
        .y;
    // The inner text persisted its own InlineFlow (the atomic's inner IFC); its
    // block_start must have tracked the box, not stayed at the pre-shift origin.
    let inner_block = dom
        .world()
        .get::<&InlineFlow>(inner)
        .expect("the inner text persists an InlineFlow")
        .lines[0]
        .block_start;
    assert!(
        inner_block > 0.5,
        "the inner text's InlineFlow moved off content_origin with the box, got {inner_block}"
    );
    assert!(
        (inner_block - box_y).abs() < 0.5,
        "inner text InlineFlow tracks the repositioned box (box y {box_y}, inner block_start {inner_block})"
    );
}

#[test]
fn persists_pseudo_element_flow() {
    // Slice 3: pseudo `content` (incl. counter()) is resolved into the pseudo's
    // `TextContent` by the pre-layout generated-content pass, so layout measures
    // the real text and a plain-LTR, non-transformed pseudo run now persists like
    // any text run (the slice-1 `has_pseudo` gate is gone). Render consumes it.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    for &c in &dom.composed_children(parent) {
        dom.remove_child(parent, c);
    }
    let pseudo = dom.create_element("span", Attributes::default());
    let pseudo_style = ComputedStyle {
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(pseudo, pseudo_style);
    let _ = dom.world_mut().insert_one(pseudo, PseudoElementMarker);
    let _ = dom
        .world_mut()
        .insert_one(pseudo, TextContent("AB".to_string()));
    dom.append_child(parent, pseudo);

    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("plain-LTR pseudo run with resolved text must persist (slice 3)");
    assert!(
        flow.lines
            .iter()
            .flat_map(|l| &l.runs)
            .any(|r| r.text().is_some_and(|t| t.contains("AB"))),
        "persisted flow should carry the pseudo's resolved generated text"
    );
}

// (Vertical writing modes now persist — slice 2. See `persists_vertical_rl_flow`,
// `persists_vertical_lr_flow`, and `vertical_absolute_coordinates_swap_axes` below;
// the slice-1 `gate_excludes_vertical_writing_mode` was removed when the gate
// dropped.)

#[test]
fn gate_excludes_justify() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi there") else {
        return;
    };
    style.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "text-align: justify falls back to render until per-line distribution moves to layout"
    );
}

/// F9: off the paged path `layout_generation` is constant 0, so a run that
/// becomes non-persistable must be cleared by an explicit remove — not by
/// generation comparison. Persist, then re-layout gated-out, and assert removed.
#[test]
fn stale_flow_cleared_when_run_becomes_gated_out() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);

    // Pass 1: simple run persists.
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    assert!(
        dom.world().get::<&InlineFlow>(key).is_ok(),
        "pass 1 persists"
    );

    // Flip the gate: switch the parent to justify (same generation = 0).
    let mut justified = style;
    justified.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, justified);

    // Pass 2: now gated out → the stale flow must be removed (not consumable).
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
        dom.world().get::<&InlineFlow>(key).is_err(),
        "stale InlineFlow must be explicitly cleared when the run becomes gated out \
         (generation is constant 0 off the paged path, so it can't signal staleness)"
    );
}

#[test]
fn gate_excludes_rtl_text() {
    // Hebrew text needs bidi visual reordering, which render applies but layout's
    // logical-order positions do not encode → must fall back (slice 4 handles bidi).
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("שלום עולם") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "RTL/bidi text must not persist — render reorders visually, layout positions logically"
    );
}

#[test]
fn gate_excludes_text_transform() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("hello") else {
        return;
    };
    style.text_transform = TextTransform::Uppercase;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "text-transform must not persist — layout measures untransformed text but render \
         transforms before shaping, so baked positions would be wrong"
    );
}

#[test]
fn gate_excludes_fragmented() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    let frag = InlineFragConstraint {
        available_block: 1000.0,
        orphans: 2,
        widows: 2,
        skip_lines: 0,
    };
    layout_inline_context_fragmented(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
        Some(&frag),
    );

    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "fragmented (paged) runs must not persist — flow_lines are not yet sliced per fragment"
    );
}

// --- slice 2: vertical writing modes now persist (the `!is_vertical` gate dropped) ---

#[test]
fn persists_vertical_rl_flow() {
    let Some((dom, parent, key)) =
        layout_vertical("Hi", WritingMode::VerticalRl, 800.0, Point::ZERO)
    else {
        return;
    };
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("vertical-rl text now persists an InlineFlow (slice 2)");
    assert_eq!(flow.lines.len(), 1, "short text → one line/column");
    assert_eq!(flow.lines[0].runs.len(), 1);
    assert_eq!(flow.lines[0].runs[0].text(), Some("Hi"));
    assert_eq!(flow.lines[0].runs[0].entity(), parent);
}

#[test]
fn persists_vertical_lr_flow() {
    let Some((dom, _parent, key)) =
        layout_vertical("Hi", WritingMode::VerticalLr, 800.0, Point::ZERO)
    else {
        return;
    };
    assert!(
        dom.world().get::<&InlineFlow>(key).is_ok(),
        "vertical-lr text persists too (slice 2 dropped only the is_vertical gate)"
    );
}

#[test]
fn vertical_absolute_coordinates_swap_axes() {
    // The persist fold applies the is_vertical projection rule: inline-axis maps to
    // physical y, block-axis to physical x (the swap, mirroring static_positions).
    // With origin (10, 20): block_start = origin.x = 10, inline_start = origin.y = 20
    // — the OPPOSITE of the horizontal case (block_start = y, inline_start = x).
    let origin = Point::new(10.0, 20.0);
    let Some((dom, _parent, key)) = layout_vertical("Hi", WritingMode::VerticalRl, 800.0, origin)
    else {
        return;
    };
    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(
        flow.lines[0].block_start, 10.0,
        "vertical: block-axis maps to physical x = origin.x"
    );
    assert_eq!(
        flow.lines[0].runs[0].inline_start(),
        20.0,
        "vertical: inline-axis maps to physical y = origin.y (start-aligned)"
    );
}

#[test]
fn vertical_multi_line_increasing_block_start() {
    // Tiny inline-axis (vertical) extent forces a wrap at the space → two columns
    // stacking along the block axis (physical x), so block_start increases.
    let Some((dom, _parent, key)) =
        layout_vertical("hello world", WritingMode::VerticalRl, 1.0, Point::ZERO)
    else {
        return;
    };
    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.lines.len(), 2, "wraps into two columns");
    assert_eq!(flow.lines[0].block_start, 0.0);
    assert!(
        flow.lines[1].block_start > flow.lines[0].block_start,
        "second column is offset along the block axis (x): block_start {} > {}",
        flow.lines[1].block_start,
        flow.lines[0].block_start
    );
}

#[test]
fn vertical_justify_still_gated() {
    // Slice 2 dropped ONLY the is_vertical exclusion; every other gate still applies.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hi") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    style.text_align = TextAlign::Justify;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    assert!(
        dom.world().get::<&InlineFlow>(key).is_err(),
        "vertical + justify must not persist (justify is still render's fallback)"
    );
}
