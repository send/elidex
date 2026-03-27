//! Server-Sent Events I/O thread (WHATWG HTML §9.2).
//!
//! Spawns a dedicated thread with a current-thread tokio runtime for each
//! `EventSource` connection. Handles auto-reconnection with `retry` delay.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

use crate::cookie_jar::CookieJar;

/// Unique SSE connection identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SseId(pub u64);

static SSE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl SseId {
    /// Generate a new unique SSE ID.
    #[must_use]
    pub fn next() -> Self {
        Self(SSE_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Events from the SSE I/O thread to the content thread.
#[derive(Clone, Debug)]
pub enum SseEvent {
    /// HTTP connection established with `text/event-stream` content type.
    Connected,
    /// Server-sent event received.
    Event {
        /// Event type (default `"message"`).
        event_type: String,
        /// Event data (may be multi-line, joined with `\n`).
        data: String,
        /// Last event ID (sticky across events per §9.2.6).
        last_event_id: String,
    },
    /// Recoverable error (network failure). Auto-reconnect after retry delay.
    Error(String),
    /// Fatal error (HTTP non-200, wrong Content-Type). No reconnect.
    FatalError(String),
}

/// Commands from the content thread to the SSE I/O thread.
#[derive(Debug)]
pub enum SseCommand {
    /// Close the connection (no auto-reconnect).
    Close,
}

/// Handle to a running SSE I/O thread.
pub struct SseHandle {
    /// Unique connection identifier.
    pub id: SseId,
    /// Send commands to the I/O thread.
    pub command_tx: Sender<SseCommand>,
    /// Receive events from the I/O thread.
    pub event_rx: Receiver<SseEvent>,
    /// Thread join handle.
    pub thread: Option<JoinHandle<()>>,
}

/// Spawn an SSE I/O thread.
///
/// Creates a background thread that makes an HTTP GET request with
/// `Accept: text/event-stream` and parses the SSE stream. Auto-reconnects
/// on network errors with the configured retry delay.
///
/// # Security
///
/// The caller is responsible for SSRF validation before calling this.
#[must_use]
pub fn spawn_sse_thread(
    url: url::Url,
    with_credentials: bool,
    last_event_id: Option<String>,
    cookie_jar: Option<Arc<CookieJar>>,
) -> SseHandle {
    let id = SseId::next();

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SseCommand>();
    let (evt_tx, evt_rx) = crossbeam_channel::bounded::<SseEvent>(256);

    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for SSE");
        rt.block_on(sse_io_loop(
            url,
            with_credentials,
            last_event_id,
            cookie_jar,
            cmd_rx,
            evt_tx,
        ));
    });

    SseHandle {
        id,
        command_tx: cmd_tx,
        event_rx: evt_rx,
        thread: Some(thread),
    }
}

/// SSE field parser state (WHATWG HTML §9.2.6).
struct SseParserState {
    /// Current event type (reset to "message" after each dispatch).
    event_type: String,
    /// Accumulated data buffer (multi-line data: fields joined with `\n`).
    data_buf: String,
    /// Last event ID (sticky — persists across events and reconnections).
    last_event_id: String,
    /// Reconnection time in milliseconds (default 3000ms).
    retry_ms: u64,
}

impl SseParserState {
    fn new(last_event_id: Option<String>) -> Self {
        Self {
            event_type: "message".to_string(),
            data_buf: String::new(),
            last_event_id: last_event_id.unwrap_or_default(),
            retry_ms: 3000,
        }
    }

    /// Process a single SSE field line (WHATWG §9.2.6).
    fn process_field(&mut self, line: &str) {
        if line.is_empty() {
            return; // Empty line = dispatch event (handled by caller).
        }

        // Lines starting with ':' are comments — ignore.
        if line.starts_with(':') {
            return;
        }

        let (field, value) = if let Some(colon_pos) = line.find(':') {
            let field = &line[..colon_pos];
            let mut value = &line[colon_pos + 1..];
            // Strip a single leading space from value (§9.2.6 step 3).
            if value.starts_with(' ') {
                value = &value[1..];
            }
            (field, value)
        } else {
            // No colon — treat the entire line as field name with empty value.
            (line, "")
        };

        match field {
            "event" => {
                self.event_type = value.to_string();
            }
            "data" => {
                if !self.data_buf.is_empty() {
                    self.data_buf.push('\n');
                }
                self.data_buf.push_str(value);
            }
            "id" => {
                // §9.2.6: If the field value does not contain U+0000 NULL,
                // set the last event ID buffer to the value.
                if !value.contains('\0') {
                    self.last_event_id = value.to_string();
                }
            }
            "retry" => {
                // §9.2.6: Only accept if value consists of only ASCII digits.
                if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                    if let Ok(ms) = value.parse::<u64>() {
                        self.retry_ms = ms;
                    }
                }
            }
            _ => {
                // Unknown field names are ignored (§9.2.6 step 13).
            }
        }
    }

    /// Check if there is a pending event to dispatch (non-empty data buffer).
    fn has_pending_event(&self) -> bool {
        !self.data_buf.is_empty()
    }

    /// Take the pending event, resetting the parser state for the next event.
    fn take_event(&mut self) -> SseEvent {
        SseEvent::Event {
            event_type: std::mem::replace(&mut self.event_type, "message".to_string()),
            data: std::mem::take(&mut self.data_buf),
            last_event_id: self.last_event_id.clone(),
        }
    }
}

