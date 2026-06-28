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
//! `Window.prototype` carries the viewport accessors
//! (`innerWidth` / `scrollX` / `devicePixelRatio` / …), the scroll
//! methods (`scrollTo` / `scrollBy`), the WindowProxy iframe
//! accessors (`self` / `parent` / `top` / `frames` / `frameElement` /
//! `opener` / `length` / `closed`, WHATWG HTML §7.2.2), and the
//! writable `name` accessor pair so every `globalThis` reads them
//! from the shared prototype rather than each wrapper holding its
//! own copy.  Global singletons that are values rather than
//! prototype-shared behaviour (`navigator`, `location`, `history`,
//! `performance`, `document`) live on `globalThis` itself and are
//! installed by their respective `register_*_global()` helpers.

#![cfg(feature = "engine")]

use elidex_css::media::{ColorScheme, ReducedMotion};
// Read only by the (compat-webapi-gated) Web Storage accessor install (A2).
#[cfg(feature = "compat-webapi")]
use elidex_script_session::web_storage_spec_level;

use super::super::coerce;
use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::super::VmInner;

/// The single transported device-facts SoT for this window, backing the
/// `innerWidth` / `innerHeight` / `scrollX` / `scrollY` / `devicePixelRatio`
/// window getters, the `screen.*` monitor-dims getters, the `visualViewport`
/// geometry getters, AND the media-query evaluator. It holds viewport geometry
/// (`inner_width` / `inner_height` / `device_pixel_ratio`), live scroll, the
/// monitor dims (`screen_width` / `screen_height` / `avail_width` /
/// `avail_height`), and the user-preference media facts (`color_scheme` /
/// `reduced_motion`).
///
/// It is the **one** struct the shell drives device state through (mirroring
/// the engine-independent [`elidex_css::media::MediaEnvironment`]'s own
/// co-location), so
/// [`VmInner::media_environment`](crate::vm::VmInner::media_environment) derives
/// the evaluator environment from here rather than from a second prefs struct
/// (one-issue-one-way). The name reads "viewport" for history; it is a
/// **superset** of geometry — the prefs + monitor-dims fields are device facts
/// too. The viewport/prefs fields are pushed by `set_media_environment`; the
/// monitor dims by the dedicated `set_screen_dimensions` endpoint (NOT a media
/// input — no `change` event, no media re-eval turn).
///
/// Defaults: 1024×768 CSS px viewport, 1.0 dppx, `Light` / `NoPreference`, and a
/// 1920×1080 monitor (DISTINCT from the viewport default so `screen.width !==
/// innerWidth` out of the box), until the shell pushes real values (`scroll_x` /
/// `scroll_y` are driven by `scrollTo` / `scrollBy`).
#[derive(Debug)]
pub(crate) struct ViewportState {
    pub(crate) inner_width: f64,
    pub(crate) inner_height: f64,
    pub(crate) scroll_x: f64,
    pub(crate) scroll_y: f64,
    pub(crate) device_pixel_ratio: f64,
    /// The **monitor** (display) CSS-px width (CSSOM-View §4.3 `Screen.width`).
    /// A device fact DISTINCT from `inner_width` (the layout viewport): a
    /// non-maximized window has `inner_width < screen_width`. Pushed by the
    /// shell's `set_screen_dimensions` transport (the producer rides the S5-6
    /// flip); a realistic 1920×1080 desktop default until then. Read only by
    /// the `screen.*` getters — NOT a `MediaEnvironment` input (no media
    /// feature reads it, no `change` event for `screen`).
    pub(crate) screen_width: f64,
    /// The monitor CSS-px height (CSSOM-View §4.3 `Screen.height`). Sibling of
    /// [`Self::screen_width`].
    pub(crate) screen_height: f64,
    /// The **available** monitor CSS-px width (CSSOM-View §4.3
    /// `Screen.availWidth`) — the OS-chrome-excluded screen area. winit exposes
    /// no cross-platform work-area API, so the shell pushes the full monitor
    /// dims here (boa parity, common UA fallback; the real work-area source is
    /// `#11-screen-available-area-workarea-source`).
    pub(crate) avail_width: f64,
    /// The available monitor CSS-px height (CSSOM-View §4.3 `Screen.availHeight`).
    /// Sibling of [`Self::avail_width`].
    pub(crate) avail_height: f64,
    /// `prefers-color-scheme` user preference (MQ5 §12.5). Defaults to
    /// `Light` (UA convention, no active preference); the shell's
    /// theme-change producer (carved `#11-media-prefers-features`) will drive
    /// it via `set_media_environment`. VM tests drive it directly.
    pub(crate) color_scheme: ColorScheme,
    /// `prefers-reduced-motion` user preference (MQ5 §12.1). Defaults to
    /// `NoPreference`; producer wiring carved like `color_scheme`.
    pub(crate) reduced_motion: ReducedMotion,
    /// Scroll offset a script requested via `scrollTo` / `scrollBy`
    /// (CSSOM View §4) that the shell has not yet applied + echoed back.
    /// Drained by [`Vm::take_pending_scroll`](crate::vm::Vm::take_pending_scroll);
    /// `scroll_x`/`scroll_y` are updated eagerly alongside it so JS reads
    /// stay self-consistent, while the shell clamps + echoes the applied
    /// value back via [`Vm::set_scroll_offset`](crate::vm::Vm::set_scroll_offset).
    pub(crate) pending_scroll: Option<(f64, f64)>,
}

