//! WebSocket I/O thread (RFC 6455).
//!
//! Spawns a dedicated thread with a current-thread tokio runtime for each
//! WebSocket connection. Both channels are unbounded per WHATWG spec:
//! Commands use `tokio::sync::mpsc` unbounded (JS `send()` must not block, §9.3.1).
//! Events use crossbeam unbounded (messages must not be dropped, §9.3.2).
//! Memory is bounded by the 4 MiB `max_message_size` + TCP backpressure.
//!
//! **Architecture note**: In M4-7 (Sandbox Hardening), `spawn_ws_thread` will
//! migrate from direct thread spawning to Network Process IPC. The JS API layer
//! and drain logic are unchanged (channel abstraction is the same).
//!
//! ## Module layout (slot #10.6a Copilot R6 HX26 split)
//!
//! - this file (`ws/mod.rs`) — public surface: handle / event / command
//!   types, `spawn_ws_thread`, the `WS_OP_TIMEOUT` constant.
//! - `io_loop` — async I/O loop + per-frame send helpers
//!   (`send_frame` / `send_close_frame`) + handshake setup.
//!   `pub(super)` items are exposed to the tests module but not
//!   outside the crate.
//! - `tests` (`#[cfg(test)]`) — all unit + cancel-arm regression
//!   tests, including the `HangingSink` fixture used by the
//!   timeout-arm tests.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::CancelHandle;

mod io_loop;

#[cfg(test)]
mod tests;

/// Maximum time a single WebSocket I/O operation may stay
/// blocked before the worker treats it as a connection failure
/// and emits an abnormal close to the JS side (slot #10.6a
/// Copilot R5 HX23 / HX24 / HX25).  Applies to:
/// - the upgrade-response read inside
///   `tokio_tungstenite::connect_async_tls_with_config` — bounds
///   pre-OPEN handshake hangs (e.g. server accepts the TCP
///   connection but never replies to the upgrade).
/// - each `write.send(msg).await` issued by
///   `io_loop::send_frame` — bounds stuck data sends behind a
///   peer that stopped reading (kernel send buffer full).
/// - the close-frame `write.send` inside
///   `io_loop::send_close_frame` — same hazard as data sends,
///   but for the JS-initiated close path.
///
/// (The two helpers are intentionally module-private — see
/// [`WsHandle::cancel`] for the rationale on broker-only
/// command-channel short-circuits — so rustdoc renders them as
/// inline code rather than intra-doc links per the project's
/// strict `-D rustdoc::private_intra_doc_links` policy, slot
/// #10.6a Copilot R7 HX29.)
///
/// 30 seconds is generous for legitimate slow peers (matches the
/// existing fetch-side `request_timeout` and the close-handshake
/// budget) while still bounding the worst case to a deterministic
/// upper limit instead of OS-level TCP keepalive (which can take
/// minutes to hours to fire).  Without this bound, JS-initiated
/// `WebSocket.close()` against an unresponsive peer would hang
/// the worker indefinitely while still holding the connection's
/// kernel resources.
pub(super) const WS_OP_TIMEOUT: Duration = Duration::from_secs(30);

/// Unique WebSocket connection identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WsId(pub u64);

