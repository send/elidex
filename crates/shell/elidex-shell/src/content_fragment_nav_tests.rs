//! S5-5b — same-document (fragment) navigation: the content-thread no-rebuild
//! branch + unload gating.
//!
//! Boa is the live shell engine until the S5-6 flip, so the popstate/hashchange
//! FIRING is VM-tested (flip-inert — boa stubs `deliver_history_step_events`).
//! These tests pin the **engine-agnostic-now** observable behavior: a fragment
//! nav does NOT rebuild the pipeline (no `load_document`, so it *succeeds* over
//! the disconnected test network where a rebuild `Err`s), commits exactly one
//! history entry, scrolls to the indicated element, preserves focus, keeps the
//! document origin stable by construction, and — the reload / removal / unload
//! pins — a `location.reload()` / fragment removal / body-bearing nav still
//! rebuilds, and a same-page `#fragment` address-bar nav fires no unload.
//!
//! Oracle: the disconnected `NetworkHandle` makes every REBUILD `load_document`
//! fail (`handle_navigate` → `false` + a `NavigationFailed` on the channel,
//! `state.pipeline` unchanged), while a FRAGMENT nav takes the no-rebuild path
//! (→ `true`, no `NavigationFailed`, `pipeline.url` updated in place). So
//! "rebuilds vs no-rebuild" reads off `handle_navigate`'s bool + the channel.

use elidex_script_session::HostDriver;

use super::navigation::{handle_navigate, process_pending_actions, HistoryCursorOp};
use super::test_support::build_test_content_state_with_url;
use super::ContentState;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// The top-level document URL most tests build against.
fn base() -> url::Url {
    url::Url::parse("https://example.com/").unwrap()
}

fn url(s: &str) -> url::Url {
    url::Url::parse(s).unwrap()
}

/// Discard every message currently queued on the browser channel end.
fn drain_browser(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) {
    while browser.try_recv().is_ok() {}
}

/// Whether a `NavigationFailed` was shipped — the disconnected-network signature
/// of a REBUILD attempt (a fragment nav never fetches, so it never fails).
fn saw_navigation_failed(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) -> bool {
    let mut failed = false;
    while let Ok(msg) = browser.try_recv() {
        if matches!(msg, ContentToBrowser::NavigationFailed { .. }) {
            failed = true;
        }
    }
    failed
}

/// The absolute border-box top of the `#id` element in document coordinates —
/// the offset a fragment nav to `#id` should land the viewport on (pre-clamp).
fn element_top(state: &ContentState, id: &str) -> f32 {
    let entity = state
        .pipeline
        .dom
        .find_by_id(state.pipeline.document, id)
        .expect("element with id exists");
    state
        .pipeline
        .dom
        .world()
        .get::<&elidex_plugin::LayoutBox>(entity)
        .expect("laid-out element has a LayoutBox")
        .border_box()
        .origin
        .y
}

/// A fresh fragment navigation does NO fetch (takes the no-rebuild path) and
/// commits exactly one history entry, updating `pipeline.url` in place.
#[test]
fn fragment_nav_no_refetch_pushes_one_entry() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    drain_browser(&browser);

    let target = url("https://example.com/#sec");
    let ok = handle_navigate(&mut state, &target, HistoryCursorOp::Push, None);

    assert!(
        ok,
        "a fragment nav takes the no-rebuild path → succeeds (no load)"
    );
    assert!(
        !saw_navigation_failed(&browser),
        "a fragment nav does not re-fetch (a rebuild would fail on the disconnected network)"
    );
    assert_eq!(
        state.nav_controller.len(),
        1,
        "a fragment nav commits exactly one history entry (push)"
    );
    assert_eq!(
        state.pipeline.url.as_ref().map(url::Url::as_str),
        Some("https://example.com/#sec"),
        "pipeline.url is updated in place to the fragment URL"
    );
    assert_eq!(
        state
            .pipeline
            .runtime
            .current_url()
            .as_ref()
            .map(url::Url::as_str),
        Some("https://example.com/#sec"),
        "the runtime current_url (location.*/document.URL) is updated too"
    );
}

