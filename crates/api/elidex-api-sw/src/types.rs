//! IPC message types between content thread and Service Worker thread.

use crate::registration::SwState;
use crate::security::SwRegisterError;

/// Request data sent to a Service Worker's fetch event.
#[derive(Debug, Clone)]
pub struct SwRequest {
    pub url: url::Url,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Fetch request mode ("navigate", "same-origin", "cors", "no-cors").
    pub mode: String,
    /// Resource destination ("document", "script", "style", "image", etc.).
    pub destination: String,
    /// Subresource integrity hash (e.g., "sha384-...").
    pub integrity: Option<String>,
    /// Redirect mode ("follow", "error", "manual").
    pub redirect: String,
    /// Referrer URL or "about:client".
    pub referrer: String,
    /// Referrer policy.
    pub referrer_policy: String,
    /// Cache mode ("default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached").
    pub cache_mode: String,
    /// Whether the request should persist the connection.
    pub keepalive: bool,
}

/// Response data from a Service Worker's respondWith().
#[derive(Debug, Clone)]
pub struct SwResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub url: url::Url,
}

/// Lifecycle event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Install,
    Activate,
}

/// Client context type (WHATWG SW §4.2 `ClientType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    Window,
    Worker,
    SharedWorker,
}

/// Frame type of a window client (WHATWG SW §4.2 `FrameType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    TopLevel,
    Nested,
    Auxiliary,
    None,
}

/// Page visibility of a window client (W3C Page Visibility, the
/// `VisibilityState` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityState {
    Visible,
    Hidden,
}

/// An engine-independent, `Send` snapshot of one client controlled by the
/// Service Worker (WHATWG SW §4.2 `Client`).
///
/// The engine-bound shell tracks live clients as `ClientState`
/// (`sw_coordinator.rs`); this is the marshalled `Send` twin pushed to the
/// SW thread (in the spawn payload and via [`ContentToSw::ClientList`]) so
/// the SW realm's `clients.matchAll()` / `clients.get()` natives can answer
/// without reaching back into the content/shell process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSnapshot {
    /// Opaque client id (UUID) — the `Client.id` value and the routing key
    /// for `Client.postMessage`.
    pub id: String,
    /// The client's creation URL (`Client.url`).
    pub url: String,
    /// `Client.type`.
    pub client_type: ClientType,
    /// `WindowClient.frameType` (meaningful only for window clients).
    pub frame_type: FrameType,
    /// `WindowClient.visibilityState`.
    pub visibility: VisibilityState,
    /// `WindowClient.focused`.
    pub focused: bool,
}

/// Messages from content thread to Service Worker thread.
#[derive(Debug)]
pub enum ContentToSw {
    /// Deliver a FetchEvent to the SW.
    FetchEvent {
        fetch_id: u64,
        request: Box<SwRequest>,
        /// Client ID (UUID) of the requesting context.
        client_id: String,
        /// For navigation requests: the client ID of the resulting document.
        resulting_client_id: String,
    },
    /// Fire the 'install' ExtendableEvent.
    Install,
    /// Fire the 'activate' ExtendableEvent.
    Activate,
    /// Fire a Background Sync event (WICG).
    SyncEvent { tag: String, last_chance: bool },
    /// Fire a Periodic Background Sync event (WICG).
    PeriodicSyncEvent { tag: String },
    /// Deliver a message from client.postMessage().
    PostMessage {
        data: String,
        origin: String,
        client_id: String,
    },
    /// Deliver a notification click/close event.
    NotificationEvent {
        action: NotificationAction,
        tag: Option<String>,
        notification_data: Option<String>,
    },
    /// Push (or replace) the SW realm's view of the clients it controls
    /// (WHATWG SW §4.1(3) — feeds the `Clients` side-store read by
    /// `clients.matchAll()` / `clients.get()`).  Coordinator-originated
    /// (trusted shell); the SW VM treats it as the authoritative snapshot.
    ClientList { clients: Vec<ClientSnapshot> },
    /// Shut down the SW thread.
    Shutdown,
}

/// Notification event actions.
#[derive(Debug, Clone)]
pub enum NotificationAction {
    Click { action: Option<String> },
    Close,
}

/// Messages from Service Worker thread to content/browser thread.
#[derive(Debug)]
pub enum SwToContent {
    /// SW called respondWith(response) for a fetch event.
    FetchResponse { fetch_id: u64, response: SwResponse },
    /// SW did not call respondWith — fall through to network.
    FetchPassthrough { fetch_id: u64 },
    /// Lifecycle event completed.
    LifecycleComplete {
        event: LifecycleEvent,
        /// `false` if any waitUntil() promise rejected.
        success: bool,
    },
    /// SW called self.skipWaiting().
    SkipWaiting,
    /// SW called self.clients.claim().
    ClientsClaim,
    /// Background Sync event completed.
    SyncComplete { tag: String, success: bool },
    /// Periodic Background Sync event completed.
    PeriodicSyncComplete { tag: String, success: bool },
    /// SW called self.registration.showNotification().
    ShowNotification { title: String, options_json: String },
    /// SW sent a message to a client.
    PostMessage { client_id: String, data: String },
    /// Error in SW script.
    Error {
        message: String,
        filename: String,
        lineno: u32,
        colno: u32,
    },
}

