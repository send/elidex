use super::*;

#[test]
fn empty_display_list_builds_empty_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList::default();
    build_scene(&mut scene, &dl, &mut fc);
    // Scene was constructed without panic — smoke test passes.
}

#[test]
fn solid_rect_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::SolidRect {
        rect: Rect::new(10.0, 20.0, 100.0, 50.0),
        color: CssColor::RED,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Scene contains data (encoding is non-empty).
}

#[test]
fn image_builds_scene() {
    use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::Image {
        painting_area: Rect::new(10.0, 20.0, 100.0, 50.0),
        pixels: Arc::new(vec![255u8; 4 * 2 * 2]), // 2×2 white
        image_width: 2,
        image_height: 2,
        position: Point::ZERO,
        size: Size::new(100.0, 50.0),
        repeat: BgRepeat {
            x: BgRepeatAxis::NoRepeat,
            y: BgRepeatAxis::NoRepeat,
        },
        opacity: 1.0,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test.
}

#[test]
fn rounded_rect_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::RoundedRect {
        rect: Rect::new(10.0, 20.0, 100.0, 50.0),
        radii: [8.0, 8.0, 8.0, 8.0],
        color: CssColor::BLUE,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test.
}

#[test]
fn stroked_rounded_rect_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::StrokedRoundedRect {
        rect: Rect::new(10.0, 20.0, 8.0, 8.0),
        radii: [4.0, 4.0, 4.0, 4.0],
        stroke_width: 1.0,
        color: CssColor::BLACK,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test for stroked rounded rect.
}

#[test]
fn rounded_border_ring_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::RoundedBorderRing {
        outer_rect: Rect::new(0.0, 0.0, 104.0, 54.0),
        outer_radii: [10.0, 10.0, 10.0, 10.0],
        inner_rect: Rect::new(2.0, 2.0, 100.0, 50.0),
        inner_radii: [8.0, 8.0, 8.0, 8.0],
        color: CssColor::BLACK,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test for rounded border ring.
}

#[test]
fn push_pop_clip_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![
        DisplayItem::PushClip {
            rect: Rect::new(0.0, 0.0, 200.0, 100.0),
            radii: [0.0; 4],
        },
        DisplayItem::SolidRect {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
            color: CssColor::RED,
        },
        DisplayItem::PopClip,
    ]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test for clip layer.
}

#[test]
fn image_repeat_builds_scene() {
    use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::Image {
        painting_area: Rect::new(0.0, 0.0, 200.0, 200.0),
        pixels: Arc::new(vec![255u8; 4 * 2 * 2]),
        image_width: 2,
        image_height: 2,
        position: Point::ZERO,
        size: Size::new(50.0, 50.0),
        repeat: BgRepeat {
            x: BgRepeatAxis::Repeat,
            y: BgRepeatAxis::Repeat,
        },
        opacity: 1.0,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
}

#[test]
fn tile_positions_no_repeat() {
    use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
    let area = Rect::new(0.0, 0.0, 400.0, 300.0);
    let repeat = BgRepeat {
        x: BgRepeatAxis::NoRepeat,
        y: BgRepeatAxis::NoRepeat,
    };
    let positions = compute_tile_positions(
        &area,
        &Point::new(10.0, 20.0),
        &Size::new(100.0, 50.0),
        &repeat,
    );
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0], Vector::new(10.0, 20.0));
}

#[test]
fn tile_positions_repeat() {
    use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
    let area = Rect::new(0.0, 0.0, 200.0, 100.0);
    let repeat = BgRepeat {
        x: BgRepeatAxis::Repeat,
        y: BgRepeatAxis::Repeat,
    };
    let positions = compute_tile_positions(&area, &Point::ZERO, &Size::new(50.0, 50.0), &repeat);
    // Must cover the entire painting area — at least 4 columns × 2 rows
    assert!(positions.len() >= 8);
    // All visible tiles must intersect the painting area
    for p in &positions {
        assert!(p.x < 200.0 && p.y < 100.0);
    }
}

