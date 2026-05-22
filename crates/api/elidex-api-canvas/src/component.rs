//! Per-canvas-entity 2D context plumbing (ECS-native).
//!
//! The raster state ([`Canvas2dContext`], `Send + Sync + 'static`) lives as a
//! **component on the canvas `Element` entity**, not in a host-side registry —
//! per the side-store audit a `Send + Sync` per-entity value belongs on the
//! entity (SameObject = component get, despawn = automatic drop). This crate is
//! the engine-independent home for the component plumbing: insert/query/dirty +
//! the per-frame `ImageData` sync + the width/height bitmap-reset reconciler.
//! The JS wrapper *identity* (`canvas.getContext('2d')` SameObject) is handled
//! separately by the binding crate's wrapper-identity seam (the per-VM
//! `ObjectId` is not a component — it aliases cross-DOM, lesson #195).

use std::sync::Arc;

use elidex_ecs::{EcsDom, Entity, ImageData, MutationEvent};
use elidex_web_canvas::{Canvas2dContext, DEFAULT_HEIGHT, DEFAULT_WIDTH};

/// Marker component on a canvas entity whose [`Canvas2dContext`] has been
/// mutated since the last [`sync_dirty_canvases`] flush. Drained per frame.
pub struct CanvasDirty;

/// Resolve a canvas entity's bitmap dimensions from its `width`/`height`
/// content attributes (HTML §4.12.5), each defaulting to 300 / 150 when
/// absent or invalid. The single source of truth shared by [`ensure_context`]
/// and [`CanvasReconciler`].
#[must_use]
pub fn canvas_dimensions(dom: &EcsDom, entity: Entity) -> (u32, u32) {
    let width = parse_canvas_dim(dom.get_attribute(entity, "width").as_deref(), DEFAULT_WIDTH);
    let height = parse_canvas_dim(
        dom.get_attribute(entity, "height").as_deref(),
        DEFAULT_HEIGHT,
    );
    (width, height)
}

/// Ensure the canvas `entity` carries a [`Canvas2dContext`] component, creating
/// one at the entity's current attribute-derived dimensions if absent (the lazy
/// `getContext('2d')` allocation). No-op when one already exists. Returns `true`
/// if a context is now present; `false` only if the component insertion fails
/// (a non-live entity). Dimensions are always representable — zero / huge sizes
/// clamp to a 1×1 bitmap (see `make_context`), so they never cause `false`.
pub fn ensure_context(dom: &mut EcsDom, entity: Entity) -> bool {
    if dom.world().get::<&Canvas2dContext>(entity).is_ok() {
        return true;
    }
    let (width, height) = canvas_dimensions(dom, entity);
    dom.world_mut()
        .insert_one(entity, make_context(width, height))
        .is_ok()
}

/// Clamp a content-attribute dimension to the backing bitmap's representable
/// floor of 1: tiny-skia cannot allocate a 0-sized `Pixmap`, so a `width`/
/// `height` of 0 renders as 1×1 rather than leaving a stale bitmap (the
/// `canvas.width`/`height` IDL getters still reflect the raw attribute,
/// unclamped). True 0×0 bitmap support is deferred to
/// `#11-canvas-zero-dimension-bitmap`.
fn bitmap_dim(n: u32) -> u32 {
    n.max(1)
}

/// Build a context bitmap that is always allocatable, clamping both ends of the
/// representable range: the low end via [`bitmap_dim`] (0 → 1), and the high end
/// via a 1×1 fallback when a huge dimension makes `Pixmap` allocation fail. A
/// requested size that the backend cannot represent must still yield a *fresh*
/// bitmap (never a stale prior one), so the per-frame sync can't keep publishing
/// pixels at the old size. 1×1 always succeeds, so this is infallible.
fn make_context(width: u32, height: u32) -> Canvas2dContext {
    Canvas2dContext::new(bitmap_dim(width), bitmap_dim(height))
        .unwrap_or_else(|| Canvas2dContext::new(1, 1).expect("1×1 bitmap is always allocatable"))
}

