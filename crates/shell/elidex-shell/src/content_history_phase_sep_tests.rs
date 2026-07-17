//! Content-mode Slice-A **phase-separation conformance** — the coordinator
//! sequencing + same-document-apply + URL-binding half of the drain suite
//! (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md`).
//!
//! Split out of `content_history_drain_tests.rs` at the file's authored
//! "Slice A phase-separation conformance (A / B / D / E + I1 + loop-inert)"
//! section boundary (touch-time 1000-line split, Codex PR#469 R3). The sibling
//! `content_history_drain_tests` keeps the core same-turn drain / navigation
//! ordering / failed-load / supersede-discard scenarios; this module keeps the
//! cross-task-boundary phase-separation conformance: I1 ordering across the task
//! boundary, later-turn `pump_turn` application, the bounded-drain loop-inert
//! assertions, peek-classify partition, default-suppression frame-ship, and the
//! cancellation of a deferred `SyncUpdate` that straddles a same-turn traversal
//! (Resolution D generalized, Codex PR#469 R6 — supersedes the R3 T3 call-time-URL
//! binding).
//!
//! Same-document entries (`push` + `push_same_document`, shared
//! `document_sequence`) take the no-fetch `same_document_step` path, so their
//! Phase-2 apply SUCCEEDS in the disconnected harness (a cross-document rebuild
//! would fail — that leg is pinned by the substrate isolation test
//! `traversal_queue_tests::syncupdate_canceled_after_document_changing_traversal`
//! plus VM/connected-integration coverage).

use elidex_navigation::{DrainCoordinator, DrainHost};
use elidex_script_session::HostDriver;

use super::test_support::build_test_content_state_with_url;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel};

/// A primary-button `MouseClickEvent` at viewport point `(x, y)` — drives the
/// `handle_click` path for the F3 frame-ship regression.
fn click_at(x: f32, y: f32) -> crate::ipc::MouseClickEvent {
    crate::ipc::MouseClickEvent {
        point: elidex_plugin::Point::new(x, y),
        client_point: elidex_plugin::Point::new(f64::from(x), f64::from(y)),
        button: 0,
        mods: crate::ipc::ModifierState::default(),
        placement_seq: 0,
    }
}

/// The top-level document URL every test builds against.
fn base() -> url::Url {
    url::Url::parse("https://example.com/").unwrap()
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

/// loop-bound (plan §1 loop-bound / T1): content's Phase-2 apply does NOT
/// re-enqueue, so `run_deferred_traversals` drains its bounded snapshot to empty in
/// one pass. The drain is bounded-by-construction (T1); the canonical reentrancy-guard
/// serialization WIRING for a reentrant DIRECT nav is Slice 4 (the reachable SW-pump
/// message vector is already closed this slice by `dispatch_or_buffer_reentrant`).
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

/// F4 (later traversal not dropped by the pre-apply peek): `back(); forward()` in
/// ONE turn from `[base, /a]` at `/a`. `back()` peek-classifies in-range (barrier);
/// `forward()` peeks the STILL-UNMOVED index-1 cursor (len 2) → out-of-range, but
/// because a barrier now exists it enqueues UNCONDITIONALLY (F4 — its target
/// resolves at Phase-2 apply time). Phase 2 applies BOTH (same-document): `back()`
/// → `base`, then `forward()` → `/a`, netting back on `/a`. The old pre-apply peek
/// DROPPED the forward, leaving Phase 2 to apply only `back()` → landing on `base`.
#[test]
fn back_then_forward_applies_both_and_nets_to_last_entry() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.back(); history.forward();");
    drain_browser(&browser);

    // Phase 1 enqueues BOTH — the forward() is not dropped by peeking the unmoved
    // cursor (F4).
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "both back() and forward() queued for Phase 2 (forward not dropped)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "Phase 1 only enqueued (cursor unmoved on /a)"
    );

    // Phase 2: back() → base, then forward() → /a. Net no-op landing on /a — NOT
    // base (the old drop left only back() to apply).
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(
        state.traversal_queue().is_empty(),
        "both traversals drained"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "back() then forward() nets to the last entry /a (F4: forward applied, not dropped → NOT base)"
    );
}

