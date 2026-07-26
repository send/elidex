//! S5-5b — same-document (fragment) navigation: the app-mode (legacy inline)
//! no-rebuild branch + the reload/replace nav-type distinction.
//!
//! App-mode is GET-only and honors the [`NavigationType`] directly, so its
//! Fragment-branch gate is `classify_navigation == SameDocument AND nav_type !=
//! Reload`. These tests build an `App` via [`App::new_interactive_with_url`] (no
//! winit — `render_state` is `None`) over a **disconnected** network, so a
//! REBUILD `load_document` fails (leaving `pipeline.url` + the controller
//! unchanged) while a FRAGMENT nav takes the no-rebuild path (updating
//! `pipeline.url` in place + committing an entry). The reload/replace collision
//! the `NavigationType` enum fixes (a `replace: bool` conflated them) is pinned
//! via the scroll side effect: a `Replace('#x')` fragment-skips (and scrolls to
//! `#x`), a `Reload` rebuilds (no fragment-skip scroll).

use elidex_script_session::NavigationType;

use super::test_support::{app_at, base, history_len, pipeline_url, url};
use super::App;

fn scroll_y(app: &App) -> f32 {
    app.interactive.as_ref().unwrap().pipeline.scroll_offset.y
}

/// An app-mode fragment nav takes the no-rebuild path: `pipeline.url` updates in
/// place (a rebuild would fail on the disconnected network, leaving it unchanged)
/// and one history entry is committed.
#[test]
fn app_fragment_nav_no_rebuild_pushes_entry() {
    let mut app = app_at("<p>doc</p>", base());
    assert_eq!(history_len(&app), 1, "the initial entry is seeded");

    app.navigate(&url("https://example.com/#sec"), NavigationType::Push);

    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/#sec"),
        "pipeline.url is updated in place (no rebuild — a failed rebuild would leave it unchanged)"
    );
    assert_eq!(history_len(&app), 2, "the fragment nav pushed one entry");
}

/// A JS `location.reload()` of a fragment-URL page rebuilds and does NOT grow
/// history: the `nav_type != Reload` gate keeps the reload off the no-rebuild
/// path (without it, `Reload` would fragment-skip-push → `len == 2`).
#[test]
fn app_js_reload_of_fragment_url_no_history_growth() {
    let mut app = app_at("<p>doc</p>", url("https://example.com/a#x"));
    assert_eq!(history_len(&app), 1);

    // location.reload() → a Reload navigation to the current URL.
    let _ = app
        .interactive
        .as_mut()
        .unwrap()
        .pipeline
        .runtime
        .vm()
        .eval("location.reload();");
    app.process_pending_navigation();

    assert_eq!(
        history_len(&app),
        1,
        "reload rebuilds (Reload → no cursor move), NOT a fragment-skip push (which would be len 2)"
    );
}

/// A chrome `ChromeAction::Reload` of a fragment-URL page rebuilds (does NOT take
/// the fragment no-rebuild path): the round-3 IMP pin that chrome reload maps to
/// `NavigationType::Reload`, not `Replace`. If it were `Replace`, the fragment
/// branch would fire and scroll to the off-screen `#x` target; a correct `Reload`
/// leaves the scroll at the top.
#[test]
fn app_chrome_reload_of_fragment_url_does_not_fragment_skip() {
    let html = r#"<div style="height:2000px"></div><div id="x">X</div>"#;
    let mut app = app_at(html, url("https://example.com/a#x"));
    assert_eq!(scroll_y(&app), 0.0, "starts at the top");

    app.handle_chrome_action(crate::chrome::ChromeAction::Reload);

    assert_eq!(
        scroll_y(&app),
        0.0,
        "chrome Reload rebuilds (Reload, not Replace) → it does NOT fragment-skip-scroll to #x"
    );
    assert_eq!(history_len(&app), 1, "chrome reload does not grow history");
}

/// `location.replace('#x')` is distinguished from a reload (the collision the
/// `NavigationType` enum fixes): it is a SameDocument nav → fragment-skip that
/// REPLACES the entry in place (app-mode honors replace) AND scrolls to `#x`.
/// Contrast `app_chrome_reload_of_fragment_url_does_not_fragment_skip` — a reload
/// of the same shape does NOT scroll.
#[test]
fn app_replace_fragment_is_distinguished_from_reload() {
    let html = r#"<div style="height:2000px"></div><div id="x">X</div>"#;
    let mut app = app_at(html, base());
    assert_eq!(history_len(&app), 1);
    assert_eq!(scroll_y(&app), 0.0, "starts at the top");

    // location.replace('#x') → a Replace navigation to /#x (SameDocument).
    app.navigate(&url("https://example.com/#x"), NavigationType::Replace);

    assert_eq!(
        history_len(&app),
        1,
        "replace replaces the entry in place (no new entry — app-mode honors Replace)"
    );
    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/#x"),
        "the URL is updated in place (no rebuild)"
    );
    assert!(
        scroll_y(&app) > 0.0,
        "replace('#x') fragment-skips and scrolls to the off-screen #x target"
    );
}
