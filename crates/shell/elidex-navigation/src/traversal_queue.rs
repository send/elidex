//! The traversable's **session history traversal queue** + the shared
//! **drain-coordinator** — the additive substrate of the session-history
//! task-boundary phase-separation
//! (`docs/plans/2026-07-session-history-task-queue-model.md`, Slice 1).
//!
//! elidex historically drained a *turn's* staged navigation intents in one
//! synchronous pass (window-opens → history FIFO → last-wins navigation),
//! collapsing the spec's two task-timing classes onto a single synchronous
//! return (plan §1). This module introduces, in its **final phase-separated
//! shape**, the primitive both shells (`content/navigation.rs`,
//! `app/navigation.rs`) will adopt (Slices 2/3):
//!
//! - a [`TraversalQueue`] — the WHATWG HTML §7.3.1.1 *session history traversal
//!   queue* (`#tn-session-history-traversal-queue`) carrying the
//!   **"running nested apply history step" boolean** — realized as a
//!   **cooperative deferred queue on elidex's single-writer event loop**, NOT an
//!   OS parallel-queue thread (plan §4.1; CLAUDE.md "Concurrency by ownership and
//!   phases"); and
//! - a [`DrainCoordinator`] — the phase-partition driver, parameterized by the
//!   [`DrainHost`] trait so `ContentState` / `InteractiveState` / the pipeline /
//!   `EcsDom` stay **behind the trait** and never cross the `elidex-navigation`
//!   crate boundary (plan §4.5 "OO→ECS / layer map").
//!
//! Slice 1 wires **no shell**: this is pure additive substrate, exercised only by
//! the isolation unit tests below. Slices 2/3 make each shell drive it and retire
//! the two synchronous drains.
//!
//! ## The task-timing partition (plan §4.2)
//!
//! - **Phase 1 — synchronous, in-task:** window-opens (§7.2.2.1) → synchronous
//!   history *updates* (`pushState` / `replaceState`, WHATWG HTML §7.4.4 *URL and
//!   history update steps*) → last-wins navigation (`location.*`, §7.4.2). These
//!   mutate the session history / rebuild the pipeline in the current task.
//! - **Phase 2 — deferred traversal apply (a later task):** a `Back` / `Forward`
//!   / `Go` *traversal* (§7.4.3 *traverse the history by a delta* step 4 —
//!   "append … traversal steps to traversable") is **not** applied inline; it is
//!   appended to the [`TraversalQueue`] and applied by a separately scheduled
//!   drain that runs *after* Phase 1's updates have landed, realizing §7.4.6.1
//!   *apply the history step* step 12's two-part split ("synchronous navigations
//!   processed before documents unload").
//!
//! The **scope fence** (plan §0) is single-traversable (top-level) only: the
//! §7.4.6.1 multi-navigable fan-out (steps 3/4/6/7 + the per-navigable global
//! task of 8/12) is B1-gated and NOT modelled here.

use elidex_script_session::HistoryAction;

/// A resolved session-history **traversal** delta — the subset of
/// [`HistoryAction`] that defers to a later task (WHATWG HTML §7.4.3 *traverse
/// the history by a delta*), separated from the synchronous
/// `PushState` / `ReplaceState` *updates* (§7.4.4) that stay in-task.
///
/// The delta is carried un-resolved: §7.4.6.1 *apply the history step* resolves
/// the target step index at **apply** time against the (possibly Phase-1-mutated)
/// entry list, so a deferred traversal must NOT pre-resolve a concrete index at
/// issue time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalDelta {
    /// `history.back()` — delta −1.
    Back,
    /// `history.forward()` — delta +1.
    Forward,
    /// `history.go(delta)` — the raw signed delta (`0` = reload, History.go
    /// step 4).
    Go(i32),
}

impl TraversalDelta {
    /// Classify a staged [`HistoryAction`] as a deferred traversal, or `None` for
    /// a synchronous `PushState` / `ReplaceState` *update* (the Phase-1 /
    /// Phase-2 partition predicate, plan §4.5 I2).
    #[must_use]
    pub fn from_history_action(action: &HistoryAction) -> Option<Self> {
        match action {
            HistoryAction::Back => Some(Self::Back),
            HistoryAction::Forward => Some(Self::Forward),
            HistoryAction::Go(delta) => Some(Self::Go(*delta)),
            HistoryAction::PushState { .. } | HistoryAction::ReplaceState { .. } => None,
        }
    }
}

