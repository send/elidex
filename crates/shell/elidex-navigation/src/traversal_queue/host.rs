//! The [`DrainHost`] contract — the shell-specific seams the
//! [`DrainCoordinator`] drives.
//!
//! Implementing it keeps `ContentState` / `InteractiveState` / the pipeline /
//! `EcsDom` **behind the trait**, so no shell type crosses the `elidex-navigation`
//! crate boundary (plan §4.5 "OO→ECS / layer map"). Both shells implement it:
//! `content/drain_host.rs` (Slice A) and `app/drain_host.rs` (Slice B).
//!
//! This is also where the substrate's two **contract-level spec divergences** live
//! — at the contract that binds both shells rather than duplicated at either impl:
//! [`DrainHost::classify_traversal`] (the §7.4.3 sub-steps 4.1–4.4 issue-time
//! hoist, half of a coupled pair) and [`DrainHost::handle_navigation`]
//! (suppression from *enqueue* time, a deliberate divergence from §7.4.2.2
//! step 19). Both are engine-wide and pre-existing, so a reader reaches them from
//! the contract, not from one shell's copy of the predicate.
//!
//! These are **not** the substrate's only divergences: a third is a property of the
//! Phase-2 drain, not of this contract, and lives at the apply site in
//! `coordinator.rs` (a §7.4.4 intent staged by a `popstate` handler mid-drain is
//! not consumed before the next queued traversal runs, against §7.4.6.1 step 14's
//! note). It is the only one still fenced to an open slot
//! (`#11-sync-navigation-steps-queue-tagging`), so enumerating the substrate's
//! divergences means reading both files.
//!
//! [`DrainCoordinator`]: super::DrainCoordinator

use elidex_script_session::HistoryAction;

use super::queue::TraversalQueue;
use super::step::{PendingTraversal, TraversalDelta};

