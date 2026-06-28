//! `VisualViewport` interface + `window.visualViewport` (CSSOM-View §12.1
//! *The VisualViewport Interface* / §4 *Extensions to the Window Interface*).
//!
//! `VisualViewport` is an `EventTarget` that is *not* a `Node`, so — like
//! `MediaQueryList` / `Window` / `AbortSignal` — its prototype chain is:
//!
//! ```text
//! VisualViewport instance (ObjectKind::VisualViewport, payload-free)
//!   → VisualViewport.prototype  (this module)
//!     → EventTarget.prototype   (no Node members)
//!       → Object.prototype
//! ```
//!
//! ## Real EventTarget + live event producer (S5-2)
//!
//! boa exposed `visualViewport` as a plain object with **stub** (no-op)
//! `addEventListener` / `removeEventListener`. The VM makes it a genuine
//! `EventTarget` singleton: `addEventListener('resize'|'scroll'|'scrollend',
//! …)` / `removeEventListener` / `dispatchEvent` are **inherited** from
//! `EventTarget.prototype` (routed to the object's `vm_event_listeners` home via
//! [`DispatchTarget::VmObject`](super::dispatch_target::DispatchTarget), gated by
//! [`ObjectKind::is_non_node_event_target`](super::super::value::ObjectKind::is_non_node_event_target)),
//! and `onresize` / `onscroll` / `onscrollend` are event-handler IDL attributes.
//!
//! The `resize` / `scroll` / `scrollend` events fire through a **real VM
//! producer** ([`VmInner::deliver_visual_viewport_events`], modeled on
//! [`VmInner::deliver_media_query_changes`]): it diffs the current
//! `ViewportState` geometry against the stored
//! [`visual_viewport_delivered`](super::super::VmInner::visual_viewport_delivered)
//! prior and fires each changed event at the singleton (per-axis: `resize` on a
//! size change, `scroll` on a scroll-offset change, `scrollend` only when
//! `scroll` fired). The live shell call-site rides the S5-6 flip; until then the
//! producer is exercised by VM tests.
//!
//! ## Singleton + identity + GC
//!
//! `window.visualViewport` is a `[SameObject, Replaceable]` readonly attribute
//! of type `VisualViewport?` (CSSOM-View §4 — nullable). It is installed as a
//! **no-setter RO accessor on `Window.prototype`** whose getter returns the
//! cached singleton `ObjectId` (the `localStorage` / `[SameObject]` form),
//! normalizing it onto the same treatment its sibling `[Replaceable]` Window
//! attrs (`innerWidth` / `scrollX` / `devicePixelRatio`) already use — replacing
//! the anomalous writable `globals.insert`. (Proper `[Replaceable]`
//! value-shadowing is implemented for NONE of the family → deferred engine-wide,
//! `#11-window-platform-object-rigor-engine-wide`.) The singleton is cached in
//! `VmInner::visual_viewport_instance` (rooted via the GC proto-roots, SameObject
//! for free) and **cleared on `Vm::unbind`** alongside the `vv_delivered` prior
//! (the `localStorage` cross-DOM precedent) so the producer never fires at a
//! stale `ObjectId` from a prior `EcsDom`. The brand is payload-free; GC has
//! nothing to trace or prune.

#![cfg(feature = "engine")]

use super::super::shape::PropertyAttrs;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::event_target_dispatch_vm::fire_vm_event;
use super::events::EventInit;

