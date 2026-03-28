use super::*;

// --- Inline axis constraint equation tests ---

#[test]
fn inline_all_three_auto() {
    // start, size, end all auto → shrink-to-fit, start = static pos
    let r = resolve_inline_axis(
        &make_inline_props(None, None, None, 0.0, 0.0, false, false, 0.0, 800.0, 50.0),
        || 200.0,
    );
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 50.0)); // static position
}

#[test]
fn inline_size_auto_stretch() {
    // start + end specified, size auto → stretch
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(10.0),
            None,
            Some(20.0),
            0.0,
            0.0,
            false,
            false,
            0.0,
            800.0,
            0.0,
        ),
        || 100.0,
    );
    assert!(approx_eq(r.size, 770.0)); // 800 - 10 - 20
    assert!(approx_eq(r.offset, 10.0));
}

#[test]
fn inline_overconstrained_centering() {
    // All three specified + margin auto → centering
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(0.0),
            Some(200.0),
            Some(0.0),
            0.0,
            0.0,
            true,
            true,
            0.0,
            800.0,
            0.0,
        ),
        || 200.0,
    );
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 0.0));
    // margins split remaining 600 equally
    assert!(approx_eq(r.margin_start, 300.0));
    assert!(approx_eq(r.margin_end, 300.0));
}

// --- Block axis constraint equation tests ---

#[test]
fn block_start_end_stretch() {
    // start + end specified, size auto (no content) → stretch
    let r = resolve_block_axis(&make_block_props(
        Some(10.0),
        Some(20.0),
        None,
        None,
        0.0,
        0.0,
        false,
        false,
        0.0,
        600.0,
        0.0,
    ));
    assert!(approx_eq(r.size, 570.0)); // 600 - 10 - 20
    assert!(approx_eq(r.offset, 10.0));
}

#[test]
fn block_margin_centering() {
    // start + size + end all specified + margin auto → centering
    let r = resolve_block_axis(&make_block_props(
        Some(0.0),
        Some(0.0),
        Some(200.0),
        None,
        0.0,
        0.0,
        true,
        true,
        0.0,
        600.0,
        0.0,
    ));
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 0.0));
    assert!(approx_eq(r.margin_start, 200.0));
    assert!(approx_eq(r.margin_end, 200.0));
}

#[test]
fn inline_overconstrained_ignores_end() {
    // Over-constrained with no auto margins: inline-end is ignored, start kept.
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(10.0),
            Some(200.0),
            Some(50.0),
            0.0,
            0.0,
            false,
            false,
            0.0,
            800.0,
            0.0,
        ),
        || 200.0,
    );
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 10.0)); // start kept as-is
}

#[test]
fn block_centering_negative_space_equal_margins() {
    // Block axis: margin-start = margin-end always, even when negative.
    let r = resolve_block_axis(&make_block_props(
        Some(0.0),
        Some(0.0),
        Some(500.0),
        None,
        0.0,
        0.0,
        true,
        true,
        0.0,
        400.0,
        0.0,
    ));
    assert!(approx_eq(r.size, 500.0));
    assert!(approx_eq(r.offset, 0.0));
    // Both margins should be equal: -100 / 2 = -50 each
    assert!(approx_eq(r.margin_start, -50.0));
    assert!(approx_eq(r.margin_end, -50.0));
}

// --- Integration: absolute layout via block layout ---

#[test]
fn absolute_removed_from_flow() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let sibling = elem(&mut dom, "div");
    dom.append_child(root, sibling);

    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        sibling,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    // Sibling should be at y=100 (after normal child), not y=300 (after abs)
    let sib_lb = dom.world().get::<&LayoutBox>(sibling).unwrap();
    assert!(approx_eq(sib_lb.content.origin.y, 100.0));
}

#[test]
fn absolute_top_left_zero() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 0.0));
    assert!(approx_eq(lb.content.origin.y, 0.0));
}

#[test]
fn absolute_bottom_right() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            bottom: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Should be at right-bottom corner of CB (800x600)
    assert!(approx_eq(lb.content.origin.x, 600.0)); // 800 - 200
    assert!(approx_eq(lb.content.origin.y, 500.0)); // 600 - 100
}

#[test]
fn absolute_width_auto_stretch() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // width auto + left/right specified → stretch: 800 - 50 - 50 = 700
    assert!(approx_eq(lb.content.size.width, 700.0));
    assert!(approx_eq(lb.content.origin.x, 50.0));
}

#[test]
fn absolute_percentage_offsets() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Percentage(10.0),  // 10% of 600 = 60
            left: Dimension::Percentage(25.0), // 25% of 800 = 200
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, Point::ZERO, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 200.0));
    assert!(approx_eq(lb.content.origin.y, 60.0));
}

