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
use crate::CancelHandle;
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
    /// Cooperative cancellation signal triggered by the broker
    /// during teardown.  Critical for SSE: `cmd_rx` is only polled
    /// via `try_recv` between `read_line` chunks, so a server that
    /// holds the connection open without sending data leaves the
    /// worker parked inside `reader.read_line(&mut line).await`
    /// indefinitely (the 60-second `tokio::time::timeout` only
    /// puts an upper bound on each individual read, not on the
    /// stuck state).  The cancel arm of `sse_io_loop`'s
    /// `tokio::select!` aborts the read future immediately so
    /// the broker's `thread.join()` returns within bounded time
    /// (slot #10.6a follow-up to PR #142 HCau).
    ///
    /// Crate-private: the cancel signal short-circuits the
    /// command/event flow, which is broker-only behaviour.
    /// Downstream callers must terminate workers via the
    /// documented `SseCommand::Close` path (slot #10.6a Copilot
    /// R3 HX11).
    pub(crate) cancel: CancelHandle,
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
    with_credentials: bool,
) -> SseHandle {
    let id = SseId::next();

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SseCommand>();
    // Unbounded: SSE events must not be dropped (WHATWG HTML §9.2).
    // Memory is bounded by TCP backpressure + MAX_SSE_LINE_SIZE (1 MiB).
    let (evt_tx, evt_rx) = crossbeam_channel::unbounded::<SseEvent>();

    let cancel = CancelHandle::new();
    let worker_cancel = cancel.clone();
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
            with_credentials,
            cmd_rx,
            evt_tx,
            worker_cancel,
        ));
    });

    SseHandle {
        id,
        command_tx: cmd_tx,
        event_rx: evt_rx,
        thread: Some(thread),
        cancel,
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
            "id" if !value.contains('\0') && !value.contains('\r') && !value.contains('\n') => {
                // §9.2.6: If the field value does not contain U+0000 NULL,
                // set the last event ID buffer to the value. Also reject
                // CR/LF to prevent header injection when the ID is sent
                // back as `Last-Event-ID`.
                self.last_event_id = value.to_string();
            }
            "retry" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                // §9.2.6: Only accept if value consists of only ASCII digits.
                if let Ok(ms) = value.parse::<u64>() {
                    self.retry_ms = ms.max(1000); // Enforce minimum 1 second
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

/// Check the SSE command channel and the cancel signal for a close signal.
///
/// Returns `true` if the I/O loop should exit: a `Close` command was
/// received, the channel was disconnected, or the broker triggered the
/// per-handle [`CancelHandle`] (slot #10.6a — drop-cmd alone is invisible
/// to a worker parked deep inside `read_line` against a silent server,
/// so the cancel signal is the only reliable post-handshake teardown
/// hook for SSE).
fn should_close(cmd_rx: &Receiver<SseCommand>, cancel: &CancelHandle) -> bool {
    if cancel.is_cancelled() {
        return true;
    }
    match cmd_rx.try_recv() {
        Ok(SseCommand::Close) | Err(crossbeam_channel::TryRecvError::Disconnected) => true,
        Err(crossbeam_channel::TryRecvError::Empty) => false,
    }
}

/// Race a future against a [`CancelHandle`] — returns `Some(value)`
/// when the future resolved first, or `None` when the broker
/// triggered cancel.  Used by [`sse_io_loop`] to wrap
/// [`connect_sse_stream`]'s long-running response read so a peer
/// that accepts the TCP connection but never replies to the HTTP
/// GET still unblocks the worker on broker-driven teardown
/// (slot #10.6a).  Extracted into a free helper so unit tests
/// can exercise the cancel arm directly without binding a
/// real TCP fixture (Copilot R1 HX1).
async fn await_with_cancel<F>(fut: F, cancel: &CancelHandle) -> Option<F::Output>
where
    F: std::future::Future,
{
    tokio::select! {
        biased;
        () = cancel.cancelled() => None,
        out = fut => Some(out),
    }
}

/// Race [`tokio::io::AsyncBufReadExt::read_line`] (wrapped in a
/// per-attempt `tokio::time::timeout`) against a [`CancelHandle`].
/// Returns `None` if cancel fired (caller should `return`),
/// otherwise the inner `Result<Result<usize, std::io::Error>, Elapsed>`
/// from the timeout.  Extracted from [`sse_io_loop`]'s body-read
/// loop so unit tests can drive the cancel arm against a
/// `tokio::io::duplex` fixture without going through
/// `connect_sse_stream`'s SSRF check (Copilot R1 HX2).
async fn read_line_with_cancel<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    line: &mut String,
    per_read_timeout: Duration,
    cancel: &CancelHandle,
) -> Option<Result<Result<usize, std::io::Error>, tokio::time::error::Elapsed>> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => None,
        res = tokio::time::timeout(per_read_timeout, reader.read_line(line)) => Some(res),
    }
}

