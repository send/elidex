//! Content-mode **pump-turn drain-unification** conformance — the message-held
//! `pump_turn` skeleton that retired the reentrant-replay 2nd channel
//! (`docs/plans/2026-07-session-history-slice-A-pump-turn-drain-unification.md` §4/§6).
//!
//! Carved from `content_history_phase_sep_tests.rs` at the touch-time 1000-line
//! split (M6): that file owns the coordinator sequencing / same-document-apply /
//! interim-reentrancy-guard conformance; this sibling owns the three R13
//! pump-turn-ordering scenarios the message-held skeleton resolves by construction —
//! (:416) a queued traversal applies before a held direct nav, (:73) a
//! popstate-staged `pushState` survives a held nav (fresh AND buffered), and (:49) a
//! queued `Shutdown` preempts the Phase-2 apply.
//!
//! The R14 seam-boundary completion adds the teardown-safety scenarios: the
//! pipeline-mutating `DrainHost` seams fail closed after a mid-drain teardown
//! (`route_window_opens` HOLE A / `ship_frame` HOLE B), and the step-1 intake probes
//! the channel each buffer-drain turn so a `Shutdown` preempts (teardown-priority)
//! while a probed non-`Shutdown` message is preserved in FIFO order (Finding 2).
//!
//! The R15 intake DRAIN generalizes the buffer-drain probe to a full non-blocking
//! drain (until `Shutdown` or `Empty`), so a `Shutdown` BEHIND an earlier channel
//! message still preempts rather than being starved a turn late (Codex PR#469 R15).
//!
//! Same-document entries (`push` + `push_same_document`, shared `document_sequence`)
//! take the no-fetch `same_document_step` path, so their Phase-2 apply succeeds in the
//! disconnected harness (a cross-document rebuild would fail — exercised elsewhere).

use elidex_navigation::{DrainCoordinator, DrainHost};
use elidex_script_session::HostDriver;

use super::test_support::{
    base, build_test_content_state_with_url, drain_browser, seed_same_document_pair,
};
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// Count the `DisplayListReady` messages currently queued on the browser channel —
/// the "did the (possibly torn-down) pipeline ship a frame?" witness for the R14
/// seam-guard tests (mirrors the sibling `content_history_drain_tests` helper).
fn count_display_lists(browser: &LocalChannel<BrowserToContent, ContentToBrowser>) -> usize {
    let mut n = 0;
    while let Ok(msg) = browser.try_recv() {
        if matches!(msg, ContentToBrowser::DisplayListReady(_)) {
            n += 1;
        }
    }
    n
}

/// **:416 — a queued traversal applies BEFORE a held direct navigate.** A prior turn
/// queued a same-document `back()`; a direct `Navigate` is HELD this turn. The pump
/// applies the queued traversal at step 3 (cursor → the back target) and only THEN
/// dispatches the held message at step 4, so the direct nav cannot overtake the
/// traversal and apply against a different history list.
///
/// The held `Navigate` targets a fragment of the BACK TARGET (`/a#c`). Under the
/// correct order (step 3 before step 4) it dispatches while `current` is already `/a`,
/// so `classify(/a, /a#c)` is same-document and the push lands. Under the :416 bug
/// (the direct nav overtaking the traversal) it would classify against `/b`
/// (cross-document → fails in the disconnected harness) and never apply — a distinct,
/// observable end state.
#[test]
fn queued_traversal_applies_before_a_held_direct_navigate() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // 0: base
    state
        .nav_controller
        .push_same_document(url::Url::parse("https://example.com/a").unwrap()); // 1: /a
    state
        .nav_controller
        .push_same_document(url::Url::parse("https://example.com/b").unwrap()); // 2: /b, cursor here
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "a prior turn queued the same-document back() for this turn's Phase-2 apply"
    );
    drain_browser(&browser);

    // Hold a DIRECT Navigate to a fragment of the back target.
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Navigate(
            url::Url::parse("https://example.com/a#c").unwrap(),
        ));

    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(flow.is_continue(), "a non-shutdown turn continues the loop");
    assert!(
        state.traversal_queue().is_empty(),
        "the queued back() applied at step 3"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a#c"),
        ":416 — the queued back() applied FIRST (cursor → /a), THEN the held Navigate \
         pushed its same-document /a#c fragment; the direct nav did not overtake the traversal"
    );
    assert_eq!(
        state.nav_controller.len(),
        3,
        "[base, /a, /a#c] — /b was truncated by the same-document push from /a"
    );
}

