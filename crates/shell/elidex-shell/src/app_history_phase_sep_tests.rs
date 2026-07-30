//! App-mode Slice-B **phase-separation conformance** — the turn-partition half of
//! the app-mode drain suite
//! (`docs/plans/2026-07-session-history-slice-B-app-phase-separation.md` §8).
//!
//! Split out of `app_history_drain_tests.rs` at that file's authored section
//! boundaries (touch-time 1000-line split), mirroring the carve the content leg
//! already made from `content_history_drain_tests` to
//! `content_history_phase_sep_tests`. The sibling `app_history_drain_tests` keeps
//! the core same-turn drain — Resolution-A supersede-and-discard plus cursor
//! atomicity / failed-load; this module keeps how a turn PARTITIONS: I1 ordering
//! inside the handler (axis a), the Resolution-E no-op peek-classify (axis e), the
//! Resolution-B default-suppression consumer through the real click path, and the
//! I3 liveness-inert bounded snapshot (axis b).
//!
//! **Both `#11-app-mode-turn-completion-drain` pins have MOVED OUT** — they flipped
//! when the drive-site quiescence loop landed and now assert turn-completion
//! conformance rather than the bounded-but-wrong status quo, so they live with that
//! family in `app_turn_completion_tests` (which carries the co-location rationale
//! and the content-side cross-check note forward). What stays here is the
//! *traversal*-granularity divergence pin
//! (`app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`),
//! which belongs to a different slot — `#11-sync-navigation-steps-queue-tagging` —
//! and did NOT flip.
//!
//! The axis-c pin is that both shells drive the identical shared
//! `DrainCoordinator`, so everything the coordinator OWNS lands the same way on
//! both, test-enforced across the two entry points rather than merely asserted.
//!
//! **The two tables are not identical, because the two schedules are not.**
//! Content-mode drives the coordinator from FIVE sites: three on its async pump
//! (`content/event_loop.rs` — `run_deferred_traversals` → `drain_synchronous_updates`
//! → the bottom `drain_synchronous_phase`) and two INSIDE its input handlers
//! (`content/event_handlers.rs`, one per handler). That in-handler pair is the
//! structural counterpart of app-mode's single end-of-input-handler
//! `drain_same_turn` — the click one consumes `suppress_default` as an early return
//! exactly as `app/events.rs::handle_click` does — so what app-mode lacks is the
//! PUMP, not the in-handler drain. The headline difference is
//! Phase-2 pump timing — content on a later async-pump turn via
//! `run_deferred_traversals`, app-mode back-to-back inside the input handler as its
//! *degenerate* later task. Content's post-Phase-2 settle (its R9 pin,
//! `content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn`)
//! DOES have an app-mode twin now — the drive-site quiescence loop, pinned in
//! `app_turn_completion_tests`.
//!
//! The `App`-building seeds and history/URL probes — and the **harness
//! reachability** contract every assertion here rests on — live in
//! `app_test_support`; the partition-specific probes are below.

use elidex_navigation::DrainHost;

use super::test_support::{
    app_at, base, current_url, cursor_over_content, entry_url, eval, history_len, pipeline_url,
    seed_same_document_pair, stamped,
};
use super::App;

/// Whether a `popstate` listener ran — the direct probe for "the SAME-DOCUMENT
/// traversal arm was taken" (§7.4.6.2 step 6.4.3 fires popstate in place; the
/// cross-document rebuild arm does not).
fn popstate_fired(app: &App) -> bool {
    stamped(app, "p", "data-popstate")
}

// ---------------------------------------------------------------------------
// Phase-sep ordering (axis a / I1 app-leg)
// ---------------------------------------------------------------------------

/// I1 (app leg — the ordering realized by `drain_same_turn`'s sequential body):
/// `pushState('/a'); history.back()` in ONE input handler commits the pushState `/a`
/// entry in **Phase 1** (in-task), and only THEN applies the traversal in **Phase 2**
/// against the UPDATED entry list. WHATWG HTML §7.4.6.1 *Updating the traversable*
/// step 12: "This set of steps are split into two parts to allow synchronous
/// navigations to be processed before documents unload."
///
/// App-mode has no async pump, so both phases run inside the one handler — but
/// strictly in that order, which is exactly what the single-traversable case needs.
#[test]
fn app_phase_sep_pushstate_then_back_orders_within_the_handler() {
    let mut app = app_at("<p>doc</p>", base());
    eval(
        &mut app,
        "history.pushState(null, '', '/a'); history.back();",
    );

    let outcome = app.process_pending_navigation();

    assert_eq!(
        history_len(&app),
        2,
        "the Phase-1 pushState committed /a BEFORE the traversal applied (it is not truncated \
         by the traversal — the retired supersede-return would have to run first to drop it)"
    );
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/a"),
        "entry 1 is the Phase-1 pushState /a"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/"),
        "the deferred back() then applied against the Phase-1-updated list → base (I1)"
    );
    assert!(
        app.traversal_queue().is_empty(),
        "Phase 2 ran inside the same handler — nothing left queued (app-mode has no later pump)"
    );
    assert!(
        outcome.own_context_action && outcome.shipped,
        "the same-document traversal apply shipped this turn"
    );
}

