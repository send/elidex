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
//! Slice A co-designs the substrate with its first consumer (content mode) — the
//! peek-classify (`classify_traversal`), nav-suppression (`handle_navigation`
//! drain-and-discard), and deferred-`SyncUpdate` cancellation (Phase 2 drops a
//! straddle sync behind ANY traversal, Resolution D generalized) seams are each
//! designed **correct against the real shell state** the inert substrate lacked
//! (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md`).
//! The isolation unit tests below still pin the coordinator in isolation; content
//! mode drives it (`content/navigation.rs`). App mode = Slice B.
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
//!   appended to the [`TraversalQueue`] and applied *after* Phase 1's updates have
//!   landed, realizing §7.4.6.1 *apply the history step* step 12's two-part split
//!   ("synchronous navigations processed before documents unload").
//!
//! The two phases are **separately callable** so the shell can realize the task
//! boundary: [`DrainCoordinator::drain_synchronous_phase`] runs Phase 1 (window-
//! opens + sync updates + last-wins navigation) and enqueues traversals **without
//! applying them**; [`DrainCoordinator::run_deferred_traversals`] runs Phase 2
//! (the deferred traversal apply) on a **later turn** — content-mode schedules it
//! on a subsequent async-pump turn, app-mode drains it at end-of-input-handler,
//! strictly after Phase 1 (Slices 2/3). [`DrainCoordinator::drain_same_turn`] is a
//! **same-turn convenience** that combines both phases in one call (the app-mode-
//! degenerate path + the isolation tests); adopting it wholesale would collapse
//! the very task boundary this substrate exists to remove, so content-mode drives
//! the two entry points separately (see each method's doc).
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
/// §7.4.6.1). Carries the resolved [`TraversalDelta`] and the §7.4.3 **step-2
/// [`UserInvolvement`]** input captured at issue time.
///
/// The *fuller* §7.4.3 steps-1–3 **source snapshot** (source document / initiator
/// — consumed by §7.4.6.1 for the sandbox check and cross-document target
/// population) is **NOT** captured here: it references the shell's document
/// identity, a type the engine-agnostic substrate does not have. Slice 2/3 threads
/// it at wire time (the same document-identity boundary as a deferred `SyncUpdate`,
/// Codex PR#464 R3-D → slot `#11-sync-navigation-steps-queue-tagging`), so until
/// then a deferred traversal's apply must read live document state, not an
/// issue-time source snapshot. Only the `Copy` `UserInvolvement` input (no shell
/// type) is capturable in Slice 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingTraversal {
    /// The resolved traversal delta (`Back` / `Forward` / `Go(delta)`).
    pub delta: TraversalDelta,
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

    /// Whether the queue holds a pending **traversal** step (ignoring any
    /// `SyncUpdate`-only steps) — the ONE shared default-suppression signal
    /// (plan §1 B / Resolution E). Consulted by BOTH the coordinator's Phase-1c
    /// nav-suppression decision (drain-and-discard a same-turn `location.*` while
    /// a traversal is pending — §7.4.2.2 step 19 "ignored") AND the content
    /// shell's `<a href>`-default suppression site. Cross-turn-robust by
    /// construction: a Turn-1 traversal still queued in Turn-2 (Phase 2 not yet
    /// pumped) is seen, so the default is suppressed until the traversal applies
    /// (plan §1 E1). Resolution E's peek-classify guarantees a no-op `go(999)`
    /// never leaves a `Traversal` step here, so it does not over-suppress.
    #[must_use]
    pub fn has_pending_traversal(&self) -> bool {
        self.pending
            .iter()
            .any(|step| matches!(step, PendingHistoryStep::Traversal(_)))
    }

    /// Pop the next deferred step in issue order (the Phase-2 drain cursor).
    fn pop_next(&mut self) -> Option<PendingHistoryStep> {
        self.pending.pop_front()
    }

    /// Number of deferred steps pending — the **bounded-snapshot size** the
    /// Phase-2 drain captures at drain-start so it processes only the steps that
    /// were already queued, terminating by construction even if an apply
    /// re-enqueues (plan §1 loop-bound / Codex PR#469 R3 T1).
    fn pending_len(&self) -> usize {
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
    fn enter_nested_apply(&mut self) {
        self.running_nested_apply_history_step = true;
    }

    /// Exit the nested-apply bracket (clear the guard after the commit). See
    /// [`Self::enter_nested_apply`].
    fn exit_nested_apply(&mut self) {
        self.running_nested_apply_history_step = false;
    }
}

