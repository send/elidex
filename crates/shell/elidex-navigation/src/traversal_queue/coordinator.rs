//! The shared **drain-coordinator** — the phase-partition driver — plus the
//! [`DrainOutcome`] summary each of its passes returns.
//!
//! [`DrainCoordinator`] is stateless and owns the plan §4.5 *ordering* + *guard*
//! invariants (I1 Phase-1-before-Phase-2, I2 issue-order-preserving partition, I3
//! the nested-apply bracket) — the ordering that realizes WHATWG HTML §7.4.6.1
//! *Updating the traversable* step 12's two-part split ("synchronous navigations
//! processed before documents unload"); the per-turn state lives on the host's
//! [`TraversalQueue`], reached through [`DrainHost::traversal_queue`]. Its four
//! public entry points differ only in **which phases run and when** — each
//! method's doc names the shell that drives it.
//!
//! [`DrainOutcome`] is defined here, with its sole producer: every one of its
//! fields is computed by the bodies below, and the queue never observes it.
//! `suppress_default` is derived exactly once, at the end of the shared
//! `run_synchronous_phase_body`, so the split
//! [`DrainCoordinator::drain_synchronous_phase`] (content-mode) and the same-turn
//! [`DrainCoordinator::drain_same_turn`] (app-mode) compute it identically.
//!
//! [`TraversalQueue`]: super::TraversalQueue

use super::host::DrainHost;
use super::step::{PendingHistoryStep, TraversalDelta};

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
    /// turn — in practice exactly one consumer per shell: the `<a href>` default
    /// navigation on the click path (`content/event_handlers.rs`,
    /// `app/events.rs::handle_click`). It is deliberately NOT a render gate:
    /// content's keyboard turn keys its own render on `!shipped` (see its comment
    /// there), and app-mode's keyboard turn discards this outcome entirely.
    /// Computed ONCE at the end of [`drain_synchronous_phase`] as
    /// `own_context_action || <the queue holds a pending `Traversal` step>` (plan
    /// §1 B/E1), so the "own-context effect OR a pending traversal supersedes"
    /// rule has a **single home** and both content call sites read one field
    /// rather than re-deriving the queue query. Where a turn's drive ITERATES
    /// (app-mode's quiescence loop), the shell OR-latches the per-iteration values
    /// into the one its consumer reads — the per-call derivation here is unchanged,
    /// and "at most once per turn" is a claim about the CONSUMER, not about how
    /// many times the value is derived. Cross-turn-robust: a Turn-1
    /// traversal still queued in Turn-2 keeps this `true` until Phase 2 drains it;
    /// Resolution E guarantees a no-op `go(999)` leaves no `Traversal` step, so it
    /// never over-suppresses a legitimate default.
    ///
    /// [`drain_synchronous_phase`]: DrainCoordinator::drain_synchronous_phase
    pub suppress_default: bool,
}

/// The shared **drain-coordinator** — the stateless phase-partition driver. It
/// owns the §4.5 I1/I2/I3 *ordering* + *guard* invariants; the per-turn queue
/// state lives on the host (§7.3.1.1's traversable owns its queue), reached
/// through [`DrainHost::traversal_queue`].
///
/// Both shells implement [`DrainHost`]. **Content-mode** drives the two phases
/// separately — [`DrainCoordinator::drain_synchronous_phase`] (in-task) +
/// [`DrainCoordinator::run_deferred_traversals`] (a later pump turn), the seam
/// that realizes the task boundary — plus
/// [`DrainCoordinator::drain_synchronous_updates`] as its top-of-turn settle.
/// **App-mode** has no pump and drives the same-turn
/// [`DrainCoordinator::drain_same_turn`] (the app-mode-degenerate path + the
/// isolation tests) as the **iteration unit of its drive-site quiescence loop** —
/// once per turn in the common case, repeated while the turn's handlers keep
/// staging (`elidex-shell` `app/drain_host/mod.rs`). The loop is a shell schedule
/// policy: this coordinator stays a stateless phase driver, and every invariant
/// below is stated per call.
pub struct DrainCoordinator;

