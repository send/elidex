//! Shared test helpers for the content-thread test modules — spawning a content
//! thread over a test network broker, building a `ContentState` to drive on the
//! test thread, and the browser-channel observers / DOM probes those modules
//! assert against. This is the single canonical home: a sibling content test
//! module imports from here rather than re-declaring its own copy.

use elidex_script_session::HostDriver;

use super::iframe::{BrowserToIframe, IframeEntry, IframeHandle, IframeToBrowser, InProcessIframe};
use super::{spawn_content_thread, ContentState};
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// A throwaway disk-backed [`WebStorageManager`] rooted at a unique temp dir, so
/// content-thread tests exercise the real construction-seam install path without
/// polluting (or reading) the shared platform profile. Tests that do not touch
/// `localStorage` simply never write to it.
pub(super) fn test_web_storage() -> std::sync::Arc<elidex_storage_core::WebStorageManager> {
    let dir = std::env::temp_dir().join(format!(
        "elidex-test-webstorage-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::sync::Arc::new(elidex_storage_core::WebStorageManager::new(dir))
}

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
    spawn_content_thread(
        content,
        nh,
        jar,
        test_web_storage(),
        html,
        css,
        viewport_cell,
        Box::new(|| {}),
    )
}

/// Shared setup **head** of the two `build_test_content_state*` builders: a
/// browser/content channel pair, a disconnected network handle, and the DEFAULT
/// viewport. Each builder constructs its own `PipelineResult` over these
/// (differing only in the `build_pipeline_*` entry point / cookie jar / URL),
/// then hands it to the shared [`finalize_test_content_state`] tail.
fn test_content_scaffold() -> (
    LocalChannel<BrowserToContent, ContentToBrowser>,
    LocalChannel<ContentToBrowser, BrowserToContent>,
    std::rc::Rc<elidex_net::broker::NetworkHandle>,
    elidex_plugin::Size,
) {
    let (browser, content) = crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
    let nh = std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected());
    let viewport = elidex_plugin::Size::new(
        crate::DEFAULT_VIEWPORT_WIDTH,
        crate::DEFAULT_VIEWPORT_HEIGHT,
    );
    (browser, content, nh, viewport)
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
    let (browser, content, nh, viewport) = test_content_scaffold();
    let jar = std::sync::Arc::new(elidex_net::CookieJar::new());
    let pipeline = crate::build_pipeline_interactive_with_network(
        html,
        css,
        nh,
        jar,
        test_web_storage(),
        viewport,
        crate::ipc::DeviceFacts::default(),
    );
    (
        finalize_test_content_state(content, pipeline, viewport),
        browser,
    )
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
    let (browser, content, nh, viewport) = test_content_scaffold();
    let pipeline = crate::build_pipeline_interactive_shared(
        html,
        Some(url),
        std::sync::Arc::new(elidex_text::FontDatabase::new()),
        nh,
        std::sync::Arc::new(crate::create_css_property_registry()),
        None,
        Some(test_web_storage()),
        viewport,
        crate::ipc::DeviceFacts::default(),
        // Top-level document: no frame security (origin derives from `url`).
        None,
    );
    (
        finalize_test_content_state(content, pipeline, viewport),
        browser,
    )
}

/// Wrap a freshly built pipeline into a driveable `ContentState` — the shared
/// tail of [`build_test_content_state`] / [`build_test_content_state_with_url`].
///
/// Seeds the viewport cell at the build size (seq 0 / facts_seq 0) → high-water
/// marks 0, matching the `build_pipeline_*` size the pipeline was built at; a
/// test `SetViewport` then applies with `seq ≥ 1`, a `SetDeviceFacts` with
/// `facts_seq ≥ 1`. Scans initial iframes and renders once so the state is
/// ready to drive.
fn finalize_test_content_state(
    content: LocalChannel<ContentToBrowser, BrowserToContent>,
    pipeline: crate::PipelineResult,
    viewport: elidex_plugin::Size,
) -> ContentState {
    let viewport_cell = crate::ipc::ViewportCell::new(viewport);
    let mut state = ContentState::new(
        content,
        elidex_navigation::NavigationController::new(),
        pipeline,
        test_web_storage(),
        Box::new(|| {}),
        viewport_cell,
        0,
        0,
        crate::ipc::DeviceFacts::default(),
    );
    super::scroll::update_viewport_scroll_dimensions(&mut state);
    super::iframe::scan_initial_iframes(&mut state);
    state.re_render();
    state
}

/// The top-level document URL the content test modules build against.
pub(super) fn base() -> url::Url {
    url::Url::parse("https://example.com/").unwrap()
}

/// Discard every message currently queued on the browser channel end so a later
/// measurement observes only post-drain sends.
pub(super) fn drain_browser(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) {
    while browser.try_recv().is_ok() {}
}

/// Count the `DisplayListReady` messages currently queued on the browser channel
/// — the "did the (possibly torn-down) pipeline ship a frame?" witness the
/// history drain / phase-separation / pump-turn modules measure after a drain.
pub(super) fn count_display_lists(
    browser: &LocalChannel<BrowserToContent, ContentToBrowser>,
) -> usize {
    let mut n = 0;
    while let Ok(msg) = browser.try_recv() {
        if matches!(msg, ContentToBrowser::DisplayListReady(_)) {
            n += 1;
        }
    }
    n
}