/// A same-document navigation to the URL the active entry ALREADY has (including
/// the fragment) REPLACES rather than pushes (WHATWG HTML §7.4.2.2 step 13 auto
/// history-handling / §7.4.2.3.3 step 7), so `location.href = location.href` or
/// re-clicking the current `#id` does NOT grow `history.length`.
#[test]
fn fragment_nav_to_identical_url_replaces_not_pushes() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    drain_browser(&browser);

    let target = url("https://example.com/#sec");
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));
    let len_after_first = state.nav_controller.len();

    // Navigate to the SAME URL (including the fragment) again — must REPLACE.
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));
    assert_eq!(
        state.nav_controller.len(),
        len_after_first,
        "re-navigating to the identical fragment URL replaces (no history growth)"
    );
}

/// The scroll lands on the `#id` element AND the resolved offset reaches BOTH
/// the display-list scroll offset and the JS-observable `scrollX`/`scrollY` echo
/// (not shipped un-applied) — I6, via the post-layout `re_render` seam.
#[test]
fn fragment_nav_scrolls_to_id_and_echoes_offset() {
    // A tall page so the `#sec` target sits well within the scrollable range
    // (spacer before + a taller spacer after ⇒ no clamping).
    let html = r#"<div style="height:1000px"></div><div id="sec" style="height:20px">S</div><div style="height:2000px"></div>"#;
    let (mut state, browser) = build_test_content_state_with_url(html, base());
    assert_eq!(
        state.viewport_scroll.scroll_offset.y, 0.0,
        "initial scroll is at the top"
    );
    drain_browser(&browser);

    let target = url("https://example.com/#sec");
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));

    let top = element_top(&state, "sec");
    assert!(
        top > 0.0,
        "the target sits below the spacer (real scroll offset)"
    );
    assert!(
        (state.viewport_scroll.scroll_offset.y - top).abs() < f32::EPSILON,
        "the viewport scroll lands on the #sec element's top ({top}), got {}",
        state.viewport_scroll.scroll_offset.y
    );
    assert!(
        (state.pipeline.scroll_offset.y - top).abs() < f32::EPSILON,
        "the resolved offset reaches the pipeline display-list scroll offset"
    );
    #[allow(clippy::cast_possible_truncation)]
    // test compares an f32-rendered coord; the f64→f32 narrowing is intended
    let scroll_y = state.pipeline.runtime.eval_f64("window.scrollY") as f32;
    assert!(
        (scroll_y - top).abs() < f32::EPSILON,
        "the resolved offset is echoed to window.scrollY (not shipped un-applied)"
    );
}

/// An empty fragment (`#`) scrolls to the top of the document.
#[test]
fn fragment_nav_empty_hash_scrolls_to_top() {
    let html = r#"<div style="height:1000px"></div><div id="sec">S</div><div style="height:2000px"></div>"#;
    let (mut state, browser) = build_test_content_state_with_url(html, base());
    // Pre-scroll away from the top so the empty-# reset is observable.
    state.viewport_scroll.scroll_offset = elidex_plugin::Vector::new(0.0, 500.0);
    drain_browser(&browser);

    let target = url("https://example.com/#");
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));

    assert_eq!(
        state.viewport_scroll.scroll_offset.y, 0.0,
        "an empty fragment scrolls to the top of the document"
    );
}

/// Focus persists across a fragment nav — the no-rebuild branch keeps the
/// existing `EcsDom` (and its `ElementState::FOCUS`), the exact bit a rebuild
/// would have reset (I3).
#[test]
fn fragment_nav_preserves_focus() {
    let html = r#"<input id="f"><div style="height:2000px"></div><div id="sec">S</div>"#;
    let (mut state, browser) = build_test_content_state_with_url(html, base());
    let input = state
        .pipeline
        .dom
        .find_by_id(state.pipeline.document, "f")
        .expect("the <input> exists");
    crate::content::focus::set_focus(&mut state.pipeline, input);
    assert_eq!(
        elidex_dom_api::focus::current_focus(&state.pipeline.dom, state.pipeline.document),
        Some(input),
        "the input is focused before the fragment nav"
    );
    drain_browser(&browser);

    let target = url("https://example.com/#sec");
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));

    assert_eq!(
        elidex_dom_api::focus::current_focus(&state.pipeline.dom, state.pipeline.document),
        Some(input),
        "focus persists across a fragment nav (the no-rebuild branch never resets it)"
    );
}