/// E divergence, STACKED (plan §5 / §6 Q-E accepted-bounded): `back(); back()` in
/// ONE turn peek-classifies BOTH backs against the UNMOVED cursor in Phase 1b
/// (peek-classify runs at enqueue against the pre-traversal list), so both classify
/// in-range and enqueue. `has_pending_traversal()` is therefore true after Phase 1
/// — a legitimate FIRST traversal IS pending, so the default-suppression predicate
/// is correct here (the queue-Traversal-pending shape means there is NO spurious
/// over-suppression). In Phase 2 the 1st back applies (same-document, `/a` → base,
/// ships one frame); the 2nd back re-peeks from the now-moved cursor (base), finds
/// no prior entry, and applies as a NO-OP that ships nothing (`apply_traversal`
/// correctly ships nothing for the no-op). The only residual is `deferred_own_context`
/// possibly over-set for the stacked case — pinned here as ACCEPTED, not slotted
/// (an accepted bounded behavior is not a platform gap). This test pins the plan §6
/// Q-E accepted-bounded divergence.
#[test]
fn stacked_back_back_second_traversal_is_a_noop() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.back(); history.back();");
    drain_browser(&browser);

    // Phase 1: both backs peek the unmoved cursor (index 1 → index 0) and enqueue.
    // A legitimate first traversal is pending → default-suppression is correct (no
    // spurious over-suppression from the queue-Traversal-pending shape).
    let outcome = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the stacked back();back() enqueued — a legitimate first traversal is pending"
    );
    assert!(
        outcome.suppress_default,
        "default-suppression is correct (a real first traversal is pending — not spurious)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "Phase 1 only enqueued (cursor unmoved on /a)"
    );
    drain_browser(&browser);

    // Phase 2: the 1st back applies (same-document, /a → base, ships ONE frame); the
    // 2nd back re-peeks from base, finds no prior entry → a NO-OP that ships nothing.
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(
        state.traversal_queue().is_empty(),
        "both queued backs drained (the 2nd as a no-op)"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/"),
        "landed on base — the 1st back applied, the 2nd was absorbed as a no-op"
    );
    assert_eq!(
        count_display_lists(&browser),
        1,
        "exactly ONE frame ships: the 1st same-document apply; the 2nd no-op ships nothing"
    );
}

/// F3 (default-suppression must not strand the re-rendered frame): a click whose
/// handler mutates the current document AND queues an in-range (same-document)
/// traversal. The traversal defers to Phase 2, so `own_context_action == false`
/// and the drain ships nothing; `suppress_default` is true (a traversal is
/// pending), which suppresses the `<a href>` default. The bug: the click handler's
/// early `return` on `suppress_default` SKIPPED `send_display_list()`, stranding
/// the DOM-mutating `re_render()`'d frame until Phase 2 (never sent if Phase 2's
/// apply fails). The fix ships the frame keyed on `!drained.shipped`, decoupled
/// from default-suppression — so a display list IS sent this turn.
#[test]
fn click_ships_mutated_frame_when_default_suppressed_by_pending_traversal() {
    // A click handler (registered via an inline `<script>`, so it runs through the
    // build-time flush and is live for `dispatch_event`) that mutates the current
    // document AND queues an in-range same-document back() (which defers to Phase 2
    // and suppresses the `<a href>` default).
    let (mut state, browser) = build_test_content_state_with_url(
        "<div id=\"btn\" style=\"display:block;width:200px;height:100px\">Click</div>\
         <script>\
           document.getElementById('btn').addEventListener('click', function () {\
             document.body.appendChild(document.createElement('span'));\
             history.back();\
           });\
         </script>",
        base(),
    );
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a → back() in-range
    state.re_render();
    drain_browser(&browser);

    super::event_handlers::handle_click(&mut state, &click_at(50.0, 50.0));

    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the handler's history.back() queued an in-range traversal (the default is suppressed)"
    );
    assert!(
        count_display_lists(&browser) >= 1,
        "the DOM-mutating re_render'd frame ships THIS turn (not stranded by suppress_default) — F3"
    );
}

