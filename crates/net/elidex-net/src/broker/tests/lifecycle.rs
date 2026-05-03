//! Broker integration tests for lifecycle (spawn / register / unregister / drain / id / multi-renderer / debug).
//!
//! Sub-module of `broker::tests`; helpers (e.g. `test_client`) and
//! shared imports come from `super` (`tests/mod.rs`).

use std::sync::atomic::Ordering;
use std::time::Duration;

use super::super::*;
use super::test_client;

#[test]
fn spawn_and_shutdown() {
    let handle = spawn_network_process(test_client());
    handle.shutdown();
}

#[test]
fn create_renderer_handle() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    assert!(renderer.client_id() > 0);
    handle.shutdown();
}

#[test]
fn unregister_renderer() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    let cid = renderer.client_id();
    handle.unregister_renderer(cid);
    // Brief wait for unregistration to propagate.
    std::thread::sleep(Duration::from_millis(10));
    handle.shutdown();
}
#[test]
fn drain_events_empty() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    let events = renderer.drain_events();
    assert!(events.is_empty());
    handle.shutdown();
}

#[test]
fn fetch_id_monotonic() {
    let a = FetchId::next();
    let b = FetchId::next();
    assert!(b.0 > a.0);
}

#[test]
fn multiple_renderers() {
    let handle = spawn_network_process(test_client());
    let r1 = handle.create_renderer_handle();
    let r2 = handle.create_renderer_handle();
    assert_ne!(r1.client_id(), r2.client_id());
    handle.shutdown();
}

#[test]
fn debug_network_handle() {
    let handle = spawn_network_process(test_client());
    let renderer = handle.create_renderer_handle();
    let debug = format!("{renderer:?}");
    assert!(debug.contains("NetworkHandle"));
    handle.shutdown();
}

// ---------------------------------------------------------------------------
// Slot #10.6c — RegisterRenderer ack handshake
//
// `create_renderer_handle` / `create_sibling_handle` now block
// on a one-shot ack that the broker emits AFTER `clients.insert`
// in `dispatch::handle_control::RegisterRenderer`.  This closes
// the cross-channel race where a `Fetch` on `request_tx` could
// be observed by the broker BEFORE the matching `Register` on
// `control_tx` (the stale-cid gate would silently drop the
// Fetch).  On ack timeout / disconnect the factories fall
// through to a pre-unregistered handle whose every operation
// short-circuits via the slot #10.6b `unregistered` flag.
// ---------------------------------------------------------------------------

