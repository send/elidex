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
//! ## S5-2 design — real EventTarget, not a stub
//!
//! boa exposed `visualViewport` as a plain object with **stub** (no-op)
//! `addEventListener` / `removeEventListener`. The VM makes it a genuine
//! `EventTarget` singleton: `addEventListener('resize'|'scroll'|'scrollend',
//! …)` / `removeEventListener` / `dispatchEvent` are **inherited** from
//! `EventTarget.prototype` (routed to the object's `vm_event_listeners` home via
//! [`DispatchTarget::VmObject`](super::dispatch_target::DispatchTarget), gated by
//! [`ObjectKind::is_non_node_event_target`](super::super::value::ObjectKind::is_non_node_event_target)),
//! and `onresize` / `onscroll` / `onscrollend` are event-handler IDL attributes.
//! This is the spec-faithful shape (CSSOM-View §12.1 `: EventTarget`) and
//! strictly exceeds boa (real listener storage vs. silently-dropped stubs).
//!
//! ## Singleton + GC
//!
//! `window.visualViewport` is a `[SameObject]` per-window singleton, created
//! once at registration and held as a fixed `globals` entry (the `navigator` /
//! `screen` precedent): every read resolves the same `ObjectId` (SameObject for
//! free) and the `globals` map roots it (a GC root). It is therefore **never**
//! kept alive only by a listener, so the listener-keepalive-rooting hazard the
//! generic mechanism `#11-eventtarget-listener-keepalive-rooting` (S5-3) covers
//! does **not** apply here (it is a permanently-rooted singleton, like `window`
//! itself). The brand is payload-free; GC has nothing to trace or prune (the
//! `TextEncoder` / `MediaQueryList`-prototype precedent).
//!
//! ## Geometry source + deferred fidelity (presence-first)
//!
//! The seven `readonly attribute double`s are value-derived getters reading the
//! single transported device-facts SoT
//! [`ViewportState`](super::window::ViewportState):
//!
//! - `width` / `height` → the visual viewport size = `inner_width` /
//!   `inner_height` (equal to the layout viewport when `scale == 1`).
//! - `pageLeft` / `pageTop` → the page-relative offset = `scroll_x` / `scroll_y`
//!   (layout-viewport scroll + the visual `offsetLeft`/`offsetTop`, which are 0).
//! - `offsetLeft` / `offsetTop` → `0` and `scale` → `1` — the **exact** values
//!   for a UA with no pinch-zoom (which the engine does not model), not
//!   placeholders.
//!
//! The `resize` / `scroll` / `scrollend` events have no shell producer yet, so
//! they never fire (listeners are stored, just not invoked — the same pre-producer
//! state `MediaQueryList` was in before `deliver_media_query_changes`). A
//! pinch-zoom + visual-viewport-event producer (live `scale` / `offset` + event
//! delivery) is deferred to slot `#11-s5-2-window-parity-live-producers`.

#![cfg(feature = "engine")]

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `VisualViewport.prototype`, the illegal-constructor interface
    /// object, and the per-window singleton instance, exposing
    /// `window.visualViewport` + the `VisualViewport` global.
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

        // ---- the per-window singleton instance ----
        // Created once, held as a fixed `globals` entry: SameObject for free
        // (every `window.visualViewport` read resolves the same id) + rooted by
        // the `globals` GC root, so it is never listener-only-rooted (§module
        // docs: the S5-3 keepalive hazard does not apply).
        let instance = self.alloc_object(Object {
            kind: ObjectKind::VisualViewport,
            storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
            prototype: Some(proto_id),
            extensible: true,
        });
        let attr_name = self.strings.intern("visualViewport");
        self.globals.insert(attr_name, JsValue::Object(instance));
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

/// `visualViewport.offsetLeft` / `.offsetTop` (CSSOM-View §12.1) — the offset of
/// the visual viewport from the layout viewport: `0` with no pinch-zoom (the
/// engine models none), the exact value rather than a placeholder.
fn native_vv_get_offset_left(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "offsetLeft")?;
    Ok(JsValue::Number(0.0))
}

fn native_vv_get_offset_top(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "offsetTop")?;
    Ok(JsValue::Number(0.0))
}

/// `visualViewport.pageLeft` / `.pageTop` (CSSOM-View §12.1) — the page-relative
/// offset = layout-viewport scroll (`scroll_x`/`scroll_y`) + the visual
/// `offsetLeft`/`offsetTop` (which are 0).
fn native_vv_get_page_left(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "pageLeft")?;
    Ok(JsValue::Number(ctx.vm.viewport.scroll_x))
}

fn native_vv_get_page_top(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "pageTop")?;
    Ok(JsValue::Number(ctx.vm.viewport.scroll_y))
}

/// `visualViewport.width` / `.height` (CSSOM-View §12.1) — the visual viewport
/// size, equal to the layout viewport (`inner_width`/`inner_height`) when
/// `scale == 1`.
fn native_vv_get_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "width")?;
    Ok(JsValue::Number(ctx.vm.viewport.inner_width))
}

fn native_vv_get_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "height")?;
    Ok(JsValue::Number(ctx.vm.viewport.inner_height))
}

/// `visualViewport.scale` (CSSOM-View §12.1) — the pinch-zoom scale factor: `1`
/// with no pinch-zoom (exact, not a placeholder).
fn native_vv_get_scale(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_visual_viewport_this(ctx, this, "scale")?;
    Ok(JsValue::Number(1.0))
}
