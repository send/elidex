//! Shared test helpers for the app-mode (legacy inline) test modules
//! (`app_fragment_nav_tests`, `app_history_drain_tests`) — building a driveable
//! `App` over a disconnected network, plus the history/URL probes both modules
//! assert on. Kept in one place so neither test module owns the scaffolding, and
//! so the long `build_pipeline_interactive_shared` call (whose signature has
//! churned) has ONE app-mode caller to update.
//!
//! **App-local on purpose — NOT a fold into `content_test_support`.** That module
//! is content-thread specific: it spawns content threads over a test broker,
//! builds `ContentState`, and its `drain_browser` consumes a browser IPC channel
//! an inline `App` does not have (inline mode is synchronous). Folding these
//! helpers there would couple app-mode tests to that scaffolding for no gain
//! (plan §5: a FALSE unification target).

use super::App;

/// The top-level document URL every app-mode test builds against.
pub(super) fn base() -> url::Url {
    url::Url::parse("https://example.com/").unwrap()
}

pub(super) fn url(s: &str) -> url::Url {
    url::Url::parse(s).unwrap()
}

/// Build an app-mode `App` at `url` over a disconnected network, laid out (so hit
/// testing and fragment scroll-resolution see `LayoutBox`es).
/// `new_interactive_with_url` seeds the initial history entry from `pipeline.url`
/// (so `len` starts at 1, cursor at index 0).
pub(super) fn app_at(html: &str, url: url::Url) -> App {
    let pipeline = crate::build_pipeline_interactive_shared(
        html,
        Some(url),
        std::sync::Arc::new(elidex_text::FontDatabase::new()),
        std::rc::Rc::new(elidex_net::broker::NetworkHandle::disconnected()),
        std::sync::Arc::new(crate::create_css_property_registry()),
        None,
        None, // No WebStorageManager (app-mode test → in-memory fallback).
        elidex_plugin::Size::new(1024.0, 768.0),
        crate::ipc::DeviceFacts::default(),
        None,
    );
    let mut app = App::new_interactive_with_url(pipeline, "elidex".to_string());
    crate::re_render(&mut app.interactive.as_mut().unwrap().pipeline);
    app
}

/// The session-history entry count (`history.length`'s shell-side source).
pub(super) fn history_len(app: &App) -> usize {
    app.interactive.as_ref().unwrap().nav_controller.len()
}

/// The ACTIVE DOCUMENT's URL (`pipeline.url`) — distinct from the history cursor's
/// entry URL, which is what makes it the probe for "did the document actually
/// navigate" as opposed to "did the cursor move".
pub(super) fn pipeline_url(app: &App) -> Option<String> {
    app.interactive
        .as_ref()
        .unwrap()
        .pipeline
        .url
        .as_ref()
        .map(|u| u.as_str().to_string())
}
