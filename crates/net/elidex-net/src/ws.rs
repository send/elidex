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

/// Maximum time a single WebSocket I/O operation may stay
/// blocked before the worker treats it as a connection failure
/// and emits an abnormal close to the JS side (slot #10.6a
/// Copilot R5 HX23 / HX24 / HX25).  Applies to:
/// - the upgrade-response read inside
///   `tokio_tungstenite::connect_async_tls_with_config` — bounds
///   pre-OPEN handshake hangs (e.g. server accepts the TCP
///   connection but never replies to the upgrade).
/// - each `write.send(msg).await` issued by [`send_frame`] —
///   bounds stuck data sends behind a peer that stopped reading
///   (kernel send buffer full).
/// - the close-frame `write.send` inside [`send_close_frame`]
///   — same hazard as data sends, but for the JS-initiated
///   close path.
///
/// 30 seconds is generous for legitimate slow peers (matches the
/// existing fetch-side `request_timeout` and the close-handshake
/// budget) while still bounding the worst case to a deterministic
/// upper limit instead of OS-level TCP keepalive (which can take
/// minutes to hours to fire).  Without this bound, JS-initiated
/// `WebSocket.close()` against an unresponsive peer would hang
/// the worker indefinitely while still holding the connection's
/// kernel resources.
const WS_OP_TIMEOUT: Duration = Duration::from_secs(30);

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

/// Outcome of [`send_frame`] — distinguishes a normal abort
/// (caller should run the standard close-write path) from a
/// cancel-driven abort (caller should `return` immediately
/// because the broker is tearing the worker down).
enum SendFrameOutcome {
    /// Frame was sent successfully (or the send failed and an
    /// abnormal-close event was already pushed); caller should
    /// continue the main loop.
    Ok,
    /// `write.send` returned an error; caller should `return`
    /// from the worker (abnormal close already emitted).
    SendErr,
    /// The cancel signal fired while the send was in flight or
    /// queued; caller should `return` immediately without
    /// running any further write paths.  The peer may not
    /// observe this frame, which is acceptable because the
    /// broker has already declared the worker dead (slot #10.6a
    /// HX5).
    Cancelled,
}

/// Send a WebSocket frame and update buffered byte tracking.
///
/// The JS layer increments `bufferedAmount` synchronously when `send()` is called.
/// After successful transmission, we decrement by the sent length. On error,
/// the connection is closed (bufferedAmount becomes irrelevant).
///
/// The send is wrapped in a `tokio::select!` against
/// `cancel.cancelled()` so a peer that stops reading TCP
/// (filling the kernel send buffer) cannot block the worker
/// indefinitely.  Without the cancel arm `write.send().await`
/// would await the full TCP timeout (minutes) before
/// `close_all_for_client`'s `thread.join()` could complete
/// (slot #10.6a HX5).
/// Internal outcome of [`send_frame`]'s `tokio::select!` race.
/// Hoisted to the module scope (rather than nested in the
/// function body) to satisfy clippy's
/// `items_after_statements`.
enum SendFrameSelectOutcome {
    Cancelled,
    TimedOut,
    Sent(Result<(), tokio_tungstenite::tungstenite::Error>),
}

async fn send_frame(
    write: &mut (impl futures_util::SinkExt<
        tokio_tungstenite::tungstenite::Message,
        Error = tokio_tungstenite::tungstenite::Error,
    > + Unpin),
    msg: tokio_tungstenite::tungstenite::Message,
    evt_tx: &Sender<WsEvent>,
    cancel: &CancelHandle,
) -> SendFrameOutcome {
    let len = msg.len() as u64;
    // Three-arm race: cancel | timeout | write.  The timeout arm
    // bounds JS-initiated close against a peer that has stopped
    // reading TCP — without it `write.send` would hang for the
    // OS-level keepalive timeout (slot #10.6a Copilot R5 HX24).
    let outcome = tokio::select! {
        biased;
        () = cancel.cancelled() => SendFrameSelectOutcome::Cancelled,
        () = tokio::time::sleep(WS_OP_TIMEOUT) => SendFrameSelectOutcome::TimedOut,
        res = write.send(msg) => SendFrameSelectOutcome::Sent(res),
    };
    match outcome {
        SendFrameSelectOutcome::Cancelled => SendFrameOutcome::Cancelled,
        SendFrameSelectOutcome::TimedOut => {
            let _ = evt_tx.send(WsEvent::Error(format!(
                "send timeout ({}s)",
                WS_OP_TIMEOUT.as_secs()
            )));
            send_abnormal_close(evt_tx);
            SendFrameOutcome::SendErr
        }
        SendFrameSelectOutcome::Sent(Err(_)) => {
            let _ = evt_tx.send(WsEvent::Error("send failed".to_string()));
            send_abnormal_close(evt_tx);
            SendFrameOutcome::SendErr
        }
        SendFrameSelectOutcome::Sent(Ok(())) => {
            let _ = evt_tx.send(WsEvent::BytesSent(len));
            SendFrameOutcome::Ok
        }
    }
}

