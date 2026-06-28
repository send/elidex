//! `screen` global — the `Screen` interface (CSSOM-View §4.3 *The Screen
//! Interface*; the `window.screen` attribute is §4 *Extensions to the Window
//! Interface*).
//!
//! S5-2 minor-window-parity. `Screen` is NOT an `EventTarget` and NOT a `Node`
//! (CSSOM-View §4.3 declares no supertype), so its prototype chain is the
//! minimal:
//!
//! ```text
//! Screen instance (ObjectKind::Screen, payload-free)
//!   → Screen.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! Mirrors [`VisualViewport`](super::visual_viewport) **minus the EventTarget
//! surface** — there is no `Screen.prototype → EventTarget.prototype` link and
//! no `addEventListener` / `on*` handlers (the interface defines none).
//!
//! ## Device-fact source (monitor dims, not viewport)
//!
//! The interface members are **value-derived RO accessors** reading the single
//! transported device-facts SoT [`ViewportState`](super::window::ViewportState):
//!
//! - `width` / `height` / `availWidth` / `availHeight` report the **monitor**
//!   (display) CSS-pixel size from the dedicated `screen_*` / `avail_*` fields —
//!   NOT the layout viewport (`inner_width`). A non-maximized window legitimately
//!   has `screen.width > window.innerWidth` (the boa parity: boa reads
//!   `bridge.monitor_width()`, not the viewport). The dims arrive over the
//!   dedicated [`set_screen_dimensions`](crate::vm::Vm::set_screen_dimensions)
//!   transport endpoint (a device-fact push with no delivery turn — there is no
//!   `change` event for `screen`); the live shell observe rides the S5-6 flip.
//!   `availWidth`/`availHeight` use the full monitor dims until a work-area
//!   source lands (`#11-screen-available-area-workarea-source`).
//! - `colorDepth` / `pixelDepth` report the universal `24` (8 bits per RGB
//!   channel) every browser returns — CSSOM-View §4.3 sanctions returning 24
//!   when the UA does not expose color depth, and the two return the same value
//!   "for compatibility reasons". Constant, not a transported fact.
//!
//! ## Singleton + identity + GC
//!
//! `window.screen` is a `[SameObject, Replaceable]` readonly attribute
//! (CSSOM-View §4). It is installed as a **no-setter RO accessor on
//! `Window.prototype`** whose getter returns the cached singleton `ObjectId`
//! (the `localStorage` / `[SameObject]` form), normalizing it onto the same
//! treatment its sibling `[Replaceable]` Window attrs (`innerWidth` / `scrollX`
//! / `devicePixelRatio`) already use — replacing the anomalous writable
//! `globals.insert`. (Proper `[Replaceable]` value-shadowing is implemented for
//! NONE of the family → deferred engine-wide, `#11-window-platform-object-rigor-engine-wide`.)
//! The singleton is cached in `VmInner::screen_instance` (rooted via the GC
//! proto-roots, SameObject for free) and **cleared on `Vm::unbind`** (the
//! `localStorage` cross-DOM precedent) so a *fresh* `window.screen` read after a
//! rebind allocates a NEW singleton rather than handing back a stale `ObjectId`
//! from the prior `EcsDom`. A script that *retained* the old `screen` across the
//! rebind, however, still reads the current `ViewportState` through the shared
//! getter — it does NOT become the §4.3 not-fully-active object for its old
//! associated document. That residual cross-DOM wrapper-aliasing is the
//! engine-wide gap shared by every cached singleton (`localStorage` /
//! `subtleCrypto` / `visualViewport`), whose payload-free brands carry no
//! per-document identity; it is resolved uniformly by the world-id discriminator
//! program (`#11-wrapper-cache-cross-dom-discriminator`), not by a Screen-only
//! patch (One-issue-one-way).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `Screen.prototype` + the illegal-constructor `Screen` interface
    /// object, exposing the `Screen` global and the `window.screen`
    /// no-setter RO accessor (the singleton instance is allocated lazily by the
    /// getter via [`Self::alloc_or_cached_screen`]). Window realm only
    /// (`[Exposed=Window]`); the caller gates this on `GlobalScopeKind::Window`.
    ///
    /// Called from `register_globals()`. Chains `Screen.prototype` to
    /// `Object.prototype` (Screen is not an EventTarget).
    pub(in crate::vm) fn register_screen_global(&mut self) {
        // ---- Screen.prototype ----
        // `create_object_with_methods(&[])` gives the ordinary plain-object
        // allocation + `Object.prototype` wiring for free (no methods — the
        // interface is accessor-only).
        let proto_id = self.create_object_with_methods(&[]);
        self.install_ro_accessors(proto_id, SCREEN_RO_ACCESSORS);
        self.screen_prototype = Some(proto_id);

        // ---- Screen interface object ----
        // WebIDL: `Screen` declares NO constructor — `new Screen()` / `Screen()`
        // throw a TypeError. Registered as an illegal-constructor so `screen
        // instanceof Screen` and `Screen.prototype` parity work (the
        // `VisualViewport` / `MediaQueryList` precedent). boa exposed no `Screen`
        // constructor either, so VM ≥ boa holds.
        let ctor = self.create_illegal_constructor_function(
            "Screen",
            super::super::value::native_illegal_constructor_unreachable,
        );
        self.wire_interface_ctor_prototype(ctor, proto_id);
        let ctor_name = self.strings.intern("Screen");
        self.globals.insert(ctor_name, JsValue::Object(ctor));
    }

    /// Return the cached `Screen` singleton, allocating it on the first
    /// `window.screen` read. `[SameObject]`: the same `ObjectId` is returned
    /// across reads for the lifetime of one bind cycle (cleared on `Vm::unbind`
    /// via [`Self::clear_window_parity_instance_cache`]). Mirrors
    /// [`Self::alloc_or_cached_subtle_crypto`].
    pub(in crate::vm) fn alloc_or_cached_screen(&mut self) -> ObjectId {
        if let Some(id) = self.screen_instance {
            return id;
        }
        let proto = self
            .screen_prototype
            .expect("alloc_or_cached_screen before register_screen_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::Screen,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: true,
        });
        self.screen_instance = Some(id);
        id
    }
}