/// The summary of one [`DrainCoordinator::drain_same_turn`] pass — mirrors the shells'
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
    /// Whether the shell must **suppress a caller's fallback/default action** this
    /// turn — an `<a href>` default navigation (click path) or a keyboard turn's
    /// own render. Computed ONCE at the end of [`drain_synchronous_phase`] as
    /// `own_context_action || <the queue holds a pending `Traversal` step>` (plan
    /// §1 B/E1), so the "own-context effect OR a pending traversal supersedes"
    /// rule has a **single home** and both content call sites read one field
    /// rather than re-deriving the queue query. Cross-turn-robust: a Turn-1
    /// traversal still queued in Turn-2 keeps this `true` until Phase 2 drains it;
    /// Resolution E guarantees a no-op `go(999)` leaves no `Traversal` step, so it
    /// never over-suppresses a legitimate default.
    ///
    /// [`drain_synchronous_phase`]: DrainCoordinator::drain_synchronous_phase
    pub suppress_default: bool,
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

    /// **Phase 1b — peek-classify** a `Back` / `Forward` / `Go` delta against the
    /// host's live entry list (plan §1 Resolution E). Returns `Some(PendingTraversal)`
    /// for an **in-range** traversal — the host resolves the delta via
    /// [`NavigationController::peek_back`](crate::NavigationController::peek_back)
    /// etc. and fills the §7.4.3 step-2 [`UserInvolvement`] (scripted =
    /// [`UserInvolvement::None`]) — or `None` for a **no-op** (out-of-range,
    /// §7.4.3 sub-step 4.4 "does not exist ⇒ abort"). The coordinator makes an
    /// `Some` a partition **barrier** (enqueue + `seen_traversal`) and lets a
    /// `None` **fall through** (no barrier — trailing same-turn sync/nav stay
    /// in-task), so a no-op `go(999)` neither defers a trailing `pushState` nor
    /// suppresses a same-turn navigation. Moving `PendingTraversal` construction
    /// here (out of the coordinator) is what lets the host supply real
    /// involvement + the in-range decision the engine-agnostic layer cannot make.
    ///
    /// Only the **first** traversal of a turn uses this peek-gated form (to decide
    /// whether it STARTS a barrier). Once a barrier exists, every subsequent
    /// traversal enqueues via [`pending_traversal`](Self::pending_traversal)
    /// unconditionally (F4) — so an impl should keep this equal to
    /// `self.peek_delta(delta).map(|_| self.pending_traversal(delta))` (the peek
    /// decides `Some`/`None`; `pending_traversal` builds the value).
    fn classify_traversal(&mut self, delta: TraversalDelta) -> Option<PendingTraversal>;

    /// **Phase 1b — construct a pending traversal WITHOUT a peek** (plan §1 F4).
    /// Once a partition barrier already exists this turn — an earlier in-range
    /// traversal, a still-queued cross-turn traversal, or an in-flight apply
    /// ([`TraversalQueue::is_applying`]) — every subsequent `Back`/`Forward`/`Go`
    /// must enqueue **unconditionally**: its target is resolved at *apply* time
    /// (§7.4.6.1 *apply the history step*), NOT against the still-unmoved cursor at
    /// enqueue time. Peek-classifying a later traversal against the pre-traversal
    /// entry list wrongly **drops** one whose target only becomes in-range after an
    /// earlier queued traversal applies: from `[base, /a]` at `/a`,
    /// `history.back(); history.forward()` — `back()` enqueues (in-range), but
    /// `forward()` peeked against the STILL-UNMOVED index-1 cursor (len 2) resolves
    /// to index 2 → out-of-range → dropped, so Phase 2 lands on `base` instead of
    /// re-applying `forward()` back to `/a`.
    ///
    /// This builds the [`PendingTraversal`] (delta + the host-supplied
    /// [`UserInvolvement`]) with NO peek; [`classify_traversal`] is its peek-gated
    /// form used only for the FIRST traversal (to decide whether it STARTS a
    /// barrier — a no-op first `go(999)` must NOT become one, Resolution E).
    ///
    /// [`classify_traversal`]: Self::classify_traversal
    fn pending_traversal(&mut self, delta: TraversalDelta) -> PendingTraversal;

    /// Apply ONE [`HistoryAction`] against the session history — a synchronous
    /// `pushState` / `replaceState` *update* in Phase 1 (§7.4.4), or a deferred
    /// `SyncUpdate` step in Phase 2. Mirrors the shells' existing
    /// `handle_history_action`. A synchronous update does NOT ship its own frame
    /// (the coordinator ships once at end); it must NOT peek/commit the cursor.
    fn handle_history_action(&mut self, action: &HistoryAction);

    /// **Phase 1c** — drain the last-wins own-context navigation slot
    /// (`pending_navigation`, §7.4.2). Returns `true` iff a navigation applied
    /// (replaced the pipeline **and** shipped its own frame).
    ///
    /// **`suppress` is drain-and-DISCARD, not skip (plan §1 A / F1).** When a
    /// same-turn (or cross-turn still-queued) in-range traversal is pending, the
    /// coordinator passes `suppress = true`: the impl MUST still drain the VM
    /// `pending_navigation` slot (its only drain) but **drop** the request
    /// without applying, returning `false`. Skipping the drain would strand the
    /// slot so the suppressed `location.*` fires **a turn late** (a spurious
    /// deferred nav). This matches §7.4.2.2 step-19 "ignored" (= discarded, not
    /// deferred) — a navigation issued while a traversal is ongoing is dropped.
    fn handle_navigation(&mut self, suppress: bool) -> bool;

    /// **Phase 2** — apply ONE deferred [`PendingTraversal`] (§7.4.6.1 *apply the
    /// history step*). Called **inside** the nested-apply guard bracket (plan
    /// §4.5 I3), so a reentrant nav-mutating message arriving during this call
    /// must consult [`TraversalQueue::is_applying`] and
    /// [`enqueue_traversal`](TraversalQueue::enqueue_traversal) (serialize) rather
    /// than mutate the cursor. The peek→commit atomicity of the underlying
    /// [`NavigationController`](crate::NavigationController) is thereby structural.
    ///
    /// Returns `true` iff the traversal applied AND shipped its own frame (a
    /// rebuild or same-document apply). A **no-op traversal** — no-target (e.g.
    /// `history.go(999)` with no entry at the resolved step, or a stacked
    /// `back(); back()` whose cursor already moved), or a failed cross-document
    /// load — returns `false`, so the coordinator marks NO own-context action and
    /// the caller's fallback/default is not suppressed (mirrors
    /// [`handle_navigation`](Self::handle_navigation)).
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
/// Slices 2/3 adopt this by implementing [`DrainHost`] on each shell and driving
/// the two phases via [`DrainCoordinator::drain_synchronous_phase`] (in-task) +
/// [`DrainCoordinator::run_deferred_traversals`] (a later turn) — the seam that
/// realizes the task boundary. [`DrainCoordinator::drain_same_turn`] is the same-turn
/// convenience combining both (the app-mode-degenerate path + the isolation tests).
pub struct DrainCoordinator;

