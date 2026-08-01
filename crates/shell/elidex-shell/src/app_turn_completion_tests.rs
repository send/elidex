//! App-mode **turn-completion conformance** — the drive site runs the input turn
//! to quiescence (`docs/plans/2026-07-app-mode-turn-completion-drain.md` §8).
//!
//! Carved as a NEW sibling of `app_history_phase_sep_tests` while writing (the
//! touch-time split discipline): that file was 756 lines and its own module doc
//! already names its scenario seams, so growing it by this whole family would have
//! walked it into the 1000-line guideline PR #490 just discharged for this suite.
//! What lives here is the family the fix creates — the loop settling what a turn's
//! handlers staged — as opposed to how a turn PARTITIONS, which stays there.
//!
//! **Both `#11-app-mode-turn-completion-drain` pins moved here with the fix**, and
//! they are no longer divergence pins: after the flip they assert turn-completion
//! conformance, not the bounded-but-wrong status quo. The co-location rationale
//! survives the move — they are the *latency* and the *destructive* facet of one
//! slot ([`app_popstate_staged_pushstate_settles_within_the_same_turn`],
//! [`app_popstate_staged_push_lands_on_the_issuing_entry_across_an_interleaved_chrome_traversal`])
//! — and so does the content-side cross-check note: content-mode has *three*
//! popstate-staged pins spread across two files
//! (`content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn`
//! and `::pump_enqueues_popstate_staged_traversal_for_next_turn_not_same_turn`, plus
//! `content_history_pump_turn_tests::popstate_staged_pushstate_applied_with_held_navigate_fresh_and_buffered`),
//! so anyone changing the app-mode schedule again should check the content side too.
//!
//! The first test below is *both* the flipped latency pin and the app-mode twin of
//! content's Slice-A R9 pin — one scenario, one test, not two (the plan lists them
//! as separate bullets; they describe the same drive).
//!
//! **What is NOT pinned here, deliberately**: the *traversal*-granularity settle
//! (`#11-sync-navigation-steps-queue-tagging`) — a popstate-staged intent consumed
//! *between* two traversals of one Phase-2 snapshot — stays pinned as a divergence
//! by `app_history_phase_sep_tests::app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`.
//! Same mechanism, different granularity, different slot.
//!
//! Harness reachability (the disconnected network, the `render_state`-less window)
//! is contracted in `app_test_support`; the degraded-exit probe it adds for this
//! module is `staged_session_history_work` (the §4.4 peek — the ONLY honest way to
//! assert "work is still staged", since no residue flag exists).

use elidex_navigation::DrainHost;

use super::test_support::{
    app_at, base, current_url, cursor_over_content, document_marker, entry_url, eval, history_len,
    pipeline_url, popstate_every, popstate_fires, popstate_once, seed_same_document_pair,
    seed_same_document_triple, staged_session_history_work, stamped,
};

/// The adversarial re-stager body: ping-pong `forward()`/`back()` between two
/// same-document entries, so every loop iteration applies exactly one traversal
/// and fires exactly one `popstate` — which is what makes
/// [`popstate_fires`](super::test_support::popstate_fires) a direct count of the
/// loop's iterations. Shared so the cap tests and the residual pin cannot drift
/// apart on the shape that makes that counting valid.
const PING_PONG: &str = "if (window.__n % 2 === 1) { history.forward(); } else { history.back(); }";

// ---------------------------------------------------------------------------
// The two flipped slot pins
// ---------------------------------------------------------------------------

