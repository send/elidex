//! The traversable's **session history traversal queue** (WHATWG HTML §7.3.1.1
//! `#tn-session-history-traversal-queue`) — the deferred-step FIFO plus the
//! "running nested apply history step" reentrancy guard.
//!
//! **State only.** The queue owns the pending [`PendingHistoryStep`]s and the guard
//! boolean; it schedules nothing itself. The **drain cursor** — popping, the
//! snapshot bound, and the nested-apply bracket — is driven by the
//! [`DrainCoordinator`], which reaches this state through
//! [`DrainHost::traversal_queue`] (§7.3.1.1's traversable owns its queue, so the
//! coordinator itself stays stateless). Those four operations are `pub(super)`
//! rather than `pub`, which keeps them off the crate's public surface but does
//! **not** make coordinator-only an enforced invariant: `pub(super)` here means
//! `pub(in crate::traversal_queue)`, so the sibling submodules and this module's
//! test children can reach them too — and one deliberately does
//! (`traversal_queue_drain_tests` drives the nested-apply bracket itself to
//! simulate mid-Phase-2 state). Narrowing them further would break that test.
//!
//! The enqueue side is **`pub`** — the seam a reentrant nav-mutating step is meant
//! to serialize onto rather than apply under a held peek (plan §4.4, stated at
//! [`enqueue_traversal`](TraversalQueue::enqueue_traversal)). **No shell takes that
//! route today**: the coordinator is its only caller, and content-mode's interim
//! guard buffers a reentrant message shell-side instead
//! (`content/drain_host.rs::dispatch_or_buffer_reentrant`), while app-mode is
//! structurally reentrancy-free — its turn-completion loop is **site-driven and
//! sequential** (iterations begin only after the previous one's bodies return), not
//! a reentrant drive, so it neither needs this seam nor weakens that property.
//! Routing every nav-mutating step through this queue
//! is Slice 4 (`#11-session-history-task-queue-model`). What the shells actually
//! call today is [`new`](TraversalQueue::new) (both construct one — `app/mod.rs`,
//! `content/mod.rs`) plus [`is_applying`](TraversalQueue::is_applying) and
//! [`is_empty`](TraversalQueue::is_empty);
//! [`has_pending_traversal`](TraversalQueue::has_pending_traversal) is read by the
//! coordinator and by shell *tests* only, since the shells' default-suppression
//! sites consult [`DrainOutcome::suppress_default`] instead.
//!
//! [`DrainCoordinator`]: super::DrainCoordinator
//! [`DrainHost::traversal_queue`]: super::DrainHost::traversal_queue
//! [`DrainOutcome::suppress_default`]: super::DrainOutcome::suppress_default

use elidex_script_session::HistoryAction;

use super::step::{PendingHistoryStep, PendingTraversal};

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
    ///
    /// [`DrainCoordinator`]: super::DrainCoordinator
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

    /// Whether the queue holds a pending **traversal** step (ignoring any
    /// `SyncUpdate`-only steps) — the ONE shared default-suppression signal
    /// (plan §1 B / Resolution E). Consulted by BOTH the coordinator's Phase-1c
    /// nav-suppression decision (drain-and-discard a same-turn `location.*` while
    /// a traversal is pending — a deliberate **divergence** from §7.4.2.2 step 19,
    /// stated in full at [`DrainHost::handle_navigation`]) AND the content
    /// shell's `<a href>`-default suppression site. Cross-turn-robust by
    /// construction: a Turn-1 traversal still queued in Turn-2 (Phase 2 not yet
    /// pumped) is seen, so the default is suppressed until the traversal applies
    /// (plan §1 E1). Resolution E's peek-classify guarantees a no-op `go(999)`
    /// never leaves a `Traversal` step here, so it does not over-suppress.
    ///
    /// [`DrainHost::handle_navigation`]: super::DrainHost::handle_navigation
    #[must_use]
    pub fn has_pending_traversal(&self) -> bool {
        self.pending
            .iter()
            .any(|step| matches!(step, PendingHistoryStep::Traversal(_)))
    }

    /// Pop the next deferred step in issue order (the Phase-2 drain cursor).
    pub(super) fn pop_next(&mut self) -> Option<PendingHistoryStep> {
        self.pending.pop_front()
    }

    /// Number of deferred steps pending — the **bounded-snapshot size** the
    /// Phase-2 drain captures at drain-start so it processes only the steps that
    /// were already queued, terminating by construction even if an apply
    /// re-enqueues (plan §1 loop-bound / Codex PR#469 R3 T1).
    pub(super) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Enter the WHATWG HTML §7.3.1.1 "running nested apply history step" bracket
    /// (set the guard before the peek). Paired with [`Self::exit_nested_apply`].
    ///
    /// The bracket is a method pair rather than an RAII `Drop` guard because a Drop
    /// guard would have to hold `&mut TraversalQueue` across the
    /// `DrainHost::apply_traversal(&mut host)` call — but the queue lives *on* the
    /// host (host-owns-queue, plan §4.1), so that borrow conflicts. The coordinator
    /// owns the ordering of set→apply→clear (plan §4.5 I3).
    pub(super) fn enter_nested_apply(&mut self) {
        self.running_nested_apply_history_step = true;
    }

    /// Exit the nested-apply bracket (clear the guard after the commit). See
    /// [`Self::enter_nested_apply`].
    pub(super) fn exit_nested_apply(&mut self) {
        self.running_nested_apply_history_step = false;
    }
}