impl DrainCoordinator {
    /// The **Phase-1 body** — the synchronous, in-task work — over `host`, with
    /// **NO ship logic**: window-opens (§7.2.2.1) → synchronous history *updates*
    /// (§7.4.4 *URL and history update steps*) → last-wins own-context navigation
    /// (§7.4.2). A `Back` / `Forward` / `Go` *traversal* (§7.4.3) is **enqueued**
    /// onto the [`TraversalQueue`] but **NOT applied**. Returns the raw Phase-1
    /// [`DrainOutcome`]; the caller ([`drain_synchronous_phase`] /
    /// [`drain_same_turn`](Self::drain_same_turn)) applies the single shared
    /// [`ship_if_needed`] tail.
    ///
    /// Separating the body from the ship is what makes shipping a **single shared
    /// decision** (`ship_if_needed`) regardless of whether Phase 2 runs on this
    /// turn or a later one: Phase 1's own-context effect (a `pushState` render)
    /// must ship on Phase 1's turn even when a traversal is also queued for a
    /// *later* turn — the earlier bug gated Phase-1's ship on an empty queue and a
    /// `pushState + no-op-traversal` turn stranded the committed frame (neither
    /// phase shipped it).
    ///
    /// Honors the plan §4.5 invariants that belong to Phase 1:
    ///
    /// - **I1 (ordering).** Phase-1 synchronous writes complete **before** any
    ///   Phase-2 traversal apply reads the entry list — enforced structurally by
    ///   this body NOT running Phase 2 (the caller sequences the two entry
    ///   points).
    /// - **I2 (partition).** The issue-ordered history FIFO is partitioned
    ///   sync-in-task / traversal-deferred **without reordering**: only the
    ///   *prefix* of synchronous updates issued **before** the first traversal
    ///   runs in Phase 1; from the first traversal onward every step defers (in
    ///   issue order) onto the [`TraversalQueue`]. A trailing sync update never
    ///   jumps ahead of an earlier traversal ("all sync first" is NOT the model).
    ///
    /// [`drain_synchronous_phase`]: Self::drain_synchronous_phase
    /// [`ship_if_needed`]: Self::ship_if_needed
    fn run_synchronous_phase_body<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = DrainOutcome::default();

