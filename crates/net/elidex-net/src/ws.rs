//! WebSocket I/O thread (RFC 6455).
//!
//! Spawns a dedicated thread with a current-thread tokio runtime for each
//! WebSocket connection. Communication with the content thread uses crossbeam
//! channels (same pattern as OOP iframe threads).
//!
//! **Architecture note**: In M4-7 (Sandbox Hardening), `spawn_ws_thread` will
//! migrate from direct thread spawning to Network Process IPC. The JS API layer
//! and drain logic are unchanged (channel abstraction is the same).

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

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
    /// Updated buffered byte count after a send completes.
    BufferedAmountUpdate(u64),
}

/// Handle to a running WebSocket I/O thread.
pub struct WsHandle {
    /// Unique connection identifier.
    pub id: WsId,
    /// Send commands to the I/O thread.
    pub command_tx: Sender<WsCommand>,
    /// Receive events from the I/O thread.
    pub event_rx: Receiver<WsEvent>,
    /// Thread join handle.
    pub thread: Option<JoinHandle<()>>,
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

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<WsCommand>();
    let (evt_tx, evt_rx) = crossbeam_channel::bounded::<WsEvent>(64);

    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for WebSocket");
        rt.block_on(ws_io_loop(url, protocols, origin, cmd_rx, evt_tx));
    });

    WsHandle {
        id,
        command_tx: cmd_tx,
        event_rx: evt_rx,
        thread: Some(thread),
    }
}

/// Send an abnormal-close error event.
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
    buffered_bytes: &mut u64,
    evt_tx: &Sender<WsEvent>,
) -> bool {
    let len = msg.len() as u64;
    *buffered_bytes += len;
    if write.send(msg).await.is_err() {
        let _ = evt_tx.send(WsEvent::Error("send failed".to_string()));
        send_abnormal_close(evt_tx);
        return true;
    }
    *buffered_bytes = buffered_bytes.saturating_sub(len);
    let _ = evt_tx.send(WsEvent::BufferedAmountUpdate(*buffered_bytes));
    false
}

/// Async WebSocket I/O loop running inside the thread's tokio runtime.
#[allow(clippy::too_many_lines)]
async fn ws_io_loop(
    url: url::Url,
    protocols: Vec<String>,
    origin: String,
    cmd_rx: Receiver<WsCommand>,
    evt_tx: Sender<WsEvent>,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite;

    type WsReadResult = Option<
        Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>,
    >;

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
            let _ = evt_tx.send(WsEvent::Error(format!("invalid WebSocket request: {e}")));
            send_abnormal_close(&evt_tx);
            return;
        }
    };

    // Perform the WebSocket handshake with message size limits.
    let mut ws_config = tungstenite::protocol::WebSocketConfig::default();
    ws_config.max_message_size = Some(4 << 20); // 4 MiB
    ws_config.max_frame_size = Some(1 << 20); // 1 MiB
    let (ws_stream, response) = match tokio_tungstenite::connect_async_tls_with_config(
        request,
        Some(ws_config),
        false, // disable_nagle
        None,  // TLS connector (uses default rustls)
    )
    .await
    {
        Ok(pair) => pair,
        Err(e) => {
            let _ = evt_tx.send(WsEvent::Error(format!("WebSocket handshake failed: {e}")));
            send_abnormal_close(&evt_tx);
            return;
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
    let mut buffered_bytes: u64 = 0;

    loop {
        // Poll WebSocket stream with 1ms timeout, then check commands.
        let ws_msg: Result<WsReadResult, _> =
            tokio::time::timeout(Duration::from_millis(1), read.next()).await;

        match ws_msg {
            Ok(Some(Ok(msg))) => {
                if close_sent {
                    // Discard data frames after close frame sent (RFC 6455 §5.5.1).
                    if msg.is_close() {
                        // Received reciprocal close — connection cleanly closed.
                        let (code, reason) = extract_close_data(&msg);
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
            Ok(Some(Err(e))) => {
                let _ = evt_tx.send(WsEvent::Error(format!("WebSocket error: {e}")));
                send_abnormal_close(&evt_tx);
                return;
            }
            Ok(None) => {
                // Stream ended (server closed connection without close frame).
                send_abnormal_close(&evt_tx);
                return;
            }
            Err(_) => {
                // Timeout — no data from server this iteration.
            }
        }

        // Close handshake timeout: if 30s have elapsed since we sent a close frame
        // and the server hasn't responded, treat as unclean close.
        if close_sent && close_sent_at.elapsed() > Duration::from_secs(30) {
            let _ = evt_tx.send(WsEvent::Closed {
                code: 1006,
                reason: String::new(),
                was_clean: false,
            });
            return;
        }

        // Check for commands from the content thread.
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => match cmd {
                    WsCommand::SendText(text) => {
                        let msg = tungstenite::Message::Text(text.into());
                        if send_frame(&mut write, msg, &mut buffered_bytes, &evt_tx).await {
                            return;
                        }
                    }
                    WsCommand::SendBinary(data) => {
                        let msg = tungstenite::Message::Binary(data.into());
                        if send_frame(&mut write, msg, &mut buffered_bytes, &evt_tx).await {
                            return;
                        }
                    }
                    WsCommand::Close(code, reason) => {
                        close_sent = true;
                        close_sent_at = tokio::time::Instant::now();
                        let frame = tungstenite::protocol::CloseFrame {
                            code: code.into(),
                            reason: reason.into(),
                        };
                        let _ = write.send(tungstenite::Message::Close(Some(frame))).await;
                        // Continue loop to wait for reciprocal close.
                    }
                },
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // Content thread dropped the command sender — close and exit.
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
        let evt = WsEvent::BufferedAmountUpdate(42);
        if let WsEvent::BufferedAmountUpdate(n) = evt {
            assert_eq!(n, 42);
        } else {
            panic!("expected BufferedAmountUpdate");
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
}
