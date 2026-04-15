//! `Window.prototype` intrinsic (WHATWG HTML §7.2).
//!
//! The `globalThis` / `window` object is a `HostObject` (backed by a
//! dedicated Window ECS entity), and its prototype chain is:
//!
//! ```text
//! globalThis (HostObject)
//!   → Window.prototype        (this intrinsic)
//!     → EventTarget.prototype (PR3)
//!       → Object.prototype    (bootstrap)
//! ```
//!
//! Inheriting from `EventTarget.prototype` is what makes
//! `window.addEventListener('scroll', …)` resolve the same way as
//! `element.addEventListener(…)` — no per-entity method install, just
//! prototype lookup.  Because the `HostObject` carries the Window
//! entity's `entity_bits`, the shared `addEventListener` native looks
//! up `ctx.host().dom()` and records the listener against the correct
//! ECS entity (distinct from the Document).
//!
//! At C2 `Window.prototype` is an **empty** object — its sole purpose
//! is to provide the inheritance seam.  Window-specific own-properties
//! (`innerWidth`, `scrollX`, `scrollTo`, `navigator`, `location`, …)
//! are installed by later PR4b commits either on this prototype (for
//! shared accessor/method slots) or as globals on `globalThis`.

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::VmInner;

/// In-memory viewport state (size + scroll offset) backing the
/// `innerWidth` / `innerHeight` / `scrollX` / `scrollY` /
/// `devicePixelRatio` window getters.
///
/// Phase 2 values are fixed defaults until the shell integration
/// pushes real values (PR6):
///
/// - `inner_width` / `inner_height` — 1024 × 768 CSS pixels,
///   matching the most common responsive-breakpoint assumption.
/// - `scroll_x` / `scroll_y` — mutated by `scrollTo` / `scrollBy`
///   but not otherwise observable until compositing lands.
/// - `device_pixel_ratio` — 1.0 (browsers on standard DPI
///   displays).
#[derive(Debug)]
pub(crate) struct ViewportState {
    pub(crate) inner_width: f64,
    pub(crate) inner_height: f64,
    pub(crate) scroll_x: f64,
    pub(crate) scroll_y: f64,
    pub(crate) device_pixel_ratio: f64,
}

impl ViewportState {
    pub(crate) fn new() -> Self {
        Self {
            inner_width: 1024.0,
            inner_height: 768.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            device_pixel_ratio: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// scrollTo / scrollBy
// ---------------------------------------------------------------------------
//
// Store scroll position on `VmInner::viewport` (added in C8 alongside
// this module).  Phase 2 is purely in-memory — the shell has not yet
// been wired to an actual render surface, so updating these fields
// has no visible effect, but `scrollX` / `scrollY` read them back so
// JS observes self-consistent state.

fn to_f64_or_zero(ctx: &mut NativeContext<'_>, v: JsValue) -> Result<f64, VmError> {
    match v {
        JsValue::Undefined => Ok(0.0),
        other => coerce::to_number(ctx.vm, other),
    }
}

pub(super) fn native_window_scroll_to(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // §CSSOM-View: `scrollTo(x, y)` or `scrollTo({left, top})`.  We
    // support only the positional form here — the options-object
    // form lands with the full scroll-anchoring implementation in a
    // later PR.
    let x = to_f64_or_zero(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    let y = to_f64_or_zero(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    // NaN → 0 per CSSOM-View "normalizing scroll amounts".
    ctx.vm.viewport.scroll_x = if x.is_finite() { x } else { 0.0 };
    ctx.vm.viewport.scroll_y = if y.is_finite() { y } else { 0.0 };
    Ok(JsValue::Undefined)
}

pub(super) fn native_window_scroll_by(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let dx = to_f64_or_zero(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    let dy = to_f64_or_zero(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    if dx.is_finite() {
        ctx.vm.viewport.scroll_x += dx;
    }
    if dy.is_finite() {
        ctx.vm.viewport.scroll_y += dy;
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Viewport / scroll getters
// ---------------------------------------------------------------------------

pub(super) fn native_window_get_inner_width(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.inner_width))
}

pub(super) fn native_window_get_inner_height(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.inner_height))
}

pub(super) fn native_window_get_scroll_x(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.scroll_x))
}

pub(super) fn native_window_get_scroll_y(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.scroll_y))
}

pub(super) fn native_window_get_device_pixel_ratio(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(ctx.vm.viewport.device_pixel_ratio))
}

impl VmInner {
    /// Populate `self.window_prototype` with the window-specific
    /// own-property suite (viewport accessors + scrollTo/scrollBy)
    /// whose prototype chain terminates at `EventTarget.prototype`.
    ///
    /// Called from `register_globals()` **after**
    /// `register_event_target_prototype()` — the latter's result is
    /// what this method chains to.
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` has not been populated
    /// (would mean `register_event_target_prototype` was skipped or
    /// called in the wrong order).
    pub(in crate::vm) fn register_window_prototype(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_window_prototype called before register_event_target_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });

        // Methods on Window.prototype — every `globalThis` shares the
        // same fn_id, matching the browser pattern where `Window`
        // methods live on the prototype rather than each instance.
        let methods: &[(&str, super::super::NativeFn)] = &[
            ("scrollTo", native_window_scroll_to),
            ("scrollBy", native_window_scroll_by),
        ];
        for &(name, func) in methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                proto_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // Read-only accessors.  Aliases (`pageXOffset` ≡ `scrollX`,
        // `pageYOffset` ≡ `scrollY`) share the same getter fn_id.
        let g_scroll_x = self.create_native_function("get scrollX", native_window_get_scroll_x);
        let g_scroll_y = self.create_native_function("get scrollY", native_window_get_scroll_y);
        let g_width = self.create_native_function("get innerWidth", native_window_get_inner_width);
        let g_height =
            self.create_native_function("get innerHeight", native_window_get_inner_height);
        let g_dpr = self
            .create_native_function("get devicePixelRatio", native_window_get_device_pixel_ratio);
        let accessors: &[(&str, super::super::value::ObjectId)] = &[
            ("innerWidth", g_width),
            ("innerHeight", g_height),
            ("scrollX", g_scroll_x),
            ("scrollY", g_scroll_y),
            ("pageXOffset", g_scroll_x),
            ("pageYOffset", g_scroll_y),
            ("devicePixelRatio", g_dpr),
        ];
        for &(name, gid) in accessors {
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                proto_id,
                key,
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        self.window_prototype = Some(proto_id);
    }

    /// Install `globalThis.window = globalThis` — the WHATWG HTML
    /// §7.2 self-reference that makes `window === globalThis` hold.
    ///
    /// Also used for scripts that use `window.X` to access a global
    /// unambiguously (distinguishing from a local `X` with the same
    /// name).
    pub(in crate::vm) fn install_window_self_ref(&mut self) {
        let name = self.well_known.window;
        self.globals
            .insert(name, JsValue::Object(self.global_object));
    }
}
