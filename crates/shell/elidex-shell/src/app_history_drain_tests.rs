//! App-mode (legacy inline) history/navigation drain — Slice B phase-separation
//! conformance
//! (`docs/plans/2026-07-session-history-slice-B-app-phase-separation.md` §8).
//!
//! The app-mode leg of the scenario table `content_history_drain_tests` /
//! `content_history_phase_sep_tests` run against the content shell. The axis-c pin
//! is that both shells drive the identical shared `DrainCoordinator`, so everything
//! the coordinator OWNS — the I2 partition, the Resolution A supersede, the
//! Resolution D `SyncUpdate` cancel, the Resolution E no-op peek-classify — lands
//! the same way on both, test-enforced across the two entry points rather than
//! merely asserted.
//!
//! **The two tables are not identical, because the two schedules are not.**
//! Content-mode makes THREE coordinator calls per event-loop turn
//! (`content/event_loop.rs:205` / `:206` / `:497`); app-mode makes ONE
//! (`drain_same_turn`, at end-of-input-handler). The headline difference is
//! Phase-2 pump timing — content on a later async-pump turn via
//! `run_deferred_traversals`, app-mode back-to-back inside the input handler as its
//! *degenerate* later task — but app-mode also has **no post-Phase-2 synchronous
//! settle**, so content's R9 pin
//! (`content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn`)
//! has no app-mode twin: its counterpart below
//! (`app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue`)
//! pins the opposite, bounded behavior, fenced to
//! `#11-app-mode-turn-completion-drain`.
//!
//! **Harness reachability.** These tests build an `App` via
//! [`App::new_interactive_with_url`] (no winit — `render_state` is `None`) over a
//! **disconnected** network, so a *successful* cross-document rebuild is not
//! reachable: `load_document` always fails, leaving the pipeline + cursor unchanged.
//! Traversals between entries that share a `document_sequence` (seeded with
//! `push_same_document`, or created by `pushState`) take the no-fetch
//! `same_document_step` path, so their Phase-2 apply SUCCEEDS here — that is the
//! path every "the traversal landed" assertion below uses. Because `render_state` is
//! `None` the frame-ship is unobservable as an OS repaint, so ship-once is asserted
//! on the coordinator's own bookkeeping ([`DrainOutcome::shipped`] /
//! [`DrainOutcome::own_context_action`]), which is what gates
//! `DrainHost::ship_frame`.
//!
//! The `App`-building + history/URL probes live in `app_test_support` (shared with
//! `app_fragment_nav_tests`); the drain-specific seeds and probes are below.

use elidex_navigation::{DrainHost, TraversalDelta};
use elidex_script_session::HostDriver;

use super::drain_host::apply_traversal_delta;
use super::test_support::{app_at, base, history_len, pipeline_url, url};
use super::App;

/// Seed `[base, /a]` sharing ONE `document_sequence`, cursor on `/a` — the
/// same-document pair whose `back()` applies in the disconnected harness (no fetch).
/// The app-mode mirror of `content_test_support::seed_same_document_pair`.
fn seed_same_document_pair(app: &mut App) {
    let a = url("https://example.com/a");
    // index 0 = base was seeded by `new_interactive_with_url`; index 1 = /a inherits
    // its `document_sequence`, so a traversal between them is SameDocument.
    app.interactive
        .as_mut()
        .unwrap()
        .nav_controller
        .push_same_document(a.clone());
    activate_seeded_entry(app, a);
}

/// Seed `[base, /a]` as two DISTINCT documents (fresh `document_sequence`s), cursor
/// on `/a` — a `back()` here classifies `Rebuild`, and its cross-document load FAILS
/// in the disconnected harness (the failed-load / cursor-atomicity path).
///
/// The ONLY difference from [`seed_same_document_pair`] is `push` vs
/// `push_same_document`; everything downstream of the cursor move is the shared
/// [`activate_seeded_entry`] tail.
fn seed_cross_document_pair(app: &mut App) {
    let a = url("https://example.com/a");
    app.interactive
        .as_mut()
        .unwrap()
        .nav_controller
        .push(a.clone());
    activate_seeded_entry(app, a);
}

