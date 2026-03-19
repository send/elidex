//! Hit testing: find which DOM element is at a given viewport coordinate.
//!
//! Uses stacking context layer order (CSS 2.1 Appendix E) so that
//! higher z-index elements are hit first. Within each layer,
//! later DOM-order entities win (painter's order = front-most).

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::paint_order::{collect_sc_participants, is_positioned};
use elidex_plugin::transform_math::{
    compute_element_transform, invert_affine, is_affine_identity, mul_affine,
    resolve_child_perspective, Perspective, IDENTITY,
};
use elidex_plugin::{ComputedStyle, Display, LayoutBox};

/// Result of a hit test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitTestResult {
    /// The entity at the hit point (front-most in painter's order).
    pub entity: Entity,
}

/// Immutable query parameters for a hit test.
///
/// Groups viewport coordinates and scroll offset so they can be threaded
/// through the recursive walk by reference instead of as four separate scalars.
pub struct HitTestQuery {
    /// Viewport coordinates `(x, y)`.
    pub point: (f32, f32),
    /// Viewport scroll offset `(scroll_x, scroll_y)`.
    pub scroll: (f32, f32),
}

/// Transform + fixed/scroll state propagated through the hit test tree walk.
#[derive(Clone, Copy)]
struct TransformContext {
    cumulative: [f64; 6],
    perspective: Perspective,
    /// `true` when this element or an ancestor is viewport-fixed
    /// (`position: fixed` with no transform ancestor), meaning scroll
    /// offset is not applied for hit testing.
    in_fixed: bool,
    /// `true` when an ancestor has CSS transform (fixed behaves as absolute).
    in_transform: bool,
}

/// Find the front-most entity at viewport coordinates `(x, y)`.
///
/// Coordinates are viewport-relative (top-left = 0,0) and must be finite.
/// Returns `None` for non-finite inputs.
///
/// Uses stacking context layer order so that higher z-index elements win.
///
/// This is a convenience wrapper around [`hit_test_with_scroll`] with zero scroll offset.
#[must_use]
pub fn hit_test(dom: &EcsDom, point: (f32, f32)) -> Option<HitTestResult> {
    hit_test_with_scroll(
        dom,
        &HitTestQuery {
            point,
            scroll: (0.0, 0.0),
        },
    )
}

/// Find the front-most entity at viewport coordinates,
/// accounting for viewport scroll offset.
///
/// **Sign convention:** rendering translates content by `(-scroll.0, -scroll.1)`,
/// so to find which content-space element is under a viewport point we add
/// `+scroll` to the point: `content_coord = viewport_point + scroll_offset`.
/// `position: fixed` elements (without a transform ancestor) bypass this
/// adjustment and are tested at the original viewport `point`.
#[must_use]
pub fn hit_test_with_scroll(dom: &EcsDom, query: &HitTestQuery) -> Option<HitTestResult> {
    if !query.point.0.is_finite() || !query.point.1.is_finite() {
        return None;
    }
    debug_assert!(
        query.scroll.0.is_finite() && query.scroll.1.is_finite(),
        "scroll offset must be finite"
    );
    let mut result = None;
    let ctx = TransformContext {
        cumulative: IDENTITY,
        perspective: Perspective::default(),
        in_fixed: false,
        in_transform: false,
    };
    for root in dom.root_entities() {
        hit_test_subtree(dom, root, query, &mut result, 0, ctx);
    }
    result
}

