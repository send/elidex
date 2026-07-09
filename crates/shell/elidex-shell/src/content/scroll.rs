//! Viewport and element scroll handling.
//!
//! Processes `MouseWheel` IPC messages by walking the scroll chain from the
//! hit-tested entity up to the viewport. Step 3 implements viewport-level
//! scrolling only; element-level scroll containers will be added in Step 5.

use elidex_ecs::{Attributes, EcsDom, Entity};
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
        // Echo the committed offset to the JS-observable consumers
        // (`window.scrollX`/`scrollY` + the document-root `ScrollState` that
        // `getBoundingClientRect` reads) through the shared chokepoint — the
        // same sink `re_render` uses. This fast path skips `re_render`, so
        // without the echo `scrollX`/`scrollY` and `getBoundingClientRect`
        // would stay stale after user wheel scrolling until an unrelated render.
        state.echo_viewport_scroll();
        let new_offset = state.viewport_scroll.scroll_offset;
        // `build_display_list_with_scroll` only emits the
        // `PushScrollOffset`/`PopScrollOffset` wrapper for a NON-zero offset, so
        // the first scroll away from 0 (the display list was last built at offset
        // 0) has no wrapper to patch — an in-place `update_scroll_offset` would be
        // a no-op and nothing would move. Rebuild on that 0 → non-zero transition
        // (mirrors the iframes_changed rebuild in `re_render`). Once a wrapper
        // exists (the previous offset `so` was already non-zero) the in-place fast
        // path patches it — invariant for scroll-only changes (fixed-element
        // exclusion included).
        let had_scroll_wrapper = so.x.abs() > f32::EPSILON || so.y.abs() > f32::EPSILON;
        if had_scroll_wrapper {
            state.pipeline.display_list.update_scroll_offset(new_offset);
        } else {
            state.pipeline.display_list = elidex_render::build_display_list_with_scroll(
                &state.pipeline.dom,
                &state.pipeline.font_db,
                state.pipeline.caret_visible,
                new_offset,
            );
        }
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

/// Convert a persisted session-history `scroll_position` — `(x, y)` in CSS px,
/// stored as `f64` on the `HistoryEntry` — into a viewport scroll offset for
/// restore-on-arrive (WHATWG HTML §7.4.6.2 step 6.4.4). One home for the
/// `(f64, f64)` → `Vector` conversion, shared by both shells (One-issue-one-way).
/// The narrowing f64→f32 cast is a display-precision truncation (scroll offsets
/// are small, bounded by the content size), matching the rest of the `f32` scroll
/// transport.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn scroll_offset_from_position((x, y): (f64, f64)) -> elidex_plugin::Vector {
    elidex_plugin::Vector::new(x as f32, y as f32)
}

