//! Content-mode history/navigation drain — Slice A phase-separation
//! (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md`).
//!
//! The single synchronous `process_pending_actions` drain is retired: these tests
//! drive the shared [`DrainCoordinator`] — [`DrainCoordinator::drain_synchronous_phase`]
//! (Phase 1, in-task: window-opens → §7.4.4 sync updates → last-wins nav,
//! enqueuing in-range traversals) and [`DrainCoordinator::run_deferred_traversals`]
//! (Phase 2, §7.4.6.1 *apply the history step*). The oracle is the shell-owned
//! `NavigationController` (entry commit + cursor), the traversal queue
//! (`has_pending_traversal`), and the browser channel's `DisplayListReady` count.
//!
//! **Harness reachability:** cross-document loads fail over the disconnected test
//! network, so a *successful* cross-document rebuild (and thus a document-changing
//! traversal's `changed_document = true` → Resolution-D `SyncUpdate` cancel) is
//! **not** reachable here — it is pinned by the substrate isolation test
//! (`traversal_queue_tests::syncupdate_canceled_after_document_changing_traversal`)
//! plus VM/connected-integration coverage. A **same-document** traversal takes the
//! no-fetch `same_document_step` path and DOES apply in the harness, so the
//! same-document complements are pinned here. Supersede / cross-turn / peek-classify
//! are asserted at the queue + coordinator level (plan §5).

use elidex_navigation::{DrainCoordinator, DrainHost};
use elidex_script_session::HistoryAction;

use super::navigation::{handle_history_action, handle_navigate, HistoryCursorOp};
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
        serialized_state: None,
    }
}

