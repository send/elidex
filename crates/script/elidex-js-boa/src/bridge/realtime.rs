//! WebSocket / SSE callback registry for the JS bridge.
//!
//! Manages JS callback state for active WebSocket and `EventSource` connections.
//! Network I/O is handled by the Network Process (via `NetworkHandle` in
//! `elidex-net/broker.rs`); this module only stores the JS-side state
//! (event handlers, ready state, buffered amount, etc.).
//!
//! Connection lifecycle:
//! 1. JS constructor calls `register_ws_callbacks()` / `register_sse_callbacks()`
//! 2. Network open/send/close goes through `NetworkHandle` (not this module)
//! 3. Events arrive via `NetworkHandle::drain_events()`, dispatched via callbacks here
//! 4. On close/disconnect, callbacks are removed via `remove_ws()` / `remove_sse()`

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use boa_engine::JsObject;

use elidex_net::sse::SseEvent;
use elidex_net::ws::WsEvent;

/// Return type for splitting drained `NetworkToRenderer` events into WS/SSE.
pub(crate) type RealtimeEvents = (Vec<(u64, WsEvent)>, Vec<(u64, SseEvent)>);

/// Maximum concurrent WebSocket + SSE connections per document.
const MAX_REALTIME_CONNECTIONS: usize = 256;

/// WebSocket/SSE callback state, stored in `HostBridgeInner`.
///
/// This struct only manages JS callbacks — no network handles or I/O threads.
/// Network operations are mediated through `NetworkHandle` in `HostBridge`.
#[derive(Default)]
pub(crate) struct RealtimeState {
    /// Active WebSocket callbacks, keyed by connection ID.
    ws_callbacks: HashMap<u64, WsCallbacks>,
    /// Active SSE callbacks, keyed by connection ID.
    sse_callbacks: HashMap<u64, SseCallbacks>,
    /// Next connection ID (shared counter for WS and SSE).
    next_id: u64,
}

/// WebSocket JS callback state.
pub(crate) struct WsCallbacks {
    /// `onopen` handler.
    pub onopen: Option<JsObject>,
    /// `onmessage` handler.
    pub onmessage: Option<JsObject>,
    /// `onerror` handler.
    pub onerror: Option<JsObject>,
    /// `onclose` handler.
    pub onclose: Option<JsObject>,
    /// The JS `WebSocket` object (for `addEventListener` dispatch).
    pub js_object: JsObject,
    /// Connection URL (for `WebSocket.url` property).
    #[allow(dead_code)]
    pub url: String,
    /// URL origin (for `MessageEvent.origin`).
    pub origin: String,
    /// Ready state (CONNECTING=0, OPEN=1, CLOSING=2, CLOSED=3).
    pub ready_state: Cell<u16>,
    /// Negotiated sub-protocol (updated on `Connected`).
    pub protocol: RefCell<String>,
    /// Negotiated extensions (updated on `Connected`).
    pub extensions: RefCell<String>,
    /// Buffered bytes awaiting send (updated by I/O thread).
    pub buffered_amount: Cell<u64>,
    /// Event listeners registered via `addEventListener`, keyed by event type.
    pub listener_registry: HashMap<String, Vec<JsObject>>,
}

/// SSE JS callback state.
pub(crate) struct SseCallbacks {
    /// `onopen` handler.
    pub onopen: Option<JsObject>,
    /// `onmessage` handler.
    pub onmessage: Option<JsObject>,
    /// `onerror` handler.
    pub onerror: Option<JsObject>,
    /// The JS `EventSource` object (for `addEventListener` dispatch).
    pub js_object: JsObject,
    /// Connection URL (for `EventSource.url` property).
    #[allow(dead_code)]
    pub url: String,
    /// URL origin (for `MessageEvent.origin`).
    pub origin: String,
    /// Ready state (CONNECTING=0, OPEN=1, CLOSED=2).
    pub ready_state: Cell<u16>,
    /// Event listeners registered via `addEventListener`, keyed by event type.
    pub listener_registry: HashMap<String, Vec<JsObject>>,
}