/// `Screen`'s value-derived RO accessors (CSSOM-View §4.3). Each reads the
/// VM-global [`ViewportState`](super::window::ViewportState) after the WebIDL
/// branded-receiver gate.
const SCREEN_RO_ACCESSORS: &[(&str, NativeFn)] = &[
    ("width", native_screen_get_width),
    ("height", native_screen_get_height),
    ("availWidth", native_screen_get_avail_width),
    ("availHeight", native_screen_get_avail_height),
    ("colorDepth", native_screen_get_color_depth),
    ("pixelDepth", native_screen_get_color_depth),
];

/// WebIDL branded-receiver gate for `Screen.prototype.*` attribute getters.
/// Throws a TypeError ("illegal invocation") on a non-branded receiver (the
/// `VisualViewport` stance — boa skipped this; the VM enforces it).
fn require_screen_this(ctx: &NativeContext<'_>, this: JsValue, attr: &str) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::Screen) {
            return Ok(());
        }
    }
    Err(VmError::type_error(format!(
        "Failed to read the '{attr}' property from 'Screen': illegal invocation"
    )))
}

/// `screen.width` (CSSOM-View §4.3) — the **monitor** CSS-px width truncated to
/// a WebIDL `long` (integer-valued; the shell pushes integer CSS px, `.trunc()`
/// honours the IDL type if a fractional value ever arrives). Reads the dedicated
/// `screen_width` device fact, NOT `inner_width` (T1: a non-maximized window's
/// `screen.width` is the display size, not the window size).
fn native_screen_get_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_screen_this(ctx, this, "width")?;
    Ok(JsValue::Number(ctx.vm.viewport.screen_width.trunc()))
}

/// `screen.height` (CSSOM-View §4.3) — the monitor CSS-px height.
fn native_screen_get_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_screen_this(ctx, this, "height")?;
    Ok(JsValue::Number(ctx.vm.viewport.screen_height.trunc()))
}

/// `screen.availWidth` (CSSOM-View §4.3) — the OS-chrome-excluded available
/// monitor width. winit exposes no cross-platform work-area API → reads the full
/// monitor `avail_width` field (boa parity; real work-area source deferred to
/// `#11-screen-available-area-workarea-source`).
fn native_screen_get_avail_width(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_screen_this(ctx, this, "availWidth")?;
    Ok(JsValue::Number(ctx.vm.viewport.avail_width.trunc()))
}

/// `screen.availHeight` (CSSOM-View §4.3) — the available monitor height.
fn native_screen_get_avail_height(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_screen_this(ctx, this, "availHeight")?;
    Ok(JsValue::Number(ctx.vm.viewport.avail_height.trunc()))
}

/// `screen.colorDepth` / `screen.pixelDepth` (CSSOM-View §4.3) — the universal
/// `24` (8 bits per RGB channel) every browser returns. The exact value, not a
/// placeholder (a deeper-color device-fact transport is not a thing the engine
/// needs to represent).
fn native_screen_get_color_depth(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_screen_this(ctx, this, "colorDepth")?;
    Ok(JsValue::Number(24.0))
}
