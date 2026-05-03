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

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::Sender;
use tokio::sync::mpsc;

use crate::CancelHandle;

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
    /// The cancel arm of `ws_io_loop`'s `tokio::select!` aborts
    /// the read future immediately so the broker's `thread.join()`
    /// returns deterministically (slot #10.6a follow-up to PR #142
    /// HCau / HCv / HJTV / HKhZ).
    pub cancel: CancelHandle,
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
        rt.block_on(ws_io_loop(
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

/// Send an abnormal-close error event.
///
/// Uses blocking `send` because close events are critical and must not be dropped.
fn send_abnormal_close(evt_tx: &Sender<WsEvent>) {
    let _ = evt_tx.send(WsEvent::Closed {
        code: 1006,
        reason: String::new(),
        was_clean: false,
    });
}

/// Handle an incoming WebSocket message in normal (non-closing) state.
///
/// Returns `true` if the caller should return from the I/O loop.
fn handle_ws_message(
    msg: tokio_tungstenite::tungstenite::Message,
    evt_tx: &Sender<WsEvent>,
) -> bool {
    use tokio_tungstenite::tungstenite;

    match msg {
        tungstenite::Message::Text(text) => {
            evt_tx.send(WsEvent::TextMessage(text.to_string())).is_err()
        }
        tungstenite::Message::Binary(data) => {
            evt_tx.send(WsEvent::BinaryMessage(data.to_vec())).is_err()
        }
        tungstenite::Message::Close(frame) => {
            let (code, reason) = frame.map_or((1005, String::new()), |f| {
                (f.code.into(), f.reason.to_string())
            });
            // Close events are critical — use blocking send to guarantee delivery.
            let _ = evt_tx.send(WsEvent::Closed {
                code,
                reason,
                was_clean: true,
            });
            true
        }
        // Ping/pong handled automatically by tungstenite; Frame is internal.
        tungstenite::Message::Ping(_)
        | tungstenite::Message::Pong(_)
        | tungstenite::Message::Frame(_) => false,
    }
}

/// Send a WebSocket frame and update buffered byte tracking.
///
/// The JS layer increments `bufferedAmount` synchronously when `send()` is called.
/// After successful transmission, we decrement by the sent length. On error,
/// the connection is closed (bufferedAmount becomes irrelevant).
///
/// Returns `true` if a send error occurred (caller should abort the loop).
async fn send_frame(
    write: &mut (impl futures_util::SinkExt<
        tokio_tungstenite::tungstenite::Message,
        Error = tokio_tungstenite::tungstenite::Error,
    > + Unpin),
    msg: tokio_tungstenite::tungstenite::Message,
    evt_tx: &Sender<WsEvent>,
) -> bool {
    let len = msg.len() as u64;
    if write.send(msg).await.is_err() {
        // Error events precede close and must not be lost — use blocking send.
        let _ = evt_tx.send(WsEvent::Error("send failed".to_string()));
        send_abnormal_close(evt_tx);
        return true;
    }
    // BytesSent is infrequent (once per send) — use blocking send to guarantee delivery.
    let _ = evt_tx.send(WsEvent::BytesSent(len));
    false
}

/// Async WebSocket I/O loop running inside the thread's tokio runtime.
#[allow(clippy::too_many_lines)]
async fn ws_io_loop(
    url: url::Url,
    protocols: Vec<String>,
    origin: String,
    mut cmd_rx: mpsc::UnboundedReceiver<WsCommand>,
    evt_tx: Sender<WsEvent>,
    cancel: CancelHandle,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite;

    // Build the HTTP request for the WebSocket handshake.
    let mut request = tungstenite::http::Request::builder()
        .uri(url.as_str())
        .header("Origin", &origin);

    if !protocols.is_empty() {
        request = request.header("Sec-WebSocket-Protocol", protocols.join(", "));
    }

    let request = match request.body(()) {
        Ok(r) => r,
        Err(e) => {
            // Error events precede close and must not be lost — use blocking send.
            let _ = evt_tx.send(WsEvent::Error(format!("invalid WebSocket request: {e}")));
            send_abnormal_close(&evt_tx);
            return;
        }
    };

    // Perform the WebSocket handshake with message size limits.
    let mut ws_config = tungstenite::protocol::WebSocketConfig::default();
    ws_config.max_message_size = Some(4 << 20); // 4 MiB
    ws_config.max_frame_size = Some(1 << 20); // 1 MiB
                                              // Cancel-aware handshake: a server that accepts the TCP
                                              // connection but never replies to the upgrade request would
                                              // otherwise stall the worker for the full TCP-stack timeout
                                              // (minutes).  `tokio::select!` aborts `connect_async_tls_with_config`
                                              // immediately on cancel so the broker's join completes (slot #10.6a).
    let connect_fut = tokio_tungstenite::connect_async_tls_with_config(
        request,
        Some(ws_config),
        false, // disable_nagle
        None,  // TLS connector (uses default rustls)
    );
    let (ws_stream, response) = tokio::select! {
        biased;
        () = cancel.cancelled() => {
            send_abnormal_close(&evt_tx);
            return;
        }
        res = connect_fut => match res {
            Ok(pair) => pair,
            Err(e) => {
                // Error events precede close and must not be lost — use blocking send.
                let _ = evt_tx.send(WsEvent::Error(format!("WebSocket handshake failed: {e}")));
                send_abnormal_close(&evt_tx);
                return;
            }
        }
    };

    // Extract negotiated protocol and extensions from the response.
    let protocol = response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let extensions = response
        .headers()
        .get("Sec-WebSocket-Extensions")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if evt_tx
        .send(WsEvent::Connected {
            protocol,
            extensions,
        })
        .is_err()
    {
        return; // Content thread dropped receiver.
    }

    // Split the stream into read and write halves.
    let (mut write, mut read) = ws_stream.split();

    let mut close_sent = false;
    let mut close_sent_at = tokio::time::Instant::now();

    loop {
        tokio::select! {
            biased;
            // Broker-driven teardown: skip clean-close negotiation
            // and exit immediately so `close_all_for_client`'s
            // `thread.join()` returns within bounded time even if
            // the peer is silent on `read.next()` (slot #10.6a).
            () = cancel.cancelled() => {
                send_abnormal_close(&evt_tx);
                return;
            }
            ws_msg = read.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if close_sent {
                            // Discard data frames after close frame sent (RFC 6455 §5.5.1).
                            if msg.is_close() {
                                // Received reciprocal close — connection cleanly closed.
                                let (code, reason) = extract_close_data(&msg);
                                // Close events are critical — use blocking send.
                                let _ = evt_tx.send(WsEvent::Closed {
                                    code,
                                    reason,
                                    was_clean: true,
                                });
                                return;
                            }
                            continue;
                        }
                        if handle_ws_message(msg, &evt_tx) {
                            return;
                        }
                    }
                    Some(Err(e)) => {
                        // Error events precede close and must not be lost — use blocking send.
                        let _ = evt_tx.send(WsEvent::Error(format!("WebSocket error: {e}")));
                        send_abnormal_close(&evt_tx);
                        return;
                    }
                    None => {
                        // Stream ended (server closed connection without close frame).
                        send_abnormal_close(&evt_tx);
                        return;
                    }
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(WsCommand::SendText(text)) => {
                        let msg = tungstenite::Message::Text(text.into());
                        if send_frame(&mut write, msg, &evt_tx).await {
                            return;
                        }
                    }
                    Some(WsCommand::SendBinary(data)) => {
                        let msg = tungstenite::Message::Binary(data.into());
                        if send_frame(&mut write, msg, &evt_tx).await {
                            return;
                        }
                    }
                    Some(WsCommand::Close(code, reason)) => {
                        close_sent = true;
                        close_sent_at = tokio::time::Instant::now();
                        let frame = tungstenite::protocol::CloseFrame {
                            code: code.into(),
                            reason: reason.into(),
                        };
                        let _ = write.send(tungstenite::Message::Close(Some(frame))).await;
                        // Continue loop to wait for reciprocal close.
                    }
                    None => {
                        // Channel closed — content thread dropped sender.
                        if !close_sent {
                            let frame = tungstenite::protocol::CloseFrame {
                                code: 1001u16.into(),
                                reason: "going away".into(),
                            };
                            let _ = write.send(tungstenite::Message::Close(Some(frame))).await;
                        }
                        send_abnormal_close(&evt_tx);
                        return;
                    }
                }
            }
        }

        // Close handshake timeout: if 30s have elapsed since we sent a close frame
        // and the server hasn't responded, treat as unclean close.
        if close_sent && close_sent_at.elapsed() > Duration::from_secs(30) {
            // Close events are critical — use blocking send.
            let _ = evt_tx.send(WsEvent::Closed {
                code: 1006,
                reason: String::new(),
                was_clean: false,
            });
            return;
        }
    }
}