/// The document origin is unchanged across a fragment nav in a top-level doc:
/// `set_current_url` re-derives the same URL-tuple origin (only the fragment
/// changed) and never touches the installed override — so fetch/WS keep keying
/// on the unchanged origin (I2 / §6.5, closes `#11-vm-navigation-origin-resync`).
#[test]
fn fragment_nav_top_level_origin_unchanged() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // Install the tuple origin the real load computes.
    let tuple = elidex_plugin::SecurityOrigin::from_url(&base());
    state.pipeline.runtime.set_origin(tuple.clone());
    drain_browser(&browser);

    let target = url("https://example.com/#sec");
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));

    assert_eq!(
        state.pipeline.runtime.origin(),
        tuple,
        "the tuple origin is unchanged across the fragment nav"
    );
}

/// The document origin is unchanged across a fragment nav in a **sandboxed
/// opaque** context: the same `Opaque(id)` survives (by-construction, no active
/// resync) — the `#11-vm-navigation-origin-resync` closure test for the opaque
/// case (fetch/`new WebSocket()` still key on the isolated opaque origin).
#[test]
fn fragment_nav_sandboxed_opaque_origin_unchanged() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // A sandboxed iframe installs a unique opaque origin over the URL tuple.
    let opaque = elidex_plugin::SecurityOrigin::opaque();
    state.pipeline.runtime.set_origin(opaque.clone());
    drain_browser(&browser);

    let target = url("https://example.com/#sec");
    assert!(handle_navigate(
        &mut state,
        &target,
        HistoryCursorOp::Push,
        None
    ));

    let after = state.pipeline.runtime.origin();
    assert!(
        matches!(after, elidex_plugin::SecurityOrigin::Opaque(_)),
        "the origin stays opaque"
    );
    assert_eq!(
        after, opaque,
        "the SAME opaque origin survives — set_current_url never touches the override"
    );
}

/// A fragment **removal** (`/a#x → /a`, target fragment null) is cross-document
/// per navigate step 15 and still REBUILDS (the corrected-classifier regression
/// pin) — over the disconnected network the rebuild fails.
#[test]
fn fragment_removal_rebuilds() {
    let (mut state, browser) =
        build_test_content_state_with_url("<p>doc</p>", url("https://example.com/a#x"));
    drain_browser(&browser);

    // `/a#x → /a`: target fragment is null ⇒ CrossDocument.
    let target = url("https://example.com/a");
    let ok = handle_navigate(&mut state, &target, HistoryCursorOp::Push, None);

    assert!(
        !ok,
        "fragment removal is cross-document → rebuild (fails on the disconnected network)"
    );
    assert!(
        saw_navigation_failed(&browser),
        "a cross-document rebuild attempts a fetch → NavigationFailed"
    );
    assert_eq!(
        state.pipeline.url.as_ref().map(url::Url::as_str),
        Some("https://example.com/a#x"),
        "the failed rebuild leaves pipeline.url unchanged (no no-rebuild skip)"
    );
}