/// Two same-document entries `[base, /a]` (shared `document_sequence`), cursor on
/// `/a`, so a `back()` resolves same-document and applies in place (no fetch).
///
/// Mirrors a production same-document `pushState('/a')` (`content/navigation.rs`
/// `apply_push_replace_state`): the active-document facts — pipeline URL + VM
/// current-URL + VM session-history `(index, length)` — track the cursor, so the
/// fixture never leaves the production-impossible state where `location` /
/// `history.length` / relative-URL resolution describe `base` while the cursor sits on
/// `/a` (Codex PR#469 R18).
pub(super) fn seed_same_document_pair(state: &mut ContentState) {
    let a = url::Url::parse("https://example.com/a").unwrap();
    state.nav_controller.push(base()); // index 0, base
    state.nav_controller.push_same_document(a.clone()); // index 1, /a
    state.pipeline.url = Some(a);
    state
        .pipeline
        .runtime
        .set_current_url(state.pipeline.url.clone());
    state.pipeline.runtime.set_session_history(
        state.nav_controller.current_index(),
        state.nav_controller.len(),
    );
}

/// The single `<iframe>` entity in the parent DOM (the one carrying `IframeData`)
/// — the shared handle both iframe-driving test modules resolve before asserting
/// on the loaded entry / lazy-pending state.
pub(super) fn iframe_entity(state: &ContentState) -> elidex_ecs::Entity {
    (&mut state
        .pipeline
        .dom
        .world()
        .query::<(elidex_ecs::Entity, &elidex_ecs::IframeData)>())
        .into_iter()
        .next()
        .map(|(e, _)| e)
        .expect("an <iframe> entity carrying IframeData should exist")
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

/// The loaded entry for the single iframe, which must be in-process.
pub(super) fn in_process_entry(state: &ContentState) -> &InProcessIframe {
    let entity = iframe_entity(state);
    let entry: &IframeEntry = state
        .iframes
        .get(entity)
        .expect("the iframe should have loaded an entry");
    match &entry.handle {
        IframeHandle::InProcess(ip) => ip,
        IframeHandle::OutOfProcess(_) => panic!("expected an in-process iframe entry"),
    }
}

/// Synthesize the `LoadedDocument` a successful fetch of `url` would produce
/// (the test network cannot produce a real one): `html` parsed with
/// `parse_progressive_str` (strict first, tolerant fallback), as the loader
/// parses a fetched document.
pub(super) fn synth_loaded(html: &str, url: &str) -> elidex_navigation::LoadedDocument {
    let parsed = elidex_html_parser::parse_progressive_str(html);
    let scripts = elidex_navigation::extract_inline_scripts(&parsed.dom, parsed.document);
    elidex_navigation::LoadedDocument {
        dom: parsed.dom,
        document: parsed.document,
        stylesheets: Vec::new(),
        scripts,
        url: url::Url::parse(url).expect("test URL should parse"),
        response_headers: std::collections::HashMap::new(),
        manifest_url: None,
    }
}

/// Pump the OOP entry's channel until two `DisplayListReady` frames arrive,
/// then shut the thread down and return the collected `PostMessage`s.
///
/// Two frames, not one: the iframe thread's loop sends the frame's display
/// list BEFORE draining the bridge's pending `postMessage` queue in the same
/// iteration, so posts queued by the *initial* scripts (during the build,
/// before the loop) are on the channel only once the SECOND frame is —
/// channel delivery is FIFO. The two-frame requirement also proves the
/// pipeline built and the thread is alive (non-vacuity for the
/// no-message-expected case).
pub(super) fn run_oop_and_collect_posts(entry: IframeEntry) -> Vec<(String, String)> {
    let IframeHandle::OutOfProcess(mut oop) = entry.handle else {
        panic!("expected an out-of-process iframe entry");
    };
    let mut posts = Vec::new();
    let mut frames = 0_usize;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while frames < 2 && std::time::Instant::now() < deadline {
        match oop
            .channel
            .recv_timeout(std::time::Duration::from_millis(100))
        {
            Ok(IframeToBrowser::DisplayListReady(_)) => frames += 1,
            Ok(IframeToBrowser::PostMessage { data, origin, .. }) => posts.push((data, origin)),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        frames >= 2,
        "the OOP iframe thread never produced two frames — pipeline build \
         failed or thread died"
    );
    let _ = oop.channel.send(BrowserToIframe::Shutdown);
    if let Some(thread) = oop.thread.take() {
        thread
            .join()
            .expect("OOP iframe thread should join cleanly");
    }
    posts
}

/// Build a top-level (main-document) pipeline for `html` at
/// `https://example.com/` through `crate::build_pipeline_from_loaded`, the
/// builder every URL-loaded document goes through, over [`test_network`].
/// Returns the broker handle with the pipeline so the broker outlives it.
pub(super) fn build_main_document(
    html: &str,
) -> (
    crate::PipelineResult,
    elidex_net::broker::NetworkProcessHandle,
) {
    let (nh, jar, np) = test_network();
    let pipeline = crate::build_pipeline_from_loaded(
        synth_loaded(html, "https://example.com/"),
        std::rc::Rc::new(nh),
        std::sync::Arc::new(elidex_text::FontDatabase::new()),
        Some(jar),
        None,
        elidex_plugin::Size::new(
            crate::DEFAULT_VIEWPORT_WIDTH,
            crate::DEFAULT_VIEWPORT_HEIGHT,
        ),
        crate::ipc::DeviceFacts::default(),
        // Top-level document: no frame security (origin derives from the URL).
        None,
        None,
    );
    (pipeline, np)
}

/// Read attribute `attr` of the document's `<body>` element — a Rust-side
/// read of `pipeline.dom`, opening no script bracket.
pub(super) fn probe_body_attr(pipeline: &crate::PipelineResult, attr: &str) -> Option<String> {
    let body = pipeline.dom.query_by_tag("body").into_iter().next()?;
    pipeline
        .dom
        .world()
        .get::<&elidex_ecs::Attributes>(body)
        .ok()
        .and_then(|a| a.get(attr).map(str::to_owned))
}