/// The shared tail of the two seeds: point the ACTIVE-DOCUMENT facts at the entry
/// the cursor was just moved onto — pipeline URL + VM current-URL + VM
/// session-history `(index, length)`.
///
/// Mirrors what production does on every cursor move (`navigate` /
/// `same_document_step` / `navigate_to_history_url` / `apply_state_change` all
/// end in `set_session_history`), so neither seed leaves the
/// production-impossible state where `history.length` still describes the
/// pre-push list while the cursor sits on `/a`. Factored out because the
/// cross-document seed silently OMITTED the `set_session_history` call — the exact
/// bug shape Codex fixed in `content_test_support::seed_same_document_pair` on
/// PR #469 R18, where a controller-only seed let spurious passes through.
fn activate_seeded_entry(app: &mut App, url: url::Url) {
    let interactive = app.interactive.as_mut().unwrap();
    interactive.pipeline.url = Some(url.clone());
    interactive.pipeline.runtime.set_current_url(Some(url));
    interactive.pipeline.runtime.set_session_history(
        interactive.nav_controller.current_index(),
        interactive.nav_controller.len(),
    );
}

fn eval(app: &mut App, script: &str) {
    let _ = app
        .interactive
        .as_mut()
        .unwrap()
        .pipeline
        .runtime
        .vm()
        .eval(script);
}

/// The history CURSOR's entry URL — the drain-specific complement of
/// `test_support::pipeline_url` (the active document's URL): a traversal that moved
/// the cursor but whose document load failed leaves the two disagreeing.
fn current_url(app: &App) -> Option<String> {
    app.interactive
        .as_ref()
        .unwrap()
        .nav_controller
        .current_url()
        .map(|u| u.as_str().to_string())
}