/// Run `f` against the canvas entity's [`Canvas2dContext`] component, returning
/// its result, or `None` if the entity carries no context.
pub fn with_context<R>(
    dom: &mut EcsDom,
    entity: Entity,
    f: impl FnOnce(&mut Canvas2dContext) -> R,
) -> Option<R> {
    let mut comp = dom.world_mut().get::<&mut Canvas2dContext>(entity).ok()?;
    Some(f(&mut comp))
}

/// Mark a canvas entity dirty so the next [`sync_dirty_canvases`] re-publishes
/// its pixels into the [`ImageData`] component. No-op if already marked.
pub fn mark_dirty(dom: &mut EcsDom, entity: Entity) {
    let _ = dom.world_mut().insert_one(entity, CanvasDirty);
}

/// Flush every dirty canvas: read its [`Canvas2dContext`] pixels (straight
/// alpha) into the [`ImageData`] component (the display-list source) and clear
/// the [`CanvasDirty`] marker. Called once per frame by the shell.
pub fn sync_dirty_canvases(dom: &mut EcsDom) {
    let pending: Vec<(Entity, u32, u32, Vec<u8>)> = dom
        .world_mut()
        .query::<(Entity, &Canvas2dContext, &CanvasDirty)>()
        .iter()
        .map(|(e, ctx, _)| (e, ctx.width(), ctx.height(), ctx.to_rgba8_straight()))
        .collect();
    for (entity, width, height, pixels) in pending {
        let _ = dom.world_mut().insert_one(
            entity,
            ImageData {
                pixels: Arc::new(pixels),
                width,
                height,
            },
        );
        let _ = dom.world_mut().remove_one::<CanvasDirty>(entity);
    }
}

/// Parse a canvas `width`/`height` content attribute per the HTML "rules for
/// parsing non-negative integers" (§2.4.4.2) as applied by the reflected
/// `unsigned long`: skip leading ASCII whitespace + an optional `+`, take the
/// leading ASCII-digit run (so `"100px"` → 100, trailing garbage ignored), and
/// saturate to `u32::MAX` on overflow (clamp to the `unsigned long` range).
/// Falls back to `default` (300 / 150) only when there is no leading digit
/// (absent / non-numeric / negative value).
fn parse_canvas_dim(value: Option<&str>, default: u32) -> u32 {
    let Some(s) = value else {
        return default;
    };
    let s = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let s = s.strip_prefix('+').unwrap_or(s);
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return default;
    }
    // `digits` counts ASCII digits, each 1 byte, so the byte slice is valid.
    s[..digits].parse::<u32>().unwrap_or(u32::MAX)
}

/// [`MutationEvent`] consumer resetting a canvas bitmap when its `width`/
/// `height` content attribute changes (HTML §4.12.5 "set bitmap dimensions":
/// setting width/height — even to the same value — clears the bitmap to
/// transparent black and resets the drawing state).
///
/// Driven from the `AttributeChange` SoT rather than the IDL setter so it
/// covers `setAttribute('width', …)` and parser-baked attributes uniformly
/// (the `set_attribute` chokepoint) — the same anti-pattern the
/// `FormControlReconciler` docstring rejects. Resets only entities that already
/// carry a [`Canvas2dContext`] (no-op before the first `getContext`).
///
/// Composed as a typed field of
/// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.
pub struct CanvasReconciler;

