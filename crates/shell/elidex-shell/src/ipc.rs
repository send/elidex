//! Inter-thread communication types for browser ↔ content thread messaging.
//!
//! Defines the message protocol and a bidirectional channel abstraction
//! used between the browser thread (window, events, rendering) and the
//! content thread (DOM, JS, style, layout).

use elidex_plugin::{Point, Vector};
use elidex_render::DisplayList;

pub use elidex_plugin::{channel_pair, LocalChannel};

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
    /// Parsed Web App Manifest from browser thread.
    ManifestParsed(Box<elidex_api_sw::WebAppManifest>),
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
        /// The origin to estimate.
        origin: String,
    },
    /// Request persistent storage for this origin (W3C Storage Standard §4).
    StoragePersist {
        /// The origin requesting persistence.
        origin: String,
    },
    /// Query whether this origin has persistent storage.
    StoragePersisted {
        /// The origin to query.
        origin: String,
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
