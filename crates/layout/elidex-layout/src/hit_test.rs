//! Hit testing: find which DOM element is at a given viewport coordinate.
//!
//! Uses a pre-order DOM traversal checking `LayoutBox::border_box()`
//! containment. The last entity hit wins (painter's order = front-most).

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::transform_math::{
    compute_element_transform, invert_affine, is_affine_identity, mul_affine,
    resolve_child_perspective, IDENTITY,
};
use elidex_plugin::{ComputedStyle, Display, LayoutBox};

/// Result of a hit test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitTestResult {
    /// The entity at the hit point (front-most in painter's order).
    pub entity: Entity,
}

/// Transform state propagated through the hit test tree walk.
#[derive(Clone, Copy)]
struct TransformContext {
    cumulative: [f64; 6],
    parent_perspective: Option<f32>,
    parent_perspective_origin: (f64, f64),
}

/// Find the front-most entity at viewport coordinates `(x, y)`.
///
/// Coordinates are viewport-relative (top-left = 0,0) and must be finite.
/// Returns `None` for non-finite inputs.
///
/// Performs a pre-order traversal of the DOM tree, checking each entity's
/// `LayoutBox::border_box()` for containment. The last hit wins because
/// later elements in pre-order are painted on top (painter's order).
/// Right and bottom edges are exclusive (half-open interval `[x, x+w)`).
///
/// Entities with `display: none` are skipped along with their subtrees.
#[must_use]
pub fn hit_test(dom: &EcsDom, x: f32, y: f32) -> Option<HitTestResult> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let mut result = None;
    let ctx = TransformContext {
        cumulative: IDENTITY,
        parent_perspective: None,
        parent_perspective_origin: (0.0, 0.0),
    };
    for root in dom.root_entities() {
        hit_test_subtree(dom, root, x, y, &mut result, 0, ctx);
    }
    result
}

/// Recursively walk the subtree rooted at `entity` in pre-order.
fn hit_test_subtree(
    dom: &EcsDom,
    entity: Entity,
    x: f32,
    y: f32,
    result: &mut Option<HitTestResult>,
    depth: u32,
    ctx: TransformContext,
) {
    if depth > elidex_layout_block::MAX_LAYOUT_DEPTH {
        return;
    }

    // Skip display:none subtrees.
    let style_opt = dom.world().get::<&ComputedStyle>(entity).ok();
    let display = style_opt.as_ref().map_or(Display::default(), |s| s.display);
    if display == Display::None {
        return;
    }

    // Compute transform for this element.
    // Cache LayoutBox to avoid duplicate ECS queries (used for transform, hit test, perspective).
    let layout_box_opt = dom.world().get::<&LayoutBox>(entity).ok();
    let cached_bb = layout_box_opt.as_ref().map(|lb| lb.border_box());

    let mut local_transform = ctx.cumulative;
    if let (Some(ref style), Some(bb)) = (&style_opt, cached_bb) {
        // Match builder/transform.rs: apply transform when element has own transform
        // OR when parent perspective is present (perspective warps non-transformed children too).
        if style.has_transform || ctx.parent_perspective.is_some() {
            if let Some(affine) = compute_element_transform(
                style,
                &bb,
                ctx.parent_perspective,
                ctx.parent_perspective_origin,
            ) {
                local_transform = mul_affine(ctx.cumulative, affine);
            } else {
                // backface-hidden and facing away — skip subtree
                return;
            }
        }
    }

    // Check if this entity's border box contains the point.
    if let Some(bb) = cached_bb {
        // Transform the test point into local coordinates.
        let (test_x, test_y) = if is_affine_identity(&local_transform) {
            (x, y)
        } else if let Some(inv) = invert_affine(local_transform) {
            let lx = inv[0] * f64::from(x) + inv[2] * f64::from(y) + inv[4];
            let ly = inv[1] * f64::from(x) + inv[3] * f64::from(y) + inv[5];
            if !lx.is_finite() || !ly.is_finite() {
                // Degenerate inverse transform — skip hit test for this element.
                (f32::MAX, f32::MAX)
            } else {
                #[allow(clippy::cast_possible_truncation)]
                (lx as f32, ly as f32)
            }
        } else {
            (x, y)
        };
        if test_x >= bb.x && test_x < bb.x + bb.width && test_y >= bb.y && test_y < bb.y + bb.height
        {
            *result = Some(HitTestResult { entity });
        }
    }

    // Compute perspective to propagate to children.
    let (child_perspective, child_perspective_origin) = match (&style_opt, cached_bb) {
        (Some(style), Some(bb)) => resolve_child_perspective(style, &bb),
        _ => (None, (0.0, 0.0)),
    };

    // Walk children in order (pre-order: parent before children).
    let child_ctx = TransformContext {
        cumulative: local_transform,
        parent_perspective: child_perspective,
        parent_perspective_origin: child_perspective_origin,
    };
    for child in dom.children_iter(entity) {
        hit_test_subtree(dom, child, x, y, result, depth + 1, child_ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{EdgeSizes, Rect};

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
        assert_eq!(hit_test(&dom, 100.0, 100.0), None);
    }

    #[test]
    fn miss_returns_none() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 100.0);
        // Click outside the box.
        assert_eq!(hit_test(&dom, 200.0, 200.0), None);
    }

    #[test]
    fn single_box_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 10.0, 10.0, 100.0, 50.0);
        let result = hit_test(&dom, 50.0, 30.0);
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
        let result = hit_test(&dom, 60.0, 60.0);
        assert_eq!(result, Some(HitTestResult { entity: inner }));

        // Click outside inner but inside outer.
        let result = hit_test(&dom, 10.0, 10.0);
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

        let result = hit_test(&dom, 75.0, 75.0);
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
        let result = hit_test(&dom, 50.0, 50.0);
        assert_eq!(result, Some(HitTestResult { entity: parent }));
    }

    #[test]
    fn edge_exact_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 10.0, 10.0, 100.0, 50.0);

        // Exactly at top-left corner.
        assert_eq!(
            hit_test(&dom, 10.0, 10.0),
            Some(HitTestResult { entity: e })
        );
        // Exactly at bottom-right (exclusive) — miss.
        assert_eq!(hit_test(&dom, 110.0, 60.0), None);
        // Just inside bottom-right.
        assert_eq!(
            hit_test(&dom, 109.99, 59.99),
            Some(HitTestResult { entity: e })
        );
    }

    #[test]
    fn zero_size_box_no_hit() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 50.0, 50.0, 0.0, 0.0);
        assert_eq!(hit_test(&dom, 50.0, 50.0), None);
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

        let result = hit_test(&dom, 250.0, 250.0);
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
            hit_test(&dom, 10.0, 10.0),
            Some(HitTestResult { entity: e })
        );
        assert_eq!(hit_test(&dom, 9.0, 9.0), None);
    }

    #[test]
    fn nan_returns_none() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        set_layout(&mut dom, e, 0.0, 0.0, 100.0, 100.0);

        assert_eq!(hit_test(&dom, f32::NAN, 50.0), None);
        assert_eq!(hit_test(&dom, 50.0, f32::NAN), None);
        assert_eq!(hit_test(&dom, f32::INFINITY, 50.0), None);
    }
}