impl ViewportState {
    pub(crate) fn new() -> Self {
        Self {
            inner_width: 1024.0,
            inner_height: 768.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            device_pixel_ratio: 1.0,
            // Realistic desktop monitor default (1920×1080), DISTINCT from the
            // 1024×768 viewport default so a test/page can observe `screen.width
            // !== innerWidth` out of the box; overridden by the
            // `set_screen_dimensions` producer at the flip. `avail_* = full`
            // until a work-area source lands (§9).
            screen_width: 1920.0,
            screen_height: 1080.0,
            avail_width: 1920.0,
            avail_height: 1080.0,
            // #360 `MediaEnvironment::default` prefs (MQ5 §12.5 / §12.1) until
            // the shell producer wiring lands (`#11-media-prefers-features`).
            color_scheme: ColorScheme::Light,
            reduced_motion: ReducedMotion::NoPreference,
            pending_scroll: None,
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

/// Parse the CSSOM-View `scroll()` / `scrollBy()` argument overloads: the
/// two-argument positional form `(x, y)` or a single `ScrollToOptions`
/// dictionary `{ left, top }` (CSSOM-View §4 "Extensions to the Window
/// Interface"). Returns `(left, top)` where an absent dictionary member is
/// `None` so the caller substitutes the per-method default — the current offset
/// for `scrollTo` (absolute), `0` for `scrollBy` (delta). The `behavior` member
/// (`auto` / `instant` / `smooth`) is a UA hint this engine does not honour — it
/// always scrolls instantly, which is conforming (the spec lets a UA realize the
/// requested scroll behaviour at its own discretion); it is not a pending
/// feature. It is still **validated** as a `ScrollBehavior` enum
/// ([`validate_scroll_behavior`]) — Web IDL rejects an invalid value with a
/// TypeError even when the value is unused.
///
/// Restores the options-object overload the boa→VM scroll cutover dropped: the
/// replaced boa path parsed `{ left, top }`, so without this
/// `window.scrollTo({ top: 100 })` would coerce the object to `NaN`→0 and
/// silently scroll to the origin.
fn parse_scroll_args(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<(Option<f64>, Option<f64>), VmError> {
    // Two-argument positional overload: `x` and `y` are the arguments.
    if args.len() >= 2 {
        let x = coerce::to_number(ctx.vm, args[0])?;
        let y = coerce::to_number(ctx.vm, args[1])?;
        return Ok((Some(x), Some(y)));
    }
    // One-argument options overload.
    if let Some(&first) = args.first() {
        match first {
            // `null` / `undefined` convert to an EMPTY `ScrollToOptions`
            // dictionary (Web IDL §3.2.17), so `scrollTo(null)` holds the
            // current offset (both members absent — a no-op) rather than
            // scrolling to the origin.
            JsValue::Null | JsValue::Undefined => Ok((None, None)),
            // A `{ left, top }` dictionary. Web IDL converts dictionary members
            // in lexicographic order — `behavior` before `left`/`top` — and
            // `behavior` is a `ScrollBehavior` enum, so an invalid value must
            // throw HERE, before any offset is queued (validated even though the
            // hint is not honoured — see `validate_scroll_behavior`).
            JsValue::Object(id) => {
                validate_scroll_behavior(ctx, id)?;
                let left = read_optional_scroll_member(ctx, id, "left")?;
                let top = read_optional_scroll_member(ctx, id, "top")?;
                Ok((left, top))
            }
            // A lone NON-object primitive (number / string / boolean) still
            // resolves to the one-argument options overload — Web IDL §3.2.17
            // converts it to a `ScrollToOptions` dictionary, which throws a
            // TypeError because it is not an object. Matches browsers
            // (`scrollTo(40)` is a TypeError, not a positional `x`); the
            // two-argument positional overload requires BOTH `x` and `y`.
            _ => Err(VmError::type_error(
                "Failed to execute 'scrollTo'/'scrollBy': the provided value is not of type 'ScrollToOptions'.",
            )),
        }
    } else {
        // No arguments: an empty options dictionary — both members absent, so
        // each method holds its current offset (a no-op scroll).
        Ok((None, None))
    }
}

/// Validate the `ScrollToOptions` `behavior` member as a `ScrollBehavior` enum
/// (CSSOM-View §4 — `enum ScrollBehavior { "auto", "instant", "smooth" }`). Read
/// via `[[Get]]`; an absent / `undefined` member is the `"auto"` default (a
/// no-op), any other value is `ToString`-coerced and must match an enum member
/// or Web IDL throws a TypeError before the scroll runs. The value is not
/// otherwise used (this engine scrolls instantly regardless, see
/// [`parse_scroll_args`]), but the conversion's rejection of an invalid value is
/// script-observable, so it cannot be skipped (Codex S2 final pass).
fn validate_scroll_behavior(ctx: &mut NativeContext<'_>, obj_id: ObjectId) -> Result<(), VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern("behavior"));
    let raw = ctx.get_property_value(obj_id, key)?;
    if matches!(raw, JsValue::Undefined) {
        return Ok(());
    }
    let sid = ctx.to_string_val(raw)?;
    let s = ctx.get_utf8(sid);
    if matches!(s.as_str(), "auto" | "instant" | "smooth") {
        Ok(())
    } else {
        Err(VmError::type_error(format!(
            "Failed to read the 'behavior' property from 'ScrollToOptions': the provided value \
             '{s}' is not a valid enum value of type ScrollBehavior."
        )))
    }
}

/// Read a `ScrollToOptions` numeric member via `[[Get]]`, returning `None` when
/// the member is absent / `undefined` so the caller applies the per-method
/// default rather than `0`.
fn read_optional_scroll_member(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
    name: &str,
) -> Result<Option<f64>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(name));
    match ctx.get_property_value(obj_id, key)? {
        JsValue::Undefined => Ok(None),
        v => Ok(Some(coerce::to_number(ctx.vm, v)?)),
    }
}

pub(super) fn native_window_scroll_to(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // CSSOM-View §4 `scrollTo(x, y)` / `scrollTo({ left, top })`. An absent
    // dictionary member holds the current offset on that axis (step 1.2/1.3),
    // not 0 — so `scrollTo({ top: 100 })` keeps `scrollX`.
    let (left, top) = parse_scroll_args(ctx, args)?;
    let x = left.unwrap_or(ctx.vm.viewport.scroll_x);
    let y = top.unwrap_or(ctx.vm.viewport.scroll_y);
    // Normalize non-finite values to 0 (CSSOM-View step 3).
    ctx.vm.viewport.scroll_x = if x.is_finite() { x } else { 0.0 };
    ctx.vm.viewport.scroll_y = if y.is_finite() { y } else { 0.0 };
    // Record the request so the shell can apply it to the real viewport
    // (drained via `Vm::take_pending_scroll`); the eager `scroll_x/y`
    // update above keeps `scrollX`/`scrollY` self-consistent for JS reads
    // until the shell clamps + echoes the applied value back.
    ctx.vm.viewport.pending_scroll = Some((ctx.vm.viewport.scroll_x, ctx.vm.viewport.scroll_y));
    Ok(JsValue::Undefined)
}

pub(super) fn native_window_scroll_by(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // CSSOM-View §4 `scrollBy(dx, dy)` / `scrollBy({ left, top })` — an absent
    // dictionary member is a 0 delta on that axis.
    let (left, top) = parse_scroll_args(ctx, args)?;
    let dx = left.unwrap_or(0.0);
    let dy = top.unwrap_or(0.0);
    if dx.is_finite() {
        ctx.vm.viewport.scroll_x += dx;
    }
    if dy.is_finite() {
        ctx.vm.viewport.scroll_y += dy;
    }
    ctx.vm.viewport.pending_scroll = Some((ctx.vm.viewport.scroll_x, ctx.vm.viewport.scroll_y));
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

// ---------------------------------------------------------------------------
// Iframe-related WindowProxy getters (WHATWG HTML §7.2.2.4 / §7.2.3)
// `#11-windowproxy-browsing-context`
// ---------------------------------------------------------------------------
//
// Deferred stubs for `parent`/`top`/`frameElement`/`length`/`closed`
// — all under `#11-windowproxy-browsing-context`.
//
// `self` and `frames` getter bodies return `ctx.vm.global_object` and are
// spec-correct (§7.2.2: return `this`'s own global object), but only when
// C1+'s WindowProxy [[Get]] forwarding executes them inside the receiver's
// VM.  C1+ need not change the getter bodies, but MUST implement the
// forwarding mechanism; see receiver requirements below.  What C1+ must
// add separately is `frames[i]` indexed access — an exotic property
// operation on the WindowProxy (§7.2.3), not a change to the `frames`
// attribute getter itself.
//
// `opener` is included in this group **mechanically** (same null stub) but
// its correctness is tracked under a separate slot:
//   `#11-auxiliary-browsing-context-opener`
//   Why: `window.open()` auxiliary browsing-context creation is not yet
//     implemented; the opener WindowProxy depends on that mechanism.
//   Trigger: window.open() / auxiliary browsing-context program (post-S5).
//   Revisit: when window.open() is tackled.
// C1+ may close `#11-windowproxy-browsing-context` (sub-frame accessors)
// while leaving `opener` as a null stub under its own slot.
//
// Why: sub-frame browsing-context entity model and cross-VM
// Document/Window proxy identity are not yet implemented.  The VM
// currently models a single top-level browsing context — `self`,
// `parent`, `top`, and `frames` resolve to `globalThis`; `frameElement`
// and `opener` return `null`; `length` returns `0`; `closed` returns
// `false`.
//
// These stubs are correct only for a genuine top-level window with no
// opener.  For sub-frame or opened windows:
//   • `parent` / `top`: sub-frames must receive an ancestor WindowProxy
//     (§7.2.2.4), not globalThis — even cross-origin (restricted proxy).
//   • `frameElement`: `null` is spec-correct for cross-origin callers
//     (§7.2.2.4), but same-origin sub-frames must receive the element.
//   • `opener`: a window opened via `window.open()` must expose the
//     opener WindowProxy (possibly restricted cross-origin), NOT null.
//   • `length` / `closed`: reflect actual child-frame count / window
//     state, not 0 / false.
//
// Trigger: `world_id` / cross-DOM program + S5/boa removal.
// Revisit date: when the `world_id` / S5 program begins.
//
// Stubs currently ignore `_this`: single-VM, so there is no other
// browsing context to route to.  C1+ receiver requirements differ:
//   • `self` / `frames`: return `this`'s own global object (§7.2.2).
//     The getter body ignores `_this` and returns `ctx.vm.global_object`.
//     This is only spec-correct when C1+'s WindowProxy [[Get]] forwarding
//     executes the body inside the receiver's VM — making the executing
//     VM's global equal to the receiver's global.  C1+ must NOT leave
//     `_this` ignored without that forwarding; without it,
//     `childWindowProxy.self` returns the calling VM's global, not the
//     child's.
//   • `length` / `closed`: return state intrinsic to `this`'s own
//     context (child-frame count / browsing-context-null check per
//     §7.2.2.2 / §7.2.2.1).  These are NOT truly receiver-independent
//     in multi-VM — `childWindowProxy.length` must return the child's
//     frame count, not the parent's.  C1+ must dispatch to the receiver's
//     VM before reading these values (same dispatch mechanism as
//     `self`/`frames`, but the returned state is not trivially `this`).
//   • `parent` / `top` / `frameElement` / `opener`: return ancestor /
//     container / opener state from a *different* browsing context.
//     C1+ MUST make these receiver-relative (routing via the navigable
//     tree, not VM-wide state); keeping VM-wide state in the real
//     implementation would cause `childWindow.parent` to resolve
//     relative to the wrong window.

pub(super) fn native_window_get_self(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Object(ctx.vm.global_object))
}

pub(super) fn native_window_get_parent(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Object(ctx.vm.global_object))
}

pub(super) fn native_window_get_top(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Object(ctx.vm.global_object))
}