/// Send a WebSocket Close frame, racing the write against the
/// cancel signal.  Returns `true` if the caller should `return`
/// from the worker because cancel fired during the send;
/// `false` if the send completed (either successfully or with
/// an error — in either case the caller's existing flow handles
/// the next step).  Slot #10.6a HX5 — same rationale as
/// [`send_frame`]: a peer that stops reading the socket would
/// otherwise block this `await` for the full TCP timeout.
async fn send_close_frame(
    write: &mut (impl futures_util::SinkExt<
        tokio_tungstenite::tungstenite::Message,
        Error = tokio_tungstenite::tungstenite::Error,
    > + Unpin),
    code: u16,
    reason: String,
    cancel: &CancelHandle,
) -> bool {
    let frame = tokio_tungstenite::tungstenite::protocol::CloseFrame {
        code: code.into(),
        reason: reason.into(),
    };
    let close_msg = tokio_tungstenite::tungstenite::Message::Close(Some(frame));
    // Three-arm race: cancel | timeout | write.  Same rationale
    // as [`send_frame`]: a peer that has stopped reading would
    // otherwise block this `await` for the OS-level TCP timeout,
    // hanging JS-initiated close indefinitely (slot #10.6a
    // Copilot R5 HX25).  On timeout we treat the close-frame
    // send as failed — the caller's next step is to exit anyway,
    // and the connection is being torn down regardless, so a
    // partial send leaves the worker in the same state as a
    // dropped TCP connection.
    tokio::select! {
        biased;
        () = cancel.cancelled() => true,
        () = tokio::time::sleep(WS_OP_TIMEOUT) => true,
        res = write.send(close_msg) => {
            // Discard the result — close-frame write errors
            // are non-fatal here; the caller's next step is to
            // exit anyway (waiting for reciprocal close on a
            // broken pipe is harmless because the read arm
            // also fails on the same condition).
            let _ = res;
            false
        }
    }
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
    use futures_util::StreamExt;
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
                                              // Cancel-aware handshake with a bounded timeout.  Three-arm
                                              // race:
                                              // - cancel — broker-driven teardown.
                                              // - timeout — bounded handshake wait so a JS
                                              //   `WebSocket.close()` arriving in `cmd_rx` before the
                                              //   upgrade completes still releases the worker within
                                              //   [`WS_OP_TIMEOUT`] (slot #10.6a Copilot R5 HX23).
                                              //   Without this arm a stuck handshake would hold the
                                              //   worker until OS-level TCP keepalive fires
                                              //   (minutes / hours).
                                              // - the connect future itself.
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
        () = tokio::time::sleep(WS_OP_TIMEOUT) => {
            let _ = evt_tx.send(WsEvent::Error(format!(
                "WebSocket handshake timeout ({}s)",
                WS_OP_TIMEOUT.as_secs()
            )));
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
    // `cmd_rx_open` gates the cmd_rx arm of the select.  After
    // the broker drops `command_tx` (`close_all_for_client`'s
    // Phase 1) the channel is permanently disconnected and
    // `cmd_rx.recv()` resolves to `Ready(None)` forever — keeping
    // it in the select would tight-loop the worker.  We observe
    // the disconnect once, transition into "drain reciprocal
    // close" mode by clearing this flag, and let the close
    // handshake play out via the read arm + the close-deadline
    // sleep arm (slot #10.6a Copilot R4 HX18).
    let mut cmd_rx_open = true;

    loop {
        // 30-second close-handshake deadline as a tokio
        // [`Instant`].  Computed every iteration so the sleep
        // arm always points at the right wall-clock moment;
        // the arm only fires when `close_sent` is true (see the
        // `if close_sent` guard below).
        let close_deadline = close_sent_at + Duration::from_secs(30);

        // No `biased;` here: a 3-arm select with biased ordering
        // would let a continuously-readable `read.next()` arm
        // starve `cmd_rx` (and vice versa) under heavy
        // unidirectional traffic, breaking JS-side `send()` /
        // `close()` responsiveness.  Tokio's default fair
        // selection gives every arm an equal probability per
        // poll, which keeps cancel responsive (the cancelled
        // future is always-ready once `cancel.cancel()` fires
        // and wins within bounded polls) without the starvation
        // hazard (slot #10.6a HX3).
        tokio::select! {
            // Broker-driven teardown: skip clean-close negotiation
            // and exit immediately so `close_all_for_client`'s
            // `thread.join()` returns within bounded time even if
            // the peer is silent on `read.next()`.
            () = cancel.cancelled() => {
                send_abnormal_close(&evt_tx);
                return;
            }
            // RFC 6455 §7.1.1: the close handshake has a
            // bounded wait.  Pre-fix the equivalent check ran
            // AFTER the select returned, but if the peer goes
            // silent and `cmd_tx` stays open (the JS-driven
            // `WebSocket.close()` path) all three arms are
            // pending and the loop never re-iterates — the
            // worker would stay stuck in CLOSING forever (slot
            // #10.6a Copilot R4 HX19).  Wiring the deadline as
            // a select arm with `if close_sent` makes the
            // timeout reactive: it never fires before close was
            // initiated and never lets a silent peer hold the
            // worker beyond 30 s after.
            () = tokio::time::sleep_until(close_deadline), if close_sent => {
                let _ = evt_tx.send(WsEvent::Closed {
                    code: 1006,
                    reason: String::new(),
                    was_clean: false,
                });
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
            cmd = cmd_rx.recv(), if cmd_rx_open => {
                match cmd {
                    Some(WsCommand::SendText(text)) => {
                        let msg = tungstenite::Message::Text(text.into());
                        match send_frame(&mut write, msg, &evt_tx, &cancel).await {
                            SendFrameOutcome::Ok => {}
                            SendFrameOutcome::SendErr | SendFrameOutcome::Cancelled => return,
                        }
                    }
                    Some(WsCommand::SendBinary(data)) => {
                        let msg = tungstenite::Message::Binary(data.into());
                        match send_frame(&mut write, msg, &evt_tx, &cancel).await {
                            SendFrameOutcome::Ok => {}
                            SendFrameOutcome::SendErr | SendFrameOutcome::Cancelled => return,
                        }
                    }
                    Some(WsCommand::Close(code, reason)) => {
                        close_sent = true;
                        close_sent_at = tokio::time::Instant::now();
                        if send_close_frame(&mut write, code, reason, &cancel).await {
                            // Cancel fired during the close-frame send;
                            // bail out without further writes.
                            return;
                        }
                        // Continue loop to wait for reciprocal close.
                    }
                    None => {
                        // Channel closed — content thread dropped
                        // sender (typically broker-driven teardown
                        // queueing `WsCommand::Close` and dropping
                        // `cmd_tx` on the next op).  Send the
                        // going-away close frame if we haven't
                        // already, then transition into "drain
                        // reciprocal close" mode by closing the
                        // cmd_rx arm — the next iterations wait for
                        // the peer's reciprocal close (read arm),
                        // the 30 s deadline (sleep arm), or cancel
                        // (slot #10.6a Copilot R4 HX18).  Pre-fix
                        // this branch unconditionally emitted
                        // `Closed{1006, was_clean=false}` and
                        // returned, breaking the close handshake
                        // even for responsive peers.
                        if !close_sent {
                            if send_close_frame(
                                &mut write,
                                1001,
                                "going away".to_string(),
                                &cancel,
                            )
                            .await
                            {
                                // Cancel fired during the close-frame send.
                                send_abnormal_close(&evt_tx);
                                return;
                            }
                            close_sent = true;
                            close_sent_at = tokio::time::Instant::now();
                        }
                        cmd_rx_open = false;
                    }
                }
            }
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
        // Fail-fast: poll `is_finished` against the 1-second
        // deadline rather than calling `join()` directly.  An
        // unconditional `thread.join()` would block forever on
        // a regression and the elapsed-time assertion below
        // would only run AFTER the join returned — i.e. never,
        // until the test harness's overall deadline fired
        // (slot #10.6a Copilot R1 HX6).
        let deadline = started + std::time::Duration::from_secs(1);
        while !thread.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            thread.is_finished(),
            "ws worker did not exit within 1s of cancel + drop — handshake select missing cancel arm?"
        );
        thread.join().expect("worker thread panicked");

        // Tear down the server fixture deterministically.
        // After the worker has exited (asserted above) its TCP
        // socket is closed, so the server's blocking `read`
        // returns Ok(0) and the server thread exits via its
        // disconnect branch.  Assert the join succeeds within a
        // bounded window — failing here would indicate a real
        // fixture-leak regression that would otherwise hide
        // sockets / open resources in the shared test process
        // and silently flake later tests (slot #10.6a Copilot
        // R3 HX12).
        stop.store(true, Ordering::SeqCst);
        let join_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !server.is_finished() && std::time::Instant::now() < join_deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            server.is_finished(),
            "server fixture thread did not exit within 2s — leaking thread + listener resources \
             into subsequent tests; check that the worker actually closed the socket"
        );
        server.join().expect("server fixture thread panicked");
    }

    /// Test fixture for the [`send_frame`] / [`send_close_frame`]
    /// cancel-arm regressions (slot #10.6a Copilot R2 HX8 +
    /// HX9): a [`futures_util::Sink`] whose `poll_ready` /
    /// `poll_flush` / `poll_close` always return `Pending` so
    /// the inner `write.send().await` future never resolves on
    /// its own.  Models a peer that has stopped reading the
    /// TCP socket (kernel send buffer full) without binding a
    /// real TCP stream — the helper's cancel arm is the only
    /// thing that can wake the future.
    struct HangingSink;

    impl futures_util::Sink<tokio_tungstenite::tungstenite::Message> for HangingSink {
        type Error = tokio_tungstenite::tungstenite::Error;

        fn poll_ready(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            // Always pending: the test relies on `cancel.race`
            // being the only resolution.
            std::task::Poll::Pending
        }

        fn start_send(
            self: std::pin::Pin<&mut Self>,
            _item: tokio_tungstenite::tungstenite::Message,
        ) -> Result<(), Self::Error> {
            // Unreachable because `poll_ready` is always
            // `Pending`, but the trait requires a body.
            unreachable!("HangingSink::poll_ready never returns Ready")
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Pending
        }

        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Pending
        }
    }

    /// Slot #10.6a (Copilot R2 HX8) regression: a peer that
    /// stops reading TCP can fill the kernel send buffer such
    /// that `write.send(msg).await` never resolves — without a
    /// cancel arm, the broker's `thread.join()` would block for
    /// the full TCP timeout (minutes).  [`send_frame`] now
    /// races each `write.send` against `cancel.cancelled()`;
    /// this test verifies cancel preempts a pending send within
    /// bounded time and the helper returns
    /// [`SendFrameOutcome::Cancelled`].
    #[tokio::test]
    async fn send_frame_returns_cancelled_when_write_blocks() {
        let mut sink = HangingSink;
        let (evt_tx, _evt_rx) = crossbeam_channel::unbounded::<WsEvent>();
        let cancel = CancelHandle::new();
        let cancel_for_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            cancel_for_trigger.cancel();
        });
        let started = std::time::Instant::now();
        let outcome = send_frame(
            &mut sink,
            tokio_tungstenite::tungstenite::Message::Text("hi".into()),
            &evt_tx,
            &cancel,
        )
        .await;
        let elapsed = started.elapsed();
        assert!(
            matches!(outcome, SendFrameOutcome::Cancelled),
            "expected SendFrameOutcome::Cancelled when cancel fires before write completes, got {outcome:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "send_frame blocked for {elapsed:?} — cancel arm of write.send() select missing?"
        );
    }

    /// Slot #10.6a (Copilot R5 HX24) regression: a peer that
    /// stops reading TCP can fill the kernel send buffer and
    /// keep `write.send(msg).await` blocked indefinitely — even
    /// a JS-initiated `WebSocket.close()` would queue behind
    /// the in-flight send and only complete once the OS-level
    /// TCP keepalive fired (minutes / hours).  [`send_frame`]
    /// adds a [`WS_OP_TIMEOUT`] arm to its `tokio::select!`
    /// race so the worst case is bounded to a deterministic
    /// upper limit.  The test uses `start_paused = true` so the
    /// virtual clock auto-advances to the timeout's wakeup as
    /// soon as the runtime sees all tasks pending — no real
    /// wall-clock time elapses.
    #[tokio::test(start_paused = true)]
    async fn send_frame_returns_send_err_on_write_timeout() {
        let mut sink = HangingSink;
        let (evt_tx, evt_rx) = crossbeam_channel::unbounded::<WsEvent>();
        let cancel = CancelHandle::new();
        let outcome = send_frame(
            &mut sink,
            tokio_tungstenite::tungstenite::Message::Text("hi".into()),
            &evt_tx,
            &cancel,
        )
        .await;
        assert!(
            matches!(outcome, SendFrameOutcome::SendErr),
            "expected SendFrameOutcome::SendErr after WS_OP_TIMEOUT elapses, got {outcome:?}"
        );
        // Worker must have emitted both the timeout-tagged
        // error event and the abnormal Close so JS sees the
        // failure surface, not just silent disconnect.
        let mut saw_timeout_error = false;
        let mut saw_abnormal_close = false;
        while let Ok(evt) = evt_rx.try_recv() {
            match evt {
                WsEvent::Error(msg) if msg.contains("timeout") => saw_timeout_error = true,
                WsEvent::Closed {
                    code: 1006,
                    was_clean: false,
                    ..
                } => saw_abnormal_close = true,
                _ => {}
            }
        }
        assert!(saw_timeout_error, "missing timeout-tagged Error event");
        assert!(saw_abnormal_close, "missing abnormal Close event");
    }

    /// Slot #10.6a (Copilot R2 HX9) regression: same hazard as
    /// [`send_frame_returns_cancelled_when_write_blocks`], but
    /// for the close-frame send path used inside the
    /// `WsCommand::Close` and `cmd_rx == None` branches of
    /// [`ws_io_loop`].  [`send_close_frame`] returns `true` to
    /// signal the caller that cancel preempted the send — the
    /// caller is expected to `return` from the worker without
    /// running further write paths.
    #[tokio::test]
    async fn send_close_frame_returns_true_when_write_blocks() {
        let mut sink = HangingSink;
        let cancel = CancelHandle::new();
        let cancel_for_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            cancel_for_trigger.cancel();
        });
        let started = std::time::Instant::now();
        let cancelled =
            send_close_frame(&mut sink, 1001, "navigated away".to_string(), &cancel).await;
        let elapsed = started.elapsed();
        assert!(
            cancelled,
            "send_close_frame must return true when cancel fires before the close frame is sent"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "send_close_frame blocked for {elapsed:?} — cancel arm of write.send() select missing?"
        );
    }

    /// Slot #10.6a (Copilot R5 HX25) regression: same hazard as
    /// the [`send_frame`] timeout, but for the close-frame send
    /// path.  Without the timeout arm a JS `WebSocket.close()`
    /// against a peer that has stopped reading the socket would
    /// hang in `write.send(close_msg).await` for the OS-level
    /// TCP keepalive (minutes / hours).  [`send_close_frame`]
    /// now races cancel | timeout | write — when the timeout
    /// arm wins it returns `true` so the caller exits the
    /// worker without further write paths, matching the cancel
    /// path's exit semantics.
    #[tokio::test(start_paused = true)]
    async fn send_close_frame_returns_true_on_write_timeout() {
        let mut sink = HangingSink;
        let cancel = CancelHandle::new();
        let cancelled =
            send_close_frame(&mut sink, 1001, "navigated away".to_string(), &cancel).await;
        assert!(
            cancelled,
            "send_close_frame must return true when WS_OP_TIMEOUT elapses with the write still pending"
        );
    }

    impl std::fmt::Debug for SendFrameOutcome {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SendFrameOutcome::Ok => write!(f, "Ok"),
                SendFrameOutcome::SendErr => write!(f, "SendErr"),
                SendFrameOutcome::Cancelled => write!(f, "Cancelled"),
            }
        }
    }
}
