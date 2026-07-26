use super::*;

// ---------------------------------------------------------------------------
// Screen-geometry provenance (terminal-Z C-3a §2)
//
// The phase guard's acceptance matrix (plan §5 items 6-9), gathered here rather
// than split across `basic.rs` (screen) and `fragmentation.rs` (paged): the rule
// is ONE rule over BOTH geometry sources, so its counter-pins only read as pairs.
// `a_boxless_dispatch_…` (no write ⇒ no demotion) is only meaningful beside
// `direct_dispatch_…` (a write ⇒ demotion), and the paged zero-write case is the
// same pair again on the other source.
// ---------------------------------------------------------------------------

#[test]
fn layout_tree_publishes_completed_screen_provenance() {
    // Provenance (terminal-Z C-3a §2): `layout_tree` invalidates before laying out
    // and PUBLISHES a completed screen pass at completion (single publisher). So
    // `screen_geometry()` opens only after a full screen pass.
    let (mut dom, ..) = build_styled_dom();
    assert!(
        dom.screen_geometry().is_none(),
        "no completed screen pass before layout"
    );

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);

    assert!(
        dom.screen_geometry().is_some(),
        "layout_tree publishes CompletedScreen at completion"
    );
}

#[test]
fn direct_dispatch_after_publish_invalidates_the_phase() {
    // Codex PR#488 R1: `dispatch_layout_child` is `pub`, re-exported, and already
    // called cross-crate. A dirty-subtree / probe relayout of an ORDINARY non-multicol
    // box rewrites the `LayoutBox` COMPONENT without touching any `FragmentTree`
    // mutator, so the phase stayed `CompletedScreen` and `screen_geometry()` served a
    // MIXED GENERATION — this pass's component geometry beside the previous pass's
    // fragments — while promising a completed screen pass. It now demotes because the
    // dispatch reaches a real `LayoutBox` write, which goes through
    // `EcsDom::set_layout_box` (NOT because this fn brackets — see the two tests below).
    let (mut dom, _root, _html, body) = build_styled_dom();
    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);
    assert!(dom.screen_geometry().is_some(), "full pass published");

    let input = LayoutInput {
        containing: CssSize::definite(800.0, 600.0),
        containing_inline_size: 800.0,
        offset: elidex_plugin::Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    dispatch_layout_child(&mut dom, body, &input);

    assert!(
        dom.screen_geometry().is_none(),
        "a direct post-publish dispatch invalidates — no mixed-generation read"
    );
}