pub(super) fn native_window_get_frames(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Object(ctx.vm.global_object))
}

pub(super) fn native_window_get_frame_element(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Null)
}

pub(super) fn native_window_get_opener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Null)
}

pub(super) fn native_window_get_length(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(0.0))
}

pub(super) fn native_window_get_closed(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(false))
}

/// `window.name` (WHATWG HTML §7.3.3.5) — DOMString attribute that
/// survives same-document reloads.  The setter coerces with
/// `ToString` per WebIDL and stores into `VmInner::window_name`.
/// The cross-document reset described in §7.10.4 step 7 is **not**
/// applied by the current codebase (a repo-wide search shows only
/// init + setter writes touch the field); when navigation gains
/// that responsibility, the clear belongs in the navigation
/// pipeline rather than the getter / setter protocol here.
pub(super) fn native_window_get_name(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::String(ctx.vm.window_name))
}

pub(super) fn native_window_set_name(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    ctx.vm.window_name = coerce::to_string(ctx.vm, val)?;
    Ok(JsValue::Undefined)
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

        // `globalThis` shares this prototype's methods, matching the
        // browser pattern where `Window` methods live on the prototype
        // rather than each instance.
        self.install_methods(proto_id, WINDOW_METHODS);
        // `pageXOffset` / `pageYOffset` map to the same semantics as
        // `scrollX` / `scrollY`; the native bodies all read the shared
        // `ViewportState` so any pair points at the same slot.
        self.install_ro_accessors(proto_id, WINDOW_RO_ACCESSORS);
        // `window.screen` / `window.visualViewport` (CSSOM-View §4
        // Extensions to the Window Interface) — `[SameObject, Replaceable]`
        // readonly attributes installed as no-setter RO accessors returning the
        // cached singleton (the `localStorage` / `[SameObject]` form), NOT a
        // writable `globals.insert`. This normalizes them onto the same
        // treatment their sibling `[Replaceable]` Window attrs above
        // (`innerWidth` / `scrollX` / `devicePixelRatio`) already use; assigning
        // `screen = …` hits the inherited-no-setter branch (a silent no-op in
        // sloppy mode / throws in strict). S5-2.
        self.install_ro_accessors(proto_id, WINDOW_PARITY_ACCESSORS);
        // `name` is the only writable Window attribute the VM exposes;
        // its backing field (`VmInner::window_name`) is initialised to
        // an empty string and updated by the setter.
        self.install_rw_accessors(proto_id, WINDOW_RW_ACCESSORS);
        // `localStorage` / `sessionStorage` accessor pair (WHATWG HTML
        // §12.2.3 localStorage getter / §12.2.2 sessionStorage getter).
        // Read-only getter that returns the cached `Storage` wrapper from
        // `VmInner::alloc_or_cached_storage` so `localStorage === localStorage`
        // holds (`[SameObject]`).
        //
        // Seam-1a of the A1 Web-API core/compat gate: the Web Storage family's
        // Window-accessor surface, gated by the family-neutral `installs(level)`
        // predicate reading the family's SINGLE classification source
        // `web_storage_spec_level()` (Codex R7) — shared with the
        // `Storage`/`StorageEvent` globals (seam-2) and `window.onstorage` (seam-3),
        // so A2 demotes the whole family by flipping that one source — present
        // only under `BrowserCompat`. The accessors + their natives are
        // `compat-webapi`-gated (A2) so `App` builds drop them entirely.
        #[cfg(feature = "compat-webapi")]
        if self.installs(web_storage_spec_level()) {
            self.install_ro_accessors(proto_id, WINDOW_STORAGE_ACCESSORS);
        }
        // Event-handler IDL attributes (WHATWG HTML §8.1.8.2.1): Window
        // mixes in GlobalEventHandlers + WindowEventHandlers.  Both
        // target the Window entity directly (`entity_from_this(window)`),
        // so the normal (non-delegating) backend pair is used; the body
        // proto installs the WindowEventHandlers delegation overrides
        // separately.
        self.install_event_handler_attrs(
            proto_id,
            &[
                elidex_script_session::HandlerScope::Global,
                elidex_script_session::HandlerScope::Window,
            ],
        );

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

const WINDOW_METHODS: &[(&str, super::super::NativeFn)] = &[
    ("scrollTo", native_window_scroll_to),
    // CSSOM View "Extensions to the Window Interface": `scroll(x, y)` /
    // `scroll(options)` is defined to run the exact same steps as `scrollTo`,
    // so it shares the native fn (without the alias, `window.scroll(...)` is a
    // `TypeError`).
    ("scroll", native_window_scroll_to),
    ("scrollBy", native_window_scroll_by),
    (
        "postMessage",
        super::pending_tasks::native_window_post_message,
    ),
    (
        "getComputedStyle",
        super::css_style_declaration::native_window_get_computed_style,
    ),
    // Selection API §2: `getSelection()` returns the per-document
    // Selection singleton.  Identical binding on `Document.prototype`
    // (see `vm/host/document.rs`).  Single-doc VM never returns null
    // here; gated to `InvalidStateError` if host is unbound.
    ("getSelection", native_window_get_selection),
    // CSSOM-View §4 "Extensions to the Window Interface": `matchMedia(query)`
    // returns a live `MediaQueryList` (CSSOM-View §4.2).
    ("matchMedia", super::media_query::native_window_match_media),
];

// `pageXOffset` / `pageYOffset` are spec aliases for `scrollX` /
// `scrollY`; they share the same underlying native fn.
//
// The iframe WindowProxy accessors live on `Window.prototype` per
// WHATWG HTML §7.2.2.  Slot ownership per getter:
//   `parent`/`top`/`frameElement`/`length`/`closed` — deferred stubs,
//     `#11-windowproxy-browsing-context` (see comment block above
//     `native_window_get_self` for why/trigger/date).
//   `self`/`frames` — getter bodies already spec-correct (return `this`);
//     only `frames[i]` exotic indexed access is deferred under the same slot.
//   `opener` — deferred stub, `#11-auxiliary-browsing-context-opener`
//     (window.open() scope; see comment block above for why/trigger/date).
const WINDOW_RO_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("innerWidth", native_window_get_inner_width),
    ("innerHeight", native_window_get_inner_height),
    ("scrollX", native_window_get_scroll_x),
    ("scrollY", native_window_get_scroll_y),
    ("pageXOffset", native_window_get_scroll_x),
    ("pageYOffset", native_window_get_scroll_y),
    ("devicePixelRatio", native_window_get_device_pixel_ratio),
    ("self", native_window_get_self),
    ("parent", native_window_get_parent),
    ("top", native_window_get_top),
    ("frames", native_window_get_frames),
    ("frameElement", native_window_get_frame_element),
    ("opener", native_window_get_opener),
    ("length", native_window_get_length),
    ("closed", native_window_get_closed),
];

