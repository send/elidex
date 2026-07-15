//! Isolation unit tests for the [`super::DrainCoordinator`] +
//! [`super::TraversalQueue`] — no shell, a mock [`DrainHost`]. They pin the plan
//! §4.5 invariants (`docs/plans/2026-07-session-history-task-queue-model.md`):
//! I1 (Phase-1-before-Phase-2 ordering), I2 (issue-order-preserving partition —
//! a `pushState; back(); pushState` sequence must NOT reorder the trailing push
//! ahead of the traversal), and I3 (guard bracket + eventual drain).
//!
//! Per plan §4.5 these tests assert the coordinator's **ordering / guard
//! structure**, NOT a specific same-turn-straddle *navigation outcome* (that is
//! conformance-test territory once the shell bodies exist, Slices 2/3).

use super::*;
use elidex_script_session::HistoryAction;

// --- action builders -------------------------------------------------------

fn push(url: &str) -> HistoryAction {
    HistoryAction::PushState {
        url: Some(url.to_string()),
        title: String::new(),
        serialized_state: None,
    }
}

fn back() -> HistoryAction {
    HistoryAction::Back
}

fn go(delta: i32) -> HistoryAction {
    HistoryAction::Go(delta)
}

/// A short label for a synchronous update, so the event log is legible.
fn label(action: &HistoryAction) -> String {
    match action {
        HistoryAction::PushState { url, .. } => format!("push:{}", url.as_deref().unwrap_or("")),
        HistoryAction::ReplaceState { url, .. } => {
            format!("replace:{}", url.as_deref().unwrap_or(""))
        }
        other => format!("{other:?}"),
    }
}

// --- the mock host ---------------------------------------------------------

/// One observed coordinator→host call, in the order the coordinator made it —
/// the event log is the ordering proof for I1/I2.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ev {
    WindowOpens,
    /// A synchronous update apply; `guard` = the nested-apply boolean at call
    /// time (must be `false` — a sync update is never bracketed).
    SyncUpdate {
        label: String,
        guard: bool,
    },
    Navigation,
    /// A Phase-1c navigation drained-and-DISCARDED under suppression (plan §1 A /
    /// F1): the slot was drained (so it cannot re-fire a turn late) but the
    /// request dropped without applying.
    NavigationDiscarded,
    /// A deferred traversal apply; `guard` = the boolean at call time (must be
    /// `true` — the I3 bracket). Issue order is pinned by the *position* of this
    /// event in the log (the VecDeque preserves FIFO order — plan §4.5 I2), not a
    /// stored index.
    TraversalApply {
        delta: TraversalDelta,
        guard: bool,
    },
    ShipFrame,
}

// A test mock's independent behavior toggles read most clearly as flat bools.
#[allow(clippy::struct_excessive_bools)]
struct MockHost {
    queue: TraversalQueue,
    pending: Vec<HistoryAction>,
    /// What [`DrainHost::handle_navigation`] reports (did a `location.*` apply).
    nav_applies: bool,
    /// A reentrant traversal to enqueue mid-apply on the FIRST
    /// [`DrainHost::apply_traversal`] — the SW-pump reentrancy vector (plan §4.4).
    reentrant_once: Option<PendingTraversal>,
    /// When set, [`DrainHost::apply_traversal`] reports `shipped = false` (a
    /// **no-op** traversal — no-target `go(999)` / failed cross-document load)
    /// instead of the default apply-and-ship `shipped = true`.
    traversal_noop: bool,
    /// When set, [`DrainHost::apply_traversal`] reports `changed_document = true`
    /// (a §7.4.6.1 `Rebuild` that landed a fresh document) — drives the
    /// Resolution-D `SyncUpdate` cancellation (plan §1 D).
    traversal_changes_document: bool,
    /// When set, [`DrainHost::classify_traversal`] returns `None` (a no-op
    /// out-of-range peek — Resolution E), so the traversal is NOT a barrier.
    classify_noop: bool,
    log: Vec<Ev>,
}

impl MockHost {
    fn new(pending: Vec<HistoryAction>) -> Self {
        Self {
            queue: TraversalQueue::new(),
            pending,
            nav_applies: false,
            reentrant_once: None,
            traversal_noop: false,
            traversal_changes_document: false,
            classify_noop: false,
            log: Vec::new(),
        }
    }

