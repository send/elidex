//! Per-`OffscreenCanvas`-entity 2D context plumbing (WHATWG HTML §4.12.5.1.7
//! "The OffscreenCanvas interface", main-thread side).
//!
//! Sibling of [`crate::component`] which holds the `<canvas>` Element side.
//! The two share the [`Canvas2dContext`] raster backend verbatim — only the
//! owner entity differs (`NodeKind::Element` for `<canvas>`,
//! [`NodeKind::OffscreenCanvas`] for an `OffscreenCanvas`) and the dimension
//! source (`width`/`height` attributes on `<canvas>` vs the
//! [`OffscreenCanvasDims`] component for `OffscreenCanvas`, since `OC` has no
//! attribute backing).
//!
//! All per-OC state is `Send + Sync` per-entity and not a per-VM identity
//! handle, so per the side-store audit it lives as **ECS components on the
//! OC entity** — never in `HostData::offscreen_canvas_registry` /
//! `HashMap<ObjectId, _>` side-tables (the boa-port D-21 R3 CRIT trap).
//!
//! ## Mutex invariant (Canvas2dContext / PlaceholderCanvas)
//!
//! A `<canvas>` Element entity carries `Canvas2dContext` XOR
//! [`PlaceholderCanvas`] — never both. Spec (HTML §4.12.5 step 1 of the
//! transferControlToOffscreen algorithm): "If this element's context mode is
//! not 'none', throw an InvalidStateError." Enforced **by construction at the
//! write-side gates**:
//! - [`transfer_canvas_to_offscreen`] refuses if `Canvas2dContext` present
//!   (Err::AlreadyHasContext) and refuses double-transfer
//!   (Err::AlreadyPlaceholder).
//! - The `<canvas>.getContext` host guard (in the binding crate) reads
//!   [`is_placeholder`] first and throws InvalidStateError if true.
//!
//! No read-side `debug_assert!` is used — both components can legitimately
//! be absent on a freshly-spawned OffscreenCanvas (pre-
//! [`ensure_offscreen_context`]), so a XOR assertion would falsely fire.

use elidex_ecs::{EcsDom, Entity, NodeKind};
use elidex_web_canvas::Canvas2dContext;

use crate::component::{make_context, reset_canvas_bitmap};

/// Dimensions component for an [`NodeKind::OffscreenCanvas`] entity (WHATWG
/// HTML §4.12.5.1.7 IDL `width` / `height`). `Send + Sync` per-entity → ECS
/// component (audit rule); the IDL setter is the only mutation path, since
/// OC has no attribute backing.
#[derive(Clone, Copy, Debug)]
pub struct OffscreenCanvasDims {
    pub width: u32,
    pub height: u32,
}

/// Marker component on a `<canvas>` HTMLCanvasElement entity post-
/// `transferControlToOffscreen` (HTML §4.12.5 "placeholder canvas element"
/// definition). Stores the OffscreenCanvas entity it was transferred to (for
/// future `commit`-from-worker reflection — v1 reads only the presence). The
/// `<canvas>.getContext` guard throws `InvalidStateError` when this is
/// present; `transferControlToOffscreen` refuses double-transfer.
#[derive(Clone, Copy, Debug)]
pub struct PlaceholderCanvas {
    pub transferred_to: Entity,
}

/// Why a `transferControlToOffscreen` call refused (WHATWG HTML §4.12.5
/// transfer algorithm step 1: "context mode must be `none`"). Both variants
/// map to the spec `InvalidStateError` DOMException at the host marshalling
/// layer; the distinction is observability-only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaceholderError {
    /// The `<canvas>` already has a 2D context — `getContext('2d')` was
    /// called before `transferControlToOffscreen` (spec step 1 violation).
    AlreadyHasContext,
    /// The `<canvas>` was already transferred — `transferControlToOffscreen`
    /// called twice on the same element (spec step 1 violation, second-call
    /// branch).
    AlreadyPlaceholder,
}