/// Extract close code and reason from a Close message.
fn extract_close_data(msg: &tokio_tungstenite::tungstenite::Message) -> (u16, String) {
    if let tokio_tungstenite::tungstenite::Message::Close(Some(frame)) = msg {
        (frame.code.into(), frame.reason.to_string())
    } else {
        (1005, String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_id_unique() {
        let a = WsId::next();
        let b = WsId::next();
        assert_ne!(a, b);
    }

    #[test]
    fn ws_id_copy_clone() {
        let a = WsId::next();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn ws_command_debug() {
        let cmd = WsCommand::SendText("hello".to_string());
        let debug = format!("{cmd:?}");
        assert!(debug.contains("SendText"));
    }

    #[test]
    fn ws_event_debug() {
        let evt = WsEvent::Connected {
            protocol: "chat".to_string(),
            extensions: String::new(),
        };
        let debug = format!("{evt:?}");
        assert!(debug.contains("Connected"));
    }

    #[test]
    fn ws_event_closed() {
        let evt = WsEvent::Closed {
            code: 1000,
            reason: "normal".to_string(),
            was_clean: true,
        };
        if let WsEvent::Closed {
            code,
            reason,
            was_clean,
        } = evt
        {
            assert_eq!(code, 1000);
            assert_eq!(reason, "normal");
            assert!(was_clean);
        } else {
            panic!("expected Closed");
        }
    }

    #[test]
    fn ws_event_buffered_amount() {
        let evt = WsEvent::BytesSent(42);
        if let WsEvent::BytesSent(n) = evt {
            assert_eq!(n, 42);
        } else {
            panic!("expected BytesSent");
        }
    }

    #[test]
    fn ws_event_text_message() {
        let evt = WsEvent::TextMessage("hello".to_string());
        if let WsEvent::TextMessage(data) = evt {
            assert_eq!(data, "hello");
        } else {
            panic!("expected TextMessage");
        }
    }

    #[test]
    fn ws_event_binary_message() {
        let evt = WsEvent::BinaryMessage(vec![1, 2, 3]);
        if let WsEvent::BinaryMessage(data) = evt {
            assert_eq!(data, vec![1, 2, 3]);
        } else {
            panic!("expected BinaryMessage");
        }
    }

    /// Slot #10.6a regression: the WebSocket I/O thread must
    /// observe a [`CancelHandle::cancel`] AND the dropped
    /// `command_tx` and exit within bounded time, even when the
    /// peer is silent post-`accept` and the worker is parked
    /// inside `tokio_tungstenite::connect_async`'s response
    /// read.  Pre-fix the handshake await had no cancel arm —
    /// dropping `command_tx` was invisible because the worker
    /// hadn't reached its post-handshake `tokio::select!` yet,
    /// and the broker's `close_all_for_client` removed the
    /// `WsHandle` while the worker was still alive (detached
    /// thread that survived `np.shutdown()` until TCP keepalive
    /// timed out).  Post-fix the handshake's `tokio::select!`
    /// against `cancel.cancelled()` aborts the read future
    /// immediately.  This is the same teardown sequence
    /// `crate::broker::dispatch::NetworkProcessState::close_all_for_client`
    /// drives at the broker level.
    ///
    /// Verified by binding a `TcpListener` that accepts the
    /// connection and never replies — the worker is parked in
    /// the response-read state.  We then trigger
    /// `handle.cancel.cancel()` + `drop(handle.command_tx)` and
    /// `handle.thread.take().unwrap().join()` within a 1-second
    /// deadline.  A regression that left the cancel arm out
    /// would block the join until the test's `.expect("...")`
    /// timeout fires (or, in `panic = "abort"` builds, until
    /// the OS kills the thread on process exit).
    #[test]
    fn ws_worker_exits_on_cancel_during_silent_handshake() {
        use std::io::Read;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_server = Arc::clone(&stop);

        let server = std::thread::spawn(move || {
            // Single connection only — the test client makes
            // exactly one connect attempt.
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut buf = [0u8; 4096];
            // Read and discard the upgrade request, then go
            // silent — the worker is now parked on response
            // read and will never get a reply from us.
            let _ = stream.read(&mut buf);
            // Block on read until client disconnects (Ok(0))
            // OR until the test signals stop (via shutdown
            // + drop), at which point we just exit.
            while !stop_for_server.load(Ordering::SeqCst) {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => return, // client gone
                    Ok(_) => {}               // stray data — keep waiting
                }
            }
        });

        let url = url::Url::parse(&format!("ws://127.0.0.1:{}/", addr.port())).unwrap();
        let mut handle = spawn_ws_thread(url, Vec::new(), "http://example.com".to_string());

        // Yield so the worker reaches the response-read await
        // (TCP connect + request write happen synchronously
        // on loopback; the read await is where cancel will
        // bite).
        std::thread::sleep(std::time::Duration::from_millis(80));

        let started = std::time::Instant::now();
        handle.cancel.cancel();
        drop(handle.command_tx);
        let thread = handle.thread.take().expect("thread set on spawn");
        thread.join().expect("worker thread panicked");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "ws worker did not exit within 1s of cancel + drop — handshake select missing cancel arm?"
        );

        // Tear down the server fixture deterministically.
        stop.store(true, Ordering::SeqCst);
        // The server may still be parked on `read` here; close
        // the listener implicitly by joining (server thread
        // returns on next read iteration).  Use a short deadline
        // so a hung server thread doesn't block the test forever.
        let join_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !server.is_finished() && std::time::Instant::now() < join_deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if server.is_finished() {
            let _ = server.join();
        }
        // If server is still alive past the deadline, leak it
        // (test still passes — the worker assertion above is
        // the load-bearing one).  Avoids cross-test flake from
        // server-side blocking reads that occasionally outlive
        // their owner.
    }
}
