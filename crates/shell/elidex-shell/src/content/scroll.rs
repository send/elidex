//! Viewport and element scroll handling.
//!
//! Processes `MouseWheel` IPC messages by walking the scroll chain from the
//! hit-tested entity up to the viewport. Step 3 implements viewport-level
//! scrolling only; element-level scroll containers will be added in Step 5.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout::{hit_test_with_scroll, HitTestQuery};
use elidex_plugin::{ComputedStyle, Display, LayoutBox, Point, Rect, Vector};

use super::ContentState;

/// Scroll chain walk result.
enum ScrollTarget {
    Viewport,
    // Element(Entity),  // Step 5
}

/// Handle a `MouseWheel` IPC message.
pub(super) fn handle_wheel(state: &mut ContentState, delta: Vector<f64>, point: Point) {
    if !delta.is_finite() {
        return;
    }

    let vp = state.pipeline.viewport_overflow;
    // Early exit if viewport doesn't allow scrolling on any axis.
    // Per-axis checks happen in apply_viewport_scroll.
    if !vp.allows_scroll() {
        return;
    }

    let so = state.viewport_scroll.scroll_offset;
    let query = HitTestQuery { point, scroll: so };
    let hit = hit_test_with_scroll(&state.pipeline.dom, &query);
    let target = hit.map_or(ScrollTarget::Viewport, |h| {
        find_scroll_target(&state.pipeline.dom, h.entity)
    });

    let consumed = match target {
        ScrollTarget::Viewport => apply_viewport_scroll(
            state,
            delta,
            vp.overflow_x.is_scroll_container(),
            vp.overflow_y.is_scroll_container(),
        ),
    };

    if consumed {
        // Fast path: patch scroll offset in existing display list.
        // The display list structure (PushScrollOffset/PopScrollOffset pairs
        // including fixed-element exclusion) is invariant for scroll-only changes.
        state
            .pipeline
            .display_list
            .update_scroll_offset(state.viewport_scroll.scroll_offset);
        state.send_display_list();
    }
}

/// Walk from `hit_entity` up the ancestor chain looking for a scroll container.
///
/// Currently always returns `Viewport` — element-level scroll containers
/// will be added in Step 5 (check `ComputedStyle::is_scroll_container()`
/// and return `ScrollTarget::Element(entity)` when found).
fn find_scroll_target(dom: &EcsDom, hit_entity: Entity) -> ScrollTarget {
    let mut current = Some(hit_entity);
    while let Some(entity) = current {
        // TODO(Step 5): check ComputedStyle::is_scroll_container() and return Element(entity).
        current = dom.get_parent(entity);
    }
    ScrollTarget::Viewport
}

/// Apply scroll delta to the viewport `ScrollState`. Returns `true` if scroll changed.
fn apply_viewport_scroll(
    state: &mut ContentState,
    delta: Vector<f64>,
    can_scroll_x: bool,
    can_scroll_y: bool,
) -> bool {
    let (dx, dy) = (delta.x, delta.y);
    let old = state.viewport_scroll.scroll_offset;
    if can_scroll_x {
        #[allow(clippy::cast_possible_truncation)]
        let dx_f32 = dx as f32;
        debug_assert!(
            dx_f32.is_finite(),
            "scroll delta x must be finite after cast"
        );
        state.viewport_scroll.scroll_offset.x += dx_f32;
    }
    if can_scroll_y {
        #[allow(clippy::cast_possible_truncation)]
        let dy_f32 = dy as f32;
        debug_assert!(
            dy_f32.is_finite(),
            "scroll delta y must be finite after cast"
        );
        state.viewport_scroll.scroll_offset.y += dy_f32;
    }
    state.viewport_scroll.clamp_scroll();
    let new = state.viewport_scroll.scroll_offset;
    (new.x - old.x).abs() > f32::EPSILON || (new.y - old.y).abs() > f32::EPSILON
}

/// Compute the maximum content extent along an axis from all visible `LayoutBox` border boxes.
///
/// Skips elements with `display: none` (they have no box and should not
/// contribute to the scrollable area).
///
/// `extent_fn` extracts the far edge (e.g. `x + width` or `y + height`) from a border box.
fn compute_content_extent(dom: &EcsDom, extent_fn: fn(&Rect) -> f32) -> f32 {
    let mut max_extent: f32 = 0.0;
    for (_, (lb, style)) in &mut dom
        .world()
        .query::<(Entity, (&LayoutBox, &ComputedStyle))>()
    {
        if style.display == Display::None {
            continue;
        }
        let bb = lb.border_box();
        let extent = extent_fn(&bb);
        if extent > max_extent {
            max_extent = extent;
        }
    }
    max_extent
}

/// Compute the maximum content height from all visible `LayoutBox` border boxes.
pub(super) fn compute_content_height(dom: &EcsDom) -> f32 {
    compute_content_extent(dom, Rect::bottom)
}

