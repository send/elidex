//! App-mode (legacy inline) realization of the shared [`DrainHost`] drain adapter
//! (`docs/plans/2026-07-session-history-slice-B-app-phase-separation.md` §4).
//!
//! The direct mirror of `content/drain_host.rs`: the `impl DrainHost for App`
//! phase-drain seams plus the one free function that ONLY serves those seams — the
//! Phase-2 traversal-apply body [`apply_traversal_delta`]. The sibling
//! `app/navigation.rs` keeps the shell bodies these seams delegate to (the
//! pipeline-rebuild `navigate` / `navigate_to_history_url` / `load_url_into_pipeline`,
//! the same-document-step primitive, the index-keyed traversal apply `traverse_to`,
//! the §7.4.4 sync-update body `handle_history_action`, and URL normalization) —
//! bodies that also serve the **non-drain** callers (chrome toolbar, Alt+arrow,
//! `<a href>` click), so nothing moved behind the trait.
//!
//! **Both shells now drive the SAME primitive** (One-issue-one-way — the axis-c
//! fork this slice closes). The hand-rolled synchronous app-mode drain — window-open
//! drop → history FIFO with a traversal-supersede `return` → last-wins navigation —
//! is retired: [`App::process_pending_navigation`] is now a thin guard over
//! [`DrainCoordinator::drain_same_turn`].
//!
//! **What differs is the SCHEDULE — and the two schedules are not mirror images.**
//! Content-mode drives the coordinator from FIVE sites, in two groups. Three belong
//! to its async pump, in per-turn order (`content/event_loop.rs`:
//! `run_deferred_traversals` → `drain_synchronous_updates` → the bottom
//! `drain_synchronous_phase`). The other two are IN-input-handler drains
//! (`content/event_handlers.rs`, one per handler, each `drain_synchronous_phase`) —
//! and **those** are the structural counterpart of app-mode's single
//! end-of-input-handler drain: the click one consumes
//! [`suppress_default`](DrainOutcome::suppress_default) as an early return exactly
//! as `app/events.rs::handle_click` does. What app-mode has is that counterpart and
//! nothing else — no pump. The *headline* difference is therefore WHEN Phase 2 is
//! pumped — content-mode on a later async-pump turn
//! ([`run_deferred_traversals`](DrainCoordinator::run_deferred_traversals)),
//! app-mode back-to-back inside the input handler, so its Phase 2 is a
//! *degenerate* later task (Q-SCHED option (i)). Content's *post*-Phase-2 settle —
//! its pump-turn `drain_synchronous_updates` top drain — has an app-mode
//! counterpart of a different shape: with no later pump turn to defer to, the
//! drive site runs [`drain_same_turn`](DrainCoordinator::drain_same_turn) **in a
//! bounded loop until the turn is quiescent**
//! ([`App::process_pending_navigation`]), so what a `popstate` handler stages
//! during Phase 2 settles on the turn that fired it. That
//! degenerate collapse still delivers WHATWG HTML §7.4.6.1 *Updating the
//! traversable* step 12's ordering guarantee for a single top-level traversable
//! ("This set of steps are split into two parts to allow synchronous navigations to
//! be processed before documents unload"); the multi-navigable fan-out that would
//! need a *real* later task is B1-gated
//! (`#11-session-history-task-queue-model`).
//!
//! ## Reentrancy: the vector is DEAD BY CONSTRUCTION in app-mode (plan §4.4)
//!
//! Content-mode's Phase 2 drains a **bounded snapshot** and leans on its every-turn
//! async pump for liveness — a step serialized mid-apply drains next turn. App-mode
//! has no pump, so a mid-apply-serialized step would strand for an UNBOUNDED time:
//! not merely "until the next input event" — the next input event need not drain at
//! all, since `events::handle_click` returns early on a hit-test miss / a chrome-band
//! click / an unset `cursor_pos`, and `events::handle_keyboard` on an unfocused
//! document, all of them BEFORE the drive site is reached. (The dispatch-entry
//! drives [`App::process_pending_navigation`]'s turn-completion loop schedules do
//! **not** bound this shape: they are gated on the VM staging peek
//! [`HostDriver::has_pending_session_history_work`], which reads the engine's
//! channels — a step already serialized onto the [`TraversalQueue`] is invisible to
//! it. That is the exit `debug_assert`'s residual, not the loop's.) It cannot
//! happen here, because the inline path has **no reentrancy
//! vector at all**:
//!
//! 1. This drive runs EXCLUSIVELY on the legacy-inline `InteractiveState` path
//!    ([`App::new_interactive_with_url`]), reached only from
//!    `events::handle_click` / `events::handle_keyboard`. Threaded mode uses a
//!    different method set that messages the content thread, which runs its own
//!    content-mode `DrainHost`.
//! 2. The inline path has NO service-worker machinery: `new_interactive_with_url`
//!    sets `network_process: None` and `origin_storage: None`.
//! 3. Its navigation body issues a DIRECT blocking fetch with no SW hook
//!    (`load_url_into_pipeline` → `elidex_navigation::load_document` →
//!    `fetch_blocking`). Content-mode's SW-fetch **wait loop**, which re-dispatches
//!    `BrowserToContent` messages mid-navigation (guarded by
//!    `content/drain_host.rs::dispatch_or_buffer_reentrant`), is the ENTIRE
//!    content-mode reentrancy vector and is structurally absent here.
//! 4. The app-mode SW facilities (`app/sw_coordinator.rs`, `app/sw_fetch_relay.rs`)
//!    are browser-thread facilities operating over content-thread channels; they
//!    never touch `InteractiveState`.
//! 5. **Root invariant:** *no app-mode apply body synchronously drives the
//!    coordinator's Phase-1 partition* (`run_synchronous_phase_body`). The Phase-2
//!    apply chain ([`apply_traversal_delta`] → `traverse_to` →
//!    `same_document_step` / `navigate_to_history_url`) never calls
//!    [`App::process_pending_navigation`]. A popstate handler, or a freshly-rebuilt
//!    page's initial scripts, may **stage** new history actions onto the VM
//!    `pending_history` FIFO — but staging is not partitioning: those actions reach
//!    the [`TraversalQueue`] only on the NEXT ITERATION of the drive site's
//!    turn-completion loop, never re-entering the current one.
//!
//!    **A site-driven loop is not a re-entry — the distinction this invariant
//!    turns on.** [`App::process_pending_navigation`] repeats whole iterations of
//!    `drain_same_turn` + the reinstatement tail, and each iteration starts only
//!    after every body of the previous one has RETURNED: the iterations are
//!    strictly sequential, so no two partitions are ever interleaved and each
//!    Phase-1b partitions a FIFO holding only what the previous iteration's bodies
//!    staged (issue order, I2). What premise 5 forbids is the *nested* shape — a
//!    seam body / apply body / the tail calling back into the drive **while the
//!    outer drive is still on the stack** — which does interleave two partitions
//!    and silently voids I2 and the Resolution-D latch. So the entry assert below
//!    is unchanged in force by the loop and must not be "fixed" to accommodate it:
//!    nesting trips it, iterating does not. Any future change to an
//!    apply body MUST preserve
//!    this — eagerly re-draining pending nav from inside an apply would re-open the
//!    mid-apply re-enqueue vector. Machine-guarded at the drive site by the
//!    [`App::process_pending_navigation`] `debug_assert` PAIR: the ENTRY assert reads
//!    the host's own [`InteractiveState::drain_in_progress`](super::InteractiveState)
//!    re-entry flag, which brackets the WHOLE drive — every `DrainHost` seam body,
//!    every Phase-2 apply body, and the reinstatement tail — so a re-drive from ANY
//!    of them is caught; the EXIT assert (`is_empty()`) catches a residual step. The
//!    queue's own [`TraversalQueue::is_applying`] would NOT do for the entry assert:
//!    it brackets `apply_traversal` alone, so it is blind to the most natural
//!    regression shape (a re-drain at the end of `navigate`, which app-mode reaches
//!    from Phase 1c).
//!
//! Consequently the bounded snapshot captured at Phase-2 drain-start **equals the
//! entire queue**, the drain is complete-and-terminating by construction, and
//! nothing strands. App-mode therefore adds **no** reentrancy machinery: no
//! `deferred_reentrant_messages`, no `dispatch_or_buffer_reentrant` mirror. The
//! §7.3.1.1 "running nested apply history step" guard the coordinator brackets each
//! apply with is present-but-inert here. (Re-eval trigger, not a current residual:
//! if M4-10 ever wires an SW-fetch relay into the inline navigation path, premises
//! 2/3 break and app-mode inherits Slice 4's canonical DIRECT-nav serialization.)