#[test]
fn a_boxless_dispatch_after_publish_leaves_the_phase_alone() {
    // Codex PR#488 R3 (the OVER-EAGER direction). `display: contents` generates no box:
    // that arm of `dispatch_layout_child` constructs a `LayoutOutcome` and returns it,
    // writing neither `LayoutBox` nor the store. Under the old dispatch-site bracket the
    // phase demoted anyway, so a probe / dirty-subtree caller could close
    // `screen_geometry()` engine-wide having changed nothing — the branch's own no-op
    // rule ("a write of nothing must not demote a published store") broken at a 4th site.
    // With the guard moved to the write chokepoint this is exact: no write, no demotion.
    let (mut dom, _root, _html, body) = build_styled_dom();
    let contents = dom.create_element("div", Attributes::default());
    dom.append_child(body, contents);
    dom.world_mut().insert_one(
        contents,
        ComputedStyle {
            display: Display::Contents,
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);
    assert!(dom.screen_geometry().is_some(), "full pass published");

    let input = LayoutInput {
        containing: CssSize::definite(800.0, 600.0),
        containing_inline_size: 800.0,
        offset: elidex_plugin::Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    dispatch_layout_child(&mut dom, contents, &input);

    assert!(
        dom.screen_geometry().is_some(),
        "a boxless dispatch wrote nothing, so the published pass still stands"
    );
}

#[test]
fn a_boxless_layout_generation_stamp_leaves_the_phase_alone() {
    // Codex PR#488 R6, and the LIVE instance of it. `dispatch_layout_child`'s paged
    // `layout_generation` stamp calls `EcsDom::layout_box_mut` on the dispatched entity,
    // but the `display: contents` arm inserts no `LayoutBox` — so the handle is `None`
    // and nothing is written. An earlier revision of the chokepoint demoted BEFORE the
    // lookup, which closed `screen_geometry()` engine-wide for a paged pass over a
    // boxless element: the same defect that moved the guard off the dispatch bracket,
    // reintroduced one layer down. `layout_generation: 1` is what makes it reachable —
    // `a_boxless_dispatch_after_publish_leaves_the_phase_alone` above uses 0, so it
    // never enters the stamp and cannot catch this.
    let (mut dom, _root, _html, body) = build_styled_dom();
    let contents = dom.create_element("div", Attributes::default());
    dom.append_child(body, contents);
    dom.world_mut().insert_one(
        contents,
        ComputedStyle {
            display: Display::Contents,
            ..Default::default()
        },
    );

    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);
    assert!(dom.screen_geometry().is_some(), "full pass published");
    assert!(
        dom.world().get::<&LayoutBox>(contents).is_err(),
        "precondition: display:contents carries no LayoutBox, so the stamp finds nothing"
    );

    let input = LayoutInput {
        containing: CssSize::definite(800.0, 600.0),
        containing_inline_size: 800.0,
        offset: elidex_plugin::Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 1,
        is_probe: false,
    };
    dispatch_layout_child(&mut dom, contents, &input);

    assert!(
        dom.screen_geometry().is_some(),
        "the stamp found no LayoutBox, wrote nothing, and must not demote"
    );
}

#[test]
fn a_write_that_bypasses_the_dispatcher_still_invalidates() {
    // Codex PR#488 R4 (the BYPASSABLE direction). The layout algorithms below
    // `dispatch_layout_child` are `pub` and are called directly cross-crate, so a guard
    // at the dispatcher was only as good as callers entering through it. Here a caller
    // reaches `layout_block_only` directly — never touching `dispatch_layout_child` —
    // and the phase must still demote, because the write itself goes through
    // `EcsDom::set_layout_box`. This test could not exist under the bracket design; that
    // it passes IS the difference between a review convention and a structural guard.
    let (mut dom, _root, _html, body) = build_styled_dom();
    let font_db = FontDatabase::new();
    layout_tree(&mut dom, Size::new(800.0, 600.0), &font_db);
    assert!(dom.screen_geometry().is_some(), "full pass published");

    let input = LayoutInput {
        containing: CssSize::definite(800.0, 600.0),
        containing_inline_size: 800.0,
        offset: elidex_plugin::Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _ = elidex_layout_block::layout_block_only(&mut dom, body, &input);

    assert!(
        dom.screen_geometry().is_none(),
        "a bypassing LayoutBox write still demotes — the guard is at the write, not the dispatch"
    );
}

#[test]
fn layout_fragmented_invalidates_screen_geometry_provenance() {
    // Provenance (terminal-Z C-3a §2): a paged pass that lays anything out must leave a
    // prior screen pass's `CompletedScreen` demoted — else `screen_geometry()` would read
    // a page-relative store as screen geometry (soundness hole 1). It holds without any
    // entry mark: whatever the pass lays out writes `LayoutBox`es through
    // `EcsDom::set_layout_box`, which invalidates. (A paged attempt that lays out NOTHING is the opposite case and must
    // NOT demote — `a_zero_write_paged_early_return_leaves_the_phase_alone` below.)
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    // A prior screen pass completed.
    dom.screen_layout_pass(|_| {});
    assert!(
        dom.screen_geometry().is_some(),
        "precondition: completed screen"
    );

    let font_db = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 200.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = elidex_layout_block::LayoutInput {
        containing: CssSize::definite(400.0, 1000.0),
        containing_inline_size: 400.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _ = layout_fragmented(&mut dom, div, &input, frag);

    assert!(
        dom.screen_geometry().is_none(),
        "the paged pass's geometry writes invalidated the stale CompletedScreen"
    );
}

#[test]
fn layout_paged_invalidates_screen_geometry_provenance() {
    // The OTHER layout-crate paged public fn: `layout_paged` reaches
    // `layout_fragmented_with_tokens` via `layout_fragmented`, so it too demotes a prior
    // screen pass once it lays anything out. Guards the path a future
    // non-interleaved-driver caller of `layout_paged` would take.
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    dom.screen_layout_pass(|_| {});
    assert!(
        dom.screen_geometry().is_some(),
        "precondition: completed screen"
    );

    let font_db = FontDatabase::new();
    let page_ctx = elidex_plugin::PagedMediaContext {
        page_width: 400.0,
        page_height: 600.0,
        page_margins: elidex_plugin::EdgeSizes::uniform(10.0),
        page_rules: Vec::new(),
    };
    let _ = layout_paged(&mut dom, &page_ctx, &font_db);

    assert!(
        dom.screen_geometry().is_none(),
        "layout_paged laid content out, so its writes cleared CompletedScreen"
    );
}

#[test]
fn a_zero_write_paged_early_return_leaves_the_phase_alone() {
    // Codex PR#488 R2 asked for an `invalidate()` *before* the paged fns'
    // validation returns, so that entering paged mode always clears
    // `CompletedScreen`. Deliberately NOT done — the invariant is about the store's
    // CONTENTS, not about which mode is executing. `CompletedScreen` means "the store
    // reflects a completed screen pass", and a paged attempt that bails on
    // `roots.is_empty()` or a non-positive content area writes NOTHING (`find_roots` /
    // `find_roots_mut` both take `&EcsDom`), so the store still holds exactly that and
    // `Some` is the truthful answer.
    //
    // Invalidating there would be a spurious demotion — the same defect this branch
    // removed from `remove_entity` one round earlier (a mutator that writes nothing must
    // not demote a published store, pinned by
    // `every_content_mutator_invalidates_a_published_store`). Codex's patch would have
    // reintroduced it on the paged side and forced a full relayout before any
    // screen-geometry read whenever a print attempt no-ops.
    //
    // The plan's §2 wording ("BOTH paged entries leave phase `Invalid`") was the real
    // defect here — it stated the guarantee at MODE granularity ("a paged fn was
    // called"), whereas what actually demotes is the pass reaching a store writer or a
    // component write; an attempt that returns before both reaches neither. §2 is
    // corrected there. ENTERING a mode is not itself a demotion event; writing is. Both
    // geometry sources now invalidate at their own write (`FragmentTree`'s mutators;
    // `EcsDom::set_layout_box`/`layout_box_mut`), so the rule is uniform and this case
    // falls out of it rather than being an exception to it. This test pins the decision
    // so a future reader of that section cannot "fix" the code back.
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    dom.world_mut().insert_one(
        div,
        ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    dom.screen_layout_pass(|_| {});

    let font_db = FontDatabase::new();
    // Margins exceed the page box ⇒ content_width/height <= 0 ⇒ the early return.
    let degenerate = elidex_plugin::PagedMediaContext {
        page_width: 10.0,
        page_height: 10.0,
        page_margins: elidex_plugin::EdgeSizes::uniform(20.0),
        page_rules: Vec::new(),
    };
    let pages = layout_paged(&mut dom, &degenerate, &font_db);

    assert!(pages.is_empty(), "degenerate page box lays nothing out");
    assert!(
        dom.screen_geometry().is_some(),
        "a zero-write paged early return must not demote the published screen pass"
    );

    // The SECOND zero-write arm of the same fn — `roots.is_empty()`. Pinning only the
    // non-positive-area arm would leave the decision half-covered, and a "fix" that
    // added the invalidate to this arm alone would pass.
    let mut empty_dom = EcsDom::new();
    empty_dom.screen_layout_pass(|_| {});
    let sane = elidex_plugin::PagedMediaContext {
        page_width: 400.0,
        page_height: 600.0,
        page_margins: elidex_plugin::EdgeSizes::uniform(10.0),
        page_rules: Vec::new(),
    };
    let pages = layout_paged(&mut empty_dom, &sane, &font_db);

    assert!(pages.is_empty(), "no layout roots ⇒ nothing laid out");
    assert!(
        empty_dom.screen_geometry().is_some(),
        "the roots.is_empty() early return is a zero-write path too — no demotion"
    );
}
