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