fn replace_state(path: &str) -> HistoryAction {
    HistoryAction::ReplaceState {
        url: Some(path.to_string()),
        title: String::new(),
        serialized_state: None,
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

/// Count the `NavigationState` (chrome `can_go_back`/`can_go_forward`) messages
/// currently queued on the browser channel — shipped only from `notify_navigation`
/// in `handle_navigate`'s `Ok` branch (post-cursor-commit).
fn count_navigation_states(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) -> usize {
    let mut n = 0;
    while let Ok(msg) = browser.try_recv() {
        if matches!(msg, ContentToBrowser::NavigationState { .. }) {
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
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.pushState(null, '', '/a'); location.assign('/b');");
    drain_browser(&browser);

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);

    assert!(
        outcome.own_context_action,
        "a same-turn navigation is an own-context action"
    );
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

    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.replaceState(null, '', '/a'); location.assign('/b');");
    drain_browser(&browser);

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);

    assert!(outcome.own_context_action);
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
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("location.assign('/next');");
    drain_browser(&browser);

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);

    assert!(
        outcome.own_context_action,
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
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.pushState(null, '', '/a');");
    drain_browser(&browser);

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);

    assert!(
        outcome.own_context_action,
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
///
/// Also pins that the [`HistoryCursorOp`] is applied ONLY in the `Ok` branch: a
/// fresh-nav `Push` on a failed load pushes NOTHING (the cursor op is
/// success-gated, so a failed load never mutates the controller — the reachable
/// half of the R5 commit-before-notify move).
#[test]
fn handle_navigate_reports_false_on_failed_load() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    let target = url::Url::parse("https://example.com/a").unwrap();
    assert!(
        !handle_navigate(&mut state, &target, HistoryCursorOp::Push, None),
        "a failed load leaves the pipeline unchanged → handle_navigate reports false"
    );
    assert_eq!(
        state.nav_controller.len(),
        0,
        "Push runs only in the Ok branch → a failed load pushes no entry (cursor op is success-gated)"
    );
}

/// The `HistoryCursorOp::Commit` half of the R5 fix, at the `handle_navigate`
/// seam: a JS traversal threads `Commit(target)` INTO `handle_navigate` (its `Ok`
/// branch, before `notify_navigation`) rather than committing in the caller after
/// return. On a failed load (the disconnected harness) the `Ok` branch is
/// unreached, so the commit never fires and the cursor stays put — pinning that
/// the commit is success-gated at the seam (the atomic-traversal invariant now
/// living inside `handle_navigate`). The success path (commit THEN notify, so the
/// shipped `NavigationState` reads the moved cursor) needs a real load and is
/// VM / connected-integration coverage.
#[test]
fn handle_navigate_commit_op_is_success_gated() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // index 0
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap()); // index 1 (current)

    // Ask handle_navigate to commit back to index 0, but the load FAILS → the Ok
    // branch (where Commit runs) is never reached → the cursor stays on index 1.
    assert!(
        !handle_navigate(&mut state, &base(), HistoryCursorOp::Commit(0), None),
        "a failed load reports false"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "Commit runs only in the Ok branch → a failed load never moves the cursor (still on /a)"
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

/// Traversal atomicity (Codex R3, peek-then-commit): a traversal peeks its
/// target WITHOUT moving the cursor and commits the move (`commit_index`) ONLY on
/// a successful load. When the load fails (the disconnected harness) the cursor
/// never moved, so the still-active document — and a trailing same-turn
/// `pushState` committing after it — is unaffected. This replaces the retired
/// eager-move + `restore_index` rollback with never-moving-until-success (the
/// `current_index`/`restore_index` cursor pair the R3 fix added is gone).
#[test]
fn failed_traversal_load_leaves_cursor_unmoved() {
    let (mut state, _browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // index 0 = base
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap()); // index 1 = /a
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "start on /a (the last-pushed entry)"
    );

    // A Back whose load FAILS must NOT move the cursor (peek-then-commit only
    // commits on a successful load).
    let superseded = handle_history_action(&mut state, &HistoryAction::Back);
    assert!(!superseded, "a failed-load traversal does not supersede");
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the cursor never moved — the still-active document is /a, not the unreached base"
    );

    // A trailing same-turn pushState commits from the unmoved index: it appends
    // after /a (len 3), preserving /a. Had the cursor speculatively moved to base,
    // this pushState would TRUNCATE /a (len 2).
    handle_history_action(&mut state, &push_state("/kept"));
    assert_eq!(
        state.nav_controller.len(),
        3,
        "pushState appended after /a → [base, /a, /kept]; the cursor never left /a"
    );
    assert_eq!(
        state.nav_controller.go_back().map(url::Url::as_str),
        Some("https://example.com/a"),
        "back from /kept lands on /a — preserved because the failed Back never moved the cursor"
    );
}

/// FLIP (#283 re-anchor — plan §5): under phase-separation the same-turn
/// `history.back(); location.assign('/b')` no longer "falls through and drains
/// /b." An **in-range** back() is peek-classified into the traversal queue in
/// Phase 1b, so Phase 1c **drain-and-DISCARDS** the /b navigation (§7.4.2.2 step
/// 19 "any attempts to navigate a navigable that is currently traversing are
/// ignored"; §7.4.6.1 step 12 splits the traversal onto a later task). The /b nav
/// is dropped WITHOUT applying (and WITHOUT stranding to re-fire a turn late,
/// F1); the traversal defers to Phase 2. The old-model "the /b navigation drained
/// (1 display list)" flips to "the /b nav is discarded (0 display lists)."
///
/// The Phase-2 back() here is CROSS-document ([base, /a] via `push`, distinct
/// `document_sequence`s), so its `load_document` fails over the disconnected
/// harness → `shipped = false`, cursor left unmoved on /a (the successful-rebuild
/// landing is VM / connected-integration coverage). The land-on-the-back-target
/// success complement is pinned by `nav_vs_traversal_supersede_lands_on_back_target`
/// using a same-document back() (the no-fetch path the harness can apply).
#[test]
fn same_turn_traversal_supersedes_and_discards_navigation() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base());
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap());
    // index=1 on /a; a same-turn in-range Back + a location nav.
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.back(); location.assign('/b');");
    drain_browser(&browser);

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);

    // The in-range back() enqueued (Phase 1b) → the nav is drain-and-discarded
    // (Phase 1c), so NO own-context nav applied this turn — but the default IS
    // suppressed via the queue-pending signal (the shell's suppression predicate).
    assert!(
        !outcome.own_context_action,
        "the nav was discarded (not applied) — no own-context nav in Phase 1"
    );
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the in-range back() is queued for Phase 2 (supersedes the same-turn nav)"
    );
    // Documents the production default-suppression predicate: the coordinator's
    // single `suppress_default` field (own-context effect OR a pending traversal) is
    // exactly what the shell's click path reads to drop the `<a href>` default.
    assert!(
        outcome.suppress_default,
        "the shell suppresses the <a href> default (a pending traversal supersedes)"
    );
    // The /b nav was DISCARDED, not drained-and-applied: it shipped no display
    // list (the old model shipped its pre-send DL = 1; this flips to 0).
    assert_eq!(
        count_display_lists(&browser),
        0,
        "the discarded /b nav ships no display list (FLIP: was 1 under the fall-through model)"
    );
    assert_eq!(
        state.nav_controller.len(),
        2,
        "no /b entry: the nav was discarded, not applied"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "cursor still on /a — Phase 1 only enqueued the traversal (Phase 2 not yet run)"
    );

    // Phase 2: the cross-document back() applies but its load fails (harness) →
    // cursor unmoved, queue drained. (Successful rebuild = VM/connected coverage.)
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(
        state.traversal_queue().is_empty(),
        "Phase 2 drained the queue (no re-enqueue — loop-inert)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the cross-document back() load failed → cursor unmoved (still /a), /b never navigated"
    );
}

