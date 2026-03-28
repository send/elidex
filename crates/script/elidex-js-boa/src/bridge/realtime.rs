//! WebSocket / SSE connection registry for the JS bridge.
//!
//! Manages active WebSocket and `EventSource` connections, storing callbacks
//! and connection handles. Integrated into `HostBridgeInner` as a sub-struct.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use boa_engine::JsObject;

use elidex_net::sse::{SseCommand, SseEvent, SseHandle};
use elidex_net::ws::{WsCommand, WsEvent, WsHandle};
use elidex_net::CookieJar;

/// Return type for `drain_realtime_events()`.
pub(crate) type RealtimeEvents = (Vec<(u64, WsEvent)>, Vec<(u64, SseEvent)>);

/// Maximum concurrent WebSocket + SSE connections per document.
const MAX_REALTIME_CONNECTIONS: usize = 256;

/// WebSocket/SSE connection state, stored in `HostBridgeInner`.
#[derive(Default)]
pub(crate) struct RealtimeState {
    /// Active WebSocket connections.
    ws_connections: HashMap<u64, WsConnection>,
    /// Active SSE connections.
    sse_connections: HashMap<u64, SseConnection>,
    /// Next connection ID (shared counter for WS and SSE).
    next_id: u64,
    /// Shared cookie jar for `withCredentials` support on SSE connections.
    cookie_jar: Option<Arc<CookieJar>>,
}

struct WsConnection {
    handle: WsHandle,
    pub(crate) callbacks: WsCallbacks,
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

struct SseConnection {
    handle: SseHandle,
    pub(crate) callbacks: SseCallbacks,
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
    /// Set the shared cookie jar for `withCredentials` support.
    pub fn set_cookie_jar(&mut self, jar: Option<Arc<CookieJar>>) {
        self.cookie_jar = jar;
    }
    /// Open a new WebSocket connection. Returns the connection ID, or an error
    /// if the per-document connection limit has been reached.
    pub fn open_websocket(
        &mut self,
        url: url::Url,
        protocols: Vec<String>,
        origin: String,
        js_object: JsObject,
    ) -> Result<u64, String> {
        let total = self.ws_connections.len() + self.sse_connections.len();
        if total >= MAX_REALTIME_CONNECTIONS {
            return Err("too many concurrent connections".to_string());
        }

        let id = self.next_id;
        self.next_id += 1;

        let url_str = url.to_string();
        let url_origin = url.origin().ascii_serialization();
        let handle = elidex_net::ws::spawn_ws_thread(url, protocols, origin);

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

        self.ws_connections
            .insert(id, WsConnection { handle, callbacks });
        Ok(id)
    }

    /// Send a text message on a WebSocket. Returns `false` if the ID is invalid.
    #[must_use]
    pub fn ws_send_text(&self, id: u64, data: String) -> bool {
        self.ws_connections.get(&id).is_some_and(|conn| {
            conn.handle
                .command_tx
                .send(WsCommand::SendText(data))
                .is_ok()
        })
    }

    /// Send a binary message on a WebSocket.
    #[must_use]
    #[allow(dead_code)] // Called when binaryType="arraybuffer" support lands (M4-9).
    pub fn ws_send_binary(&self, id: u64, data: Vec<u8>) -> bool {
        self.ws_connections.get(&id).is_some_and(|conn| {
            conn.handle
                .command_tx
                .send(WsCommand::SendBinary(data))
                .is_ok()
        })
    }

    /// Initiate WebSocket close handshake.
    pub fn ws_close(&self, id: u64, code: u16, reason: String) {
        if let Some(conn) = self.ws_connections.get(&id) {
            let _ = conn.handle.command_tx.send(WsCommand::Close(code, reason));
        }
    }