/// User navigation involvement (WHATWG HTML §7.4.2.1 *user navigation
/// involvement*, `#user-navigation-involvement`) — the §7.4.3 step-2 snapshot a
/// deferred traversal captures at **issue** time so the later §7.4.6.1 apply
/// reads the value as it was when the traversal was issued, not when it applies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UserInvolvement {
    /// The traversal was initiated via browser UI (a chrome Back/Forward button).
    BrowserUi,
    /// Initiated via an element's activation behavior (a trusted click).
    Activation,
    /// Not user-initiated — the default for a scripted `history.back()` / `go()`.
    #[default]
    None,
}

/// A pending deferred **traversal apply** (WHATWG HTML §7.4.3 step 4 — the
/// traversal appended onto the traversable, applied as a later task via
/// §7.4.6.1). Carries the resolved [`TraversalDelta`] plus the §7.4.3 steps 1–3
/// source snapshot captured at issue time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingTraversal {
    /// The resolved traversal delta (`Back` / `Forward` / `Go(delta)`).
    pub delta: TraversalDelta,
    /// The turn-relative FIFO position this traversal was issued at. The single
    /// `pending_history` FIFO is the **sole ordering source of truth** (plan
    /// §4.5 I2 axis d), so the queue *preserves* issue order rather than
    /// re-deriving it.
    pub issue_order: usize,
    /// The §7.4.3 step-2 [`UserInvolvement`] snapshot. Slice 1 defaults this
    /// (the VM staging carries no involvement fact today, Q-VM-MODEL =
    /// shell-drain-only); Slices 2/3 thread the real issue-time snapshot (a
    /// chrome-button traversal is [`UserInvolvement::BrowserUi`]).
    pub user_involvement: UserInvolvement,
}

/// One deferred step on the [`TraversalQueue`]. The spec's **one** session
/// history traversal queue carries *tagged step-sets* (WHATWG HTML §7.4.1.3
/// *Centralized modifications of session history* — Q-SYNC-FINALIZE): *traversal
/// steps* (§7.4.3 step 4) and *synchronous navigation steps* (§7.4.4 step 13).
///
/// elidex defers a step-set onto this queue **from the first traversal of a turn
/// onward**, preserving issue order (plan §4.5 I2 — *never reorder a sync update
/// ahead of a traversal issued before it*). A synchronous update issued **after**
/// a same-turn traversal therefore rides this queue as a tagged
/// [`Self::SyncUpdate`] rather than jumping ahead into Phase 1.
///
/// (No `PartialEq`: [`HistoryAction`] carries serialized state and is not `Eq`;
/// tests assert the coordinator's *observed apply order*, not step equality.)
#[derive(Clone, Debug)]
pub enum PendingHistoryStep {
    /// A deferred *traversal* (§7.4.3 → §7.4.6.1 *apply the history step*).
    Traversal(PendingTraversal),
    /// A synchronous `pushState` / `replaceState` *update* (§7.4.4) issued
    /// **after** a same-turn traversal, deferred onto the queue in issue order
    /// (plan §4.5 I2) rather than applied in Phase 1. Its exact same-turn
    /// *straddle* outcome is deliberately NOT pinned here (plan §4.5 I2 / §7
    /// Q-SYNC-FINALIZE — Slice 1/2 conformance-test territory); Slice 1 fixes only
    /// the issue-order-preserving **structure**.
    SyncUpdate(HistoryAction),
}