/// Slot #10.6c regression: after `create_renderer_handle`
/// returns, the broker is guaranteed to have inserted the
/// renderer's `client_id` into its `clients` map.  We assert
/// this by issuing a `cancel_fetch` against a freshly-allocated
/// (and never-issued) `FetchId` immediately after creation —
/// the broker's `handle_cancel_fetch` path always pushes a
/// synthetic `Err("aborted")` reply, but ONLY for cids it
/// recognises in `clients`.  Pre-#10.6c the renderer thread
/// could send `CancelFetch` on `request_tx` before the broker
/// had drained the matching `Register` on `control_tx` (the
/// race window is the broker's `for _ in 0..64` request-drain
/// loop, which can pull messages that arrived AFTER the start
/// of the iteration), so the cid would not be in `clients` and
/// the gate would silently drop the message — the renderer
/// would never observe the synthetic reply.  Post-#10.6c the
/// ack handshake makes that race impossible.
///
/// Drains for up to 1 s (generous for a healthy broker; the
/// reply arrives in a couple of broker iterations on
/// loopback) so the test stays robust on loaded CI without
/// converting the gate into a fixed-sleep timing oracle
/// (lesson #133).
#[test]
fn create_renderer_handle_synchronously_registers_cid() {
    let np = spawn_network_process(test_client());
    let renderer = np.create_renderer_handle();

    let probe = FetchId::next();
    assert!(
        renderer.cancel_fetch(probe),
        "fresh renderer's cancel_fetch send failed — handle should be live post-#10.6c"
    );

    // Wait for the broker's synthetic `Err("aborted")` reply.
    // Without the ack handshake, the CancelFetch could land in
    // the broker's request-drain phase before the matching
    // Register reaches `clients.insert`, and the stale-cid
    // gate would drop it silently → no synthetic reply ever
    // arrives → this loop would time out.
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut saw = false;
    while std::time::Instant::now() < deadline {
        let events = renderer.drain_events();
        if events.iter().any(|ev| {
            matches!(
                ev,
                NetworkToRenderer::FetchResponse(rid, Err(msg))
                    if *rid == probe && msg.contains("aborted")
            )
        }) {
            saw = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        saw,
        "broker did not emit synthetic 'aborted' reply for post-create CancelFetch \
         within 1s — Register did not happen-before CancelFetch (slot #10.6c regression)"
    );

    np.shutdown();
}

/// Slot #10.6c (Copilot R7) regression: same happens-before
/// guarantee for `NetworkHandle::create_sibling_handle`.  The
/// sibling factory drives its own `register_with_ack`
/// invocation (separate code path from the parent factory),
/// so the worker-handle Register × request race could regress
/// independently if no test exercises it.  We probe the
/// guarantee the same way as
/// `create_renderer_handle_synchronously_registers_cid`: issue
/// `cancel_fetch` for a freshly-allocated id immediately after
/// creation and assert the broker emits a synthetic
/// `Err("aborted")` (which only fires when the cid is in
/// `clients`, i.e. Register has been processed).
#[test]
fn create_sibling_handle_synchronously_registers_cid() {
    let np = spawn_network_process(test_client());
    let parent = np.create_renderer_handle();
    let sibling = parent.create_sibling_handle();

    let probe = FetchId::next();
    assert!(
        sibling.cancel_fetch(probe),
        "fresh sibling's cancel_fetch send failed — handle should be live post-#10.6c"
    );

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut saw = false;
    while std::time::Instant::now() < deadline {
        let events = sibling.drain_events();
        if events.iter().any(|ev| {
            matches!(
                ev,
                NetworkToRenderer::FetchResponse(rid, Err(msg))
                    if *rid == probe && msg.contains("aborted")
            )
        }) {
            saw = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        saw,
        "broker did not emit synthetic 'aborted' reply for post-create-sibling CancelFetch \
         within 1s — Register did not happen-before CancelFetch on the sibling code path"
    );

    np.shutdown();
}

/// Slot #10.6c regression: when the broker has already
/// exited, `NetworkProcessHandle::create_renderer_handle`
/// must return promptly with a pre-unregistered handle whose
/// layer-1 short-circuit (slot #10.6b) fires on every
/// operation, rather than blocking for the full ack timeout.
///
/// Construction: spawn a real broker, take a `control_tx`
/// clone via `r0.control_tx` (the `pub(super)` field is
/// visible to this test module), send `Shutdown` through the
/// clone, and poll-spin on the same clone until a probe send
/// fails — the failure proves the broker thread has exited
/// and `control_rx` has been dropped.  Then call
/// `np.create_renderer_handle()`: the `register_with_ack`
/// helper observes the dead control channel on its first
/// `send`, takes the fast-fail branch, and returns
/// `pre_unregistered=true`.  The deterministic gate
/// observation is `cancel_fetch(FetchId::next()) == false`
/// (lesson #133): a pre-unregistered handle's
/// `cancel_fetch` short-circuits via `check_unregistered`
/// and returns `false` synchronously.
///
/// Wall-clock budget for the post-shutdown call is well below
/// the 500 ms `REGISTER_ACK_TIMEOUT` because the disconnected
/// path bypasses the recv altogether — anything above
/// hundreds of ms here would mean the fast-fail branch
/// failed to fire.
#[test]
fn create_renderer_handle_post_shutdown_returns_pre_unregistered() {
    let np = spawn_network_process(test_client());
    // First renderer is the donor of the control_tx clone we
    // use to manually shut the broker down.
    let r0 = np.create_renderer_handle();
    let ctrl = r0.control_tx.clone();

    // Send Shutdown.  Broker drains it, returns false from
    // `handle_control`, exits its main loop, and
    // `network_process_main` drops `control_rx` on return.
    ctrl.send(NetworkProcessControl::Shutdown)
        .expect("first Shutdown send should succeed against a live broker");

    // Poll-spin probe sends until the broker is observably
    // gone (send returns Err because control_rx has been
    // dropped).  2 s budget absorbs CI scheduler jitter; the
    // healthy path is microseconds.  Sending Shutdown
    // repeatedly is harmless: each is a no-op until the broker
    // exits, at which point the send fails.
    let probe_deadline = std::time::Instant::now() + Duration::from_secs(2);
    while ctrl.send(NetworkProcessControl::Shutdown).is_ok() {
        assert!(
            std::time::Instant::now() <= probe_deadline,
            "broker did not exit within 2 s of Shutdown"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // Broker is gone.  `np`'s `control_tx` clone is now
    // disconnected, so `register_with_ack` takes the
    // SendError fast-fail path and returns
    // `pre_unregistered=true`.  The wall-clock ceiling has
    // to thread the needle (Copilot R7 → R8): tight enough
    // to fail if a regression made the call wait the full
    // 500 ms `REGISTER_ACK_TIMEOUT` (R8 — the previous 2 s
    // bound was too loose to distinguish), but loose enough
    // to absorb CI-scheduler jitter on a correct fast-fail
    // (R7 — the original 500 ms bound was too tight).  300 ms
    // is the goldilocks: a regressed timeout-path call would
    // take ≥ 500 ms and therefore fail; a correct fast-fail
    // is sub-millisecond plus typical CI jitter (≤ 100 ms),
    // leaving ~200 ms of headroom.
    let started = std::time::Instant::now();
    let renderer = np.create_renderer_handle();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(300),
        "post-shutdown create_renderer_handle blocked for {elapsed:?} — \
         expected fast-fail via the SendError path (sub-ms + CI jitter); \
         a regression that fell through to the 500 ms REGISTER_ACK_TIMEOUT \
         recv path would land here, distinguishable by the 300 ms ceiling"
    );
    assert!(
        !renderer.cancel_fetch(FetchId::next()),
        "post-shutdown handle's cancel_fetch returned true — \
         handle should be pre-unregistered (slot #10.6b layer 1 gate)"
    );

    // np's Drop will send another Shutdown (errors silently)
    // and join the already-exited broker thread (returns
    // immediately).
}

/// Slot #10.6c (Copilot R9 + R10 + R11) regression: the broker
/// stores `true` (Release) into the renderer's shared
/// `unregistered` atomic BEFORE emitting the
/// [`NetworkToRenderer::RendererUnregistered`] marker on the
/// response channel.  This ordering — store happens-before
/// send — is the load-bearing invariant that lets a
/// concurrent observer (e.g. another renderer's
/// `create_sibling_handle` against this cid) detect the
/// unregister via an O(1) `Acquire` atomic load instead of
/// draining the parent's response channel.  Pre-R9 the
/// detection path was a bounded channel drain on the parent's
/// `response_rx`; that drain was unbounded in WS / SSE backlog
/// size and could trigger an unbounded `outstanding_fetches`
/// synthesis pass on hitting the marker.
///
/// **R11 strengthening**: this test verifies the **ordering**
/// directly, not just the eventual-set: it drains the parent's
/// `response_rx` until it observes the
/// [`NetworkToRenderer::RendererUnregistered`] marker, then
/// IMMEDIATELY does an `Acquire` load on a separate `Arc`
/// clone of `parent.unregistered`.  Because crossbeam's
/// `send` / `recv` establish a happens-before, and the
/// broker's `Release` store happens-before its `send`,
/// the test's `Acquire` load AFTER `recv` must see `true`
/// in any correct implementation.  A regression that reordered
/// the store after the send (or omitted it) would let the test
/// load `false` immediately after observing the marker — the
/// assertion catches it.  Pre-R11 the test only polled for
/// "atomic eventually becomes true", which would have passed
/// even with a regressed reordering since the store would
/// still execute on the broker thread's next instruction
/// (Copilot R11 F3).
#[test]
fn broker_sets_unregistered_atomic_synchronously_before_emitting_marker() {
    let np = spawn_network_process(test_client());
    let parent = np.create_renderer_handle();
    let cid = parent.client_id();

    // Clone the renderer-side Arc as a side observer.  We
    // never call any drain helper on `parent`, so this Arc's
    // value is influenced only by the broker's store on its
    // own clone via `ClientEntry::unregistered`.
    let observer = std::sync::Arc::clone(&parent.unregistered);

    np.unregister_renderer(cid);

    // Drain `parent.response_rx` directly (bypassing
    // `parent.drain_events` / `parent.cancel_fetch` so we don't
    // route through `process_response`, which would store
    // `true` on the renderer side and mask the broker-side
    // ordering invariant we're testing).  As soon as we
    // observe the `RendererUnregistered` marker, do an
    // `Acquire` load on the side observer: per crossbeam
    // happens-before and the R9 ordering contract (broker
    // `store(Release)` → `send`), the load must return `true`.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut saw_marker = false;
    while std::time::Instant::now() < deadline {
        match parent.response_rx.try_recv() {
            Ok(NetworkToRenderer::RendererUnregistered) => {
                saw_marker = true;
                assert!(
                    observer.load(Ordering::Acquire),
                    "broker emitted `RendererUnregistered` marker BEFORE flipping the \
                     shared `unregistered` atomic — slot #10.6c R9 ordering invariant \
                     violated.  `dispatch::emit_renderer_unregistered` must \
                     `unregistered.store(true, Release)` BEFORE \
                     `response_tx.send(RendererUnregistered)`, so any observer that \
                     syncs with the send (crossbeam happens-before) sees the prior \
                     store via an Acquire load."
                );
                break;
            }
            Ok(_other) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                panic!("response channel disconnected before marker arrived");
            }
        }
    }
    assert!(
        saw_marker,
        "broker did not emit `RendererUnregistered` marker within 2 s of \
         `np.unregister_renderer`; preconditions for the R9 ordering test not met"
    );

    np.shutdown();
}

/// Slot #10.6c (Copilot R6) regression:
/// `create_sibling_handle` must inherit the parent's
/// `unregistered` state at construction time and short-circuit
/// the broker round-trip entirely.  Without this, a parent
/// that was marked unregistered (either by observing the
/// `RendererUnregistered` back-edge or by a prior ack failure)
/// could still spawn a fresh, live sibling against a broker
/// that recovered between the parent's teardown and the
/// sibling call — leaving the embedder with the inconsistent
/// pair "broken parent / working child" that the slot #10.6c
/// fallback contract is meant to prevent.  The test induces
/// the parent's unregister via `np.unregister_renderer(cid)`,
/// gates on the deterministic `cancel_fetch == false` short-
/// circuit (lesson #133), and asserts (a) the sibling
/// creation completes in well under the 500 ms
/// `REGISTER_ACK_TIMEOUT` (no broker round-trip), and (b) the
/// sibling itself short-circuits via the same gate.
#[test]
fn create_sibling_handle_inherits_unregistered_parent_without_broker_roundtrip() {
    let np = spawn_network_process(test_client());
    let parent = np.create_renderer_handle();
    let cid = parent.client_id();

    np.unregister_renderer(cid);

    // Wait for the parent to observe the RendererUnregistered
    // marker.  Same gate pattern as the slot #10.6b post-
    // unregister tests.
    let gate_deadline = std::time::Instant::now() + Duration::from_secs(1);
    while std::time::Instant::now() < gate_deadline && parent.cancel_fetch(FetchId::next()) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !parent.cancel_fetch(FetchId::next()),
        "parent never observed RendererUnregistered marker within 1 s — preconditions not met"
    );

    // Sibling creation off an unregistered parent must short-
    // circuit (no register_with_ack round-trip).  The
    // functional discriminator is the `cancel_fetch == false`
    // assertion: without the parent-fast-path R6 short-circuit,
    // `register_with_ack` would succeed against the live
    // broker (parent's cid is gone but the broker is healthy
    // and would ack a fresh sibling cid), the sibling would be
    // constructed live, and `cancel_fetch` would return
    // `true` — failing the assertion.  A wall-clock ceiling
    // does not add discriminability here: both the
    // parent-fast-path (atomic load) and a hypothetical
    // bare-register-success path are sub-millisecond against a
    // healthy broker, so timing cannot tell them apart
    // (Copilot R11).
    let sibling = parent.create_sibling_handle();
    assert!(
        !sibling.cancel_fetch(FetchId::next()),
        "sibling spawned off an unregistered parent must inherit pre-unregistered state — \
         a working child of a broken parent is the inconsistency slot #10.6c R6 fixed; \
         a regression that removes the parent-fast-path would let the live broker register \
         a working sibling, making this `cancel_fetch` return true and the assertion fail"
    );

    np.shutdown();
}

/// Slot #10.6c regression: the same pre-unregistered fallback
/// covers `NetworkHandle::create_sibling_handle`.  A renderer
/// thread that races the parent's
/// `NetworkProcessHandle::shutdown` while constructing a Web
/// Worker handle must NOT block for the full
/// `REGISTER_ACK_TIMEOUT` — it must return quickly with a
/// handle whose `cancel_fetch` short-circuits.
///
/// **R10 note**: with the slot #10.6c R9 architectural fix
/// (broker stores `true` into the parent's shared
/// `unregistered` atomic before emitting the marker), an
/// orderly shutdown completes the parent's
/// `unregistered=true` flip BEFORE we observe the broker is
/// gone via the probe-spin below.  That means
/// `create_sibling_handle` here typically takes the parent-
/// fast-path R6 short-circuit (atomic-load → pre-unregistered)
/// rather than the SendError fast-fail in `register_with_ack`.
/// Both paths produce the same observable "pre-unregistered
/// sibling without broker round-trip" outcome, which is what
/// this test asserts.  The SendError fast-fail itself is
/// covered separately by
/// [`super::super::register::tests::send_error_fast_fail_when_broker_already_gone`]
/// — that unit test drives the helper directly with an
/// unregistered atomic that stays `false`, so it exercises the
/// fall-through path without needing a broker that races the
/// parent's atomic flip.
#[test]
fn create_sibling_handle_post_shutdown_returns_pre_unregistered() {
    let np = spawn_network_process(test_client());
    let parent = np.create_renderer_handle();
    let ctrl = parent.control_tx.clone();
    ctrl.send(NetworkProcessControl::Shutdown)
        .expect("first Shutdown send should succeed against a live broker");

    let probe_deadline = std::time::Instant::now() + Duration::from_secs(2);
    while ctrl.send(NetworkProcessControl::Shutdown).is_ok() {
        assert!(
            std::time::Instant::now() <= probe_deadline,
            "broker did not exit within 2 s of Shutdown"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // Wall-clock ceiling: 300 ms.  Both viable paths
    // (parent-fast-path / SendError fast-fail) are sub-ms;
    // 300 ms catches a regression that fell through to the
    // 500 ms `REGISTER_ACK_TIMEOUT` recv path.
    let started = std::time::Instant::now();
    let sibling = parent.create_sibling_handle();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(300),
        "post-shutdown create_sibling_handle blocked for {elapsed:?} — \
         expected pre-unregistered short-circuit (either parent-fast-path \
         or SendError fast-fail; both sub-ms); a regression that fell \
         through to the 500 ms REGISTER_ACK_TIMEOUT recv path would land \
         here, distinguishable by the 300 ms ceiling"
    );
    assert!(
        !sibling.cancel_fetch(FetchId::next()),
        "post-shutdown sibling's cancel_fetch returned true — \
         sibling should inherit the pre-unregistered fallback"
    );
}