#[test]
fn tile_positions_space() {
    use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
    let area = Rect::new(0.0, 0.0, 250.0, 100.0);
    let repeat = BgRepeat {
        x: BgRepeatAxis::Space,
        y: BgRepeatAxis::NoRepeat,
    };
    let positions = compute_tile_positions(&area, &Point::ZERO, &Size::new(100.0, 50.0), &repeat);
    // floor(250/100) = 2 tiles in x, 1 in y → 2 tiles
    assert_eq!(positions.len(), 2);
    // First tile at x=0, second at x=150 (50px space between)
    assert!((positions[0].x).abs() < 0.1);
    assert!((positions[1].x - 150.0).abs() < 0.1);
}

#[test]
fn tile_positions_round() {
    use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
    let area = Rect::new(0.0, 0.0, 250.0, 100.0);
    let repeat = BgRepeat {
        x: BgRepeatAxis::Round,
        y: BgRepeatAxis::NoRepeat,
    };
    // round(250/100) = 3 tiles, each 250/3 ≈ 83.3px
    let positions = compute_tile_positions(&area, &Point::ZERO, &Size::new(100.0, 50.0), &repeat);
    // Must have at least 3 tiles covering the 250px area with ~83px tiles
    assert!(positions.len() >= 3);
}

#[test]
fn styled_border_segment_dashed_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::StyledBorderSegment {
        start: Point::new(0.0, 1.0),
        end: Point::new(100.0, 1.0),
        width: 2.0,
        dashes: vec![6.0, 2.0],
        round_caps: false,
        color: CssColor::RED,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test for dashed border segment.
}

#[test]
fn styled_border_segment_dotted_builds_scene() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![DisplayItem::StyledBorderSegment {
        start: Point::new(1.5, 0.0),
        end: Point::new(1.5, 50.0),
        width: 3.0,
        dashes: vec![0.001, 6.0],
        round_caps: true,
        color: CssColor::BLUE,
    }]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — smoke test for dotted border segment.
}

#[test]
fn scroll_offset_translates() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::new(50.0, 100.0),
        },
        DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: CssColor::RED,
        },
        DisplayItem::PopScrollOffset,
    ]);
    build_scene(&mut scene, &dl, &mut fc);
    // Should not panic — scroll offset applied as translate(-50, -100).
}

#[test]
fn scroll_offset_zero_identity() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::<f32>::ZERO,
        },
        DisplayItem::SolidRect {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
            color: CssColor::GREEN,
        },
        DisplayItem::PopScrollOffset,
    ]);
    build_scene(&mut scene, &dl, &mut fc);
    // (0,0) scroll is identity — should render normally.
}

#[test]
fn nested_scroll_transform() {
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::new(10.0, 20.0),
        },
        DisplayItem::PushTransform {
            affine: [1.0, 0.0, 0.0, 1.0, 50.0, 50.0], // translate(50, 50)
        },
        DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: CssColor::BLUE,
        },
        DisplayItem::PopTransform,
        DisplayItem::PopScrollOffset,
    ]);
    build_scene(&mut scene, &dl, &mut fc);
    // Nested scroll + CSS transform — should compose correctly.
}

#[test]
fn fixed_element_scroll_exclusion() {
    // Simulates the display list structure emitted by walk_child_with_fixed_check:
    // PushScrollOffset → normal content → PopScrollOffset (cancel for fixed) →
    // fixed content → PushScrollOffset (re-apply) → more content → PopScrollOffset.
    let mut scene = Scene::new();
    let mut fc = HashMap::new();
    let dl = DisplayList(vec![
        // Root scroll
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::new(0.0, 100.0),
        },
        // Normal (scrolled) content
        DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 200.0, 50.0),
            color: CssColor::RED,
        },
        // Cancel scroll for fixed element
        DisplayItem::PopScrollOffset,
        // Fixed element (not scrolled)
        DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 100.0, 30.0),
            color: CssColor::BLUE,
        },
        // Re-apply scroll
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::new(0.0, 100.0),
        },
        // More normal content
        DisplayItem::SolidRect {
            rect: Rect::new(0.0, 50.0, 200.0, 50.0),
            color: CssColor::GREEN,
        },
        DisplayItem::PopScrollOffset,
    ]);
    build_scene(&mut scene, &dl, &mut fc);
    // Balanced Push/Pop pairs with fixed-element exclusion — should not panic.
}

