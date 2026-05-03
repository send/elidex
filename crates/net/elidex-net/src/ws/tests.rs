//! Unit + cancel-arm regression tests for the WebSocket I/O
//! thread.  Split out of `ws.rs` in slot #10.6a Copilot R6 HX26
//! once the file crossed the project's 1000-line convention.

use super::io_loop::{send_close_frame, send_frame, SendFrameOutcome};
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
/// [`super::io_loop::ws_io_loop`].  [`send_close_frame`]
/// returns `true` to signal the caller that cancel preempted
/// the send — the caller is expected to `return` from the
/// worker without running further write paths.
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
    let cancelled = send_close_frame(&mut sink, 1001, "navigated away".to_string(), &cancel).await;
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
    let cancelled = send_close_frame(&mut sink, 1001, "navigated away".to_string(), &cancel).await;
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
