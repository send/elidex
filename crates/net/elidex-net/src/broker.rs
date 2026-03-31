//! Network Process broker (design doc §5.2, §5.3.3).
//!
//! Implements the Network Process as a singleton thread that owns the tokio
//! runtime, [`NetClient`], cookie jar, and all WebSocket/SSE I/O loops.
//!
//! Content threads (Renderers) communicate exclusively through typed channels:
//! - [`RendererToNetwork`]: requests from content thread → Network Process
//! - [`NetworkToRenderer`]: responses/events from Network Process → content thread
//!
//! The broker is spawned once by the browser thread via [`spawn_network_process`].
//! Each content thread receives a [`NetworkHandle`] for IPC. All network access
//! is mediated through the broker — content threads never touch network APIs
//! directly, enabling OS-level sandbox enforcement (seccomp-bpf, etc.).
//!
//! # Cookie sharing
//!
//! The broker owns a single [`NetClient`] (with shared `CookieJar`), fixing
//! the previous design where each content thread had its own `FetchHandle`
//! with an isolated cookie jar (spec violation — cookies must be shared
//! across browsing contexts within a profile).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{self, TryRecvError};

use crate::sse::{SseCommand, SseEvent, SseHandle};
use crate::ws::{WsCommand, WsEvent, WsHandle};
use crate::{NetClient, Request, Response};

// ---------------------------------------------------------------------------
// ID types
// ---------------------------------------------------------------------------

/// Unique fetch request identifier (monotonic per-renderer).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FetchId(pub u64);

