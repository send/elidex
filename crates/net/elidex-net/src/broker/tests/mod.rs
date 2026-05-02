//! Broker integration tests, organised by theme.  Originally a single
//! `broker/tests.rs`; split out of that file in PR-file-split-a Copilot
//! R13 (HL_O) once it crossed the project's 1000-line file convention
//! (the convention applies to test modules too — see
//! `crates/script/elidex-js/src/vm/tests/tests_readable_stream/mod.rs`
//! for the same pattern in the engine test tree).
//!
//! Layout:
//! - [`lifecycle`] — spawn / register / unregister / drain / id / multi-renderer / debug.
//! - [`fetch`] — `fetch_blocking` + `fetch_async` happy paths, plus the
//!   disconnected-handle synthetic disconnect.
//! - [`cancel`] — `cancel_fetch` synthetic reply, inflight-slot release,
//!   cancel-spam regression, unknown-id idempotence, cross-client
//!   cancel isolation.
//! - [`teardown`] — `UnregisterRenderer` / broker `Shutdown` / the
//!   realtime-only `RendererToNetwork::Shutdown` semantics, stale-handle
//!   gating, and synthetic-aborted-reply delivery on teardown.
//!
//! [`test_client`] is the shared NetClient builder — every sub-file
//! imports it via `use super::test_client;`.

#![cfg(test)]

use crate::{NetClient, NetClientConfig, TransportConfig};

mod cancel;
mod fetch;
mod lifecycle;
mod teardown;

pub(super) fn test_client() -> NetClient {
    NetClient::with_config(NetClientConfig {
        transport: TransportConfig {
            allow_private_ips: true,
            // Lift per-origin and global connection caps well
            // above `MAX_CONCURRENT_FETCHES` so cancel-spam
            // regression tests can keep ≥`MAX_CONCURRENT_FETCHES`
            // workers genuinely stalled on transport IO.  With
            // the production defaults (`6` per-origin, `256`
            // total) most workers in those tests would fail
            // fast on the per-origin cap inside
            // `pool::create_connection` — that's a different
            // error path than the cancel-vs-stall race those
            // tests are meant to exercise (Copilot R1).
            max_connections_per_origin: 256,
            max_total_connections: 1024,
            ..Default::default()
        },
        ..Default::default()
    })
}