use elidex_navigation::{
    DrainCoordinator, DrainHost, DrainOutcome, PendingTraversal, TraversalDelta, TraversalQueue,
    UserInvolvement,
};
use elidex_script_session::{HistoryAction, HostDriver};

use super::navigation::{handle_history_action, resolve_nav_url};
use super::App;

/// Maximum iterations of the app-mode turn-completion loop
/// ([`App::process_pending_navigation`]) per drive.
///
/// A page whose `popstate` handler re-stages unconditionally makes the fixpoint
/// unreachable, and this loop runs on the single-writer renderer thread, so an
/// unbounded loop is a hang. Same order (and same degrade shape — `eprintln!` on
/// the final round, work deferred to the next dispatch) as the in-tree
/// `MAX_CE_STABILIZATION_ROUNDS`, and far above any legitimate depth: each round
/// requires the page to have staged NEW work from inside the previous round's
/// handlers.
///
/// The drain-start-snapshot bound the coordinator's Phase 2 uses
/// (`pending_len()`) is deliberately NOT the idiom here: it terminates a drain of
/// *pre-existing* work by excluding work created during it, and consuming exactly
/// that work is this loop's entire purpose — a start-snapshot of the loop is the
/// degenerate "one iteration", i.e. the defect it fixes.
const MAX_TURN_COMPLETION_ROUNDS: usize = 8;

