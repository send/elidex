//! Drain-semantics companion to [`super`]'s `traversal_queue_tests.rs` — split
//! at the drain/guard/cancel seam when the suite crossed the 1000-line touch-time
//! boundary (CLAUDE.md; PR469 Codex R7). Nested as a child `mod drain;` of the
//! `tests` module so it shares that file's `MockHost` harness + action builders
//! via `use super::*` (no duplicated helpers).
//!
//! Covers plan §4.5 I3 (guard bracket + bounded next-turn drain) and the Slice A
//! co-design seams A/B/D/E (SyncUpdate cancel/apply — same-document /
//! document-changing / failed-load-barrier — and default nav-suppression).

use super::*;

// --- I3: guard bracket + eventual drain -------------------------------------

#[test]
fn i3_guard_is_set_during_traversal_apply_only() {
    // The nested-apply boolean is TRUE inside `apply_traversal` and FALSE for a
    // (Phase-1) sync update — the bracket covers the traversal peek→commit only.
    let mut host = MockHost::new(vec![push("/a"), back()]);
    assert!(!host.queue.is_applying(), "initially false (§7.3.1.1)");
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    for ev in &host.log {
        match ev {
            Ev::TraversalApply { guard, .. } => assert!(*guard, "guard set during traversal apply"),
            Ev::SyncUpdate { guard, .. } => assert!(!*guard, "guard clear for a sync update"),
            _ => {}
        }
    }
    assert!(
        !host.queue.is_applying(),
        "guard cleared after the commit (bracket closed)"
    );
}

#[test]
fn i3_reentrant_message_deferred_to_next_turn_bounded_drain() {
    // T1 BOUNDED SNAPSHOT (Codex PR#469 R3): a reentrant traversal enqueued DURING
    // the first apply (the SW-pump vector) is NOT drained to exhaustion within this
    // pass — the drain processes only the snapshot pending at entry, so it
    // TERMINATES BY CONSTRUCTION. The re-enqueued Forward stays for the NEXT
    // `run_deferred_traversals` turn (content mode pumps Phase 2 every event-loop
    // turn, so liveness holds via the async pump). This removes the unbounded
    // re-check-until-empty loop that could hang the renderer thread.
    let reentrant = PendingTraversal {
        delta: TraversalDelta::Forward,
        user_involvement: UserInvolvement::default(),
    };
    let mut host = MockHost::new(vec![back()]).with_reentrant(reentrant);

    // Pass 1: only the snapshot (the Back) applies; the reentrant Forward, enqueued
    // mid-apply, is deferred — it remains on the queue, NOT drained this pass.
    let _ = DrainCoordinator::drain_same_turn(&mut host);
    let pass1: Vec<_> = host
        .log
        .iter()
        .filter_map(|e| match e {
            Ev::TraversalApply { delta, guard, .. } => Some((*delta, *guard)),
            _ => None,
        })
        .collect();
    assert_eq!(
        pass1,
        vec![(TraversalDelta::Back, true)],
        "pass 1 applies ONLY the initial snapshot (the Back), inside the guard — \
         the reentrant Forward is NOT drained to exhaustion (bounded, T1)"
    );
    assert!(
        host.queue.has_pending_traversal(),
        "the reentrant Forward is deferred — still queued after the bounded pass"
    );

    // Pass 2 (a later pump turn): the deferred Forward now applies, still inside the
    // guard, and the queue drains empty (liveness via the async pump).
    let _ = DrainCoordinator::run_deferred_traversals(&mut host);
    let pass2: Vec<_> = host
        .log
        .iter()
        .filter_map(|e| match e {
            Ev::TraversalApply { delta, guard, .. } => Some((*delta, *guard)),
            _ => None,
        })
        .collect();
    assert_eq!(
        pass2,
        vec![
            (TraversalDelta::Back, true),
            (TraversalDelta::Forward, true)
        ],
        "the deferred Forward drained on the NEXT turn (after the Back), inside the guard"
    );
    assert!(
        host.queue.is_empty(),
        "the next turn's bounded pass drained the deferred Forward — nothing stranded"
    );
}