/// Resolution D (inherited from the shared coordinator): the REVERSE order
/// `history.back(); pushState('/x')` defers the trailing synchronous update behind
/// the barrier traversal (I2 — never reorder a sync ahead of a traversal issued
/// before it), and Phase 2 **CANCELS** it once the traversal MOVES THE CURSOR.
/// Applying it against the post-traversal cursor would land `/x` on the traversal
/// target and corrupt the current entry; dropping it preserves coherent state. The
/// §7.4.1.3 *Centralized modifications of session history* jump-the-queue
/// application to the call-time entry is fenced
/// (`#11-sync-navigation-steps-queue-tagging`).
#[test]
fn app_trailing_syncupdate_canceled_behind_cursor_moving_traversal() {
    let mut app = app_at("<p>doc</p>", base());
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(
        &mut app,
        "history.back(); history.pushState(null, '', '/x');",
    );
    let _ = app.process_pending_navigation();

    // `history_len` cannot say this: an APPLIED straddle would `push_same_document`
    // from the post-traversal cursor (index 0), truncating the forward `/a` and
    // appending `/x` — still 2 entries. The entry list is what tells them apart.
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/a"),
        "the straddle pushState /x was CANCELED — Resolution D. Had it applied it would \
         have truncated the forward /a and appended /x in its place, at the same length"
    );
    assert_eq!(history_len(&app), 2, "no third entry either");
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/"),
        "the cursor is the back target (base), NOT a /x landed on top of it"
    );
    assert!(app.traversal_queue().is_empty(), "the queue drained");
}

