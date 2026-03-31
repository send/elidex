//! Iframe types: IPC messages, handle types, metadata, and load context.

use std::thread::JoinHandle;

use elidex_ecs::{Entity, ScrollState};
use elidex_navigation::NavigationController;
use elidex_plugin::SecurityOrigin;
use elidex_render::DisplayList;

use crate::ipc::LocalChannel;
use crate::PipelineResult;

// ---------------------------------------------------------------------------
// IPC message types for cross-origin iframe communication
// ---------------------------------------------------------------------------

/// Messages sent from the parent content thread to a cross-origin iframe thread.
#[derive(Debug)]
#[allow(dead_code)] // Variants constructed by parent thread for IPC dispatch.
pub enum BrowserToIframe {
    /// Navigate the iframe to a new URL.
    Navigate(url::Url),
    /// Mouse click at iframe-local coordinates.
    MouseClick(crate::ipc::MouseClickEvent),
    /// Key event (keydown or keyup).
    KeyEvent {
        /// DOM event type ("keydown" or "keyup").
        event_type: String,
        /// DOM key value.
        key: String,
        /// DOM code value.
        code: String,
        /// Whether this is a repeat event.
        repeat: bool,
        /// Modifier keys.
        mods: crate::ipc::ModifierState,
    },
    /// Viewport size changed.
    SetViewport {
        /// New width in logical pixels.
        width: f32,
        /// New height in logical pixels.
        height: f32,
    },
    /// Cross-document postMessage (WHATWG HTML §9.4.3).
    PostMessage {
        /// JSON-serialized message data.
        data: String,
        /// Sender's serialized origin.
        origin: String,
    },
    /// Shut down the iframe thread.
    Shutdown,
}

/// Messages sent from a cross-origin iframe thread to the parent content thread.
#[derive(Debug)]
pub enum IframeToBrowser {
    /// A new display list is ready for compositing into the parent.
    DisplayListReady(DisplayList),
    /// Cross-document postMessage from iframe to parent (WHATWG HTML §9.4.3).
    PostMessage {
        /// JSON-serialized message data.
        data: String,
        /// Sender's serialized origin.
        origin: String,
    },
}

// ---------------------------------------------------------------------------
// Iframe handle types
// ---------------------------------------------------------------------------

/// Same-origin iframe: runs in the parent content thread with direct access.
#[allow(dead_code)] // Fields read by iframe event routing and lifecycle management.
pub struct InProcessIframe {
    /// Full rendering pipeline (DOM, JS, styles, layout, display list).
    pub pipeline: PipelineResult,
    /// Independent navigation history for this iframe.
    pub nav_controller: NavigationController,
    /// Currently focused entity within this iframe's document.
    pub focus_target: Option<Entity>,
    /// Independent scroll state for this iframe's viewport.
    pub scroll_state: ScrollState,
    /// Whether this iframe needs a re-render on the next frame.
    pub needs_render: bool,
    /// Cached `Arc<DisplayList>` to avoid re-cloning on every parent render.
    /// Updated only when `needs_render` is true and re-render completes.
    pub cached_display_list: Option<std::sync::Arc<DisplayList>>,
}

/// Cross-origin iframe: runs in a separate thread, communicates via IPC.
pub struct OutOfProcessIframe {
    /// IPC channel to the iframe thread.
    pub channel: LocalChannel<BrowserToIframe, IframeToBrowser>,
    /// Latest display list received from the iframe thread.
    ///
    /// Updated atomically when `IframeToBrowser::DisplayListReady` is received.
    /// The parent thread always renders the most recent snapshot; stale frames
    /// are acceptable and will be replaced on the next update.
    pub display_list: DisplayList,
    /// Handle to the iframe's content thread.
    pub thread: Option<JoinHandle<()>>,
}

/// Iframe handle: dispatches to in-process or out-of-process implementation
/// based on the origin relationship with the parent document.
pub enum IframeHandle {
    /// Same-origin iframe: parent thread owns the `PipelineResult` directly.
    /// Boxed to avoid large size difference between variants (`PipelineResult` is ~1.7KB).
    InProcess(Box<InProcessIframe>),
    /// Cross-origin iframe: separate thread with IPC communication.
    OutOfProcess(OutOfProcessIframe),
}

/// An iframe entry stored in the `IframeRegistry`.
pub struct IframeEntry {
    /// Handle to the iframe's pipeline (in-process or out-of-process).
    pub handle: IframeHandle,
}

/// A postMessage received from an out-of-process iframe.
#[allow(dead_code)] // Fields read by message dispatch in content event loop.
pub struct OopPostMessage {
    /// The `<iframe>` entity that sent the message.
    pub entity: Entity,
    /// JSON-serialized message data.
    pub data: String,
    /// Sender's serialized origin.
    pub origin: String,
}

// ---------------------------------------------------------------------------
// IframeLoadContext
// ---------------------------------------------------------------------------

/// Context from the parent document needed to load an iframe.
pub struct IframeLoadContext<'a> {
    /// Security origin of the parent document.
    pub parent_origin: &'a SecurityOrigin,
    /// URL of the parent document (for relative URL resolution).
    pub parent_url: Option<&'a url::Url>,
    /// Shared font database.
    pub font_db: &'a std::sync::Arc<elidex_text::FontDatabase>,
    /// Network handle for communicating with the Network Process broker.
    pub network_handle: &'a std::rc::Rc<elidex_net::broker::NetworkHandle>,
    /// Parent's cookie jar (for same-origin iframe `document.cookie`).
    pub cookie_jar: Option<std::sync::Arc<elidex_net::CookieJar>>,
    /// Iframe nesting depth (for `MAX_IFRAME_DEPTH` enforcement).
    pub depth: usize,
    /// Shared CSS property registry.
    pub registry: &'a std::sync::Arc<elidex_plugin::CssPropertyRegistry>,
}