        // Phase 1a — window.open effects (§7.2.2.1), other-context, drained first.
        host.route_window_opens();

        // Phase 1b — partition the issue-ordered History FIFO (I2). Sync updates
        // (§7.4.4) issued BEFORE any traversal apply in-task; from the first
        // traversal (§7.4.3) onward, every step defers onto the queue in issue
        // order (never reorder a sync ahead of a traversal issued before it).
        //
        // Seed `seen_traversal` from whether a barrier ALREADY exists coming into
        // this turn: the queue holds a pending *traversal* (a prior turn's
        // `drain_synchronous_phase` enqueued one this turn's Phase 2 has not yet
        // drained — the single-FIFO ordering (I2) holds ACROSS turns), OR a
        // traversal apply is currently IN FLIGHT (`is_applying()` — Phase 1 was
        // re-entered reentrantly DURING Phase 2, so the in-flight traversal has
        // been POPPED off the pending queue but still owns the peek→commit window;
        // F1). A fresh sync update this turn must NOT overtake either — it defers
        // onto the queue (drained by a later Phase-2 bounded-snapshot pass). The
        // barrier concept is a *Traversal* being pending OR in flight (not merely a
        // non-empty queue): a `SyncUpdate`-only queue must NOT seed the barrier,
        // consistent with the Phase-1c suppress predicate. (Empty / sync-only queue
        // with no in-flight apply = the common case = `false`.)
        let mut seen_traversal =
            host.traversal_queue().has_pending_traversal() || host.traversal_queue().is_applying();
        for action in host.take_pending_history() {
            match TraversalDelta::from_history_action(&action) {
                Some(delta) => {
                    // A `Back`/`Forward`/`Go`. The FIRST traversal peek-classifies
                    // against the host's live entry list (Resolution E): only an
                    // IN-RANGE traversal STARTS a partition barrier; a no-op (peek →
                    // `None`) falls through WITHOUT flipping `seen_traversal`, so
                    // subsequent same-turn sync updates + the nav still drain
                    // in-task. Once a barrier exists, every SUBSEQUENT traversal
                    // enqueues UNCONDITIONALLY (F4) — its target resolves at apply
                    // time (§7.4.6.1), so peeking it against the still-unmoved
                    // cursor would wrongly DROP one that only becomes in-range after
                    // an earlier queued traversal applies (`back(); forward()`).
                    if seen_traversal {
                        let pending = host.pending_traversal(delta);
                        host.traversal_queue().enqueue_traversal(pending);
                    } else if let Some(pending) = host.classify_traversal(delta) {
                        seen_traversal = true;
                        host.traversal_queue().enqueue_traversal(pending);
                    }
                    // else: a no-op FIRST traversal (peek → `None`) — not a barrier.
                }
                None if seen_traversal => {
                    // A synchronous update issued AFTER a same-turn traversal —
                    // defer it (tagged, in issue order) so it cannot jump ahead
                    // (I2). Phase 2 CANCELS it once the barrier traversal applies
                    // (Resolution D generalized — a straddle `SyncUpdate` is dropped,
                    // not applied against the post-traversal cursor). Enqueued here
                    // then canceled in `drain_traversal_queue` — the single
                    // cancellation home, uniform across same-turn and cross-turn
                    // straddles. The correct §7.4.1.3 jump-the-queue application to
                    // the CALL-TIME entry is fenced to
                    // `#11-sync-navigation-steps-queue-tagging`.
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

        // Phase 1c — last-wins own-context navigation (§7.4.2), in-task. The
        // supersede-`return` the shells used today is REMOVED, BUT when a traversal
        // is pending (this turn, still-queued cross-turn) OR a traversal apply is
        // IN FLIGHT (`is_applying()` — a reentrant Phase 1 nested inside Phase 2,
        // F1) the navigation is SUPPRESSED: drain-and-DISCARD the
        // `pending_navigation` slot so it cannot re-fire a turn late (§7.4.2.2 step
        // 19 "ignored"; plan §1 A / F1). No-ops never enqueue a `Traversal` step
        // (Resolution E), so they never suppress.
        let suppress =
            host.traversal_queue().has_pending_traversal() || host.traversal_queue().is_applying();
        if host.handle_navigation(suppress) {
            outcome.own_context_action = true;
            outcome.shipped = true;
        }

        // The single home for the default-suppression rule (plan §1 B/E1 + F1): an
        // own-context effect happened this turn OR a `Traversal` step is pending
        // (this-turn or still-queued cross-turn) OR a traversal apply is in flight.
        // `handle_navigation` never enqueues a traversal, so `suppress` (read just
        // above) still reflects the queue's Traversal-pending / in-flight state.
        // Both content call sites read this field instead of re-deriving the query.
        outcome.suppress_default = outcome.own_context_action || suppress;

        outcome
    }

    /// The **single shared ship decision** (plan §4.5 ship-once): ship exactly one
    /// frame iff an own-context effect happened this pass and no apply body already
    /// shipped its own. Every entry point ([`drain_synchronous_phase`] /
    /// [`run_deferred_traversals`] / [`drain_same_turn`]) funnels its trailing ship through
    /// here, so the decision cannot fragment into per-phase guards whose
    /// intersection strands a legitimate frame.
    ///
    /// [`drain_synchronous_phase`]: Self::drain_synchronous_phase
    /// [`run_deferred_traversals`]: Self::run_deferred_traversals
    /// [`drain_same_turn`]: Self::drain_same_turn
    fn ship_if_needed<H: DrainHost>(host: &mut H, outcome: &mut DrainOutcome) {
        if outcome.own_context_action && !outcome.shipped {
            host.ship_frame();
            outcome.shipped = true;
        }
    }

    /// Run **Phase 1** — the synchronous, in-task work — over `host`, WITHOUT
    /// applying any deferred traversal, then ship Phase 1's own frame. This is the
    /// WHATWG HTML Phase-1 body (`run_synchronous_phase_body`) plus the shared
    /// `ship_if_needed` tail: window-opens (§7.2.2.1) → synchronous history
    /// *updates* (§7.4.4) → last-wins own-context navigation (§7.4.2), enqueuing
    /// each `Back` / `Forward` / `Go` *traversal* (§7.4.3) without applying it. The
    /// caller runs Phase 2 via [`run_deferred_traversals`] **separately**:
    /// content-mode on a later async-pump turn, app-mode at end-of-input-handler,
    /// realizing §7.4.6.1 *apply the history step* step-12's task boundary (plan
    /// §4.5 I1). The caller checks [`TraversalQueue::is_empty`] (via
    /// [`DrainHost::traversal_queue`]) to know whether Phase-2 work is pending.
    ///
    /// **Ships Phase 1's own-context effect on Phase 1's own turn** (own-context
    /// action happened and nothing already shipped) — even when a traversal is
    /// **also** queued for a later turn. In the separated model Phase 2 is a
    /// *later* turn and must NOT be relied on to ship Phase 1's frame; gating this
    /// ship on an empty queue stranded the committed `pushState` frame of a
    /// `pushState + no-op-traversal` turn (neither phase shipped). A pure-sync turn
    /// therefore also ships here.
    ///
    /// [`run_deferred_traversals`]: Self::run_deferred_traversals
    #[must_use]
    pub fn drain_synchronous_phase<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = Self::run_synchronous_phase_body(host);
        Self::ship_if_needed(host, &mut outcome);
        outcome
    }

    /// Run **Phase 2** — apply the deferred traversal(s) queued by
    /// [`drain_synchronous_phase`](Self::drain_synchronous_phase) — as a **later
    /// task**: WHATWG HTML §7.4.6.1 *apply the history step* (plan §4.2). Call
    /// this **after** `drain_synchronous_phase`, on a later turn (content-mode's
    /// async pump) or at end-of-input-handler (app-mode), so the traversal apply
    /// reads the entry list only after Phase 1's updates have landed (I1).
    ///
    /// - **I3 (guard bracket).** The [`TraversalQueue`]'s "running nested apply
    ///   history step" boolean (observable via [`TraversalQueue::is_applying`]) is
    ///   set **before** each traversal apply and cleared **after** it, covering
    ///   the whole peek→commit window. This drain processes a **bounded snapshot**
    ///   of the steps pending at entry (T1 — it terminates by construction even if
    ///   an apply re-enqueues); a step serialized mid-apply is left for the **next**
    ///   `run_deferred_traversals` turn (content mode pumps Phase 2 every event-loop
    ///   turn, so liveness holds via the async pump, not exhaustion).
    ///
    /// Ships a frame iff an own-context effect happened and no apply body already
    /// shipped (the deferred-apply render tail), via the shared
    /// `ship_if_needed`; ship-once is preserved.
    #[must_use]
    pub fn run_deferred_traversals<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = DrainOutcome::default();
        Self::drain_traversal_queue(host, &mut outcome);
        Self::ship_if_needed(host, &mut outcome);
        outcome
    }