impl RealtimeState {
    /// Register WebSocket callbacks for a new connection.
    /// Returns the connection ID, or an error if the limit is reached.
    ///
    /// The caller is responsible for sending `RendererToNetwork::WebSocketOpen`
    /// via `NetworkHandle` after this call succeeds.
    pub fn register_ws_callbacks(
        &mut self,
        url: &url::Url,
        js_object: JsObject,
    ) -> Result<u64, String> {
        let total = self.ws_callbacks.len() + self.sse_callbacks.len();
        if total >= MAX_REALTIME_CONNECTIONS {
            return Err("too many concurrent connections".to_string());
        }

        let id = self.next_id;
        self.next_id += 1;

        let url_str = url.to_string();
        let url_origin = url.origin().ascii_serialization();

        let callbacks = WsCallbacks {
            onopen: None,
            onmessage: None,
            onerror: None,
            onclose: None,
            js_object,
            url: url_str,
            origin: url_origin,
            ready_state: Cell::new(0), // CONNECTING
            protocol: RefCell::new(String::new()),
            extensions: RefCell::new(String::new()),
            buffered_amount: Cell::new(0),
            listener_registry: HashMap::new(),
        };

        self.ws_callbacks.insert(id, callbacks);
        Ok(id)
    }

    /// Register SSE callbacks for a new connection.
    /// Returns the connection ID, or an error if the limit is reached.
    ///
    /// The caller is responsible for sending `RendererToNetwork::EventSourceOpen`
    /// via `NetworkHandle` after this call succeeds.
    pub fn register_sse_callbacks(
        &mut self,
        url: &url::Url,
        js_object: JsObject,
    ) -> Result<u64, String> {
        let total = self.ws_callbacks.len() + self.sse_callbacks.len();
        if total >= MAX_REALTIME_CONNECTIONS {
            return Err("too many concurrent connections".to_string());
        }

        let id = self.next_id;
        self.next_id += 1;

        let url_str = url.to_string();
        let url_origin = url.origin().ascii_serialization();

        let callbacks = SseCallbacks {
            onopen: None,
            onmessage: None,
            onerror: None,
            js_object,
            url: url_str,
            origin: url_origin,
            ready_state: Cell::new(0), // CONNECTING
            listener_registry: HashMap::new(),
        };

        self.sse_callbacks.insert(id, callbacks);
        Ok(id)
    }

    /// Close and remove an SSE connection's callbacks.
    pub fn remove_sse(&mut self, id: u64) {
        self.sse_callbacks.remove(&id);
    }

    /// Get a reference to a WebSocket connection's callbacks.
    pub fn ws_callbacks(&self, id: u64) -> Option<&WsCallbacks> {
        self.ws_callbacks.get(&id)
    }

    /// Get a mutable reference to a WebSocket connection's callbacks.
    pub fn ws_callbacks_mut(&mut self, id: u64) -> Option<&mut WsCallbacks> {
        self.ws_callbacks.get_mut(&id)
    }

    /// Get a reference to an SSE connection's callbacks.
    pub fn sse_callbacks(&self, id: u64) -> Option<&SseCallbacks> {
        self.sse_callbacks.get(&id)
    }

    /// Get a mutable reference to an SSE connection's callbacks.
    pub fn sse_callbacks_mut(&mut self, id: u64) -> Option<&mut SseCallbacks> {
        self.sse_callbacks.get_mut(&id)
    }

    /// Remove a WebSocket connection from the registry.
    pub fn remove_ws(&mut self, id: u64) {
        self.ws_callbacks.remove(&id);
    }

    /// Iterate over all WS callback sets (for GC tracing).
    pub fn ws_iter(&self) -> impl Iterator<Item = &WsCallbacks> {
        self.ws_callbacks.values()
    }

    /// Iterate over all SSE callback sets (for GC tracing).
    pub fn sse_iter(&self) -> impl Iterator<Item = &SseCallbacks> {
        self.sse_callbacks.values()
    }