/// A `go(0)` **reload**. The spec never routes it through a traversal at all:
/// `go(delta)`'s method steps are "delta traverse this given delta", and *delta
/// traverse* (WHATWG HTML §7.2.5 *The History interface*) **step 4** short-circuits
/// — "If delta is 0, then reload document's node navigable, and return" — taking
/// the §7.4.3 *Reloading and traversing* reload path and never entering *traverse
/// the history by a delta*.
///
/// elidex reaches the same OUTCOME (a reload) by a different route, which is what
/// this test pins: `peek_delta(Go(0))` resolves to the current entry, so the step is
/// IN-RANGE and enqueues as a partition barrier, and `resolve_traversal` returns
/// **`Rebuild`** — not same-document — whenever the target IS the current index. So
/// Phase 2 attempts a document rebuild, i.e. the reload. That rebuild FAILS over the
/// disconnected harness, leaving the cursor and entry list untouched and reporting no
/// own-context action.
///
/// **Which arm ran is the whole point, so it is asserted directly.** `history_len`
/// and `current_url` are *invariants of both arms* — a same-document `go(0)` commits
/// the index it already has and rewrites `pipeline.url` to the URL it already has —
/// so neither can tell the arms apart. Two facts can: the same-document arm fires
/// **popstate** (§7.4.6.2 step 6.3) and **ships**, the rebuild arm does neither.
#[test]
fn app_go_zero_is_an_in_range_barrier_that_rebuilds() {
    let mut app = app_at(
        "<p>doc</p>\
         <script>window.addEventListener('popstate', function () {\
           document.querySelector('p').setAttribute('data-popstate', '1');\
         });</script>",
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.go(0);");
    let outcome = app.process_pending_navigation();

    assert!(
        outcome.suppress_default,
        "an in-range go(0) is a barrier → the caller's default is suppressed"
    );
    // DISCRIMINATOR (1): only the same-document arm fires popstate.
    assert!(
        !popstate_fired(&app),
        "go(0) took the REBUILD arm — no popstate. A same-document apply would have \
         restored state and fired popstate in place"
    );
    // DISCRIMINATOR (2): only the same-document arm ships (the rebuild's load fails).
    assert!(
        !outcome.shipped,
        "the rebuild's load fails in the disconnected harness → nothing shipped"
    );
    // Invariants of BOTH arms — they pin the reload's no-op-ness, not the arm.
    assert_eq!(
        history_len(&app),
        2,
        "a reload neither pushes nor drops an entry"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a"),
        "a reload does not move the cursor"
    );
    assert!(app.traversal_queue().is_empty(), "the queue drained");
}

// ---------------------------------------------------------------------------
// No-op peek-classify (Resolution E, axis e)
// ---------------------------------------------------------------------------

/// Resolution E: an out-of-range `history.go(999)` peek-classifies as a **no-op**
/// (WHATWG HTML §7.4.3 *Reloading and traversing* step 4.4 — "If
/// `allSteps[targetStepIndex]` does not exist, then abort these steps"), so it is NOT
/// a partition barrier: it enqueues no `Traversal` step and the trailing
/// `pushState('/x')` applies IN-TASK rather than deferring behind it.
///
/// **The two legs are not interchangeable — leg (1) is the whole discriminator.**
/// A no-op traversal mutates nothing, so promoting it to a barrier does not change
/// leg (2)'s *end state* at all: the trailing update merely rides the queue as a
/// `SyncUpdate` and is applied by Phase 2's `traversal_applied == false` arm instead
/// of by Phase 1b, landing on the same entry with the same cursor. (Mutation-checked:
/// forcing `classify_traversal` to classify every delta in-range leaves leg (2)
/// byte-identically green.) What a wrong barrier DOES change is
/// [`DrainOutcome::suppress_default`](elidex_navigation::DrainOutcome::suppress_default)
/// — latched at Phase-1 exit for a pending `Traversal` step — and that is observable
/// only on a turn with no other own-context effect, which is why leg (1) drains the
/// bare `go(999)` on its own.
#[test]
fn app_noop_traversal_does_not_defer_trailing_sync_update() {
    let mut app = app_at("<p>doc</p>", base()); // [base] only → go(999) is out of range

    // (1) NOT A BARRIER. A bare no-op turn must leave every outcome field clear: no
    //     `Traversal` step was enqueued, so nothing latches `suppress_default`.
    eval(&mut app, "history.go(999);");
    let noop_only = app.process_pending_navigation();

    assert!(
        !noop_only.suppress_default,
        "the out-of-range go(999) enqueued no Traversal step, so nothing latched \
         suppress_default — an in-range classification would have"
    );
    assert!(
        !noop_only.own_context_action,
        "a no-op traversal applies nothing"
    );
    assert!(
        app.traversal_queue().is_empty(),
        "the no-op left no step on the queue"
    );
    assert_eq!(history_len(&app), 1, "the entry list is untouched");

    // (2) THEREFORE the trailing synchronous update is not deferred behind it — it
    //     applies in Phase 1, in-task.
    eval(
        &mut app,
        "history.go(999); history.pushState(null, '', '/x');",
    );
    let _ = app.process_pending_navigation();

    assert!(
        app.traversal_queue().is_empty(),
        "nothing was deferred by the no-op"
    );
    assert_eq!(
        history_len(&app),
        2,
        "the trailing pushState /x applied (the no-op did not defer it)"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/x"),
        "the pushState landed on /x"
    );
}

// ---------------------------------------------------------------------------
// Default-suppression consumer (Resolution B, app-mode form) — the `handle_click`
// refinement, end to end through the real click path.
// ---------------------------------------------------------------------------

/// Markup whose `<a href="#frag">` fills the top-left of the content area and whose
/// click listener runs `script` — the harness for the two default-suppression legs.
/// The fragment href keeps the DEFAULT navigation observable in the disconnected
/// harness (a same-document fragment nav needs no fetch).
///
/// The handler also stamps `data-ran` so each test can assert the listener actually
/// fired: without that control the "the default DID fire" leg would pass even if the
/// handler never ran (a silently-dead listener looks the same as a no-op traversal).
fn link_with_click_handler(script: &str) -> String {
    format!(
        "<a href=\"#frag\" style=\"display:block;width:200px;height:100px\">link</a>\
         <div id=\"frag\" style=\"height:2000px\"></div>\
         <script>\
           document.querySelector('a').addEventListener('click', function () {{\
             document.querySelector('a').setAttribute('data-ran', '1'); {script}\
           }});\
         </script>"
    )
}

/// Whether the `link_with_click_handler` listener ran (its `data-ran` stamp reached
/// the DOM).
fn click_handler_ran(app: &App) -> bool {
    stamped(app, "a", "data-ran")
}

/// Resolution B (app-mode form): a click whose handler runs a VALID `history.back()`
/// makes the coordinator's single `suppress_default` field `true`, and
/// `handle_click` consumes that field to drop the `<a href>` default navigation.
///
/// The app-mode subtlety this pins: `drain_same_turn` has ALSO already applied the
/// traversal by the time the click path reads the outcome, yet the SAME field is
/// correct — it is computed at the END of Phase 1, while the in-range traversal was
/// still enqueued-but-unapplied, exactly as in content mode (one field, one consumer
/// rule, both shells).
#[test]
fn app_click_default_suppressed_by_valid_back() {
    let mut app = app_at(&link_with_click_handler("history.back();"), base());
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a → back() is in-range
    cursor_over_content(&mut app, 50.0, 50.0);

    app.handle_click(winit::event::MouseButton::Left);

    assert!(click_handler_ran(&app), "the click listener fired");
    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/"),
        "the handler's back() applied and the <a href=\"#frag\"> default was SUPPRESSED. \
         The counterfactual is https://example.com/#frag, NOT .../a#frag: Phase 2 has \
         already moved pipeline.url to base by the time the default would resolve #frag"
    );
    // `history_len` cannot say this: the counterfactual default resolves `#frag`
    // against the post-traversal base and `push_same_document`s from index 0,
    // truncating the forward `/a` — [base, /#frag], still 2 entries.
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/a"),
        "the suppressed default left the entry list intact — an unsuppressed one would \
         have replaced the forward /a with /#frag at the same length"
    );
    assert_eq!(history_len(&app), 2, "and appended nothing");
}

