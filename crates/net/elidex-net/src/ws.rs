//! WebSocket I/O thread (RFC 6455).
//!
//! Spawns a dedicated thread with a current-thread tokio runtime for each
//! WebSocket connection. Communication with the content thread uses crossbeam
//! channels (same pattern as OOP iframe threads).

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
#[must_use]
pub fn spawn_ws_thread(url: url::Url, protocols: Vec<String>, origin: String) -> WsHandle {
    let id = WsId::next();

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<WsCommand>();
    let (evt_tx, evt_rx) = crossbeam_channel::bounded::<WsEvent>(256);

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

/// Async WebSocket I/O loop running inside the thread's tokio runtime.
async fn ws_io_loop(
    url: url::Url,
    protocols: Vec<String>,
    origin: String,
    cmd_rx: Receiver<WsCommand>,
    evt_tx: Sender<WsEvent>,
) {
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
            let _ = evt_tx.send(WsEvent::Error(format!("invalid WebSocket request: {e}")));
            return;
        }
    };

    // Perform the WebSocket handshake.
    let (ws_stream, response) = match tokio_tungstenite::connect_async(request).await {
        Ok(pair) => pair,
        Err(e) => {
            let _ = evt_tx.send(WsEvent::Error(format!("WebSocket handshake failed: {e}")));
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
    use futures_util::{SinkExt, StreamExt};
    let (mut write, mut read) = ws_stream.split();
    // Type annotation for the timeout result to help inference.
    type WsReadResult = Option<
        Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>,
    >;

    let mut close_sent = false;
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
                match msg {
                    tungstenite::Message::Text(text) => {
                        if evt_tx.send(WsEvent::TextMessage(text.to_string())).is_err() {
                            return;
                        }
                    }
                    tungstenite::Message::Binary(data) => {
                        if evt_tx.send(WsEvent::BinaryMessage(data.to_vec())).is_err() {
                            return;
                        }
                    }
                    tungstenite::Message::Close(frame) => {
                        let (code, reason) = frame
                            .map(|f| (f.code.into(), f.reason.to_string()))
                            .unwrap_or((1005, String::new()));
                        let _ = evt_tx.send(WsEvent::Closed {
                            code,
                            reason,
                            was_clean: true,
                        });
                        return;
                    }
                    tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => {
                        // Ping/pong handled automatically by tungstenite.
                    }
                    _ => {}
                }
            }
            Ok(Some(Err(e))) => {
                let _ = evt_tx.send(WsEvent::Error(format!("WebSocket error: {e}")));
                let _ = evt_tx.send(WsEvent::Closed {
                    code: 1006,
                    reason: String::new(),
                    was_clean: false,
                });
                return;
            }
            Ok(None) => {
                // Stream ended (server closed connection without close frame).
                let _ = evt_tx.send(WsEvent::Closed {
                    code: 1006,
                    reason: String::new(),
                    was_clean: false,
                });
                return;
            }
            Err(_) => {
                // Timeout — no data from server this iteration.
            }
        }

        // Check for commands from the content thread.
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                WsCommand::SendText(text) => {
                    let len = text.len() as u64;
                    buffered_bytes += len;
                    if write
                        .send(tungstenite::Message::Text(text.into()))
                        .await
                        .is_err()
                    {
                        let _ = evt_tx.send(WsEvent::Error("send failed".to_string()));
                        let _ = evt_tx.send(WsEvent::Closed {
                            code: 1006,
                            reason: String::new(),
                            was_clean: false,
                        });
                        return;
                    }
                    buffered_bytes = buffered_bytes.saturating_sub(len);
                    let _ = evt_tx.send(WsEvent::BufferedAmountUpdate(buffered_bytes));
                }
                WsCommand::SendBinary(data) => {
                    let len = data.len() as u64;
                    buffered_bytes += len;
                    if write
                        .send(tungstenite::Message::Binary(data.into()))
                        .await
                        .is_err()
                    {
                        let _ = evt_tx.send(WsEvent::Error("send failed".to_string()));
                        let _ = evt_tx.send(WsEvent::Closed {
                            code: 1006,
                            reason: String::new(),
                            was_clean: false,
                        });
                        return;
                    }
                    buffered_bytes = buffered_bytes.saturating_sub(len);
                    let _ = evt_tx.send(WsEvent::BufferedAmountUpdate(buffered_bytes));
                }
                WsCommand::Close(code, reason) => {
                    close_sent = true;
                    let frame = tungstenite::protocol::CloseFrame {
                        code: code.into(),
                        reason: reason.into(),
                    };
                    let _ = write.send(tungstenite::Message::Close(Some(frame))).await;
                    // Continue loop to wait for reciprocal close.
                }
            }
        }

        // Check if command channel is disconnected (content thread dropped sender).
        if cmd_rx.is_empty() && evt_tx.is_empty() {
            // Both channels may still be alive; only break if cmd channel disconnected.
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
}