impl DrainCoordinator {
    /// **Phase 1a + 1b body** — window-opens (§7.2.2.1) + the synchronous
    /// history-FIFO partition (§7.4.4 same-document *URL and history update steps*
    /// applied in-task; §7.4.3 traversals **enqueued** onto the [`TraversalQueue`],
    /// not applied), with **NO Phase 1c** (§7.4.2 last-wins own-context navigation)
    /// and **NO ship**. Returns the raw Phase-1a/1b [`DrainOutcome`].
    ///
    /// The shared core of two callers that differ ONLY in whether Phase 1c runs:
    /// - [`run_synchronous_phase_body`] appends Phase 1c → the FULL drain
    ///   ([`drain_synchronous_phase`] / [`drain_same_turn`]).
    /// - [`drain_synchronous_updates`] ships this body as-is → the **top-drain seam**
    ///   that MUST settle only the same-document sync intent (`pending_history`) and
    ///   window-opens, and MUST NOT apply a same-turn cross-document
    ///   `pending_navigation` (that defers to the bottom full drain, so a held input
    ///   dispatched between the two drains hits the pre-navigation document — see
    ///   [`drain_synchronous_updates`]).
    ///
    /// Honors the Phase-1 slice of the plan §4.5 invariants (I1 ordering — this body
    /// runs no Phase 2; I2 partition — the history FIFO defers from the first
    /// traversal onward without reordering).
    ///
    /// [`TraversalQueue`]: super::TraversalQueue
    fn run_synchronous_updates_body<H: DrainHost>(host: &mut H) -> DrainOutcome {
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

        outcome
    }

    /// The **full Phase-1 body** — [`run_synchronous_updates_body`] (1a + 1b) plus
    /// **Phase 1c** (§7.4.2 last-wins own-context navigation, in-task), with **NO
    /// ship logic**. Returns the raw Phase-1 [`DrainOutcome`]; the caller
    /// ([`drain_synchronous_phase`] / [`drain_same_turn`](Self::drain_same_turn))
    /// applies the single shared [`ship_if_needed`] tail.
    ///
    /// Separating the body from the ship is what makes shipping a **single shared
    /// decision** (`ship_if_needed`) regardless of whether Phase 2 runs on this
    /// turn or a later one: Phase 1's own-context effect (a `pushState` render)
    /// must ship on Phase 1's turn even when a traversal is also queued for a
    /// *later* turn — the earlier bug gated Phase-1's ship on an empty queue and a
    /// `pushState + no-op-traversal` turn stranded the committed frame (neither
    /// phase shipped it).
    ///
    /// [`drain_synchronous_phase`]: Self::drain_synchronous_phase
    /// [`ship_if_needed`]: Self::ship_if_needed
    fn run_synchronous_phase_body<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = Self::run_synchronous_updates_body(host);