/// The shell-specific seams the [`DrainCoordinator`] drives — the hooks the two
/// shells diverge on (Slice-0 assessment). Implementing this keeps
/// `ContentState` / `InteractiveState` / the pipeline / `EcsDom` **behind the
/// trait**: the coordinator owns the phase *ordering* + the §4.5 I1/I2/I3
/// invariants; the host owns the irreducibly shell-specific *bodies* (pipeline
/// rebuild, frame shipping, network) and the [`TraversalQueue`] state
/// (§7.3.1.1's traversable owns its queue).
///
/// [`DrainCoordinator`]: super::DrainCoordinator
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
    ///
    /// [`DrainOutcome::own_context_action`]: super::DrainOutcome::own_context_action
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
    ///
    /// ⚠ **ISSUE-TIME HOIST of §7.4.3 sub-steps 4.1–4.4 — half of a coupled pair**
    /// (`#11-sync-navigation-steps-queue-tagging`). WHATWG HTML §7.4.3
    /// *Reloading and traversing* ("traverse the history by a delta") step 4
    /// **appends** the traversal steps to the traversable UNCONDITIONALLY:
    /// `allSteps` / `currentStepIndex` / `targetStepIndex` (sub-steps 4.1–4.3) and
    /// the "If `allSteps[targetStepIndex]` does not exist, then abort these steps"
    /// bail-out (sub-step 4.4) all live INSIDE those appended steps, so the spec
    /// evaluates them when the queued steps RUN. This seam evaluates them at
    /// *issue* time instead.
    ///
    /// **The hoist has no reachable divergence today** (webref-verified
    /// 2026-07-26), and the scenario earlier revisions of this note called one is
    /// not. From `[base]` with the cursor on `base`,
    /// `history.back(); history.pushState({}, '', '/x')` peeks index −1 → `None`,
    /// the coordinator DISCARDS the traversal (no barrier, nothing queued), Phase 1
    /// commits `/x`, and the list ends `[base, /x]` with the cursor on `/x`. **The
    /// spec lands in the same place.** §7.4.3 step 4 appends the traversal steps
    /// **T** when `back()` is called; §7.4.4 *Non-fragment synchronous
    /// "navigations"* (*URL and history update steps*) step 13 appends the
    /// *synchronous navigation steps* **S** BEHIND them, and the entries-list
    /// mutation lives only in §7.4.2.3.3 *Fragment navigations* (*finalize a
    /// same-document navigation* step 5.4, "Append targetEntry to targetEntries"),
    /// i.e. inside **S**.
    /// §7.4.1.3 *Centralized modifications of session history* states this for its
    /// own worked example: the synchronous URL change *"does not yet update the
    /// current session history entry, current session history step, or the session
    /// history entries list; those updates cannot be done synchronously, and
    /// instead must be done as part of the queued steps"*, and it resolves the
    /// traversal against *"the current session history step (i.e., 1) plus the
    /// intended delta of −1"* — the PRE-sync-navigation step. So **T** dequeues
    /// first, its 4.1 *get all used history steps* (§7.4.1.4 *Low-level operations
    /// on session history*) walks the still-`[base]` entries list, `targetStepIndex`
    /// is −1, and 4.4 aborts. A spec-faithful dequeue-time implementation drops that
    /// `back()` exactly as this one does.
    ///
    /// **Why the hoist is sound in general, and what it rests on.** The peek is
    /// reached ONLY when the queue holds no pending `Traversal` and no apply is in
    /// flight (`run_synchronous_updates_body` seeds `seen_traversal` from
    /// [`has_pending_traversal`](TraversalQueue::has_pending_traversal) `||`
    /// [`is_applying`](TraversalQueue::is_applying)), and the moment it returns
    /// `Some` the barrier defers every later step of the turn. Everything that could
    /// still grow the entry list before this traversal would dequeue is therefore
    /// issued LATER in the same task — and the spec queues those steps BEHIND
    /// **T** for exactly that reason. Phase 1 applies §7.4.4 updates in-task in
    /// issue order, reproducing the spec queue's relative order step for step, so a
    /// DISCARDED traversal is followed by precisely the spec's post-abort
    /// continuation. The one shape that would break the equivalence is a
    /// `SyncUpdate` step already sitting in the queue AHEAD of a freshly-peeked
    /// first traversal (only a `Traversal` seeds the barrier — a `SyncUpdate` does
    /// not): the spec would run it first and grow the list. That needs a step to
    /// survive a drain, which is the same cross-drain-boundary carry
    /// `DrainCoordinator::drain_traversal_queue` argues
    /// unreachable (content-mode: the interim buffer guard plus a `pending_len()`
    /// snapshot that never splits a `[Traversal, SyncUpdate]` pair; app-mode:
    /// structurally void, no reentrant Phase 1), and it belongs to
    /// `#11-sync-navigation-steps-queue-tagging`.
    ///
    /// **The two issue-time hoists are a COUPLED PAIR — neither moves alone.**
    /// elidex hoists §7.4.3 4.1–4.4 to issue time *and* commits the §7.4.4 update
    /// in-task ([`handle_history_action`](Self::handle_history_action)) where the
    /// spec queues its entries-list mutation. Those are what make each other safe.
    /// Making this seam unconditional turns every traversal into a partition
    /// barrier, which reintroduces the Resolution-E over-suppression the seam exists
    /// to prevent: an out-of-range `go(999)` would enqueue a `Traversal` step,
    /// making [`TraversalQueue::has_pending_traversal`] true, which latches
    /// [`DrainOutcome::suppress_default`] and kills a legitimate `<a href>` default;
    /// in content-mode it would additionally drain-and-DISCARD a same-turn
    /// `location.*`, and it would defer a trailing `pushState` — including the
    /// §7.4.4 steps 3–11 the spec really does run synchronously — onto a later turn.
    /// So moving the classification to apply time has to move the §7.4.4 commit onto
    /// the queue and re-derive `suppress_default` from apply-time state in the SAME
    /// change. That is `elidex-navigation` **behavior** affecting BOTH shells and
    /// couples Resolution E × Resolution B × the I2 partition × apply-time
    /// resolution, so it is edge-dense (`/elidex-plan-review` mandatory) and lands
    /// with the tagged queue rather than on its own; the deferred plan-review owns
    /// the final shape.
    ///
    /// **Engine-wide and pre-existing**, not a property of any one shell: the
    /// identical predicate is `app/drain_host.rs`'s and `content/drain_host.rs`'s
    /// `classify_traversal`, which is why the note lives here at the CONTRACT. It is
    /// the **first**-traversal counterpart of the shape
    /// [`pending_traversal`](Self::pending_traversal)'s doc records for SUBSEQUENT
    /// traversals (F4, `back(); forward()`) — but only the counterpart, not the same
    /// defect: the F4 case IS reachable because an earlier queued traversal really
    /// does move the cursor before the later one applies, whereas nothing can move
    /// it ahead of the FIRST traversal of a turn.
    ///
    /// [`UserInvolvement`]: super::UserInvolvement
    /// [`UserInvolvement::None`]: super::UserInvolvement::None
    /// [`DrainOutcome::suppress_default`]: super::DrainOutcome::suppress_default
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
    /// [`UserInvolvement`]: super::UserInvolvement
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
    /// deferred nav).
    ///
    /// **Spec basis — this is a deliberate DIVERGENCE, NOT an application of
    /// §7.4.2.2 step 19** (webref-verified 2026-07-26; slot
    /// `#11-nav-supersede-window-vs-ongoing-navigation`). Earlier revisions of this
    /// contract cited step 19's *"Any attempts to navigate a navigable that is
    /// currently traversing are ignored"* as the rule being implemented. It is not.
    /// Step 19 gates on *ongoing navigation* == `"traversal"` **evaluated at the
    /// moment `navigate` runs**, and the ONLY thing that sets that value is §7.4.6.1
    /// *Updating the traversable* **step 8.4** (*"Set the ongoing navigation for
    /// navigable to "traversal"."*), inside the APPLY; three sites reset it to null
    /// (the same-document branch — *apply the history step* step 14.10.1, inside the
    /// step-14 *"While completedChangeJobs does not equal totalChangeJobs"* loop; the
    /// pageswap/unload branch — *deactivate a document for a cross-document
    /// navigation* step 5.2, which **precedes** the `pageswap` fire because 5.1 only
    /// *defines* `firePageSwapBeforeUnload` and the event fires inside 5.3's unload;
    /// and the appended session history traversal steps of that algorithm's
    /// view-transition branch), each annotated *"This allows new navigations of
    /// navigable to start, whereas during the traversal they were blocked."*
    /// §7.4.3's **enqueue** sets nothing. So the spec's blocking window is strictly
    /// *during the apply*: a `location.*` issued BEFORE it —
    /// `history.back(); location.assign('/b')` in one handler — never meets step
    /// 19's condition, **whether or not the queued traversal later applies**. elidex
    /// suppresses from **enqueue** time instead, a strict superset of the spec's
    /// window.
    ///
    /// ⚠ **"Strict superset" is a property of TODAY's apply schedule, not a
    /// permanent one.** It holds because the apply is *synchronous and
    /// non-yielding*: once a traversal is enqueued, elidex's suppression window runs
    /// unbroken from enqueue through the apply, so it contains the spec's
    /// during-the-apply window. Under the planned task-queued apply
    /// (`#11-session-history-task-queue-model`) the apply may yield — a nav issued
    /// *during* a yielded apply would be step-19-ignored by the spec while elidex,
    /// which re-derives `suppress` per drain from
    /// [`has_pending_traversal`](TraversalQueue::has_pending_traversal) `||`
    /// [`is_applying`](TraversalQueue::is_applying), may or may not still be
    /// suppressing. Containment then stops being one-directional, so any narrowing
    /// work must re-derive the relation rather than inherit this sentence.
    ///
    /// The divergence is engine-wide and **pre-existing** (Resolution A, PR #469;
    /// `content/drain_host.rs` carries the same rule) and its predicate is shared
    /// with [`DrainOutcome::suppress_default`], so narrowing it is edge-dense and
    /// lands as its own plan-reviewed PR under the slot above — not here.
    ///
    /// [`DrainOutcome::suppress_default`]: super::DrainOutcome::suppress_default
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