impl App {
    /// Drive the input turn's session-history / navigation work **to quiescence**
    /// — the app-mode leg of the shared phase-partition, and app-mode's
    /// counterpart of content-mode's post-Phase-2 settle.
    ///
    /// Called at the end of an input handler (`events::handle_click` /
    /// `events::handle_keyboard`), after event dispatch + re-render, and at the
    /// three winit dispatch entries (below). Each ITERATION runs
    /// [`DrainCoordinator::drain_same_turn`], whose body sequences **Phase 1**
    /// (window-opens §7.2.2.1 → §7.4.4 synchronous `pushState`/`replaceState`
    /// updates applied in-task, with §7.4.3 `Back`/`Forward`/`Go` traversals merely
    /// *enqueued* → §7.4.2 last-wins own-context navigation) strictly BEFORE
    /// **Phase 2** (the §7.4.6.1 deferred traversal apply), then ships at most one
    /// frame — followed by the **reinstatement tail** below. That call ordering IS
    /// app-mode's realization of the task boundary
    /// (I1, app leg): every Phase-1 write to the entry list lands before any
    /// Phase-2 apply reads it.
    ///
    /// **This is the SOLE method that drives the coordinator in app-mode** — the
    /// `interactive.is_some()` guard here is what makes every per-seam
    /// [`App::inline_state`] / [`App::inline_state_mut`] reach-through an
    /// unreachable panic ([`INTERACTIVE_DRIVE_ONLY`](super::INTERACTIVE_DRIVE_ONLY)).
    /// It has FIVE callers, all top-level arms/targets of the winit dispatch with
    /// no synchronous path from any drain body back into them (so the premise-5
    /// argument is unchanged in substance — only the named call sites grew): the
    /// two in-handler drives (`events::handle_click` `handle_keyboard`) and the
    /// three peek-gated dispatch-entry drives routed through
    /// [`Self::drive_staged_session_history_work`].
    ///
    /// Returns the turn's [`DrainOutcome`] (the shared summary both shells return)
    /// rather than the retired ad-hoc `bool` — the **field-wise OR** of every
    /// iteration's outcome, because the fields describe the TURN, not the last
    /// iteration (`merge_turn_outcome`). Callers read the field they
    /// need: `handle_click` consumes
    /// [`suppress_default`](DrainOutcome::suppress_default) to drop the `<a href>`
    /// default navigation; `handle_keyboard` calls for effect and ignores it; a
    /// dispatch-entry drive DISCARDS it (it settles a *previous* turn's residue, so
    /// merging it into the turn that follows would over-suppress that turn's `<a
    /// href>` default — the same different-turn discard the coordinator already
    /// documents for app-mode's keyboard turn). When
    /// `interactive` is absent (threaded mode) no drain runs and the default
    /// outcome — every field `false`, i.e. "nothing happened, suppress nothing" —
    /// is returned.
    ///
    /// # Why the turn is run to quiescence, and why the unit is a WHOLE iteration
    ///
    /// A §7.4.4 intent staged **during** Phase 2 — canonically a `pushState` from
    /// the `popstate` handler a same-document traversal fires synchronously — lands
    /// on the VM `pending_history` FIFO after Phase 1b has already partitioned, so
    /// ONE `drain_same_turn` returns with the turn unsettled. The harm is not
    /// latency but **wrong-entry mutation**: the non-drain cursor movers (chrome
    /// toolbar Back/Forward and Alt+←/→ call
    /// [`App::traverse_to`](super::App::traverse_to) directly; the `<a href>`
    /// default, the address bar and Reload call
    /// [`App::navigate`](super::App::navigate)) move the cursor without draining
    /// that FIFO, so a later drive applies the staged update against a cursor that
    /// has MOVED — the replace arm overwriting the wrong current entry, the push
    /// arm reaching `push_entry`'s `entries.truncate(current_index + 1)` (the
    /// in-tree image of *finalize a same-document navigation* §7.4.2.3.3 step 5.1
    /// "Clear the forward session history of traversable", invoked from §7.4.4 step
    /// 13.1) and **destroying live forward entries**: correct truncation semantics
    /// applied against the wrong cursor.
    ///
    /// WHATWG HTML §8.1.7.3 *Processing model* gives no warrant for settling that
    /// in-turn: it runs one task (step 2.6) then a microtask checkpoint (2.8), and
    /// §7.4.4 *URL and history update steps* step 13 **appends** the synchronous
    /// navigation steps to the *traversable* — the §7.3.1.1 session history
    /// traversal queue — outside the task. The spec's settle point is §7.4.6.1
    /// *apply the history step* **step 14.1.1**: a bounded drain of staged sync-nav
    /// steps *between* traversal change-jobs, bracketed by `running nested apply
    /// history step`, so that they *"jump the queue at this point … before this
    /// traversal potentially unloads their document"*. The inline shell has no pump
    /// and no parallel queue, so that settle is realized here at **turn
    /// granularity** — everything the turn's handlers staged is settled before
    /// returning to winit. The **traversal-granularity** component (14.1.1's
    /// between-change-jobs placement, which is what lands the intent on the entry
    /// whose handler issued it when more traversals follow in one Phase-2 snapshot)
    /// is `#11-sync-navigation-steps-queue-tagging`, still pinned by
    /// `app_history_phase_sep_tests::app_multi_traversal_snapshot_lands_popstate_staged_update_on_the_wrong_entry`.
    /// Same mechanism, two granularities, two slots.
    ///
    /// The unit is a whole iteration and **NOT** a trailing
    /// [`DrainCoordinator::drain_synchronous_updates`] — the literal transcription
    /// of content-mode's Slice-A fix (Codex #469 R9, `content/event_loop.rs`,
    /// pinned by
    /// `content_history_phase_sep_tests::pump_drains_popstate_staged_pushstate_this_turn`).
    /// That trailing drain is not merely insufficient, it is **wrong**. It would
    /// settle a popstate-staged `pushState`, but a popstate-staged `back()` would be
    /// peek-classified (Resolution E) and left **resident on the [`TraversalQueue`]
    /// across the turn boundary**. Such a step is NOT stranded: the next
    /// `drain_same_turn` seeds
    /// `seen_traversal` from [`TraversalQueue::has_pending_traversal`] and its Phase 2
    /// drains it. What the trailing drain does is
    /// **freeze the in-range classification a turn early**, voiding the queue's own
    /// contract that "Resolution E's peek-classify guarantees a no-op `go(999)` never
    /// leaves a `Traversal` step here, so it does not over-suppress"
    /// ([`TraversalQueue::has_pending_traversal`]) — the **non-drain** cursor movers
    /// run in between, so the resident step can be a
    /// no-op by the next turn while still acting as a FULL barrier: it seeds
    /// `seen_traversal` at Phase-1 **entry** (deferring every fresh `pushState` behind
    /// it) and latches [`suppress_default`](DrainOutcome::suppress_default) **true** at
    /// Phase-1 **exit**, killing an unrelated `<a href>` default for a traversal whose
    /// Phase-2 re-peek then finds it out of range and no-ops. And when the resident
    /// step IS still in range, its apply ships, so the Resolution-D
    /// `traversal_applied` latch **cancels** every `pushState` deferred behind it.
    /// (That last cancel is behavior a parked `back()` leading the VM
    /// `pending_history` FIFO produces anyway, pinned by
    /// `app_history_phase_sep_tests::app_trailing_syncupdate_canceled_behind_cursor_moving_traversal`;
    /// the **over-suppression** is what the trailing drain newly breaks.) It would also
    /// contradict this site's premise-5 exit assert by construction — the queue would
    /// be deliberately non-empty at drain exit.
    ///
    /// Iterating the WHOLE unit keeps every property the trailing drain loses,
    /// *because* the unit is a whole cycle: each traversal is classified and
    /// applied in the same iteration (no frozen classification); each iteration's
    /// Phase 2 empties the queue (the exit assert stays true); and each iteration's
    /// Phase 1b partitions a FIFO holding only what the previous iteration staged,
    /// which IS issue order (I2). The reinstatement tail runs INSIDE the iteration
    /// for the same reason: a navigation held by iteration N's Phase 1c and refuted
    /// by its Phase 2 must apply before iteration N+1 partitions fresh intents,
    /// else a `location.*` issued in N would apply after intents issued in N+1.
    ///
    /// # The three exits, and the one follow-up rule
    ///
    /// (1) **quiescent** — [`HostDriver::has_pending_session_history_work`] reads
    /// false. (2) **cap-hit** — [`MAX_TURN_COMPLETION_ROUNDS`] iterations, for a
    /// handler that re-stages unconditionally
    /// (`onpopstate = () => history.pushState(…)`), which would otherwise hang the
    /// single-writer renderer thread; an adversarial-but-legal page must degrade,
    /// not panic a debug build, so the cap warns via `eprintln!` (the
    /// `MAX_CE_STABILIZATION_ROUNDS` idiom) rather than asserting. (3) **pipeline
    /// swap** — the document marker moved (`current_document_marker`).
    ///
    /// Neither non-quiescent exit records any state: the work stays on the
    /// **surviving** runtime's channels, which is where the peek reads. **Unified
    /// exit rule — any exit whose exit-time peek still reads true requests a
    /// follow-up dispatch** ([`Self::schedule_followup_dispatch`]). One rule covers
    /// the cap and the swap alike, so no exit path depends on a frame being
    /// "already" scheduled — a rebuild does NOT schedule one (`shipped` does not
    /// mean "a frame was requested", `#11-nav-applied-shipped-decouple`; the
    /// app-mode nav bodies request no repaint of their own, see
    /// [`ship_frame`](DrainHost::ship_frame)). Cost is ≈0: winit coalesces
    /// concurrent requests into one `RedrawRequested`.
    ///
    /// That redraw is a *wakeup*, not the correctness argument. What bounds the
    /// residue is structural: the SAME peek is read at the three winit **dispatch
    /// entries** (`app/inline.rs` — the `RedrawRequested` arm,
    /// `handle_keyboard_inline`, `handle_mouse_press_inline`), each driving this
    /// method before anything else in its dispatch and therefore before every
    /// non-drain mover that dispatch can reach (chrome actions run in the redraw
    /// arm's tail; Alt+←/→ in the keyboard dispatch's own branch, which never
    /// reaches `events::handle_keyboard`; the `<a href>` default in
    /// `handle_click`). Residue is therefore **(next-dispatch ∨ frame)-bounded**,
    /// and a click on blank space now drains what it used to strand —
    /// `handle_click`'s early returns are all downstream of the mouse-press entry
    /// drive. Peek-gating rather than flag-gating is load-bearing: staged work has
    /// three sources — this loop's non-quiescent exits, a **mover-fired**
    /// synchronous `popstate`/`hashchange` whose handler stages (that mover never
    /// enters the loop at all), and a fresh document's load-time staging — and a
    /// flag set at loop exits would see only the first, while the channels
    /// themselves answer for all three (One issue, one way: the channels are the
    /// SoT and the peek is their only mirror, so no drive-schedule flag exists to
    /// fall out of sync with them).
    ///
    /// The residual this leaves is bounded and named: within ONE dispatch, an entry
    /// drive that exits non-quiescent (cap re-hit, or the swap exit) followed by a
    /// mover later in that same dispatch. Accepted for this slice and pinned in
    /// `app_turn_completion_tests`; the mover routing that closes it is Slice 4's
    /// (`#11-session-history-task-queue-model`).
    pub(super) fn process_pending_navigation(&mut self) -> DrainOutcome {
        if self.interactive.is_none() {
            return DrainOutcome::default();
        }
        // Plan §4.4 premise 5 ("no app-mode apply body may synchronously drive the
        // coordinator's Phase-1 partition") is what makes Phase 2's bounded snapshot
        // equal the whole queue. It has TWO failure shapes and takes one assert each
        // — the pair is complete, neither alone is.
        //
        // ENTRY (this one) — a **re-drive**: some body this drive runs calls
        // `process_pending_navigation` / `DrainCoordinator::drain_same_turn` itself,
        // the most natural future regression ("just re-drain at the end of
        // `navigate`"). The signal is the host's OWN `drain_in_progress` flag, set
        // immediately below and cleared at exit, because the guard must cover EVERY
        // body — not just an apply body. `TraversalQueue::is_applying()` cannot do
        // that job: the coordinator brackets `enter_nested_apply` /
        // `exit_nested_apply` around `DrainHost::apply_traversal` ALONE
        // (`traversal_queue/coordinator.rs::drain_traversal_queue`), so a re-drive from
        // `route_window_opens` / `handle_history_action` / `handle_navigation` /
        // `ship_frame` / the reinstatement tail observes `is_applying() == false` and
        // passes — and the headline `navigate` case is one of those, reached from
        // Phase 1c, outside the bracket. The EXIT assert below is BLIND to a re-drive
        // too: the nested `drain_traversal_queue` recomputes `pending_len()` and
        // drains the OUTER pass's un-popped steps, so the outer `pop_next()` then
        // returns `None` and breaks, leaving the queue EMPTY — while issue ordering
        // (I2) and the per-drain Resolution-D `traversal_applied` latch have already
        // been violated silently.
        debug_assert!(
            !self.inline_state().drain_in_progress,
            "app-mode re-drove the coordinator from INSIDE its own drive (a `DrainHost` \
             seam body, a Phase-2 apply body, or the reinstatement tail), breaking plan \
             §4.4 premise 5 — the nested drain consumes the outer pass's un-popped steps, \
             violating issue ordering (I2) and the per-drain Resolution-D \
             `traversal_applied` latch while leaving the queue deceptively empty"
        );
        self.inline_state_mut().drain_in_progress = true;
        let mut outcome = DrainOutcome::default();
        for round in 0..MAX_TURN_COMPLETION_ROUNDS {
            // Sampled BEFORE the iteration so the comparison below sees the whole
            // iteration's document work, tail included (the tail's `navigate` is a
            // rebuild path).
            let doc_marker = self.current_document_marker();
            let iteration = DrainCoordinator::drain_same_turn(self);
            let iteration = self.reinstate_deferred_navigation(iteration);
            merge_turn_outcome(&mut outcome, iteration);
            if self.current_document_marker() != doc_marker {
                // EXIT (3) — a pipeline swap ended the turn. The predicate would
                // otherwise silently switch to reading the NEW document's runtime,
                // making "work this turn's handlers staged" and "the new document's
                // initial staging" indistinguishable; and §7.4.6.1 step 14.12.5
                // queues `updateDocument` as a later task when the target document
                // is not the displayed one, so a fresh document's initial scripts
                // are that later task's business, not this input turn's. The
                // unified exit rule below reads the NEW runtime and wakes the
                // follow-up dispatch, which drains that staging as a NEW turn.
                break;
            }
            if !self.staged_work_pending() {
                break; // EXIT (1) — quiescent.
            }
            if round == MAX_TURN_COMPLETION_ROUNDS - 1 {
                // EXIT (2) — cap. `eprintln!`, never `debug_assert!`: an
                // adversarial-but-legal page must degrade, not panic a debug build.
                // The staged work stays on the current runtime's channels and the
                // exit rule below wakes the follow-up dispatch, whose entry drive
                // re-runs this loop with a fresh cap — bounded work per dispatch,
                // no hang, and no unbounded window for the non-drain movers.
                eprintln!(
                    "[history] app-mode turn-completion loop hit max rounds \
                     ({MAX_TURN_COMPLETION_ROUNDS}); the staged session-history work \
                     is deferred to the next dispatch"
                );
            }
        }
        // Every body this drive runs has now returned, so the re-entry window closes
        // here — BEFORE the exit assert, so a debug panic there cannot leave the flag
        // latched and turn the next drive's entry assert into a false positive.
        self.inline_state_mut().drain_in_progress = false;
        // EXIT — a **residual step**: this drain left Phase-2 work behind. Reached
        // when something enqueued onto the queue without re-driving the coordinator —
        // an apply body calling `TraversalQueue::enqueue_traversal` directly (the shape
        // `MockHost::apply_traversal` models in `elidex-navigation`'s
        // `traversal_queue_tests.rs`), or a future drive site appending a Phase-1-only
        // pass that classifies a traversal with no Phase 2 behind it.
        //
        // Both asserts guard a future CODE change, not an unreachable state — which is
        // why the plan's rejection of an end-of-handler *re-drain* as dead code does
        // not cover them (a `debug_assert` re-drains nothing).
        debug_assert!(
            self.traversal_queue().is_empty(),
            "app-mode drain left a residual traversal step — something enqueued onto \
             the queue that this drain did not drain (an apply-body `enqueue_traversal`, \
             or a Phase-1 pass with no Phase 2 behind it). It does not strand \
             permanently — the NEXT `drain_same_turn` seeds `seen_traversal` from it and \
             its Phase-2 bounded snapshot drains it — but nothing bounds WHEN that turn \
             arrives: the turn-completion loop and its peek-gated dispatch-entry drives \
             are gated on `HostDriver::has_pending_session_history_work`, which reads the \
             ENGINE's staging channels, so a step already sitting on this QUEUE is \
             invisible to them. Until a drive is reached the residual acts as a full \
             partition barrier: it defers every fresh `pushState` behind it and latches \
             `suppress_default`, killing an unrelated default for a traversal that may \
             have gone out of range meanwhile"
        );
        // The unified exit rule (all three exits): the turn ends with work still
        // staged ⇒ schedule the follow-up dispatch whose entry drive will take it.
        // Issued through the named seam, not a bare `request_redraw`, so the
        // issuance is observable where the real repaint is `render_state`-gated.
        if self.staged_work_pending() {
            self.schedule_followup_dispatch();
        }
        outcome
    }