/// T2 (Codex PR#469 R3, the LATER-TURN BOUNDARY): the `run_event_loop` pump applies
/// a Phase-2 traversal on a turn AFTER the input turn that enqueued it — NOT the
/// same iteration. `pump_turn` drains Phase 2 at the TOP of the turn, so a
/// `history.back()` an input handler enqueues (Phase 1b, inside `handle_message` →
/// `handle_click`) is NOT applied that turn; the NEXT pump turn's top-drain applies
/// it (plan §4.5 I1 "the async pump exposes the deferred apply only on a later
/// turn"). Regression: the old BOTTOM-of-loop drain applied the just-enqueued
/// traversal in the SAME iteration, collapsing the task boundary the
/// phase-separation exists to create.
#[test]
fn pump_turn_applies_enqueued_traversal_on_a_later_turn() {
    // A clickable element whose handler runs a same-document, in-range
    // history.back() — the input handler enqueues the traversal in Phase 1b.
    let (mut state, browser) = build_test_content_state_with_url(
        "<div id=\"btn\" style=\"display:block;width:200px;height:100px\">Back</div>\
         <script>\
           document.getElementById('btn').addEventListener('click', function () {\
             history.back();\
           });\
         </script>",
        base(),
    );
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a → back() in-range
    state.re_render();
    drain_browser(&browser);
    let mut last_frame = std::time::Instant::now();

    // Turn N: deliver the click through the full pump. The top-drain runs FIRST on an
    // empty queue (no-op), THEN `handle_message` → `handle_click` enqueues the back().
    browser
        .send(BrowserToContent::MouseClick(click_at(50.0, 50.0)))
        .unwrap();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(flow.is_continue(), "a click turn continues the loop");
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the click handler ENQUEUED the back() (Phase 1b) this turn"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "the back() was NOT applied on the enqueuing turn (cursor still /a) — the OLD \
         bottom-of-loop drain would have applied it HERE, collapsing the task boundary (T2)"
    );

    // Turn N+1: the top-of-loop Phase-2 drain applies the deferred back() BEFORE this
    // turn's message. A Shutdown makes `recv` return right after the top-drain.
    browser.send(BrowserToContent::Shutdown).unwrap();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(flow.is_break(), "Shutdown breaks the loop");
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/"),
        "the NEXT pump turn's top-drain applied the deferred back() → base \
         (later-turn boundary — plan §4.5 I1)"
    );
    assert!(
        state.traversal_queue().is_empty(),
        "the deferred back() drained on the later turn"
    );
}

/// R9 (Codex PR#469 — the reachable "navigation stuck" bug): a callback the pump
/// runs stages a §7.4.4 synchronous nav intent, and the pump DRAINS it THIS turn.
/// The top-of-turn `run_deferred_traversals` applies a same-document traversal
/// whose `popstate` handler calls `history.pushState('/frompop')` — which only
/// STAGES a `PushState` in the VM `pending_history` buffer. Before the fix the pump
/// never ran the Phase-1 synchronous drain (`drain_synchronous_phase` ran ONLY from
/// input handlers), so the staged pushState sat UNPROCESSED until an unrelated later
/// INPUT turn drained it — firing much too late. The bottom-of-turn Phase-1 drain
/// completes the event-loop turn: the popstate-staged pushState is drained + applied
/// THIS turn (`current_url` becomes `/frompop`), and `pending_history` no longer
/// holds it.
#[test]
fn pump_drains_popstate_staged_pushstate_this_turn() {
    let (mut state, browser) = build_test_content_state_with_url(
        "<p>doc</p>\
         <script>\
           window.addEventListener('popstate', function () {\
             history.pushState(null, '', '/frompop');\
           });\
         </script>",
        base(),
    );
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a → back() same-document
                                         // Queue a same-document back() (Phase 1); it applies at the TOP of the pump turn.
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the back() is queued for the next pump turn's top-of-loop Phase-2 apply"
    );
    drain_browser(&browser);

    // Drive ONE pump turn. Top-of-loop `run_deferred_traversals` applies the back()
    // → fires popstate → the handler stages a `pushState('/frompop')`. A benign
    // `CursorLeft` unblocks this turn's `recv_timeout` WITHOUT itself draining
    // history or shutting down, so the bottom-of-turn Phase-1 drain is what must
    // pick up the staged pushState.
    browser.send(BrowserToContent::CursorLeft).unwrap();
    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(
        flow.is_continue(),
        "a non-shutdown pump turn continues the loop"
    );

    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/frompop"),
        "the popstate-staged pushState was drained + applied THIS turn (R9) — not \
         stranded in pending_history until a later input"
    );
    assert!(
        state.pipeline.runtime.take_pending_history().is_empty(),
        "the VM pending_history buffer holds nothing after the turn — the callback-\
         staged intent was drained by the pump's bottom-of-turn Phase-1 drain"
    );
}

