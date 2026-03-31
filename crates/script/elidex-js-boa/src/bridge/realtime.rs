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
        url: url::Url,
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
        url: url::Url,
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