    fn with_navigation(mut self) -> Self {
        self.nav_applies = true;
        self
    }

    /// Make [`DrainHost::apply_traversal`] a no-op (report `shipped = false`) — a
    /// no-target `history.go(999)` / failed cross-document load. Pins Codex
    /// Finding 3.
    fn with_noop_traversal(mut self) -> Self {
        self.traversal_noop = true;
        self
    }

    /// Make [`DrainHost::apply_traversal`] report `changed_document = true` (a
    /// document-changing `Rebuild`) — Resolution D drives the `SyncUpdate` cancel.
    fn with_document_change(mut self) -> Self {
        self.traversal_changes_document = true;
        self
    }

    /// Make [`DrainHost::classify_traversal`] return `None` (a no-op out-of-range
    /// traversal — Resolution E) so it is NOT a partition barrier.
    fn with_noop_classify(mut self) -> Self {
        self.classify_noop = true;
        self
    }

    fn with_reentrant(mut self, traversal: PendingTraversal) -> Self {
        self.reentrant_once = Some(traversal);
        self
    }

    /// Index of the first event matching `pred`, for ordering assertions.
    fn position(&self, pred: impl Fn(&Ev) -> bool) -> Option<usize> {
        self.log.iter().position(pred)
    }
}

impl DrainHost for MockHost {
    fn traversal_queue(&mut self) -> &mut TraversalQueue {
        &mut self.queue
    }

    fn route_window_opens(&mut self) {
        self.log.push(Ev::WindowOpens);
    }

    fn take_pending_history(&mut self) -> Vec<HistoryAction> {
        std::mem::take(&mut self.pending)
    }

    fn handle_history_action(&mut self, action: &HistoryAction) {
        let guard = self.queue.is_applying();
        self.log.push(Ev::SyncUpdate {
            label: label(action),
            guard,
        });
    }

    fn classify_traversal(&mut self, delta: TraversalDelta) -> Option<PendingTraversal> {
        // Resolution E peek-classify: `None` = a no-op out-of-range traversal (not
        // a barrier); `Some` = in-range, enqueued as a barrier. The mock's
        // `classify_noop` flag stands in for the shell's `peek_*` returning `None`.
        if self.classify_noop {
            return None;
        }
        Some(PendingTraversal {
            delta,
            user_involvement: UserInvolvement::default(),
        })
    }

    fn handle_navigation(&mut self, suppress: bool) -> bool {
        if suppress {
            // Drain-and-DISCARD (plan §1 A / F1): the slot IS drained so it cannot
            // re-fire a turn late; a nav it held is dropped without applying
            // (logged only when there WAS one — an empty slot drains to a no-op).
            if self.nav_applies {
                self.log.push(Ev::NavigationDiscarded);
            }
            return false;
        }
        if self.nav_applies {
            self.log.push(Ev::Navigation);
            true
        } else {
            false
        }
    }

    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> TraversalApplyOutcome {
        // The coordinator must have set the guard BEFORE this call (I3).
        let guard = self.queue.is_applying();
        self.log.push(Ev::TraversalApply {
            delta: traversal.delta,
            guard,
        });
        // Simulate a reentrant nav-mutating message (SW-pump) arriving mid-apply:
        // it is SERIALIZED onto the queue (never applied under the held peek).
        if let Some(reentrant) = self.reentrant_once.take() {
            assert!(
                self.queue.is_applying(),
                "reentrant message must observe the guard set (I3)"
            );
            self.queue.enqueue_traversal(reentrant);
        }
        // `shipped = false` = a no-op traversal (no-target / failed load); default
        // `true` = applied and shipped its own frame. `changed_document` drives
        // Resolution D's `SyncUpdate` cancel.
        TraversalApplyOutcome {
            shipped: !self.traversal_noop,
            changed_document: self.traversal_changes_document,
        }
    }

    fn ship_frame(&mut self) {
        self.log.push(Ev::ShipFrame);
    }
}

// --- I1: Phase-1 (sync + navigation) completes before Phase-2 traversal apply

