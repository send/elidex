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

use super::drain_host::MAX_TURN_COMPLETION_ROUNDS;
use super::test_support::{
    app_at, base, current_url, cursor_over_content, document_marker, entry_url, eval, history_len,
    pipeline_url, popstate_every, popstate_fires, popstate_once, seed_cross_document_pair,
    seed_same_document_pair, seed_same_document_triple, staged_session_history_work, stamped,
};

/// The adversarial re-stager body: ping-pong `forward()`/`back()` between two
/// same-document entries, so every loop iteration applies exactly one traversal
/// and fires exactly one `popstate` — which is what makes
/// [`popstate_fires`](super::test_support::popstate_fires) a direct count of the
/// loop's iterations. Named rather than inlined because that counting argument —
/// one apply, one `popstate`, one iteration — is the whole reason the cap pin's
/// `assert_eq!` is a valid iteration count, and it belongs next to the script it
/// is an argument about.
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
/// The destruction scenario is **not claimed unreachable**, and nothing here pins
/// that it is. It survives on the loop's non-quiescent exits (cap-hit, swap): the
/// residue then waits for the next drive that is actually REACHED, and a mover can
/// run first. Bounding that residue is a separate plan-reviewed slice alongside
/// Slice 4's mover routing (`#11-session-history-task-queue-model`) — see the
/// residue note on `App::process_pending_navigation`.
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
fn app_turn_completion_terminates_at_the_cap_and_leaves_the_residue_staged() {
    let mut app = app_at(&popstate_every(PING_PONG), base());
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert_eq!(
        popstate_fires(&app),
        MAX_TURN_COMPLETION_ROUNDS,
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
// The reinstatement tail's PLACEMENT (§4.2) — inside the iteration, not after
// the loop
// ---------------------------------------------------------------------------

/// **The reinstatement tail must run INSIDE each iteration**, not once after the
/// loop — the placement two doc contracts rest on
/// (`App::process_pending_navigation`'s "else a `location.*` issued in N would
/// apply after intents issued in N+1", and `InteractiveState::deferred_navigation`'s
/// "provably `None` at every iteration boundary, so there is no cross-iteration
/// overwrite case to define").
///
/// The tail reads exactly like a per-DRIVE tail — it was one before the loop
/// landed — so the regression is a later cleanup hoisting it out of the `for` body.
/// Without this pin that hoist is invisible: every other app-mode test stays green.
///
/// The discriminator is a turn whose LATER work exists only because the tail ran
/// in-iteration:
/// 1. `back()` (cross-document, so its load fails in this harness) + a
///    `location.href` to a fragment. Phase 1b queues the traversal, so Phase 1c
///    SUPPRESSES the navigation and holds it; Phase 2's failed load moves no
///    cursor, so the hold is refuted rather than cancelled.
/// 2. The in-iteration tail reinstates it. It resolves SameDocument, so
///    `same_document_step` runs and its `hashchange` handler runs **within this
///    turn** — the drain-timing divergence from WHATWG HTML §7.4.6.2 step 6.4.5
///    that [`app_same_document_navigate_mid_loop_does_not_end_the_turn`] states in
///    full. The discriminator below depends on it, so a change to `hashchange`
///    scheduling will fail THIS test too, and that is why.
/// 3. That handler stages a `pushState` — which only a LATER iteration can apply.
///
/// Hoist the tail and step 2 happens after the loop has already exited (Phase 1c
/// drained the nav slot, so the peek reads false and iteration 1 is the last one):
/// the `hashchange`-staged `pushState` is then stranded, and both assertions below
/// fail.
#[test]
fn app_reinstated_navigation_runs_in_iteration_so_its_own_staging_settles() {
    let mut app = app_at(
        "<p>doc</p>\
         <script>\
           window.addEventListener('hashchange', function () {\
             history.pushState(null, '', '/from-hash');\
           });\
         </script>",
        base(),
    );
    // [base, /a] as DISTINCT documents, cursor on /a — so `back()` classifies
    // Rebuild and its cross-document load fails, leaving the cursor unmoved.
    seed_cross_document_pair(&mut app);

    eval(&mut app, "history.back(); location.href = '#one';");
    let _ = app.process_pending_navigation();

    assert!(
        !staged_session_history_work(&app),
        "the drive reached quiescence: the tail's same-document navigate ran INSIDE \
         iteration 1, so the `pushState` its `hashchange` staged was still this \
         drive's business and iteration 2 applied it. A tail hoisted out of the loop \
         runs after the last iteration and strands it here"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/from-hash"),
        "and it applied: /a -> /a#one (the reinstated navigation) -> /from-hash (what \
         its own hashchange handler staged)"
    );
}

// ---------------------------------------------------------------------------
// The ACCEPTED residual (§7 Q3) — a mover runs while work is staged
// ---------------------------------------------------------------------------

/// **The accepted residual, pinned at its mechanism** — a non-drain cursor mover
/// runs while session-history work is staged, and the next drive then applies that
/// work against the cursor the mover moved. This is `#11-…-turn-completion-drain`'s
/// §1 harm, still live after this slice, fenced to the withdrawn-work slice
/// alongside Slice 4's mover routing (`#11-session-history-task-queue-model`).
///
/// **It is NOT a regression pin — it pins behavior this slice deliberately leaves
/// unchanged**, which is why it asserts the divergent outcome rather than the
/// correct one. Read it with the drive site's residue note.
///
/// The earlier form of this pin drove the whole chain through the loop's cap exit.
/// This one pins the *mechanism* instead — that the mover drains nothing — because
/// the mechanism is what the fenced slice will change: the day a drive lands on the
/// mover's own dispatch, the middle assertion below flips first and says so
/// directly, instead of a cap-length chain failing for an unrelated-looking reason.
///
/// `handle_chrome_action` is invoked directly; in production its sole app-mode
/// caller is `handle_redraw_inline`'s tail.
#[test]
fn app_mover_does_not_drain_staged_work_and_the_next_drive_applies_it_at_the_moved_cursor() {
    let mut app = app_at("<p>doc</p>", base());
    // [base, /a, /b] sharing one document, cursor on /b.
    seed_same_document_triple(&mut app);

    // Stage a pushState WITHOUT driving — the state a turn is left in by either
    // non-quiescent exit (cap-hit, swap), and by mover-fired popstate staging.
    eval(&mut app, "history.pushState(null, '', '/staged');");
    assert!(
        staged_session_history_work(&app),
        "precondition: the pushState is staged on the VM FIFO, undrained"
    );

    // A non-drain cursor mover, with that work still staged.
    app.handle_chrome_action(crate::chrome::ChromeAction::Back);

    assert!(
        staged_session_history_work(&app),
        "THE RESIDUAL, at its mechanism: chrome Back reaches `traverse_to` directly \
         and never routes through `process_pending_navigation`, so it moved the \
         cursor WITHOUT draining the staged intent. When the fenced slice puts a \
         drain in front of the movers, this assertion is the first one to flip"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a"),
        "and the cursor really did move, /b -> /a"
    );

    // The next drive that is reached now applies the staged intent — against the
    // moved cursor, not the entry it was issued from.
    let _ = app.process_pending_navigation();

    assert_eq!(
        entry_url(&app, 2).as_deref(),
        Some("https://example.com/staged"),
        "DIVERGENCE (pinned, not fixed): the push was issued while /b was current, \
         so it belongs AFTER /b; it landed after /a instead — and took /b with it \
         via `push_entry`'s truncate. WHATWG HTML §7.4.6.1 step 14.1.1's note is \
         explicit that a synchronous navigation settles before a traversal can \
         unload its document"
    );
    assert_eq!(
        history_len(&app),
        3,
        "[base, /a, /staged] — the live forward entry /b the user could still have \
         returned to is destroyed. This is §1's harm, unchanged by this slice"
    );
    assert!(
        !staged_session_history_work(&app),
        "the drive itself reached quiescence — the residual is about WHEN the drive \
         happens relative to the mover, never about the drive being incomplete"
    );
}

// ---------------------------------------------------------------------------
// The swap exit's negative side (§4.5 (c)) — the marker is an OBSERVED change,
// never a navigate attempt
// ---------------------------------------------------------------------------

/// **A FAILED mid-loop load must not move the swap marker.** `navigate`
/// early-returns on a failed `load_url_into_pipeline`, BEFORE any
/// `push`/`replace`/`restamp_current_document`, so `document_sequence` — the loop's
/// swap marker — is unchanged and the old pipeline and its FIFO are intact. That is
/// the correct semantics: the turn's remaining staged work is still this turn's.
///
/// **What this test does and does not pin, stated exactly.** It pins the *input* to
/// the swap comparison (the marker is stable across a failed load), not the
/// comparison itself: on THIS scenario a break-on-navigate-*attempt*
/// misimplementation would also pass, because a failed cross-document navigate is
/// always the drive's LAST iteration by construction — Phase 1c runs after Phase 1b,
/// a failed load runs no script, and the only mid-drain script vector is the
/// `popstate` of a cursor-MOVING Phase-2 apply, which CANCELS the held navigation
/// (`DrainHost::apply_traversal`) rather than letting it run. So nothing can be
/// staged after the failed navigate for a continuation assertion to observe. The
/// comparison's *sensitivity* — "a successful same-document navigate must not end
/// the turn" — is pinned by the sibling
/// [`app_same_document_navigate_mid_loop_does_not_end_the_turn`], whose mid-loop
/// navigate SUCCEEDS and stages more work behind itself.
///
/// **Neither pins that the branch EXISTS.** Deleting `if
/// self.current_document_marker() != doc_marker { break; }` outright leaves the
/// whole `elidex-shell` suite green (mutation-verified), because every scenario the
/// disconnected harness can build is one where the branch would not have fired
/// anyway. Pinning its presence needs the firing side, i.e. a successful
/// cross-document rebuild mid-loop, which the harness cannot produce — so the
/// branch's presence, not merely its end-to-end behavior, is part of the plan's
/// own-deferral (§9 #1). Do not read the two pins as covering it.
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

/// **A SUCCESSFUL mid-loop navigate that is same-document must not end the turn** —
/// the regression guard on the swap comparison itself (the round-2-rejected
/// misimplementation that ended the loop on a navigate *attempt*).
///
/// The chain, one drive: `back()` applies → `popstate` stages
/// `location.href = '#frag'` → iteration 2's Phase 1c runs it, and because it
/// classifies `SameDocument` it takes `same_document_step` (`app/navigation.rs`) —
/// a real, SUCCEEDING navigate — → `hashchange` → that handler stages a
/// `pushState` → iteration 3 applies it.
///
/// The `hashchange` leg rides a **known in-tree divergence**, stated here so the
/// chain is not mistaken for spec-shaped — and stated precisely, because the
/// obvious summary is wrong. WHATWG HTML §7.4.6.2 *update document for history
/// step application* step 6.4.5 **queues a global task** on the DOM manipulation
/// task source to fire `hashchange`, and elidex **does queue it** —
/// `VmInner::deliver_history_step_events` calls
/// `queue_task(PendingTask::HashChange)`. The divergence is the **drain timing**:
/// it then calls `drain_tasks()` immediately, settling that task inside the same
/// turn instead of leaving it for a later one. So the handler stages into this
/// turn. The test pins elidex's behavior; it does not claim the spec settles a
/// queued `hashchange` task inside the turn that queued it.
///
/// A same-document navigate does NOT re-stamp `document_sequence` (the fragment arm
/// takes `push_same_document`, which INHERITS the current document identity), so the
/// marker is unchanged and the loop correctly CONTINUES — which is right, since the
/// staged follow-ups are this turn's own work.
///
/// **This test pins the swap marker's DEFINITION directly** — what the §4.5 (c)
/// argument rests on, and what no assertion about the loop itself can reach: make
/// `same_document_step`'s fragment arm re-stamp the document identity and the
/// quiescence and entry assertions below fail (the loop takes the swap exit after
/// iteration 2 and strands the `hashchange`-staged `pushState`).
/// **Two** tests fail under that mutation — this one and
/// [`app_reinstated_navigation_runs_in_iteration_so_its_own_staging_settles`],
/// which trips it incidentally because its reinstated `location.href = '#one'`
/// also takes the fragment arm, inside iteration 1 and before the marker
/// comparison. **Re-measure that count whenever a test is added that reaches the
/// fragment arm** — a "only this test covers it" claim here goes stale silently,
/// since nothing fails when it does.
///
/// What IS unique here is the scenario: the only mid-loop navigate that SUCCEEDS
/// from Phase 1c — the failed-load sibling reaches `navigate`'s early return —
/// hence the only coverage of a completed Phase-1c navigation inside the loop.
#[test]
fn app_same_document_navigate_mid_loop_does_not_end_the_turn() {
    // The popstate half is the shared one-shot builder — the fragment nav's own
    // popstate must re-enter the handler as a plain no-op, which is exactly the
    // guard `popstate_once` owns. Only the hashchange listener is this test's.
    let mut app = app_at(
        &format!(
            "{}<script>\
               window.addEventListener('hashchange', function () {{\
                 history.pushState(null, '', '/after-hash');\
               }});\
             </script>",
            popstate_once("location.href = '#frag';")
        ),
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a
    let marker_before = document_marker(&app);

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert_eq!(
        document_marker(&app),
        marker_before,
        "a same-document navigate re-stamps nothing, so the swap marker is unchanged \
         — the loop's exit condition is a marker CHANGE, not the navigate itself"
    );
    assert!(
        !staged_session_history_work(&app),
        "so the loop ran the third iteration and consumed what the hashchange \
         handler staged. A break-on-navigate-attempt implementation leaves it here"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/after-hash"),
        "the whole chain settled in ONE drive: back() → popstate → fragment nav → \
         hashchange → pushState"
    );
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/#frag"),
        "the fragment nav pushed from base (index 0), truncating the forward /a"
    );
    assert_eq!(
        history_len(&app),
        3,
        "[base, base#frag, /after-hash] — the pushState appended behind the fragment \
         entry it was issued from"
    );
}
