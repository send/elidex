//! Slot #10.6c: `RegisterRenderer` ack-handshake helper.
//!
//! Extracted from `broker/handle.rs` in slot #10.6c Copilot R2
//! HX5 once the new helper + its tests pushed the parent file
//! past the project's ~1000-line file-split threshold (the same
//! convention applied earlier in slot #10.5 to
//! `vm/gc/mod.rs:16-28` and `vm/host/headers/mod.rs:67-82`).
//!
//! Defines:
//! - [`REGISTER_ACK_TIMEOUT`] — 500 ms upper bound on the
//!   `RegisterRenderer` ack wait.  See the doc comment for the
//!   trade-off rationale.
//! - [`register_with_ack`] / [`register_with_ack_for_test`] —
//!   the ack-handshake itself, plus a timeout-parameterised
//!   variant for unit tests.
//!
//! Production callers ([`super::handle::NetworkProcessHandle::create_renderer_handle`]
//! / [`super::handle::NetworkHandle::create_sibling_handle`])
//! flow through `register_with_ack`; only the unit tests in the
//! local `tests` submodule reach for `register_with_ack_for_test`.

use std::time::Duration;

use super::{NetworkProcessControl, NetworkToRenderer};

/// Slot #10.6c: upper bound a renderer-creation call waits for
/// the broker's `RegisterRenderer` ack.
///
/// **500 ms** is intentionally tight: the healthy path is
/// sub-millisecond (one broker iteration —
/// `sel.ready_timeout(1s)` wakes immediately on the new control
/// message — plus the ack send), and even a heavily-loaded
/// broker drains control before request and inserts into
/// `clients` in tens of microseconds.  Anything past 500 ms is
/// pathology (broker thread starved, hung in a long lock, or
/// blocked on a slow `cleanup_finished` poll), at which point
/// we'd rather fast-fail to a pre-unregistered handle than
/// freeze the caller for several seconds.
///
/// The shorter ceiling matters because
/// [`super::handle::NetworkProcessHandle::create_renderer_handle`]
/// / [`super::handle::NetworkHandle::create_sibling_handle`]
/// are called directly from browser-thread paths (e.g.
/// `App::open_new_tab`, `sw_coordinator::register`) and the
/// `new Worker()` constructor in the JS host — none of those
/// tolerate a multi-second freeze of the event loop on a
/// stalled broker (Copilot R1).  An R1-era 5 s ceiling would
/// have been a UX regression on the very paths that drove the
/// original race fix.
///
/// On timeout the caller receives a pre-unregistered
/// `NetworkHandle` so every subsequent operation surfaces the
/// slot #10.6b synthetic error path immediately rather than
/// queueing into a broker that has no `clients` entry for us.
/// The timeout branch ALSO sends a follow-up
/// [`NetworkProcessControl::UnregisterRenderer`] so a broker
/// that resumes draining later cleans up the stale entry
/// itself (FIFO on `control_tx` guarantees the orphan-
/// preventing pair is processed in order — Copilot R1 F1).
pub(super) const REGISTER_ACK_TIMEOUT: Duration = Duration::from_millis(500);

/// Slot #10.6c: send `RegisterRenderer { client_id, response_tx,
/// ack_tx }` on `control_tx` and block on the ack receiver up
/// to [`REGISTER_ACK_TIMEOUT`].  Returns `true` when the caller
/// should construct a **pre-unregistered** [`super::handle::NetworkHandle`]
/// (the ack was lost — broker is hung or already gone),
/// `false` on a healthy ack.
///
/// `caller_label` distinguishes the warn-log emit site
/// (`create_renderer_handle` vs `create_sibling_handle`) so an
/// operator chasing an ack-timeout in the wild can pinpoint the
/// factory without adding a stack-trace to every log line.
///
/// The `bounded(1)` capacity is the standard rendezvous shape
/// for a single-shot ack: it guarantees that a successful broker
/// send happens-before the receiver's `recv` returns Ok.  A
/// `bounded(0)` rendezvous would also work but adds a
/// synchronous-handoff requirement that does nothing for
/// correctness here (the broker has already inserted into
/// `clients` by the time it sends the ack — buffering one element
/// for the broker to release the channel immediately is fine).
pub(super) fn register_with_ack(
    control_tx: &crossbeam_channel::Sender<NetworkProcessControl>,
    client_id: u64,
    response_tx: crossbeam_channel::Sender<NetworkToRenderer>,
    caller_label: &'static str,
) -> bool {
    register_with_ack_for_test(
        control_tx,
        client_id,
        response_tx,
        caller_label,
        REGISTER_ACK_TIMEOUT,
    )
}