    /// Open a new SSE connection. Returns the connection ID, or an error
    /// if the per-document connection limit has been reached.
    ///
    /// Uses the shared `cookie_jar` (set via `set_cookie_jar()`) when
    /// `with_credentials` is true; otherwise no cookies are sent.
    /// `origin` is the document origin for CORS validation.
    pub fn open_event_source(
        &mut self,
        url: url::Url,
        with_credentials: bool,
        origin: Option<String>,
        js_object: JsObject,
    ) -> Result<u64, String> {
        let total = self.ws_connections.len() + self.sse_connections.len();
        if total >= MAX_REALTIME_CONNECTIONS {
            return Err("too many concurrent connections".to_string());
        }

        let id = self.next_id;
        self.next_id += 1;

        let url_str = url.to_string();
        let url_origin = url.origin().ascii_serialization();
        let jar = if with_credentials {
            self.cookie_jar.clone()
        } else {
            None
        };
        let handle = elidex_net::sse::spawn_sse_thread(url, None, jar, origin);

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

        self.sse_connections
            .insert(id, SseConnection { handle, callbacks });
        Ok(id)
    }

    /// Close an SSE connection.
    pub fn sse_close(&self, id: u64) {
        if let Some(conn) = self.sse_connections.get(&id) {
            let _ = conn.handle.command_tx.send(SseCommand::Close);
        }
    }

    /// Drain all pending events from all WS and SSE connections.
    pub fn drain_realtime_events(&mut self) -> RealtimeEvents {
        let mut ws_events = Vec::new();
        for (&id, conn) in &self.ws_connections {
            while let Ok(event) = conn.handle.event_rx.try_recv() {
                ws_events.push((id, event));
            }
        }

        let mut sse_events = Vec::new();
        for (&id, conn) in &self.sse_connections {
            while let Ok(event) = conn.handle.event_rx.try_recv() {
                sse_events.push((id, event));
            }
        }

        (ws_events, sse_events)
    }

    /// Get a reference to a WebSocket connection's callbacks.
    pub fn ws_callbacks(&self, id: u64) -> Option<&WsCallbacks> {
        self.ws_connections.get(&id).map(|c| &c.callbacks)
    }

    /// Get a mutable reference to a WebSocket connection's callbacks.
    pub fn ws_callbacks_mut(&mut self, id: u64) -> Option<&mut WsCallbacks> {
        self.ws_connections.get_mut(&id).map(|c| &mut c.callbacks)
    }

    /// Get a reference to an SSE connection's callbacks.
    pub fn sse_callbacks(&self, id: u64) -> Option<&SseCallbacks> {
        self.sse_connections.get(&id).map(|c| &c.callbacks)
    }

    /// Get a mutable reference to an SSE connection's callbacks.
    pub fn sse_callbacks_mut(&mut self, id: u64) -> Option<&mut SseCallbacks> {
        self.sse_connections.get_mut(&id).map(|c| &mut c.callbacks)
    }

    /// Remove a WebSocket connection from the registry.
    pub fn remove_ws(&mut self, id: u64) {
        self.ws_connections.remove(&id);
    }

    /// Remove an SSE connection from the registry.
    pub fn remove_sse(&mut self, id: u64) {
        self.sse_connections.remove(&id);
    }

    /// Iterate over all WS callback sets (for GC tracing).
    pub fn ws_iter(&self) -> impl Iterator<Item = &WsCallbacks> {
        self.ws_connections.values().map(|c| &c.callbacks)
    }

    /// Iterate over all SSE callback sets (for GC tracing).
    pub fn sse_iter(&self) -> impl Iterator<Item = &SseCallbacks> {
        self.sse_connections.values().map(|c| &c.callbacks)
    }

    /// Shut down all connections gracefully.
    ///
    /// Sends close commands to all I/O threads and drops handles without joining.
    /// Threads will exit when they detect channel disconnect or the close
    /// handshake completes.
    pub fn shutdown_all(&mut self) {
        for (_, conn) in self.ws_connections.drain() {
            let _ = conn
                .handle
                .command_tx
                .send(WsCommand::Close(1001, String::new()));
            // Don't join — thread will exit when it detects channel disconnect
            // or close handshake completes.
        }
        for (_, conn) in self.sse_connections.drain() {
            let _ = conn.handle.command_tx.send(SseCommand::Close);
        }
    }
}

impl Drop for RealtimeState {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realtime_state_default_is_empty() {
        let mut state = RealtimeState::default();
        let (ws, sse) = state.drain_realtime_events();
        assert!(ws.is_empty());
        assert!(sse.is_empty());
    }

    #[test]
    fn realtime_state_shutdown_empty_ok() {
        let mut state = RealtimeState::default();
        state.shutdown_all(); // Should not panic
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