/// Resolution E at the consumer: a click whose handler runs a NO-OP
/// `history.go(999)` leaves `suppress_default` `false` (no `Traversal` step was
/// enqueued and no own-context effect happened), so the `<a href>` default DOES
/// fire — the no-op never over-suppresses a legitimate link default.
#[test]
fn app_click_default_fires_after_noop_traversal() {
    let mut app = app_at(&link_with_click_handler("history.go(999);"), base());
    // [base] only → go(999) is out of range.
    cursor_over_content(&mut app, 50.0, 50.0);

    app.handle_click(winit::event::MouseButton::Left);

    assert!(
        click_handler_ran(&app),
        "the click listener fired — so the go(999) really was staged and classified, not skipped"
    );
    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/#frag"),
        "the no-op go(999) suppressed nothing → the <a href=\"#frag\"> default navigated"
    );
    assert_eq!(
        history_len(&app),
        2,
        "the fragment default pushed its same-document entry"
    );
}

// ---------------------------------------------------------------------------
// Liveness-inert (axis b / I3 resolution (b)) — the reentrancy vector is dead by
// construction, so the bounded snapshot drains the WHOLE turn's queue.
// ---------------------------------------------------------------------------

/// I3 (b): app-mode's Phase-2 apply does not re-enqueue, so the bounded snapshot
/// `drain_same_turn` captures at Phase-2 drain-start equals the entire queue and the
/// drain leaves **no residual**. This matters more in app-mode than in content mode:
/// content leans on its every-turn async pump for liveness (a step serialized
/// mid-apply drains next turn), while app-mode has no pump — a stranded step would
/// wait for the next input event **that reaches the drive site at all**, and the
/// early returns in `events::handle_click` / `events::handle_keyboard` mean that is
/// unbounded, not next-input-bounded. Two traversals queued by one turn both apply
/// in that turn.
///
/// `back(); forward()` also pins the no-peek enqueue of the SECOND traversal: the
/// `forward()` is peeked nowhere at enqueue time (a peek against the still-unmoved
/// index-1 cursor would resolve out of range and DROP it), so Phase 2 applies both —
/// `back()` → base, then `forward()` → `/a`, netting back onto `/a`.
#[test]
fn app_drain_same_turn_leaves_no_residual_and_applies_every_queued_traversal() {
    let mut app = app_at("<p>doc</p>", base());
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.back(); history.forward();");
    let _ = app.process_pending_navigation();

    assert!(
        app.traversal_queue().is_empty(),
        "the bounded snapshot drained the whole turn's queue — no residual to strand \
         (app-mode has no pump to catch one)"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a"),
        "back() then forward() both applied in the one turn, netting onto /a \
         (a dropped forward would have left the cursor on base)"
    );
}