/// **:73 — a popstate-staged `pushState` is applied to the entry list this turn, with
/// a nav-mutating held message present (fresh AND buffered).** The step-3 Phase-2
/// apply of a same-document `back()` fires `popstate`, whose handler stages
/// `pushState('/frompop')`; the step-3 TOP drain settles that intent into the
/// `NavigationController` (which survives a pipeline rebuild) BEFORE the held message
/// dispatches at step 4. So the pushState is committed even though a nav-mutating
/// message is delivered the same turn — uniformly whether it came from a fresh channel
/// `recv` or the reentrant buffer.
///
/// Harness honesty (supported-surface — stated plainly): this pins the plan §6
/// **correct-behavior** assertion — the popstate `pushState` is *applied to the entry
/// list* this turn (not left stranded in `pending_history`) with a nav-mutating message
/// in flight. It is **NOT a fail-before-fix regression of the VM-swap severance**: the
/// disconnected harness classifies the cross-document held `Navigate` → `load_document`
/// fails → NO pipeline rebuild → so the severance cannot occur under EITHER the old or
/// the new code, and this test would pass against both. The severance-regression is
/// instead guarded **BY CONSTRUCTION** — step-4's held-message dispatch runs strictly
/// after step-3's atomic Phase-2 apply + top drain, so no rebuild can interpose between
/// the popstate staging and its settle — and the real swap is a connected-integration
/// concern (no `#11-*` slot: by-construction ordering + this §6 assertion are adequate
/// supported-surface coverage). The cross-document held `Navigate` (classified against
/// the still-`base` `pipeline.url`, which a same-document `pushState` does not resync)
/// does not itself apply — its role is to be the in-flight nav-mutating message the
/// settled pushState must survive.
///
/// Where the step-3-before-step-4 ORDERING is EXECUTABLY pinned: the sibling
/// [`queued_traversal_applies_before_a_held_direct_navigate`] (:416) is a genuine
/// fail-before-fix guard — it fails if step 4 precedes step 3 (a held direct `Navigate`
/// classifies cross-document against the pre-traversal URL and never applies). So the
/// ordering this :73 test relies on is regression-covered there, not here.
#[test]
fn popstate_staged_pushstate_applied_with_held_navigate_fresh_and_buffered() {
    for buffered in [false, true] {
        let (mut state, browser) = build_test_content_state_with_url(
            "<p>doc</p>\
             <script>window.addEventListener('popstate', function () {\
               history.pushState(null, '', '/frompop');\
             });</script>",
            base(),
        );
        seed_same_document_pair(&mut state); // [base, /a], cursor on /a
        let _ = state.pipeline.runtime.vm().eval("history.back();");
        let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
        drain_browser(&browser);

        let held =
            BrowserToContent::Navigate(url::Url::parse("https://example.com/frompop#nav").unwrap());
        let mut last_frame = std::time::Instant::now();
        let flow = if buffered {
            state.deferred_reentrant_messages.push(held);
            super::event_loop::pump_turn(&mut state, &mut last_frame)
        } else {
            browser.send(held).unwrap();
            super::event_loop::pump_turn(&mut state, &mut last_frame)
        };
        assert!(
            flow.is_continue(),
            "buffered={buffered}: a non-shutdown turn continues the loop"
        );
        assert_eq!(
            state.nav_controller.current_url().map(url::Url::as_str),
            Some("https://example.com/frompop"),
            "buffered={buffered}: :73 — the step-3 TOP drain settled the popstate \
             pushState into the entry list (cursor → /frompop) with the nav-mutating \
             held message in flight; the intent was applied, not stranded"
        );
        assert!(
            state.pipeline.runtime.take_pending_history().is_empty(),
            "buffered={buffered}: no history intent left staged after the turn"
        );
    }
}

