//! App-mode (legacy inline) history/navigation drain — Slice B phase-separation
//! conformance, **core same-turn drain** half
//! (`docs/plans/2026-07-session-history-slice-B-app-phase-separation.md` §8).
//!
//! The app-mode leg of the scenario table `content_history_drain_tests` /
//! `content_history_phase_sep_tests` run against the content shell, carved along
//! the same boundary the content leg uses (touch-time 1000-line split). This module
//! keeps the **core same-turn drain**: the Resolution-A nav-vs-traversal
//! supersede-and-discard (with the removal of the retired hand-rolled
//! `process_pending_navigation` supersede `return`), and cursor atomicity —
//! peek-then-commit, the failed-load paths, and the `apply_traversal_delta` outcome
//! contract. The sibling `app_history_phase_sep_tests` owns the rest of how a turn
//! partitions — I1 ordering, the Resolution-E peek-classify, Resolution-B
//! default-suppression, the I3 bounded snapshot.
//!
//! ⚠ **One partition pin stays here**, deliberately:
//! `app_full_fifo_survives_an_applied_traversal_mid_stream` pins the full
//! issue-ordered FIFO (two pushes committed in Phase 1, then two traversals both
//! applied in Phase 2) because it is the **#259 regression guard** for the retired
//! `process_pending_navigation` truncation, which is this half's charter. So a
//! change to the Phase-1/Phase-2 partition — the queue-tagging work that reorders
//! sync-nav settling between traversals, above all — must audit **both** files, not
//! just the phase-sep sibling.
//!
//! The axis-c pin is that both shells drive the identical shared
//! `DrainCoordinator`, so everything the coordinator OWNS lands the same way on
//! both, test-enforced across the two entry points rather than merely asserted.
//! What this half enforces of that is the Resolution-A supersede, the Resolution-D
//! re-check gate, the Resolution-E no-supersede outcome, and the full-FIFO issue
//! order above.
//!
//! The `App`-building seeds and history/URL probes — and the **harness
//! reachability** contract every assertion here rests on — live in
//! `app_test_support`.

use elidex_navigation::{DrainHost, TraversalDelta};

use super::drain_host::apply_traversal_delta;
use super::test_support::{
    app_at, base, current_url, entry_url, eval, history_len, pipeline_url,
    seed_cross_document_pair, seed_same_document_pair,
};

// ---------------------------------------------------------------------------
// Nav-vs-traversal supersede (Resolution A) + the removal of the retired
// hand-rolled `process_pending_navigation` traversal-supersede `return`
// ---------------------------------------------------------------------------

/// Resolution A: a same-turn `history.back(); location.assign(...)` lands on the
/// **back target**, the navigation drain-and-DISCARDED. The reverse cross-channel
/// order lands the same way: the VM stages traversals and `location.*` on two
/// separate channels, so their relative issue order is not observable to the drain,
/// and **the traversal wins in both orders under the spec too** (§7.4.2.2 step 20's
/// "aborting other ongoing navigations" aborts *navigations*, not the traversal).
///
/// ⚠ *Wins the landing*, but elidex still **loses an entry the spec keeps — in BOTH
/// orders.** (Corrected 2026-07-26; an earlier revision of this note claimed the
/// fragment entry is appended *synchronously* and that order 1 yields 2 entries. Both
/// were wrong, and the first contradicted the §7.4.1.3 quote on the
/// [`DrainHost::classify_traversal`] contract.)
///
/// `location.assign('#b')` is a **fragment** navigation: §7.4.2.2 *Beginning
/// navigation* **step 15** dispatches *navigate to a fragment* (§7.4.2.3.3 *Fragment
/// navigations*), whose **step 13** synchronously sets only the *active session
/// history entry*, while **step 17** *appends* the session history synchronous
/// navigation steps — so the entries-list append (*finalize a same-document
/// navigation* step 5.4) is **queued, not synchronous**, exactly as §7.4.1.3 says.
///
/// §7.4.1.3's worked example **is order 1** (`history.back(); location.href = '#foo'`
/// from step 1 of `[/a, /b]`). Its stated desired result **adds** the `/b#foo` entry
/// (step 2 = "the current session history step (i.e., 1) plus 1") *and* finishes
/// moving to step 0. So from `[base, /a]` on `/a` the spec gives **3 entries in both
/// orders**, differing only in the landing: order 1 (`back(); assign('#b')`) lands
/// `base`, order 2 (`assign('#b'); back()`) lands `/a`.
///
/// elidex produces **2 entries and a `base` landing in both** — so the divergence is
/// not merely the cross-channel *ordering* (order 2's landing) but a **dropped
/// fragment entry in order 1 as well**, where the ordering question does not even
/// arise. Root: Phase 1c drain-and-DISCARDs the navigation outright, so `#b` never
/// runs and never appends. That is upstream of the queue-tagging work — the VM
/// destroys the cross-channel order at *staging* (`vm/host/navigation.rs` single-slot
/// `pending_navigation` vs the `pending_history` FIFO), so recovering it additionally
/// requires reopening Q-VM-MODEL (Slice-A memo §2). Fenced with that prerequisite
/// named at `#11-nav-supersede-window-vs-ongoing-navigation`.
///
/// **The discard is a deliberate DIVERGENCE, not §7.4.2.2 step 19** (webref-verified
/// 2026-07-26; slot `#11-nav-supersede-window-vs-ongoing-navigation`). Step 19's
/// gate — *ongoing navigation* == "traversal" — is read when `navigate` runs and is
/// set only by the §7.4.6.1 step-8.4 APPLY, so a `location.*` issued while the
/// traversal is merely QUEUED never meets it. **A second, independent reason step 19
/// is the wrong citation for THIS pin**: `location.assign('#b')` is a fragment
/// navigation, and §7.4.2.2 **step 15** dispatches *Navigate to a fragment* and
/// **Returns** before step 19 is ever reached — so step 19 could not gate it even
/// with *ongoing navigation* == "traversal". elidex suppresses from enqueue time, a
/// strict superset of the spec's window (under today's synchronous, non-yielding
/// apply); this test pins that behavior (unchanged), not a step-19 derivation of it.
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
/// traversal-supersede `return` no longer **truncates the FIFO replay**: every step
/// the turn staged runs, whatever it is and however many traversals precede it.
///
/// **What actually discriminates the removal.** The obvious shape
/// (`pushState; pushState; back()`) does NOT: the retired drain applied traversals
/// INSIDE the FIFO loop, so its `return` fired on the LAST step and both pushes had
/// already committed — byte-identical to the phase-separated result. Nor does
/// `back(); pushState; pushState`: the retired `return` dropped both trailing pushes,
/// and the coordinator's Resolution-D cancel drops them too (the barrier moved the
/// cursor). The one class the retired `return` truly lost is a **step after the
/// first traversal that the new code still runs** — i.e. a SECOND traversal. So this
/// pins the full FIFO: two pushes before the barrier (Phase 1, in issue order) and
/// two traversals after it (Phase 2, both applied), where the retired drain stopped
/// dead at the first.
#[test]
fn app_full_fifo_survives_an_applied_traversal_mid_stream() {
    let mut app = app_at("<p>doc</p>", base());
    eval(
        &mut app,
        "history.pushState(null, '', '/a'); history.pushState(null, '', '/b'); \
         history.back(); history.back();",
    );

    let _ = app.process_pending_navigation();

    assert_eq!(
        history_len(&app),
        3,
        "both pushStates committed in Phase 1 → [base, /a, /b]"
    );
    assert_eq!(
        entry_url(&app, 2).as_deref(),
        Some("https://example.com/b"),
        "in issue order — /b is the second push, not the first"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/"),
        "BOTH deferred back()s applied against the fully-replayed list: /b → /a → base. \
         The retired supersede `return` fired on the first one and never reached the \
         second, leaving the cursor on /a"
    );
    assert!(app.traversal_queue().is_empty(), "the queue drained");
}

