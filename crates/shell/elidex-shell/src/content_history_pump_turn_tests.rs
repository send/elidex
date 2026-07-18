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
//! Same-document entries (`push` + `push_same_document`, shared `document_sequence`)
//! take the no-fetch `same_document_step` path, so their Phase-2 apply succeeds in the
//! disconnected harness (a cross-document rebuild would fail — exercised elsewhere).

use elidex_navigation::{DrainCoordinator, DrainHost};
use elidex_script_session::HostDriver;

use super::test_support::{
    base, build_test_content_state_with_url, drain_browser, seed_same_document_pair,
};
use crate::ipc::BrowserToContent;

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