/// **:49 — a queued `Shutdown` preempts the Phase-2 apply (teardown-priority).** A
/// prior turn queued a same-document `back()`, and a `Shutdown` is already queued on
/// the channel at pump-turn start. The Shutdown is intaken (step 1) and handled at
/// step 2 — BEFORE the step-3 Phase-2 apply — so the `back()` is NOT applied, its
/// `popstate` never fires, and no `pushState('/frompop')` is staged: no script /
/// document load runs on the closing tab.
#[test]
fn queued_shutdown_preempts_the_phase2_apply() {
    let (mut state, browser) = build_test_content_state_with_url(
        "<p>doc</p>\
         <script>window.addEventListener('popstate', function () {\
           history.pushState(null, '', '/frompop');\
         });</script>",
        base(),
    );
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "a prior turn queued the same-document back()"
    );
    drain_browser(&browser);

    browser.send(BrowserToContent::Shutdown).unwrap();
    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);

    assert!(flow.is_break(), ":49 — a queued Shutdown breaks the pump");
    assert!(
        state.traversal_queue().has_pending_traversal(),
        ":49 — the queued back() was NOT applied (Phase-2 preempted by teardown)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        ":49 — cursor unchanged: no traversal apply / no popstate ran on the closing tab"
    );
    assert_eq!(
        state.nav_controller.len(),
        2,
        ":49 — no /frompop entry: the popstate handler never ran (Phase-2 did not apply)"
    );
}

/// **Regression (drain-split): a popstate-staged CROSS-document navigation is deferred
/// below the held input.** The step-3 top drain is `drain_synchronous_updates` (Phase
/// 1a + 1b), NOT the full `drain_synchronous_phase` (which also runs Phase 1c). So when
/// a prior-turn same-document `back()` applies at step 3 and its `popstate` handler
/// calls a CROSS-document `location.assign`, that navigation is NOT applied at the top:
/// it stays staged in `pending_navigation` past the step-4 held-input dispatch, so the
/// input hits the PRE-navigation document, and the cross-document nav applies only at
/// the step-6 bottom full drain (a later task — WHATWG HTML: `location.assign` completes
/// in a later task than an already-pending input). A whole-body top drain would apply it
/// at step 3, blocking-loading /other and rebuilding `state.pipeline` before the held
/// `MouseClick`/`KeyDown` dispatched → the input would hit the wrong document.
///
/// Pinned STRUCTURALLY (harness-independent): the disconnected harness cannot perform a
/// real cross-document rebuild (`load_document` fails), so this asserts the seam
/// contract directly — the top updates-drain leaves `pending_navigation` staged, and the
/// full drain (the step-6 role) is the one that consumes it via Phase 1c. The connected
/// rebuild-then-input-hit is out of harness scope; the ordering is guaranteed BY
/// CONSTRUCTION (step 4 dispatches strictly between the 1a/1b top drain and the 1c bottom
/// drain).
#[test]
fn popstate_cross_document_navigation_deferred_below_held_input() {
    let (mut state, browser) = build_test_content_state_with_url(
        "<p>doc</p>\
         <script>window.addEventListener('popstate', function () {\
           location.assign('https://example.com/other');\
         });</script>",
        base(),
    );
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "a prior turn queued the same-document back()"
    );
    drain_browser(&browser);

    // === step 3 === run_deferred_traversals applies the back() → fires popstate →
    // its handler stages a CROSS-document location.assign in pending_navigation. The
    // updates-only TOP drain runs Phase 1a + 1b but NOT Phase 1c, so it does NOT apply
    // that navigation.
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    let _ = DrainCoordinator::drain_synchronous_updates(&mut state);

    // The cross-document navigation SURVIVED the top drain — it is still staged (the
    // updates-only drain deferred Phase 1c). So a held MouseClick/KeyDown dispatched at
    // step 4 hits the PRE-nav document; the nav applies only at the step-6 bottom drain.
    // (Consuming assertion — no non-mutating peek exists; that it is still takeable is
    // the proof it was not drained at the top.)
    assert!(
        state.pipeline.runtime.take_pending_navigation().is_some(),
        "the popstate-staged cross-document location.assign was NOT applied by the \
         updates-only top drain (Phase 1c deferred to the bottom full drain)"
    );

    // Complement: the FULL drain (the step-6 bottom drain's role) IS the sole drain of
    // a cross-document `pending_navigation` — Phase 1c consumes it. Stage one directly
    // and confirm `drain_synchronous_phase` takes it.
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("location.assign('https://example.com/other2');");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.pipeline.runtime.take_pending_navigation().is_none(),
        "the full drain (step-6 role) runs Phase 1c and consumes the cross-document nav"
    );
}

