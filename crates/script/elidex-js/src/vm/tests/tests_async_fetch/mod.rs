//! M4-12 PR5-async-fetch: Promise/abort lifecycle tests for the
//! async `fetch()` path (WHATWG Fetch §5.1).
//!
//! These tests focus on behaviours that *depend* on the
//! broker-reply-via-`tick_network` indirection: the Promise stays
//! pending across the fetch dispatch, an explicit `tick_network`
//! call is required to settle it, and abort fan-out can interpose
//! between dispatch and reply.
//!
//! Round-trip happy-path coverage continues to live in
//! `tests_fetch.rs`; this file complements it with the
//! abort + dedup edges that the synchronous-blocking variant
//! could not exercise.
//!
//! The original 1204-line single file was split scenario-aligned to
//! keep each child under the 1000-line convention:
//!
//! - [`lifecycle`] — basic Promise lifecycle (`tick_network`,
//!   microtask drain), WS/SSE interaction with the same handle,
//!   `install_network_handle` semantics.
//! - [`abort`] — `controller.abort()` fan-out, GC rooting across
//!   abort rejection, broker-side `CancelFetch` wire.
//! - [`cors`] — PR5-cors Stages 3 / 4 / 5 (Origin / redirect /
//!   credentials thread, `Response.type` classification matrix,
//!   cache-mode header injection) + signal back-ref pruning +
//!   the `pending_fetch_cors` meta-missing fail-closed regression.

#![cfg(feature = "engine")]

mod abort;
mod cors;
mod lifecycle;

use std::rc::Rc;

use elidex_net::broker::NetworkHandle;
use elidex_net::{HttpVersion, Response as NetResponse};

use super::super::Vm;

pub(super) fn ok_response(url: &str, body: &'static str) -> NetResponse {
    let parsed = url::Url::parse(url).expect("valid URL");
    NetResponse {
        status: 200,
        headers: vec![("content-type".to_string(), "text/plain".to_string())],
        body: bytes::Bytes::from_static(body.as_bytes()),
        url: parsed.clone(),
        version: HttpVersion::H1,
        url_list: vec![parsed],
        is_redirect_tainted: false,
        credentialed_network: false,
    }
}

pub(super) fn mock_vm(responses: Vec<(url::Url, Result<NetResponse, String>)>) -> Vm {
    let mut vm = Vm::new();
    // Default the document origin to `http://example.com/page` so
    // the lifecycle tests below (which fetch `http://example.com/...`
    // URLs) classify as same-origin → `Basic`.  Without this, the
    // default `about:blank` origin would become opaque after Copilot
    // R3 fix, making every fetch cross-origin and tripping the
    // `NetworkError` (no ACAO) path — these tests aren't about CORS,
    // they're about the Promise / abort lifecycle.
    vm.inner.navigation.current_url =
        url::Url::parse("http://example.com/page").expect("valid base URL");
    vm.install_network_handle(Rc::new(NetworkHandle::mock_with_responses(responses)));
    vm
}

pub(super) use super::drain_fetch_replies as drain;