impl VmInner {
    /// Allocate `VisualViewport.prototype` + the illegal-constructor interface
    /// object, exposing the `VisualViewport` global. The per-window singleton
    /// instance is allocated lazily by the `window.visualViewport` RO-accessor
    /// getter ([`Self::alloc_or_cached_visual_viewport`]).
    ///
    /// Called from `register_globals()` **after**
    /// [`Self::register_event_target_prototype`] (the prototype chains to
    /// `event_target_prototype`). Window realm only (`[Exposed=Window]`).
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` is `None` — would mean
    /// `register_event_target_prototype` was skipped or run out of order.
    pub(in crate::vm) fn register_visual_viewport_global(&mut self) {
        let event_target_proto = self.event_target_prototype.expect(
            "register_visual_viewport_global called before register_event_target_prototype",
        );

        // ---- VisualViewport.prototype ----
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.install_ro_accessors(proto_id, VISUAL_VIEWPORT_RO_ACCESSORS);
        // `onresize` / `onscroll` / `onscrollend` event-handler IDL attributes
        // (CSSOM-View §12.1), each bound to its event-type SID over the shared
        // VmObject event-handler backend (the `MediaQueryList::onchange`
        // precedent). `addEventListener` / `removeEventListener` /
        // `dispatchEvent` are INHERITED from `EventTarget.prototype`.
        for (handler, event_type) in [
            ("onresize", "resize"),
            ("onscroll", "scroll"),
            ("onscrollend", "scrollend"),
        ] {
            let handler_sid = self.strings.intern(handler);
            let event_sid = self.strings.intern(event_type);
            self.install_bound_accessor_pair(
                proto_id,
                handler_sid,
                super::event_handler_attrs::native_vm_event_handler_get as NativeFn,
                Some(super::event_handler_attrs::native_vm_event_handler_set as NativeFn),
                event_sid,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        self.visual_viewport_prototype = Some(proto_id);

        // ---- VisualViewport interface object ----
        // WebIDL: `VisualViewport` declares NO constructor — `new
        // VisualViewport()` / `VisualViewport()` throw a TypeError. Registered
        // as an illegal-constructor so `vv instanceof VisualViewport` and
        // `VisualViewport.prototype` parity still work (the `MediaQueryList`
        // precedent).
        let ctor = self.create_illegal_constructor_function(
            "VisualViewport",
            super::super::value::native_illegal_constructor_unreachable,
        );
        self.wire_interface_ctor_prototype(ctor, proto_id);
        let ctor_name = self.strings.intern("VisualViewport");
        self.globals.insert(ctor_name, JsValue::Object(ctor));
    }

    /// Return the cached `VisualViewport` singleton, allocating it on the first
    /// `window.visualViewport` read. `[SameObject]`: the same `ObjectId` across
    /// reads for one bind cycle (cleared on `Vm::unbind`).
    ///
    /// **Seeds [`Self::visual_viewport_delivered`]** (the producer's diff prior)
    /// to the current `ViewportState` geometry **at allocation** — the exact
    /// [`Self::create_media_query_list`] `last_matches` seed parallel. Because
    /// the producer resolves the singleton through THIS same getter, the seed
    /// happens-before the producer's first diff-read by construction, so the
    /// first `deliver_visual_viewport_events` after creation (or after an unbind
    /// that reset the prior to `None`) fires NOTHING spuriously.
    pub(in crate::vm) fn alloc_or_cached_visual_viewport(&mut self) -> ObjectId {
        if let Some(id) = self.visual_viewport_instance {
            return id;
        }
        let proto = self
            .visual_viewport_prototype
            .expect("alloc_or_cached_visual_viewport before register_visual_viewport_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::VisualViewport,
            storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        });
        self.visual_viewport_instance = Some(id);
        // Seed the diff prior at construction (the `last_matches` parallel) so
        // the first deliver diffs against the real starting geometry.
        if self.visual_viewport_delivered.is_none() {
            self.visual_viewport_delivered = Some(current_vv_geometry(self));
        }
        id
    }

    /// CSSOM-View §12.1 producer — the per-turn report-changes pass for
    /// `VisualViewport`. Diffs the current `ViewportState` geometry against the
    /// [`Self::visual_viewport_delivered`] prior and fires, **per axis**, a
    /// trusted plain `Event` at the singleton:
    ///
    /// - `resize` ⇐ the `(width, height)` pair changed vs the prior;
    /// - `scroll` ⇐ the `(scroll_x, scroll_y)` pair changed vs the prior;
    /// - `scrollend` ⇐ **only when `scroll` fired** (a resize-only deliver must
    ///   NOT fire `scroll`/`scrollend`; the shell echoes discrete settled
    ///   offsets, so each settled scroll fires `scroll`+`scrollend` — momentum /
    ///   gesture-end debounce timing is `#11-screen-available-area-workarea-source`'s
    ///   shell-input-fidelity tail).
    ///
    /// Mirrors [`Self::deliver_media_query_changes`]: a no-op while unbound (no
    /// JS context to fire into), resolves the singleton through
    /// [`Self::alloc_or_cached_visual_viewport`] (which seeds the prior on first
    /// alloc, so a first deliver fires nothing), advances the prior after each
    /// deliver, and ends on a microtask checkpoint. The shell drives this from
    /// its update-the-rendering step after a resize / scroll-echo (the call-site
    /// rides the S5-6 flip); VM tests drive it directly.
    pub(in crate::vm) fn deliver_visual_viewport_events(&mut self) {
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }

        // Resolve (and lazily seed) the singleton through the shared getter — the
        // same resolution path the RO accessor uses, so the diff prior is the one
        // seeded at allocation. After this call `visual_viewport_delivered` is
        // `Some` (seeded by the getter on first alloc, else carried forward).
        let target = self.alloc_or_cached_visual_viewport();
        let (now_width, now_height, now_scroll_x, now_scroll_y) = current_vv_geometry(self);
        let (prev_width, prev_height, prev_scroll_x, prev_scroll_y) = self
            .visual_viewport_delivered
            .expect("alloc_or_cached_visual_viewport seeds visual_viewport_delivered");

        let resized = now_width != prev_width || now_height != prev_height;
        let scrolled = now_scroll_x != prev_scroll_x || now_scroll_y != prev_scroll_y;

        // Advance the prior BEFORE firing (the `last_matches` discipline) so a
        // listener that re-reads geometry or triggers a re-entrant deliver sees
        // the settled state and the re-entrant deliver is a no-op for this axis.
        self.visual_viewport_delivered = Some((now_width, now_height, now_scroll_x, now_scroll_y));

        if resized || scrolled {
            let shape = self
                .precomputed_event_shapes
                .as_ref()
                .expect("precomputed_event_shapes built during VM init")
                .core;
            // Plain (non-bubbling, non-cancelable) `Event`s — VisualViewport's
            // resize/scroll/scrollend carry no extra IDL attributes.
            let init = EventInit {
                bubbles: false,
                cancelable: false,
                composed: false,
            };
            // Root the singleton across the fires (the MQL `push_temp_root`
            // discipline): `fire_vm_event` allocates the event object and may
            // trigger a GC before dispatch. The singleton is already a permanent
            // GC root (`visual_viewport_instance`), so this is belt-and-suspenders
            // symmetry with the MQL producer.
            let mut guard = self.push_temp_root(JsValue::Object(target));
            let mut ctx = NativeContext::new_call(&mut guard);
            // `resize` first, then `scroll`, then `scrollend` — the natural
            // observation order (a settled scroll-with-resize sees the new size
            // before the new offset). `scrollend` fires only when `scroll` did.
            if resized {
                fire_vv_event(&mut ctx, target, "resize", init, shape);
            }
            if scrolled {
                fire_vv_event(&mut ctx, target, "scroll", init, shape);
                fire_vv_event(&mut ctx, target, "scrollend", init, shape);
            }
        }

        // Each pass is its own microtask checkpoint (the `deliver_*` parity),
        // even when nothing changed.
        self.drain_microtasks();
    }
}

