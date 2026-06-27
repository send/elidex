//! Inter-thread communication types for browser ↔ content thread messaging.
//!
//! Defines the message protocol and a bidirectional channel abstraction
//! used between the browser thread (window, events, rendering) and the
//! content thread (DOM, JS, style, layout).

use std::sync::{Arc, Mutex, PoisonError};

use elidex_css::media::ColorScheme;
use elidex_plugin::{Point, Size, Vector};
use elidex_render::DisplayList;

pub use elidex_plugin::{channel_pair, LocalChannel};

/// Browser-published per-window **device facts**: the device-pixel ratio
/// (`window.devicePixelRatio` — CSSOM View §4 — / `@media (resolution)` — Media
/// Queries L5 §5.1) and the `prefers-color-scheme` preference (Media Queries L5
/// §12.5). Orthogonal to the viewport *size* — a
/// pure-scale change the OS absorbs (physical size changes, CSS-logical size
/// preserved) shifts `dppx` without a `size_logical` generation — so they ride the
/// [`ViewportCell`] beside `{ size, seq }` but are change-detected independently of
/// `seq` (C3 D3). Content-agnostic CSS px / preference values, never winit types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeviceFacts {
    /// Device pixel ratio (`window.scale_factor()`), the `@media (resolution)` dppx.
    pub dppx: f32,
    /// `prefers-color-scheme` preference (from the window theme; `Light` on the
    /// platforms winit reports no theme — X11/Wayland).
    pub color_scheme: ColorScheme,
}

impl Default for DeviceFacts {
    /// The pre-publish baseline: 1× scale, `Light` — matching the boa bridge's
    /// construction defaults (`device_pixel_ratio: 1.0`, `ColorScheme::Light`), so a
    /// tab born before the first real publish reads the same facts the bridge already
    /// holds (publishing the real facts is then a single change-detected delta).
    fn default() -> Self {
        Self {
            dppx: 1.0,
            color_scheme: ColorScheme::Light,
        }
    }
}

/// What [`ViewportCell::publish_device_state`] changed this publish, so the caller
/// can fan out exactly the affected IPC + repaint work: `size` → `SetViewport`
/// (+seq bump, the C2 input-drop discipline); device facts → `SetDeviceFacts`
/// (no seq — facts are seq-orthogonal, C3 D3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceDelta {
    /// `size_logical` changed → a new viewport generation (seq bumped).
    pub size_changed: bool,
    /// `dppx` and/or `color_scheme` changed → device facts to re-broadcast.
    pub facts_changed: bool,
}

/// The atomic `(size, seq, facts)` triple a content thread reads from the
/// [`ViewportCell`] at build time (one lock-copy-release). Replaces the former
/// `(Size, u64)` tuple so a spawned tab is born with the device facts too (C3
/// construction-input), not only the size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportSnapshot {
    /// Content-area size in CSS logical px.
    pub size: Size,
    /// Monotonic `size_logical`-generation seq (see [`ViewportCell`]).
    pub seq: u64,
    /// Latest device facts (dppx / color-scheme).
    pub facts: DeviceFacts,
    /// Monotonic **device-facts** generation, independent of `seq` (a pure-scale
    /// change bumps `facts_seq` but not `seq`, D3). It is the high-water mark a
    /// content build records so a queued *older* `SetDeviceFacts` — already folded
    /// into the `facts` this build read from the cell — is dropped instead of
    /// replaying a stale color-scheme/dppx backward (the facts analog of the `seq`
    /// staleness guard `SetViewport` carries).
    pub facts_seq: u64,
}

