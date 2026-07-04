//! S5-5a — history-before-navigation drain order + FIFO history drain.
//!
//! Pins the `process_pending_actions` drain reorder (WHATWG HTML §7.4.4): a
//! synchronous `pushState`/`replaceState` (its URL/history update already ran
//! during the script) must commit its `NavigationController` entry BEFORE an
//! async pipeline-replacing navigation supersedes. The old order drained the
//! navigation first and early-returned, stranding the history mutation.
//!
//! Boa is the live shell engine until the S5-6 flip, so the oracle is the
//! shell-owned `NavigationController` (entry commit + order) plus the browser
//! channel's `DisplayListReady` count (the no-redundant-double-send structure).
//! Navigations fail to load over the disconnected test network, so a stranded
//! navigation leaves only the committed history entries in the controller — the
//! clean new-vs-old signature.
//!
//! Boa's `pending_history` bridge slot is single last-wins, so a genuine
//! multi-`pushState` turn is only producible at the VM engine (post-flip,
//! `take_pending_history() -> Vec` of every action — covered by `elidex-js`
//! `tests_engine_s1c`). The multi-action FIFO-apply that the shell drain loop
//! (`for action in &pending_history`) relies on is pinned here at the
//! `handle_history_action` seam.

use elidex_script_session::{HistoryAction, NavigationRequest};

use super::navigation::{handle_history_action, process_pending_actions};
use super::test_support::build_test_content_state_with_url;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// The top-level document URL every test builds against.
fn base() -> url::Url {
    url::Url::parse("https://example.com/").unwrap()
}

fn push_state(path: &str) -> HistoryAction {
    HistoryAction::PushState {
        url: Some(path.to_string()),
        title: String::new(),
    }
}

fn replace_state(path: &str) -> HistoryAction {
    HistoryAction::ReplaceState {
        url: Some(path.to_string()),
        title: String::new(),
    }
}

fn nav_to(url: &str) -> NavigationRequest {
    NavigationRequest {
        url: url.to_string(),
        replace: false,
    }
}

/// Discard every message currently queued on the browser channel end so a later
/// [`count_display_lists`] measures only the post-drain sends.
fn drain_browser(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) {
    while browser.try_recv().is_ok() {}
}

/// Count the `DisplayListReady` messages currently queued on the browser channel.
fn count_display_lists(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) -> usize {
    let mut n = 0;
    while let Ok(msg) = browser.try_recv() {
        if matches!(msg, ContentToBrowser::DisplayListReady(_)) {
            n += 1;
        }
    }
    n
}

/// The core fix: a same-turn `pushState('/a'); location.href='/b'` commits the
/// pushState `/a` entry (drained FIRST) rather than dropping it. The old
/// navigation-first order early-returned and never drained the history, leaving
/// the controller empty.
#[test]
fn history_drains_before_navigation() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    let bridge = state.pipeline.runtime.bridge();
    bridge.set_pending_history(push_state("/a"));
    bridge.set_pending_navigation(nav_to("/b"));
    drain_browser(&browser);

    let processed = process_pending_actions(&mut state);

    assert!(processed, "a same-turn navigation is an own-context action");
    // The `/b` navigation fails to load over the disconnected test network, so
    // it never enters the controller — but the pushState `/a` entry, drained
    // FIRST now, is committed. Under the old (navigation-first) order the
    // navigation early-returned and the pushState was NEVER drained (len 0).
    assert_eq!(
        state.nav_controller.len(),
        1,
        "the pushState /a entry is committed (old order dropped it → len 0)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the committed entry is the pushState /a URL, applied BEFORE the /b navigation drained"
    );
    // No redundant double-send: the pushState (which renders nothing) adds no
    // display list; only the navigation's pre-send ships one.
    assert_eq!(
        count_display_lists(&browser),
        1,
        "history + navigation ships exactly the navigation's display list (no redundant history send)"
    );
}

/// Multiple synchronous pushStates in one turn commit in FIFO order. Boa's
/// single-slot bridge cannot hold two, so this pins the FIFO-apply the shell
/// drain loop relies on by applying the two actions in order via the same
/// `handle_history_action` the loop calls (the VM produces the real two-element
/// `Vec` post-flip — `elidex-js` `tests_engine_s1c`).
#[test]
fn multiple_pushstates_commit_in_fifo_order() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());

    handle_history_action(&mut state, &push_state("/a"));
    handle_history_action(&mut state, &push_state("/b"));

    assert_eq!(
        state.nav_controller.len(),
        2,
        "both pushState entries committed"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/b"),
        "the last pushState is the current entry"
    );
    assert_eq!(
        state.nav_controller.go_back().map(url::Url::as_str),
        Some("https://example.com/a"),
        "FIFO order: /a committed before /b"
    );
}

/// A same-turn `replaceState(…, '/a'); location.href='/b'` applies the
/// replaceState (in place) before the navigation drains.
#[test]
fn replacestate_then_navigation_ordering() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // Seed the initial document's session-history entry (the real load pushes
    // it; the harness builds the pipeline directly) so replaceState replaces IN
    // PLACE rather than acting as a push.
    state.nav_controller.push(base());

    let bridge = state.pipeline.runtime.bridge();
    bridge.set_pending_history(replace_state("/a"));
    bridge.set_pending_navigation(nav_to("/b"));
    drain_browser(&browser);

    let processed = process_pending_actions(&mut state);

    assert!(processed);
    // replaceState replaced the initial entry in place (len stays 1, no new
    // entry), applied BEFORE the /b navigation (which fails to load). The old
    // order early-returned on the navigation and dropped replaceState, leaving
    // the un-replaced initial `/` entry.
    assert_eq!(
        state.nav_controller.len(),
        1,
        "replaceState replaces in place — one entry, not two"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "replaceState /a applied BEFORE the /b navigation (old order left the un-replaced initial /)"
    );
}

/// Regression pin: a pure-navigation turn (no history) is untouched — it
/// pre-sends the current display list and reports the own-context action.
#[test]
fn pure_navigation_turn_unchanged() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state
        .pipeline
        .runtime
        .bridge()
        .set_pending_navigation(nav_to("/next"));
    drain_browser(&browser);

    let processed = process_pending_actions(&mut state);

    assert!(
        processed,
        "a pure-navigation turn reports the own-context action"
    );
    // The navigation fails to load over the disconnected test network, leaving
    // the controller empty — the observable contract (report + one pre-send
    // display list) is unchanged from before the reorder.
    assert_eq!(state.nav_controller.len(), 0);
    assert_eq!(
        count_display_lists(&browser),
        1,
        "a pure-navigation turn ships exactly the navigation's pre-send display list (unchanged)"
    );
}

/// Regression pin: a pure-history (pushState) turn is untouched — it commits the
/// entry, ships exactly one display list, and reports the own-context action.
#[test]
fn pure_pushstate_turn_unchanged() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state
        .pipeline
        .runtime
        .bridge()
        .set_pending_history(push_state("/a"));
    drain_browser(&browser);

    let processed = process_pending_actions(&mut state);

    assert!(
        processed,
        "a pure-history turn reports the own-context action"
    );
    assert_eq!(state.nav_controller.len(), 1);
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
    );
    assert_eq!(
        count_display_lists(&browser),
        1,
        "a pure-history turn ships exactly one display list (unchanged single-action behavior)"
    );
}