    /// The **reinstatement tail** — one iteration's, run inside the iteration
    /// (before the next Phase 1 partitions fresh intents, so a held `location.*`
    /// issued in iteration N can never apply after intents issued in N+1).
    ///
    /// The slot's one-drive lifetime contract narrows to a one-ITERATION lifetime:
    /// it is provably `None` at every iteration boundary, so there is no
    /// cross-iteration overwrite case to define
    /// ([`InteractiveState::deferred_navigation`](super::InteractiveState)).
    fn reinstate_deferred_navigation(&mut self, mut outcome: DrainOutcome) -> DrainOutcome {
        // **Reinstate a suppression this turn REFUTED** — the §7.4.2 leg of the
        // coordinator's Resolution-D rule, and the app-mode half of what keeps elidex's
        // enqueue-time nav-suppression from over-reaching.
        //
        // Phase 1c suppresses the own-context navigation whenever a `Traversal` step is
        // QUEUED. That is a deliberate **divergence** from WHATWG HTML §7.4.2.2
        // *Beginning navigation* step 19, not an application of it (webref-verified
        // 2026-07-26; slot `#11-nav-supersede-window-vs-ongoing-navigation`, and the
        // full statement lives on the `DrainHost::handle_navigation` contract in
        // `elidex-navigation`). Step 19's gate — *ongoing navigation* == "traversal"
        // (§7.4.2.5 *Aborting navigation*: "a navigation ID, "traversal", or null,
        // initially null") — is evaluated when `navigate` RUNS, and only §7.4.6.1
        // *Updating the traversable* step 8.4 ever sets that value, inside the APPLY
        // (three sites reset it to null, each noting "This allows new navigations of
        // navigable to start, whereas during the traversal they were blocked").
        // §7.4.3's enqueue sets nothing. So a `location.*` issued before the apply is
        // NEVER step-19-ignored, however the queued traversal later resolves —
        // elidex's window is a strict superset of the spec's. (That containment is a
        // property of TODAY's synchronous, non-yielding apply and is not permanent —
        // see the `DrainHost::handle_navigation` contract for why the planned
        // task-queued apply, `#11-session-history-task-queue-model`, breaks it.)
        //
        // App-mode narrows the superset back down within the turn.
        // [`apply_traversal`](DrainHost::apply_traversal) cancels the held request the
        // moment a traversal MOVES THE CURSOR — the same "cursor-moved" condition the
        // coordinator's `traversal_applied` latch uses to cancel a deferred
        // `SyncUpdate`, applied to the §7.4.2 leg instead of the §7.4.4 one. What
        // reaches here is therefore a suppression whose premise this turn REFUTED:
        // every queued traversal was a no-op or a failed load and the navigable never
        // traversed at all — so the navigation still applies, in the turn that issued
        // it. (Not "a turn late": Phase 1c already drained the VM slot, and the request
        // has been held on the host ever since, so it cannot re-fire on a later turn.)
        // The residual divergence — a traversal that DOES move the cursor still drops a
        // navigation the spec would have let start — is the slot's, not this tail's.
        //
        // This restores the contract origin/main's hand-rolled drain had on the §7.4.2
        // leg — "a no-target / failed-load traversal returns `false` … so the loop
        // CONTINUES and trailing same-turn intents still apply (Codex R1 P2 / R2)" —
        // which Slice B otherwise kept only for the deferred `SyncUpdate` leg (pinned
        // by `app_failed_traversal_does_not_cancel_trailing_sync_update`).
        //
        // Content-mode deliberately has NO mirror of this tail: its Phase 2 is a
        // genuinely later task, so by the time the prediction settles, applying the
        // request WOULD be the fire-a-turn-late that drain-and-discard exists to
        // prevent. Its fix is the tagged queue that carries the navigation as a queued
        // step (`#11-sync-navigation-steps-queue-tagging`), not a copy of this.
        if let Some((url, nav_type)) = self.inline_state_mut().deferred_navigation.take() {
            self.navigate(&url, nav_type);
            // Mirror the Phase-1c leg exactly — `handle_navigation`'s unconditional
            // `true`, known applied/shipped conflation included
            // (`#11-nav-applied-shipped-decouple`). `suppress_default` needs no update:
            // it is `own_context_action || suppress`, and a held request exists only
            // when `suppress` was true, so it is already `true`.
            outcome.own_context_action = true;
            outcome.shipped = true;
        }
        outcome
    }

