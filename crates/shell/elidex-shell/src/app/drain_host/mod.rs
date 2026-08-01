//! App-mode (legacy inline) realization of the shared [`DrainHost`] drain adapter
//! (`docs/plans/2026-07-session-history-slice-B-app-phase-separation.md` §4).
//!
//! The app-mode counterpart of `content/drain_host.rs`, split in two at its own
//! cohesion seam: **this module is the DRIVE SITE and its schedule policy** —
//! [`App::process_pending_navigation`], the turn-completion loop,
//! [`MAX_TURN_COMPLETION_ROUNDS`], the quiescence predicate
//! ([`App::staged_work_pending`]), the swap marker
//! ([`App::current_document_marker`]) and the per-turn outcome merge — while the
//! sibling [`host`] holds the `impl DrainHost for App` phase-drain seam bodies the
//! coordinator calls back into. The further sibling
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
//! is retired: [`App::process_pending_navigation`] now drives
//! [`DrainCoordinator::drain_same_turn`] inside a guard pair and a bounded
//! quiescence loop, and holds no drain logic of its own.
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
//! document, all of them BEFORE the drive site is reached. It cannot
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
//!    the [`TraversalQueue`](elidex_navigation::TraversalQueue) only on the NEXT ITERATION of the drive site's
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
//!    queue's own [`TraversalQueue::is_applying`](elidex_navigation::TraversalQueue::is_applying) would NOT do for the entry assert:
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

use super::App;

mod host;

/// Re-exported under the short `drain_host::` path the drain suite has always
/// used. Test-only: the seam body's sole production caller is
/// [`DrainHost::apply_traversal`], in `host` itself.
#[cfg(test)]
pub(super) use host::apply_traversal_delta;

/// Maximum iterations of the app-mode turn-completion loop
/// ([`App::process_pending_navigation`]) per drive.
///
/// A page whose `popstate` handler re-stages unconditionally makes the fixpoint
/// unreachable, and this loop runs on the single-writer renderer thread, so an
/// unbounded loop is a hang. Same order as the in-tree
/// `MAX_CE_STABILIZATION_ROUNDS`, and far above any legitimate depth: each round
/// requires the page to have staged NEW work from inside the previous round's
/// handlers.
///
/// **The degrade shape is NOT the same, and that difference is load-bearing.**
/// Both siblings pair their cap with a *next-frame* statement — "some mutations may
/// be deferred to next frame", "any remaining messages will be drained on the next
/// frame" — and this cap must NOT be read as carrying one. Precisely:
/// `MAX_DRAIN_PER_TAB` really does have a frame behind it, but only in THREADED
/// mode (`handle_redraw_threaded` → `drain_content_messages`); it is unreachable
/// inline. `MAX_CE_STABILIZATION_ROUNDS` has no frame behind it inline either — it
/// lives in `crate::re_render`, which `handle_redraw_inline` never calls — but its
/// residue is reached by a *broad* input set: every path that re-renders, a mouse
/// release included. This cap's residue is reached only by
/// [`App::process_pending_navigation`], whose two callers
/// (`events::handle_click` / `events::handle_keyboard`) BOTH return early on
/// ordinary inputs — a hit-test miss, a chrome-band click, an unfocused document.
/// So this cap borrows the bound and the `eprintln!` and NOT the reachability:
/// nothing schedules a drive, and the inputs that would reach one are a strict
/// subset of the inputs that reach either sibling's consumer.
///
/// The drain-start-snapshot bound the coordinator's Phase 2 uses
/// (`pending_len()`) is deliberately NOT the idiom here: it terminates a drain of
/// *pre-existing* work by excluding work created during it, and consuming exactly
/// that work is this loop's entire purpose — a start-snapshot of the loop is the
/// degenerate "one iteration", i.e. the defect it fixes.
///
/// `pub(super)` for the same reason as [`App::staged_work_pending`]: the cap pin
/// asserts the loop ran EXACTLY this many iterations, so it must read the cap
/// itself — a literal `8` in the test would keep passing after this value changed,
/// silently pinning the wrong number.
pub(super) const MAX_TURN_COMPLETION_ROUNDS: usize = 8;