/// **HOLE A — `route_window_opens` fails closed after a mid-drain teardown.** In a pump
/// turn's step 3, `run_deferred_traversals` (drain-1) can apply a traversal whose SW-wait
/// re-dispatches a `Shutdown` (`dispatch_or_buffer_reentrant`), tearing the pipeline down
/// and setting `shutdown_requested`. The very next drain-2 (`drain_synchronous_updates`)
/// runs Phase-1a `route_window_opens` BEFORE the pump's post-step-3 `shutdown_requested`
/// check, so without the seam's entry guard it would re-render + ship a display list (and
/// route an `OpenNewTab`) from the torn-down pipeline (Codex PR#469 R14).
///
/// The disconnected harness cannot drive a real SW-wait teardown, so this pins the SEAM
/// guard directly: with `shutdown_requested` already set (as drain-1 would leave it) and a
/// `window.open` popup staged, the drain-2 entry point must NOT consume the pending open
/// nor ship. Fail-before-fix witnesses: the open stays QUEUED (the guard returned before
/// `take_pending_window_opens`) and zero display lists are sent (unguarded,
/// `route_window_opens` takes the open, routes an `OpenNewTab`, and ships its frame).
#[test]
fn route_window_opens_fails_closed_after_mid_drain_teardown() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // Stage a `window.open` popup — a Phase-1a intent the drain-2 seam would route.
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("window.open('https://example.com/popup');");
    // Simulate drain-1's teardown: the SW-wait `Shutdown` already set the flag.
    state.shutdown_requested = true;
    drain_browser(&browser);

    // Drain-2 of step 3 — its Phase-1a `route_window_opens` seam must fail closed.
    let _ = DrainCoordinator::drain_synchronous_updates(&mut state);

    assert_eq!(
        count_display_lists(&browser),
        0,
        "HOLE A — a torn-down pipeline's drain-2 route_window_opens ships nothing"
    );
    assert!(
        !state
            .pipeline
            .runtime
            .take_pending_window_opens()
            .is_empty(),
        "HOLE A — the seam returned at its entry guard BEFORE taking the window-open \
         (unguarded it would have consumed + routed it)"
    );
}