/// Slot #10.6c: implementation core of [`register_with_ack`]
/// with an explicit timeout, so unit tests can exercise the
/// timeout/disconnect branches without waiting the full 500 ms
/// production ceiling.  Production call sites flow through
/// `register_with_ack` which fixes `timeout` to
/// [`REGISTER_ACK_TIMEOUT`].
fn register_with_ack_for_test(
    control_tx: &crossbeam_channel::Sender<NetworkProcessControl>,
    client_id: u64,
    response_tx: crossbeam_channel::Sender<NetworkToRenderer>,
    caller_label: &'static str,
    timeout: Duration,
) -> bool {
    let (ack_tx, ack_rx) = crossbeam_channel::bounded(1);

    if control_tx
        .send(NetworkProcessControl::RegisterRenderer {
            client_id,
            response_tx,
            ack_tx,
        })
        .is_err()
    {
        // Broker control channel is already closed: no point
        // waiting on the ack at all.  Return pre-unregistered so
        // the caller's NetworkHandle short-circuits every
        // subsequent operation via the slot #10.6b machinery.
        tracing::warn!(
            client_id,
            caller = caller_label,
            "RegisterRenderer send failed — broker is gone; \
             returning a pre-unregistered handle"
        );
        return true;
    }

    match ack_rx.recv_timeout(timeout) {
        Ok(()) => false,
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
            tracing::warn!(
                client_id,
                caller = caller_label,
                timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                "RegisterRenderer ack timed out — broker may be hung; \
                 returning a pre-unregistered handle (fetch_async / \
                 fetch_blocking will surface a synthetic 'renderer \
                 unregistered' Err; cancel_fetch / send return false)"
            );
            // Slot #10.6c (Copilot R1 F1): the Register message
            // is still in flight on `control_tx` — a broker that
            // is merely stalled (not dead) may eventually drain
            // it and insert `client_id` into `clients`, leaving
            // an orphan entry whose response_tx is held by a
            // renderer that will never read from it.  Pre-emptively
            // queue the matching `UnregisterRenderer` so when the
            // broker resumes draining, the FIFO order on
            // `control_tx` guarantees Register is processed
            // first (insert) and our follow-up second
            // (synthesise / close / cancel / clients.remove).
            // No reliance on the caller's eventual
            // `NetworkHandle::Drop` — pre-unregistered handles
            // can outlive their factory call by an unbounded
            // amount under embedder control.
            //
            // Best-effort: if the broker exits between the
            // timeout and this send, the channel is now
            // disconnected and the send fails silently — that's
            // fine, the broker exit drops `control_rx`
            // unconditionally so any orphan would have been
            // dropped too.
            let _ = control_tx.send(NetworkProcessControl::UnregisterRenderer { client_id });
            true
        }
        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
            tracing::warn!(
                client_id,
                caller = caller_label,
                "RegisterRenderer ack channel disconnected — broker exited \
                 before acking; returning a pre-unregistered handle"
            );
            // No follow-up needed: Disconnected means the broker
            // exited without sending the ack, which means the
            // Register message was either never processed (and
            // got dropped along with `control_rx`) OR the ack_tx
            // was dropped without a send.  Either way there is
            // no live `clients` entry to clean up.
            true
        }
    }
}

#[cfg(test)]
mod tests {
    //! Slot #10.6c unit tests for [`register_with_ack`].  The
    //! integration coverage in `broker/tests/lifecycle.rs`
    //! exercises the live-broker happy path and the broker-
    //! exited disconnect path, but the **timeout** branch is
    //! hard to trigger with a real broker (we'd have to hang
    //! the broker thread for >`REGISTER_ACK_TIMEOUT`).  These
    //! tests drive [`register_with_ack_for_test`] directly with
    //! a tight injected timeout against a manually-controlled
    //! `control_rx` so the timeout / pre-unregistered cleanup
    //! branches fire deterministically in milliseconds.
    use std::time::Duration;

    use super::*;