/// **The flipped latency pin** of `#11-app-mode-turn-completion-drain`, and the
/// app-mode twin of content's Slice-A R9 pin
/// (`content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn`)
/// — the shape app-mode had no counterpart for until the drive-site loop landed.
///
/// A same-document `back()` fires `popstate` **inside** Phase 2, and its handler's
/// `pushState` lands on the VM `pending_history` FIFO after Phase 1b has already
/// partitioned. ONE `drain_same_turn` therefore returns with it unsettled; the loop
/// runs a second iteration whose Phase 1b applies it, in the turn that staged it.
///
/// The end state is the one the SECOND drive used to produce — that is the point:
/// the fix is a schedule change, not an outcome change. So the discriminator is not
/// the entry list but **that no second drive is needed**, asserted as quiescence at
/// drive exit (the §4.4 peek reads false).
#[test]
fn app_popstate_staged_pushstate_settles_within_the_same_turn() {
    let mut app = app_at(
        &popstate_once("history.pushState(null, '', '/from-popstate');"),
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert!(
        !staged_session_history_work(&app),
        "the drive ran the turn to QUIESCENCE — the popstate-staged pushState was \
         consumed by a later iteration of the same drive, not left on the VM FIFO"
    );
    assert!(
        app.traversal_queue().is_empty(),
        "every iteration's Phase 2 emptied the queue — the exit assert's invariant \
         is per-iteration, not merely per-drive"
    );
    // `history_len` cannot say this: the push applied from the post-back cursor
    // (index 0), truncating the forward `/a` — [base, /from-popstate], still 2.
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/from-popstate"),
        "the staged pushState applied within THIS drive, truncating the forward /a \
         and appending itself in its place"
    );
    assert_eq!(
        history_len(&app),
        2,
        "truncate-then-append keeps the length"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/from-popstate"),
        "and the cursor moved onto it"
    );
}

/// **The flipped destructive pin** — the facet that raised the slot from "unbounded
/// latency" to **wrong-entry mutation** (Codex `/external-converge` R5/R8 + the R10
/// fix-delta gate on PR #487).
///
/// The scenario is unchanged: from `[base, /a, /b]` on `/b`, `history.back()`
/// applies → cursor `/a` → `popstate` stages `pushState('/from-popstate')` → the
/// user presses toolbar **Back**. What changed is that the turn no longer ends with
/// the intent unsettled, so the chrome traversal cannot move the cursor out from
/// under it: the push lands on `/a`, **the entry whose handler issued it**, which is
/// what WHATWG HTML §7.4.6.1 *Updating the traversable* step 14.1.1's note requires
/// (synchronous navigations *"jump the queue … before this traversal potentially
/// unloads their document"*).
///
/// `/b` is still destroyed — but by *finalize a same-document navigation*
/// §7.4.2.3.3 step 5.1 "Clear the forward session history of traversable" (invoked
/// from §7.4.4 step 13.1), i.e. by the push doing exactly what a push does from the
/// entry it belongs to. That is conformance, not the divergence: the divergence was
/// destroying `/a` — the issuing entry — as well.
///
/// The destruction scenario is not claimed unreachable: it survives on the
/// non-quiescent-exit paths, pinned as an ACCEPTED residual by
/// [`app_accepted_residual_from_a_non_quiescent_entry_drive_then_same_dispatch_mover`].
#[test]
fn app_popstate_staged_push_lands_on_the_issuing_entry_across_an_interleaved_chrome_traversal() {
    let mut app = app_at(
        &popstate_once("history.pushState(null, '', '/from-popstate');"),
        base(),
    );
    // Seed [base, /a, /b] sharing one document_sequence, cursor on /b.
    seed_same_document_triple(&mut app);

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert!(
        !staged_session_history_work(&app),
        "the turn completed — nothing is left for a later drive to apply against a \
         moved cursor"
    );
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/a"),
        "/a — the entry whose popstate handler issued the push — SURVIVES. That is \
         the whole flip: the divergence destroyed it, because the push used to be \
         applied from a cursor the chrome traversal had already moved to base"
    );
    assert_eq!(
        entry_url(&app, 2).as_deref(),
        Some("https://example.com/from-popstate"),
        "the push settled AFTER /a — the entry it belongs to per §7.4.6.1 step \
         14.1.1's note — not against wherever a later mover left the cursor"
    );
    assert_eq!(
        entry_url(&app, 0).as_deref(),
        Some("https://example.com/"),
        "base — behind the issuing entry — is untouched"
    );
    assert_eq!(
        history_len(&app),
        3,
        "[base, /a, /from-popstate]: the forward /b went with the push's own \
         §7.4.2.3.3 step 5.1 forward-clear, which is what a push from /a does. The \
         divergence was destroying /a as well, leaving 2"
    );

    // The toolbar Back now arrives AFTER the turn completed and simply traverses.
    app.handle_chrome_action(crate::chrome::ChromeAction::Back);

    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a"),
        "chrome Back traverses normally, /from-popstate → /a — there was no staged \
         residue for it to move out from under"
    );
    assert!(
        !staged_session_history_work(&app),
        "and it staged nothing new (the one-shot guard), so nothing is pending after it"
    );
}

