//! Server-Sent Events I/O thread (WHATWG HTML §9.2).
//!
//! Spawns a dedicated thread with a current-thread tokio runtime for each
//! `EventSource` connection. Handles auto-reconnection with `retry` delay.
//!
//! **Architecture note**: In M4-7 (Sandbox Hardening), `spawn_sse_thread` will
//! migrate from direct thread spawning to Network Process IPC. The JS API layer
//! and drain logic are unchanged (channel abstraction is the same).

mod connect;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use tokio::io::AsyncBufReadExt;

use crate::cookie_jar::CookieJar;
use connect::{connect_sse_stream, SseConnectError};

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
/// If `origin` is `Some`, an `Origin` header is sent and CORS validation is
/// performed on the response.
///
/// # Security
///
/// The caller is responsible for SSRF validation before calling this.
#[must_use]
pub fn spawn_sse_thread(
    url: url::Url,
    last_event_id: Option<String>,
    cookie_jar: Option<Arc<CookieJar>>,
    origin: Option<String>,
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
            last_event_id,
            cookie_jar,
            origin,
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
                // set the last event ID buffer to the value. Also reject
                // CR/LF to prevent header injection when the ID is sent
                // back as `Last-Event-ID`.
                if !value.contains('\0') && !value.contains('\r') && !value.contains('\n') {
                    self.last_event_id = value.to_string();
                }
            }
            "retry" => {
                // §9.2.6: Only accept if value consists of only ASCII digits.
                if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                    if let Ok(ms) = value.parse::<u64>() {
                        self.retry_ms = ms.max(1000); // Enforce minimum 1 second
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

/// Maximum SSE line size (1 MiB). Lines exceeding this are skipped.
const MAX_SSE_LINE_SIZE: usize = 1 << 20;

/// Check the SSE command channel for a close signal.
///
/// Returns `true` if the I/O loop should exit (Close received or channel disconnected).
fn should_close(cmd_rx: &Receiver<SseCommand>) -> bool {
    match cmd_rx.try_recv() {
        Ok(SseCommand::Close) | Err(crossbeam_channel::TryRecvError::Disconnected) => true,
        Err(crossbeam_channel::TryRecvError::Empty) => false,
    }
}

/// Async SSE I/O loop with auto-reconnection.
///
/// Connects to the SSE endpoint using a raw TCP/TLS connection and reads the
/// response body incrementally line-by-line, enabling true streaming support
/// for long-lived SSE connections.
#[allow(clippy::too_many_lines)]
async fn sse_io_loop(
    url: url::Url,
    last_event_id: Option<String>,
    cookie_jar: Option<Arc<CookieJar>>,
    origin: Option<String>,
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
        if should_close(&cmd_rx) {
            return;
        }

        // Build extra headers.
        let mut extra_headers: Vec<(String, String)> = Vec::new();
        if !parser.last_event_id.is_empty() {
            extra_headers.push(("Last-Event-ID".to_string(), parser.last_event_id.clone()));
        }
        if let Some(ref jar) = cookie_jar {
            if let Some(cookie_val) = jar.cookie_header_for_url(&url) {
                if !cookie_val.is_empty() {
                    extra_headers.push(("Cookie".to_string(), cookie_val));
                }
            }
        }

        // Connect and get a streaming reader.
        let mut reader = match connect_sse_stream(&url, &extra_headers, origin.as_deref()).await {
            Ok(r) => r,
            Err(SseConnectError::Fatal(msg)) => {
                let _ = evt_tx.send(SseEvent::FatalError(msg));
                return;
            }
            Err(SseConnectError::Recoverable(msg)) => {
                if evt_tx.send(SseEvent::Error(msg)).is_err() {
                    return;
                }
                if wait_or_close(&cmd_rx, parser.retry_ms, &mut closed).await {
                    return;
                }
                continue;
            }
        };

        // Connected successfully.
        if evt_tx.send(SseEvent::Connected).is_err() {
            return;
        }

        // Stream body line-by-line.
        // The spec requires handling CR, LF, and CRLF line endings (§9.2.6).
        // `read_line` splits on LF, so standalone CR needs manual handling.
        let mut line = String::new();
        let mut first_line = true;
        loop {
            line.clear();
            match tokio::time::timeout(Duration::from_secs(60), reader.read_line(&mut line)).await {
                Ok(Ok(0) | Err(_)) => break, // EOF or read error.
                Ok(Ok(_)) => {
                    match cmd_rx.try_recv() {
                        Ok(SseCommand::Close)
                        | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                            return;
                        }
                        Err(crossbeam_channel::TryRecvError::Empty) => {}
                    }
                    // Skip oversized lines to prevent unbounded memory growth.
                    if line.len() > MAX_SSE_LINE_SIZE {
                        line.clear();
                        continue;
                    }
                    // Strip UTF-8 BOM from the very first line (§9.2.5).
                    if first_line {
                        first_line = false;
                        if line.starts_with('\u{FEFF}') {
                            line.drain(..'\u{FEFF}'.len_utf8());
                        }
                    }
                    // Handle standalone CR line endings: split on CR that
                    // isn't followed by LF. LF and CRLF are already handled
                    // by read_line + trim.
                    let content = line.trim_end_matches('\n').trim_end_matches('\r');
                    // A line might contain multiple CR-delimited sub-lines
                    // (e.g., "data: a\rdata: b\n"). Split on CR.
                    let sub_lines: Vec<&str> = if content.contains('\r') {
                        content.split('\r').collect()
                    } else {
                        vec![content]
                    };
                    for sub in sub_lines {
                        if sub.is_empty() {
                            // Empty line: dispatch if data pending, else reset event_type.
                            if parser.has_pending_event() {
                                if evt_tx.send(parser.take_event()).is_err() {
                                    return;
                                }
                            } else {
                                // §9.2.6 step 1: reset event type even if no data.
                                parser.event_type = "message".to_string();
                            }
                        } else {
                            parser.process_field(sub);
                        }
                    }
                }
                Err(_) => match cmd_rx.try_recv() {
                    Ok(SseCommand::Close) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        return;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {}
                },
            }
        }

        // Dispatch any remaining event (stream ended without trailing blank line).
        if parser.has_pending_event() {
            if evt_tx.send(parser.take_event()).is_err() {
                return;
            }
        } else {
            parser.event_type = "message".to_string();
        }

        // Stream ended (EOF) — recoverable error, auto-reconnect.
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

        // ID with CR or LF is rejected (header injection prevention).
        parser.process_field("id: evil\r\nX-Injected: yes");
        assert_eq!(parser.last_event_id, "42");
        parser.process_field("id: evil\nX-Injected: yes");
        assert_eq!(parser.last_event_id, "42");
        parser.process_field("id: evil\rX-Injected: yes");
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

        // Values below 1000ms are clamped to 1000ms minimum.
        parser.process_field("retry: 100");
        assert_eq!(parser.retry_ms, 1000);
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

    #[test]
    fn sse_parser_event_type_reset_on_empty_data() {
        // §9.2.6 step 1: if data buffer is empty, reset event type.
        let mut parser = SseParserState::new(None);
        parser.process_field("event: custom");
        // No data: field — data_buf is empty.
        assert!(!parser.has_pending_event());
        // Simulate empty line dispatch (caller checks has_pending_event,
        // then must reset event_type).
        if !parser.has_pending_event() {
            parser.event_type = "message".to_string();
        }
        // Next event without explicit event: should use "message".
        parser.process_field("data: hello");
        let event = parser.take_event();
        if let SseEvent::Event { event_type, .. } = event {
            assert_eq!(event_type, "message");
        } else {
            panic!("expected Event");
        }
    }

    #[test]
    fn sse_parser_no_trailing_newline_in_data() {
        // Multi-line data should be joined with \n but no trailing \n.
        let mut parser = SseParserState::new(None);
        parser.process_field("data: line1");
        parser.process_field("data: line2");
        let event = parser.take_event();
        if let SseEvent::Event { data, .. } = event {
            assert_eq!(data, "line1\nline2");
            assert!(!data.ends_with('\n'));
        } else {
            panic!("expected Event");
        }
    }

    #[test]
    fn sse_parser_field_no_colon() {
        // A line without a colon is treated as a field name with empty value.
        // "data" with empty string value: push_str("") on empty data_buf
        // leaves it empty, so no event is pending.
        let mut parser = SseParserState::new(None);
        parser.process_field("data");
        assert!(!parser.has_pending_event());

        // But after prior data, "data" (no colon) appends \n + "".
        parser.process_field("data: first");
        parser.process_field("data");
        let event = parser.take_event();
        if let SseEvent::Event { data, .. } = event {
            assert_eq!(data, "first\n");
        } else {
            panic!("expected Event");
        }
    }

    #[test]
    fn sse_parser_leading_space_stripped() {
        // §9.2.6 step 3: A single leading space in the value is stripped.
        let mut parser = SseParserState::new(None);
        parser.process_field("data:  two spaces");
        let event = parser.take_event();
        if let SseEvent::Event { data, .. } = event {
            // One leading space stripped, one remains.
            assert_eq!(data, " two spaces");
        } else {
            panic!("expected Event");
        }
    }
}