    /// The **app-mode-degenerate / atomic same-turn** drain — runs Phase 1
    /// (`run_synchronous_phase_body`) then Phase 2 (`drain_traversal_queue`)
    /// back-to-back and ships **exactly once** at the end. This is the shape
    /// app-mode wants: app-mode has **no task boundary** (plan §4.3 option i /
    /// §4.5 I1), so its end-of-input-handler drain collapses the two phases into a
    /// single synchronous return that renders **one** frame — not a per-phase
    /// frame per turn. It is also the isolation-test convenience.
    ///
    /// **Content-mode does NOT use this path.** Content-mode has a real task
    /// boundary and schedules the two phases across *separate turns* via the split
    /// entry points ([`drain_synchronous_phase`] in-task +
    /// [`run_deferred_traversals`] on the async pump) — Phase 1 ships its own frame
    /// on its turn, Phase 2 ships on a later turn. This same-turn method is the
    /// degenerate collapse of that schedule, not a driver of the split.
    ///
    /// Ship-once is structural: both phase bodies accumulate into one
    /// [`DrainOutcome`] and a single trailing `ship_if_needed` fires at most one
    /// [`DrainHost::ship_frame`]. A `pushState + no-op-traversal` turn accumulates
    /// `own_context_action = true` (the push) with `shipped = false` (the no-op
    /// traversal ships nothing) → the single tail ships the push's frame. A
    /// pure-sync turn ships the push; a navigation turn already shipped so the tail
    /// is a no-op; an empty turn ships nothing. Honors plan §4.5 I1 (Phase 1 before
    /// Phase 2), I2 (Phase-1b partition), and I3 (Phase-2 guard bracket).
    ///
    /// [`drain_synchronous_phase`]: Self::drain_synchronous_phase
    /// [`run_deferred_traversals`]: Self::run_deferred_traversals
    #[must_use]
    pub fn drain_same_turn<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = Self::run_synchronous_phase_body(host);
        Self::drain_traversal_queue(host, &mut outcome);
        Self::ship_if_needed(host, &mut outcome);
        outcome
    }

    /// The Phase-2 deferred drain (plan §4.5 I3). Pops steps in issue order,
    /// bracketing each traversal apply in the nested-apply guard, over a **bounded
    /// snapshot** of the steps pending at drain-start (plan §1 loop-bound / Codex
    /// PR#469 R3 T1).
    fn drain_traversal_queue<H: DrainHost>(host: &mut H, outcome: &mut DrainOutcome) {
        // BOUNDED SNAPSHOT (Codex PR#469 R3 T1): capture the number of steps
        // pending at drain-start and process ONLY those (`remaining`). A step
        // enqueued DURING this drain — a reentrant SW-pump message serialized onto
        // the back of the queue — is left for the NEXT `run_deferred_traversals`
        // turn rather than drained to exhaustion, so this loop TERMINATES BY
        // CONSTRUCTION: a wired host whose `apply_traversal` re-enqueues on every
        // apply can no longer loop forever and hang the single-writer renderer
        // thread. Content mode pumps Phase 2 every event-loop turn
        // (`event_loop.rs` top-of-loop), so a deferred reentrant step drains on the
        // next turn — liveness is preserved via the async pump, not exhaustion.
        //
        // Slice-4 CARRY (narrowed): the BOUND now lives here; what stays Slice 4 is
        // the FULL canonical reentrant-message *serialization* semantics (§7.3.1.1
        // running-nested-apply guard WIRING for a reentrant DIRECT nav — T4 below),
        // NOT the loop bound. The reachable reentrancy window (an SW-controlled page
        // re-dispatching a nav-mutating `BrowserToContent` from the SW-fetch wait
        // loop DURING a Phase-2 apply) is closed for this slice by the shell's
        // INTERIM buffer-during-apply guard (`content/navigation.rs`
        // `dispatch_or_buffer_reentrant`): while `is_applying()` holds, such a
        // message is buffered, not dispatched, so it cannot mutate the cursor under
        // the held peek. Content's own `apply_traversal` does not re-enqueue (plan §1
        // loop-bound).
        //
        // `traversal_applied` latch (Resolution D — GENERALIZED, Codex PR#469 R6;
        // re-check-gated on `shipped`): once a traversal has MOVED THE CURSOR this
        // drain (same-document apply OR document-changing rebuild — both ship), every
        // subsequent deferred `SyncUpdate` (within this snapshot) is CANCELED. A
        // straddle sync update (`back(); replaceState('/x')`) must NOT apply against
        // the POST-traversal cursor — that lands the update on the traversal target
        // (corrupting the current entry) instead of the call-time entry. The R3 T3
        // call-time-URL capture was a piecemeal patch on the apply-after model (it
        // fixed the URL but not the entry/index); this generalization SUPERSEDES it —
        // the straddle sync is dropped, preserving coherent state (correct cursor +
        // correct current entry), the ONLY divergence being the lost straddle update
        // (bounded, pinned-not-silent). A **failed-load / no-op** barrier does NOT
        // ship (peek-then-commit atomicity: the cursor never moved), so it does NOT
        // set the latch: the still-active document is the call-time entry, and a
        // trailing straddle sync applies coherently there — no jump-the-queue needed
        // (matching the R2 contract `failed_traversal_load_does_not_drop_trailing_history`).
        // The correct §7.4.1.3 "Centralized modifications of session history"
        // jump-the-queue application to the CALL-TIME entry (before a cursor-MOVING
        // traversal moves the cursor) is fenced to
        // `#11-sync-navigation-steps-queue-tagging` (edge-dense — `/elidex-plan-review`
        // mandatory). Monotonic: it never re-clears within a drain.
        let mut remaining = host.traversal_queue().pending_len();
        let mut traversal_applied = false;
        while remaining > 0 {
            remaining -= 1;
            let Some(step) = host.traversal_queue().pop_next() else {
                break; // Queue emptied early (a step was consumed elsewhere) — done.
            };
            match step {
                PendingHistoryStep::Traversal(traversal) => {
                    // I3 guard bracket: set BEFORE the peek (inside `apply_traversal`),
                    // clear AFTER the commit. A reentrant message arriving in-bracket
                    // is serialized (drained on the NEXT pump turn — outside this
                    // bounded snapshot), never applied under the held peek. The
                    // reachable vector is the SW-fetch reentrant message pump: while
                    // this bracket holds, `handle_navigate`'s SW-wait loop consults
                    // `is_applying()` and BUFFERS a re-dispatched nav-mutating message
                    // (`content/navigation.rs` `dispatch_or_buffer_reentrant`, the
                    // shell's INTERIM guard) instead of mutating the cursor between
                    // this peek and its commit. NOTE (T4 → Slice 4): the FULL
                    // canonical serialization — routing EVERY nav-mutating step
                    // (JS traversals + sync updates + direct/chrome/input navigations)
                    // through this queue with per-step apply-time context (issue-order,
                    // call-time URL, cross-turn document-changed cancellation), per
                    // WHATWG HTML §7.4.1.3 *Centralized modifications* + §7.3.1.1 — is
                    // Slice 4 (`/elidex-plan-review` mandatory — edge-dense, I1×I2×I3
                    // intersecting). The interim buffer closes the reachable corruption
                    // window until then.
                    host.traversal_queue().enter_nested_apply();
                    let shipped = host.apply_traversal(&traversal);
                    host.traversal_queue().exit_nested_apply();
                    // A traversal that MOVED THE CURSOR turns any trailing deferred
                    // `SyncUpdate` in this snapshot into a straddle behind it, CANCELED
                    // below (Resolution D generalized, R6). Set only when the traversal
                    // moved the cursor (`shipped` — same-document apply / rebuild both
                    // ship); a failed-load / no-op barrier leaves the cursor on the
                    // call-time entry, so a trailing straddle sync applies coherently
                    // there — no jump-the-queue needed (the §7.4.1.3 jump-the-queue for
                    // the cursor-MOVED straddle remains `#11-sync-navigation-steps-queue-tagging`).
                    // Over-cancelling here (the pre-R6-re-check bug) wrongly dropped a
                    // trailing `pushState`/`replaceState` after a failed cross-document
                    // load — contradicting the R2 contract
                    // `failed_traversal_load_does_not_drop_trailing_history`.
                    traversal_applied |= shipped;
                    // Gate own-context on the apply OUTCOME (mirrors
                    // `handle_navigation`): a no-op traversal (no-target `go(999)` /
                    // failed cross-document load) reports `shipped = false` and marks
                    // NOTHING, so the caller's fallback/default is not over-suppressed.
                    if shipped {
                        outcome.own_context_action = true;
                        outcome.shipped = true;
                    }
                }
                PendingHistoryStep::SyncUpdate(action) => {
                    if traversal_applied {
                        // Resolution D (GENERALIZED, R6) — CANCEL: a `SyncUpdate`
                        // deferred behind ANY same-turn traversal is dropped, not
                        // applied against the post-traversal cursor. Applying it there
                        // would land the update on the traversal target, corrupting
                        // the current entry (`back(); replaceState('/x')` would land
                        // `/x`-current instead of leaving `base` current). Dropping
                        // preserves coherent state; the correct jump-the-queue
                        // application to the call-time entry is fenced to
                        // `#11-sync-navigation-steps-queue-tagging`.
                        continue;
                    }
                    // A deferred synchronous update with no preceding traversal in
                    // this snapshot (a `SyncUpdate`-only tail) — apply in issue order;
                    // no cursor peek/commit, so no guard bracket. In practice a
                    // `SyncUpdate` is only deferred behind a barrier traversal, so this
                    // arm is reached only when no traversal has applied yet.
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