/// Async SSE I/O loop with auto-reconnection.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn sse_io_loop(
    url: url::Url,
    _with_credentials: bool,
    last_event_id: Option<String>,
    cookie_jar: Option<Arc<CookieJar>>,
    cmd_rx: Receiver<SseCommand>,
    evt_tx: Sender<SseEvent>,
) {
    let mut parser = SseParserState::new(last_event_id);
    let mut closed = false;

    loop {
        if closed {
            return;
        }

        // Check for close command before connecting.
        if cmd_rx.try_recv().is_ok() {
            // Any command is Close.
            return;
        }

        // Build and send the HTTP request.
        let client = crate::NetClient::new();
        let mut request = crate::Request {
            method: "GET".to_string(),
            url: url.clone(),
            headers: vec![("Accept".to_string(), "text/event-stream".to_string())],
            body: bytes::Bytes::new(),
        };

        // Add Last-Event-ID header for reconnections.
        if !parser.last_event_id.is_empty() {
            request
                .headers
                .push(("Last-Event-ID".to_string(), parser.last_event_id.clone()));
        }

        // Add cookies if withCredentials is set.
        if let Some(ref jar) = cookie_jar {
            if let Some(cookie_val) = jar.cookie_header_for_url(&url) {
                if !cookie_val.is_empty() {
                    request.headers.push(("Cookie".to_string(), cookie_val));
                }
            }
        }

        let response = match client.send(request).await {
            Ok(r) => r,
            Err(e) => {
                if evt_tx
                    .send(SseEvent::Error(format!("SSE connection error: {e}")))
                    .is_err()
                {
                    return;
                }
                // Auto-reconnect after retry delay.
                if wait_or_close(&cmd_rx, parser.retry_ms, &mut closed).await {
                    return;
                }
                continue;
            }
        };

        // Validate response status.
        if response.status != 200 {
            let _ = evt_tx.send(SseEvent::FatalError(format!(
                "SSE: HTTP {} (expected 200)",
                response.status
            )));
            return;
        }

        // Validate Content-Type.
        let content_type = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map_or("", |(_, v)| v.as_str());
        if !content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("text/event-stream")
        {
            let _ = evt_tx.send(SseEvent::FatalError(format!(
                "SSE: unexpected Content-Type: {content_type}"
            )));
            return;
        }

        // Connected successfully.
        if evt_tx.send(SseEvent::Connected).is_err() {
            return;
        }

        // Parse the SSE stream from the response body.
        // The response body is already fully buffered (elidex-net loads full body).
        // For true streaming, we would need chunked transfer or hyper body stream.
        // Current limitation: entire body is in memory, parsed line-by-line.
        let body = String::from_utf8_lossy(&response.body);
        for line in body.split('\n') {
            // Check for close command.
            if cmd_rx.try_recv().is_ok() {
                return;
            }

            let line = line.trim_end_matches('\r');

            if line.is_empty() {
                // Empty line = dispatch event if data buffer is non-empty.
                if parser.has_pending_event() {
                    let event = parser.take_event();
                    if evt_tx.send(event).is_err() {
                        return;
                    }
                }
            } else {
                parser.process_field(line);
            }
        }

        // Dispatch any remaining event (stream ended without trailing blank line).
        if parser.has_pending_event() {
            let event = parser.take_event();
            if evt_tx.send(event).is_err() {
                return;
            }
        }

        // Stream ended (EOF) — this is a recoverable error, auto-reconnect.
        if evt_tx
            .send(SseEvent::Error("SSE stream ended".to_string()))
            .is_err()
        {
            return;
        }
        if wait_or_close(&cmd_rx, parser.retry_ms, &mut closed).await {
            return;
        }
    }
}