    /// Drive the turn-completion loop from a winit **dispatch entry** iff work is
    /// staged — the peek-gated reader that makes a previous turn's residue
    /// **(next-dispatch ∨ frame)-bounded** instead of unbounded.
    ///
    /// Called at the top of each of the three dispatch bodies that can reach a
    /// non-drain cursor mover (`app/inline.rs`: the `WindowEvent::RedrawRequested`
    /// arm, whose tail runs chrome actions; `handle_keyboard_inline`, whose Alt+←/→
    /// branch traverses and returns without ever reaching
    /// `events::handle_keyboard`; `handle_mouse_press_inline`, ahead of
    /// `events::handle_click` and its `<a href>` default) — so every mover is
    /// preceded, in its own dispatch, by a drain of whatever was staged before it.
    ///
    /// Gated on the peek rather than on a residue flag because the loop's
    /// non-quiescent exits are only ONE of three staging sources: a mover that
    /// fires `popstate`/`hashchange` in place never enters the loop at all, and
    /// neither does a fresh document's load-time staging. A flag would see only the
    /// first; the channels answer for all three
    /// ([`process_pending_navigation`](Self::process_pending_navigation)).
    ///
    /// **The outcome is deliberately DISCARDED** (§4.5 (a)): this settles a
    /// *previous* turn's residue, and merging its `suppress_default` into the turn
    /// that follows would over-suppress that turn's `<a href>` default. That
    /// isolation is exact when this drive reaches quiescence; when it exits
    /// non-quiescent the surviving residue is consumed by the same dispatch's
    /// in-turn drive and shapes THAT turn's `suppress_default` — the same
    /// conservative over-suppression shape the coordinator's cross-turn-robust
    /// `suppress_default` already documents.
    pub(super) fn drive_staged_session_history_work(&mut self) {
        if self.staged_work_pending() {
            let _ = self.process_pending_navigation();
        }
    }

    /// The §4.4 quiescence predicate — "is session-history work staged on the
    /// CURRENT runtime?", read through the non-consuming
    /// [`HostDriver::has_pending_session_history_work`] peek.
    ///
    /// Reached through `interactive.pipeline.runtime`, which a swap replaces
    /// wholesale together with the pipeline (`app/navigation.rs`
    /// `teardown_document` + the assignment that follows), so this can only ever
    /// see the CURRENT document's staged work — never a torn-down document's
    /// residue, which is dropped with the old runtime (the pre-existing
    /// runtime-scoped-FIFO divergence fenced to
    /// `#11-session-history-task-queue-model`'s queue substrate).
    ///
    /// `false` in threaded mode (no inline state, so nothing to drive).
    fn staged_work_pending(&self) -> bool {
        self.interactive
            .as_ref()
            .is_some_and(|i| i.pipeline.runtime.has_pending_session_history_work())
    }

    /// The **document-swap marker** the loop compares across an iteration: the
    /// current session-history entry's `document_sequence`.
    ///
    /// Every rebuild path re-stamps it (`push` / `replace` /
    /// `restamp_current_document` — the last reached by a reload and by a
    /// cross-document traversal's commit), and it is allocated monotonically, so a
    /// changed value IS a swap and there is no ABA. Same-document applies (a
    /// fragment nav, a same-document traversal) do NOT restamp, so they do not end
    /// the loop — correct, since their staged follow-ups are this turn's work. A
    /// mid-loop navigate whose load FAILS does not restamp either (`navigate`
    /// early-returns before any of the three), so the loop CONTINUES against the
    /// still-intact old pipeline and FIFO — also correct.
    fn current_document_marker(&self) -> Option<u64> {
        self.inline_state()
            .nav_controller
            .current_document_sequence()
    }

