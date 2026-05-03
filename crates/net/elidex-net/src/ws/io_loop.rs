//! Async I/O loop + per-frame helpers for [`super::WsHandle`].
//!
//! Split out of `ws.rs` in slot #10.6a Copilot R6 HX26 once the
//! file crossed the project's 1000-line convention.  All items
//! are `pub(super)` so the [`super::tests`] module can exercise
//! the cancel / timeout arms directly without exposing them
//! outside the crate.

use std::time::Duration;

use crossbeam_channel::Sender;
use tokio::sync::mpsc;

use super::{WsCommand, WsEvent, WS_OP_TIMEOUT};
use crate::CancelHandle;

/// Send an abnormal-close error event.
///
/// Uses blocking `send` because close events are critical and must not be dropped.
pub(super) fn send_abnormal_close(evt_tx: &Sender<WsEvent>) {
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
pub(super) enum SendFrameOutcome {
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

/// Internal outcome of [`send_frame`]'s `tokio::select!` race.
/// Hoisted to the module scope (rather than nested in the
/// function body) to satisfy clippy's
/// `items_after_statements`.
enum SendFrameSelectOutcome {
    Cancelled,
    TimedOut,
    Sent(Result<(), tokio_tungstenite::tungstenite::Error>),
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
pub(super) async fn send_frame(
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
pub(super) async fn send_close_frame(
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
pub(super) async fn ws_io_loop(
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
                            // Cancel fired OR `WS_OP_TIMEOUT`
                            // elapsed during the close-frame
                            // send.  Either way we exit the
                            // worker without further writes —
                            // emit the abnormal-close event
                            // first so the JS-side
                            // `WebSocket.close()` Promise / event
                            // surface still observes a
                            // terminal `Closed` and the readyState
                            // can transition out of CLOSING
                            // (slot #10.6a Copilot R6 HX27).
                            send_abnormal_close(&evt_tx);
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