    /// Clear all callbacks. Called during navigation/shutdown.
    pub fn clear_all(&mut self) {
        self.ws_callbacks.clear();
        self.sse_callbacks.clear();
    }
}

// --- HostBridge realtime methods (extracted from mod.rs) ---

use super::HostBridge;

impl HostBridge {
    /// Drain all pending WebSocket and SSE events from the Network Process.
    pub fn drain_realtime_events(&self) -> RealtimeEvents {
        let inner = self.inner.borrow();
        let Some(handle) = &inner.network_handle else {
            return (Vec::new(), Vec::new());
        };
        let events = handle.drain_events();
        let mut ws_events = Vec::new();
        let mut sse_events = Vec::new();
        for event in events {
            match event {
                elidex_net::broker::NetworkToRenderer::WebSocketEvent(conn_id, ws_event) => {
                    ws_events.push((conn_id, ws_event));
                }
                elidex_net::broker::NetworkToRenderer::EventSourceEvent(conn_id, sse_event) => {
                    sse_events.push((conn_id, sse_event));
                }
                elidex_net::broker::NetworkToRenderer::FetchResponse(..) => {
                    // Late-arriving fetch response after content-thread timeout
                    // (30s in fetch_blocking). Safe to drop — no JS promise waiting.
                }
            }
        }
        (ws_events, sse_events)
    }

    /// Shut down all WebSocket and SSE connections via the Network Process.
    pub fn shutdown_all_realtime(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.realtime.clear_all();
        if let Some(handle) = &inner.network_handle {
            handle.send(elidex_net::broker::RendererToNetwork::Shutdown);
        }
    }

    // --- WebSocket API ---

    /// Open a WebSocket connection. Returns connection ID or error.
    pub fn open_websocket(
        &self,
        url: url::Url,
        protocols: Vec<String>,
        origin: String,
        js_object: JsObject,
    ) -> Result<u64, String> {
        let mut inner = self.inner.borrow_mut();
        match inner.network_handle.as_ref() {
            None => return Err("network unavailable".to_string()),
            Some(h) if h.client_id() == 0 => return Err("network disconnected".to_string()),
            _ => {}
        }
        let conn_id = inner.realtime.register_ws_callbacks(&url, js_object)?;
        if let Some(handle) = &inner.network_handle {
            if !handle.send(elidex_net::broker::RendererToNetwork::WebSocketOpen {
                conn_id,
                url,
                protocols,
                origin,
            }) {
                inner.realtime.remove_ws(conn_id);
                return Err("network broker disconnected".to_string());
            }
        }
        Ok(conn_id)
    }