#[test]
fn bounded_drain_processes_only_the_entry_snapshot() {
    // T1 termination-by-construction: a host whose `apply_traversal` re-enqueues
    // mid-apply cannot make the drain over-run its snapshot. Seed TWO steps pending
    // at entry; the first apply re-enqueues a third (`reentrant_once`). The bounded
    // pass pops exactly the 2-step snapshot and TERMINATES — the re-enqueued Go(1)
    // is left for the next turn, NOT drained to exhaustion.
    let mut host = MockHost::new(vec![]).with_reentrant(PendingTraversal {
        delta: TraversalDelta::Go(1),
        user_involvement: UserInvolvement::default(),
    });
    host.queue.enqueue_traversal(PendingTraversal {
        delta: TraversalDelta::Back,
        user_involvement: UserInvolvement::default(),
    });
    host.queue.enqueue_traversal(PendingTraversal {
        delta: TraversalDelta::Forward,
        user_involvement: UserInvolvement::default(),
    });

    let _ = DrainCoordinator::run_deferred_traversals(&mut host);
    let applied = host
        .log
        .iter()
        .filter(|e| matches!(e, Ev::TraversalApply { .. }))
        .count();
    assert_eq!(
        applied, 2,
        "the bounded pass applied ONLY the 2-step entry snapshot (terminated), not \
         the mid-apply re-enqueued Go(1)"
    );
    assert!(
        host.queue.has_pending_traversal(),
        "the re-enqueued Go(1) is deferred to the next turn (bounded, not exhausted)"
    );
}

// --- Slice A co-design seams (A / B / D / E) --------------------------------

#[test]
fn noop_traversal_peek_classify_falls_through() {
    // Resolution E: `go(999); pushState('/x')` at end-of-history — the no-op
    // go(999) classifies as `None`, so it is NOT a partition barrier. The trailing
    // push applies IN-TASK (Phase 1), no `Traversal` step is queued, and a
    // same-turn nav is NOT suppressed.
    let mut host = MockHost::new(vec![go(999), push("/x")])
        .with_noop_classify()
        .with_navigation();
    let outcome = DrainCoordinator::drain_synchronous_phase(&mut host);

    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, guard } if label == "push:/x" && !*guard)),
        "the trailing push applied in-task (a no-op traversal is not a barrier)"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "a no-op traversal never enqueues → nothing to apply"
    );
    assert!(
        !host.queue.has_pending_traversal(),
        "no Traversal step queued by a no-op"
    );
    assert!(host.queue.is_empty(), "no deferred work");
    assert!(
        host.log.iter().any(|e| matches!(e, Ev::Navigation)),
        "the same-turn nav APPLIED (not drain-and-discarded) — no-op is not a barrier"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::NavigationDiscarded)),
        "a no-op traversal must not suppress the nav"
    );
    assert!(outcome.own_context_action);
}

#[test]
fn pending_traversal_drains_and_discards_navigation() {
    // Resolution A / F1: `history.back(); location.href='/b'` — the in-range
    // back() enqueues (Phase 1b), so Phase 1c drains-and-DISCARDS the nav slot: the
    // nav does not apply AND does not strand to re-fire a turn late. Uses
    // `drain_synchronous_phase` so Phase 2 does not run (the traversal stays queued).
    let mut host = MockHost::new(vec![back()]).with_navigation();
    let outcome = DrainCoordinator::drain_synchronous_phase(&mut host);

    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::NavigationDiscarded)),
        "the nav slot was drained-and-discarded (not stranded)"
    );
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::Navigation)),
        "the suppressed nav did NOT apply"
    );
    assert!(
        host.queue.has_pending_traversal(),
        "the traversal is still queued for Phase 2 (default-suppression signal — B)"
    );
    assert!(
        !outcome.own_context_action,
        "no own-context nav applied in Phase 1 (the traversal defers to Phase 2)"
    );
}

#[test]
fn cross_turn_pending_traversal_still_discards_navigation() {
    // Resolution A cross-turn (E1): a traversal queued last turn (Phase 2 not yet
    // pumped) still suppresses THIS turn's nav via `has_pending_traversal`, so the
    // seed (`seen_traversal = !is_empty()`) and the drain-and-discard both key on
    // the cross-turn queue state.
    let mut host = MockHost::new(vec![]).with_navigation();
    host.queue.enqueue_traversal(PendingTraversal {
        delta: TraversalDelta::Back,
        user_involvement: UserInvolvement::default(),
    });

    let _ = DrainCoordinator::drain_synchronous_phase(&mut host);
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::NavigationDiscarded)),
        "a still-queued cross-turn traversal drains-and-discards this turn's nav"
    );
    assert!(!host.log.iter().any(|e| matches!(e, Ev::Navigation)));
}