/// R9 + I1 (the traversal leg): when the callback-staged intent is a `Back` /
/// `Forward` / `Go`, the pump DRAINS it this turn but ENQUEUES it for the NEXT
/// turn's top-of-loop `run_deferred_traversals` — it is NOT applied same-turn
/// (`drain_synchronous_phase` only enqueues traversals; the apply is Phase 2, at the
/// TOP of a later turn). A `popstate` handler runs `history.back()` while the pump's
/// top-drain is applying an in-range back(); the staged Back is drained out of
/// `pending_history` (not stranded) AND enqueued, and applies only on the following
/// turn — preserving the §4.5 I1 task boundary.
#[test]
fn pump_enqueues_popstate_staged_traversal_for_next_turn_not_same_turn() {
    let (mut state, browser) = build_test_content_state_with_url(
        "<p>doc</p>\
         <script>\
           window.addEventListener('popstate', function () {\
             history.back();\
           });\
         </script>",
        base(),
    );
    // Three same-document entries [base, /a, /b], cursor on /b.
    state.nav_controller.push(base()); // 0
    state
        .nav_controller
        .push_same_document(url::Url::parse("https://example.com/a").unwrap()); // 1
    state
        .nav_controller
        .push_same_document(url::Url::parse("https://example.com/b").unwrap()); // 2
                                                                                // Queue the first back() (from /b, in-range → applies at the top of turn 1).
    let _ = state.pipeline.runtime.vm().eval("history.back();");
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    drain_browser(&browser);

    // Turn 1: top-drain applies back() (/b → /a) → popstate → the handler stages a
    // `history.back()` (in-range from /a). The bottom-of-turn Phase-1 drain ENQUEUES
    // it (does NOT apply it — I1).
    browser.send(BrowserToContent::CursorLeft).unwrap();
    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(flow.is_continue());
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "the popstate-staged back() was drained from pending_history AND enqueued this turn"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/a"),
        "I1: the popstate-staged traversal is NOT applied same-turn (cursor still /a) — \
         drain_synchronous_phase enqueues it for the NEXT run_deferred_traversals"
    );
    assert!(
        state.pipeline.runtime.take_pending_history().is_empty(),
        "the VM pending_history buffer is empty — the staged Back was drained (enqueued), \
         not left to fire on a later input"
    );

    // Turn 2: the top-of-loop Phase-2 drain applies the enqueued back() (/a → base) —
    // proving the enqueue from turn 1 lands on the NEXT turn (I1). Its own popstate
    // stages a back() from `base` (out of range → a no-op that enqueues nothing), so
    // the loop self-terminates.
    browser.send(BrowserToContent::CursorLeft).unwrap();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(flow.is_continue());
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/"),
        "the turn-1-enqueued traversal applied on turn 2's top-of-loop Phase-2 drain (I1)"
    );
    assert!(
        state.traversal_queue().is_empty(),
        "the out-of-range popstate back() on turn 2 enqueued nothing — the queue drained"
    );
}

