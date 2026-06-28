//! `screen` global — the `Screen` interface (CSSOM-View §4.3 *The Screen
//! Interface*; the `window.screen` attribute is §4 *Extensions to the Window
//! Interface*).
//!
//! S5-2 minor-window-parity. Like `navigator` (`host/navigator.rs`), `screen`
//! is installed as a **plain object** carrying the interface's `readonly
//! attribute`s — NOT an `EventTarget`, NOT a `Node` (CSSOM-View §4.3 declares no
//! supertype). The `Screen` **interface object** (`new Screen()` / `screen
//! instanceof Screen`) is deferred with the sibling `navigator` interface-object
//! branding (slot `#11-navigator-interface-object-branding`): boa exposed no
//! `Screen` constructor either, so VM ≥ boa holds.
//!
//! ## Device-fact source (presence-first)
//!
//! The interface members are **value-derived RO accessors** reading the single
//! transported device-facts SoT [`ViewportState`](super::window::ViewportState)
//! — the same struct `innerWidth` / `devicePixelRatio` / `matchMedia` read. The
//! VM has no separate *monitor*-resolution transport yet, so:
//!
//! - `width` / `height` / `availWidth` / `availHeight` report the **viewport**
//!   CSS-pixel size (`inner_width` / `inner_height`). This is the presence-first
//!   approximation a maximized window satisfies exactly, and it uses real
//!   transported data rather than a fabricated monitor size — a responsive
//!   `screen.width <= 768` probe then tracks the actual window, which is
//!   arguably more useful than a hard-coded desktop resolution. `availWidth` =
//!   `width` / `availHeight` = `height` (no OS-taskbar inset is transported);
//!   boa likewise aliased `availWidth`→`width`.
//! - `colorDepth` / `pixelDepth` report the universal `24` (8 bits per RGB
//!   channel) every browser returns; this is the exact value, not a placeholder.
//!
//! A real monitor-resolution + available-rect device-fact transport (distinct
//! from the layout viewport) is deferred to slot
//! `#11-s5-2-window-parity-live-producers` (the producer rides the shell
//! device-facts wiring, like the `color_scheme` / `reduced_motion` prefs the
//! `ViewportState` already carries with a deferred producer). The interface
//! SHAPE is spec-faithful today; only the monitor-vs-viewport distinction is the
//! deferred fidelity.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;

impl VmInner {
    /// Install `globalThis.screen` — a plain object with the `Screen`
    /// interface's six RO accessors (CSSOM-View §4.3). Window realm only
    /// (`[Exposed=Window]`); the caller gates this on
    /// `GlobalScopeKind::Window`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (`create_object_with_methods` chains the object to `Object.prototype`).
    pub(in crate::vm) fn register_screen_global(&mut self) {
        // No methods — an empty slice gives the ordinary plain-object
        // allocation + `Object.prototype` wiring for free (navigator parity).
        let obj_id = self.create_object_with_methods(&[]);
        self.install_ro_accessors(obj_id, SCREEN_RO_ACCESSORS);

        // `window.screen` is a `[SameObject]` readonly attribute (CSSOM-View §4)
        // — a fixed `globals` entry holding the one Screen object is SameObject
        // for free (every read resolves the same `ObjectId`) and roots it (the
        // `globals` map is a GC root), the `navigator` precedent.
        let name = self.strings.intern("screen");
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}

/// `Screen`'s value-derived RO accessors (CSSOM-View §4.3). Each reads the
/// VM-global [`ViewportState`](super::window::ViewportState); none holds
/// per-instance state, so — like `navigator.cookieEnabled` — the getters do
/// not brand-check `this`.
const SCREEN_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("width", native_screen_get_width),
    ("availWidth", native_screen_get_width),
    ("height", native_screen_get_height),
    ("availHeight", native_screen_get_height),
    ("colorDepth", native_screen_get_color_depth),
    ("pixelDepth", native_screen_get_color_depth),
];

/// `screen.width` / `screen.availWidth` (CSSOM-View §4.3) — the viewport CSS-px
/// width truncated to a WebIDL `long` (integer-valued; the shell pushes integer
/// CSS px, `.trunc()` honours the IDL type if a fractional value ever arrives).
fn native_screen_get_width(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.inner_width.trunc()))
}

/// `screen.height` / `screen.availHeight` (CSSOM-View §4.3).
fn native_screen_get_height(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.inner_height.trunc()))
}

/// `screen.colorDepth` / `screen.pixelDepth` (CSSOM-View §4.3) — the universal
/// `24` (8 bits per RGB channel) every browser returns. The exact value, not a
/// placeholder (a deeper-color device-fact transport is not a thing the engine
/// needs to represent).
fn native_screen_get_color_depth(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(24.0))
}