/// The traversable's **session history traversal queue** (WHATWG HTML §7.3.1.1
/// `#tn-session-history-traversal-queue`) — the deferred [`PendingHistoryStep`]
/// queue plus the **"running nested apply history step" boolean** (initially
/// `false`), the reentrancy guard that serializes a re-entrant nav-mutating
/// apply (plan §4.4 / §4.5 I3).
///
/// Lives on/near the host's [`NavigationController`](crate::NavigationController)
/// (both are the engine-agnostic traversable proxy), so both shells share one
/// primitive (plan §4.1). Realized as a **cooperative single-threaded** queue on
/// elidex's single-writer event loop, not an OS parallel-queue thread (the
/// two-part split needs *ordering*, not parallelism — plan §4.1).
#[derive(Debug, Default)]
pub struct TraversalQueue {
    /// Deferred steps in issue order (plan §4.5 I2 — the single FIFO is the sole
    /// ordering SoT; this queue preserves it).
    pending: std::collections::VecDeque<PendingHistoryStep>,
    /// WHATWG HTML §7.3.1.1 "running nested apply history step", initially
    /// `false`. Set **before the peek** and cleared **after the commit** by the
    /// [`DrainCoordinator`] Phase-2 loop (plan §4.5 I3), covering the entire
    /// peek→commit window so a reentrant nav-mutating message (the SW-fetch
    /// message pump) is *serialized* onto the queue instead of mutating the
    /// cursor under the held peek.
    running_nested_apply_history_step: bool,
}

impl TraversalQueue {
    /// A fresh empty queue with the nested-apply guard cleared (§7.3.1.1
    /// "initially false").
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a deferred **traversal** (§7.4.3 step 4 "append … traversal steps
    /// to traversable"). The reentrant SW-pump vector (plan §4.4) calls this
    /// mid-apply — while [`is_applying`](Self::is_applying) holds — to *serialize*
    /// its traversal onto the queue rather than apply it under a held peek.
    pub fn enqueue_traversal(&mut self, traversal: PendingTraversal) {
        self.pending
            .push_back(PendingHistoryStep::Traversal(traversal));
    }

    /// Append a synchronous *update* issued **after** a same-turn traversal, as a
    /// tagged [`PendingHistoryStep::SyncUpdate`] (plan §4.5 I2 — it may not jump
    /// ahead of the earlier traversal into Phase 1).
    pub fn enqueue_sync_update(&mut self, action: HistoryAction) {
        self.pending
            .push_back(PendingHistoryStep::SyncUpdate(action));
    }

    /// Whether a traversal apply is in progress — the §7.3.1.1 "running nested
    /// apply history step" boolean (plan §4.5 I3). A shell's reentrant
    /// nav-mutating message consults this to decide *serialize onto the queue*
    /// (guard set) vs *apply directly* (guard clear).
    #[must_use]
    pub fn is_applying(&self) -> bool {
        self.running_nested_apply_history_step
    }

    /// Whether the deferred queue is empty (no Phase-2 work pending).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// The number of deferred steps queued.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Pop the next deferred step in issue order (the Phase-2 drain cursor).
    fn pop_next(&mut self) -> Option<PendingHistoryStep> {
        self.pending.pop_front()
    }
}

/// The summary of one [`DrainCoordinator::drain`] pass — mirrors the shells'
/// `process_pending_*` boolean while exposing the frame-ship bookkeeping the
/// coordinator uses to avoid a double-send.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrainOutcome {
    /// An **own-context** history / navigation effect happened this turn (the
    /// shell suppresses a link's default action). `window.open` effects do NOT
    /// count — they act on *other* browsing contexts (plan §6 / the content
    /// drain's `route_window_opens` contract).
    pub own_context_action: bool,
    /// An apply body (a navigation or a traversal) already shipped its display
    /// list, so the coordinator's end-of-turn [`DrainHost::ship_frame`] is
    /// suppressed (no redundant double-send).
    pub shipped: bool,
}

/// The shell-specific seams the [`DrainCoordinator`] drives — the hooks the two
/// shells diverge on (Slice-0 assessment). Implementing this keeps
/// `ContentState` / `InteractiveState` / the pipeline / `EcsDom` **behind the
/// trait**: the coordinator owns the phase *ordering* + the §4.5 I1/I2/I3
/// invariants; the host owns the irreducibly shell-specific *bodies* (pipeline
/// rebuild, frame shipping, network) and the [`TraversalQueue`] state
/// (§7.3.1.1's traversable owns its queue).
pub trait DrainHost {
    /// Access the host's [`TraversalQueue`] (living near its
    /// [`NavigationController`](crate::NavigationController)). The coordinator
    /// partitions into it (Phase 1) and drains it (Phase 2) through this seam, so
    /// the queue state never leaves the host.
    fn traversal_queue(&mut self) -> &mut TraversalQueue;