const WINDOW_RW_ACCESSORS: &[(&str, super::super::NativeFn, super::super::NativeFn)] =
    &[("name", native_window_get_name, native_window_set_name)];

#[cfg(feature = "compat-webapi")]
const WINDOW_STORAGE_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("localStorage", native_window_get_local_storage),
    ("sessionStorage", native_window_get_session_storage),
];

// `window.screen` / `window.visualViewport` (CSSOM-View §4) — no-setter RO
// accessors returning the cached singleton (`[SameObject]`); the
// `localStorage`/`sessionStorage` install form, normalizing the previously
// anomalous writable-global install onto the sibling `[Replaceable]`
// no-setter-RO-accessor family. S5-2.
const WINDOW_PARITY_ACCESSORS: &[(&str, super::super::NativeFn)] = &[
    ("screen", native_window_get_screen),
    ("visualViewport", native_window_get_visual_viewport),
];

/// `window.screen` getter (CSSOM-View §4) — `[SameObject]`: returns the same
/// `Screen` singleton across reads, allocated lazily on the first access via
/// [`crate::vm::VmInner::alloc_or_cached_screen`]. The cache is cleared on
/// `Vm::unbind` (cross-DOM hygiene), so a retained reference cannot alias a
/// prior `EcsDom`'s `ObjectId` space. `Screen` is non-nullable (no §4 null
/// branch — unlike `visualViewport`).
fn native_window_get_screen(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = ctx.vm.alloc_or_cached_screen();
    Ok(JsValue::Object(id))
}