#[test]
fn i1_sync_update_applies_before_traversal() {
    // `pushState('/a'); history.back()` — the sync update must land in Phase 1
    // (in-task) before the Phase-2 traversal apply reads the entry list.
    let mut host = MockHost::new(vec![push("/a"), back()]);
    let outcome = DrainCoordinator::drain_same_turn(&mut host);

    let sync = host
        .position(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/a"))
        .expect("sync update applied");
    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .expect("traversal applied");
    assert!(
        sync < traversal,
        "Phase-1 sync must precede Phase-2 traversal"
    );
    assert!(outcome.own_context_action);
    assert!(outcome.shipped, "the traversal apply ships its own frame");
    // window-opens are drained first (before any own-context work).
    assert_eq!(host.log.first(), Some(&Ev::WindowOpens));
}

#[test]
fn i1_full_phase1_precedes_phase2_and_nav_suppressed() {
    // FLIP (plan §5, Resolution A): the pre-phase-sep "run both nav AND traversal"
    // model is retired. A Phase-1 sync update precedes the Phase-2 traversal apply
    // (I1), and the same-turn last-wins navigation is now SUPPRESSED
    // (drain-and-discard) because an in-range traversal is pending — the nav is
    // discarded, not applied (§7.4.2.2 step 19 "ignored"; the old shell
    // `return true` supersede's phase-separated form).
    let mut host = MockHost::new(vec![push("/a"), back()]).with_navigation();
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let sync = host
        .position(|e| matches!(e, Ev::SyncUpdate { .. }))
        .unwrap();
    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .unwrap();
    assert!(
        sync < traversal,
        "Phase-1 sync precedes the Phase-2 traversal apply (I1)"
    );
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::NavigationDiscarded)),
        "the same-turn nav is drain-and-discarded (a pending traversal supersedes)"
    );
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::Navigation)),
        "the suppressed nav did NOT apply"
    );
}

// --- I2: issue-order-preserving partition (no "all sync first") -------------