/// Wait for `retry_ms` or until a close command is received.
/// Returns `true` if the connection should be closed (command received or channel disconnected).
async fn wait_or_close(cmd_rx: &Receiver<SseCommand>, retry_ms: u64, closed: &mut bool) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(retry_ms);
    loop {
        if tokio::time::Instant::now() >= deadline {
            return false; // Retry delay elapsed, reconnect.
        }
        // Check for close command every 10ms during wait.
        tokio::time::sleep(Duration::from_millis(10)).await;
        match cmd_rx.try_recv() {
            Ok(SseCommand::Close) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                *closed = true;
                return true;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_id_unique() {
        let a = SseId::next();
        let b = SseId::next();
        assert_ne!(a, b);
    }

    #[test]
    fn sse_parser_data_field() {
        let mut parser = SseParserState::new(None);
        parser.process_field("data: hello");
        assert!(parser.has_pending_event());
        let event = parser.take_event();
        if let SseEvent::Event {
            data, event_type, ..
        } = event
        {
            assert_eq!(data, "hello");
            assert_eq!(event_type, "message");
        } else {
            panic!("expected Event");
        }
    }

    #[test]
    fn sse_parser_multiline_data() {
        let mut parser = SseParserState::new(None);
        parser.process_field("data: line1");
        parser.process_field("data: line2");
        let event = parser.take_event();
        if let SseEvent::Event { data, .. } = event {
            assert_eq!(data, "line1\nline2");
        } else {
            panic!("expected Event");
        }
    }

    #[test]
    fn sse_parser_custom_event_type() {
        let mut parser = SseParserState::new(None);
        parser.process_field("event: update");
        parser.process_field("data: payload");
        let event = parser.take_event();
        if let SseEvent::Event {
            event_type, data, ..
        } = event
        {
            assert_eq!(event_type, "update");
            assert_eq!(data, "payload");
        } else {
            panic!("expected Event");
        }
        // Event type resets to "message" after dispatch.
        assert_eq!(parser.event_type, "message");
    }

    #[test]
    fn sse_parser_id_field() {
        let mut parser = SseParserState::new(None);
        parser.process_field("id: 42");
        assert_eq!(parser.last_event_id, "42");

        // ID with NUL character is ignored.
        parser.process_field("id: bad\0id");
        assert_eq!(parser.last_event_id, "42");
    }

    #[test]
    fn sse_parser_retry_field() {
        let mut parser = SseParserState::new(None);
        assert_eq!(parser.retry_ms, 3000);

        parser.process_field("retry: 5000");
        assert_eq!(parser.retry_ms, 5000);

        // Non-numeric retry is ignored.
        parser.process_field("retry: abc");
        assert_eq!(parser.retry_ms, 5000);

        // Empty retry is ignored.
        parser.process_field("retry: ");
        assert_eq!(parser.retry_ms, 5000);
    }

    #[test]
    fn sse_parser_comment_ignored() {
        let mut parser = SseParserState::new(None);
        parser.process_field(": this is a comment");
        assert!(!parser.has_pending_event());
        assert!(parser.data_buf.is_empty());
    }

    #[test]
    fn sse_parser_unknown_field_ignored() {
        let mut parser = SseParserState::new(None);
        parser.process_field("unknown: value");
        assert!(!parser.has_pending_event());
    }

    #[test]
    fn sse_parser_last_event_id_sticky() {
        let mut parser = SseParserState::new(Some("initial".to_string()));
        assert_eq!(parser.last_event_id, "initial");

        parser.process_field("data: test");
        let event = parser.take_event();
        if let SseEvent::Event { last_event_id, .. } = event {
            assert_eq!(last_event_id, "initial");
        } else {
            panic!("expected Event");
        }
        // ID persists after event dispatch.
        assert_eq!(parser.last_event_id, "initial");
    }
}