/// Browser-published latest content-area viewport (CSS logical px) that content
/// threads read as a **pull source** at build time, plus a monotonic `seq`
/// correlating each build's cell-read with the FIFO [`BrowserToContent::SetViewport`]
/// message stream.
///
/// **Why a cell + seq** (see `docs/plans/2026-06-viewport-latest-value-cell-plan.md`):
/// a content thread that blocks on `load_document` before its first build can have
/// the window resized *during* that load. A spawn-time `Size` snapshot would then
/// build (cascade `@media`, run initial `innerWidth`/`matchMedia` scripts, lay out)
/// at the stale pre-resize size — and the script having already run, no later resize
/// re-runs it. Reading this cell *after* the load returns makes the build observe the
/// **latest** size by construction (no reconcile-after-the-fact).
///
/// The `seq` reconciles that cell-read build with the in-flight `SetViewport` stream:
/// the build records the read seq as its high-water mark, and the consumer drops any
/// `SetViewport` whose `seq` is `≤` that mark (already consumed by the build or a
/// prior apply), so the cell does not introduce a backward "flash" to a replayed
/// intermediate. Any genuinely newer resize carries `seq >` the mark and applies — no
/// lost update.
///
/// **Single writer** (the browser main thread, via [`Self::publish_device_state`]); **many readers**
/// (each content thread, via [`Self::read`]). One per window — all tabs share the
/// window content area, so they share one `Arc<ViewportCell>`.
#[derive(Debug)]
pub struct ViewportCell {
    inner: Mutex<ViewportCellValue>,
}

#[derive(Clone, Copy, Debug)]
struct ViewportCellValue {
    size: Size,
    seq: u64,
    facts: DeviceFacts,
    facts_seq: u64,
}

impl ViewportCell {
    /// Construct a cell seeded with `size` at **seq 0** and default
    /// [`DeviceFacts`] — the pre-publish baseline. The first
    /// [`publish_device_state`](Self::publish_device_state) of a *different* size
    /// bumps to seq 1, so a content thread that reads the cell before any
    /// size-changing publish sees seq 0 (and any later real `SetViewport` carries
    /// `seq ≥ 1 >` 0, hence applies). Device facts seed at the bridge's own defaults
    /// (`DeviceFacts::default`) at `facts_seq 0` so the seed → real-facts publish is
    /// one clean delta on its own generation (the first real-facts publish bumps
    /// `facts_seq` 0 → 1, exactly as the size publish bumps `seq`).
    #[must_use]
    pub fn new(size: Size) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(ViewportCellValue {
                size,
                seq: 0,
                facts: DeviceFacts::default(),
                facts_seq: 0,
            }),
        })
    }

    /// Browser writer: store the latest content-area `size` + device `facts`,
    /// reporting which changed via [`DeviceDelta`] (C3 D3).
    ///
    /// **Size** bumps the monotonic seq **iff** it differs (the C2 discipline,
    /// verbatim): `seq` identifies `size_logical` *generations* — the unit
    /// `placement_seq` / `applied_viewport_seq` reconcile against
    /// ([`MouseClickEvent::placement_seq`], `content/event_loop.rs`
    /// `input_placement_stale`). Bumping it on an **unchanged** size would manufacture
    /// a phantom generation that spuriously supersedes legitimately-queued input
    /// mapped against the still-current layout — a pure DPI/scale `Resized` carries
    /// the same `size_logical` (CSS px is scale-invariant). **Device facts** are
    /// seq-**orthogonal**: a pure-scale change the OS absorbs shifts `dppx` while
    /// `size_logical` is preserved, so it must ship `SetDeviceFacts` **without**
    /// bumping `seq` (else the phantom input-drop returns). Instead facts carry their
    /// **own** monotonic `facts_seq`, bumped here iff they differ — the high-water mark
    /// a content build records (`ViewportSnapshot::facts_seq`) so a queued *older*
    /// `SetDeviceFacts` already folded into the build's cell-read is dropped, the facts
    /// analog of the `seq` staleness guard (`content/event_loop.rs`). The caller gates
    /// `broadcast_viewport` on `size_changed` and `broadcast_device_facts` on
    /// `facts_changed`, so an idempotent per-frame republish (redraw cadence) emits
    /// nothing.
    ///
    /// Poison-recovers (`into_inner`): the guarded value is three plain fields written
    /// as one assignment, so a reader panicking mid-critical-section leaves no broken
    /// invariant and the browser must not panic in turn.
    #[must_use = "the caller must gate its broadcasts on which facts changed"]
    pub fn publish_device_state(&self, size: Size, facts: DeviceFacts) -> DeviceDelta {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let size_changed = guard.size != size;
        if size_changed {
            guard.size = size;
            guard.seq += 1;
        }
        let facts_changed = guard.facts != facts;
        if facts_changed {
            guard.facts = facts;
            guard.facts_seq += 1;
        }
        DeviceDelta {
            size_changed,
            facts_changed,
        }
    }

    /// Content reader: the latest [`ViewportSnapshot`] atomically (a
    /// lock-copy-release; no lock is held across the caller's subsequent build /
    /// `load_document`).
    #[must_use]
    pub fn read(&self) -> ViewportSnapshot {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        ViewportSnapshot {
            size: guard.size,
            seq: guard.seq,
            facts: guard.facts,
            facts_seq: guard.facts_seq,
        }
    }
}