/// Resolution D GENERALIZED (Codex PR#469 R6) — SUPERSEDES the R3 T3
/// call-time-URL binding: a `SyncUpdate` that STRADDLES a same-turn traversal is
/// CANCELED, not applied against the post-traversal cursor. `back();
/// replaceState('/x')` from `/a` on `[base, /a]` → back() applies same-document to
/// `base` (correct landing), and the deferred replaceState is DROPPED — final on
/// `base`, list still `[base, /a]`. Applying the straddle replaceState against the
/// post-traversal cursor (`base`) would corrupt the current entry (land `/x`-current
/// with list `[/x, /a]`); the correct §7.4.1.3 "Centralized modifications of session
/// history" jump-the-queue application to the CALL-TIME entry (before the traversal
/// moves the cursor) is fenced to `#11-sync-navigation-steps-queue-tagging`. Pinned,
/// not silent (supported-surface testing): the bounded divergence is the lost
/// straddle update, not the previously-corrupt current entry.
#[test]
fn deferred_syncupdate_canceled_behind_same_document_traversal() {
    let (mut state, browser) = build_test_content_state_with_url(
        "<p>doc</p>",
        url::Url::parse("https://example.com/a").unwrap(),
    );
    seed_same_document_pair(&mut state); // [base, /a], cursor on /a; call-time URL = /a
    let _ = state
        .pipeline
        .runtime
        .vm()
        .eval("history.back(); history.replaceState(null, '', '/x');");
    drain_browser(&browser);

    // Phase 1: back() enqueued (barrier); the replace('/x') defers behind it (I2).
    let _ = DrainCoordinator::drain_synchronous_phase(&mut state);
    assert!(
        state.traversal_queue().has_pending_traversal(),
        "back() queued; the straddle replaceState deferred behind it (I2)"
    );

    // Phase 2: same-document back() → base; the deferred straddle replaceState is
    // CANCELED (Resolution D generalized) — the current entry stays `base`, NOT `/x`.
    let _ = DrainCoordinator::run_deferred_traversals(&mut state);
    assert!(state.traversal_queue().is_empty(), "queue drained");
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::as_str),
        Some("https://example.com/"),
        "back() landed on base and the straddle replaceState was CANCELED — the \
         current entry is the back target (base), not the corrupt /x (R6)"
    );
    assert_eq!(
        state.nav_controller.len(),
        2,
        "the entry list is unchanged [base, /a] — the canceled replaceState \
         mutated nothing"
    );
}

/// **Interim reentrancy guard** (Codex PR#469 R4): a nav-mutating `BrowserToContent`
/// buffered while a Phase-2 apply held the peek→commit window does NOT mutate the
/// `NavigationController` while it sits in the buffer, and IS replayed + applied on
/// a later `pump_turn` once `is_applying()` has cleared.
///
/// This pins the guard's REPLAY contract and its no-mutation-while-buffered
/// invariant (the reachable corruption window the guard closes: a re-dispatched
/// message must not mutate the entry list between the in-flight traversal's peek and
/// its commit). The buffered state is simulated directly — the buffering DECISION
/// under `is_applying()` is exercised separately by `dispatch_or_buffer_reentrant`
/// (see `interim_guard_dispatches_reentrant_when_not_applying`), while the SW-fetch
/// wait loop that SETS `is_applying()` true is not unit-drivable (its internally
/// generated `fetch_id` cannot be matched to break the blocking wait without a 30s
/// timeout). Uses a same-document fragment `Navigate` so the replay applies in the
/// disconnected harness (no fetch).
#[test]
fn interim_guard_buffered_nav_replays_on_later_pump_turn() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base()); // one entry; pipeline.url = base
    drain_browser(&browser);

    let len_before = state.nav_controller.len();
    let url_before = state.nav_controller.current_url().map(url::Url::to_string);

    // A nav-mutating message arrives mid-apply and is BUFFERED (as the SW-wait guard
    // does while `is_applying()` holds) — a same-document fragment nav.
    let frag = url::Url::parse("https://example.com/#frag").unwrap();
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Navigate(frag));

    // While buffered, it has mutated NOTHING — the entry list/cursor are unchanged
    // (no mutation between the in-flight traversal's peek and its commit).
    assert_eq!(
        state.nav_controller.len(),
        len_before,
        "a buffered nav must not mutate the entry list while it waits"
    );
    assert_eq!(
        state.nav_controller.current_url().map(url::Url::to_string),
        url_before,
        "a buffered nav must not move the cursor while it waits"
    );

    // A later pump turn replays the buffer (after the apply committed / guard clear).
    // A same-document fragment nav signals no exit, so replay reports `Continue`.
    assert!(
        super::event_loop::replay_deferred_reentrant_messages(&mut state).is_continue(),
        "a buffered non-Shutdown nav replays without signalling the thread to exit"
    );

    assert!(
        state.deferred_reentrant_messages.is_empty(),
        "the buffer drains on replay"
    );
    assert_eq!(
        state.pipeline.url.as_ref().map(url::Url::as_str),
        Some("https://example.com/#frag"),
        "the buffered fragment nav applied on the replay turn"
    );
    assert_eq!(
        state.nav_controller.len(),
        len_before + 1,
        "the replayed fragment nav pushed its same-document entry (applied after the window)"
    );
}

