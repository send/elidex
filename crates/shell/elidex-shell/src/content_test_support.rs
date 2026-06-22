//! Shared test helpers for the content-thread test modules
//! (`content_tests`, `viewport_tests`) — spawning a content thread over a test
//! network broker. Kept in one place so neither test module owns the scaffolding.

use super::spawn_content_thread;
use crate::ipc::{BrowserToContent, ContentToBrowser};

/// Create a `NetworkHandle` + `CookieJar` backed by a test broker.
/// Returns the `NetworkProcessHandle` so the caller keeps the broker alive.
pub(super) fn test_network() -> (
    elidex_net::broker::NetworkHandle,
    std::sync::Arc<elidex_net::CookieJar>,
    elidex_net::broker::NetworkProcessHandle,
) {
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let nh = np.create_renderer_handle();
    let jar = std::sync::Arc::clone(np.cookie_jar());
    (nh, jar, np)
}

/// Spawn a content thread for tests with a **no-op** wake. The wake mechanism
/// itself (PR-A repaint-wake) is exercised by
/// `content_thread_wake_fires_on_display_list`; every other test only needs the
/// content thread to run, so it injects a do-nothing `WakeHandle`.
///
/// Spawns at the DEFAULT viewport (the window-less explicit choice, D6). Tests
/// whose initial scripts must observe a specific `innerWidth`/`matchMedia`
/// threshold spawn at that size instead via [`spawn_test_content_sized`].
pub(super) fn spawn_test_content(
    content: crate::ipc::LocalChannel<ContentToBrowser, BrowserToContent>,
    nh: elidex_net::broker::NetworkHandle,
    jar: std::sync::Arc<elidex_net::CookieJar>,
    html: String,
    css: String,
) -> std::thread::JoinHandle<()> {
    let viewport = elidex_plugin::Size::new(
        crate::DEFAULT_VIEWPORT_WIDTH,
        crate::DEFAULT_VIEWPORT_HEIGHT,
    );
    spawn_test_content_sized(content, nh, jar, html, css, viewport)
}

/// Spawn a content thread for tests at an **explicit** `viewport`. C1 seeds both
/// the JS bridge (`window.innerWidth`/`matchMedia`) and the CSS cascade from this
/// size in `run_scripts_and_finalize` **before** initial scripts run, so a test
/// asserting an initial-load `mql.matches` / `innerWidth` value must spawn at the
/// size that produces the intended state (e.g. below a `min-width` threshold to
/// later cross it on resize).
pub(super) fn spawn_test_content_sized(
    content: crate::ipc::LocalChannel<ContentToBrowser, BrowserToContent>,
    nh: elidex_net::broker::NetworkHandle,
    jar: std::sync::Arc<elidex_net::CookieJar>,
    html: String,
    css: String,
    viewport: elidex_plugin::Size,
) -> std::thread::JoinHandle<()> {
    spawn_content_thread(content, nh, jar, html, css, viewport, Box::new(|| {}))
}