#[test]
fn syncupdate_canceled_after_document_changing_traversal() {
    // Resolution D (GENERALIZED, Codex PR#469 R6): `back(); pushState('/x')` where
    // the back() rebuilds a FRESH document — the deferred /x push is CANCELED (it
    // must not mutate the newly-active document's identity), shipping no incoherent
    // cross-document state. Now the SAME code path as the same-document case
    // (`syncupdate_canceled_after_same_document_traversal`): ANY traversal cancels a
    // trailing straddle sync (the mock no longer distinguishes rebuild vs
    // same-document — that discrimination was superseded).
    let mut host = MockHost::new(vec![back(), push("/x")]);
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the back() traversal applied"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x")),
        "the deferred push is CANCELED after the traversal (Resolution D generalized)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn syncupdate_canceled_after_same_document_traversal() {
    // Resolution D GENERALIZATION (Codex PR#469 R6) — the FLIP of the retired
    // `syncupdate_applies_after_same_document_traversal`: a straddle sync behind a
    // SAME-document traversal now also CANCELS (it previously applied). Applying it
    // against the post-traversal cursor would land the update on the traversal
    // target, corrupting the current entry (`back(); replaceState('/x')` would land
    // `/x`-current instead of leaving `base` current). The correct §7.4.1.3
    // jump-the-queue application to the call-time entry is fenced to
    // `#11-sync-navigation-steps-queue-tagging`.
    let mut host = MockHost::new(vec![back(), push("/x")]);
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the back() traversal applied"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x")),
        "the deferred push is CANCELED after ANY same-turn traversal, not applied \
         against the post-traversal cursor (Resolution D generalized, R6)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn syncupdate_applies_after_failed_load_barrier_did_not_move_cursor() {
    // Resolution D re-check (Codex PR#469 R6 fix-delta): the straddle cancel is
    // gated on the barrier traversal MOVING THE CURSOR (`shipped`), NOT on merely
    // being processed. `back(); pushState('/kept')` where the back()'s cross-document
    // load FAILS at apply (`apply_traversal` reports `shipped = false` —
    // peek-then-commit atomicity, the cursor never moved) leaves the still-active
    // document on the CALL-TIME entry. The trailing straddle `SyncUpdate('/kept')`
    // must therefore APPLY there (coherently — no jump-the-queue needed), NOT be
    // canceled. Contrast `syncupdate_canceled_after_same_document_traversal` /
    // `_after_document_changing_traversal` (both `shipped = true` → cursor moved →
    // cancel). This drives through the coordinator (the R2 test
    // `failed_traversal_load_does_not_drop_trailing_history` bypasses the latch by
    // hand-sequencing the two host calls), pinning the coordinator-level path.
    let mut host = MockHost::new(vec![back(), push("/kept")]).with_noop_traversal();
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the back() barrier traversal was applied (and reported a failed load)"
    );
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/kept")),
        "the trailing straddle pushState IS applied — a failed-load barrier did NOT \
         move the cursor, so the latch stays clear and the sync is NOT canceled \
         (cancellation is only when a traversal moved the cursor)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn has_pending_traversal_reflects_only_traversal_steps() {
    // The ONE default-suppression signal (B): true iff a `Traversal` step is
    // queued; a `SyncUpdate`-only queue reports false (so it does not over-suppress).
    let mut host = MockHost::new(vec![]);
    assert!(!host.queue.has_pending_traversal(), "empty queue");
    host.queue.enqueue_sync_update(push("/x"));
    assert!(
        !host.queue.has_pending_traversal(),
        "a SyncUpdate-only queue holds no Traversal step"
    );
    host.queue.enqueue_traversal(PendingTraversal {
        delta: TraversalDelta::Back,
        user_involvement: UserInvolvement::default(),
    });
    assert!(
        host.queue.has_pending_traversal(),
        "now a Traversal step is pending"
    );
}