/// Place the inline cursor over content-area point `(x, y)` (winit client coords
/// are chrome-inclusive, so the chrome bar height is added back).
fn cursor_over_content(app: &mut App, x: f64, y: f64) {
    app.interactive.as_mut().unwrap().cursor_pos = Some(elidex_plugin::Point::new(
        x,
        y + f64::from(crate::chrome::CHROME_HEIGHT),
    ));
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
        app.interactive
            .as_ref()
            .unwrap()
            .nav_controller
            .entry(1)
            .map(|e| e.url.as_str().to_string()),
        Some("https://example.com/a".to_string()),
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

    assert_eq!(
        history_len(&app),
        2,
        "the straddle pushState /x was CANCELED (no third entry) — Resolution D"
    );
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
/// own-context action. Had it wrongly taken the same-document path it would have
/// applied in place and shipped.
#[test]
fn app_go_zero_is_an_in_range_barrier_that_rebuilds() {
    let mut app = app_at("<p>doc</p>", base());
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.go(0);");
    let outcome = app.process_pending_navigation();

    assert!(
        outcome.suppress_default,
        "an in-range go(0) is a barrier → the caller's default is suppressed"
    );
    assert!(
        !outcome.shipped,
        "go(0) took the REBUILD arm, whose load fails in the harness → nothing shipped \
         (a same-document apply would have shipped)"
    );
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
// Nav-vs-traversal supersede (Resolution A) + the removal of the retired
// hand-rolled `process_pending_navigation` traversal-supersede `return`
// ---------------------------------------------------------------------------

/// Resolution A: a same-turn `history.back(); location.assign(...)` lands on the
/// **back target**, the navigation drain-and-DISCARDED — WHATWG HTML §7.4.2.2
/// *Beginning navigation* step 19 ("Any attempts to navigate a navigable that is
/// currently traversing are ignored"). The reverse cross-channel order lands the
/// same way: the VM stages traversals and `location.*` on two separate channels, so
/// their relative issue order is not observable to the drain, and the spec lands on
/// the traversal target in both orders anyway (step 20's "aborting other ongoing
/// navigations" aborts *navigations*, not the traversal).
///
/// **FLIP of the retired hand-rolled
/// `app/navigation.rs::process_pending_navigation` traversal-supersede `return`.**
/// That drain applied the traversal INSIDE the history-FIFO loop and returned
/// immediately, so `take_pending_navigation()` never ran and the `location.*` request
/// stayed **stranded** on the runtime — re-firing a turn late (a spurious deferred
/// navigation). Phase-1c now drains the slot and drops the request, so a second drain
/// finds nothing: the fragment target is never navigated to, on this turn or any
/// later one.
#[test]
fn app_nav_vs_traversal_supersede_discards_and_does_not_strand() {
    for script in [
        "history.back(); location.assign('#b');",
        // Reverse cross-channel order — same landing.
        "location.assign('#b'); history.back();",
    ] {
        let mut app = app_at("<p>doc</p>", base());
        seed_same_document_pair(&mut app); // [base, /a], cursor on /a

        eval(&mut app, script);
        let outcome = app.process_pending_navigation();

        assert!(
            outcome.suppress_default,
            "{script}: the in-range back() supersedes → the caller's default is suppressed"
        );
        assert_eq!(
            current_url(&app).as_deref(),
            Some("https://example.com/"),
            "{script}: landed on the back target (base), NOT the #b fragment"
        );
        assert_eq!(
            history_len(&app),
            2,
            "{script}: no #b entry — the navigation was discarded, not applied"
        );

        // The discarded slot was DRAINED, not skipped: a later turn must not
        // resurrect it (the retired supersede `return` left it staged to fire late).
        let later = app.process_pending_navigation();
        assert!(
            !later.own_context_action,
            "{script}: nothing left staged — the suppressed navigation cannot re-fire a turn late"
        );
        assert_eq!(
            history_len(&app),
            2,
            "{script}: still no #b entry on the following turn"
        );
        assert_eq!(
            pipeline_url(&app).as_deref(),
            Some("https://example.com/"),
            "{script}: the document URL is the back target — #b was never navigated to"
        );
    }
}

/// #259 (app leg) — the retired hand-rolled `process_pending_navigation`
/// traversal-supersede `return` no longer truncates the FIFO replay:
/// `pushState; pushState; back()` keeps BOTH pushes (Phase 1 applies every
/// synchronous update issued before the barrier, in issue order) and the traversal
/// applies afterwards in Phase 2.
#[test]
fn app_multiple_pushstates_survive_a_trailing_traversal() {
    let mut app = app_at("<p>doc</p>", base());
    eval(
        &mut app,
        "history.pushState(null, '', '/a'); history.pushState(null, '', '/b'); history.back();",
    );

    let _ = app.process_pending_navigation();

    assert_eq!(
        history_len(&app),
        3,
        "both pushStates committed in Phase 1 → [base, /a, /b]"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a"),
        "the Phase-2 back() moved /b → /a against the fully-replayed list"
    );
}

// ---------------------------------------------------------------------------
// No-op peek-classify (Resolution E, axis e)
// ---------------------------------------------------------------------------

/// Resolution E: an out-of-range `history.go(999)` peek-classifies as a **no-op**
/// (WHATWG HTML §7.4.3 *Reloading and traversing* step 4.4 — "If
/// `allSteps[targetStepIndex]` does not exist, then abort these steps"), so it is NOT
/// a partition barrier: it enqueues no `Traversal` step and the trailing
/// `pushState('/x')` applies IN-TASK rather than deferring behind it.
#[test]
fn app_noop_traversal_does_not_defer_trailing_sync_update() {
    let mut app = app_at("<p>doc</p>", base()); // [base] only → go(999) is out of range

    eval(
        &mut app,
        "history.go(999); history.pushState(null, '', '/x');",
    );
    let _ = app.process_pending_navigation();

    assert!(
        !app.traversal_queue().has_pending_traversal(),
        "the no-op go(999) enqueued no Traversal step (not a barrier)"
    );
    assert!(
        app.traversal_queue().is_empty(),
        "nothing was deferred by the no-op"
    );
    assert_eq!(
        history_len(&app),
        2,
        "the trailing pushState /x applied IN-TASK (the no-op did not defer it)"
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
/// the DOM). The click path's `re_render` flushes the script session, so the
/// attribute is committed by assertion time.
fn click_handler_ran(app: &App) -> bool {
    let pipeline = &app.interactive.as_ref().unwrap().pipeline;
    pipeline.dom.query_by_tag("a").into_iter().any(|e| {
        pipeline
            .dom
            .world()
            .get::<&elidex_ecs::Attributes>(e)
            .is_ok_and(|a| a.get("data-ran") == Some("1"))
    })
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
        "the handler's back() applied and the <a href=\"#frag\"> default was SUPPRESSED \
         (an unsuppressed default would have landed .../a#frag)"
    );
    assert_eq!(
        history_len(&app),
        2,
        "the suppressed default pushed no fragment entry"
    );
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
/// wait for the next input event. Two traversals queued by one turn both apply in
/// that turn.
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

/// **Pins app-mode's CURRENT BOUNDED behavior, not a correct-by-design one** — slot
/// `#11-app-mode-turn-completion-drain`.
///
/// Two facts are entangled here and only the first is by design. **(1)** The root
/// invariant behind the by-construction proof (plan §4.4 premise 5): no app-mode
/// apply body synchronously drives Phase 1, so a history action STAGED during a
/// Phase-2 apply — here a `pushState` from the synchronously-fired `popstate`
/// handler of a same-document traversal — is NOT partitioned into the CURRENT
/// drain's queue, which is exactly what makes the bounded snapshot complete.
/// **(2)** But nothing settles it either: `drain_same_turn` has no post-Phase-2
/// synchronous drain, so the staged `pushState` sits on the VM FIFO until the NEXT
/// input event. That is the shape Slice A describes as "firing much too late" and
/// FIXED for content mode (Codex #469 R9) by running
/// `DrainCoordinator::drain_synchronous_updates` immediately after
/// `run_deferred_traversals` — pinned by the content counterpart
/// `content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn`,
/// which asserts the popstate-staged `pushState` lands on the SAME turn. App-mode
/// has no such counterpart, so the assertions below describe the status quo.
///
/// The fix is **loop-until-quiescent turn completion**, NOT a trailing
/// `drain_synchronous_updates` — that trailing drain is not merely insufficient, it
/// is **wrong**. It would settle a popstate-staged `pushState`, but a popstate-staged
/// `back()` would be peek-classified (Resolution E) and left **resident on the
/// `TraversalQueue` across the turn boundary**. Such a step is NOT stranded: the next
/// turn's `drain_same_turn` seeds `seen_traversal` from `has_pending_traversal()` and
/// its Phase 2 drains it, at exactly the latency it has today. What the trailing drain
/// does is **freeze the in-range classification a turn early**, voiding the queue's
/// own contract that Resolution E "leaves no `Traversal` step for a no-op, so it never
/// over-suppresses": the **non-drain** cursor movers run between turns (chrome toolbar
/// Back/Forward and Alt+←/→ call `traverse_to` directly; an `<a href>` default calls
/// `navigate`), so the resident step can be a no-op by the next turn while still acting
/// as a FULL barrier — seeding `seen_traversal` at Phase-1 ENTRY (deferring every fresh
/// `pushState` behind it) and latching `suppress_default` true at Phase-1 EXIT, killing
/// an unrelated `<a href>` default for a traversal whose Phase-2 re-peek then finds it
/// out of range and no-ops. When the resident step IS still in range its apply ships,
/// so the Resolution-D `traversal_applied` latch CANCELS every `pushState` deferred
/// behind it — that specific cancel is *today's* behavior too (a parked `back()` leads
/// the same VM FIFO on the next turn, pinned by
/// [`app_trailing_syncupdate_canceled_behind_cursor_moving_traversal`]), so the
/// over-suppression above is what the trailing drain newly breaks. It would also
/// contradict `process_pending_navigation`'s premise-5 exit assert by construction
/// (the queue would be deliberately non-empty at drain exit). Edge-dense ⇒ its own
/// plan-reviewed PR, at which point this test flips to the content shape.
#[test]
fn app_popstate_staged_action_defers_to_the_next_drain_not_the_current_queue() {
    let mut app = app_at(
        "<p>doc</p>\
         <script>window.addEventListener('popstate', function () {\
           history.pushState(null, '', '/from-popstate');\
         });</script>",
        base(),
    );
    seed_same_document_pair(&mut app); // [base, /a], cursor on /a

    eval(&mut app, "history.back();");
    let _ = app.process_pending_navigation();

    assert!(
        app.traversal_queue().is_empty(),
        "the popstate handler's pushState did not re-enter this drain's partition"
    );
    assert_eq!(
        history_len(&app),
        2,
        "the popstate-staged pushState is NOT applied by the drain that fired popstate"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/"),
        "this turn ends on the back target"
    );

    // The next turn's drain partitions it — the degenerate later task.
    let _ = app.process_pending_navigation();
    assert_eq!(
        history_len(&app),
        2,
        "the popstate pushState truncated the forward entry and appended its own → still 2"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/from-popstate"),
        "the staged pushState applied on the NEXT drain"
    );
}

// ---------------------------------------------------------------------------
// Cursor atomicity (axis e) — peek-then-commit survives the restructure.
// ---------------------------------------------------------------------------

/// Traversal atomicity: `apply_traversal_delta` peeks its target WITHOUT moving the
/// cursor, and `traverse_to` commits (`commit_index`) only after a successful load.
/// The cross-document load fails in the disconnected harness, so the cursor never
/// moves and the apply reports `false` (no supersede) — no speculative move, no
/// rollback path.
#[test]
fn app_failed_traversal_load_leaves_cursor_unmoved() {
    let mut app = app_at("<p>doc</p>", base());
    seed_cross_document_pair(&mut app); // [base, /a] as two documents, cursor on /a

    let shipped = apply_traversal_delta(&mut app, TraversalDelta::Back);

    assert!(!shipped, "a failed-load traversal does not supersede");
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a"),
        "the cursor never moved — the still-active document is /a, not the unreached base"
    );
}

/// A no-op traversal (out of range) drives no apply at all and reports `false`, so
/// it neither supersedes nor over-suppresses.
#[test]
fn app_noop_traversal_reports_no_supersede() {
    let mut app = app_at("<p>doc</p>", base());

    assert!(
        !apply_traversal_delta(&mut app, TraversalDelta::Go(999)),
        "an out-of-range go is a no-op (no traversal body runs) → reports no supersede"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/"),
        "the cursor is untouched"
    );
}

/// The Resolution-D re-check gate (the complement of
/// `app_trailing_syncupdate_canceled_behind_cursor_moving_traversal`): a barrier
/// traversal whose load FAILED never moved the cursor, so the still-active document
/// IS the call-time entry and a trailing straddle `SyncUpdate` applies there
/// coherently rather than being canceled.
#[test]
fn app_failed_traversal_does_not_cancel_trailing_sync_update() {
    let mut app = app_at("<p>doc</p>", base());
    seed_cross_document_pair(&mut app); // [base, /a] as two documents → back() rebuilds + fails

    eval(
        &mut app,
        "history.back(); history.pushState(null, '', '/kept');",
    );
    let _ = app.process_pending_navigation();

    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/kept"),
        "the failed-load traversal left the cursor on /a, so the trailing pushState committed \
         from there (a cursor-MOVING traversal would have canceled it)"
    );
    assert_eq!(
        history_len(&app),
        3,
        "the pushState appended after /a → [base, /a, /kept]"
    );
}
