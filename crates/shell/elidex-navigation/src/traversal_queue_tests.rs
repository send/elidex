//! Isolation unit tests for the [`super::DrainCoordinator`] +
//! [`super::TraversalQueue`] — no shell, a mock [`DrainHost`]. They pin the plan
//! §4.5 invariants (`docs/plans/2026-07-session-history-task-queue-model.md`):
//! I1 (Phase-1-before-Phase-2 ordering), I2 (issue-order-preserving partition —
//! a `pushState; back(); pushState` sequence must NOT reorder the trailing push
//! ahead of the traversal), and I3 (guard bracket + bounded next-turn drain — the
//! Phase-2 drain processes a bounded snapshot, deferring a reentrant re-enqueue to
//! the next turn rather than draining to exhaustion, T1).
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

fn forward() -> HistoryAction {
    HistoryAction::Forward
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
    /// When set, [`DrainHost::classify_traversal`] returns `None` (a no-op
    /// out-of-range peek — Resolution E), so the traversal is NOT a barrier.
    classify_noop: bool,
    /// Per-call [`DrainHost::classify_traversal`] answers (front = first call):
    /// `true` = in-range (`Some`), `false` = out-of-range (`None`). Models a peek
    /// that is in-range for the FIRST traversal but out-of-range against the
    /// still-UNMOVED cursor for a later one — the F4 barrier case. When empty,
    /// falls back to the `classify_noop` flag.
    classify_answers: std::collections::VecDeque<bool>,
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
            classify_noop: false,
            classify_answers: std::collections::VecDeque::new(),
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
        // a barrier); `Some` = in-range, enqueued as a barrier. A per-call
        // `classify_answers` entry stands in for the shell's `peek_*` (front =
        // first call — models an in-range FIRST + out-of-range LATER peek, F4);
        // absent it, the `classify_noop` flag governs.
        let in_range = self
            .classify_answers
            .pop_front()
            .unwrap_or(!self.classify_noop);
        in_range.then(|| self.pending_traversal(delta))
    }

    fn pending_traversal(&mut self, delta: TraversalDelta) -> PendingTraversal {
        // F4: build the pending value with NO peek — the coordinator calls this
        // for every traversal after a barrier exists (target resolves at apply
        // time), so a later traversal peeking out-of-range is never dropped.
        PendingTraversal {
            delta,
            user_involvement: UserInvolvement::default(),
        }
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

    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> bool {
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
        // `false` = a no-op traversal (no-target / failed load); default `true` =
        // applied and shipped its own frame. Any traversal apply now triggers the
        // coordinator's generalized straddle-`SyncUpdate` cancel (Resolution D, R6).
        !self.traversal_noop
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
    // Post-R6 the trailing straddle push is then CANCELED in Phase 2 (Resolution D
    // generalized — a straddle sync behind ANY traversal is dropped, not applied
    // against the post-traversal cursor). I2 (no reorder-ahead) is what this pins;
    // the cancel is what the syncupdate_canceled_* tests pin.
    let mut host = MockHost::new(vec![push("/a"), back(), push("/x")]);
    let _ = DrainCoordinator::drain_same_turn(&mut host);

    let lead = host
        .position(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/a"))
        .expect("leading push applied in Phase 1");
    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .expect("traversal applied");

    assert!(lead < traversal, "the leading push is a Phase-1 sync");
    // The trailing push (issued after the traversal) was NOT hoisted ahead into
    // Phase 1 — it deferred behind the traversal and was CANCELED (never applied).
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x")),
        "the trailing straddle push was deferred behind the traversal (not reordered \
         ahead, I2) and then canceled (Resolution D generalized, R6)"
    );
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

    // Draining Phase 2 applies the older traversal, then CANCELS the deferred push
    // (Resolution D generalized, R6 — the cross-turn straddle sync is dropped, not
    // applied against the post-traversal cursor; the same cancellation as the
    // same-turn straddle, uniform across turns).
    let _ = DrainCoordinator::run_deferred_traversals(&mut host);
    assert!(
        host.log
            .iter()
            .any(|e| matches!(e, Ev::TraversalApply { .. })),
        "the older traversal applied"
    );
    assert!(
        !host
            .log
            .iter()
            .any(|e| matches!(e, Ev::SyncUpdate { label, .. } if label == "push:/x")),
        "the cross-turn deferred push is CANCELED after the older traversal applies \
         (not applied against the post-traversal cursor — R6)"
    );
    assert!(host.queue.is_empty(), "everything drained");
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

// Drain / guard / cancel semantics live in a sibling test file, nested here so
// they share the `MockHost` harness above via `super::*` (one helper copy).
#[path = "traversal_queue_drain_tests.rs"]
mod drain;