/// **Interim reentrancy guard** — the common (non-applying) path is UNCHANGED: when
/// NO Phase-2 apply is in progress (`is_applying()` false), a re-dispatched message
/// is dispatched SYNCHRONOUSLY (not buffered), so a normal SW-fetch re-dispatch does
/// not regress. The fragment nav applies immediately and the buffer stays empty.
#[test]
fn interim_guard_dispatches_reentrant_when_not_applying() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base());
    drain_browser(&browser);

    assert!(
        !state.traversal_queue().is_applying(),
        "no Phase-2 apply is in progress (the common case)"
    );
    let len_before = state.nav_controller.len();

    let frag = url::Url::parse("https://example.com/#frag").unwrap();
    super::drain_host::dispatch_or_buffer_reentrant(&mut state, BrowserToContent::Navigate(frag));

    assert!(
        state.deferred_reentrant_messages.is_empty(),
        "with no apply in progress the message dispatches synchronously — not buffered"
    );
    assert_eq!(
        state.pipeline.url.as_ref().map(url::Url::as_str),
        Some("https://example.com/#frag"),
        "the synchronously-dispatched fragment nav applied immediately (common path unchanged)"
    );
    assert_eq!(
        state.nav_controller.len(),
        len_before + 1,
        "the immediate fragment nav pushed its same-document entry"
    );
}

/// **Interim reentrancy guard** — the DEFENSIVE R5 replay-exit backstop (Codex
/// PR#469 R5, retained under R8). Any exit-signaling message that reaches the buffer
/// must PROPAGATE its exit signal when replayed: `handle_message(Shutdown)` returns
/// the content-thread exit signal (`false`) after running unload/teardown;
/// `replay_deferred_reentrant_messages` must surface it as `ControlFlow::Break` and
/// `pump_turn` must RETURN that Break so `run_event_loop` exits.
///
/// RE-ANCHORED (R8): the LIVE guard no longer routes `Shutdown` into this buffer — it
/// is now handled IMMEDIATELY at `dispatch_or_buffer_reentrant` (teardown + the
/// `shutdown_requested` flag) so teardown does not wait on the ~30s SW deadline (see
/// `interim_guard_shutdown_handled_immediately_not_buffered`). This test injects a
/// `Shutdown` DIRECTLY into the buffer to pin the retained defensive propagation — so a
/// buffered exit-signaling message can never leave the pump hung (the original R5 bug:
/// the replay discarded the signal, so the Shutdown was consumed from the buffer yet
/// the pump kept running, then blocked on the next `recv`).
#[test]
fn interim_guard_buffered_shutdown_breaks_the_pump() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    drain_browser(&browser);

    // A Shutdown arrived mid-apply and was BUFFERED (as the SW-wait guard does while
    // `is_applying()` holds) rather than dispatched inline.
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Shutdown);

    // Replaying it propagates the exit signal as Break — NOT discarded.
    let replay_flow = super::event_loop::replay_deferred_reentrant_messages(&mut state);
    assert!(
        replay_flow.is_break(),
        "a buffered Shutdown's exit signal must propagate as ControlFlow::Break — not be \
         discarded (which would leave the content thread pumping, then blocked on recv)"
    );
    assert!(
        state.deferred_reentrant_messages.is_empty(),
        "the buffer drains on replay (the Shutdown was consumed)"
    );

    // And driven through the whole pump: a buffered Shutdown replayed at the top of
    // `pump_turn` makes the turn RETURN Break (the content thread exits), not Continue.
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    drain_browser(&browser);
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Shutdown);
    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(
        flow.is_break(),
        "pump_turn must return Break when the top-of-turn replay hits a buffered Shutdown \
         (so run_event_loop exits) — the old code swallowed the signal and continued"
    );
}