/// **HOLE B — `ship_frame` fails closed after a mid-drain teardown.** `ship_frame` is
/// reached via the coordinator's shared `ship_if_needed` tail, which runs INSIDE
/// `run_deferred_traversals` / `drain_synchronous_phase` — BEFORE the pump's post-drain
/// `shutdown_requested` check. So a Phase-1c `handle_navigation` SW-wait (or a Phase-2
/// apply) that tears down mid-drain, with an own-context effect already recorded this pass
/// (e.g. a co-staged `pushState`), would let `ship_if_needed` ship the torn-down
/// pipeline's display list — a dead frame (Codex PR#469 R14).
///
/// Pinned via the real coordinator path with `shutdown_requested` pre-set (as a mid-drain
/// teardown would leave it): a staged `pushState` drives `run_synchronous_updates_body` to
/// record `own_context_action` (even though the already-guarded `handle_history_action`
/// no-ops on the torn-down controller), so `ship_if_needed` calls `ship_frame`. The
/// `ship_frame` entry guard is the last line of defense — fail-before-fix: unguarded it
/// sends one `DisplayListReady` from the dead pipeline; guarded it sends none.
#[test]
fn ship_frame_fails_closed_after_mid_drain_teardown() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // A co-staged synchronous history update — the own-context effect that makes
    // `ship_if_needed` want to ship at the end of the drain.
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.pushState(null, '', '/a');");
    // Simulate the mid-drain teardown: Phase-1c's SW-wait `Shutdown` already set it.
    state.shutdown_requested = true;
    drain_browser(&browser);

    // The full Phase-1 drain (the step-6 role) records own_context_action from the
    // pushState and reaches `ship_if_needed` → `ship_frame`, which must fail closed.
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);

    assert_eq!(
        count_display_lists(&browser),
        0,
        "HOLE B — ship_frame ships nothing from the torn-down pipeline"
    );
    assert_eq!(
        state.nav_controller.len(),
        0,
        "the guarded handle_history_action left the torn-down controller untouched \
         (no pushState applied post-teardown)"
    );
}

/// **Finding 2 (teardown-priority) — a channel `Shutdown` preempts a buffer-drain turn.**
/// When the reentrant buffer is non-empty, the step-1 intake used to deliver a buffered
/// message WITHOUT reading the channel, starving a `Shutdown` waiting on the channel behind
/// the buffer (teardown-priority could not fire until the buffer emptied — up to the ~30s
/// SW deadline). The intake now probes the channel non-blocking each buffer-drain turn: a
/// `Shutdown` preempts (handed to step 2's teardown) while the buffer stays intact (Codex
/// PR#469 R14).
///
/// Fail-before-fix: with a buffered message queued AND a `Shutdown` on the channel, the
/// turn must BREAK (teardown-priority) — the old drain-the-buffer-first intake returned
/// `Continue`, delivering the buffered message and leaving the `Shutdown` unread.
#[test]
fn channel_shutdown_preempts_a_buffer_drain_turn() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // A buffered reentrant message awaiting one-per-turn re-delivery.
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::CursorLeft);
    // A Shutdown arrives on the channel while the buffer is non-empty.
    browser.send(BrowserToContent::Shutdown).unwrap();

    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);

    assert!(
        flow.is_break(),
        "Finding 2 — the channel Shutdown preempted the buffer-drain (teardown-priority \
         fired); the old intake would have delivered the buffered message and returned Continue"
    );
    assert_eq!(
        state.deferred_reentrant_messages.len(),
        1,
        "the buffer stayed intact — the Shutdown preempted without draining a buffered message"
    );
    assert!(
        matches!(
            state.deferred_reentrant_messages.first(),
            Some(BrowserToContent::CursorLeft)
        ),
        "the exact buffered message is preserved (the Shutdown was surfaced from the channel)"
    );
}