/// Recursively walk the subtree rooted at `entity`.
///
/// For stacking contexts, children are tested in layer order (Layer 2 through 7).
/// "Last hit wins" within each layer, and higher layers overwrite lower ones.
fn hit_test_subtree(
    dom: &EcsDom,
    entity: Entity,
    query: &HitTestQuery,
    result: &mut Option<HitTestResult>,
    depth: u32,
    ctx: TransformContext,
) {
    if depth >= elidex_layout_block::MAX_LAYOUT_DEPTH {
        return;
    }

    // Skip display:none subtrees.
    let style_opt = dom.world().get::<&ComputedStyle>(entity).ok();
    let display = style_opt.as_ref().map_or(Display::default(), |s| s.display);
    if display == Display::None {
        return;
    }

    // CSS Transforms L1 §2: a transform ancestor makes fixed behave like absolute.
    let has_transform = style_opt.as_ref().is_some_and(|s| s.has_transform);
    let child_in_transform = ctx.in_transform || has_transform;

    // Determine if this element is viewport-fixed (position:fixed with no transform ancestor).
    let is_this_fixed = style_opt
        .as_ref()
        .is_some_and(|s| s.position == elidex_plugin::Position::Fixed);
    let effective_fixed = if is_this_fixed && !ctx.in_transform {
        true
    } else {
        ctx.in_fixed
    };

    // Compute transform for this element.
    let layout_box_opt = dom.world().get::<&LayoutBox>(entity).ok();
    let cached_bb = layout_box_opt.as_ref().map(|lb| lb.border_box());

    let mut local_transform = ctx.cumulative;
    if let (Some(ref style), Some(bb)) = (&style_opt, cached_bb) {
        if style.has_transform || ctx.perspective.distance.is_some() {
            if let Some(affine) = compute_element_transform(style, &bb, &ctx.perspective) {
                local_transform = mul_affine(ctx.cumulative, affine);
            } else {
                // backface-hidden and facing away — skip subtree
                return;
            }
        }
    }

    // Check if this entity's border box contains the point.
    // Fixed elements use viewport coordinates; others use scrolled coordinates.
    if let Some(bb) = cached_bb {
        let (hit_x, hit_y) = if effective_fixed {
            query.point
        } else {
            (
                query.point.0 + query.scroll.0,
                query.point.1 + query.scroll.1,
            )
        };
        let (test_x, test_y) = if is_affine_identity(&local_transform) {
            (hit_x, hit_y)
        } else if let Some(inv) = invert_affine(local_transform) {
            let lx = inv[0] * f64::from(hit_x) + inv[2] * f64::from(hit_y) + inv[4];
            let ly = inv[1] * f64::from(hit_x) + inv[3] * f64::from(hit_y) + inv[5];
            if !lx.is_finite() || !ly.is_finite() {
                (f32::MAX, f32::MAX)
            } else {
                #[allow(clippy::cast_possible_truncation)]
                (lx as f32, ly as f32)
            }
        } else {
            (hit_x, hit_y)
        };
        if test_x >= bb.x && test_x < bb.x + bb.width && test_y >= bb.y && test_y < bb.y + bb.height
        {
            *result = Some(HitTestResult { entity });
        }
    }

    // Compute perspective to propagate to children.
    let child_perspective = match (&style_opt, cached_bb) {
        (Some(style), Some(bb)) => resolve_child_perspective(style, &bb),
        _ => Perspective::default(),
    };

    let child_ctx = TransformContext {
        cumulative: local_transform,
        perspective: child_perspective,
        in_fixed: effective_fixed,
        in_transform: child_in_transform,
    };

    // Determine if this element is a stacking context.
    let is_sc = style_opt
        .as_ref()
        .is_some_and(|s| s.creates_stacking_context())
        || dom.get_parent(entity).is_none();

    if is_sc {
        hit_test_sc_layers(dom, entity, query, result, depth, child_ctx);
    } else {
        hit_test_non_sc(dom, entity, query, result, depth, child_ctx);
    }
}

/// Hit test children in stacking context layer order.
#[allow(clippy::similar_names)]
fn hit_test_sc_layers(
    dom: &EcsDom,
    entity: Entity,
    query: &HitTestQuery,
    result: &mut Option<HitTestResult>,
    depth: u32,
    ctx: TransformContext,
) {
    let children = elidex_layout_block::composed_children_flat(dom, entity);
    let parent_display = elidex_layout_block::try_get_style(dom, entity).map(|s| s.display);
    let layers = collect_sc_participants(dom, &children, parent_display);

    // Layer 2: negative z (z ascending → last wins for same z).
    for &child in &layers.negative_z {
        hit_test_subtree(dom, child, query, result, depth + 1, ctx);
    }
    // Layer 3: in-flow blocks.
    for &child in &layers.in_flow_blocks {
        hit_test_subtree(dom, child, query, result, depth + 1, ctx);
    }
    // Layer 4: floats.
    for &child in &layers.floats {
        hit_test_subtree(dom, child, query, result, depth + 1, ctx);
    }
    // Layer 5: inline (DOM order, non-positioned).
    for &child in &layers.all_children {
        if !is_positioned(dom, child) && !is_block_or_float(dom, child) {
            hit_test_subtree(dom, child, query, result, depth + 1, ctx);
        }
    }
    // Layer 6: positioned auto + z:0 (DOM order interleave).
    let mut layer6: Vec<Entity> = layers
        .positioned_auto
        .iter()
        .chain(layers.zero_z.iter())
        .copied()
        .collect();
    layer6.sort_by(|&a, &b| dom.tree_order_cmp(a, b));
    for &child in &layer6 {
        hit_test_subtree(dom, child, query, result, depth + 1, ctx);
    }
    // Layer 7: positive z (z ascending).
    for &child in &layers.positive_z {
        hit_test_subtree(dom, child, query, result, depth + 1, ctx);
    }
}

