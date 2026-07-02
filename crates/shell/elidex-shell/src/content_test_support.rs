//! Shared test helpers for the content-thread test modules
//! (`content_tests`, `viewport_tests`) — spawning a content thread over a test
//! network broker, and building a `ContentState` to drive on the test thread.
//! Kept in one place so neither test module owns the scaffolding.

use super::{spawn_content_thread, ContentState};
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

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
    // The content thread reads its build size from the cell (seq 0), so a test that
    // later sends a real `SetViewport` must tag it `seq ≥ 1` to clear the build's
    // high-water mark. See [`crate::ipc::ViewportCell`].
    let viewport_cell = crate::ipc::ViewportCell::new(viewport);
    spawn_content_thread(content, nh, jar, html, css, viewport_cell, Box::new(|| {}))
}

/// Build a `ContentState` directly (over a disconnected network handle) for tests
/// that drive the content thread **synchronously on the test thread** — e.g.
/// iframe lifecycle, or `run_event_loop` shutdown handling (the pipeline is
/// `!Send`, so it cannot move to a spawned thread).
///
/// Returns the live browser channel end alongside the state so it stays in scope
/// — dropping it would disconnect the content channel.
pub(super) fn build_test_content_state(
    html: &str,
    css: &str,
) -> (
    ContentState,
    LocalChannel<BrowserToContent, ContentToBrowser>,
) {
    let (browser, content) = crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let nh = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
    let jar = std::sync::Arc::new(elidex_net::CookieJar::new());
    let viewport = elidex_plugin::Size::new(
        crate::DEFAULT_VIEWPORT_WIDTH,
        crate::DEFAULT_VIEWPORT_HEIGHT,
    );
    let pipeline = crate::build_pipeline_interactive_with_network(
        html,
        css,
        nh,
        jar,
        viewport,
        crate::ipc::DeviceFacts::default(),
    );
    // Build at the cell's seed (DEFAULT, seq 0 / facts_seq 0) → high-water marks 0,
    // matching the `build_pipeline_*` size above; a test `SetViewport` then applies with
    // `seq ≥ 1`, a `SetDeviceFacts` with `facts_seq ≥ 1`.
    let viewport_cell = crate::ipc::ViewportCell::new(viewport);
    let mut state = ContentState::new(
        content,
        elidex_navigation::NavigationController::new(),
        pipeline,
        Box::new(|| {}),
        viewport_cell,
        0,
        0,
    );
    super::scroll::update_viewport_scroll_dimensions(&mut state);
    super::iframe::scan_initial_iframes(&mut state);
    state.re_render();
    (state, browser)
}

/// Like [`build_test_content_state`], but with a top-level document **URL** —
/// so the parent pipeline carries a real tuple origin (`SecurityOrigin::from_url`)
/// and `state.pipeline.url` is `Some`. Iframe security tests use this: a
/// sandboxed child's opaque origin is only distinguishable from "inherited the
/// parent origin" when the parent origin is a tuple, and the srcdoc/blank
/// iframe build feeds the parent URL into the child pipeline as its base URL.
pub(super) fn build_test_content_state_with_url(
    html: &str,
    url: url::Url,
) -> (
    ContentState,
    LocalChannel<BrowserToContent, ContentToBrowser>,
) {
    let (browser, content) = crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let nh = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
    let viewport = elidex_plugin::Size::new(
        crate::DEFAULT_VIEWPORT_WIDTH,
        crate::DEFAULT_VIEWPORT_HEIGHT,
    );
    let pipeline = crate::build_pipeline_interactive_shared(
        html,
        Some(url),
        std::sync::Arc::new(elidex_text::FontDatabase::new()),
        nh,
        std::sync::Arc::new(crate::create_css_property_registry()),
        None,
        viewport,
        crate::ipc::DeviceFacts::default(),
        // Top-level document: no frame security (origin derives from `url`).
        None,
    );
    let viewport_cell = crate::ipc::ViewportCell::new(viewport);
    let mut state = ContentState::new(
        content,
        elidex_navigation::NavigationController::new(),
        pipeline,
        Box::new(|| {}),
        viewport_cell,
        0,
        0,
    );
    super::scroll::update_viewport_scroll_dimensions(&mut state);
    super::iframe::scan_initial_iframes(&mut state);
    state.re_render();
    (state, browser)
}

/// Read the value of an attribute on the `<div>` with the given `id` — the shared
/// DOM probe both content-thread test modules use to assert a script-driven
/// attribute mutation landed (after a `re_render` flush).
pub(super) fn probe_attr(pipeline: &crate::PipelineResult, id: &str, attr: &str) -> Option<String> {
    let entity = pipeline.dom.query_by_tag("div").into_iter().find(|&e| {
        pipeline
            .dom
            .world()
            .get::<&elidex_ecs::Attributes>(e)
            .ok()
            .is_some_and(|a| a.get("id") == Some(id))
    })?;
    pipeline
        .dom
        .world()
        .get::<&elidex_ecs::Attributes>(entity)
        .ok()
        .and_then(|a| a.get(attr).map(str::to_owned))
}