/// **Interim reentrancy guard** (Codex PR#469 R8): a `Shutdown` arriving at the
/// reentrancy vector (`dispatch_or_buffer_reentrant`) is handled IMMEDIATELY — it runs
/// unload/teardown and sets `shutdown_requested` — and is NEVER buffered. This is the
/// follow-on to R5: R5 made a buffered Shutdown's exit signal PROPAGATE on replay, but a
/// buffered Shutdown still could not be OBSERVED until the SW-fetch wait loop unblocked,
/// which — for a delayed/lost `SwFetchResponse` during an SW-controlled cross-document
/// traversal — is only at the ~30s navigation deadline. So a tab/window close would hang
/// teardown for up to 30s even though the Shutdown was already consumed from the channel.
///
/// The fix short-circuits `Shutdown` BEFORE the `is_applying()` buffer branch, so it holds
/// under BOTH the guarded (mid-apply) and common vectors — the buffering DECISION being
/// is_applying-independent for Shutdown is the whole point. The guarded (`is_applying()`)
/// SW-wait that would otherwise buffer it is not itself unit-drivable (its internally
/// generated `fetch_id` cannot be matched to break the blocking wait without a 30s
/// timeout — see `interim_guard_buffered_nav_replays_on_later_pump_turn`), so the contract
/// is asserted at the `dispatch_or_buffer_reentrant` / `pump_turn` level directly, as the
/// sibling interim-guard tests do.
#[test]
fn interim_guard_shutdown_handled_immediately_not_buffered() {
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    drain_browser(&browser);

    assert!(
        !state.shutdown_requested,
        "precondition: the thread is live"
    );

    // A Shutdown is re-dispatched from the SW-fetch wait loop (the reentrancy vector).
    super::drain_host::dispatch_or_buffer_reentrant(&mut state, BrowserToContent::Shutdown);

    // Handled IMMEDIATELY: teardown ran (flag set) and it did NOT sit in the buffer
    // waiting on the ~30s SW deadline (the R8 hang this pins).
    assert!(
        state.shutdown_requested,
        "a Shutdown at the reentrancy vector runs teardown immediately and flags the exit \
         — not deferred to a later replay turn"
    );
    assert!(
        state.deferred_reentrant_messages.is_empty(),
        "a Shutdown is NEVER buffered — buffering it would delay teardown until the SW-wait's \
         ~30s deadline could unblock and let a later pump_turn replay observe it"
    );

    // The pump then exits PROMPTLY: pump_turn observes `shutdown_requested` right after the
    // Phase-2 drain and returns Break WITHOUT blocking on this turn's recv_timeout.
    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(
        flow.is_break(),
        "pump_turn must return Break as soon as it observes shutdown_requested (prompt exit, \
         no wait for the SW timeout)"
    );
}