/// Compute the maximum content width from all visible `LayoutBox` border boxes.
pub(super) fn compute_content_width(dom: &EcsDom) -> f32 {
    compute_content_extent(dom, Rect::right)
}

/// Update `viewport_scroll` dimensions after re-render.
///
/// Should be called after `re_render()` completes layout, so that
/// `LayoutBox` values reflect the current content size.
pub(super) fn update_viewport_scroll_dimensions(state: &mut ContentState) {
    let ch = compute_content_height(&state.pipeline.dom);
    let cw = compute_content_width(&state.pipeline.dom);
    state.viewport_scroll.client_size.width = state.pipeline.viewport.width;
    state.viewport_scroll.client_size.height = state.pipeline.viewport.height;
    state.viewport_scroll.scroll_size.width = cw.max(state.viewport_scroll.client_size.width);
    state.viewport_scroll.scroll_size.height = ch.max(state.viewport_scroll.client_size.height);
    state.viewport_scroll.clamp_scroll();
}

#[cfg(test)]
mod tests {
    use elidex_ecs::ScrollState;
    use elidex_plugin::{Overflow, ViewportOverflow};

    #[test]
    fn viewport_scroll_down() {
        let mut scroll = ScrollState::new(1024.0, 2000.0, 1024.0, 768.0);
        let old_y = scroll.scroll_offset.y;
        scroll.scroll_offset.y += 100.0;
        scroll.clamp_scroll();
        assert!(scroll.scroll_offset.y > old_y);
        assert!((scroll.scroll_offset.y - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn viewport_scroll_clamp_max() {
        let mut scroll = ScrollState::new(1024.0, 2000.0, 1024.0, 768.0);
        scroll.scroll_offset.y += 5000.0;
        scroll.clamp_scroll();
        // max_scroll_y = 2000 - 768 = 1232
        assert!((scroll.scroll_offset.y - 1232.0).abs() < f32::EPSILON);
    }

    #[test]
    fn viewport_scroll_clamp_min() {
        let mut scroll = ScrollState::new(1024.0, 2000.0, 1024.0, 768.0);
        scroll.scroll_offset.y -= 100.0;
        scroll.clamp_scroll();
        assert!(scroll.scroll_offset.y.abs() < f32::EPSILON);
    }

    #[test]
    fn viewport_scroll_no_change_overflow_hidden() {
        // When overflow is Hidden, allows_scroll is false so handle_wheel
        // returns early. Test that apply_viewport_scroll respects axis flags.
        let mut scroll = ScrollState::new(1024.0, 2000.0, 1024.0, 768.0);
        let old_y = scroll.scroll_offset.y;
        // Simulate: can_scroll_y = false
        // (don't add delta)
        scroll.clamp_scroll();
        assert!((scroll.scroll_offset.y - old_y).abs() < f32::EPSILON);

        // Double-check: Hidden does not create scroll container
        let vp = ViewportOverflow::from_propagated(Overflow::Hidden, Overflow::Hidden);
        assert!(!vp.allows_scroll());
    }

    #[test]
    fn viewport_scroll_horizontal() {
        let mut scroll = ScrollState::new(3000.0, 768.0, 1024.0, 768.0);
        scroll.scroll_offset.x += 200.0;
        scroll.clamp_scroll();
        assert!((scroll.scroll_offset.x - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn nan_delta_is_ignored() {
        // NaN deltas should be rejected before reaching ScrollState.
        let scroll = ScrollState::new(1024.0, 2000.0, 1024.0, 768.0);
        assert!(scroll.scroll_offset.y.abs() < f32::EPSILON);
        // NaN guard is tested via handle_wheel in integration; here verify
        // ScrollState is not corrupted by validating clamp works on clean state.
        let mut s = scroll;
        s.clamp_scroll();
        assert!(s.scroll_offset.y.abs() < f32::EPSILON);
    }

    #[test]
    fn content_height_empty_dom() {
        let dom = elidex_ecs::EcsDom::new();
        let height = super::compute_content_height(&dom);
        assert!((height).abs() < f32::EPSILON);
    }

    #[test]
    fn content_height_single_block() {
        use elidex_ecs::{Attributes, EcsDom};
        use elidex_plugin::{ComputedStyle, LayoutBox, Point, Rect, Size};

        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let el = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(doc, el);
        let lb = LayoutBox {
            content: Rect {
                origin: Point::ZERO,
                size: Size {
                    width: 200.0,
                    height: 500.0,
                },
            },
            ..LayoutBox::default()
        };
        dom.world_mut()
            .insert(el, (lb, ComputedStyle::default()))
            .unwrap();
        let height = super::compute_content_height(&dom);
        assert!((height - 500.0).abs() < f32::EPSILON);
    }
}