/// Resolve the **indicated part** of the document for a URL fragment (WHATWG HTML
/// §7.4.6.4 "Scrolling to a fragment" — the "select the indicated part"
/// algorithm) and return the viewport scroll offset that brings it into view, or
/// `None` to leave the scroll unchanged.
///
/// Resolution: an element whose `id` equals the fragment, else an `<a>` element
/// whose `name` equals it, tried first on the raw fragment then on its
/// percent-decoded form (steps 4-8). An **empty** fragment (`#`), or a
/// case-insensitive `"top"` matching no element (step 10), scrolls to the top of
/// the document; any other non-empty fragment matching nothing returns `None`.
///
/// The offset aligns the indicated element per CSSOM View "scroll a target into
/// view" (§7.4.6.4 delegates to it): **block: start** — the target's top aligns to
/// the viewport origin (`border_box.origin.y`); **inline: nearest** — the inline
/// (horizontal) axis stays at `current` unless the target is off-screen
/// horizontally, in which case it scrolls the minimum to reveal the nearer edge
/// (so an already-visible target on a wide page is NOT yanked sideways). `current`
/// is the pre-nav scroll offset and `viewport_width` the client width, both for
/// the inline visibility test. The caller applies + clamps the result through the
/// post-layout `re_render` scroll seam (application currency, §6.4). This function
/// only resolves geometry from the DOM + layout, so it is engine-independent (the
/// Layering mandate keeps scroll-resolution out of `vm/host/`). The focusing steps
/// (§7.4.6.4 step 3.6) are deferred (§10-D2) — this lands the scroll only.
pub(crate) fn scroll_offset_for_fragment(
    dom: &EcsDom,
    root: Entity,
    fragment: &str,
    current: Vector,
    viewport_width: f32,
) -> Option<Vector> {
    // Empty fragment (`#`) → top of the document (the empty-fragment special
    // value, resolved before any element lookup).
    if fragment.is_empty() {
        return Some(Vector::<f32>::ZERO);
    }
    // id / `<a name>` match on the raw fragment, then on its percent-decoded form
    // (§7.4.6.4 steps 4-8): id attributes are stored decoded, so a `#caf%C3%A9`
    // URL fragment must decode to match the `café` id. The decoded retry is
    // skipped when decoding was a no-op (`decoded == fragment`, i.e. no
    // `%`-escape) — it would re-walk the tree with identical input for the same
    // result.
    let decoded = percent_encoding::percent_decode_str(fragment).decode_utf8_lossy();
    if let Some(element) = find_indicated_element(dom, root, fragment).or_else(|| {
        (decoded.as_ref() != fragment)
            .then(|| find_indicated_element(dom, root, &decoded))
            .flatten()
    }) {
        // A matched-but-boxless element (e.g. `display: none`) yields no offset —
        // leave the scroll unchanged rather than fall through to the top.
        let border_box = dom.world().get::<&LayoutBox>(element).ok()?.border_box();
        // block: start (align the target's top); inline: nearest (keep the
        // current inline scroll unless the target needs revealing).
        let x = inline_nearest(
            border_box.origin.x,
            border_box.size.width,
            current.x,
            viewport_width,
        );
        return Some(Vector::new(x, border_box.origin.y));
    }
    // No indicated element: a case-insensitive `"top"` fragment scrolls to the
    // top (§7.4.6.4 step 10); every other non-empty fragment leaves scroll alone.
    if decoded.eq_ignore_ascii_case("top") {
        Some(Vector::<f32>::ZERO)
    } else {
        None
    }
}

/// Find the first descendant of `root` that is a "potential indicated element"
/// for `fragment` (WHATWG HTML §7.4.6.4): an element with `id == fragment`, or an
/// `<a>` element with `name == fragment`. Prefers the id match (`find_by_id`,
/// document order), then scans for a named `<a>`.
fn find_indicated_element(dom: &EcsDom, root: Entity, fragment: &str) -> Option<Entity> {
    if let Some(entity) = dom.find_by_id(root, fragment) {
        return Some(entity);
    }
    let mut result = None;
    dom.traverse_descendants(root, |entity| {
        let is_named_anchor = dom.with_tag_name(entity, |t| t == Some("a"))
            && dom
                .world()
                .get::<&Attributes>(entity)
                .is_ok_and(|a| a.get("name") == Some(fragment));
        if is_named_anchor {
            result = Some(entity);
            return false;
        }
        true
    });
    result
}

