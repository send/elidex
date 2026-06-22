//! Inter-thread communication types for browser ↔ content thread messaging.
//!
//! Defines the message protocol and a bidirectional channel abstraction
//! used between the browser thread (window, events, rendering) and the
//! content thread (DOM, JS, style, layout).

use std::sync::{Arc, Mutex, PoisonError};

use elidex_plugin::{Point, Size, Vector};
use elidex_render::DisplayList;

pub use elidex_plugin::{channel_pair, LocalChannel};

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
/// **Single writer** (the browser main thread, via [`Self::publish`]); **many readers**
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
}

impl ViewportCell {
    /// Construct a cell seeded with `size` at **seq 0** — the pre-publish baseline.
    /// The first [`publish`](Self::publish) bumps to seq 1, so a content thread that
    /// reads the cell before any real publish sees seq 0 (and any later real
    /// `SetViewport` carries `seq ≥ 1 >` 0, hence applies).
    #[must_use]
    pub fn new(size: Size) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(ViewportCellValue { size, seq: 0 }),
        })
    }

    /// Browser writer: store a new content-area `size` and bump the monotonic seq.
    ///
    /// Poison-recovers (`into_inner`): the guarded value is two plain fields written
    /// as one assignment, so a reader panicking mid-critical-section leaves no broken
    /// invariant and the browser must not panic in turn.
    pub fn publish(&self, size: Size) {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard.size = size;
        guard.seq += 1;
    }

    /// Content reader: the latest `(size, seq)` pair atomically (a lock-copy-release;
    /// no lock is held across the caller's subsequent build / `load_document`).
    #[must_use]
    pub fn read(&self) -> (Size, u64) {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        (guard.size, guard.seq)
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
    /// Viewport size changed.
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