/// Hit test children of a non-SC element (DOM order, skip positioned).
fn hit_test_non_sc(
    dom: &EcsDom,
    entity: Entity,
    query: &HitTestQuery,
    result: &mut Option<HitTestResult>,
    depth: u32,
    ctx: TransformContext,
) {
    let children = elidex_layout_block::composed_children_flat(dom, entity);
    for child in children {
        if is_positioned(dom, child) {
            continue;
        }
        hit_test_subtree(dom, child, query, result, depth + 1, ctx);
    }
}

fn is_block_or_float(dom: &EcsDom, entity: Entity) -> bool {
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .is_some_and(|s| {
            elidex_layout_block::block::is_block_level(s.display)
                || s.float != elidex_plugin::Float::None
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{EdgeSizes, Position, Rect};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    fn set_layout(dom: &mut EcsDom, entity: Entity, x: f32, y: f32, w: f32, h: f32) {
        let lb = LayoutBox {
            content: Rect::new(x, y, w, h),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(entity, lb);
    }

    fn set_style(dom: &mut EcsDom, entity: Entity, display: Display) {
        let style = ComputedStyle {
            display,
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(entity, style);
    }

    #[test]
    fn empty_dom_returns_none() {
        let dom = EcsDom::new();
        assert_eq!(hit_test(&dom, (100.0, 100.0)), None);
    }

    #[test]
    fn miss_returns_none() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 100.0);
        // Click outside the box.
        assert_eq!(hit_test(&dom, (200.0, 200.0)), None);
    }

    #[test]
    fn single_box_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 10.0, 10.0, 100.0, 50.0);
        let result = hit_test(&dom, (50.0, 30.0));
        assert_eq!(result, Some(HitTestResult { entity: e }));
    }

    #[test]
    fn nested_returns_innermost() {
        let mut dom = EcsDom::new();
        let outer = elem(&mut dom, "div");
        let inner = elem(&mut dom, "span");
        let _ = dom.append_child(outer, inner);

        set_layout(&mut dom, outer, 0.0, 0.0, 200.0, 200.0);
        set_layout(&mut dom, inner, 50.0, 50.0, 50.0, 50.0);

        // Click inside inner.
        let result = hit_test(&dom, (60.0, 60.0));
        assert_eq!(result, Some(HitTestResult { entity: inner }));

        // Click outside inner but inside outer.
        let result = hit_test(&dom, (10.0, 10.0));
        assert_eq!(result, Some(HitTestResult { entity: outer }));
    }

    #[test]
    fn siblings_later_wins() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        let _ = dom.append_child(parent, a);
        let _ = dom.append_child(parent, b);

        // Overlapping boxes — b is later in tree order so it wins.
        set_layout(&mut dom, parent, 0.0, 0.0, 200.0, 200.0);
        set_layout(&mut dom, a, 0.0, 0.0, 100.0, 100.0);
        set_layout(&mut dom, b, 50.0, 50.0, 100.0, 100.0);

        let result = hit_test(&dom, (75.0, 75.0));
        assert_eq!(result, Some(HitTestResult { entity: b }));
    }

    #[test]
    fn display_none_skipped() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let hidden = elem(&mut dom, "span");
        let _ = dom.append_child(parent, hidden);

        set_layout(&mut dom, parent, 0.0, 0.0, 200.0, 200.0);
        set_layout(&mut dom, hidden, 0.0, 0.0, 200.0, 200.0);
        set_style(&mut dom, hidden, Display::None);

        // Hidden element should be skipped — parent wins.
        let result = hit_test(&dom, (50.0, 50.0));
        assert_eq!(result, Some(HitTestResult { entity: parent }));
    }

    #[test]
    fn edge_exact_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 10.0, 10.0, 100.0, 50.0);

        // Exactly at top-left corner.
        assert_eq!(
            hit_test(&dom, (10.0, 10.0)),
            Some(HitTestResult { entity: e })
        );
        // Exactly at bottom-right (exclusive) — miss.
        assert_eq!(hit_test(&dom, (110.0, 60.0)), None);
        // Just inside bottom-right.
        assert_eq!(
            hit_test(&dom, (109.99, 59.99)),
            Some(HitTestResult { entity: e })
        );
    }

    #[test]
    fn zero_size_box_no_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 50.0, 50.0, 0.0, 0.0);
        assert_eq!(hit_test(&dom, (50.0, 50.0)), None);
    }

    #[test]
    fn deep_nesting() {
        let mut dom = EcsDom::new();
        let mut parent = elem(&mut dom, "div");
        set_layout(&mut dom, parent, 0.0, 0.0, 500.0, 500.0);
        let root = parent;

        let mut deepest = parent;
        for i in 0..10 {
            let child = elem(&mut dom, "div");
            let _ = dom.append_child(parent, child);

            #[allow(clippy::cast_precision_loss)]
            let offset = (i + 1) as f32 * 10.0;
            set_layout(
                &mut dom,
                child,
                offset,
                offset,
                500.0 - offset * 2.0,
                500.0 - offset * 2.0,
            );
            parent = child;
            deepest = child;
        }
        let _ = root;

        let result = hit_test(&dom, (250.0, 250.0));
        assert_eq!(result, Some(HitTestResult { entity: deepest }));
    }

    #[test]
    fn with_padding_and_border() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        let lb = LayoutBox {
            content: Rect::new(20.0, 20.0, 60.0, 60.0),
            padding: EdgeSizes::uniform(5.0),
            border: EdgeSizes::uniform(5.0),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(e, lb);

        // Border box: x=10, y=10, w=80, h=80 → [10,90) × [10,90).
        assert_eq!(
            hit_test(&dom, (10.0, 10.0)),
            Some(HitTestResult { entity: e })
        );
        assert_eq!(hit_test(&dom, (9.0, 9.0)), None);
    }

    #[test]
    fn nan_returns_none() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 100.0);

        assert_eq!(hit_test(&dom, (f32::NAN, 50.0)), None);
        assert_eq!(hit_test(&dom, (50.0, f32::NAN)), None);
        assert_eq!(hit_test(&dom, (f32::INFINITY, 50.0)), None);
    }

    #[test]
    fn scroll_offset_shifts_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        // Element at (100, 100, 50, 50) in layout space.
        set_layout(&mut dom, e, 100.0, 100.0, 50.0, 50.0);

        // Without scroll, clicking (110, 110) hits.
        assert!(hit_test(&dom, (110.0, 110.0)).is_some());

        // With scroll_y=80, viewport (30, 30) maps to layout (30+0, 30+80) = (30, 110).
        // That misses the element at x=100.
        assert!(hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (30.0, 30.0),
                scroll: (0.0, 80.0)
            }
        )
        .is_none());

        // With scroll_y=80, viewport (110, 20) maps to layout (110, 100) → hit.
        assert_eq!(
            hit_test_with_scroll(
                &dom,
                &HitTestQuery {
                    point: (110.0, 20.0),
                    scroll: (0.0, 80.0)
                }
            ),
            Some(HitTestResult { entity: e })
        );
    }

    #[test]
    fn scroll_offset_misses() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 100.0);

        // Without scroll, (50, 50) hits.
        assert!(hit_test(&dom, (50.0, 50.0)).is_some());

        // With scroll_y=200, viewport (50, 50) maps to layout (50, 250) → miss.
        assert!(hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (50.0, 50.0),
                scroll: (0.0, 200.0)
            }
        )
        .is_none());
    }

    #[test]
    fn fixed_ignores_scroll() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        set_layout(&mut dom, parent, 0.0, 0.0, 1000.0, 2000.0);
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );

        let fixed = elem(&mut dom, "div");
        let _ = dom.append_child(parent, fixed);
        set_layout(&mut dom, fixed, 0.0, 0.0, 100.0, 50.0);
        let _ = dom.world_mut().insert_one(
            fixed,
            ComputedStyle {
                display: Display::Block,
                position: Position::Fixed,
                z_index: Some(0),
                ..Default::default()
            },
        );

        // Fixed element at (0,0,100,50) should be hit at viewport (50, 25)
        // regardless of scroll offset.
        assert_eq!(
            hit_test_with_scroll(
                &dom,
                &HitTestQuery {
                    point: (50.0, 25.0),
                    scroll: (0.0, 500.0)
                }
            ),
            Some(HitTestResult { entity: fixed })
        );
    }

    #[test]
    fn fixed_inside_transform_scrolls() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        set_layout(&mut dom, parent, 0.0, 0.0, 1000.0, 2000.0);
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                has_transform: true, // transform ancestor
                ..Default::default()
            },
        );

        let fixed = elem(&mut dom, "div");
        let _ = dom.append_child(parent, fixed);
        set_layout(&mut dom, fixed, 0.0, 0.0, 100.0, 50.0);
        let _ = dom.world_mut().insert_one(
            fixed,
            ComputedStyle {
                display: Display::Block,
                position: Position::Fixed,
                ..Default::default()
            },
        );

        // Fixed inside transform → behaves like absolute → scroll applies.
        // With scroll_y=200, viewport (50, 25) maps to layout (50, 225).
        // The fixed child at (0,0,100,50) is missed; parent (1000x2000) is hit.
        let result = hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (50.0, 25.0),
                scroll: (0.0, 200.0),
            },
        );
        assert_eq!(result, Some(HitTestResult { entity: parent }));

        // Without scroll, (50, 25) hits the fixed child (front-most).
        let result = hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (50.0, 25.0),
                scroll: (0.0, 0.0),
            },
        );
        assert_eq!(result, Some(HitTestResult { entity: fixed }));
    }

    #[test]
    fn backward_compat_hit_test() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 10.0, 10.0, 100.0, 50.0);

        // hit_test() should give same result as hit_test_with_scroll(0,0).
        let r1 = hit_test(&dom, (50.0, 30.0));
        let r2 = hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (50.0, 30.0),
                scroll: (0.0, 0.0),
            },
        );
        assert_eq!(r1, r2);
    }

    #[test]
    fn negative_scroll_offset_clamps_hit() {
        // Negative scroll offsets (after clamping, should be 0) — verify hit test
        // still works correctly even if a negative value slips through.
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 50.0);

        // With negative scroll (-10, -10), viewport (50, 25) maps to (40, 15).
        // Element at (0,0,100,50) still contains (40, 15) → hit.
        assert_eq!(
            hit_test_with_scroll(
                &dom,
                &HitTestQuery {
                    point: (50.0, 25.0),
                    scroll: (-10.0, -10.0),
                }
            ),
            Some(HitTestResult { entity: e })
        );
    }

    #[test]
    fn very_large_scroll_offset_misses() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 50.0);

        // Huge scroll offset pushes the test point far beyond the element.
        assert!(hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (50.0, 25.0),
                scroll: (100_000.0, 100_000.0),
            }
        )
        .is_none());
    }

    #[test]
    fn multiple_fixed_elements_z_order_with_scroll() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        set_layout(&mut dom, parent, 0.0, 0.0, 1000.0, 2000.0);
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );

        // Two overlapping fixed elements with different z-index.
        let low_z = elem(&mut dom, "div");
        let _ = dom.append_child(parent, low_z);
        set_layout(&mut dom, low_z, 0.0, 0.0, 100.0, 50.0);
        let _ = dom.world_mut().insert_one(
            low_z,
            ComputedStyle {
                display: Display::Block,
                position: Position::Fixed,
                z_index: Some(1),
                ..Default::default()
            },
        );

        let high_z = elem(&mut dom, "div");
        let _ = dom.append_child(parent, high_z);
        set_layout(&mut dom, high_z, 0.0, 0.0, 100.0, 50.0);
        let _ = dom.world_mut().insert_one(
            high_z,
            ComputedStyle {
                display: Display::Block,
                position: Position::Fixed,
                z_index: Some(5),
                ..Default::default()
            },
        );

        // Higher z-index fixed element wins, regardless of scroll.
        let result = hit_test_with_scroll(
            &dom,
            &HitTestQuery {
                point: (50.0, 25.0),
                scroll: (0.0, 800.0),
            },
        );
        assert_eq!(result, Some(HitTestResult { entity: high_z }));
    }

    #[test]
    fn z_hit_test_respects_order() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        set_layout(&mut dom, parent, 0.0, 0.0, 200.0, 200.0);
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                z_index: Some(0),
                position: Position::Relative,
                ..Default::default()
            },
        );

        // low-z child (z:1) placed first in DOM
        let low = elem(&mut dom, "div");
        let _ = dom.append_child(parent, low);
        set_layout(&mut dom, low, 0.0, 0.0, 100.0, 100.0);
        let _ = dom.world_mut().insert_one(
            low,
            ComputedStyle {
                display: Display::Block,
                position: Position::Absolute,
                z_index: Some(1),
                ..Default::default()
            },
        );

        // high-z child (z:5) placed second but overlapping
        let high = elem(&mut dom, "div");
        let _ = dom.append_child(parent, high);
        set_layout(&mut dom, high, 0.0, 0.0, 100.0, 100.0);
        let _ = dom.world_mut().insert_one(
            high,
            ComputedStyle {
                display: Display::Block,
                position: Position::Absolute,
                z_index: Some(5),
                ..Default::default()
            },
        );

        // high-z should win
        let result = hit_test(&dom, (50.0, 50.0));
        assert_eq!(result, Some(HitTestResult { entity: high }));
    }
}