/// Read an OffscreenCanvas entity's IDL dimensions (defaults to 1×1 when the
/// component is missing — a freshly-spawned OC always has the component, so
/// this fallback is defensive for the never-spawned-via-helper code path).
#[must_use]
pub fn offscreen_canvas_dimensions(dom: &EcsDom, entity: Entity) -> (u32, u32) {
    dom.world()
        .get::<&OffscreenCanvasDims>(entity)
        .map_or((1, 1), |d| (d.width, d.height))
}

/// Spawn a new [`NodeKind::OffscreenCanvas`] entity with
/// [`OffscreenCanvasDims`] populated atomically (WHATWG HTML §4.12.5.1.7 ctor
/// plus §4.12.5 `transferControlToOffscreen` algorithm). The single entity-
/// creation path for OffscreenCanvas — enforces the dim write-path by
/// construction so callers cannot forget. The `Canvas2dContext` is NOT
/// inserted here (lazy via [`ensure_offscreen_context`] on first
/// `getContext('2d')`) — matching the `<canvas>` lazy-context pattern in
/// `crate::component::ensure_context`.
pub fn spawn_offscreen_canvas_entity(dom: &mut EcsDom, width: u32, height: u32) -> Entity {
    dom.world_mut().spawn((
        NodeKind::OffscreenCanvas,
        OffscreenCanvasDims { width, height },
    ))
}

/// Ensure an OffscreenCanvas entity carries a [`Canvas2dContext`] component,
/// lazy-allocating from [`OffscreenCanvasDims`] if absent. Parallels
/// [`crate::component::ensure_context`] — separate function (not a NodeKind
/// dispatch inside `ensure_context`) for call-site clarity, since the call
/// site already knows whether it's `<canvas>` or OC.
///
/// Returns `true` if a context is now present; `false` only on a non-live
/// entity (component insertion failure). Dimensions clamp to 1×1 if
/// unrepresentable, mirroring `<canvas>` semantics.
pub fn ensure_offscreen_context(dom: &mut EcsDom, entity: Entity) -> bool {
    if dom.world().get::<&Canvas2dContext>(entity).is_ok() {
        return true;
    }
    let (width, height) = offscreen_canvas_dimensions(dom, entity);
    let ctx = make_context(width, height);
    dom.world_mut().insert_one(entity, ctx).is_ok()
}

/// IDL setter `offscreenCanvas.width = N` (WHATWG HTML §4.12.5.1.7). Writes
/// the new dim to [`OffscreenCanvasDims`] then routes through the shared
/// bitmap-reset chokepoint [`reset_canvas_bitmap`] so the reset semantics
/// match `<canvas>` exactly (one-issue-one-way). The reset is a no-op when
/// no `Canvas2dContext` is present yet (pre-`ensure_offscreen_context`); the
/// dim write still lands so a later `getContext('2d')` allocates the bitmap
/// at the new size.
pub fn set_offscreen_canvas_width(dom: &mut EcsDom, entity: Entity, width: u32) {
    let height = {
        let Ok(mut dims) = dom.world_mut().get::<&mut OffscreenCanvasDims>(entity) else {
            return;
        };
        dims.width = width;
        dims.height
    };
    let _ = reset_canvas_bitmap(dom, entity, width, height);
}

/// IDL setter `offscreenCanvas.height = N` (WHATWG HTML §4.12.5.1.7).
/// Sibling of [`set_offscreen_canvas_width`]; same shape.
pub fn set_offscreen_canvas_height(dom: &mut EcsDom, entity: Entity, height: u32) {
    let width = {
        let Ok(mut dims) = dom.world_mut().get::<&mut OffscreenCanvasDims>(entity) else {
            return;
        };
        dims.height = height;
        dims.width
    };
    let _ = reset_canvas_bitmap(dom, entity, width, height);
}

/// Read predicate: does `entity` carry [`PlaceholderCanvas`]? (HTML §4.12.5
/// "placeholder canvas element" definition read side.) Used by the
/// `<canvas>.getContext` host guard to throw `InvalidStateError`.
#[must_use]
pub fn is_placeholder(dom: &EcsDom, entity: Entity) -> bool {
    dom.world().get::<&PlaceholderCanvas>(entity).is_ok()
}