#[test]
fn i2_trailing_push_not_reordered_ahead_of_traversal() {
    // `pushState('/a'); history.back(); pushState('/x')` — the classic straddle.
    // The LEADING push is a Phase-1 sync (issued before any traversal); the
    // traversal defers; the TRAILING push was issued AFTER the traversal and so
    // must NOT jump ahead of it into Phase 1 ("all sync first" is not the model).
    let mut host = MockHost::new(vec![push("/a"), back(), push("/x")]);
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let lead = host
        .position(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/a"))
        .expect("leading push applied");
    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .expect("traversal applied");
    let trail = host
        .position(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x"))
        .expect("trailing push applied");

    assert!(lead < traversal, "the leading push is a Phase-1 sync");
    assert!(
        traversal < trail,
        "the trailing push must NOT be reordered ahead of the traversal (I2)"
    );
    // I2 is pinned by the observed drain order (lead-sync < traversal < trailing-sync):
    // exactly one traversal exists and it applied between the two syncs, so the
    // trailing push was never hoisted ahead of the traversal issued before it.
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn i2_multiple_traversals_preserve_issue_order() {
    // `back(); go(2)` — two traversals defer in issue order (0 then 1).
    let mut host = MockHost::new(vec![back(), go(2)]);
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let applied: Vec<_> = host
        .log
        .iter()
        .filter_map(|e| match e {
            Ev::TraversalApply { delta, .. } => Some(*delta),
            _ => None,
        })
        .collect();
    // The drain ORDER is the issue-order pin: Back was issued before Go(2), so it
    // must apply first (the VecDeque preserves FIFO position — plan §4.5 I2).
    assert_eq!(
        applied,
        vec![TraversalDelta::Back, TraversalDelta::Go(2)],
        "traversals apply in issue order"
    );
}

#[test]
fn i2_new_sync_defers_behind_a_traversal_queued_last_turn() {
    // CROSS-TURN I2 (Codex PR#464 R2): under the split entry points the queue
    // persists across turns. A traversal queued by a PRIOR turn's Phase 1 (Phase 2
    // not yet drained) must NOT be overtaken by THIS turn's fresh `pushState` — the
    // single-FIFO ordering holds across turns, not just within one batch.
    let mut host = MockHost::new(vec![push("/x")]);
    // Simulate a prior turn having deferred a `back()` traversal still awaiting Phase 2.
    host.queue.enqueue_traversal(PendingTraversal {
        delta: TraversalDelta::Back,
        user_involvement: UserInvolvement::default(),
    });

    // This turn's Phase 1 must DEFER the fresh push (queue already non-empty), not
    // apply it in-task ahead of the older traversal.
    let _ = DrainCoordinator::drain_synchronous_phase(&mut host);
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x")),
        "the fresh push must NOT apply in Phase 1 ahead of a last-turn traversal"
    );
    assert!(
        !host.queue.is_empty(),
        "the push deferred onto the queue behind the older traversal (not applied in Phase 1)"
    );

    // Draining Phase 2 applies the older traversal FIRST, then the deferred push.
    let _ = DrainCoordinator::run_deferred_traversals(&mut host);
    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .expect("older traversal applied");
    let deferred_push = host
        .position(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x"))
        .expect("deferred push applied");
    assert!(
        traversal < deferred_push,
        "the last-turn traversal applies before this turn's deferred push (cross-turn I2)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

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
fn i3_reentrant_message_is_serialized_and_eventually_drained() {
    // A reentrant traversal enqueued DURING the first apply (the SW-pump vector)
    // must be re-checked and drained before `drain` returns (eventual drain),
    // not stranded until the next turn — and it too runs inside the guard.
    let reentrant = PendingTraversal {
        delta: TraversalDelta::Forward,
        user_involvement: UserInvolvement::default(),
    };
    let mut host = MockHost::new(vec![back()]).with_reentrant(reentrant);
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let applies: Vec<_> = host
        .log
        .iter()
        .filter_map(|e| match e {
            Ev::TraversalApply { delta, guard, .. } => Some((*delta, *guard)),
            _ => None,
        })
        .collect();
    assert_eq!(
        applies,
        vec![
            (TraversalDelta::Back, true),
            (TraversalDelta::Forward, true),
        ],
        "the reentrant Forward drained after the Back, both inside the guard"
    );
    assert!(
        host.queue.is_empty(),
        "re-check-until-empty left nothing stranded (I3 eventual drain)"
    );
}

// --- frame shipping bookkeeping ---------------------------------------------

#[test]
fn pure_sync_turn_ships_once() {
    // A pushState-only turn ships exactly once at end (no apply body shipped).
    let mut host = MockHost::new(vec![push("/a")]);
    let outcome = DrainCoordinator::drain_same_turn(&mut host);
    assert_eq!(
        host.log
            .iter()
            .filter(|e| matches!(e, Ev::ShipFrame))
            .count(),
        1
    );
    assert!(outcome.own_context_action && outcome.shipped);
}

#[test]
fn navigation_turn_does_not_double_ship() {
    // A navigation ships its own frame — the coordinator must NOT ship again.
    let mut host = MockHost::new(vec![]).with_navigation();
    let outcome = DrainCoordinator::drain_same_turn(&mut host);
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::ShipFrame)),
        "no redundant end-of-turn ship after a navigation shipped"
    );
    assert!(outcome.own_context_action && outcome.shipped);
}

#[test]
fn noop_traversal_marks_no_action() {
    // A no-op traversal (`history.go(999)` with no target / a failed cross-document
    // load) returns `false` from `apply_traversal`, so the coordinator marks NO
    // own-context action and ships nothing — the caller's fallback/default action
    // is not over-suppressed (pins Codex Finding 3, mirrors `handle_navigation`).
    let mut host = MockHost::new(vec![back()]).with_noop_traversal();
    let outcome = DrainCoordinator::drain_same_turn(&mut host);

    assert!(
        !outcome.own_context_action,
        "a no-op traversal marks no own-context action"
    );
    assert!(!outcome.shipped, "a no-op traversal ships nothing");
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::ShipFrame)),
        "no frame shipped for a no-op traversal (nothing to suppress)"
    );
}