/// The **failed-load complement** of
/// [`app_nav_vs_traversal_supersede_discards_and_does_not_strand`], and the §7.4.2
/// twin of [`app_failed_traversal_does_not_cancel_trailing_sync_update`]: a barrier
/// traversal whose cross-document load FAILED never moved the cursor, so the
/// navigable never traversed and the same-turn `location.*` it superseded still
/// applies — in the turn that issued it.
///
/// Phase 1c suppresses on a merely-QUEUED traversal, which is a deliberate
/// **divergence** from WHATWG HTML §7.4.2.2 *Beginning navigation* step 19 rather
/// than an application of it (webref-verified 2026-07-26; slot
/// `#11-nav-supersede-window-vs-ongoing-navigation`): step 19's gate is *ongoing
/// navigation* == "traversal" (§7.4.2.5 *Aborting navigation*), which only §7.4.6.1
/// *Updating the traversable* step 8.4 sets, inside the APPLY — §7.4.3's enqueue sets
/// nothing. And independently of that, the `location.assign('#b')` this pin uses is a
/// **fragment** navigation, which §7.4.2.2 **step 15** dispatches and `Return`s from
/// **before step 19 is reached** — so step 19 never gated this scenario on either
/// count. App-mode's Phase 2 runs in the same turn, so
/// `App::process_pending_navigation` reinstates a suppression the turn refuted,
/// narrowing the superset back to "a traversal that really moved the cursor".
///
/// **Regression pin.** The retired hand-rolled drain kept this by falling through:
/// "a no-target / failed-load traversal returns `false` … so the loop CONTINUES and
/// trailing same-turn intents still apply (Codex R1 P2 / R2)". Slice B's first cut
/// kept that contract only for the deferred `SyncUpdate` leg — the §7.4.2 leg
/// drain-and-DISCARDED, stranding the user on the old document with the request gone.
#[test]
fn app_failed_traversal_reinstates_the_superseded_navigation() {
    let mut app = app_at("<p>doc</p>", base());
    seed_cross_document_pair(&mut app); // [base, /a] as two documents → back() rebuilds + fails

    eval(&mut app, "history.back(); location.assign('#b');");
    let outcome = app.process_pending_navigation();

    assert_eq!(
        pipeline_url(&app).as_deref(),
        Some("https://example.com/a#b"),
        "the back()'s cross-document load FAILED, so the navigable never traversed and the \
         superseded location.assign('#b') applied (a DISCARD leaves the document on /a)"
    );
    assert_eq!(
        current_url(&app).as_deref(),
        Some("https://example.com/a#b"),
        "the reinstated fragment navigation moved the cursor onto its own entry"
    );
    assert_eq!(
        entry_url(&app, 1).as_deref(),
        Some("https://example.com/a"),
        "the failed back() left the entry list intact; #b was pushed after /a"
    );
    assert_eq!(history_len(&app), 3, "[base, /a, /a#b]");
    assert!(
        outcome.own_context_action,
        "the reinstated navigation is an own-context effect"
    );

    // Reinstated IN-TURN, never a turn late: Phase 1c drained the VM slot and the
    // request was held on the host, so a later drain finds nothing to re-fire.
    let later = app.process_pending_navigation();
    assert!(
        !later.own_context_action,
        "nothing left staged — the reinstatement consumed the held request"
    );
    assert_eq!(
        history_len(&app),
        3,
        "no second #b entry on the following turn"
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
/// `app_history_phase_sep_tests::app_trailing_syncupdate_canceled_behind_cursor_moving_traversal`):
/// a barrier
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
