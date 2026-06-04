//! IPC message types between content thread and Service Worker thread.

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