    /// Slot #10.6c (Copilot R1 F1) regression: when the ack
    /// times out, `register_with_ack` must NOT leave the
    /// `Register` message orphaned in `control_tx` — a stalled-
    /// but-alive broker that resumes draining later would
    /// process the Register, insert into `clients`, and never
    /// see a matching cleanup.  The fix is to queue a follow-up
    /// `UnregisterRenderer` on the timeout branch; FIFO on the
    /// crossbeam channel guarantees broker drains
    /// `Register → UnregisterRenderer` in order so the orphan
    /// entry is cleaned by the broker's standard
    /// `UnregisterRenderer` teardown sequence even when the
    /// caller's `NetworkHandle::Drop` runs much later (or
    /// not at all under embedder leak).
    #[test]
    fn timeout_emits_followup_unregister_to_clean_late_register() {
        let (control_tx, control_rx) = crossbeam_channel::unbounded();
        let (response_tx, _response_rx) = crossbeam_channel::unbounded();

        let started = std::time::Instant::now();
        let pre_unregistered = register_with_ack_for_test(
            &control_tx,
            999,
            response_tx,
            "test",
            Duration::from_millis(20),
        );
        let elapsed = started.elapsed();
        assert!(
            pre_unregistered,
            "timeout branch must return pre-unregistered=true"
        );
        assert!(
            elapsed >= Duration::from_millis(20) && elapsed < Duration::from_millis(500),
            "timeout fired in {elapsed:?}; expected ≥ 20 ms and well below the 500 ms production ceiling"
        );

        // FIFO order on `control_tx`: Register first, then the
        // follow-up UnregisterRenderer.  Both sides arrive
        // synchronously in `control_rx` because the channel is
        // unbounded and the sends are local.
        match control_rx.try_recv() {
            Ok(NetworkProcessControl::RegisterRenderer { client_id, .. }) => {
                assert_eq!(client_id, 999, "first message must be the Register we sent");
            }
            other => panic!("expected RegisterRenderer, got {other:?}"),
        }
        match control_rx.try_recv() {
            Ok(NetworkProcessControl::UnregisterRenderer { client_id }) => {
                assert_eq!(
                    client_id, 999,
                    "follow-up must be the matching UnregisterRenderer for the same cid"
                );
            }
            other => {
                panic!("expected follow-up UnregisterRenderer (Copilot R1 F1 fix), got {other:?}")
            }
        }
        // No further messages.
        assert!(
            control_rx.try_recv().is_err(),
            "extra control messages emitted by register_with_ack timeout branch — \
             only Register + UnregisterRenderer are expected"
        );
    }

    /// Slot #10.6c regression: if `control_tx` is already
    /// disconnected (broker has exited), `register_with_ack`
    /// must take the SendError fast-fail branch and return
    /// `pre_unregistered=true` synchronously — no waiting on
    /// the ack channel at all.  Disconnect-after-Send is
    /// covered separately by the integration tests.
    #[test]
    fn send_error_fast_fail_when_broker_already_gone() {
        let (control_tx, control_rx) = crossbeam_channel::unbounded();
        drop(control_rx);
        let (response_tx, _response_rx) = crossbeam_channel::unbounded();

        let started = std::time::Instant::now();
        let pre_unregistered = register_with_ack_for_test(
            &control_tx,
            42,
            response_tx,
            "test",
            // The timeout would only fire if we got past the
            // SendError check.  Use a value that would visibly
            // dominate wall-clock if the fast-fail regressed.
            Duration::from_secs(10),
        );
        let elapsed = started.elapsed();
        assert!(pre_unregistered);
        assert!(
            elapsed < Duration::from_millis(100),
            "fast-fail branch took {elapsed:?}; expected sub-100 ms — \
             register_with_ack waited on ack despite the dead control_tx"
        );
    }

    /// Slot #10.6c happy path sanity: when the broker's
    /// surrogate sends the ack promptly, `register_with_ack`
    /// returns `false` (live handle).  The control_rx side
    /// drives a tiny ack-relay thread to mimic the broker's
    /// `dispatch::handle_control::RegisterRenderer` path.
    #[test]
    fn ack_path_returns_live_handle() {
        let (control_tx, control_rx) = crossbeam_channel::unbounded();
        let (response_tx, _response_rx) = crossbeam_channel::unbounded();

        let surrogate = std::thread::spawn(move || {
            // Recv with the same timeout pattern the broker uses
            // (so this test isn't sensitive to surrogate startup
            // ordering on a loaded CI runner).
            if let Ok(NetworkProcessControl::RegisterRenderer { ack_tx, .. }) =
                control_rx.recv_timeout(Duration::from_secs(2))
            {
                let _ = ack_tx.send(());
            }
        });

        let pre_unregistered =
            register_with_ack_for_test(&control_tx, 7, response_tx, "test", Duration::from_secs(2));
        assert!(
            !pre_unregistered,
            "ack received from surrogate broker — live handle expected"
        );
        surrogate.join().expect("surrogate broker thread panicked");
    }

