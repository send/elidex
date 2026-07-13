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

struct MockHost {
    queue: TraversalQueue,
    pending: Vec<HistoryAction>,
    /// What [`DrainHost::handle_navigation`] reports (did a `location.*` apply).
    nav_applies: bool,
    /// A reentrant traversal to enqueue mid-apply on the FIRST
    /// [`DrainHost::apply_traversal`] — the SW-pump reentrancy vector (plan §4.4).
    reentrant_once: Option<PendingTraversal>,
    log: Vec<Ev>,
}

impl MockHost {
    fn new(pending: Vec<HistoryAction>) -> Self {
        Self {
            queue: TraversalQueue::new(),
            pending,
            nav_applies: false,
            reentrant_once: None,
            log: Vec::new(),
        }
    }

    fn with_navigation(mut self) -> Self {
        self.nav_applies = true;
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

    fn handle_navigation(&mut self) -> bool {
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
        true // a traversal ships its own frame
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
    let outcome = DrainCoordinator::drain(&mut host);

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
fn i1_full_phase1_precedes_phase2() {
    // sync update + a last-wins navigation (Phase 1c) must BOTH precede the
    // deferred traversal apply (Phase 2).
    let mut host = MockHost::new(vec![push("/a"), back()]).with_navigation();
    let _ = DrainCoordinator::drain(&mut host);

    let sync = host
        .position(|e| matches!(e, Ev::SyncUpdate { .. }))
        .unwrap();
    let nav = host.position(|e| matches!(e, Ev::Navigation)).unwrap();
    let traversal = host
        .position(|e| matches!(e, Ev::TraversalApply { .. }))
        .unwrap();
    assert!(sync < nav, "sync update precedes the last-wins navigation");
    assert!(
        nav < traversal,
        "Phase-1 navigation precedes the Phase-2 traversal apply"
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
    let _ = DrainCoordinator::drain(&mut host);

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
    let _ = DrainCoordinator::drain(&mut host);

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

// --- I3: guard bracket + eventual drain -------------------------------------

#[test]
fn i3_guard_is_set_during_traversal_apply_only() {
    // The nested-apply boolean is TRUE inside `apply_traversal` and FALSE for a
    // (Phase-1) sync update — the bracket covers the traversal peek→commit only.
    let mut host = MockHost::new(vec![push("/a"), back()]);
    assert!(!host.queue.is_applying(), "initially false (§7.3.1.1)");
    let _ = DrainCoordinator::drain(&mut host);

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
    let _ = DrainCoordinator::drain(&mut host);

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
    let outcome = DrainCoordinator::drain(&mut host);
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
    let outcome = DrainCoordinator::drain(&mut host);
    assert!(
        !host.log.iter().any(|e| matches!(e, Ev::ShipFrame)),
        "no redundant end-of-turn ship after a navigation shipped"
    );
    assert!(outcome.own_context_action && outcome.shipped);
}

#[test]
fn empty_turn_is_a_noop() {
    // No history, no navigation — only the window-open drain runs, nothing ships.
    let mut host = MockHost::new(vec![]);
    let outcome = DrainCoordinator::drain(&mut host);
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