// ---------------------------------------------------------------------------
// The traversal case — what the trailing-drain alternative gets wrong (axis e)
// ---------------------------------------------------------------------------

/// A popstate-staged **`back()`** — the case a trailing
/// `DrainCoordinator::drain_synchronous_updates` gets WRONG rather than merely
/// misses. That trailing drain would peek-classify the staged traversal
/// (Resolution E) and leave it RESIDENT on the queue across the turn boundary,
/// freezing its in-range classification a turn early: it would then seed
/// `seen_traversal` at the next Phase-1 entry and latch `suppress_default` at
/// Phase-1 exit, killing an unrelated `<a href>` default.
///
/// Iterating whole units instead classifies and applies each traversal in ONE
/// iteration, so all three of those facts are observable here: the cursor moved,
/// the queue is empty at drive exit, and a subsequent click's `<a href>` default is
/// NOT suppressed by a surviving latch.
#[test]
fn app_popstate_staged_traversal_applies_within_the_same_turn() {
    let mut app = app_at(
        &format!(
            "<a href=\"#frag\" style=\"display:block;width:200px;height:100px\">link</a>\
             <div id=\"frag\" style=\"height:2000px\"></div>\
             {}",
            popstate_once("history.back();")
        ),
        base(),
    );
    // [base, /a, /b] one document, cursor on /b — so the staged back() is in range.
    seed_same_document_triple(&mut app);

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert!(
        app.traversal_queue().is_empty(),
        "the staged back() was classified AND applied inside this drive — no \
         resident step whose classification could go stale across the turn boundary"
    );
    assert!(
        !staged_session_history_work(&app),
        "and nothing is left staged on the VM FIFO either"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/"),
        "both traversals applied in the one turn: /b → /a (the eval) → base (the \
         popstate-staged one)"
    );

    // The surviving-latch probe: a fresh click must fire its `<a href>` default.
    cursor_over_content(&mut app, 50.0, 50.0);
    app.handle_click(winit::event::MouseButton::Left);

    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/#frag"),
        "no `Traversal` step survived the drive, so the next turn's Phase 1 latched \
         no `suppress_default` and the <a href=\"#frag\"> default navigated \
         (Resolution E's no-over-suppression contract, preserved)"
    );
}

// ---------------------------------------------------------------------------
// Outcome accumulation across iterations (axis g)
// ---------------------------------------------------------------------------

/// The turn's [`DrainOutcome`](elidex_navigation::DrainOutcome) is the **field-wise
/// OR** of its iterations', not the last iteration's: `suppress_default` describes
/// the TURN.
///
/// The discriminating iteration is a `window.open`-only settle. The coordinator
/// deliberately excludes window-opens from `own_context_action` (they act on OTHER
/// browsing contexts), so an iteration that drains nothing else returns **every
/// field false** — and a last-iteration-wins merge would clear the
/// `suppress_default` iteration 1 latched for the in-range `back()`, firing an
/// `<a href>` default that must stay dropped. It is also why the quiescence
/// predicate must INCLUDE window-opens: excluding them would exit "quiescent" with
/// the open still staged on a runtime the next navigation replaces.
#[test]
fn app_turn_outcome_or_latches_suppress_default_across_iterations() {
    let mut app = app_at(
        &format!(
            "<a href=\"#frag\" style=\"display:block;width:200px;height:100px\">link</a>\
             <div id=\"frag\" style=\"height:2000px\"></div>\
             <script>\
               document.querySelector('a').addEventListener('click', function () {{\
                 document.querySelector('a').setAttribute('data-ran', '1');\
                 history.back();\
               }});\
             </script>\
             {}",
            popstate_once("window.open('/opened');")
        ),
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a
    cursor_over_content(&mut app, 50.0, 50.0);

    app.handle_click(winit::event::MouseButton::Left);

    assert!(
        stamped(&app, "a", "data-ran"),
        "the click listener fired — so the back() really was staged"
    );
    assert!(
        !staged_session_history_work(&app),
        "iteration 2 RAN and drained the popstate-staged window.open (drain-and-drop \
         in Phase 1a); a single-iteration drive would have left it staged"
    );
    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/"),
        "iteration 1's `suppress_default` survived iteration 2's all-false outcome, \
         so the <a href=\"#frag\"> default stayed dropped. The counterfactual is \
         https://example.com/#frag"
    );
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/a"),
        "and the entry list is intact — an unsuppressed default would have replaced \
         the forward /a with /#frag at the same length"
    );
    assert_eq!(history_len(&app), 2, "and appended nothing");
}