/// Codex R5: a successful JS traversal moves the session-history cursor
/// (`HistoryCursorOp::Commit`) INSIDE `handle_navigate`, BEFORE `notify_navigation`
/// ships the chrome `NavigationState`. So `history.back()` from the last entry
/// reports `can_go_forward = true` (post-move) rather than the stale pre-move
/// `false` the caller-side commit produced. The old order committed the cursor in
/// the *caller* AFTER `handle_navigate` returned, so `notify_navigation` had
/// already shipped the pre-move state.
///
/// Reachability boundary (same as `#283` above): the DISCRIMINATING success-path
/// assertion — the shipped `NavigationState` carries the *committed*
/// `can_go_back`/`can_go_forward` — needs a real `load_document`, but the
/// disconnected test network `Err`s every load, so `handle_navigate`'s `Ok`
/// branch (where the commit + `notify_navigation` run) is unreachable here. That
/// assertion is registered as VM / connected-integration coverage (an S5-6-flip
/// live-shell deliverable, alongside the other `true`-on-success cases in this
/// file).
///
/// What IS reachable and pinned here is the COMPLEMENT: a FAILED traversal ships
/// NO `NavigationState` at all (the `Err` branch sends only `NavigationFailed`),
/// so the stale-chrome-state bug cannot manifest on the failed path — the
/// `NavigationState` is coupled to the `Ok` branch that now commits before it. A
/// regression that shipped `NavigationState` from the failed path (e.g. a caller
/// that notified unconditionally) would break this.
#[test]
fn failed_traversal_ships_no_navigation_state() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base());
    state
        .nav_controller
        .push(url::Url::parse("https://example.com/a").unwrap());
    // index=1 on /a; a Back peeks index 0 but its load FAILS in the harness.
    drain_browser(&browser);

    let superseded = handle_history_action(&mut state, &HistoryAction::Back);

    assert!(!superseded, "a failed-load traversal does not supersede");
    assert_eq!(
        count_navigation_states(&browser),
        0,
        "a failed traversal ships no NavigationState — it is sent only from the Ok branch, \
         after the cursor commit (Codex R5)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the cursor never moved (commit is success-gated inside handle_navigate)"
    );
}

// ---------------------------------------------------------------------------
// Slice A phase-separation conformance (A / B / D / E + I1 + loop-inert).
// Same-document entries (`push` + `push_same_document`, shared `document_sequence`)
// take the no-fetch `same_document_step` path, so their Phase-2 apply SUCCEEDS in
// the disconnected harness (a cross-document rebuild would fail).
// ---------------------------------------------------------------------------