    /// The §4.3 unified-exit-rule seam: request the follow-up dispatch whose entry
    /// drive will take the work this turn left staged.
    ///
    /// A named seam rather than a bare `request_redraw()` because the real request
    /// is `render_state`-gated (see [`ship_frame`](DrainHost::ship_frame)) and so a
    /// silent no-op in the disconnected test harness — the `cfg(test)` counter
    /// below is the observation point the degrade test asserts against. It is an
    /// observation point, not control state: nothing reads it outside tests, so the
    /// "no exit records any state" property stands.
    fn schedule_followup_dispatch(&mut self) {
        #[cfg(test)]
        {
            self.inline_state_mut().followup_dispatches += 1;
        }
        if let Some(state) = &self.render_state {
            state.window.request_redraw();
        }
    }
}

/// Accumulate one iteration's [`DrainOutcome`] into the turn's — **field-wise OR,
/// monotone, never cleared within a turn**.
///
/// Every field describes the TURN, not the last iteration.
/// [`suppress_default`](DrainOutcome::suppress_default) is the load-bearing case:
/// it is the ONE shared default-suppression signal, consumed by
/// `events::handle_click` as an early return, so if iteration 1 suppressed (an
/// own-context effect or a pending traversal) and iteration 2 is a quiet settle
/// returning all-false, the `<a href>` default must STAY dropped. OR-latching is
/// also what keeps the `hit_entity` staleness invariant's reasoning valid across
/// iterations (`events.rs`: "every rebuild path also latched `suppress_default`").
///
/// Destructured rather than field-accessed so that adding a field to
/// [`DrainOutcome`] is a compile error here instead of a silently-dropped signal.
fn merge_turn_outcome(turn: &mut DrainOutcome, iteration: DrainOutcome) {
    let DrainOutcome {
        own_context_action,
        shipped,
        suppress_default,
    } = iteration;
    turn.own_context_action |= own_context_action;
    turn.shipped |= shipped;
    turn.suppress_default |= suppress_default;
}

/// App-mode realization of the shared [`DrainHost`] seams
/// (`docs/plans/2026-07-session-history-slice-B-app-phase-separation.md` §4.1).
///
/// **Why `App` and not `InteractiveState`** (Q-IMPL-TARGET): the structural mirror
/// of `impl DrainHost for ContentState` is the receiver that owns EVERYTHING the
/// drain needs, the way the self-contained per-thread `ContentState` actor does.
/// That receiver is `App`:
/// - [`ship_frame`](Self::ship_frame) must perform the shell's OUTPUT *inside the
///   seam* (the mirror of `ContentState::ship_frame` → `send_display_list`), and
///   app-mode's output path is the winit window — which lives on
///   [`App::render_state`](super::App::render_state), not on `InteractiveState`.
/// - `App` also owns `web_storage`, which the rebuild body reads to re-install the
///   fresh pipeline's `localStorage` backend. Under `InteractiveState` that would
///   need a bolted-on clone of browser-level shared state (CLAUDE.md side-store
///   exception (b) forbids homing it on per-actor state) plus an external output
///   escape hatch.
///
/// The one cost is that the queue + controller are homed on `interactive` (an
/// `Option` on `App`), so the per-drain seams reach through
/// [`App::inline_state`] / [`App::inline_state_mut`] — a bounded, provably-safe
/// wrinkle (an unreachable panic, see
/// [`INTERACTIVE_DRIVE_ONLY`](super::INTERACTIVE_DRIVE_ONLY)), not an ownership
/// gap.
///
/// **Layering.** The coordinator owns the phase ordering + the I1/I2/I3 invariants;
/// these seams own the irreducibly shell-specific bodies. `App` /
/// `InteractiveState` / the pipeline / `EcsDom` / the winit window stay **behind the
/// trait** and never cross the `elidex-navigation` crate boundary: no shell type
/// appears in a coordinator signature, and every OS-window touch happens inside a
/// host-seam body. [`ship_frame`](Self::ship_frame) is not the only such touch —
/// [`handle_history_action`](Self::handle_history_action),
/// [`handle_navigation`](Self::handle_navigation) and
/// [`apply_traversal`](Self::apply_traversal) all reach `Window::set_title` through
/// the nav bodies they delegate to, deliberately (see
/// [`ship_frame`](Self::ship_frame)'s "`set_title` is deliberately NOT here"). What
/// [`ship_frame`](Self::ship_frame) alone owns is the **frame ship** — the repaint
/// request — not window access in general.
///
/// **No teardown guards.** Content-mode fails FOUR of its five pipeline-mutating
/// seams closed on `shutdown_requested` (`handle_history_action` / `apply_traversal`
/// / `route_window_opens` / `ship_frame`), because its `Shutdown` can be handled
/// mid-drain at the SW-wait reentrancy vector; it deliberately EXEMPTS the fifth,
/// `handle_navigation`, as "the teardown *cause*, never a victim … a guard there
/// would be dead code, so it is documented, not added" (`content/drain_host.rs`).
/// App-mode has no message pump and no SW-wait inside a drain (module doc, premises
/// 2–4), so there is no mid-drain teardown to guard against at all — every seam
/// here is in content's exempted `handle_navigation` position.
impl DrainHost for App {
    fn traversal_queue(&mut self) -> &mut TraversalQueue {
        &mut self.inline_state_mut().traversal_queue
    }

    /// **Phase 1a** — drain the `window.open` back-channel (§7.2.2.1) and DROP it.
    ///
    /// Legacy inline mode has no new-tab capability (`ChromeAction::NewTab` is
    /// threaded-mode only, see `handle_chrome_action`) and no iframe registry
    /// (`InteractiveState` carries no iframes — iframes are a content-thread
    /// facility), so the whole ordered queue is drained-and-dropped. Drained FIRST
    /// (unconditionally) so an own-context navigation/traversal later in the drain
    /// cannot strand queued opens on the old pipeline's runtime. These are effects
    /// on OTHER browsing contexts, so they report no own-context action — matching
    /// the seam contract.
    fn route_window_opens(&mut self) {
        let _ = self
            .inline_state_mut()
            .pipeline
            .runtime
            .take_pending_window_opens();
    }

    fn take_pending_history(&mut self) -> Vec<HistoryAction> {
        // The VM `pending_history` FIFO in issue order. Each synchronous
        // `pushState`/`replaceState` is an independent session-history commit that
        // the coordinator applies IN-TASK (WHATWG HTML §7.4.4 *Non-fragment
        // synchronous "navigations"* — the *URL and history update steps*), while
        // `Back`/`Forward`/`Go` are staged enqueue-only because §7.4.3 *Reloading
        // and traversing* step 4 appends their steps to the traversable for a LATER
        // task. That §7.4.4-vs-§7.4.3 split is exactly the partition the coordinator
        // performs on this Vec. Q-VM-MODEL: the staging model is unchanged and
        // identical to content's — only the shell drain re-times.
        self.inline_state_mut()
            .pipeline
            .runtime
            .take_pending_history()
    }

