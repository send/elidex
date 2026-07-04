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

use super::navigation::{handle_history_action, handle_navigate, process_pending_actions};
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

/// `handle_navigate` reports whether it **replaced the pipeline**. In the
/// disconnected test harness `load_document` always fails (Err branch →
/// `NavigationFailed`, `state.pipeline` unchanged), so it returns `false` — the
/// signal `handle_history_action` propagates so a failed traversal load does NOT
/// supersede the current document (Codex R2). The `true`-on-success case needs a
/// real load (VM / connected-integration-covered).
#[test]
fn handle_navigate_reports_false_on_failed_load() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    let target = url::Url::parse("https://example.com/a").unwrap();
    assert!(
        !handle_navigate(&mut state, &target, false, None),
        "a failed load leaves the pipeline unchanged → handle_navigate reports false"
    );
}

/// The `handle_history_action` return contract the drain-loop break keys on:
/// `true` ONLY for a traversal that genuinely superseded the document (a target
/// existed AND `handle_navigate` replaced the pipeline). In the disconnected
/// harness every `load_document` fails, so a `Back`/`Go` with a target still
/// returns `false` (no replacement) — alongside `PushState`/`ReplaceState` and a
/// no-op out-of-range `go`. So NONE of these break the drain loop here; the
/// `true`-on-successful-rebuild case is VM / connected-integration-covered.
#[test]
fn handle_history_action_reports_rebuild() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // Populate the controller so a Back has a prior entry to traverse to.
    state.nav_controller.push(base());
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap());

    // A Back with a prior entry drives handle_navigate, but the load fails in the
    // harness → the pipeline is NOT replaced → reports NO supersede (false).
    assert!(
        !handle_history_action(&mut state, &HistoryAction::Back),
        "a traversal whose load fails does not replace the pipeline → reports no supersede"
    );
    // pushState / replaceState never rebuild.
    assert!(
        !handle_history_action(&mut state, &push_state("/b")),
        "pushState commits an entry without rebuilding the pipeline"
    );
    assert!(
        !handle_history_action(&mut state, &replace_state("/c")),
        "replaceState commits in place without rebuilding the pipeline"
    );
    // A no-op traversal (out-of-range go) drives no handle_navigate at all → no
    // supersede → the loop must CONTINUE past it.
    assert!(
        !handle_history_action(&mut state, &HistoryAction::Go(999)),
        "an out-of-range go is a no-op (no handle_navigate) → reports no supersede"
    );
}

/// Load-failure correctness (Codex R2): when a same-turn traversal's load FAILS
/// the document is NOT superseded, so the drain loop must CONTINUE and the
/// trailing same-turn `pushState` IS applied to the (still-active) document. In
/// the disconnected harness `load_document` always fails, so this is the path
/// exercised here; the complementary successful-load supersede-and-break (the
/// trailing intent dropped) is VM / connected-integration-covered. Drives the
/// exact loop the drain runs — `for a in &history { if handle_history_action(..)
/// { break; } }` (boa's single-slot bridge can't flow a real two-item `Vec`
/// through `process_pending_actions` pre-flip).
#[test]
fn failed_traversal_load_does_not_drop_trailing_history() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base());
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap());
    // index=1, len=2; a Back traverses to `base` but its load FAILS (harness).

    let history = vec![HistoryAction::Back, push_state("/kept")];
    let mut applied = 0usize;
    for action in &history {
        applied += 1;
        if handle_history_action(&mut state, action) {
            break;
        }
    }

    assert_eq!(
        applied, 2,
        "the Back's load failed → no supersede → the loop continues to the trailing pushState"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/kept"),
        "the trailing pushState IS applied (a failed-load traversal must not drop same-turn history)"
    );
}

/// Traversal atomicity (Codex R3): `NavigationController::go_back` moves the
/// cursor eagerly, BEFORE `handle_navigate` runs. When the load fails (the
/// disconnected harness) the traversal must be atomic — the eager move is rolled
/// back (`restore_index`), leaving the cursor on the still-active document. So a
/// trailing same-turn `pushState` commits from the ORIGINAL index and does not
/// truncate the entry the failed `Back` tried (and failed) to leave. Without the
/// rollback the cursor would sit on the prior entry and the pushState would
/// truncate the active one.
#[test]
fn failed_traversal_load_rolls_back_cursor() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // index 0 = base
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap()); // index 1 = /a
    assert_eq!(state.nav_controller.current_index(), Some(1));

    // A Back whose load FAILS must NOT leave the cursor moved.
    let superseded = handle_history_action(&mut state, &HistoryAction::Back);
    assert!(!superseded, "a failed-load traversal does not supersede");
    assert_eq!(
        state.nav_controller.current_index(),
        Some(1),
        "the eager go_back move is rolled back on load failure — cursor unchanged"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the still-active document is /a, not the unreached base"
    );

    // A trailing same-turn pushState commits from the original index (1): it
    // appends after /a (len 3), preserving /a. Without the rollback the cursor
    // would sit at base (index 0) and this pushState would TRUNCATE /a (len 2).
    handle_history_action(&mut state, &push_state("/kept"));
    assert_eq!(
        state.nav_controller.len(),
        3,
        "pushState appended after /a → [base, /a, /kept]; the rollback preserved /a"
    );
    assert_eq!(
        state.nav_controller.go_back().map(url::Url::as_str),
        Some("https://example.com/a"),
        "back from /kept lands on /a — preserved because the failed Back's cursor move was rolled back"
    );
}