/// A registration's single current worker, marshalled for the window-realm
/// `navigator.serviceWorker` back-channel (WHATWG SW §3.1).
///
/// The shell coordinator holds one [`crate::registration::SwState`] per
/// registration (one worker per scope); this is the `Send` twin the
/// `ServiceWorkerRegistration.installing`/`waiting`/`active` getters and
/// `ServiceWorker.scriptURL`/`state` read from after a back-channel deliver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwWorkerSnapshot {
    /// `ServiceWorker.scriptURL` (the registered SW script URL).
    pub script_url: String,
    /// `ServiceWorker.state` (the worker's lifecycle state).
    pub state: SwState,
}

/// An inbound update from the shell coordinator to a window-realm
/// `navigator.serviceWorker` client (the browser→content direction of the
/// DR-B back-channel, WHATWG SW §3.1/§3.4).
///
/// Engine-independent sibling of [`ContentToSw`]/[`SwToContent`].  The shell
/// `BrowserToContent` SW variants map 1:1 onto this; the engine-bound VM
/// settles `register()`/`unregister()` promises and fires `statechange` /
/// `updatefound` / `controllerchange` / `message` from it
/// (`Vm::deliver_sw_client_update`).
#[derive(Debug, Clone)]
pub enum SwClientUpdate {
    /// A `register()` (or `update()`) job settled.  On success the
    /// registration's current `worker` seeds the client state authoritatively
    /// (so `.installing`/`.waiting`/`.active` have a write-path at resolve);
    /// on failure the typed `error` rejects every waiter.
    Registered {
        /// Canonical scope URL the job registered.
        scope: url::Url,
        /// Whether the registration succeeded.
        success: bool,
        /// The rejection reason (mapped 1:1 to a `DOMException`) when
        /// `success` is `false`.
        error: Option<SwRegisterError>,
        /// The registration's current worker on success.
        worker: Option<SwWorkerSnapshot>,
    },
    /// A worker's lifecycle state advanced (drives `.state`, `onstatechange`,
    /// and — for a freshly installing worker — `onupdatefound`).
    StateChanged {
        /// Scope of the registration whose worker changed.
        scope: url::Url,
        /// The worker's new state.
        state: SwState,
    },
    /// The page's controller changed (drives `controller` + `oncontrollerchange`).
    /// `None` clears the controller.
    ControllerSet {
        /// Scope of the new controlling registration (or `None`).
        scope: Option<url::Url>,
    },
    /// A worker `postMessage`d this client (drives `navigator.serviceWorker`
    /// `onmessage`, buffered until `startMessages()`/first `onmessage`).
    Message {
        /// Serialized message payload.
        data: String,
        /// Scope of the sending registration.
        source_scope: url::Url,
    },
    /// A registration was removed (`unregister()` or replacement) — drops it
    /// from the client registry and settles pending `unregister()` waiters.
    Unregistered {
        /// Scope of the removed registration.
        scope: url::Url,
        /// Whether a registration was actually removed.
        success: bool,
    },
}

/// An outbound request from a window-realm `navigator.serviceWorker` client to
/// the shell coordinator (the content→browser direction of the DR-B
/// back-channel, WHATWG SW §3.2/§3.4).
///
/// Staged on the VM by the container/registration/worker natives and drained
/// by the content event loop, which maps each onto a `ContentToBrowser` IPC
/// message.  The engine-independent twin of boa's `bridge.queue_sw_register`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwClientRequest {
    /// `ServiceWorkerContainer.register(scriptURL, { scope })` — the URLs are
    /// already resolved against the document base URL (canonical).
    Register {
        /// Resolved script URL.
        script_url: String,
        /// Resolved scope URL.
        scope: String,
    },
    /// `ServiceWorkerRegistration.update()` — re-fetch + soft-update the SW.
    Update {
        /// Scope of the registration to update.
        scope: String,
    },
    /// `ServiceWorkerRegistration.unregister()`.
    Unregister {
        /// Scope of the registration to remove.
        scope: String,
    },
    /// `ServiceWorker.postMessage(message)` — deliver to the worker at `scope`.
    PostMessage {
        /// Scope of the target worker.
        scope: String,
        /// Serialized message payload.
        data: String,
    },
}