    /// Slot #10.6c (Copilot R12 F1) regression: distinct from
    /// the SendError fast-fail path
    /// (`send_error_fast_fail_when_broker_already_gone`), the
    /// `RecvTimeoutError::Disconnected` branch fires when the
    /// broker received the RegisterRenderer message (taking
    /// ownership of `ack_tx`) but exited before invoking
    /// `ack_tx.send(())` — at that point all senders for the
    /// ack channel are dropped and `recv_timeout` returns
    /// `Disconnected` rather than `Timeout`.  This branch
    /// **intentionally** does NOT emit a follow-up
    /// `UnregisterRenderer` (unlike the timeout branch),
    /// because Disconnect means the broker is gone and the
    /// orphan-cleanup it would have served is vacuous.  We
    /// simulate it with a surrogate that consumes the message
    /// but never acks, then exits — dropping `ack_tx`.
    #[test]
    fn disconnected_path_returns_pre_unregistered_without_followup_unregister() {
        let (control_tx, control_rx) = crossbeam_channel::unbounded();
        let (response_tx, _response_rx) = crossbeam_channel::unbounded();

        // Run the helper on a separate thread so the test
        // thread can play the surrogate broker role inline:
        // recv the Register message and drop it (releasing
        // `ack_tx`, which disconnects the ack channel for the
        // helper) while keeping `control_rx` alive — that lets
        // us drain it after the call to verify the Disconnect
        // branch did NOT queue a follow-up `UnregisterRenderer`
        // (Copilot R15 F3: previously the surrogate dropped
        // `control_rx` along with the message, so any
        // accidental follow-up send would have been silently
        // swallowed and the test would have passed even on a
        // regression).
        let control_tx_for_helper = control_tx.clone();
        let helper = std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let pre_unregistered = register_with_ack_for_test(
                &control_tx_for_helper,
                123,
                response_tx,
                "test",
                // Generous timeout: the disconnect should fire well
                // before we approach this ceiling.  Anything close
                // to the ceiling means the Disconnect path was
                // mis-classified as Timeout.
                Duration::from_secs(5),
            );
            (pre_unregistered, started.elapsed())
        });

        // Receive the Register message on the test thread,
        // explicitly bind `ack_tx`, and drop it — that
        // disconnects the ack channel so the helper's
        // `recv_timeout` returns `Disconnected` rather than
        // waiting the full 5 s ceiling.  We bind by name (not
        // `..`) because struct-pattern `..` does NOT
        // necessarily drop the unmatched fields at the end of
        // the match arm in this case (they may stay attached
        // to `msg` and only drop when `msg` itself is dropped),
        // and we want a deterministic drop point so the helper
        // observes Disconnect promptly.
        let msg = control_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("Register message should arrive");
        let (got_cid, ack_tx, response_tx) = match msg {
            NetworkProcessControl::RegisterRenderer {
                client_id,
                response_tx,
                ack_tx,
            } => (client_id, ack_tx, response_tx),
            other => panic!("expected RegisterRenderer, got {other:?}"),
        };
        assert_eq!(got_cid, 123, "Register payload must carry the cid we sent");
        drop(response_tx);
        drop(ack_tx);
        // ack_tx is now gone; helper's `recv_timeout` returns
        // `Disconnected` synchronously.

        let (pre_unregistered, elapsed) = helper.join().expect("helper thread panicked");

        assert!(
            pre_unregistered,
            "Disconnect branch must return pre-unregistered=true"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "Disconnect branch took {elapsed:?}; expected sub-second.  \
             A regression that mis-classified Disconnect as Timeout \
             would have waited the full 5 s recv ceiling configured \
             in this test."
        );

        // Crucially: no follow-up `UnregisterRenderer` queued.
        // The Timeout branch deliberately emits one (R1 F1
        // orphan-cleanup); the Disconnect branch deliberately
        // does NOT, because the broker is gone and the cleanup
        // would be vacuous.  A regression that copy-pasted the
        // follow-up into Disconnect would queue an extra
        // message here, caught by this drain.
        assert!(
            control_rx.try_recv().is_err(),
            "Disconnect branch must NOT emit a follow-up `UnregisterRenderer` — \
             only the Timeout branch does (R1 F1 orphan cleanup)"
        );
    }
}