/// Read the current `VisualViewport` geometry tuple `(width, height, scroll_x,
/// scroll_y)` from the VM-global `ViewportState`. The producer's diff axes:
/// `width`/`height` back `resize`, `scroll_x`/`scroll_y` back `scroll`.
fn current_vv_geometry(vm: &VmInner) -> (f64, f64, f64, f64) {
    (
        vm.viewport.inner_width,
        vm.viewport.inner_height,
        vm.viewport.scroll_x,
        vm.viewport.scroll_y,
    )
}

/// Fire one trusted plain `Event` (`resize`/`scroll`/`scrollend`) at the
/// `VisualViewport` singleton through the unified EventTarget dispatch core.
/// `fire_vm_event` gates on a listener internally, so an unobserved event
/// allocates nothing.
fn fire_vv_event(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: &str,
    init: EventInit,
    shape: super::super::shape::ShapeId,
) {
    let type_sid = ctx.vm.strings.intern(event_type);
    let payload: Vec<PropertyValue> = Vec::new();
    let _ = fire_vm_event(ctx, target, type_sid, init, shape, None, payload);
}

impl VmInner {
    /// Clear the per-VM window-parity singleton caches (`screen` /
    /// `visualViewport`) and reset the `VisualViewport` event producer's diff
    /// prior. Called from `Vm::unbind`: the singletons are treated as **per-bind**
    /// cached singletons (the `localStorage` precedent), not realm-structural
    /// survivors, so a rebind cannot fire `fire_vm_event` at a stale `ObjectId`
    /// from a prior `EcsDom` and re-seeds the prior against the new document's
    /// starting geometry.
    pub(in crate::vm) fn clear_window_parity_instance_cache(&mut self) {
        self.screen_instance = None;
        self.visual_viewport_instance = None;
        self.visual_viewport_delivered = None;
    }
}

