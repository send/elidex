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

use elidex_navigation::{DrainCoordinator, DrainHost, DrainOutcome};
use elidex_script_session::HostDriver;

#[allow(unused_imports)] // doc-link only (the premise-5 / exit-assert prose).
use elidex_navigation::TraversalQueue;

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
    /// [`ship_frame`](DrainHost::ship_frame)).
    ///
    /// **What the rule's generality costs is ≈0; the follow-up frame itself is
    /// not free — say which.** Making the rule fire on EVERY peek-true exit rather
    /// than on the cap alone is what costs nothing, because a request issued where
    /// one was already pending coalesces into the same `RedrawRequested`. A
    /// genuinely new frame is a whole frame. On an ordinary page that happens at
    /// most once per turn and then the loop is quiescent. On the adversarial
    /// re-stager the cap exists for, it is **self-sustaining by design**: cap →
    /// follow-up dispatch → that dispatch's entry drive → cap again, i.e. a capped
    /// loop plus a paint every frame for as long as the page keeps re-staging. That
    /// is the accepted degradation — bounded WORK per dispatch, no hang, and no
    /// unbounded wrong-entry window for the §4.1 movers — not a claim that the page
    /// stops costing anything. Stopping the re-arm instead would need a
    /// consecutive-cap counter, i.e. exactly the drive-schedule state this design
    /// has none of; it is Slice 4's to revisit with the mover routing
    /// (`#11-session-history-task-queue-model`).
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
            let mut iteration = DrainCoordinator::drain_same_turn(self);
            self.reinstate_deferred_navigation(&mut iteration);
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
    fn reinstate_deferred_navigation(&mut self, outcome: &mut DrainOutcome) {
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
    ///
    /// `pub(super)` so the tests assert quiescence through THIS function rather
    /// than a copy of its reach-through: nearly every turn-completion assertion is
    /// "did the drive reach quiescence?", and a second definition would keep
    /// compiling — and keep passing — after this one's path changed.
    pub(super) fn staged_work_pending(&self) -> bool {
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
    ///
    /// `pub(super)` for the same reason as [`Self::staged_work_pending`]: the
    /// negative pin on the swap exit is a regression guard, so it must read the
    /// exact function the swap exit reads.
    pub(super) fn current_document_marker(&self) -> Option<u64> {
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
mod host;

/// Re-exported under the short `drain_host::` path the drain suite has always
/// used. Test-only: the seam body's sole production caller is
/// [`DrainHost::apply_traversal`], in `host` itself.
#[cfg(test)]
pub(super) use host::apply_traversal_delta;