    /// Read a WebSocket callback field via a closure.
    pub(crate) fn with_ws_callbacks<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&WsCallbacks) -> R,
    {
        self.inner.borrow().realtime.ws_callbacks(id).map(f)
    }

    /// Mutate a WebSocket callback field via a closure.
    pub(crate) fn with_ws_callbacks_mut<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut WsCallbacks) -> R,
    {
        self.inner.borrow_mut().realtime.ws_callbacks_mut(id).map(f)
    }

    /// Send text on a WebSocket via the Network Process.
    #[must_use]
    pub fn ws_send_text(&self, id: u64, data: String) -> bool {
        let inner = self.inner.borrow();
        if inner.realtime.ws_callbacks(id).is_none() {
            return false;
        }
        if let Some(handle) = &inner.network_handle {
            handle.send(elidex_net::broker::RendererToNetwork::WebSocketSend(
                id,
                elidex_net::ws::WsCommand::SendText(data),
            ))
        } else {
            false
        }
    }

    /// Close a WebSocket via the Network Process.
    pub fn ws_close(&self, id: u64, code: u16, reason: String) {
        let inner = self.inner.borrow();
        if let Some(handle) = &inner.network_handle {
            let _ = handle.send(elidex_net::broker::RendererToNetwork::WebSocketSend(
                id,
                elidex_net::ws::WsCommand::Close(code, reason),
            ));
        }
    }

    /// Remove a WebSocket from the registry.
    pub fn remove_ws(&self, id: u64) {
        self.inner.borrow_mut().realtime.remove_ws(id);
    }

    // --- EventSource API ---

    /// Open an `EventSource` connection via the Network Process.
    pub fn open_event_source(
        &self,
        url: url::Url,
        with_credentials: bool,
        origin: Option<String>,
        js_object: JsObject,
    ) -> Result<u64, String> {
        let mut inner = self.inner.borrow_mut();
        match inner.network_handle.as_ref() {
            None => return Err("network unavailable".to_string()),
            Some(h) if h.client_id() == 0 => return Err("network disconnected".to_string()),
            _ => {}
        }
        let conn_id = inner.realtime.register_sse_callbacks(&url, js_object)?;
        if let Some(handle) = &inner.network_handle {
            if !handle.send(elidex_net::broker::RendererToNetwork::EventSourceOpen {
                conn_id,
                url,
                last_event_id: None,
                origin,
                with_credentials,
            }) {
                inner.realtime.remove_sse(conn_id);
                return Err("network broker disconnected".to_string());
            }
        }
        Ok(conn_id)
    }

    /// Read an SSE callback field via a closure.
    pub(crate) fn with_sse_callbacks<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&SseCallbacks) -> R,
    {
        self.inner.borrow().realtime.sse_callbacks(id).map(f)
    }

    /// Mutate an SSE callback field via a closure.
    pub(crate) fn with_sse_callbacks_mut<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut SseCallbacks) -> R,
    {
        self.inner
            .borrow_mut()
            .realtime
            .sse_callbacks_mut(id)
            .map(f)
    }

    /// Close and remove an SSE connection via the Network Process.
    pub fn sse_close(&self, id: u64) {
        let mut inner = self.inner.borrow_mut();
        inner.realtime.remove_sse(id);
        if let Some(handle) = &inner.network_handle {
            let _ = handle.send(elidex_net::broker::RendererToNetwork::EventSourceClose(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realtime_state_default_is_empty() {
        let state = RealtimeState::default();
        assert!(state.ws_callbacks(0).is_none());
        assert!(state.sse_callbacks(0).is_none());
    }

    #[test]
    fn realtime_state_clear_all_ok() {
        let mut state = RealtimeState::default();
        state.clear_all(); // Should not panic
    }

    #[test]
    fn realtime_state_ws_callbacks_none_for_invalid_id() {
        let state = RealtimeState::default();
        assert!(state.ws_callbacks(999).is_none());
    }

    #[test]
    fn realtime_state_sse_callbacks_none_for_invalid_id() {
        let state = RealtimeState::default();
        assert!(state.sse_callbacks(999).is_none());
    }

    #[test]
    fn close_event_init_fields() {
        let init = elidex_plugin::CloseEventInit {
            code: 1000,
            reason: "normal".to_string(),
            was_clean: true,
        };
        assert_eq!(init.code, 1000);
        assert_eq!(init.reason, "normal");
        assert!(init.was_clean);
    }

    #[test]
    fn close_event_init_abnormal() {
        let init = elidex_plugin::CloseEventInit {
            code: 1006,
            reason: String::new(),
            was_clean: false,
        };
        assert_eq!(init.code, 1006);
        assert!(!init.was_clean);
        assert!(init.reason.is_empty());
    }

    #[test]
    fn event_payload_default_is_none() {
        let payload = elidex_plugin::EventPayload::default();
        assert!(matches!(payload, elidex_plugin::EventPayload::None));
    }

    #[test]
    fn message_payload_construction() {
        let payload = elidex_plugin::EventPayload::Message {
            data: "hello".to_string(),
            origin: "https://example.com".to_string(),
            last_event_id: "42".to_string(),
        };
        if let elidex_plugin::EventPayload::Message {
            data,
            origin,
            last_event_id,
        } = &payload
        {
            assert_eq!(data, "hello");
            assert_eq!(origin, "https://example.com");
            assert_eq!(last_event_id, "42");
        } else {
            panic!("expected Message");
        }
    }
}