// ---------------------------------------------------------------------------
// Termination + degrade (§4.3)
// ---------------------------------------------------------------------------

/// A handler that re-stages **unconditionally** makes the fixpoint unreachable, and
/// this loop runs on the single-writer renderer thread — so the loop terminates at
/// `MAX_TURN_COMPLETION_ROUNDS` and DEGRADES instead of hanging.
///
/// The re-stager ping-pongs `forward()`/`back()` between two same-document entries,
/// so every iteration applies exactly one traversal and fires exactly one
/// `popstate`; `data-n` therefore counts the loop's iterations directly.
///
/// The degrade is asserted the only honest way: the work is still **STAGED** (the
/// §4.4 peek), not "a flag was set" — no residue flag exists, by design.
#[test]
fn app_turn_completion_terminates_at_the_cap_and_defers_the_residue() {
    let mut app = app_at(&popstate_every(PING_PONG), base());
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert_eq!(
        popstate_fires(&app),
        8,
        "the loop ran exactly MAX_TURN_COMPLETION_ROUNDS iterations — one traversal \
         apply (hence one popstate) each — and then stopped instead of hanging"
    );
    assert!(
        staged_session_history_work(&app),
        "the cap exit records NO state: the re-staged traversal simply stays on the \
         current runtime's channels for the next drive that is reached"
    );
}

// ---------------------------------------------------------------------------
// Mover-fired staging — staging source (b), pinned CLOSED
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The ACCEPTED residual (§7 Q3) — fenced, not a defect regression
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The swap exit's negative side (§4.5 (c))
// ---------------------------------------------------------------------------

/// **A FAILED mid-loop load must not fire the swap exit** — the regression guard
/// against ending the turn on a navigate *attempt* rather than on an OBSERVED
/// marker change (the round-2-rejected misimplementation).
///
/// `navigate` early-returns on a failed `load_url_into_pipeline`, BEFORE any
/// `push`/`replace`/`restamp_current_document`, so `document_sequence` — the loop's
/// swap marker — is unchanged and the old pipeline and its FIFO are intact. That is
/// the correct semantics: the turn's remaining staged work is still this turn's.
///
/// **Coverage limit, stated exactly.** The stronger assertion the plan sketches —
/// "work staged AFTER the failed navigate still settles within the same drive" — is
/// not writable, and not because of the harness: a failed cross-document navigate is
/// always the drive's LAST iteration by construction. Phase 1c runs after Phase 1b,
/// a failed load runs no script, and the only mid-drain script vector is the
/// `popstate` of a cursor-MOVING Phase-2 apply — which cancels the held navigation
/// (`DrainHost::apply_traversal`) instead of letting it run. So nothing can be
/// staged after the failed navigate to observe. What is asserted is the fact the
/// swap exit actually reads: the marker did not move.
#[test]
fn app_failed_mid_loop_load_does_not_move_the_document_marker() {
    let mut app = app_at(
        &popstate_once("location.href = 'https://example.com/gone';"),
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a
    let marker_before = document_marker(&app);

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert_eq!(
        document_marker(&app),
        marker_before,
        "the popstate-staged location.href reached Phase 1c in a later iteration and \
         its cross-document load FAILED, so nothing re-stamped the current entry's \
         document identity — the swap exit reads a marker CHANGE, never a navigate \
         attempt"
    );
    assert!(
        !staged_session_history_work(&app),
        "and the drive still reached quiescence"
    );
    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/"),
        "the old pipeline is intact — the failed load left the document in place"
    );
}