/// `VisualViewport`'s value-derived RO accessors (CSSOM-View §12.1). Each reads
/// the VM-global [`ViewportState`](super::window::ViewportState) after the
/// WebIDL branded-receiver gate.
const VISUAL_VIEWPORT_RO_ACCESSORS: &[(&str, NativeFn)] = &[
    ("offsetLeft", native_vv_get_offset_left),
    ("offsetTop", native_vv_get_offset_top),
    ("pageLeft", native_vv_get_page_left),
    ("pageTop", native_vv_get_page_top),
    ("width", native_vv_get_width),
    ("height", native_vv_get_height),
    ("scale", native_vv_get_scale),
];

/// WebIDL branded-receiver gate for `VisualViewport.prototype.*` attribute
/// getters. Throws a TypeError ("illegal invocation") on a non-branded receiver
/// (boa skipped this; the VM enforces it for spec fidelity — the
/// `MediaQueryList` / S5-1 stance).
fn require_visual_viewport_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    attr: &str,
) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::VisualViewport) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "Failed to read the '{attr}' property from 'VisualViewport': illegal invocation"
    )))
}

/// Shared read path for the 7 VisualViewport geometry getters (CSSOM-View §12.1).
/// Brand-checks `this`, then applies the §12.1 step-1 "not fully active → 0" guard
/// **once** before returning `read(viewport)` for the fully-active value. The
/// not-fully-active branch is the single seam a future multi-document model wires:
/// [`window_has_fully_active_document`](super::window::window_has_fully_active_document)
/// is unconditionally `true` today (the `html_dialog_proto.rs` precedent, folded
/// into `#11-browsing-context-state-ecs-components`), so folding the guard here
/// keeps the spec step in one place instead of inlining it in all 7 getters.
fn vv_geometry_read(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    attr: &str,
    read: impl FnOnce(&super::window::ViewportState) -> f64,
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, attr)?;
    if !super::window::window_has_fully_active_document(ctx) {
        return Ok(JsValue::Number(0.0));
    }
    Ok(JsValue::Number(read(&ctx.vm.viewport)))
}

/// `visualViewport.offsetLeft` / `.offsetTop` (CSSOM-View §12.1) — the offset of
/// the visual viewport from the layout viewport: `0` with no pinch-zoom (the
/// engine models none), the exact value rather than a placeholder.
fn native_vv_get_offset_left(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "offsetLeft", |_vp| 0.0)
}

fn native_vv_get_offset_top(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "offsetTop", |_vp| 0.0)
}

/// `visualViewport.pageLeft` / `.pageTop` (CSSOM-View §12.1) — the page-relative
/// offset = layout-viewport scroll (`scroll_x`/`scroll_y`) + the visual
/// `offsetLeft`/`offsetTop` (which are 0).
fn native_vv_get_page_left(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "pageLeft", |vp| vp.scroll_x)
}

fn native_vv_get_page_top(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "pageTop", |vp| vp.scroll_y)
}

/// `visualViewport.width` / `.height` (CSSOM-View §12.1) — the visual viewport
/// size, equal to the layout viewport (`inner_width`/`inner_height`) when
/// `scale == 1`.
fn native_vv_get_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "width", |vp| vp.inner_width)
}

fn native_vv_get_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "height", |vp| vp.inner_height)
}

/// `visualViewport.scale` (CSSOM-View §12.1) — the pinch-zoom scale factor. Three
/// spec steps: (1) not fully active → 0 (the shared guard); (2) no output device →
/// 1; (3) otherwise → the scale factor. elidex models no pinch-zoom on a UA with an
/// output device, so steps 2+3 collapse to the constant `1`.
fn native_vv_get_scale(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    vv_geometry_read(ctx, this, "scale", |_vp| 1.0)
}