/// Monotonic counter for renderer client IDs.
static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Monotonic counter for fetch request IDs.
static FETCH_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl FetchId {
    /// Generate a new unique fetch ID.
    #[must_use]
    pub fn next() -> Self {
        Self(FETCH_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

// ---------------------------------------------------------------------------
// Message types (design doc §5.3.3)
// ---------------------------------------------------------------------------

/// Messages from a Renderer (content thread) to the Network Process.
#[derive(Debug)]
pub enum RendererToNetwork {
    /// HTTP fetch request.
    Fetch(FetchId, Request),
    /// Cancel a pending fetch.
    CancelFetch(FetchId),
    /// Open a WebSocket connection.
    WebSocketOpen {
        /// Connection ID (assigned by the renderer).
        conn_id: u64,
        /// WebSocket URL (ws:// or wss://).
        url: url::Url,
        /// Requested sub-protocols.
        protocols: Vec<String>,
        /// Document origin for the `Origin` header.
        origin: String,
    },
    /// Send a WebSocket command (text/binary/close).
    WebSocketSend(u64, WsCommand),
    /// Close a WebSocket connection.
    WebSocketClose(u64),
    /// Open a Server-Sent Events connection.
    EventSourceOpen {
        /// Connection ID (assigned by the renderer).
        conn_id: u64,
        /// HTTP(S) URL for the event stream.
        url: url::Url,
        /// Last event ID for reconnection.
        last_event_id: Option<String>,
        /// Document origin for CORS (None = same-origin).
        origin: Option<String>,
        /// Whether to send credentials (cookies) cross-origin.
        with_credentials: bool,
    },
    /// Close an SSE connection (stop auto-reconnect).
    EventSourceClose(u64),
    /// Shutdown all connections for this renderer.
    Shutdown,
}

/// Messages from the Network Process to a Renderer (content thread).
#[derive(Debug)]
pub enum NetworkToRenderer {
    /// HTTP fetch response.
    FetchResponse(FetchId, Result<Response, String>),
    /// WebSocket event.
    WebSocketEvent(u64, WsEvent),
    /// SSE event.
    EventSourceEvent(u64, SseEvent),
}

/// Control messages from the Browser thread to the Network Process.
#[derive(Debug)]
pub enum NetworkProcessControl {
    /// Register a new renderer (content thread).
    RegisterRenderer {
        /// Unique client identifier.
        client_id: u64,
        /// Channel to send responses/events to this renderer.
        response_tx: crossbeam_channel::Sender<NetworkToRenderer>,
    },
    /// Unregister a renderer (content thread shutting down).
    UnregisterRenderer {
        /// Client ID to remove.
        client_id: u64,
    },
    /// Shutdown the Network Process.
    Shutdown,
}

// ---------------------------------------------------------------------------
// NetworkProcessHandle (Browser side)
// ---------------------------------------------------------------------------

/// Handle held by the browser thread to control the Network Process.
///
/// Creates [`NetworkHandle`]s for content threads and manages the Network
/// Process lifecycle.
pub struct NetworkProcessHandle {
    control_tx: crossbeam_channel::Sender<NetworkProcessControl>,
    request_tx: crossbeam_channel::Sender<(u64, RendererToNetwork)>,
    thread: Option<JoinHandle<()>>,
}

impl NetworkProcessHandle {
    /// Create a new renderer handle and register it with the Network Process.
    ///
    /// The returned [`NetworkHandle`] should be passed to the content thread.
    #[must_use]
    pub fn create_renderer_handle(&self) -> NetworkHandle {
        let client_id = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let (response_tx, response_rx) = crossbeam_channel::unbounded();

        let _ = self
            .control_tx
            .send(NetworkProcessControl::RegisterRenderer {
                client_id,
                response_tx,
            });

        NetworkHandle {
            client_id,
            request_tx: self.request_tx.clone(),
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Unregister a renderer. All its connections will be closed.
    pub fn unregister_renderer(&self, client_id: u64) {
        let _ = self
            .control_tx
            .send(NetworkProcessControl::UnregisterRenderer { client_id });
    }

    /// Shutdown the Network Process and wait for the thread to finish.
    pub fn shutdown(mut self) {
        let _ = self.control_tx.send(NetworkProcessControl::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for NetworkProcessHandle {
    fn drop(&mut self) {
        let _ = self.control_tx.send(NetworkProcessControl::Shutdown);
    }
}

// ---------------------------------------------------------------------------
// NetworkHandle (Renderer / Content thread side)
// ---------------------------------------------------------------------------

/// Handle held by a content thread for communicating with the Network Process.
///
/// All network operations go through this handle. The content thread never
/// directly accesses `NetClient`, `FetchHandle`, or I/O thread handles.
///
/// Events received during a blocking fetch (WS/SSE messages) are buffered
/// internally and returned by the next call to [`drain_events`](Self::drain_events).
pub struct NetworkHandle {
    /// Unique client identifier for this renderer.
    client_id: u64,
    /// Shared request channel (all renderers → Network Process).
    request_tx: crossbeam_channel::Sender<(u64, RendererToNetwork)>,
    /// Dedicated response channel (Network Process → this renderer).
    response_rx: crossbeam_channel::Receiver<NetworkToRenderer>,
    /// Events buffered during blocking fetch (drained by `drain_events()`).
    /// Uses `RefCell` for interior mutability (content thread is single-threaded).
    buffered: std::cell::RefCell<Vec<NetworkToRenderer>>,
}

impl NetworkHandle {
    /// Create a disconnected `NetworkHandle` for tests and contexts where
    /// no Network Process is available (standalone pipelines, OOP iframes
    /// before proper handle wiring).
    ///
    /// `fetch_blocking()` will return an error; WS/SSE opens are silently dropped.
    #[must_use]
    pub fn disconnected() -> Self {
        // Create a channel pair and immediately drop the receiver,
        // making all sends on request_tx fail silently.
        let (request_tx, _request_rx) = crossbeam_channel::unbounded();
        let (_response_tx, response_rx) = crossbeam_channel::unbounded();
        Self {
            client_id: 0,
            request_tx,
            response_rx,
            buffered: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Get this renderer's client ID.
    #[must_use]
    pub fn client_id(&self) -> u64 {
        self.client_id
    }

    /// Send a blocking fetch request.
    ///
    /// The content thread blocks until the fetch completes (or times out
    /// at 30 seconds). Any WS/SSE events received while waiting are
    /// buffered internally and returned by the next [`drain_events`](Self::drain_events)
    /// call.
    pub fn fetch_blocking(&self, request: Request) -> Result<Response, String> {
        let fetch_id = FetchId::next();
        let _ = self
            .request_tx
            .send((self.client_id, RendererToNetwork::Fetch(fetch_id, request)));

        let mut buf = self.buffered.borrow_mut();
        loop {
            match self.response_rx.recv_timeout(Duration::from_secs(30)) {
                Ok(NetworkToRenderer::FetchResponse(id, result)) if id == fetch_id => {
                    return result;
                }
                Ok(other) => buf.push(other),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    return Err("fetch timeout (30s)".into());
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    return Err("network process disconnected".into());
                }
            }
        }
    }

    /// Non-blocking drain of all pending events (WS/SSE/fetch responses).
    ///
    /// Includes any events buffered during a prior [`fetch_blocking`](Self::fetch_blocking) call.
    pub fn drain_events(&self) -> Vec<NetworkToRenderer> {
        let mut events: Vec<_> = self.buffered.borrow_mut().drain(..).collect();
        while let Ok(msg) = self.response_rx.try_recv() {
            events.push(msg);
        }
        events
    }

    /// Send a message to the Network Process without waiting for a response.
    ///
    /// Used for fire-and-forget operations: WS/SSE open, send, close.
    pub fn send(&self, msg: RendererToNetwork) {
        let _ = self.request_tx.send((self.client_id, msg));
    }
}

impl std::fmt::Debug for NetworkHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkHandle")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Network Process thread
// ---------------------------------------------------------------------------

/// Spawn the Network Process thread.
///
/// Returns a [`NetworkProcessHandle`] for the browser thread to manage
/// renderer registrations and lifecycle.
#[must_use]
pub fn spawn_network_process(client: NetClient) -> NetworkProcessHandle {
    let (control_tx, control_rx) = crossbeam_channel::unbounded();
    let (request_tx, request_rx) = crossbeam_channel::unbounded();

    let client = Arc::new(client);
    let thread = std::thread::Builder::new()
        .name("elidex-network".into())
        .spawn(move || {
            network_process_main(client, request_rx, control_rx);
        })
        .expect("failed to spawn network process thread");

    NetworkProcessHandle {
        control_tx,
        request_tx,
        thread: Some(thread),
    }
}

/// Main loop of the Network Process thread.
fn network_process_main(
    client: Arc<NetClient>,
    request_rx: crossbeam_channel::Receiver<(u64, RendererToNetwork)>,
    control_rx: crossbeam_channel::Receiver<NetworkProcessControl>,
) {
    let mut state = NetworkProcessState::new();

    loop {
        // 1. Process control messages first (register/unregister/shutdown).
        //    Must run before requests so that a newly registered renderer's
        //    first request is processed with its response channel available.
        while let Ok(ctrl) = control_rx.try_recv() {
            if !state.handle_control(ctrl) {
                return;
            }
        }

        // 2. Batch-process renderer requests (non-blocking, up to 64 per iteration).
        for _ in 0..64 {
            match request_rx.try_recv() {
                Ok((cid, msg)) => state.handle_request(cid, msg, &client),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        // 3. Forward WebSocket events from I/O threads to renderers.
        state.forward_ws_events();

        // 4. Forward SSE events from I/O threads to renderers.
        state.forward_sse_events();

        // 5. Clean up finished I/O threads.
        state.cleanup_finished();

        // 6. Brief yield to avoid busy-spinning.
        //    1ms is sufficient — WS/SSE events are polled at ~1kHz,
        //    which is well within the content thread's 16ms frame budget.
        std::thread::sleep(Duration::from_millis(1));
    }
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Internal state of the Network Process.
struct NetworkProcessState {
    /// Registered renderer clients: client_id → response sender.
    clients: HashMap<u64, crossbeam_channel::Sender<NetworkToRenderer>>,
    /// Active WebSocket connections: (client_id, conn_id) → WsHandle.
    ws_handles: HashMap<(u64, u64), WsHandle>,
    /// Active SSE connections: (client_id, conn_id) → SseHandle.
    sse_handles: HashMap<(u64, u64), SseHandle>,
}

impl NetworkProcessState {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            ws_handles: HashMap::new(),
            sse_handles: HashMap::new(),
        }
    }

    fn handle_request(&mut self, cid: u64, msg: RendererToNetwork, client: &Arc<NetClient>) {
        match msg {
            RendererToNetwork::Fetch(fetch_id, request) => {
                self.handle_fetch(cid, fetch_id, request, client);
            }
            RendererToNetwork::CancelFetch(_fetch_id) => {
                // TODO: cancel in-flight fetch (requires task handle tracking).
            }
            RendererToNetwork::WebSocketOpen {
                conn_id,
                url,
                protocols,
                origin,
            } => {
                let handle = crate::ws::spawn_ws_thread(url, protocols, origin);
                self.ws_handles.insert((cid, conn_id), handle);
            }
            RendererToNetwork::WebSocketSend(conn_id, command) => {
                if let Some(handle) = self.ws_handles.get(&(cid, conn_id)) {
                    let _ = handle.command_tx.send(command);
                }
            }
            RendererToNetwork::WebSocketClose(conn_id) => {
                if let Some(handle) = self.ws_handles.get(&(cid, conn_id)) {
                    let _ = handle
                        .command_tx
                        .send(WsCommand::Close(1000, String::new()));
                }
            }
            RendererToNetwork::EventSourceOpen {
                conn_id,
                url,
                last_event_id,
                origin,
                with_credentials,
            } => {
                let cookie_jar = if with_credentials {
                    Some(client.cookie_jar_arc())
                } else {
                    None
                };
                let handle = crate::sse::spawn_sse_thread(
                    url,
                    last_event_id,
                    cookie_jar,
                    origin,
                    with_credentials,
                );
                self.sse_handles.insert((cid, conn_id), handle);
            }
            RendererToNetwork::EventSourceClose(conn_id) => {
                if let Some(handle) = self.sse_handles.get(&(cid, conn_id)) {
                    let _ = handle.command_tx.send(SseCommand::Close);
                }
            }
            RendererToNetwork::Shutdown => {
                self.close_all_for_client(cid);
            }
        }
    }

    fn handle_fetch(&self, cid: u64, fetch_id: FetchId, request: Request, client: &Arc<NetClient>) {
        let client = Arc::clone(client);
        let tx = self.clients.get(&cid).cloned();
        // Spawn a dedicated thread with its own current-thread runtime for
        // each fetch. This mirrors the proven `FetchHandle::send_blocking()`
        // pattern and avoids issues with `tokio::spawn` from non-worker threads.
        //
        // Multiple concurrent fetches run on separate threads, giving us
        // parallelism without requiring the broker's main loop to be async.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create fetch runtime");
            let result = rt
                .block_on(client.send(request))
                .map_err(|e| format!("{e:#}"));
            if let Some(tx) = tx {
                let _ = tx.send(NetworkToRenderer::FetchResponse(fetch_id, result));
            }
        });
    }

    fn handle_control(&mut self, ctrl: NetworkProcessControl) -> bool {
        match ctrl {
            NetworkProcessControl::RegisterRenderer {
                client_id,
                response_tx,
            } => {
                self.clients.insert(client_id, response_tx);
                true
            }
            NetworkProcessControl::UnregisterRenderer { client_id } => {
                self.close_all_for_client(client_id);
                self.clients.remove(&client_id);
                true
            }
            NetworkProcessControl::Shutdown => false,
        }
    }

    fn forward_ws_events(&mut self) {
        let mut remove = Vec::new();
        for (&(cid, conn_id), handle) in &self.ws_handles {
            loop {
                match handle.event_rx.try_recv() {
                    Ok(event) => {
                        if let Some(tx) = self.clients.get(&cid) {
                            let _ = tx.send(NetworkToRenderer::WebSocketEvent(conn_id, event));
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        remove.push((cid, conn_id));
                        break;
                    }
                }
            }
        }
        for key in remove {
            self.ws_handles.remove(&key);
        }
    }

    fn forward_sse_events(&mut self) {
        let mut remove = Vec::new();
        for (&(cid, conn_id), handle) in &self.sse_handles {
            loop {
                match handle.event_rx.try_recv() {
                    Ok(event) => {
                        if let Some(tx) = self.clients.get(&cid) {
                            let _ = tx.send(NetworkToRenderer::EventSourceEvent(conn_id, event));
                        }
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        remove.push((cid, conn_id));
                        break;
                    }
                }
            }
        }
        for key in remove {
            self.sse_handles.remove(&key);
        }
    }

    fn cleanup_finished(&mut self) {
        self.ws_handles
            .retain(|_, handle| handle.thread.as_ref().map_or(true, |t| !t.is_finished()));
        self.sse_handles
            .retain(|_, handle| handle.thread.as_ref().map_or(true, |t| !t.is_finished()));
    }

    fn close_all_for_client(&mut self, client_id: u64) {
        // Close WebSocket connections.
        let ws_keys: Vec<_> = self
            .ws_handles
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in ws_keys {
            if let Some(handle) = self.ws_handles.remove(&key) {
                let _ = handle
                    .command_tx
                    .send(WsCommand::Close(1001, "navigated away".into()));
            }
        }

        // Close SSE connections.
        let sse_keys: Vec<_> = self
            .sse_handles
            .keys()
            .filter(|(cid, _)| *cid == client_id)
            .copied()
            .collect();
        for key in sse_keys {
            if let Some(handle) = self.sse_handles.remove(&key) {
                let _ = handle.command_tx.send(SseCommand::Close);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NetClient, NetClientConfig, TransportConfig};

    fn test_client() -> NetClient {
        NetClient::with_config(NetClientConfig {
            transport: TransportConfig {
                allow_private_ips: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[test]
    fn spawn_and_shutdown() {
        let handle = spawn_network_process(test_client());
        handle.shutdown();
    }

    #[test]
    fn create_renderer_handle() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        assert!(renderer.client_id() > 0);
        handle.shutdown();
    }

    #[test]
    fn unregister_renderer() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        let cid = renderer.client_id();
        handle.unregister_renderer(cid);
        // Brief wait for unregistration to propagate.
        std::thread::sleep(Duration::from_millis(10));
        handle.shutdown();
    }

    #[test]
    fn fetch_blocking_connection_refused() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse("http://127.0.0.1:1/").unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
        };

        let result = renderer.fetch_blocking(request);
        assert!(result.is_err());

        handle.shutdown();
    }

    #[test]
    fn fetch_blocking_success() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        // Bind a sync TCP server — no race with thread startup.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            stream.write_all(resp).unwrap();
        });

        let np = spawn_network_process(test_client());
        let renderer = np.create_renderer_handle();

        let request = Request {
            method: "GET".to_string(),
            url: url::Url::parse(&format!("http://127.0.0.1:{}/", addr.port())).unwrap(),
            headers: Vec::new(),
            body: bytes::Bytes::new(),
        };

        let result = renderer.fetch_blocking(request);
        let resp = result.unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body.as_ref(), b"ok");

        server_thread.join().unwrap();
        np.shutdown();
    }

    #[test]
    fn drain_events_empty() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        let events = renderer.drain_events();
        assert!(events.is_empty());
        handle.shutdown();
    }

    #[test]
    fn fetch_id_monotonic() {
        let a = FetchId::next();
        let b = FetchId::next();
        assert!(b.0 > a.0);
    }

    #[test]
    fn multiple_renderers() {
        let handle = spawn_network_process(test_client());
        let r1 = handle.create_renderer_handle();
        let r2 = handle.create_renderer_handle();
        assert_ne!(r1.client_id(), r2.client_id());
        handle.shutdown();
    }

    #[test]
    fn debug_network_handle() {
        let handle = spawn_network_process(test_client());
        let renderer = handle.create_renderer_handle();
        let debug = format!("{renderer:?}");
        assert!(debug.contains("NetworkHandle"));
        handle.shutdown();
    }
}