    /// **Phase 1a** — drain the `window.open` back-channel and route each intent
    /// (WHATWG HTML §7.2.2.1): tab creation / named-frame nav / drop. Drained
    /// FIRST so an own-context navigation cannot strand queued opens (they live
    /// on the old pipeline's runtime). Shell-specific; its own frame-ship (if
    /// any) is orthogonal to [`DrainOutcome::own_context_action`].
    fn route_window_opens(&mut self);

    /// Drain this turn's staged [`HistoryAction`]s in issue order (the VM
    /// `pending_history` FIFO). The coordinator partitions the result per plan
    /// §4.5 I2; the VM staging model is unchanged (Q-VM-MODEL).
    fn take_pending_history(&mut self) -> Vec<HistoryAction>;

    /// Apply ONE [`HistoryAction`] against the session history — a synchronous
    /// `pushState` / `replaceState` *update* in Phase 1 (§7.4.4), or a deferred
    /// `SyncUpdate` step in Phase 2. Mirrors the shells' existing
    /// `handle_history_action`. A synchronous update does NOT ship its own frame
    /// (the coordinator ships once at end); it must NOT peek/commit the cursor.
    fn handle_history_action(&mut self, action: &HistoryAction);

    /// **Phase 1c** — drain + apply the last-wins own-context navigation
    /// (`pending_navigation`, §7.4.2). Returns `true` iff a navigation applied
    /// (replaced the pipeline **and** shipped its own frame).
    fn handle_navigation(&mut self) -> bool;

    /// **Phase 2** — apply ONE deferred [`PendingTraversal`] (§7.4.6.1 *apply the
    /// history step*). Called **inside** the nested-apply guard bracket (plan
    /// §4.5 I3), so a reentrant nav-mutating message arriving during this call
    /// must consult [`TraversalQueue::is_applying`] and
    /// [`enqueue_traversal`](TraversalQueue::enqueue_traversal) (serialize) rather
    /// than mutate the cursor. The peek→commit atomicity of the underlying
    /// [`NavigationController`](crate::NavigationController) is thereby structural.
    /// Returns `true` iff the traversal shipped its own frame (a rebuild or
    /// same-document apply).
    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> bool;

    /// Ship the current display list / frame (shell-specific). Called once by the
    /// coordinator iff an own-context effect happened but no apply body already
    /// shipped (a pure sync-update turn) — the shells' "history-only turn renders
    /// + returns true" tail.
    fn ship_frame(&mut self);
}

/// The shared **drain-coordinator** — the stateless phase-partition driver. It
/// owns the §4.5 I1/I2/I3 *ordering* + *guard* invariants; the per-turn queue
/// state lives on the host (§7.3.1.1's traversable owns its queue), reached
/// through [`DrainHost::traversal_queue`].
///
/// Slices 2/3 adopt this by implementing [`DrainHost`] on each shell and calling
/// [`DrainCoordinator::drain`] where the shell runs its synchronous drain today.
#[derive(Clone, Copy, Debug, Default)]
pub struct DrainCoordinator;

impl DrainCoordinator {
    /// Run one full drain pass over `host`, honoring the plan §4.5 invariants:
    ///
    /// - **I1 (ordering).** Phase-1 synchronous writes (`pushState` /
    ///   `replaceState` / `location.*`) complete **before** any Phase-2 traversal
    ///   apply reads the entry list. Structural here: [`apply_traversal`] is
    ///   invoked only after the Phase-1 loop + [`DrainHost::handle_navigation`]
    ///   return. (In a
    ///   real async shell the Phase-2 drain is pumped on a *later* turn; app-mode
    ///   realizes the same ordering by draining Phase 2 at end-of-handler,
    ///   strictly after Phase 1 — Slice 3's sequencing contract.)
    /// - **I2 (partition).** The issue-ordered history FIFO is partitioned
    ///   sync-in-task / traversal-deferred **without reordering**: only the
    ///   *prefix* of synchronous updates issued **before** the first traversal
    ///   runs in Phase 1; from the first traversal onward every step defers (in
    ///   issue order) onto the [`TraversalQueue`]. A trailing sync update never
    ///   jumps ahead of an earlier traversal ("all sync first" is NOT the model).
    /// - **I3 (guard bracket).** The [`TraversalQueue`]'s "running nested apply
    ///   history step" boolean (observable via
    ///   [`TraversalQueue::is_applying`]) is set **before** each traversal apply
    ///   and cleared **after** it, covering the whole peek→commit window; a
    ///   message serialized mid-apply is **eventually drained** (the Phase-2 loop
    ///   re-checks the queue until empty).
    ///
    /// [`apply_traversal`]: DrainHost::apply_traversal
    #[must_use]
    pub fn drain<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = DrainOutcome::default();