    /// **Phase 1b peek-classify** (Resolution E): `Some` for an **in-range**
    /// traversal (which STARTS a partition barrier), `None` for a **no-op** —
    /// `peek_delta` returns `None` out of range (§7.4.3 *Reloading and traversing*
    /// step 4.4, "If `allSteps[targetStepIndex]` does not exist, then abort these
    /// steps"), so it falls through without a barrier and the trailing same-turn
    /// sync updates + navigation stay in-task. Only the FIRST traversal of a turn
    /// is peek-gated this way; once a barrier exists the coordinator calls
    /// [`pending_traversal`](Self::pending_traversal) directly.
    ///
    /// ⚠ Evaluating that sub-step-4.4 bail-out HERE (issue time) rather than when the
    /// appended steps run is an engine-wide, pre-existing **hoist** — both shells
    /// carry this predicate. It has no reachable divergence today, but it is sound
    /// only as half of a coupled pair with the in-task §7.4.4 commit, so neither
    /// half moves to the queue alone. See the [`DrainHost::classify_traversal`]
    /// contract note (`#11-sync-navigation-steps-queue-tagging`).
    fn classify_traversal(&mut self, delta: TraversalDelta) -> Option<PendingTraversal> {
        let in_range = self
            .inline_state()
            .nav_controller
            .peek_delta(delta)
            .is_some();
        in_range.then(|| self.pending_traversal(delta))
    }

    /// **Phase 1b — build a pending traversal WITHOUT a peek.** The coordinator
    /// calls this for every traversal AFTER a barrier exists; the target resolves at
    /// Phase-2 apply time (§7.4.6.1), so a later `Forward`/`Go` is not dropped for
    /// peeking out-of-range against the still-unmoved cursor.
    fn pending_traversal(&mut self, delta: TraversalDelta) -> PendingTraversal {
        PendingTraversal {
            delta,
            // Scripted `history.back()`/`forward()`/`go()` passes a sourceDocument
            // (the calling document) to §7.4.3 *traverse the history by a delta*, so
            // step 3.3 sets `userInvolvement` to "none" (step 2's default is
            // "browser UI", overridden by the given-sourceDocument branch). The
            // chrome-button traversal (`UserInvolvement::BrowserUi`) does NOT reach
            // this seam — `handle_chrome_action` calls `traverse_to` directly and is
            // fenced out of this slice, collapsing into Slice 4's canonical
            // DIRECT-nav serialization (`#11-session-history-task-queue-model`).
            user_involvement: UserInvolvement::None,
        }
    }

    /// A synchronous `pushState`/`replaceState` *update* (§7.4.4) in Phase 1, or a
    /// deferred `SyncUpdate` step in Phase 2. The coordinator routes ONLY these here
    /// (`Back`/`Forward`/`Go` go through [`classify_traversal`](Self::classify_traversal)
    /// / [`apply_traversal`](Self::apply_traversal)), so this delegates straight to
    /// the sync-update-only [`handle_history_action`], which `debug_assert`s a
    /// mis-partitioned traversal rather than silently applying one.
    fn handle_history_action(&mut self, action: &HistoryAction) {
        handle_history_action(self, action);
    }

    /// **Phase 1c** — the last-wins own-context navigation (`location.*`, §7.4.2).
    ///
    /// On `suppress` (an in-range traversal pending this turn or still queued from
    /// an earlier one), **drain-and-HOLD**: the slot IS drained (this is its only
    /// drain) so a suppressed `location.*` cannot re-fire a turn late, and the
    /// request is not applied here — a queued traversal supersedes it. That
    /// enqueue-time supersede is a deliberate **divergence** from §7.4.2.2
    /// *Beginning navigation* step 19, whose gate (*ongoing navigation* =
    /// "traversal") only §7.4.6.1 step 8.4 sets, inside the APPLY — see the
    /// `DrainHost::handle_navigation` contract and slot
    /// `#11-nav-supersede-window-vs-ongoing-navigation`. App-mode does not DROP the
    /// request on the floor, which narrows the superset back to "a traversal that
    /// really moved the cursor": the request is held on
    /// [`InteractiveState::deferred_navigation`](super::InteractiveState) for the
    /// turn; Phase 2's [`apply_traversal`](Self::apply_traversal) cancels it iff a
    /// traversal moves the cursor, and [`App::process_pending_navigation`] reinstates
    /// whatever survives before the drive returns (full rationale there). The `Some`
    /// is therefore always consumed within one drive — never carried across turns.
    ///
    /// Returns `true` iff a navigation applied. This is where the retired
    /// hand-rolled drain's `nav-applied` early `return true` now lives: "a
    /// navigation applied" flows through
    /// [`DrainOutcome::own_context_action`]/[`shipped`](DrainOutcome::shipped)
    /// instead of short-circuiting the drain.
    ///
    /// **⚠ That `true` is UNCONDITIONAL — the known applied/shipped conflation**
    /// (slot `#11-nav-applied-shipped-decouple`, carved on PR #469 R15).
    /// [`App::navigate`](super::App::navigate) returns `()` and early-returns when
    /// `load_url_into_pipeline` fails, so a **failed** load still reports `true`
    /// here — setting BOTH [`own_context_action`](DrainOutcome::own_context_action)
    /// (→ the `<a href>` click default is suppressed) AND
    /// [`shipped`](DrainOutcome::shipped) (→ the coordinator's trailing
    /// [`ship_frame`](Self::ship_frame) is skipped) from ONE bool that overloads
    /// "moved the cursor" with "shipped a frame", against a trait contract asking
    /// for `true` iff the navigation replaced the pipeline **and** shipped its own
    /// frame. Content-mode's seam has the identical pre-existing behavior, so this
    /// is a mirrored gap, not an app-mode regression, and it is deliberately NOT
    /// fixed here (the naive propagation regressed 5 tests when the slot was
    /// carved). **App-mode is HARDER than the slot anticipated**: the slot scoped a
    /// `DrainHost`-contract change with "NO app-mode impl", but `navigate` itself
    /// returns `()` — the decouple has to change that signature too, and with it
    /// the non-drain callers (`<a href>` click, chrome address bar).
    ///
    /// `navigate` performs this leg's document work in the body (rebuild /
    /// same-document apply + `set_title`); the repaint it does NOT issue is covered
    /// by the dispatch layer — see [`ship_frame`](Self::ship_frame).
    fn handle_navigation(&mut self, suppress: bool) -> bool {
        let interactive = self.inline_state_mut();
        let Some(nav_req) = interactive.pipeline.runtime.take_pending_navigation() else {
            return false;
        };
        // Resolve BEFORE branching on `suppress`, so both legs drop an unresolvable /
        // blocked-scheme request identically, and so a reinstated request navigates to
        // exactly the URL the unsuppressed leg would have — resolved against the
        // Phase-1c document URL, not a Phase-2-mutated one.
        let Some(target_url) = resolve_nav_url(interactive.pipeline.url.as_ref(), &nav_req.url)
        else {
            return false;
        };
        if suppress {
            interactive.deferred_navigation = Some((target_url, nav_req.nav_type));
            return false;
        }
        self.navigate(&target_url, nav_req.nav_type);
        true
    }