/// Winit-independent modifier key state.
#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // Matches DOM UIEvent modifier key set (alt/ctrl/meta/shift).
pub struct ModifierState {
    /// Alt/Option key.
    pub alt: bool,
    /// Control key.
    pub ctrl: bool,
    /// Meta/Command/Windows key.
    pub meta: bool,
    /// Shift key.
    pub shift: bool,
}

/// Mouse click event data.
///
/// Bundles content-relative coordinates, viewport coordinates, button number,
/// and modifier key state for a mouse click.
#[derive(Clone, Debug)]
pub struct MouseClickEvent {
    /// Position in content area (for hit testing).
    pub point: Point,
    /// Position in viewport (for DOM event clientX/clientY).
    pub client_point: Point<f64>,
    /// Mouse button number (DOM spec: 0=primary, 1=aux, 2=secondary).
    pub button: u8,
    /// Modifier keys held during click.
    pub mods: ModifierState,
    /// The [`ViewportCell`] seq the `point`/`client_point` were mapped against
    /// (browser-stamped at send time). See [`BrowserToContent::MouseMove`] for the
    /// drop contract.
    pub placement_seq: u64,
}

/// Data for `BrowserToContent::SwRegistered` (boxed to reduce enum size).
#[derive(Debug)]
pub struct SwRegisteredData {
    /// Scope URL of the registration.
    pub scope: url::Url,
    /// Whether registration succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Messages sent from the browser thread to the content thread.
#[derive(Debug)]
pub enum BrowserToContent {
    /// Navigate to a URL.
    Navigate(url::Url),
    /// Mouse button pressed at content-relative coordinates.
    MouseClick(MouseClickEvent),
    /// Mouse button released.
    ///
    /// Per UI Events spec, `:active` pseudo-class applies from mousedown
    /// to mouseup. This message signals the content thread to clear ACTIVE.
    MouseRelease {
        /// Mouse button number.
        button: u8,
    },
    /// Mouse moved to content-relative coordinates.
    MouseMove {
        /// Position in content area (for hit testing).
        point: Point,
        /// Position in viewport (for DOM event clientX/clientY).
        client_point: Point<f64>,
        /// The [`ViewportCell`] seq the coordinates were mapped against
        /// (browser-stamped at send time). The content thread **drops** this input
        /// when `placement_seq < applied_viewport_seq` — it was hit-mapped against a
        /// viewport the seq guard has since dropped (a resize that landed during a
        /// blocking load and was superseded by the build), so hit-testing the stale
        /// coordinates against the current layout would target the wrong element.
        /// Dropping is the spec-aligned choice (re-mapping coordinates onto a
        /// different layout is ill-defined). Coordinate-bearing input only
        /// (`MouseClick`/`MouseMove`/`MouseWheel`); key/release/cursor-leave target
        /// the focused/active element and need no placement seq. See plan-memo §10.
        placement_seq: u64,
    },
    /// Cursor left the content area.
    CursorLeft,
    /// Key pressed.
    KeyDown {
        /// DOM key value.
        key: String,
        /// DOM code value.
        code: String,
        /// Whether this is a repeat event.
        repeat: bool,
        /// Modifier keys.
        mods: ModifierState,
    },
    /// Key released.
    KeyUp {
        /// DOM key value.
        key: String,
        /// DOM code value.
        code: String,
        /// Whether this is a repeat event (always false for keyup).
        repeat: bool,
        /// Modifier keys.
        mods: ModifierState,
    },
    /// Viewport size changed — and carries the settled device facts too, so a frame
    /// that changes **both** size and facts (a DPI move where the logical size also
    /// changes) delivers them in **one** message → one content re-eval reflecting both,
    /// never an inconsistent new-size + old-facts intermediate that would fire a spurious
    /// `change` for a query like `(min-width: 800px) and (prefers-color-scheme: dark)`
    /// (Codex R2). When only facts change (size unchanged) the producer sends
    /// [`Self::SetDeviceFacts`] instead; the two are mutually exclusive per frame, so the
    /// facts never travel both. The content applies size (the `seq` guard) and facts (the
    /// `facts_seq` guard) independently from this one message.
    SetViewport {
        /// New width in logical pixels.
        width: f32,
        /// New height in logical pixels.
        height: f32,
        /// Monotonic [`ViewportCell`] sequence this delivery corresponds to. The
        /// content thread drops a delivery whose `seq` is `≤` its build-time
        /// high-water mark (already consumed by the build or a prior apply), so a
        /// resize that landed during a blocking load — already reflected in the
        /// cell the build read — does not re-fire as a backward flash.
        seq: u64,
        /// The settled `prefers-color-scheme` carried alongside the size (the cell's
        /// atomic snapshot), applied via the same `facts_seq` guard as [`Self::SetDeviceFacts`].
        color_scheme: ColorScheme,
        /// The settled device-pixel ratio (dppx) carried alongside the size.
        dppx: f32,
        /// The device-facts generation for the carried facts (independent of `seq`,
        /// D3); dropped by the content when `≤` its facts high-water mark.
        facts_seq: u64,
    },
    /// Per-window device facts changed (C3): the device-pixel ratio
    /// (`window.devicePixelRatio` / `@media (resolution)`) and/or the
    /// `prefers-color-scheme` preference. **No size `seq`** — these are orthogonal to
    /// the `size_logical` generation ([`ViewportCell::publish_device_state`], D3), so a
    /// pure-scale change the OS absorbs delivers facts without dropping queued input.
    /// Carries its **own** `facts_seq` (the device-facts generation, independent of the
    /// size `seq`): the content thread drops a delivery whose `facts_seq` is `≤` its
    /// build-time facts high-water mark — a queued older fact already folded into the
    /// facts the build read from the cell — so a navigation racing rapid DPI/theme
    /// changes does not replay an obsolete color-scheme/dppx backward (the facts analog
    /// of `SetViewport`'s `seq`). One unified variant (not one per fact, D5): a single
    /// content re-eval + repaint when a `ScaleFactorChanged` co-occurs with a theme
    /// toggle. A future `prefers-reduced-motion` producer extends this variant rather
    /// than adding a new one.
    SetDeviceFacts {
        /// `prefers-color-scheme` preference.
        color_scheme: ColorScheme,
        /// Device pixel ratio (dppx).
        dppx: f32,
        /// Monotonic device-facts generation (independent of the size `seq`) this
        /// delivery corresponds to. Dropped when `≤` the content thread's build-time
        /// facts high-water mark.
        facts_seq: u64,
    },
    /// Navigate back in history.
    GoBack,
    /// Navigate forward in history.
    GoForward,
    /// Reload the current page.
    Reload,
    /// Mouse wheel scrolled in the content area.
    MouseWheel {
        /// Scroll delta in CSS pixels (positive = scroll right/down).
        delta: Vector<f64>,
        /// Content-relative coordinates for scroll target hit testing.
        point: Point,
        /// The [`ViewportCell`] seq the `point` was mapped against (browser-stamped
        /// at send time). See [`BrowserToContent::MouseMove`] for the drop contract.
        placement_seq: u64,
    },
    /// IME event.
    Ime {
        /// The IME event kind.
        kind: ImeKind,
    },
    /// A `localStorage` storage event from another tab (WHATWG HTML §11.2.1).
    ///
    /// The browser thread broadcasts this to all same-origin tabs except
    /// the one that originated the change.
    StorageEvent {
        /// The key that changed (`None` for `clear()`).
        key: Option<String>,
        /// The old value (`None` if the key was newly set or cleared).
        old_value: Option<String>,
        /// The new value (`None` if the key was removed or cleared).
        new_value: Option<String>,
        /// The URL of the document that triggered the change.
        url: String,
    },
    /// Tab visibility changed (WHATWG Page Visibility §4.1).
    ///
    /// Sent when the window is occluded/unoccluded or the tab switches.
    VisibilityChanged {
        /// `true` when the tab becomes visible, `false` when hidden.
        visible: bool,
    },
    /// Another tab is requesting an `IndexedDB` version change (W3C `IndexedDB` §2.4).
    ///
    /// This tab should fire `versionchange` events on all open connections
    /// to the named database and close them.
    IdbVersionChange {
        /// Correlation ID for matching responses to requests.
        request_id: u64,
        /// Database name.
        db_name: String,
        /// Current version of the database.
        old_version: u64,
        /// Requested new version (`None` for `deleteDatabase`).
        new_version: Option<u64>,
    },
    /// All other tabs have closed their connections — the upgrade may proceed.
    IdbUpgradeReady {
        /// Correlation ID matching the original request.
        request_id: u64,
        /// Database name.
        db_name: String,
    },
    /// Some tabs still have open connections after `versionchange` — fire `blocked`.
    IdbBlocked {
        /// Correlation ID matching the original request.
        request_id: u64,
        /// Database name.
        db_name: String,
        /// Current version of the database.
        old_version: u64,
        /// Requested new version (`None` for `deleteDatabase`).
        new_version: Option<u64>,
    },
    /// Response to `StorageEstimate` request.
    StorageEstimateResult {
        /// Bytes used by this origin.
        usage: u64,
        /// Quota available for this origin.
        quota: u64,
    },
    /// Response to `StoragePersist` request.
    StoragePersistResult {
        /// Whether persistent storage was granted.
        granted: bool,
    },
    /// Response to `StoragePersisted` request.
    StoragePersistedResult {
        /// Whether this origin has persistent storage.
        persisted: bool,
    },
    /// Service Worker registration result from browser thread.
    SwRegistered(Box<SwRegisteredData>),
    /// Service Worker controller set for this content's origin.
    SwControllerSet {
        /// Scope URL of the controlling SW.
        scope: url::Url,
    },
    /// A Service Worker's lifecycle state advanced (WHATWG SW §3.1) — drives
    /// `ServiceWorker.state` / `onstatechange` / `onupdatefound` on the
    /// window-realm `navigator.serviceWorker` client.
    SwStateChanged {
        /// Scope URL of the registration whose worker changed.
        scope: url::Url,
        /// The worker's new state.
        state: elidex_api_sw::SwState,
    },
    /// Parsed Web App Manifest from browser thread.
    ManifestParsed(Box<elidex_api_sw::WebAppManifest>),
    /// Service Worker FetchEvent response from browser thread.
    SwFetchResponse {
        /// Fetch request ID (matches `ContentToBrowser::SwFetchRequest`).
        fetch_id: u64,
        /// SW response, or `None` for passthrough (no respondWith called).
        response: Option<Box<elidex_api_sw::SwResponse>>,
    },
    /// Shut down the content thread.
    Shutdown,
}

/// IME event kinds.
#[derive(Clone, Debug)]
pub enum ImeKind {
    /// IME composition started or text updated.
    Preedit(String),
    /// IME composition committed.
    Commit(String),
    /// IME enabled.
    Enabled,
    /// IME disabled.
    Disabled,
}

/// Messages sent from the content thread to the browser thread.
#[derive(Debug)]
pub enum ContentToBrowser {
    /// A new display list is ready for rendering.
    DisplayListReady(DisplayList),
    /// The page title changed.
    TitleChanged(String),
    /// Navigation state changed (for chrome back/forward button states).
    NavigationState {
        /// Whether back navigation is available.
        can_go_back: bool,
        /// Whether forward navigation is available.
        can_go_forward: bool,
    },
    /// The current URL changed (for chrome address bar).
    UrlChanged(url::Url),
    /// A navigation request failed.
    NavigationFailed {
        /// The URL that failed to load.
        url: url::Url,
        /// Human-readable error description.
        error: String,
    },
    /// Request to open a URL in a new tab (`window.open` with `_blank` target).
    OpenNewTab(url::Url),
    /// Request the browser thread to focus the window (from `window.focus()`).
    FocusWindow,
    /// `IndexedDB` open/delete is requesting a version change (W3C `IndexedDB` §2.4).
    ///
    /// Browser thread broadcasts `IdbVersionChange` to other same-origin tabs
    /// and currently sends `IdbUpgradeReady` immediately (TODO(M4-10): wait for
    /// `IdbConnectionsClosed` responses or timeout before sending).
    IdbVersionChangeRequest {
        /// Unique request ID for correlating responses across tabs.
        request_id: u64,
        /// The origin that owns the database.
        origin: String,
        /// Database name.
        db_name: String,
        /// Current version.
        old_version: u64,
        /// Requested new version (`None` for `deleteDatabase`).
        new_version: Option<u64>,
    },
    /// This tab has closed all `IndexedDB` connections to the named database
    /// (response to `BrowserToContent::IdbVersionChange`).
    IdbConnectionsClosed {
        /// Correlation ID matching the versionchange request.
        request_id: u64,
        /// Database name.
        db_name: String,
    },
    /// Request storage usage estimate for this origin (W3C Storage Standard §4).
    StorageEstimate {
        /// The page URL (origin derived via `OriginKey::from_url`).
        origin_url: url::Url,
    },
    /// Request persistent storage for this origin (W3C Storage Standard §4).
    StoragePersist {
        /// The page URL (origin derived via `OriginKey::from_url`).
        origin_url: url::Url,
    },
    /// Query whether this origin has persistent storage.
    StoragePersisted {
        /// The page URL (origin derived via `OriginKey::from_url`).
        origin_url: url::Url,
    },
    /// A `localStorage` value was changed (WHATWG HTML §11.2.1).
    ///
    /// Sent to the browser thread so it can broadcast `StorageEvent` to other
    /// same-origin tabs.
    StorageChanged {
        /// The origin that owns the storage area.
        origin: String,
        /// The key that changed (`None` for `clear()`).
        key: Option<String>,
        /// The old value (`None` if the key was newly set or cleared).
        old_value: Option<String>,
        /// The new value (`None` if the key was removed or cleared).
        new_value: Option<String>,
        /// The URL of the document that triggered the change.
        url: String,
    },
    /// Request Service Worker registration.
    SwRegister {
        /// SW script URL.
        script_url: url::Url,
        /// Registration scope.
        scope: url::Url,
        /// Origin of the registering page.
        origin: String,
        /// URL of the registering page (for security validation).
        page_url: url::Url,
    },
    /// A `<link rel="manifest">` was discovered during page load.
    ManifestDiscovered {
        /// URL of the manifest file.
        url: url::Url,
    },
    /// Request SW FetchEvent interception for a navigation.
    SwFetchRequest {
        /// Unique fetch request ID.
        fetch_id: u64,
        /// The request to intercept.
        request: Box<elidex_api_sw::SwRequest>,
        /// Client ID of the requesting context.
        client_id: String,
        /// Client ID of the resulting document (for navigation requests).
        resulting_client_id: String,
    },
}

/// Storage change data for cross-tab broadcast.
///
/// Extracted from `ContentToBrowser::StorageChanged` for buffering during drain.
#[derive(Clone, Debug)]
pub struct StorageChangedMsg {
    /// The origin that owns the storage area.
    pub origin: String,
    /// The key that changed.
    pub key: Option<String>,
    /// The old value.
    pub old_value: Option<String>,
    /// The new value.
    pub new_value: Option<String>,
    /// The URL of the document that triggered the change.
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_state_default() {
        let m = ModifierState::default();
        assert!(!m.alt);
        assert!(!m.ctrl);
        assert!(!m.meta);
        assert!(!m.shift);
    }
}