/// **Interim reentrancy guard** (Codex PR#469 R8 **re-check**): the REPLAY phase is the
/// third — and previously uncovered — pump phase that can newly set `shutdown_requested`.
/// A replayed nav-mutating message (`Navigate`/`Reload`/`GoBack`/`GoForward`) whose nested
/// SW-wait consumes a re-dispatched `Shutdown` runs teardown + sets the flag, yet
/// `handle_message` returns `true` (those arms discard `handle_navigate`'s `false`), so the
/// replay swallows the flag-set. Without a post-replay check, `pump_turn` would then block
/// one poll interval on `recv_timeout` and run a full frame tick on the torn-down pipeline.
///
/// This pins BOTH halves of the fix:
///  1. `replay_deferred_reentrant_messages` STOPS dispatching the rest of the batch once
///     the flag is set mid-replay — a message ordered AFTER the shutdown-consuming one is
///     NOT dispatched on the torn-down pipeline (proven by exactly ONE `SwFetchRequest`
///     reaching the browser: a second dispatch would send a second one).
///  2. `pump_turn` returns `Break` right after the replay — before `recv_timeout` /
///     the frame tick — completing the "check `shutdown_requested` after every pump
///     phase" invariant.
///
/// Unlike the sibling guards (which cannot match the internally generated `fetch_id` and so
/// simulate), this drives the REAL SW-wait: seeding a controller scope makes an in-scope
/// nav take the wait path, and a PRE-queued `Shutdown` is re-dispatched there immediately
/// (no 30s deadline). A second pre-queued `Shutdown` makes a regression of the stop fail
/// FAST (a second `SwFetchRequest`) instead of hanging on the ~30s SW deadline.
#[test]
fn interim_guard_replay_stops_and_pump_breaks_on_nested_shutdown() {
    let scope = || url::Url::parse("https://example.com/app/").unwrap();

    // --- Half 1: the replay loop STOPS after the flag is set; the second nav is not run.
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base());
    // Control the page so an in-scope cross-document nav takes the SW-wait path (the
    // reentrancy vector `dispatch_or_buffer_reentrant` guards).
    state.pipeline.runtime.seed_sw_client(Some(scope()), &[]);
    drain_browser(&browser);

    // Two SW-controlled cross-document navs buffered. The FIRST will consume a Shutdown in
    // its nested SW-wait (teardown + `shutdown_requested`); the SECOND must NOT dispatch.
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Navigate(
            url::Url::parse("https://example.com/app/one").unwrap(),
        ));
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Navigate(
            url::Url::parse("https://example.com/app/two").unwrap(),
        ));

    // Queue the Shutdown the first nav's SW-wait picks up immediately (no 30s deadline).
    // The second Shutdown only matters if the stop-on-flag regresses (fail fast, no hang).
    browser.send(BrowserToContent::Shutdown).unwrap();
    browser.send(BrowserToContent::Shutdown).unwrap();

    let flow = super::event_loop::replay_deferred_reentrant_messages(&mut state);

    assert!(
        state.shutdown_requested,
        "the first replayed nav's nested SW-wait consumed a Shutdown → teardown ran + flag set"
    );
    assert!(
        flow.is_continue(),
        "a nested-SW-wait Shutdown is surfaced via `shutdown_requested` (caught by \
         pump_turn's post-replay check), so replay itself reports Continue — not Break"
    );
    assert!(
        state.deferred_reentrant_messages.is_empty(),
        "the batch was taken for replay; nothing re-buffered"
    );
    let mut sw_fetch_count = 0;
    while let Ok(msg) = browser.try_recv() {
        if matches!(msg, ContentToBrowser::SwFetchRequest { .. }) {
            sw_fetch_count += 1;
        }
    }
    assert_eq!(
        sw_fetch_count, 1,
        "only the FIRST buffered nav dispatched (one SwFetchRequest); the SECOND was dropped \
         once the replay saw shutdown_requested — never run on the torn-down pipeline"
    );

    // --- Half 2: driven through the whole pump — pump_turn Breaks right after the replay,
    // NOT blocking on recv_timeout or running the frame tick on the torn-down pipeline.
    let (mut state, browser) = build_test_content_state_with_url("<p>doc</p>", base());
    state.nav_controller.push(base());
    state.pipeline.runtime.seed_sw_client(Some(scope()), &[]);
    drain_browser(&browser);
    state
        .deferred_reentrant_messages
        .push(BrowserToContent::Navigate(
            url::Url::parse("https://example.com/app/one").unwrap(),
        ));
    browser.send(BrowserToContent::Shutdown).unwrap();

    let mut last_frame = std::time::Instant::now();
    let flow = super::event_loop::pump_turn(&mut state, &mut last_frame);
    assert!(
        flow.is_break(),
        "pump_turn must Break as soon as the replay sets shutdown_requested — before \
         recv_timeout blocks a poll interval and the frame tick touches the torn-down pipeline"
    );
    assert!(
        state.shutdown_requested,
        "the flag set by the replayed nav's nested SW-wait is what drove the Break"
    );
}