/// **Finding 2 (FIFO preserve) — a non-`Shutdown` message read during a buffer-drain is
/// buffered, not dropped.** crossbeam has no peek/putback, so the intake's channel probe
/// (added for the `Shutdown` preempt above) consumes whatever it reads. A freshly-arrived
/// non-`Shutdown` message is NEWER than every buffered one, so it is pushed to the buffer
/// BACK and the buffer FRONT is still delivered this turn — preserving FIFO and never
/// dropping the probed message (Codex PR#469 R14).
///
/// Pins the new intake's ordering: with `A` buffered and a newer `B` on the channel, one
/// turn delivers `A` (the buffer front) and leaves `B` buffered for a later turn. A
/// regression that delivered `B` (out of order) or dropped it would leave the buffer as
/// `[A]` or `[]` respectively, not `[B]`. (`A` = `CursorLeft` with no hover dispatches as a
/// no-op; `B` is left buffered, never dispatched this turn.)
#[test]
fn buffer_drain_preserves_a_probed_non_shutdown_message_in_fifo_order() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // `A` is buffered (older); dispatching it (CursorLeft, no hover) is a no-op.
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::CursorLeft);
    // `B` (newer, distinct) arrives on the channel during the buffer-drain turn.
    browser
        .send(BrowserToContent::MouseRelease { button: 0 })
        .unwrap();

    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(flow.is_continue(), "a non-shutdown turn continues the loop");

    // A was delivered (removed from the front); B was probed off the channel and pushed
    // to the buffer BACK — so the buffer holds exactly [B], preserving FIFO.
    assert_eq!(
        state.deferred_reentrant_messages.len(),
        1,
        "Finding 2 — B was buffered (not dropped) and A was delivered (FIFO front-first)"
    );
    assert!(
        matches!(
            state.deferred_reentrant_messages.first(),
            Some(BrowserToContent::MouseRelease { button: 0 })
        ),
        "the buffer now holds B (the newer channel message) for a later turn; A (the older \
         buffered message) was delivered this turn"
    );
}

/// **R15 (teardown-priority through the channel) — a `Shutdown` BEHIND a normal message
/// still preempts a buffer-drain turn.** The step-1 buffer-drain arm now DRAINS the
/// channel non-blocking until `Shutdown` or `Empty` (not a single HEAD probe): a channel
/// `[MouseRelease, Shutdown]` had its `Shutdown` starved behind the `MouseRelease` under
/// the R14 single-probe intake (which saw only the HEAD, delivered the buffer front, and
/// returned `Continue`), deferring teardown for later turns — up to the ~30s SW deadline
/// while a non-empty buffer keeps re-delivering (Codex PR#469 R15).
///
/// Fail-before-fix: with `[CursorLeft]` buffered AND `[MouseRelease, Shutdown]` on the
/// channel, one `pump_turn` must BREAK — the drain buffers `MouseRelease`, reads
/// `Shutdown`, and hands it to step-2 teardown. The old single-probe intake returned
/// `Continue`, delivering `CursorLeft` and leaving the `Shutdown` unread behind the
/// `MouseRelease`. Both messages stay buffered in FIFO order — neither is dispatched on
/// the closing tab.
#[test]
fn channel_shutdown_behind_normal_message_still_preempts() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    // A buffered reentrant message awaiting one-per-turn re-delivery.
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::CursorLeft);
    // The channel holds a normal message AHEAD of the Shutdown — the exact R15 repro. A
    // single `try_recv` probe would observe only the MouseRelease (the channel HEAD).
    browser
        .send(BrowserToContent::MouseRelease { button: 0 })
        .unwrap();
    browser.send(BrowserToContent::Shutdown).unwrap();

    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);

    assert!(
        flow.is_break(),
        "R15 — the intake DRAINS the channel to the Shutdown, so it preempts even behind a \
         normal message; the old single-probe intake delivered the buffer front and \
         returned Continue, starving the Shutdown"
    );
    // The drain buffered MouseRelease to the buffer BACK (FIFO), then read the Shutdown and
    // broke at step 2 — so BOTH messages are preserved, in order, neither dispatched.
    assert_eq!(
        state.deferred_reentrant_messages.len(),
        2,
        "both the pre-buffered message and the drained channel message are kept"
    );
    assert!(
        matches!(
            state.deferred_reentrant_messages.as_slice(),
            [
                BrowserToContent::CursorLeft,
                BrowserToContent::MouseRelease { button: 0 }
            ]
        ),
        "FIFO preserved as [CursorLeft, MouseRelease]: the drained MouseRelease went to the \
         buffer BACK and neither was dispatched before the Shutdown broke the turn"
    );
}