#[test]
fn later_traversal_enqueues_unconditionally_after_a_barrier() {
    // F4: `back(); forward()` from `[base, /a]` at `/a`. The FIRST traversal
    // (back) peek-classifies in-range → starts the barrier. The SECOND (forward)
    // peeks the STILL-UNMOVED index-1 cursor → out-of-range (`classify_answers`
    // front `false`), but because a barrier now exists it must enqueue
    // UNCONDITIONALLY (its target resolves at apply time) — the pre-apply peek must
    // NOT drop it. Old behavior dropped the forward, landing on `base`; the fix
    // applies BOTH, netting back on `/a`.
    let mut host = MockHost::new(vec![back(), forward()]);
    host.classify_answers = std::collections::VecDeque::from(vec![true, false]);

    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let applied: Vec<_> = host
        .log
        .iter()
        .filter_map(|e| match e {
            Ev::TraversalApply { delta, .. } => Some(*delta),
            _ => None,
        })
        .collect();
    assert_eq!(
        applied,
        vec![TraversalDelta::Back, TraversalDelta::Forward],
        "both traversals applied in issue order — the forward was NOT dropped by \
         the pre-apply peek against the unmoved cursor (F4)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn first_noop_traversal_before_a_barrier_is_still_dropped() {
    // F4 guard (first-traversal peek intact): `go(999); back()`. The FIRST
    // traversal (go(999)) peeks out-of-range (`classify_answers` front `false`) →
    // it is NOT a barrier and is dropped (Resolution E). Only THEN does the back()
    // — still the first REAL barrier candidate — peek in-range and enqueue. So a
    // no-op leading `go` must not enqueue, and the back is the sole applied step.
    let mut host = MockHost::new(vec![go(999), back()]);
    host.classify_answers = std::collections::VecDeque::from(vec![false, true]);

    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let applied: Vec<_> = host
        .log
        .iter()
        .filter_map(|e| match e {
            Ev::TraversalApply { delta, .. } => Some(*delta),
            _ => None,
        })
        .collect();
    assert_eq!(
        applied,
        vec![TraversalDelta::Back],
        "the no-op leading go(999) was dropped (not a barrier); only the in-range \
         back() enqueued and applied"
    );
}

#[test]
fn stacked_back_back_enqueues_both() {
    // F4 guard (`back(); back()` harmless): both backs peek in-range against the
    // unmoved cursor, so the first STARTS a barrier and the second enqueues
    // unconditionally — BOTH queue. (The 2nd applying as a no-op after the cursor
    // moved is content-level territory; here the mock just proves both enqueue.)
    let mut host = MockHost::new(vec![back(), back()]);
    host.classify_answers = std::collections::VecDeque::from(vec![true, true]);

    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let applied = host
        .log
        .iter()
        .filter(|e| matches!(e, Ev::TraversalApply { .. }))
        .count();
    assert_eq!(applied, 2, "both stacked backs enqueued and applied");
}

#[test]
fn in_flight_traversal_barrier_defers_sync_and_discards_nav() {
    // F1: Phase 1 re-entered reentrantly DURING Phase 2 — the in-flight traversal
    // was POPPED off the pending queue (`has_pending_traversal() == false`) but
    // the apply still owns the peek→commit window (`is_applying() == true`). A
    // reentrant `pushState` must DEFER onto the queue (not apply in-task) and a
    // reentrant `location.*` must be drain-and-DISCARDED — `is_applying()` is an
    // additional barrier + suppression signal, completing the nested-apply guard.
    let mut host = MockHost::new(vec![push("/reentrant")]).with_navigation();
    host.queue.enter_nested_apply(); // simulate mid-Phase-2 apply
    assert!(host.queue.is_applying(), "guard set (mid-apply)");
    assert!(
        !host.queue.has_pending_traversal(),
        "the in-flight traversal was popped — nothing pending in the queue"
    );

    let outcome = DrainCoordinator::drain_synchronous_phase(&mut host);

    // The reentrant sync update did NOT apply in-task…
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::SyncUpdate { .. })),
        "the reentrant pushState did not apply in-task (is_applying is a barrier)"
    );
    // …it was enqueued onto the queue (drained later by the Phase-2 loop).
    assert!(
        !host.queue.is_empty(),
        "the reentrant pushState was enqueued (serialized onto the queue)"
    );
    // The reentrant nav was drain-and-discarded, not applied.
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::NavigationDiscarded)),
        "the reentrant nav was drain-and-discarded (is_applying suppresses)"
    );
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::Navigation)),
        "the suppressed nav did NOT apply"
    );
    assert!(
        outcome.suppress_default,
        "is_applying sets suppress_default (the default is suppressed under a nested apply)"
    );
}