impl CanvasReconciler {
    /// Single-method dispatch entry invoked by `ConsumerDispatcher`.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        let MutationEvent::AttributeChange { node, name, .. } = *event else {
            return;
        };
        // Exact match (not case-insensitive): every attribute-write path
        // (Element.setAttribute, the width/height IDL setters, the HTML parser)
        // lowercases HTML attribute names before storage, so the AttributeChange
        // name is always lowercase here — matching the case-sensitive read in
        // `canvas_dimensions` and the `FormControlReconciler` convention. A
        // case-insensitive check here would match an event the dimension lookup
        // then misses.
        if name != "width" && name != "height" {
            return;
        }
        // Before the first getContext there is no bitmap to reset.
        if dom.world().get::<&Canvas2dContext>(node).is_err() {
            return;
        }
        let (width, height) = canvas_dimensions(dom, node);
        // A width/height change always re-allocates a *fresh* bitmap, clamping
        // both ends of the representable range so a stale prior bitmap can never
        // survive: the low end via `bitmap_dim` (0 → 1×1), and the high end via
        // a 1×1 fallback when a huge dimension makes the re-allocation fail.
        // The reset clears to transparent black regardless, so the canvas is
        // always marked dirty for re-sync.
        if with_context(dom, node, |ctx| {
            if !ctx.reset(bitmap_dim(width), bitmap_dim(height)) {
                let _ = ctx.reset(1, 1);
            }
        })
        .is_some()
        {
            mark_dirty(dom, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn canvas(dom: &mut EcsDom, w: &str, h: &str) -> Entity {
        let e = dom.create_element("canvas", Attributes::default());
        dom.set_attribute(e, "width", w);
        dom.set_attribute(e, "height", h);
        e
    }

    #[test]
    fn parse_dim_valid_invalid_negative() {
        assert_eq!(parse_canvas_dim(Some("640"), DEFAULT_WIDTH), 640);
        assert_eq!(parse_canvas_dim(Some(" 480 "), DEFAULT_HEIGHT), 480);
        assert_eq!(parse_canvas_dim(Some("abc"), DEFAULT_WIDTH), DEFAULT_WIDTH);
        assert_eq!(parse_canvas_dim(Some("-5"), DEFAULT_HEIGHT), DEFAULT_HEIGHT);
        assert_eq!(parse_canvas_dim(None, DEFAULT_WIDTH), DEFAULT_WIDTH);
    }

    #[test]
    fn parse_dim_leading_digit_run_and_overflow() {
        // HTML "rules for parsing non-negative integers": leading digit run,
        // trailing garbage ignored.
        assert_eq!(parse_canvas_dim(Some("100px"), DEFAULT_WIDTH), 100);
        assert_eq!(parse_canvas_dim(Some("12.9"), DEFAULT_WIDTH), 12);
        assert_eq!(parse_canvas_dim(Some("+50"), DEFAULT_WIDTH), 50);
        // Overflow saturates to the unsigned-long max, not the default.
        assert_eq!(
            parse_canvas_dim(Some("99999999999"), DEFAULT_WIDTH),
            u32::MAX
        );
        // Leading non-digit (after optional sign) → no digits → default.
        assert_eq!(
            parse_canvas_dim(Some("px100"), DEFAULT_WIDTH),
            DEFAULT_WIDTH
        );
    }

    #[test]
    fn dimensions_default_when_attrs_absent() {
        let mut dom = EcsDom::new();
        let e = dom.create_element("canvas", Attributes::default());
        assert_eq!(canvas_dimensions(&dom, e), (DEFAULT_WIDTH, DEFAULT_HEIGHT));
    }

    #[test]
    fn ensure_context_is_idempotent() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "10", "10");
        assert!(ensure_context(&mut dom, e));
        // Mutate so a second ensure that re-created would lose the pixels.
        with_context(&mut dom, e, |ctx| {
            ctx.set_fill_style("red");
            ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
        });
        // Second ensure is a no-op (keeps the existing, drawn-on context).
        assert!(ensure_context(&mut dom, e));
        let any_red = with_context(&mut dom, e, |ctx| ctx.pixels().iter().any(|&b| b != 0));
        assert_eq!(any_red, Some(true));
    }

    #[test]
    fn sync_writes_image_data_and_clears_marker() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "4", "3");
        ensure_context(&mut dom, e);
        with_context(&mut dom, e, |ctx| {
            ctx.set_fill_style("rgb(0, 0, 255)");
            ctx.fill_rect(0.0, 0.0, 4.0, 3.0);
        });
        mark_dirty(&mut dom, e);
        sync_dirty_canvases(&mut dom);

        let img = dom.world().get::<&ImageData>(e).expect("ImageData written");
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 3);
        assert_eq!(img.pixels.len(), 4 * 3 * 4);
        // First pixel is opaque blue (straight alpha).
        assert_eq!(&img.pixels[0..4], &[0, 0, 255, 255]);
        drop(img);
        // Marker cleared — a second sync without a re-mark is a no-op.
        assert!(dom.world().get::<&CanvasDirty>(e).is_err());
    }

    #[test]
    fn sync_skips_undrawn_canvases() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "4", "4");
        ensure_context(&mut dom, e);
        // Not marked dirty → no ImageData produced.
        sync_dirty_canvases(&mut dom);
        assert!(dom.world().get::<&ImageData>(e).is_err());
    }

    #[test]
    fn reconciler_resets_bitmap_on_dimension_change() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "10", "10");
        ensure_context(&mut dom, e);
        with_context(&mut dom, e, |ctx| {
            ctx.set_fill_style("red");
            ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
        });
        // Resize via the width attribute, then reconcile.
        dom.set_attribute(e, "width", "20");
        let mut rec = CanvasReconciler;
        rec.handle(
            &MutationEvent::AttributeChange {
                node: e,
                name: "width",
                old_value: Some("10"),
                new_value: Some("20"),
            },
            &mut dom,
        );
        let (w, h, cleared) = with_context(&mut dom, e, |ctx| {
            (
                ctx.width(),
                ctx.height(),
                ctx.pixels().iter().all(|&b| b == 0),
            )
        })
        .unwrap();
        assert_eq!((w, h), (20, 10));
        assert!(cleared, "bitmap reset to transparent black");
        assert!(
            dom.world().get::<&CanvasDirty>(e).is_ok(),
            "reset marks the canvas dirty for re-sync"
        );
    }

    #[test]
    fn reconciler_noop_before_get_context() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "10", "10");
        // No ensure_context yet → no component → reconciler is a no-op.
        let mut rec = CanvasReconciler;
        rec.handle(
            &MutationEvent::AttributeChange {
                node: e,
                name: "width",
                old_value: None,
                new_value: Some("20"),
            },
            &mut dom,
        );
        assert!(dom.world().get::<&Canvas2dContext>(e).is_err());
        assert!(dom.world().get::<&CanvasDirty>(e).is_err());
    }

    #[test]
    fn ensure_context_clamps_zero_dims_to_one() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "0", "0");
        // tiny-skia can't allocate 0×0; clamp to the 1×1 floor (still succeeds).
        assert!(ensure_context(&mut dom, e));
        assert_eq!(
            with_context(&mut dom, e, |ctx| (ctx.width(), ctx.height())),
            Some((1, 1))
        );
    }

    #[test]
    fn reconciler_huge_dim_reallocates_to_one_not_stale() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "4", "4");
        ensure_context(&mut dom, e);
        // Resize width to a value tiny-skia cannot allocate (> i32::MAX): the
        // bitmap must fall back to 1×1, NOT keep the stale 4×4 that the per-frame
        // sync would otherwise publish.
        dom.set_attribute(e, "width", "4294967295");
        let mut rec = CanvasReconciler;
        rec.handle(
            &MutationEvent::AttributeChange {
                node: e,
                name: "width",
                old_value: Some("4"),
                new_value: Some("4294967295"),
            },
            &mut dom,
        );
        assert_eq!(
            with_context(&mut dom, e, |ctx| (ctx.width(), ctx.height())),
            Some((1, 1)),
            "unrepresentable-huge dims fall back to 1×1, not the stale 4×4"
        );
        assert!(dom.world().get::<&CanvasDirty>(e).is_ok());
    }

    #[test]
    fn ensure_context_huge_dims_fall_back_to_one() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "4294967295", "4294967295");
        // Unrepresentable-huge attribute dims still yield a (1×1) context.
        assert!(ensure_context(&mut dom, e));
        assert_eq!(
            with_context(&mut dom, e, |ctx| (ctx.width(), ctx.height())),
            Some((1, 1))
        );
    }

    #[test]
    fn reconciler_zero_dim_reallocates_not_stale() {
        let mut dom = EcsDom::new();
        let e = canvas(&mut dom, "4", "4");
        ensure_context(&mut dom, e);
        // Resize width to 0: the bitmap must re-allocate to the 1×4 floor, NOT
        // keep the stale 4×4 (which the sync would otherwise publish).
        dom.set_attribute(e, "width", "0");
        let mut rec = CanvasReconciler;
        rec.handle(
            &MutationEvent::AttributeChange {
                node: e,
                name: "width",
                old_value: Some("4"),
                new_value: Some("0"),
            },
            &mut dom,
        );
        assert_eq!(
            with_context(&mut dom, e, |ctx| (ctx.width(), ctx.height())),
            Some((1, 4))
        );
        assert!(dom.world().get::<&CanvasDirty>(e).is_ok());
    }
}