impl App {
    /// Drive the input turn's session-history / navigation work **to quiescence**
    /// — the app-mode leg of the shared phase-partition, and app-mode's
    /// counterpart of content-mode's post-Phase-2 settle.
    ///
    /// Called at the end of an input handler (`events::handle_click` /
    /// `events::handle_keyboard`), after event dispatch + re-render. Each ITERATION runs
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
    /// It has TWO callers, neither reachable synchronously from any drain body:
    /// `events::handle_click` and `events::handle_keyboard`. (A peek-gated drive at
    /// the winit dispatch ENTRY, which would bound a previous turn's residue ahead
    /// of the non-drain movers, was designed and implemented for this slice and then
    /// **withdrawn** — see the residue note below.)
    ///
    /// Returns the turn's [`DrainOutcome`] (the shared summary both shells return)
    /// rather than the retired ad-hoc `bool` — the **field-wise OR** of every
    /// iteration's outcome, because the fields describe the TURN, not the last
    /// iteration (`merge_turn_outcome`). Callers read the field they
    /// need: `handle_click` consumes
    /// [`suppress_default`](DrainOutcome::suppress_default) to drop the `<a href>`
    /// default navigation; `handle_keyboard` calls for effect and ignores it. When
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
    /// peek-classified (Resolution E) and left **resident on the [`TraversalQueue`](elidex_navigation::TraversalQueue)
    /// across the turn boundary**. Such a step is NOT stranded: the next
    /// `drain_same_turn` seeds
    /// `seen_traversal` from [`TraversalQueue::has_pending_traversal`](elidex_navigation::TraversalQueue::has_pending_traversal) and its Phase 2
    /// drains it. What the trailing drain does is
    /// **freeze the in-range classification a turn early**, voiding the queue's own
    /// contract that "Resolution E's peek-classify guarantees a no-op `go(999)` never
    /// leaves a `Traversal` step here, so it does not over-suppress"
    /// ([`TraversalQueue::has_pending_traversal`](elidex_navigation::TraversalQueue::has_pending_traversal)) — the **non-drain** cursor movers
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
    /// # The three exits — none of which schedules anything
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
    /// **surviving** runtime's channels, which is where the peek reads. On the swap
    /// exit "surviving" is doing real work — what the OLD runtime held is not
    /// waiting anywhere, it was dropped with that runtime (see
    /// [`Self::staged_work_pending`]).
    ///
    /// **⚠ What this slice does NOT do — the residue is not bounded here.** On a
    /// non-quiescent exit the staged work waits for the next drive that is actually
    /// REACHED, and that is not every input: `events::handle_click` returns early on
    /// a hit-test miss / a chrome-band click / an unset `cursor_pos`, and
    /// `events::handle_keyboard` on an unfocused document, all before this drive. So
    /// the residual lifetime is **unbounded**, exactly as on `origin/main` — this
    /// slice fixes the *in-turn* settle for turns that reach quiescence (the §1
    /// harm on that path) and changes nothing about the rest. Two staging sources
    /// never enter this loop at all and are equally unbounded: a mover that fires
    /// `popstate` in place (chrome Back/Forward, Alt+←/→, a fragment `navigate`)
    /// and a fresh document's load-time staging.
    ///
    /// A peek-gated drive at the winit dispatch entry was designed, implemented and
    /// reviewed for this slice to close that, then **withdrawn before push**: it ran
    /// the FULL `drain_same_turn` — Phase 1c cross-document navigation included —
    /// ahead of the same dispatch's own input, so a click could land on a document
    /// the drive had just swapped in. `content/event_loop.rs` documents that exact
    /// ordering as forbidden and omits Phase 1c from its own top drain for the
    /// reason. Closing the residue therefore needs content-mode's SHAPE
    /// ([`DrainCoordinator::drain_synchronous_updates`], Phase 1a+1b only) plus an
    /// address-bar-focus guard — but **not necessarily at the dispatch entry**: a
    /// drive at the END of the traversal movers is an open alternative that carries
    /// no waiting input behind it, while the rebuild movers (`Navigate`/`Reload`)
    /// cannot take one without settling a fresh document's staging inside the old
    /// turn (EXIT 3's own rationale). Choosing between those placements is the
    /// substance of its own plan-reviewed slice, alongside Slice 4's mover routing
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
                // new document's initial staging is then the next drive's
                // business, as a NEW turn.
                break;
            }
            if !self.staged_work_pending() {
                break; // EXIT (1) — quiescent.
            }
            if round == MAX_TURN_COMPLETION_ROUNDS - 1 {
                // EXIT (2) — cap. `eprintln!`, never `debug_assert!`: an
                // adversarial-but-legal page must degrade, not panic a debug build.
                // The staged work stays on the current runtime's channels for
                // the next drive that is reached — bounded work per turn and no
                // hang, but NOT a bounded residue, and the message must not imply
                // one: nothing schedules that drive (method doc + the cap's own).
                eprintln!(
                    "[history] app-mode turn-completion loop hit max rounds \
                     ({MAX_TURN_COMPLETION_ROUNDS}) — a page script is re-staging \
                     history work every round. The staged work stays on the current \
                     runtime's channels: nothing schedules a drive, so it applies \
                     only if some later click/keypress reaches one (and is dropped \
                     outright if this document is replaced first)"
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
             arrives (app-mode pumps only on input, and its early returns mean the \
             next input event need not reach this drive at all). Until then the residual acts as a full \
             partition barrier: it defers every fresh `pushState` behind it and latches \
             `suppress_default`, killing an unrelated default for a traversal that may \
             have gone out of range meanwhile"
        );
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

    /// The **document-swap marker** the loop compares across an iteration.
    ///
    /// What re-stamps it and what inherits it is the controller's contract, stated
    /// once there
    /// ([`NavigationController::current_document_sequence`](elidex_navigation::NavigationController::current_document_sequence))
    /// — including the monotonic-allocation argument that gives the comparison no
    /// ABA. What belongs HERE is only what the loop does with it: a changed value
    /// ends the turn (EXIT 3), an equal value continues it. Two consequences worth
    /// naming at this end, because the loop's correctness rests on them:
    /// same-document applies do not restamp, so a fragment nav or a same-document
    /// traversal does NOT end the loop — correct, since their staged follow-ups are
    /// this turn's work; and a mid-loop navigate whose load FAILS does not restamp
    /// either (`navigate` early-returns before any stamp site), so the loop
    /// continues against the still-intact old pipeline and FIFO — also correct.
    ///
    /// `pub(super)` for the same reason as [`Self::staged_work_pending`]: the
    /// negative pin on the swap exit is a regression guard, so it must read the
    /// exact function the swap exit reads.
    pub(super) fn current_document_marker(&self) -> Option<u64> {
        self.inline_state()
            .nav_controller
            .current_document_sequence()
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