#[test]
fn sync_update_with_noop_traversal_still_ships() {
    // `history.pushState('/a'); history.go(999)` — a synchronous update PLUS a
    // no-op traversal (no target at the resolved step). The pushState is a real
    // own-context effect that must render its frame this turn; the no-op traversal
    // ships nothing. Regression: the earlier two-phase split gated Phase 1's ship
    // on an empty queue (go(999) held the queue) and Phase 2's ship on the no-op
    // apply (returns false) — their intersection stranded the committed push frame
    // (neither phase shipped). The shared `ship_if_needed` tail ships it once.
    let mut host = MockHost::new(vec![push("/a"), go(999)]).with_noop_traversal();
    let outcome = DrainCoordinator::drain_same_turn(&mut host);

    assert!(
        outcome.own_context_action,
        "the pushState is a real own-context action"
    );
    assert!(
        outcome.shipped,
        "the pushState's frame ships despite the no-op traversal"
    );
    assert_eq!(
        host.log
            .iter()
            .filter(|e| matches!(e, Ev::ShipFrame))
            .count(),
        1,
        "exactly ONE frame ships (the pushState render), not zero and not two"
    );
    // The no-op traversal DID apply (returned false) — it just shipped nothing.
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the go(999) traversal was applied (and returned no-op)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn phases_schedule_separately() {
    // The two-phase seam: `drain_synchronous_phase` runs Phase 1 (the push applies)
    // and ENQUEUES the traversal without applying it; `run_deferred_traversals`
    // applies it on a later turn (pins Codex Finding 1 / plan §4.5 I1).
    let mut host = MockHost::new(vec![push("/a"), back()]);

    let _ = DrainCoordinator::drain_synchronous_phase(&mut host);
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/a")),
        "the Phase-1 sync update applied"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the traversal is NOT applied in Phase 1"
    );
    assert!(
        !host.queue.is_empty(),
        "the traversal is still QUEUED after Phase 1"
    );

    let _ = DrainCoordinator::run_deferred_traversals(&mut host);
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the Back applied in Phase 2"
    );
    assert!(host.queue.is_empty(), "Phase 2 drained the queue");
}

#[test]
fn empty_turn_is_a_noop() {
    // No history, no navigation — only the window-open drain runs, nothing ships.
    let mut host = MockHost::new(vec![]);
    let outcome = DrainCoordinator::drain_same_turn(&mut host);
    assert_eq!(host.log, vec![Ev::WindowOpens]);
    assert_eq!(outcome, DrainOutcome::default());
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
    // Resolution D: `back(); pushState('/x')` where the back() rebuilds a FRESH
    // document — the deferred /x push is CANCELED (it must not mutate the
    // newly-active document's identity), shipping no incoherent cross-document state.
    let mut host = MockHost::new(vec![back(), push("/x")]).with_document_change();
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the back() traversal applied (document-changing)"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x")),
        "the deferred push is CANCELED after a document-changing traversal (Resolution D)"
    );
    assert!(host.queue.is_empty(), "everything drained");
}

#[test]
fn syncupdate_applies_after_same_document_traversal() {
    // Resolution D complement: a SAME-document traversal (`changed_document =
    // false`) does NOT cancel the trailing deferred push — no identity mismatch, so
    // it applies in issue order.
    let mut host = MockHost::new(vec![back(), push("/x")]); // default: changed_document false
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .expect("traversal applied");
    let trail = host
        .position(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x"))
        .expect("the trailing push applied (same-document traversal does not cancel it)");
    assert!(
        traversal < trail,
        "the deferred push applies AFTER the same-document traversal (issue order)"
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

// --- classification ---------------------------------------------------------

#[test]
fn traversal_delta_classification() {
    assert_eq!(
        TraversalDelta::from_history_action(&back()),
        Some(TraversalDelta::Back)
    );
    assert_eq!(
        TraversalDelta::from_history_action(&HistoryAction::Forward),
        Some(TraversalDelta::Forward)
    );
    assert_eq!(
        TraversalDelta::from_history_action(&go(-3)),
        Some(TraversalDelta::Go(-3))
    );
    // Synchronous updates are NOT traversals (they stay in Phase 1).
    assert_eq!(TraversalDelta::from_history_action(&push("/a")), None);
    assert_eq!(
        TraversalDelta::from_history_action(&HistoryAction::ReplaceState {
            url: None,
            title: String::new(),
            serialized_state: None,
        }),
        None
    );
}
