//! Broker integration tests for lifecycle (spawn / register / unregister / drain / id / multi-renderer / debug).
//!
//! Sub-module of `broker::tests`; helpers (e.g. `test_client`) and
//! shared imports come from `super` (`tests/mod.rs`).

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
    // `pre_unregistered=true`.  Assert wall-clock and
    // observable state.
    let started = std::time::Instant::now();
    let renderer = np.create_renderer_handle();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "post-shutdown create_renderer_handle blocked for {elapsed:?} — \
         expected sub-100 ms via the SendError fast-fail path, well below \
         the 500 ms REGISTER_ACK_TIMEOUT recv ceiling"
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
    // circuit (no register_with_ack round-trip).  The 50 ms
    // budget is generous: we expect this to be a few atomic
    // loads + a struct construction.
    let started = std::time::Instant::now();
    let sibling = parent.create_sibling_handle();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "create_sibling_handle on unregistered parent blocked for {elapsed:?} — \
         expected a sub-50 ms short-circuit, broker round-trip should have been skipped"
    );
    assert!(
        !sibling.cancel_fetch(FetchId::next()),
        "sibling spawned off an unregistered parent must inherit pre-unregistered state — \
         a working child of a broken parent is the inconsistency slot #10.6c R6 fixed"
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

    let started = std::time::Instant::now();
    let sibling = parent.create_sibling_handle();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "post-shutdown create_sibling_handle blocked for {elapsed:?} — \
         expected <500 ms via the SendError fast-fail path"
    );
    assert!(
        !sibling.cancel_fetch(FetchId::next()),
        "post-shutdown sibling's cancel_fetch returned true — \
         sibling should inherit the pre-unregistered fallback"
    );
}