#[test]
fn absolute_cb_is_positioned_ancestor() {
    // Absolute child's CB is the nearest positioned ancestor's padding box.
    let mut dom = EcsDom::new();
    let outer = elem(&mut dom, "div");
    let rel = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(outer, rel);
    dom.append_child(rel, abs_child);

    let _ = dom.world_mut().insert_one(
        outer,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(1000.0),
            height: Dimension::Length(800.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        rel,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, outer, 1000.0, Point::ZERO, &font_db());

    let rel_lb = dom.world().get::<&LayoutBox>(rel).unwrap();
    let abs_lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // abs_child should be at rel's padding box origin
    assert!(approx_eq(abs_lb.content.origin.x, rel_lb.content.origin.x));
    assert!(approx_eq(abs_lb.content.origin.y, rel_lb.content.origin.y));
}

// --- Fixed positioning tests ---

#[test]
fn fixed_uses_viewport() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            bottom: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    // Layout with viewport 800x600
    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // Fixed to viewport bottom-right: (800-100, 600-50) = (700, 550)
    assert!(approx_eq(lb.content.origin.x, 700.0));
    assert!(approx_eq(lb.content.origin.y, 550.0));
}

#[test]
fn fixed_removed_from_flow() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    let sibling = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);
    dom.append_child(root, sibling);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            width: Dimension::Length(200.0),
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        sibling,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    // Sibling should be at y=0, not y=200
    let sib_lb = dom.world().get::<&LayoutBox>(sibling).unwrap();
    assert!(approx_eq(sib_lb.content.origin.y, 0.0));
}

#[test]
fn fixed_top_left_zero() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 0.0));
    assert!(approx_eq(lb.content.origin.y, 0.0));
}

#[test]
fn fixed_percentage_against_viewport() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Percentage(10.0),  // 10% of 600 = 60
            left: Dimension::Percentage(25.0), // 25% of 800 = 200
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    assert!(approx_eq(lb.content.origin.x, 200.0));
    assert!(approx_eq(lb.content.origin.y, 60.0));
}

#[test]
fn fixed_inside_relative() {
    // Fixed child should use viewport, not the relative parent.
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let rel = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, rel);
    dom.append_child(rel, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        rel,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            top: Dimension::Length(100.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // Should be at viewport (0,0), not relative to rel parent
    assert!(approx_eq(lb.content.origin.x, 0.0));
    assert!(approx_eq(lb.content.origin.y, 0.0));
}

/// CSS Transforms L1 §2: static element with transform establishes CB for
/// fixed descendants, even when a positioned ancestor is in between.
#[test]
fn fixed_inside_static_transform_ancestor_uses_transform_cb() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let root = elem(&mut dom, "div");
    let container = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(doc, root);
    dom.append_child(root, container);
    dom.append_child(container, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Static,
            has_transform: true, // transform ancestor
            width: Dimension::Length(600.0),
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            top: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(10.0),
            left: Dimension::Length(20.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // Fixed child should be positioned relative to root's padding box (transform CB),
    // not the viewport. Root's padding box starts at (0, 0) with width 600.
    assert!(approx_eq(lb.content.origin.x, 20.0));
    assert!(approx_eq(lb.content.origin.y, 10.0));
}

/// Fixed element with NO transform ancestor should still use viewport.
#[test]
fn fixed_no_transform_uses_viewport() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let root = elem(&mut dom, "div");
    let container = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(doc, root);
    dom.append_child(root, container);
    dom.append_child(container, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Static,
            // No transform
            width: Dimension::Length(600.0),
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            top: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some(Size::new(800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // No transform → viewport is CB
    assert!(approx_eq(lb.content.origin.x, 0.0));
    assert!(approx_eq(lb.content.origin.y, 0.0));
}

/// `collect_positioned_descendants` stops fixed collection at transform boundary.
#[test]
fn collect_stops_fixed_at_transform_boundary() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let parent = elem(&mut dom, "div");
    let transform_el = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(doc, parent);
    dom.append_child(parent, transform_el);
    dom.append_child(transform_el, fixed_child);

    let _ = dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        transform_el,
        ComputedStyle {
            display: Display::Block,
            position: Position::Static,
            has_transform: true,
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            ..Default::default()
        },
    );

    let (abs, fixed) = collect_positioned_descendants(&dom, parent);
    // The fixed child should NOT be collected by parent because
    // transform_el (static + has_transform) is the CB for fixed descendants.
    assert!(abs.is_empty());
    assert!(fixed.is_empty());

    // transform_el should collect the fixed child.
    let (abs2, fixed2) = collect_positioned_descendants(&dom, transform_el);
    assert!(abs2.is_empty());
    assert_eq!(fixed2.len(), 1);
    assert_eq!(fixed2[0], fixed_child);
}

/// horizontal-tb: inline over-constrained negative centering absorbs into end-side.
#[test]
fn horizontal_tb_inline_negative_centering() {
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(0.0),
            Some(900.0),
            Some(0.0),
            0.0,
            0.0,
            true,
            true,
            0.0,
            800.0,
            0.0,
        ),
        || 900.0,
    );
    // size(900) > containing(800): available = 800 - 0 - 900 - 0 - 0 = -100
    // Negative centering: margin_start = 0, margin_end absorbs = -100.
    assert!(approx_eq(r.size, 900.0));
    assert!(approx_eq(r.margin_start, 0.0));
    assert!(approx_eq(r.margin_end, -100.0));
}