    /// **Phase 2** — apply ONE deferred traversal (§7.4.6.1 *Updating the
    /// traversable*) via the shared peek-then-commit body, returning `true` iff it
    /// applied and shipped. Called inside the coordinator's nested-apply guard
    /// bracket (inert in app-mode — module doc).
    ///
    /// A traversal that MOVED THE CURSOR also **cancels the navigation Phase 1c
    /// held** ([`handle_navigation`](Self::handle_navigation)): the navigable really
    /// did traverse, so elidex's enqueue-time supersede stands. (It stands as a
    /// deliberate divergence — the navigation was issued BEFORE the apply, so
    /// §7.4.2.2 step 19 never gated it either way; slot
    /// `#11-nav-supersede-window-vs-ongoing-navigation`.) That is the §7.4.2 leg
    /// of the identical rule the coordinator applies to the §7.4.4 leg — its
    /// `traversal_applied` latch cancels a deferred `SyncUpdate` on exactly this
    /// condition — so both classes of same-turn intent deferred behind a barrier are
    /// governed by ONE predicate ("did the barrier move the cursor"), evaluated in
    /// one place.
    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> bool {
        let moved_cursor = apply_traversal_delta(self, traversal.delta);
        if moved_cursor {
            self.inline_state_mut().deferred_navigation = None;
        }
        moved_cursor
    }

    /// Ship the frame — app-mode's OUTPUT seam, the mirror of
    /// `ContentState::ship_frame`'s `send_display_list`. The App-owned winit window
    /// is the output path, so the seam requests the repaint.
    ///
    /// The coordinator calls this at most ONCE per drain, and only when an
    /// own-context effect happened that no apply body already shipped — i.e. a turn
    /// whose only own-context effect was one or more §7.4.4 synchronous updates. A
    /// pure `pushState` turn changes no layout but DOES change the chrome URL bar
    /// (`apply_state_change` → `chrome.set_url`), so it still needs the repaint.
    ///
    /// **`set_title` is deliberately NOT here.** It stays co-located in the
    /// navigation / sync-update bodies (`navigate`, `same_document_step`,
    /// `navigate_to_history_url`, `handle_history_action`), which set
    /// `window_title` in the same breath AND serve the non-drain callers (`<a href>`
    /// click, Alt+arrow, chrome toolbar). Every path that reaches this seam has
    /// already run one of those bodies, so a `set_title` here would be redundant.
    ///
    /// **Ship-once.** Two properties, both structural:
    /// - *Within a drain*, the coordinator's own `DrainOutcome::shipped` bookkeeping
    ///   keeps it from calling this after an apply body already shipped, and its
    ///   single trailing ship-decision fires at most one `ship_frame` per
    ///   `drain_same_turn`.
    /// - *Outside a drain*, this seam is unreachable — nothing but the coordinator
    ///   calls it, and the shared navigation bodies (which DO serve the non-drain
    ///   `<a href>` / Alt+arrow / chrome callers) deliberately gained no
    ///   `request_redraw` of their own. So the non-drain callers still issue exactly
    ///   the ONE dispatch-layer repaint they always did (`app/inline.rs`).
    ///
    /// **The ship-frame-output symmetry is NOT yet realized — say what actually
    /// holds.** `ContentState::ship_frame` is the drain's own output path for every
    /// own-context effect content-mode has. This seam is not: it is reached only on
    /// a turn with **no** navigation and **no** applied traversal, because either
    /// one sets `shipped = true` and the coordinator's `ship_if_needed` then skips
    /// it — and the app-mode nav bodies issue no `request_redraw` of their own
    /// (`app/navigation.rs` contains none). So on a drain turn that navigates or
    /// traverses, the drain requests **no repaint at all**; the repaint comes from
    /// the dispatch layer's unconditional redraw (`app/inline.rs:202` on the click
    /// path, `:298` on the keyboard path), which also covers the input handler's own
    /// effects (hover/active state, the dispatched event's re-render, the `<a href>`
    /// default navigation). This seam covers exactly the leftover case: the pure
    /// §7.4.4 sync-update turn.
    ///
    /// That is correct today — winit coalesces concurrent requests into one
    /// `RedrawRequested`, so the seam and the dispatch layer never produce two
    /// frames, and every non-drain caller redraws through that same dispatch layer —
    /// but it is a **division of labour with the caller**, not the self-sufficiency
    /// `ContentState::ship_frame` has via `send_display_list`. Closing the gap needs
    /// the applied/shipped decouple first: while
    /// [`handle_navigation`](Self::handle_navigation) reports `shipped = true`
    /// unconditionally, `shipped` cannot be trusted to mean a frame went out
    /// (`#11-nav-applied-shipped-decouple`).
    fn ship_frame(&mut self) {
        if let Some(state) = &self.render_state {
            state.window.request_redraw();
        }
    }
}

/// Apply a `Back`/`Forward`/`Go` **traversal** (§7.4.6.1 *Updating the traversable*)
/// — the delta-keyed Phase-2 entry point driven by the [`DrainHost::apply_traversal`]
/// seam, the app-mode mirror of `content/drain_host.rs::apply_traversal_delta`.
///
/// A deferred traversal carries its delta **un-resolved**, and the resolution is
/// §7.4.3's, not §7.4.6.1's: the delta→index arithmetic lives in the *queued* steps
/// of §7.4.3 *Reloading and traversing* → *traverse the history by a delta* step 4
/// — 4.1 "Let allSteps be the result of getting all used history steps for
/// traversable", 4.2 "Let currentStepIndex be the index of traversable's current
/// session history step within allSteps", 4.3 "Let targetStepIndex be
/// currentStepIndex plus delta" — which run when the queued steps run, i.e. against
/// the possibly-Phase-1-mutated entry list rather than the list as it stood at issue
/// time. §7.4.6.1 *apply the history step* is downstream of that and takes an
/// **already-resolved non-negative integer step**. So this function is the step-4
/// leg: it resolves the delta at Phase-2 drain time — `peek_delta` →
/// `(target_index, url)` — and hands the resolved pair to the index-keyed
/// [`App::traverse_to`], which owns the §7.4.6.1
/// peek-then-commit apply. `None` is a no-op traversal (out of range, or a stacked
/// `back(); back()` whose cursor already moved) and returns `false` without touching
/// the document, so the coordinator marks no own-context action and the caller's
/// default is not over-suppressed.
///
/// **One body, three entry points.** `traverse_to` stays index-keyed because it also
/// serves the two **non-drain** traversal callers, which pass an `(index, url)` pair
/// and are fenced OUT of the coordinator this slice: the chrome toolbar Back/Forward
/// (`navigation.rs::handle_chrome_action`) and Alt+←/→
/// (`inline.rs::handle_keyboard_inline`). This is the exact shape of the content
/// mirror, where the delta-keyed `apply_traversal_delta` resolves and then calls the
/// shared index/op-keyed `handle_navigate`.
///
/// **What is shared is `traverse_to`, NOT the resolve prologue.** Those two callers
/// hand-roll their own `peek_back`/`peek_forward` + clone per key/button arm, so the
/// peek→clone resolve is genuinely triplicated today (this function being the third).
/// **The fence is about queue ROUTING, not this dedup** — what Slice 4 decides is
/// whether those two callers go through the coordinator/queue at all
/// (`#11-session-history-task-queue-model`); collapsing three four-line peek→clone
/// prologues is a plain local refactor the fence never covered. It is deliberately
/// deferred *with* the routing rather than justified by it: Slice 4 restructures the
/// same three call sites, so unifying now is work that would be redone there.
pub(super) fn apply_traversal_delta(app: &mut App, delta: TraversalDelta) -> bool {
    let peeked = app.inline_state().nav_controller.peek_delta(delta);
    // Clone the URL to drop the `nav_controller` borrow before the `&mut app` apply.
    let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
        return false;
    };
    app.traverse_to(target_index, &url)
}