/// **Pins a WRONG-ENTRY divergence, not correct behavior** — slot
/// `#11-sync-navigation-steps-queue-tagging` (its R16 *multi-traversal snapshot*
/// facet). Codex `/external-converge` R7 on PR #487.
///
/// A **single** Phase-2 bounded snapshot applies EVERY queued traversal
/// back-to-back, and a §7.4.4 intent staged by a `popstate` handler that an
/// intermediate apply fired is NOT consumed between them: it lands on the VM's
/// `pending_history` (out-of-band — Phase 1b has already run), so the traversals
/// that follow keep moving the cursor underneath it, and the NEXT drain's Phase 1b
/// applies it against wherever the cursor finally stopped.
///
/// From `[base, /a]` on `/a` with a one-shot `popstate` listener calling
/// `replaceState('/from-popstate')`, `history.back(); history.forward()`:
/// `back()` applies → cursor to `base` → `popstate` fires → the handler stages the
/// replace **while `base` is current** → `forward()` applies → cursor back to `/a`
/// → the replace settles against **`/a`**. WHATWG HTML §7.4.6.1 *Updating the
/// traversable* step 14.1.1's note is explicit that synchronous navigations *"jump
/// the queue … before this traversal potentially unloads their document"*, so the
/// spec applies the replace to the entry whose handler issued it — **`base`, entry
/// 0**. elidex destroys `/a` instead and leaves `base` untouched: the exact
/// inversion.
///
/// **`#11-app-mode-turn-completion-drain` narrowed the timing, not the divergence.**
/// Before the drive-site quiescence loop the staged replace also had to wait for a
/// LATER DRIVE; now it settles in a later ITERATION of the same drive, so the
/// assertions below need only one `process_pending_navigation`. That is precisely
/// the turn-granularity / traversal-granularity split: the turn-completion loop
/// closes the gap *between drives*, this slot owns the gap *between two traversals
/// of one Phase-2 snapshot*. The wrong-entry outcome is unchanged, which is why the
/// pin did not move to `app_turn_completion_tests` with the two that flipped.
///
/// **NEWLY REACHABLE in app-mode because of Slice B, and that is deliberate.**
/// `origin/main`'s hand-rolled `app/navigation.rs::process_pending_navigation`
/// `return`ed as soon as one traversal handled the turn (`:73`), so the SECOND
/// traversal was silently dropped — the #259 multi-action truncation this slice
/// exists to fix. Unlocking it necessarily exposes the straddle underneath; the
/// slice trades a *lost* traversal for a *pinned* wrong-entry write, and does not
/// pretend the residual is absent.
///
/// **Why the fix is not here**: consuming sync-nav steps BETWEEN queued traversals
/// is precisely the tagged-queue work — per-task finalization with call-time entry
/// association (§7.4.1.3) — which is edge-dense (I1 × I2 × the bounded snapshot ×
/// Resolution D's cancel latch) and carries a mandatory `/elidex-plan-review` in
/// its own PR. This test flips to asserting `base` when that lands.
#[test]
fn app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry() {
    let mut app = app_at(
        "<p>doc</p>\
         <script>window.addEventListener('popstate', function once() {\
           window.removeEventListener('popstate', once);\
           history.replaceState(null, '', '/from-popstate');\
         });</script>",
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.back(); history.forward();");
    let _ = app.process_pending_navigation();

    assert!(
        app.traversal_queue().is_empty(),
        "both queued traversals drained in the one snapshot (the Slice-B unlock)"
    );
    assert!(
        !super::test_support::staged_session_history_work(&app),
        "the ONE drive ran the turn to quiescence — the staged replace was consumed \
         by a later ITERATION, so the divergence below is no longer entangled with \
         cross-drive latency (`#11-app-mode-turn-completion-drain`)"
    );
    assert_eq!(
        app.interactive
            .as_ref()
            .unwrap()
            .nav_controller
            .current_index(),
        1,
        "back() then forward() both applied, netting the CURSOR back onto entry 1 — \
         the entry the replace then rewrote (so `current_url` reads /from-popstate, \
         not /a: the cursor did not move, the entry under it changed)"
    );
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/from-popstate"),
        "DIVERGENCE (pinned): the replace lands on entry 1 — /a, the entry the FORWARD \
         traversal moved to — destroying it"
    );
    assert_eq!(
        entry_url(&app, 0).as_deref(),
        Some("https://example.com/"),
        "and `base` — the entry that was current when the handler ran, i.e. the one \
         §7.4.6.1 step 14.1.1's note says should have been replaced — is left untouched"
    );
}