/// Two same-document entries `[base, /a]` (shared `document_sequence`), cursor on
/// `/a`, so a `back()` resolves same-document and applies in place (no fetch).
fn seed_same_document_pair(state: &mut super::ContentState) {
    state.nav_controller.push(base()); // index 0, base
    state
        .nav_controller
        .push_same_document(url::Url::parse("https://example.com/a").unwrap()); // index 1, /a
}

/// I1 (ordering across the task boundary): `pushState('/a'); history.back()` in one
/// turn — the pushState commits to the controller in Phase 1 (in-task), THEN the
/// traversal applies in Phase 2 against the UPDATED entry list (§7.4.6.1 step 12
/// "synchronous navigations processed before documents unload").
#[test]
fn phase_sep_pushstate_then_back_orders_across_task_boundary() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // index 0, base (a prior entry to go back to)
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.pushState(null, '', '/a'); history.back();");
    drain_browser(&browser);

    // Phase 1: the pushState /a committed (in-task); the back() is QUEUED, not applied.
    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        outcome.own_context_action,
        "the pushState is an own-context effect"
    );
    assert_eq!(
        state.nav_controller.len(),
        2,
        "the pushState /a entry committed in Phase 1 (before the traversal applies)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "Phase 1 landed on /a (the pushState), the traversal deferred"
    );
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the back() is queued for Phase 2, not applied in-task"
    );

    // Phase 2: the back() applies against the updated [base, /a] list → lands on base.
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(
        state.traversal_queue().is_empty(),
        "Phase 2 drained the queue"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/"),
        "the deferred back() applied against the Phase-1-updated list → base (I1)"
    );
}

/// A (nav-vs-traversal supersede): a same-turn `history.back(); location.assign('/b')`
/// lands on the **back target**, the nav drain-and-discarded. The reverse
/// cross-channel order `location.assign('/b'); history.back()` lands the SAME way —
/// staging discards cross-channel issue order, and the spec lands on the traversal
/// target in BOTH orders (§7.4.2.2 step 19 ignores a nav issued while traversing;
/// step 20 — a later same-turn navigation aborts other *navigations* but NOT a
/// traversal). Uses a same-document back() so Phase 2 applies in the harness.
#[test]
fn nav_vs_traversal_supersede_lands_on_back_target() {
    for script in [
        "history.back(); location.assign('/b');",
        // Reverse cross-channel order — same landing (issue order discarded by staging).
        "location.assign('/b'); history.back();",
    ] {
        let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
        seed_same_document_pair(&mut state); // [base, /a], cursor on /a
        let _ = state.pipeline.runtime.vm().eval(script);
        drain_browser(&browser);

        let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
        assert!(
            state.traversal_queue().has_pending_traversal(),
            "{script}: the in-range back() supersedes the same-turn nav (queued for Phase 2)"
        );
        assert_eq!(
            count_display_lists(&browser),
            0,
            "{script}: the /b nav was discarded — no pre-send display list"
        );

        let _ = DrainCoordinator::run_deferred_traversals(&mut state);
        assert_eq!(
            state.nav_controller.current_url().map(url::Url::as_str),
            Some("https://example.com/"),
            "{script}: landed on the back target (base), NOT /b — the traversal won"
        );
        assert!(
            !state
                .nav_controller
                .current_url()
                .is_some_and(|u| u.as_str().ends_with("/b")),
            "{script}: /b was never navigated"
        );
    }
}