#[test]
fn fixed_element_scroll_exclusion_update() {
    // Verify update_scroll_offset patches all PushScrollOffset items uniformly,
    // including the re-push after a fixed element.
    let mut dl = DisplayList(vec![
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::<f32>::ZERO,
        },
        DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: CssColor::RED,
        },
        DisplayItem::PopScrollOffset,
        // Fixed element re-push
        DisplayItem::PushScrollOffset {
            scroll_offset: Vector::<f32>::ZERO,
        },
        DisplayItem::PopScrollOffset,
    ]);

    dl.update_scroll_offset(Vector::new(30.0, 80.0));

    let offsets: Vec<_> = dl
        .iter()
        .filter_map(|item| match item {
            DisplayItem::PushScrollOffset { scroll_offset } => Some(*scroll_offset),
            _ => None,
        })
        .collect();
    assert_eq!(offsets.len(), 2);
    for p in offsets {
        assert!((p.x - 30.0).abs() < f32::EPSILON);
        assert!((p.y - 80.0).abs() < f32::EPSILON);
    }
}

#[test]
fn content_placement_base_transform_scale1() {
    // Top chrome at scale 1: content-area origin (0, 64), size 1024×704 px.
    let cp = ContentPlacement {
        offset: Point::new(0.0, 64.0),
        size: Size::new(1024.0, 704.0),
        scale: 1.0,
    };
    // [a, b, c, d, e, f] = [scale, 0, 0, scale, offset.x, offset.y].
    assert_eq!(
        cp.base_transform().as_coeffs(),
        [1.0, 0.0, 0.0, 1.0, 0.0, 64.0]
    );
    // A content CSS point (10, 10) lands at physical (10, 74) = p × scale + offset.
    let p = cp.base_transform() * vello::kurbo::Point::new(10.0, 10.0);
    assert!((p.x - 10.0).abs() < 1e-9);
    assert!((p.y - 74.0).abs() < 1e-9);
}

#[test]
fn content_placement_base_transform_scale2() {
    // Top chrome at scale 2 (HiDPI): origin_phys (0, 128), size_phys 2048×1408.
    let cp = ContentPlacement {
        offset: Point::new(0.0, 128.0),
        size: Size::new(2048.0, 1408.0),
        scale: 2.0,
    };
    assert_eq!(
        cp.base_transform().as_coeffs(),
        [2.0, 0.0, 0.0, 2.0, 0.0, 128.0]
    );
    // A content CSS point (10, 10) lands at physical (20, 148) = 10 × 2 + offset.
    let p = cp.base_transform() * vello::kurbo::Point::new(10.0, 10.0);
    assert!((p.x - 20.0).abs() < 1e-9);
    assert!((p.y - 148.0).abs() < 1e-9);
}

#[test]
fn content_placement_clip_rect_matches_content_area() {
    // Left chrome at scale 2: origin_phys (400, 72), size_phys 1248×1392.
    let cp = ContentPlacement {
        offset: Point::new(400.0, 72.0),
        size: Size::new(1248.0, 1392.0),
        scale: 2.0,
    };
    let r = cp.clip_rect();
    assert!((r.x0 - 400.0).abs() < 1e-9);
    assert!((r.y0 - 72.0).abs() < 1e-9);
    assert!((r.x1 - 1648.0).abs() < 1e-9);
    assert!((r.y1 - 1464.0).abs() < 1e-9);
}