        // Phase 1c — last-wins own-context navigation (§7.4.2), in-task. The
        // supersede-`return` the shells used today is REMOVED, BUT when a traversal
        // is pending (this turn, still-queued cross-turn) OR a traversal apply is
        // IN FLIGHT (`is_applying()` — a reentrant Phase 1 nested inside Phase 2,
        // F1) the navigation is SUPPRESSED: drain-and-DISCARD the
        // `pending_navigation` slot so it cannot re-fire a turn late (plan §1 A /
        // F1). Suppressing on a *queued* traversal is a deliberate DIVERGENCE from
        // §7.4.2.2 step 19 — whose gate is *ongoing navigation* == "traversal", set
        // only by the §7.4.6.1 step-8.4 APPLY — not an application of it; the full
        // statement lives on the `DrainHost::handle_navigation` contract. No-ops
        // never enqueue a `Traversal` step (Resolution E), so they never suppress.
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
    /// caller runs Phase 2 via [`run_deferred_traversals`] **separately**, on a
    /// later async-pump turn, realizing §7.4.6.1 *apply the history step*
    /// step-12's task boundary (plan §4.5 I1). **This split pair is content-mode's
    /// entry point; app-mode drives neither half** — each iteration of its
    /// end-of-input-handler drive runs both phases inside
    /// [`drain_same_turn`](Self::drain_same_turn).
    /// The caller checks [`TraversalQueue::is_empty`] (via
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
    /// [`TraversalQueue::is_empty`]: super::TraversalQueue::is_empty
    #[must_use]
    pub fn drain_synchronous_phase<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = Self::run_synchronous_phase_body(host);
        Self::ship_if_needed(host, &mut outcome);
        outcome
    }

    /// Drain **Phase 1a + 1b ONLY** — window-opens (§7.2.2.1) + the synchronous
    /// same-document history *updates* (§7.4.4 `pushState`/`replaceState`, applied
    /// in-task; §7.4.3 traversals enqueued, not applied) — then ship Phase 1's own
    /// frame. Deliberately **omits Phase 1c** (§7.4.2 last-wins own-context
    /// `pending_navigation`), unlike the full [`drain_synchronous_phase`].
    ///
    /// **Why the asymmetry (the top-drain seam).** Content-mode's event loop runs
    /// this at the TOP of a pump turn, immediately after `run_deferred_traversals`
    /// applies a deferred traversal — a same-document traversal fires `popstate`
    /// **synchronously**, whose handler may stage a `pushState` (same-document,
    /// `pending_history`) AND/OR a `location.assign` (CROSS-document,
    /// `pending_navigation`). The same-document `pushState` MUST settle here so it is
    /// committed to the entry list before a held nav-mutating message is dispatched
    /// (the :73 property). But a **cross-document** navigation MUST NOT be applied at
    /// the top: `handle_navigation` → a blocking document load rebuilds the pipeline,
    /// so a held `MouseClick`/`KeyDown` dispatched next would hit the WRONG document.
    /// Per WHATWG HTML a `location.assign` completes in a **later task**, so an
    /// already-pending input (an older task) must process against the pre-navigation
    /// document. Deferring Phase 1c to the bottom full [`drain_synchronous_phase`]
    /// (run AFTER the held-message dispatch) realizes that ordering: the input hits
    /// the pre-nav document, and the cross-document nav applies below as a later task.
    ///
    /// Ships via the shared `ship_if_needed` — a `pushState` committed here ships
    /// its Phase-1 frame on this turn exactly as the full drain would.
    ///
    /// [`drain_synchronous_phase`]: Self::drain_synchronous_phase
    #[must_use]
    pub fn drain_synchronous_updates<H: DrainHost>(host: &mut H) -> DrainOutcome {
        let mut outcome = Self::run_synchronous_updates_body(host);
        Self::ship_if_needed(host, &mut outcome);
        outcome
    }

    /// Run **Phase 2** — apply the deferred traversal(s) queued by
    /// [`drain_synchronous_phase`](Self::drain_synchronous_phase) — as a **later
    /// task**: WHATWG HTML §7.4.6.1 *apply the history step* (plan §4.2). Call
    /// this **after** `drain_synchronous_phase`, on a later turn, so the traversal
    /// apply reads the entry list only after Phase 1's updates have landed (I1).
    /// **Content-mode's async pump is its only caller** — app-mode's
    /// end-of-input-handler Phase 2 runs inside
    /// [`drain_same_turn`](Self::drain_same_turn), not here, and its drive-site
    /// quiescence loop iterates THAT, never this, so the only-caller claim holds.
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
    ///
    /// [`TraversalQueue`]: super::TraversalQueue
    /// [`TraversalQueue::is_applying`]: super::TraversalQueue::is_applying
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
    /// **It is app-mode's ITERATION UNIT, not its whole turn.** The drive site
    /// (`elidex-shell` `app/drain_host/mod.rs`) repeats this call plus its own
    /// reinstatement tail until the turn is quiescent, because a `popstate`
    /// handler this call's Phase 2 fires can stage work Phase 1b has already run
    /// past. Everything stated here is therefore per call, and holds unchanged: N
    /// iterations issue at most N `ship_frame`s, and since app-mode's `ship_frame`
    /// is `Window::request_redraw` — which winit coalesces — that is still ONE
    /// frame, with the shell OR-merging the per-iteration outcomes.
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
        // (`event_loop.rs` step-3 `run_deferred_traversals`), so a deferred reentrant step drains on the
        // next turn — liveness is preserved via the async pump, not exhaustion.
        //
        // Slice-4 CARRY (narrowed): the BOUND now lives here; what stays Slice 4 is
        // the FULL canonical reentrant-message *serialization* semantics (§7.3.1.1
        // running-nested-apply guard WIRING for a reentrant DIRECT nav — T4 below),
        // NOT the loop bound. The reachable reentrancy window (an SW-controlled page
        // re-dispatching a nav-mutating `BrowserToContent` from the SW-fetch wait
        // loop DURING a Phase-2 apply) is closed for this slice by the shell's
        // INTERIM buffer-during-apply guard (`content/drain_host.rs`
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
        //
        // ⚠ `traversal_applied` is a PER-DRAIN local (reset each call). A `SyncUpdate`
        // serialized BEHIND an in-flight traversal and left for a LATER drain would lose
        // this context — the next drain (with `traversal_applied` false again) would APPLY
        // it against the post-traversal cursor at the `SyncUpdate` arm below instead of
        // cancelling it (Codex PR#469 R18). That cross-drain-boundary carry is UNREACHABLE
        // in content-mode Slice-A: the interim buffer guard
        // (`content/drain_host.rs::dispatch_or_buffer_reentrant`) buffers EVERY reentrant
        // message while `is_applying()`, so no reentrant Phase-1 drain runs mid-apply, and
        // `pending_len()` counts ALL steps so a Phase-1-enqueued `[Traversal, SyncUpdate]`
        // pair is always captured whole in one snapshot (cancelled here). It is likewise
        // unreachable in app-mode Slice-B — but BY CONSTRUCTION rather than by a guard
        // (`app/drain_host/mod.rs` module doc, plan §4.4): the inline path has no message pump
        // and no SW-wait, so no apply body re-enters `run_synchronous_phase_body` mid-drain
        // and the app-mode R18 carry is structurally VOID, not deferred. App-mode's
        // drive-site quiescence LOOP does not weaken that: it is site-driven and
        // sequential — each iteration begins only after every body of the previous one has
        // RETURNED — so no Phase-1 partition ever runs mid-apply, and a `[Traversal,
        // SyncUpdate]` pair enqueued by one Phase 1b is captured whole by that same
        // iteration's `pending_len()` snapshot and cancelled here. A `SyncUpdate` a
        // `popstate` handler stages DURING the apply is not that carry: it never reaches
        // this queue at all (it lands on the host's channel and is partitioned by the NEXT
        // iteration's Phase 1b), which is the turn-granularity settle the loop exists for.
        // What still lands
        // with the tagged queue (`#11-sync-navigation-steps-queue-tagging`) is the CANONICAL
        // reentrant-Phase-1-under-apply case — a shell that really does re-partition the
        // FIFO mid-apply.
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
                    // (`content/drain_host.rs` `dispatch_or_buffer_reentrant`, the
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
                    // ⚠ KNOWN DIVERGENCE — a §7.4.4 intent staged by a `popstate`
                    // handler THIS apply just fired is NOT consumed before the next
                    // queued traversal runs (`#11-sync-navigation-steps-queue-tagging`,
                    // its R16 multi-traversal-snapshot facet). Such an intent lands on
                    // the host's own pending-history channel — Phase 1b has already
                    // run, so it does not reach `enqueue_sync_update` and Resolution
                    // D's `traversal_applied` cancel below never sees it. The
                    // traversals that follow keep moving the cursor underneath it, and
                    // the NEXT Phase 1b applies it wherever the cursor stopped.
                    // §7.4.6.1 *Updating the traversable* step 14's note requires the
                    // opposite: synchronous navigations "jump the queue … before this
                    // traversal potentially unloads their document", i.e. they settle
                    // against the entry whose handler issued them. Pinned (app-mode)
                    // by `app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`.
                    // NEWLY REACHABLE in app-mode via Slice B, deliberately: the
                    // retired hand-rolled drain returned after the FIRST traversal
                    // (dropping the second — the #259 truncation this slice fixes), so
                    // unlocking multi-traversal application exposes the straddle that
                    // was underneath it. The fix is per-task finalization with
                    // call-time entry association (§7.4.1.3) — edge-dense, its own
                    // plan-reviewed PR.
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