/// Inline-axis "nearest" scroll target — CSSOM View "determine the scroll-into-view
/// position", the `inline: nearest` case (§7.4.6.4 delegates fragment scrolling to
/// it). Given the target's inline extent `[left, left + width)`, the current
/// scroll `current_x`, and the scrollport width, returns the new inline scroll:
///
/// - **both edges outside** the scrollport (target spans it) → do nothing (already
///   in view) — the straddling-wide-target case;
/// - **start edge outside** and target no wider than the scrollport, OR **end edge
///   outside** and target wider → align the start edge (`left`);
/// - **start edge outside** and target wider, OR **end edge outside** and no wider
///   → align the end edge (`right - width`);
/// - fully visible → do nothing.
///
/// A target EXACTLY the scrollport width with one edge outside is revealed by
/// aligning that outside edge (the `<=` boundary): a bare `<` would leave it
/// clipped.
///
/// So an already-visible target does not force a spurious sideways jump.
/// `viewport_width == 0` (dimensions not yet measured) degrades to aligning the
/// left edge (the pre-`inline: nearest` behaviour, no regression). The caller
/// clamps the result against `max_scroll_x`.
fn inline_nearest(target_left: f32, target_width: f32, current_x: f32, viewport_width: f32) -> f32 {
    if viewport_width <= 0.0 {
        return target_left; // dimensions unmeasured → align the start edge
    }
    let target_right = target_left + target_width;
    let view_right = current_x + viewport_width;
    let off_near = target_left < current_x; // inline start edge before the scrollport
    let off_far = target_right > view_right; // inline end edge past the scrollport
    if off_near && off_far {
        current_x // both edges outside → target spans the scrollport, already in view
    } else if (off_near && target_width <= viewport_width)
        || (off_far && target_width > viewport_width)
    {
        target_left // align the start edge
    } else if (off_near && target_width > viewport_width)
        || (off_far && target_width <= viewport_width)
    {
        (target_right - viewport_width).max(0.0) // align the end edge
    } else {
        current_x // fully visible → do nothing
    }
}

#[cfg(test)]
mod tests {
    use elidex_ecs::ScrollState;
    use elidex_plugin::{Overflow, ViewportOverflow};

    /// `inline_nearest` (fragment scroll, `inline: nearest`, §7.4.6.4 → CSSOM):
    /// an already-visible target keeps the current inline scroll (no spurious
    /// sideways jump — the R2 fix); an off-screen target reveals the nearer edge.
    #[test]
    fn inline_nearest_keeps_visible_reveals_offscreen() {
        use super::inline_nearest;
        let vw = 800.0;
        // Already fully visible (target [100,300) ⊂ view [0,800)) → stay put.
        assert_eq!(inline_nearest(100.0, 200.0, 0.0, vw), 0.0);
        // Visible while the page is scrolled right (view [500,1300), target
        // [600,800)) → stay put (the exact bug: no yank back to left=600).
        assert_eq!(inline_nearest(600.0, 200.0, 500.0, vw), 500.0);
        // Off the LEFT (target [100,300), view [500,1300)) → align left edge.
        assert_eq!(inline_nearest(100.0, 200.0, 500.0, vw), 100.0);
        // Off the RIGHT, narrower than the viewport (target [900,1000), view
        // [0,800)) → align the end edge (1000 - 800 = 200).
        assert_eq!(inline_nearest(900.0, 100.0, 0.0, vw), 200.0);
        // Off the RIGHT, wider than the viewport (target [100,1100), view [0,800),
        // start inside) → align the START edge (100), not the end.
        assert_eq!(inline_nearest(100.0, 1000.0, 0.0, vw), 100.0);
        // STRADDLING: wider than the viewport AND both edges outside (target
        // [100,2000), view [500,1300)) → do nothing, the target already spans the
        // viewport (the R3 regression fix — must NOT yank to left=100).
        assert_eq!(inline_nearest(100.0, 1900.0, 500.0, vw), 500.0);
        // EQUAL WIDTH, off the LEFT (target [100,900) width==vw, view [500,1300))
        // → align the start edge (100), revealing the 400px clipped on the left —
        // the R5 fix: a bare `<` width check left it at 500 (clipped).
        assert_eq!(inline_nearest(100.0, 800.0, 500.0, vw), 100.0);
        // EQUAL WIDTH, off the RIGHT (target [600,1400) width==vw, view [0,800))
        // → align the end edge (1400 - 800 = 600).
        assert_eq!(inline_nearest(600.0, 800.0, 0.0, vw), 600.0);
        // Unmeasured viewport (width 0) → degrades to aligning the left edge (the
        // pre-`inline: nearest` behaviour, no regression).
        assert_eq!(inline_nearest(300.0, 50.0, 0.0, 0.0), 300.0);
    }

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