/// `window.visualViewport` getter (CSSOM-View §4) — type `VisualViewport?`
/// (nullable). §4: "If the associated document is fully active, … return the
/// VisualViewport object …; **Otherwise, it must return null**." In elidex's
/// single-document VM the window's associated document is unconditionally fully
/// active (the geometry reads VM-global `ViewportState`, present from VM
/// construction), so the null branch is currently unreachable; it is implemented
/// (not asserted-away) for spec-faithfulness so a future multi-document model
/// wires a genuine check here. `[SameObject]`: returns the cached singleton via
/// [`crate::vm::VmInner::alloc_or_cached_visual_viewport`] (which also seeds the
/// event-producer diff prior at allocation).
fn native_window_get_visual_viewport(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !window_has_fully_active_document(ctx) {
        // §4 attribute-level null branch (distinct from the §12.1 geometry
        // getter → 0 branch). Unreachable in the single-document model today.
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_visual_viewport();
    Ok(JsValue::Object(id))
}

/// Whether the window's associated document is fully active (CSSOM-View §4 /
/// WHATWG HTML "fully active"). The **single** fully-active predicate, shared by
/// the §4 `window.visualViewport → null` branch (above) and the §12.1
/// VisualViewport geometry getters' "not fully active → 0" branch (consumed by
/// `visual_viewport::vv_geometry_read`). In elidex's single-document model this is
/// unconditionally `true` (the `html_dialog_proto.rs` precedent — folded into
/// `#11-browsing-context-state-ecs-components`); the predicate exists so both
/// branches are real code a future multi-document model wires at one site, not
/// removed steps.
pub(super) fn window_has_fully_active_document(_ctx: &NativeContext<'_>) -> bool {
    true
}

/// `window.localStorage` getter (WHATWG HTML §11.2).  `[SameObject]`:
/// returns the same `Storage` wrapper across reads, allocated lazily
/// on the first access via [`crate::vm::VmInner::alloc_or_cached_storage`].
/// `compat-webapi`-gated (A2): the `Storage` glue is `Legacy`.
#[cfg(feature = "compat-webapi")]
fn native_window_get_local_storage(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = ctx.vm.alloc_or_cached_storage(true);
    Ok(JsValue::Object(id))
}

/// `window.sessionStorage` getter — sibling of
/// [`native_window_get_local_storage`] for the per-VM in-memory area.
#[cfg(feature = "compat-webapi")]
fn native_window_get_session_storage(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = ctx.vm.alloc_or_cached_storage(false);
    Ok(JsValue::Object(id))
}

/// `window.getSelection()` (Selection API §2): returns the
/// per-document Selection singleton.  Identical binding wired on
/// `Document.prototype` (`vm/host/document.rs`) — both resolve to the
/// same `[SameObject]` wrapper held in
/// `HostData::selection_instance`.  Lazily materialises the wrapper
/// on first call; subsequent calls return the same `ObjectId`
/// (identity preserved per spec).  In the current single-document
/// VM this never returns null; the spec "fully-active document" gate
/// becomes a real check only once multi-document arrives (D-15 /
/// iframe).
pub(super) fn native_window_get_selection(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "Failed to execute 'getSelection' on 'Window': host environment is not initialised",
        ));
    }
    let id = ctx.vm.alloc_or_cached_selection();
    Ok(JsValue::Object(id))
}