/// `location.reload()` of a fragment-URL page (`/a#x`) REBUILDS (re-fetches) and
/// does NOT grow history — the drain maps `Reload → Keep`, and the `cursor_op ==
/// Push` guard keeps a reload off the no-rebuild path (the 5b-introduced-
/// regression pin, §5.2(a)).
#[test]
fn reload_of_fragment_url_rebuilds_without_history_growth() {
    let (mut state, browser) =
        build_test_content_state_with_url("<p>doc</p>", url("https://example.com/a#x"));
    state.nav_controller.push(url("https://example.com/a#x")); // seed the entry (len 1)

    // JS `location.reload()` enqueues a Reload navigation to the current URL.
    let _ = state.pipeline.runtime.vm().eval("location.reload();");
    drain_browser(&browser);

    assert!(process_pending_actions(&mut state));

    assert!(
        saw_navigation_failed(&browser),
        "reload REBUILDS (re-fetches) → NavigationFailed, NOT a no-rebuild fragment skip"
    );
    assert_eq!(
        state.nav_controller.len(),
        1,
        "reload does not grow history (Reload → Keep, no cursor push)"
    );
}

/// A body-bearing navigation to a same-page `#fragment` (`request = Some(...)`)
/// is cross-document (a full nav, the body is sent), NOT a fragment skip — the
/// `request.is_none()` guard (§4.3 point 5 / §6.3).
#[test]
fn body_bearing_same_page_fragment_is_cross_document() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    drain_browser(&browser);

    let target = url("https://example.com/#sec"); // URL alone would classify SameDocument
    let request = elidex_net::Request {
        method: "POST".to_string(),
        url: target.clone(),
        ..Default::default()
    };
    let ok = handle_navigate(&mut state, &target, HistoryCursorOp::Push, Some(request));

    assert!(
        !ok,
        "a body-bearing same-page #fragment nav is a full (cross-document) nav → rebuild fails"
    );
    assert!(
        saw_navigation_failed(&browser),
        "the body-bearing nav rebuilds (sends the body) → NavigationFailed"
    );
}

/// A same-page `#fragment` **address-bar** nav (`BrowserToContent::Navigate`)
/// fires NO unload/beforeunload and does not rebuild: unload is gated on
/// CrossDocument, so a cancelling `beforeunload` handler is never consulted and
/// the fragment nav PROCEEDS (the unload-gating IMP, §6.3 caller-audit).
#[test]
fn addressbar_fragment_nav_skips_unload_and_does_not_rebuild() {
    let html = r"<div>doc</div><script>document.addEventListener('beforeunload', function(e) { e.preventDefault(); });</script>";
    let (mut state, browser) = build_test_content_state_with_url(html, base());
    drain_browser(&browser);

    super::event_loop::handle_message_public(
        BrowserToContent::Navigate(url("https://example.com/#sec")),
        &mut state,
    );

    assert_eq!(
        state.pipeline.url.as_ref().map(url::Url::as_str),
        Some("https://example.com/#sec"),
        "the same-page #fragment nav PROCEEDS — unload was gated off, so the cancelling \
         beforeunload handler never ran to block it"
    );
    assert!(
        !saw_navigation_failed(&browser),
        "the fragment nav does not rebuild → no re-fetch"
    );
    assert_eq!(
        state.nav_controller.len(),
        1,
        "the fragment nav pushed one entry"
    );
}

/// A cross-document **address-bar** nav DOES fire beforeunload: the cancelling
/// handler blocks the navigation (`dispatch_unload_events` returns false), so
/// `pipeline.url` is unchanged and no fetch is even attempted — the contrast that
/// proves the fragment path above genuinely skipped unload.
#[test]
fn addressbar_cross_document_nav_fires_unload() {
    let html = r"<div>doc</div><script>document.addEventListener('beforeunload', function(e) { e.preventDefault(); });</script>";
    let (mut state, browser) = build_test_content_state_with_url(html, base());
    drain_browser(&browser);

    super::event_loop::handle_message_public(
        BrowserToContent::Navigate(url("https://example.com/other")),
        &mut state,
    );

    assert_eq!(
        state.pipeline.url.as_ref().map(url::Url::as_str),
        Some("https://example.com/"),
        "a cross-document address-bar nav fires beforeunload, which cancels here → nav blocked"
    );
    assert!(
        !saw_navigation_failed(&browser),
        "beforeunload blocked the nav BEFORE any fetch (no NavigationFailed)"
    );
}