/// Async SSE I/O loop with auto-reconnection.
///
/// Connects to the SSE endpoint using a raw TCP/TLS connection and reads the
/// response body incrementally line-by-line, enabling true streaming support
/// for long-lived SSE connections.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
async fn sse_io_loop(
    url: url::Url,
    last_event_id: Option<String>,
    cookie_jar: Option<Arc<CookieJar>>,
    origin: Option<String>,
    with_credentials: bool,
    cmd_rx: Receiver<SseCommand>,
    evt_tx: Sender<SseEvent>,
    cancel: CancelHandle,
) {
    let mut parser = SseParserState::new(last_event_id);
    let mut closed = false;

    loop {
        if closed {
            return;
        }

        // Check for close command before connecting.
        if should_close(&cmd_rx, &cancel) {
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

        // Connect and get a streaming reader.  Cancel-aware: a
        // server that accepts the TCP connection but never replies
        // to the HTTP GET would otherwise stall the worker for the
        // full TCP-stack timeout (slot #10.6a).
        let connect_fut =
            connect_sse_stream(&url, &extra_headers, origin.as_deref(), with_credentials);
        let Some(connect_outcome) = await_with_cancel(connect_fut, &cancel).await else {
            return;
        };
        let mut reader = match connect_outcome {
            Ok(r) => r,
            Err(SseConnectError::Fatal(msg)) => {
                // Fatal errors are critical lifecycle events — use blocking send.
                let _ = evt_tx.send(SseEvent::FatalError(msg));
                return;
            }
            Err(SseConnectError::Recoverable(msg)) => {
                // Error events are critical lifecycle events — use blocking send.
                let _ = evt_tx.send(SseEvent::Error(msg));
                // Only exit if receiver is disconnected, not if channel is full.
                if wait_or_close(&cmd_rx, parser.retry_ms, &mut closed, &cancel).await {
                    return;
                }
                continue;
            }
        };

        // Connected successfully.
        // Only exit on Disconnected — Full (backpressure) is non-fatal.
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
            // Cancel-aware streaming read: select against the
            // per-handle [`CancelHandle`] so a server that holds
            // the connection open without sending data still
            // unblocks the worker on broker-driven teardown.  The
            // 60-second `tokio::time::timeout` only bounds each
            // individual read attempt, not the aggregate stuck
            // state — without the cancel arm a stuck SSE worker
            // would survive the broker's `thread.join()` until
            // every retry burned through its own timeout (slot
            // #10.6a).
            let Some(read_outcome) =
                read_line_with_cancel(&mut reader, &mut line, Duration::from_secs(60), &cancel)
                    .await
            else {
                return;
            };
            match read_outcome {
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
                                // Only exit on Disconnected — Full is non-fatal for data events.
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
            // Only exit on Disconnected — Full is non-fatal for data events.
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
        if wait_or_close(&cmd_rx, parser.retry_ms, &mut closed, &cancel).await {
            return;
        }
    }
}

/// Wait for `retry_ms` or until a close signal is received.
/// Returns `true` if the connection should be closed: a `Close`
/// command was queued, the command channel was disconnected, or the
/// broker triggered the per-handle [`CancelHandle`] (slot #10.6a —
/// the cancel arm of the inner `tokio::select!` resolves the wait
/// immediately so reconnects don't keep an unwanted worker alive
/// for the full retry budget after the renderer has been torn down).
async fn wait_or_close(
    cmd_rx: &Receiver<SseCommand>,
    retry_ms: u64,
    closed: &mut bool,
    cancel: &CancelHandle,
) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(retry_ms);
    loop {
        if tokio::time::Instant::now() >= deadline {
            return false; // Retry delay elapsed, reconnect.
        }
        // Check for close command / cancel every 10ms during wait.
        tokio::select! {
            biased;
            () = cancel.cancelled() => {
                *closed = true;
                return true;
            }
            () = tokio::time::sleep(Duration::from_millis(10)) => {}
        }
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

    /// Slot #10.6a: the synchronous [`should_close`] probe must
    /// honour the per-handle [`CancelHandle`] in addition to the
    /// command channel, so the SSE worker's outer-loop entry
    /// gate exits the moment the broker triggers cancel — even
    /// if no `Close` command was ever queued (the broker drops
    /// `command_tx` after `cancel.cancel()`, but a worker that
    /// reaches `should_close` before observing the disconnect
    /// would otherwise loop into another connect attempt).
    #[test]
    fn should_close_returns_true_when_cancelled() {
        let (_cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SseCommand>();
        let cancel = CancelHandle::new();
        assert!(
            !should_close(&cmd_rx, &cancel),
            "fresh handle / open channel must not signal close"
        );
        cancel.cancel();
        assert!(
            should_close(&cmd_rx, &cancel),
            "should_close must observe cancel state"
        );
    }

    /// Slot #10.6a: the asynchronous [`wait_or_close`] retry-delay
    /// loop in [`sse_io_loop`] must observe [`CancelHandle::cancel`]
    /// so a broker-driven teardown doesn't have to wait the full
    /// retry budget (default 3 seconds, configurable via
    /// `SseParserState::retry_ms`) before the worker exits.
    /// Without the cancel arm the worker keeps the SSE handle
    /// alive across the entire retry interval, blocking
    /// `close_all_for_client`'s `thread.join()` for that whole
    /// window after the renderer has already been torn down.
    #[tokio::test]
    async fn wait_or_close_returns_true_on_cancel_during_retry() {
        let (_cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SseCommand>();
        let cancel = CancelHandle::new();
        let mut closed = false;

        // Schedule cancel after a small but non-zero delay so the
        // wait genuinely enters the inner `tokio::select!` (not
        // just the synchronous `is_cancelled` fast-path on first
        // iteration).
        let cancel_for_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel_for_trigger.cancel();
        });

        let started = std::time::Instant::now();
        let result = wait_or_close(&cmd_rx, /* retry_ms */ 3_000, &mut closed, &cancel).await;
        let elapsed = started.elapsed();

        assert!(result, "wait_or_close must return true on cancel");
        assert!(closed, "wait_or_close must set the closed flag on cancel");
        assert!(
            elapsed < Duration::from_millis(500),
            "wait_or_close blocked for {elapsed:?} — cancel arm missing? \
             expected ~50ms, retry_ms was 3_000ms"
        );
    }

    /// Slot #10.6a (Copilot R1 HX1): the
    /// [`await_with_cancel`] wrapper used in [`sse_io_loop`] for
    /// the `connect_sse_stream` arm must resolve to `None` when
    /// cancel fires, even if the underlying connect future is
    /// still pending.  This test substitutes a 60-second sleep
    /// for the real connect future so the cancel arm is the
    /// only thing that can resolve it within the assertion
    /// window — if the helper's `tokio::select!` arm goes
    /// missing in a future refactor the inner sleep would
    /// dominate and the test would fail on the deadline.
    #[tokio::test]
    async fn await_with_cancel_resolves_to_none_on_cancel() {
        let cancel = CancelHandle::new();
        let cancel_for_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel_for_trigger.cancel();
        });
        let started = std::time::Instant::now();
        let pending = async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            42_u32
        };
        let outcome = await_with_cancel(pending, &cancel).await;
        let elapsed = started.elapsed();
        assert!(
            outcome.is_none(),
            "await_with_cancel must yield None when cancel fires before the inner future"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "await_with_cancel blocked for {elapsed:?} — cancel arm missing? \
             expected ~50ms"
        );
    }

    /// Slot #10.6a sanity for [`await_with_cancel`]: the helper
    /// must return `Some(value)` when the inner future resolves
    /// first.  Without this companion, a bug that always
    /// preferred the cancel arm (e.g. accidentally setting
    /// cancel as `is_cancelled`-true) could pass
    /// [`await_with_cancel_resolves_to_none_on_cancel`] while
    /// breaking the happy path.
    #[tokio::test]
    async fn await_with_cancel_resolves_inner_when_no_cancel() {
        let cancel = CancelHandle::new();
        let outcome = await_with_cancel(async { 7_u32 }, &cancel).await;
        assert_eq!(outcome, Some(7));
    }

    /// Slot #10.6a (Copilot R1 HX2): the
    /// [`read_line_with_cancel`] wrapper used in
    /// [`sse_io_loop`]'s body-read loop must resolve to `None`
    /// when cancel fires while parked on `reader.read_line`,
    /// even when the inner reader has no data and would
    /// otherwise wait the full per-attempt
    /// `tokio::time::timeout` (60 seconds in production).
    /// Driven via [`tokio::io::duplex`] so the test never
    /// touches real TCP / SSRF gating.
    #[tokio::test]
    async fn read_line_with_cancel_resolves_to_none_on_cancel() {
        // `duplex(64)` returns a paired stream; we keep
        // `_writer` alive (un-dropped) so the reader doesn't
        // EOF — `read_line` stays parked indefinitely.
        let (_writer, reader_side) = tokio::io::duplex(64);
        let mut reader = tokio::io::BufReader::new(reader_side);
        let mut line = String::new();
        let cancel = CancelHandle::new();
        let cancel_for_trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel_for_trigger.cancel();
        });
        let started = std::time::Instant::now();
        let outcome =
            read_line_with_cancel(&mut reader, &mut line, Duration::from_secs(60), &cancel).await;
        let elapsed = started.elapsed();
        assert!(
            outcome.is_none(),
            "read_line_with_cancel must yield None when cancel fires before the inner read_line completes"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "read_line_with_cancel blocked for {elapsed:?} — cancel arm missing?"
        );
        assert!(
            line.is_empty(),
            "no data was written so the line buffer must be untouched"
        );
    }

    /// Slot #10.6a sanity for [`read_line_with_cancel`]: the
    /// helper must return the inner read result when data
    /// arrives before cancel fires.
    #[tokio::test]
    async fn read_line_with_cancel_returns_inner_result_on_data() {
        use tokio::io::AsyncWriteExt;

        let (mut writer, reader_side) = tokio::io::duplex(64);
        let mut reader = tokio::io::BufReader::new(reader_side);
        let mut line = String::new();
        let cancel = CancelHandle::new();

        // Write a complete line so `read_line` resolves promptly.
        tokio::spawn(async move {
            writer.write_all(b"hello\n").await.expect("write");
        });

        let outcome =
            read_line_with_cancel(&mut reader, &mut line, Duration::from_secs(5), &cancel).await;
        let outcome = outcome.expect("inner future must resolve when no cancel fires");
        let bytes = outcome
            .expect("per-read timeout must not elapse on a 6-byte loopback write")
            .expect("read_line must succeed");
        assert_eq!(bytes, 6);
        assert_eq!(line, "hello\n");
    }

    /// Slot #10.6a sanity: [`wait_or_close`] still elapses its
    /// retry delay normally when no cancel / close arrives, and
    /// returns `false` so the outer loop reconnects.  Without
    /// this companion check, a regression that swapped the
    /// cancel arm's return value or wired the deadline incorrectly
    /// could pass the cancel-on-retry test while breaking the
    /// happy-path retry behaviour.  Uses a 60ms `retry_ms` so
    /// the test stays fast.
    #[tokio::test]
    async fn wait_or_close_elapses_retry_without_cancel() {
        let (_cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SseCommand>();
        let cancel = CancelHandle::new();
        let mut closed = false;
        let started = std::time::Instant::now();
        let result = wait_or_close(&cmd_rx, /* retry_ms */ 60, &mut closed, &cancel).await;
        let elapsed = started.elapsed();
        assert!(!result, "wait_or_close must return false on retry elapse");
        assert!(!closed, "closed flag must remain false on retry elapse");
        assert!(
            elapsed >= Duration::from_millis(50),
            "wait_or_close returned in {elapsed:?} — retry deadline ignored?"
        );
    }
}