        // Phase 1a — window.open effects (§7.2.2.1), other-context, drained first.
        host.route_window_opens();

        // Phase 1b — partition the issue-ordered History FIFO (I2). Sync updates
        // (§7.4.4) issued BEFORE any traversal apply in-task; from the first
        // traversal (§7.4.3) onward, every step defers onto the queue in issue
        // order (never reorder a sync ahead of a traversal issued before it).
        let mut seen_traversal = false;
        for (issue_order, action) in host.take_pending_history().into_iter().enumerate() {
            match TraversalDelta::from_history_action(&action) {
                Some(delta) => {
                    seen_traversal = true;
                    host.traversal_queue().enqueue_traversal(PendingTraversal {
                        delta,
                        issue_order,
                        // Slice 1 defaults the §7.4.3 step-2 snapshot (Q-VM-MODEL —
                        // the VM staging carries no involvement fact); Slices 2/3
                        // thread the real issue-time value.
                        user_involvement: UserInvolvement::default(),
                    });
                }
                None if seen_traversal => {
                    // A synchronous update issued AFTER a same-turn traversal —
                    // defer it (tagged) so it cannot jump ahead (I2).
                    host.traversal_queue().enqueue_sync_update(action);
                }
                None => {
                    // Phase-1 synchronous update (§7.4.4), applied in the current
                    // task — does NOT ship its own frame (coordinator ships once).
                    host.handle_history_action(&action);
                    outcome.own_context_action = true;
                }
            }
        }

        // Phase 1c — last-wins own-context navigation (§7.4.2), in-task. Runs
        // regardless of a deferred traversal: the supersede-`return` the shells
        // use today is REMOVED — the traversal defers to Phase 2 (plan §4.2).
        if host.handle_navigation() {
            outcome.own_context_action = true;
            outcome.shipped = true;
        }

        // Phase 2 — deferred traversal apply (a later task, §7.4.6.1), guarded by
        // the nested-apply boolean with re-check-until-empty (I3 eventual drain).
        Self::drain_traversal_queue(host, &mut outcome);

        // Ship once iff an own-context effect happened and no apply body shipped
        // (a pure sync-update turn) — the shells' history-only render tail.
        if outcome.own_context_action && !outcome.shipped {
            host.ship_frame();
            outcome.shipped = true;
        }

        outcome
    }

    /// The Phase-2 deferred drain (plan §4.5 I3). Pops steps in issue order,
    /// bracketing each traversal apply in the nested-apply guard, and **re-checks
    /// until empty** so a step serialized mid-apply (a reentrant SW-pump message)
    /// drains before returning rather than stranding until the next turn.
    fn drain_traversal_queue<H: DrainHost>(host: &mut H, outcome: &mut DrainOutcome) {
        while let Some(step) = host.traversal_queue().pop_next() {
            match step {
                PendingHistoryStep::Traversal(traversal) => {
                    // I3 guard bracket: set BEFORE the peek (inside `apply_traversal`),
                    // clear AFTER the commit. A reentrant message arriving in-bracket
                    // is serialized onto the queue (drained by a later iteration of
                    // this loop), never applied under the held peek.
                    host.traversal_queue().running_nested_apply_history_step = true;
                    let shipped = host.apply_traversal(&traversal);
                    host.traversal_queue().running_nested_apply_history_step = false;
                    outcome.own_context_action = true;
                    outcome.shipped |= shipped;
                }
                PendingHistoryStep::SyncUpdate(action) => {
                    // A deferred synchronous update (issued after a same-turn
                    // traversal) — apply in issue order; no cursor peek/commit, so
                    // no guard bracket.
                    host.handle_history_action(&action);
                    outcome.own_context_action = true;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "traversal_queue_tests.rs"]
mod tests;
