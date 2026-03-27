//! WebSocket / SSE connection registry for the JS bridge.
//!
//! Manages active WebSocket and `EventSource` connections, storing callbacks
//! and connection handles. Integrated into `HostBridgeInner` as a sub-struct.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use boa_engine::JsObject;

use elidex_net::sse::{SseCommand, SseEvent, SseHandle};
use elidex_net::ws::{WsCommand, WsEvent, WsHandle};

/// Return type for `drain_realtime_events()`.
pub(crate) type RealtimeEvents = (Vec<(u64, WsEvent)>, Vec<(u64, SseEvent)>);

/// WebSocket/SSE connection state, stored in `HostBridgeInner`.
#[derive(Default)]
pub(crate) struct RealtimeState {
    /// Active WebSocket connections.
    ws_connections: HashMap<u64, WsConnection>,
    /// Active SSE connections.
    sse_connections: HashMap<u64, SseConnection>,
    /// Next connection ID (shared counter for WS and SSE).
    next_id: u64,
}

struct WsConnection {
    handle: WsHandle,
    pub(crate) callbacks: WsCallbacks,
}

/// WebSocket JS callback state.
#[allow(dead_code)]
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
    pub url: String,
    /// URL origin (for `MessageEvent.origin`).
    pub origin: String,
    /// Ready state (CONNECTING=0, OPEN=1, CLOSING=2, CLOSED=3).
    pub ready_state: Rc<Cell<u16>>,
    /// Negotiated sub-protocol (updated on `Connected`).
    pub protocol: Rc<RefCell<String>>,
    /// Negotiated extensions (updated on `Connected`).
    pub extensions: Rc<RefCell<String>>,
    /// Buffered bytes awaiting send (updated by I/O thread).
    pub buffered_amount: Rc<Cell<u64>>,
}

struct SseConnection {
    handle: SseHandle,
    pub(crate) callbacks: SseCallbacks,
}

/// SSE JS callback state.
#[allow(dead_code)]
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
    pub url: String,
    /// URL origin (for `MessageEvent.origin`).
    pub origin: String,
    /// Ready state (CONNECTING=0, OPEN=1, CLOSED=2).
    pub ready_state: Rc<Cell<u16>>,
}

impl RealtimeState {
    /// Open a new WebSocket connection. Returns the connection ID.
    pub fn open_websocket(
        &mut self,
        url: url::Url,
        protocols: Vec<String>,
        origin: String,
        js_object: JsObject,
    ) -> u64 {
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
            ready_state: Rc::new(Cell::new(0)), // CONNECTING
            protocol: Rc::new(RefCell::new(String::new())),
            extensions: Rc::new(RefCell::new(String::new())),
            buffered_amount: Rc::new(Cell::new(0)),
        };

        self.ws_connections
            .insert(id, WsConnection { handle, callbacks });
        id
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
    #[allow(dead_code)]
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

    /// Open a new SSE connection. Returns the connection ID.
    pub fn open_event_source(
        &mut self,
        url: url::Url,
        with_credentials: bool,
        cookie_jar: Option<std::sync::Arc<elidex_net::CookieJar>>,
        js_object: JsObject,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let url_str = url.to_string();
        let url_origin = url.origin().ascii_serialization();
        let handle = elidex_net::sse::spawn_sse_thread(url, with_credentials, None, cookie_jar);

        let callbacks = SseCallbacks {
            onopen: None,
            onmessage: None,
            onerror: None,
            js_object,
            url: url_str,
            origin: url_origin,
            ready_state: Rc::new(Cell::new(0)), // CONNECTING
        };

        self.sse_connections
            .insert(id, SseConnection { handle, callbacks });
        id
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
    pub fn shutdown_all(&mut self) {
        for (_, conn) in self.ws_connections.drain() {
            let _ = conn
                .handle
                .command_tx
                .send(WsCommand::Close(1001, String::new()));
            if let Some(thread) = conn.handle.thread {
                let _ = thread.join();
            }
        }
        for (_, conn) in self.sse_connections.drain() {
            let _ = conn.handle.command_tx.send(SseCommand::Close);
            if let Some(thread) = conn.handle.thread {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for RealtimeState {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}