static WS_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl WsId {
    /// Generate a new unique WebSocket ID.
    #[must_use]
    pub fn next() -> Self {
        Self(WS_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Commands from the content thread to the WebSocket I/O thread.
#[derive(Debug)]
pub enum WsCommand {
    /// Send a text frame.
    SendText(String),
    /// Send a binary frame.
    SendBinary(Vec<u8>),
    /// Initiate close handshake with code + reason.
    Close(u16, String),
}

/// Events from the WebSocket I/O thread to the content thread.
#[derive(Debug)]
pub enum WsEvent {
    /// Connection established successfully.
    Connected {
        /// Negotiated sub-protocol from `Sec-WebSocket-Protocol` header.
        protocol: String,
        /// Negotiated extensions from `Sec-WebSocket-Extensions` header.
        extensions: String,
    },
    /// Text message received.
    TextMessage(String),
    /// Binary message received.
    BinaryMessage(Vec<u8>),
    /// Connection closed.
    Closed {
        /// Close code (RFC 6455 §7.4).
        code: u16,
        /// Close reason string.
        reason: String,
        /// Whether the close handshake completed cleanly.
        was_clean: bool,
    },
    /// Connection error.
    Error(String),
    /// Number of bytes successfully transmitted (decrement `bufferedAmount` by this).
    ///
    /// The JS layer increments `bufferedAmount` synchronously in `send()`.
    /// The I/O thread sends this event after each successful frame transmission
    /// so the content thread can decrement by the same amount.
    BytesSent(u64),
}

/// Handle to a running WebSocket I/O thread.
pub struct WsHandle {
    /// Unique connection identifier.
    pub id: WsId,
    /// Send commands to the I/O thread (unbounded — JS `send()` must not block).
    pub command_tx: mpsc::UnboundedSender<WsCommand>,
    /// Receive events from the I/O thread.
    pub event_rx: crossbeam_channel::Receiver<WsEvent>,
    /// Thread join handle.
    pub thread: Option<JoinHandle<()>>,
    /// Cooperative cancellation signal.  Triggered by the broker
    /// during teardown ([`crate::broker::NetworkProcessHandle::shutdown`]
    /// / [`crate::broker::NetworkProcessHandle::unregister_renderer`])
    /// so the worker exits within bounded time even if the
    /// underlying socket is stuck on a never-completing read
    /// (server alive but silent post-handshake).  Without this
    /// fallback, a worker stuck inside `read.next().await` would
    /// only observe a closed `command_tx` after its own select
    /// woke from the read — which never happens against a silent
    /// peer until TCP keepalive eventually times out (minutes).
    /// The cancel arm of `io_loop::ws_io_loop`'s
    /// `tokio::select!` aborts the read future immediately so
    /// the broker's `thread.join()` returns deterministically
    /// (slot #10.6a follow-up to PR #142 HCau / HCv / HJTV /
    /// HKhZ).
    ///
    /// Crate-private: the cancel signal short-circuits the
    /// command/event flow, which is broker-only behaviour.
    /// Downstream callers must terminate workers via the
    /// documented `WsCommand::Close` path so they observe the
    /// usual terminal `WsEvent::Closed` sequence (slot #10.6a
    /// Copilot R3 HX16).
    pub(crate) cancel: CancelHandle,
}

/// Spawn a WebSocket I/O thread.
///
/// Creates a background thread that performs the WebSocket handshake and
/// enters a message loop. Events are sent back via the crossbeam channel.
///
/// # Security
///
/// The caller is responsible for SSRF validation (`validate_url`) and
/// mixed-content blocking (`ws://` from `https://`) before calling this.
///
/// **Note**: DNS rebinding attacks are not fully mitigated because
/// `tokio-tungstenite::connect_async` performs DNS resolution internally.
/// The resolved IP is not re-validated against private address ranges.
/// Full mitigation requires a custom connector (deferred to M4-7 Network Process).
#[must_use]
pub fn spawn_ws_thread(url: url::Url, protocols: Vec<String>, origin: String) -> WsHandle {
    let id = WsId::next();

    // Command channel: unbounded per WHATWG §9.3.1 — send() must not block JS.
    // Memory is bounded by the 4MiB max_message_size on the WS config.
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<WsCommand>();
    // Event channel: unbounded per WHATWG §9.3.2 — messages must not be dropped.
    // Memory is bounded by the 4MiB max_message_size + TCP backpressure.
    let (evt_tx, evt_rx) = crossbeam_channel::unbounded::<WsEvent>();

    let cancel = CancelHandle::new();
    let worker_cancel = cancel.clone();
    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for WebSocket");
        rt.block_on(io_loop::ws_io_loop(
            url,
            protocols,
            origin,
            cmd_rx,
            evt_tx,
            worker_cancel,
        ));
    });

    WsHandle {
        id,
        command_tx: cmd_tx,
        event_rx: evt_rx,
        thread: Some(thread),
        cancel,
    }
}