/// Atomic write-side gate for `HTMLCanvasElement.transferControlToOffscreen()`
/// (WHATWG HTML §4.12.5 transferControlToOffscreen algorithm steps 1-7).
///
/// Combines validation + OC spawn + placeholder write into a single helper
/// so the caller cannot leave partial state on `Err` — if validation fails,
/// no OC entity is spawned; if validation passes, all three steps land
/// together. This is the structural Canvas2dContext-vs-PlaceholderCanvas
/// mutex enforcement point.
///
/// Operation order:
/// 1. Refuse if `canvas_entity` already has [`Canvas2dContext`] →
///    `Err(AlreadyHasContext)` (spec step 1: "context mode must be `none`").
/// 2. Refuse if `canvas_entity` already has [`PlaceholderCanvas`] →
///    `Err(AlreadyPlaceholder)` (double-transfer guard).
/// 3. Read `<canvas>` attribute-derived dims via the existing
///    [`crate::component::canvas_dimensions`] helper.
/// 4. Spawn the OffscreenCanvas entity via [`spawn_offscreen_canvas_entity`].
/// 5. Write [`PlaceholderCanvas { transferred_to: oc_entity }`] on the
///    `<canvas>` entity.
///
/// Returns `Ok(oc_entity)`. The host marshals `Err` to `InvalidStateError`
/// DOMException.
pub fn transfer_canvas_to_offscreen(
    dom: &mut EcsDom,
    canvas_entity: Entity,
) -> Result<Entity, PlaceholderError> {
    if dom.world().get::<&Canvas2dContext>(canvas_entity).is_ok() {
        return Err(PlaceholderError::AlreadyHasContext);
    }
    if dom.world().get::<&PlaceholderCanvas>(canvas_entity).is_ok() {
        return Err(PlaceholderError::AlreadyPlaceholder);
    }
    let (width, height) = crate::component::canvas_dimensions(dom, canvas_entity);
    let oc_entity = spawn_offscreen_canvas_entity(dom, width, height);
    // Insert is infallible here — canvas_entity is live (we just queried it
    // above), and PlaceholderCanvas is a fresh-typed component on it.
    let _ = dom.world_mut().insert_one(
        canvas_entity,
        PlaceholderCanvas {
            transferred_to: oc_entity,
        },
    );
    Ok(oc_entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::with_context;
    use elidex_ecs::Attributes;

    fn fresh_canvas(dom: &mut EcsDom, w: &str, h: &str) -> Entity {
        let e = dom.create_element("canvas", Attributes::default());
        dom.set_attribute(e, "width", w);
        dom.set_attribute(e, "height", h);
        e
    }

    #[test]
    fn spawn_populates_dims_and_node_kind() {
        let mut dom = EcsDom::new();
        let e = spawn_offscreen_canvas_entity(&mut dom, 200, 100);
        let kind = *dom.world().get::<&NodeKind>(e).expect("NodeKind component");
        assert_eq!(kind, NodeKind::OffscreenCanvas);
        assert_eq!(offscreen_canvas_dimensions(&dom, e), (200, 100));
    }

    #[test]
    fn ensure_offscreen_context_is_idempotent_and_uses_dims() {
        let mut dom = EcsDom::new();
        let e = spawn_offscreen_canvas_entity(&mut dom, 4, 5);
        assert!(ensure_offscreen_context(&mut dom, e));
        assert_eq!(
            with_context(&mut dom, e, |c| (c.width(), c.height())),
            Some((4, 5))
        );
        // Second call is a no-op (does NOT re-allocate).
        with_context(&mut dom, e, |c| c.fill_rect(0.0, 0.0, 4.0, 5.0));
        assert!(ensure_offscreen_context(&mut dom, e));
        // Pixels preserved (no re-alloc) — proves idempotency.
        let any_nonzero =
            with_context(&mut dom, e, |c| c.pixels().iter().any(|&b| b != 0)).unwrap();
        assert!(any_nonzero);
    }

    #[test]
    fn set_width_resets_bitmap_after_ensure() {
        let mut dom = EcsDom::new();
        let e = spawn_offscreen_canvas_entity(&mut dom, 4, 4);
        ensure_offscreen_context(&mut dom, e);
        with_context(&mut dom, e, |c| {
            c.set_fill_style("red");
            c.fill_rect(0.0, 0.0, 4.0, 4.0);
        });
        set_offscreen_canvas_width(&mut dom, e, 10);
        let (w, h, cleared) = with_context(&mut dom, e, |c| {
            (c.width(), c.height(), c.pixels().iter().all(|&b| b == 0))
        })
        .unwrap();
        assert_eq!((w, h), (10, 4));
        assert!(cleared, "bitmap reset to transparent black after width set");
        assert_eq!(offscreen_canvas_dimensions(&dom, e), (10, 4));
    }

    #[test]
    fn set_width_before_ensure_writes_dim_but_does_not_create_context() {
        let mut dom = EcsDom::new();
        let e = spawn_offscreen_canvas_entity(&mut dom, 4, 4);
        // No ensure_offscreen_context yet — set still writes the dim.
        set_offscreen_canvas_width(&mut dom, e, 20);
        assert_eq!(offscreen_canvas_dimensions(&dom, e), (20, 4));
        // No Canvas2dContext was created by the setter (lazy via getContext).
        assert!(dom.world().get::<&Canvas2dContext>(e).is_err());
        // A subsequent ensure picks up the new dim.
        ensure_offscreen_context(&mut dom, e);
        assert_eq!(
            with_context(&mut dom, e, |c| (c.width(), c.height())),
            Some((20, 4))
        );
    }

    #[test]
    fn transfer_happy_path_creates_oc_and_marks_placeholder() {
        let mut dom = EcsDom::new();
        let canvas = fresh_canvas(&mut dom, "320", "200");
        let oc = transfer_canvas_to_offscreen(&mut dom, canvas).expect("transfer succeeds");
        // OC entity has the right NodeKind + dims copied from canvas attrs.
        let kind = *dom.world().get::<&NodeKind>(oc).unwrap();
        assert_eq!(kind, NodeKind::OffscreenCanvas);
        assert_eq!(offscreen_canvas_dimensions(&dom, oc), (320, 200));
        // <canvas> is now a placeholder pointing at oc.
        assert!(is_placeholder(&dom, canvas));
        let ph = dom.world().get::<&PlaceholderCanvas>(canvas).unwrap();
        assert_eq!(ph.transferred_to, oc);
    }

    #[test]
    fn transfer_refuses_when_canvas_has_context() {
        let mut dom = EcsDom::new();
        let canvas = fresh_canvas(&mut dom, "10", "10");
        // Simulate getContext('2d') being called first.
        crate::component::ensure_context(&mut dom, canvas);
        let err = transfer_canvas_to_offscreen(&mut dom, canvas)
            .expect_err("must refuse after getContext");
        assert_eq!(err, PlaceholderError::AlreadyHasContext);
        // No PlaceholderCanvas was written; no OC entity was leaked into the
        // world (the helper bails BEFORE spawn — verified by the structural
        // ordering, not asserted here since searching the world for the
        // "absent" entity is awkward; the test in `transfer_double_refuses`
        // covers the other Err branch).
        assert!(!is_placeholder(&dom, canvas));
    }

    #[test]
    fn transfer_double_refuses() {
        let mut dom = EcsDom::new();
        let canvas = fresh_canvas(&mut dom, "10", "10");
        let _ = transfer_canvas_to_offscreen(&mut dom, canvas).expect("first transfer succeeds");
        let err = transfer_canvas_to_offscreen(&mut dom, canvas)
            .expect_err("second transfer must refuse");
        assert_eq!(err, PlaceholderError::AlreadyPlaceholder);
    }

    #[test]
    fn is_placeholder_false_for_fresh_canvas() {
        let mut dom = EcsDom::new();
        let canvas = fresh_canvas(&mut dom, "10", "10");
        assert!(!is_placeholder(&dom, canvas));
    }
}