/// E (no-op peek-classify): `history.go(999)` at end-of-history classifies as a
/// no-op (out-of-range peek → `None`), so it is NOT a partition barrier — the
/// trailing `pushState('/x')` applies in-task and a same-turn `location.assign`
/// is NOT suppressed (§7.4.3 sub-step 4.4 "does not exist ⇒ abort").
#[test]
fn noop_traversal_peek_classify_does_not_defer_trailing_intents() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // index 0 (go(999) is out of range)
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.go(999); history.pushState(null, '', '/x'); location.assign('/y');");
    drain_browser(&browser);

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        !state.traversal_queue().has_pending_traversal(),
        "the no-op go(999) enqueued no Traversal step (not a barrier)"
    );
    assert!(
        state.traversal_queue().is_empty(),
        "nothing deferred by the no-op"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/x"),
        "the trailing pushState /x applied IN-TASK (the no-op did not defer it)"
    );
    assert!(outcome.own_context_action);
    // The same-turn /y nav was NOT suppressed (a no-op leaves no Traversal
    // pending) → it drained and shipped its pre-send display list.
    assert_eq!(
        count_display_lists(&browser),
        1,
        "the /y nav was NOT suppressed by the no-op traversal (it drained + pre-sent)"
    );
}

/// D complement (same-document): a `SyncUpdate` deferred behind a **same-document**
/// traversal is NOT canceled — `back(); pushState('/x')` where back() is
/// same-document applies the deferred /x in Phase 2 (no identity mismatch). The
/// document-CHANGING cancel path needs a successful rebuild (VM/connected coverage);
/// the substrate isolation test pins the cancel itself.
#[test]
fn deferred_syncupdate_applies_after_same_document_traversal() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.back(); history.pushState(null, '', '/x');");
    drain_browser(&browser);

    // Phase 1: back() enqueued (barrier), the trailing pushState DEFERRED (I2), so
    // it is NOT applied in-task — the controller still reads /a.
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(state.traversal_queue().has_pending_traversal());
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the trailing pushState is DEFERRED behind the traversal (not applied in Phase 1)"
    );

    // Phase 2: same-document back() applies (no fetch) → base; then the deferred
    // /x push applies (same-document traversal did NOT cancel it — Resolution D).
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(state.traversal_queue().is_empty(), "queue drained");
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/x"),
        "the deferred /x push applied after the same-document back() (not canceled)"
    );
}

/// B cross-turn-robust (E1): a Turn-1 `history.back()` left queued (Phase 2 not yet
/// pumped) makes a Turn-2 `location.assign('/c')` supersede-suppressed — the shell's
/// default-suppression predicate reads the still-pending traversal across turns, so
/// a Turn-2 `<a href>` click's default is suppressed and the Turn-2 nav discarded.
#[test]
fn cross_turn_pending_traversal_suppresses_turn2_default_and_nav() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a

    // Turn 1: back() enqueued, Phase 2 NOT pumped (the traversal stays queued).
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "Turn-1 back() left queued (no Phase-2 pump)"
    );
    drain_browser(&browser);

    // Turn 2: a link handler runs location.assign('/c') while the Turn-1 traversal
    // is still queued. Phase 1c drains-and-discards /c; the suppression predicate
    // (own_context_action || has_pending_traversal) is TRUE → the <a href> default
    // is suppressed.
    let _ = state.pipeline.runtime.vm().eval("location.assign('/c');");
    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);
    // The production single-home predicate (`suppress_default` = own-context effect
    // OR a pending traversal) reads TRUE across the turn boundary (E1).
    assert!(
        outcome.suppress_default,
        "the still-pending cross-turn traversal suppresses the Turn-2 link default (E1)"
    );
    assert_eq!(
        count_display_lists(&browser),
        0,
        "the Turn-2 /c nav was drain-and-discarded (no pre-send display list)"
    );
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the Turn-1 traversal is still queued after Turn 2"
    );
}

/// loop-inert (plan §1 loop-bound): content's Phase-2 apply does NOT re-enqueue, so
/// `run_deferred_traversals` drains to empty in one pass (the unbounded re-check
/// loop is inert — no wired reentrant source; the structural guard is Slice 4).
#[test]
fn content_apply_traversal_does_not_re_enqueue() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    seed_same_document_pair(&mut state);
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    drain_browser(&browser);

    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "back() queued"
    );

    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(
        state.traversal_queue().is_empty(),
        "the same-document apply re-enqueued nothing → the queue drained empty"
    );
}
