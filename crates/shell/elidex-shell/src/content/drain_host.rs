//! Content-mode realization of the shared [`DrainHost`] drain adapter
//! (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md` §4).
//!
//! Carved out of `content/navigation.rs` at the drain-adapter cohesion seam
//! (touch-time 1000-line split, Codex PR#469 R5): the `impl DrainHost for
//! ContentState` phase-drain seams (`route_window_opens` / `take_pending_history` /
//! `handle_history_action` / `classify_traversal` / `pending_traversal` /
//! `handle_navigation` / `apply_traversal` / `ship_frame`) plus the two free
//! functions that ONLY serve those seams: the Phase-2 traversal-apply body
//! [`apply_traversal_delta`] and the interim reentrancy-guard
//! [`dispatch_or_buffer_reentrant`]. The sibling `content/navigation.rs` keeps the
//! pipeline-rebuild body (`handle_navigate`), the same-document-step primitive,
//! `window.open` routing, the §7.4.4 sync-update body (`handle_history_action`), and
//! URL normalization. Behavior-neutral move (no logic change).

use elidex_navigation::{
    DrainHost, PendingTraversal, TraversalDelta, TraversalQueue, UserInvolvement,
};
use elidex_script_session::{HistoryAction, HostDriver, NavigationType};

use crate::app::navigation::resolve_nav_url;
use crate::ipc::BrowserToContent;

use super::navigation::{
    handle_history_action, handle_navigate, route_window_opens, HistoryCursorOp,
};
use super::ContentState;

/// Route a `BrowserToContent` message re-dispatched from the SW-fetch wait loop
/// (`super::navigation::handle_navigate`) — the **interim
/// reentrancy guard** vector (Codex PR#469 R4).
///
/// When NO Phase-2 traversal apply is in progress (the common case — the SW-wait
/// was entered by a fresh `location.*` / address-bar navigation, not from inside
/// an `apply_traversal`), dispatch the message SYNCHRONOUSLY, exactly as before:
/// normal SW-fetch re-dispatch is unchanged.
///
/// When a Phase-2 apply IS in progress (`TraversalQueue::is_applying()` — this
/// `handle_navigate` is nested inside `apply_traversal_delta`, e.g. an
/// SW-controlled cross-document traversal), a re-dispatched nav-mutating message
/// (`Navigate` / `Reload` / chrome `GoBack`/`GoForward` / `MouseClick` /
/// `KeyDown`) would mutate the `NavigationController` entry list/cursor BETWEEN
/// the in-flight traversal's peek (`apply_traversal_delta`) and its
/// `commit_index`, committing a stale index against a mutated list — the reachable
/// corruption window. So BUFFER the message into
/// [`ContentState::deferred_reentrant_messages`] instead; the event loop
/// re-delivers it ONE per turn through its single held-message intake, once
/// `is_applying()` has cleared and the apply fully committed (see the buffer-first
/// intake in `event_loop::pump_turn` — it inherits that turn's Phase-2 apply and
/// the R9 bottom drain, so the buffered message runs OUTSIDE any held peek).
///
/// This is the bounded interim guard for ONE of the TWO nav-mutating reentrancy
/// vectors — it is NOT a complete serialization. The peek→commit corruption window
/// (a re-dispatched nav-mutating message mutating the entry list between a
/// traversal's peek and its `commit_index`) is reachable from two equally-live
/// vectors:
/// 1. **Coordinator-routed JS-queued traversals** (`history.back()`/`forward()`/`go()`
///    → the deferred Phase-2 [`apply_traversal_delta`]) — interim-guarded HERE: the
///    apply runs inside the `enter_nested_apply`/`exit_nested_apply` bracket, so
///    `is_applying()` holds and this function BUFFERS a reentrant message instead of
///    dispatching it under the held peek.
/// 2. **Chrome-direct traversals** (toolbar Back/Forward → `message_dispatch::chrome_traverse`)
///    — PRE-EXISTING and NOT interim-guarded: `chrome_traverse` peeks `target_index`
///    then calls `handle_navigate` WITHOUT the nested-apply bracket, so during its own
///    SW-wait `is_applying()` is FALSE and a re-dispatched nav-mutating message
///    dispatches SYNCHRONOUSLY, able to mutate the entry list before the stale
///    `target_index` commits. (Pre-existing: origin/main had the same peek→commit; R15
///    only carved this function.) Its corruption window is fenced to
///    `#11-session-history-task-queue-model` (which names the chrome-button-traversal
///    atomicity gap + the SW-fetch held-peek reentrancy vector).
///
/// BOTH vectors close canonically in Slice 4 — the FULL serialization routing EVERY
/// nav-mutating step (JS traversals + sync updates + direct/chrome/input navigations)
/// through the one traversal queue with per-step apply-time context (WHATWG HTML
/// §7.4.1.3 *Centralized modifications of session history*), which retires this interim
/// guard rather than only extending it.
///
/// **`Shutdown` is NEVER buffered** (Codex PR#469 R8). Buffering it would defer
/// teardown until a later `pump_turn` could re-deliver the buffer — but that
/// re-delivery cannot run until this SW-wait loop unblocks, which (for a delayed/lost
/// `SwFetchResponse`) is only at the ~30s navigation deadline. So a tab/window
/// close during an SW-controlled cross-document traversal would hang teardown for
/// up to 30s even though the `Shutdown` was already consumed from the channel.
/// Instead, handle `Shutdown` IMMEDIATELY here: run unload/teardown
/// (`handle_message_public`) and set [`ContentState::shutdown_requested`], which
/// breaks the SW-wait loop, aborts the in-flight `handle_navigate` before it can
/// load/commit against the torn-down pipeline, no-ops the remaining Phase-2 apply
/// seams, and makes `pump_turn` return `ControlFlow::Break` — a prompt exit with no
/// post-teardown mutation. Because `Shutdown` is never buffered, the buffer is
/// provably `Shutdown`-free — so the single held-message intake never has to carry
/// an exit signal out of the buffer (the retired replay batch's bespoke
/// `ControlFlow` exit-propagation is unnecessary).
pub(super) fn dispatch_or_buffer_reentrant(state: &mut ContentState, msg: BrowserToContent) {
    if matches!(msg, BrowserToContent::Shutdown) {
        // Teardown NOW (unload → iframes.shutdown_all → teardown_document).
        // `handle_message` returns `false` iff it ACTUALLY shut down (unload ran,
        // not `beforeunload`-canceled); only then flag the exit that the wait loop /
        // apply chain / `pump_turn` unwind observes. A `beforeunload`-canceled
        // Shutdown returns `true` (keep running) — do NOT force the exit, matching the
        // normal recv-path Shutdown contract (`handle_message` → `true` ⇒ no Break).
        if !super::message_dispatch::handle_message_public(msg, state) {
            state.shutdown_requested = true;
        }
    } else if state.traversal_queue.is_applying() {
        state.deferred_reentrant_messages.push(msg);
    } else {
        super::message_dispatch::handle_message_public(msg, state);
    }
}

/// Content-mode realization of the shared [`DrainHost`] seams
/// (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md` §4).
///
/// The single synchronous `process_pending_actions` drain is retired: input
/// handlers run [`DrainCoordinator::drain_synchronous_phase`](elidex_navigation::DrainCoordinator::drain_synchronous_phase)
/// **in-task** (window-opens → §7.4.4 sync updates → last-wins navigation,
/// enqueuing any in-range `Back`/`Forward`/`Go` traversal), and the async event
/// loop runs [`DrainCoordinator::run_deferred_traversals`](elidex_navigation::DrainCoordinator::run_deferred_traversals)
/// on a later pump turn (Phase 2 — the §7.4.6.1 *apply the history step*
/// realization). The coordinator owns the phase ordering + the §4.5 I1/I2/I3
/// invariants; these seams own the shell-specific bodies (pipeline rebuild, frame
/// shipping, entry-list resolution).
///
/// **Teardown-safety invariant — fail-closed at the seam boundary (Codex PR#469
/// R14).** A `Shutdown` handled mid-drain at the interim reentrancy guard
/// ([`dispatch_or_buffer_reentrant`], reached from a seam's `handle_navigate`
/// SW-wait — for EITHER reentrancy vector's SW-wait, since `Shutdown` short-circuits
/// before the `is_applying()` branch) runs teardown and sets
/// [`ContentState::shutdown_requested`]. Because
/// [`DrainCoordinator`](elidex_navigation::DrainCoordinator) touches the pipeline
/// ONLY through these `DrainHost` methods, guarding every **pipeline-mutating** seam
/// at ENTRY makes post-teardown pipeline work impossible BY CONSTRUCTION — the
/// completeness a post-drain `pump_turn` check cannot give (a check placed AFTER a
/// compound drain always leaves a "the next seam ran before the check" hole; this
/// generalizes the per-seam R8 guards into a provably-complete boundary). The
/// pipeline-mutating seams and their disposition:
/// - [`handle_history_action`](Self::handle_history_action) — fails closed at entry.
/// - [`apply_traversal`](Self::apply_traversal) — fails closed at entry.
/// - [`route_window_opens`](Self::route_window_opens) — fails closed at entry (HOLE A).
/// - [`ship_frame`](Self::ship_frame) — fails closed at entry (HOLE B).
/// - [`handle_navigation`](Self::handle_navigation) — the teardown *cause*, never a
///   victim: it is the ONLY seam whose SW-wait tears our pipeline down, and nothing
///   earlier in the same drain does (Phase 1a `route_window_opens` routes to an
///   isolated iframe pipeline, Phase 1b `handle_history_action` is same-document
///   no-SW), so at its entry `shutdown_requested` is always false and its own pre-nav
///   `send_display_list` runs on the still-live pipeline BEFORE `handle_navigate`. A
///   guard there would be dead code, so it is documented, not added.
///
/// The non-pipeline-mutating seams need no guard: [`traversal_queue`](Self::traversal_queue)
/// (accessor), [`take_pending_history`](Self::take_pending_history) (drains the VM
/// FIFO, ships nothing), [`classify_traversal`](Self::classify_traversal) /
/// [`pending_traversal`](Self::pending_traversal) (peek + value construction).
/// `pump_turn`'s own `shutdown_requested` checks are thereby demoted to prompt
/// loop-exit + a guard for the DIRECT (non-seam) pump work (the held `handle_message`
/// + the frame tick).
impl DrainHost for ContentState {
    fn traversal_queue(&mut self) -> &mut TraversalQueue {
        &mut self.traversal_queue
    }

    /// **Phase 1a** — drain + route the `window.open` back-channel (§7.2.2.1).
    /// These are effects on OTHER browsing contexts (a new tab / a child iframe)
    /// that do NOT replace our pipeline and must NOT report an own-context action;
    /// they ship their own display list when they have a real effect. Drained
    /// FIRST so an own-context navigation/traversal cannot strand queued opens
    /// (they live on the old pipeline's runtime). Same ordered routing as the
    /// async pump (edge E4).
    fn route_window_opens(&mut self) {
        // A `Shutdown` handled mid-drain (`dispatch_or_buffer_reentrant`) already ran
        // teardown; do NOT re-render / ship a frame from the torn-down pipeline. Every
        // pipeline-mutating `DrainHost` seam fails closed at entry (the provable
        // teardown-safety invariant; see the impl-level doc comment). The pump breaks on
        // `shutdown_requested` right after the drain (Codex PR#469 R14 / HOLE A).
        if self.shutdown_requested {
            return;
        }
        let window_opens = self.pipeline.runtime.take_pending_window_opens();
        if window_opens.is_empty() {
            return;
        }
        let outcome = route_window_opens(self, window_opens);
        if outcome.any_effect {
            if outcome.navigated_iframe {
                self.re_render();
            }
            self.send_display_list();
        }
    }

    fn take_pending_history(&mut self) -> Vec<HistoryAction> {
        // The VM `pending_history` FIFO (each synchronous `pushState`/`replaceState`
        // an independent session-history commit; `Back`/`Forward`/`Go` staged as
        // enqueue-only). Q-VM-MODEL: the staging model is unchanged (the VM
        // yields every action of the turn); only the shell drain re-times.
        self.pipeline.runtime.take_pending_history()
    }

    /// A synchronous `pushState`/`replaceState` *update* (§7.4.4) in Phase 1, or a
    /// deferred `SyncUpdate` step in Phase 2. The coordinator routes ONLY these
    /// here (`Back`/`Forward`/`Go` go through `classify_traversal` / `apply_traversal`),
    /// so this delegates straight to the sync-update-only [`handle_history_action`].
    fn handle_history_action(&mut self, action: &HistoryAction) {
        // A `Shutdown` handled mid-drain (`dispatch_or_buffer_reentrant`) already ran
        // teardown; a trailing Phase-2 `SyncUpdate` must NOT mutate the torn-down
        // pipeline (Codex PR#469 R8). The pump breaks on the flag right after the drain.
        if self.shutdown_requested {
            return;
        }
        handle_history_action(self, action);
    }

    /// **Phase 1b peek-classify** (Resolution E): `Some` for an in-range traversal
    /// (a partition barrier), `None` for a no-op — `peek_*` returns `None` at the
    /// ends / out of range (§7.4.3 sub-step 4.4 "does not exist ⇒ abort"), so it
    /// falls through and the trailing same-turn sync/nav stay in-task.
    fn classify_traversal(&mut self, delta: TraversalDelta) -> Option<PendingTraversal> {
        // The peek decides `Some`/`None` (in-range vs no-op — §7.4.3 sub-step 4.4);
        // `pending_traversal` builds the value. Only the FIRST traversal of a turn
        // is peek-gated this way; once a barrier exists the coordinator calls
        // `pending_traversal` directly (F4).
        let in_range = self.nav_controller.peek_delta(delta).is_some();
        in_range.then(|| self.pending_traversal(delta))
    }

    /// **Phase 1b — build a pending traversal WITHOUT a peek** (F4). The coordinator
    /// calls this for every traversal AFTER a barrier exists; the target resolves at
    /// Phase-2 apply time (§7.4.6.1), so a later `Forward`/`Go` is not dropped for
    /// peeking out-of-range against the still-unmoved cursor.
    fn pending_traversal(&mut self, delta: TraversalDelta) -> PendingTraversal {
        PendingTraversal {
            delta,
            // Scripted `history.back()`/`forward()`/`go()` passes a sourceDocument
            // (the calling document) to *traverse the history by a delta*, so
            // §7.4.3 step 3.3 sets `userInvolvement` to "none" (step 2's default is
            // "browser UI", overridden by the given-sourceDocument branch). A
            // chrome-button traversal (`BrowserUi`, no sourceDocument) is Slice B.
            user_involvement: UserInvolvement::None,
        }
    }

    /// **Phase 1c** — the last-wins own-context navigation (`location.*`, §7.4.2).
    /// On `suppress` (a pending in-range traversal), drain-and-DISCARD: the slot IS
    /// drained (its only drain, `take_pending_navigation`) so it cannot re-fire a
    /// turn late, but the request is dropped without applying — a queued traversal
    /// supersedes it (§7.4.2.2 step 19 "ignored"; plan §1 A / F1).
    fn handle_navigation(&mut self, suppress: bool) -> bool {
        let Some(nav_req) = self.pipeline.runtime.take_pending_navigation() else {
            return false;
        };
        if suppress {
            return false;
        }
        let Some(target_url) = resolve_nav_url(self.pipeline.url.as_ref(), &nav_req.url) else {
            return false;
        };
        // Pre-send the current display list (the pushState+nav common case's
        // single send), then the navigation ships its own via `notify_navigation`.
        self.send_display_list();
        // `Reload` → `Keep` (rebuild, no cursor advance); `Push`/`Replace` → `Push`
        // (thread-mode collapses `Replace` → `Push` for the cursor op, §10-D6).
        let cursor_op = match nav_req.nav_type {
            NavigationType::Reload => HistoryCursorOp::Keep,
            NavigationType::Push | NavigationType::Replace => HistoryCursorOp::Push,
        };
        handle_navigate(self, &target_url, cursor_op, None);
        true
    }

    /// **Phase 2** — apply ONE deferred traversal (§7.4.6.1 *apply the history
    /// step*) via the shared peek-then-commit body, returning `true` iff it applied
    /// and shipped. The coordinator cancels any trailing straddle `SyncUpdate` once
    /// ANY traversal applies (Resolution D generalized, R6), so the apply no longer
    /// reports document-change discrimination.
    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> bool {
        // If a `Shutdown` tore this thread down mid-drain (an earlier step's SW-wait
        // saw it — `dispatch_or_buffer_reentrant`), do NOT peek/commit a further
        // queued traversal against the torn-down pipeline (Codex PR#469 R8). Report
        // no apply; the pump breaks on `shutdown_requested` right after the drain.
        if self.shutdown_requested {
            return false;
        }
        apply_traversal_delta(self, traversal.delta)
    }

    fn ship_frame(&mut self) {
        // Fail-closed on a mid-drain teardown: a co-staged Phase-1c `handle_navigation`
        // SW-wait can tear down before `ship_if_needed` reaches here, and shipping the
        // torn-down pipeline's display list would surface a dead frame (Codex PR#469 R14
        // / HOLE B). Same seam-boundary invariant as the other `DrainHost` seams.
        if self.shutdown_requested {
            return;
        }
        self.send_display_list();
    }
}

/// Apply a `Back`/`Forward`/`Go` **traversal** (§7.4.6.1 *apply the history
/// step*) — the single peek-then-commit body driven by the deferred-Phase-2
/// [`DrainHost::apply_traversal`] seam (and the re-anchored isolation tests).
/// After phase-separation this is the SOLE traversal-apply path: the synchronous
/// `handle_history_action` seam only carries §7.4.4 sync updates now
/// (One-issue-one-way: one traversal-apply body, not a fork).
///
/// Peeks the target WITHOUT moving the cursor; `handle_navigate` commits the move
/// (via [`HistoryCursorOp::Commit`]) ONLY if the load succeeds — an atomic
/// traversal (Codex R3), with the commit threaded into `handle_navigate` before
/// its `notify_navigation` (Codex R5). A failed load leaves the cursor on the
/// still-active document, so a trailing same-turn `pushState` commits from the
/// correct index (no speculative move, no rollback).
///
/// Returns `true` iff `handle_navigate` applied (a rebuild that replaced the
/// pipeline, or a same-document apply-in-place). A no-op (no target — e.g. a
/// stacked `back(); back()` whose cursor already moved, or an out-of-range `go`)
/// or a failed cross-document load returns `false`. The coordinator cancels any
/// trailing straddle `SyncUpdate` once ANY traversal applies (Resolution D
/// generalized, R6), so this no longer reports document-change discrimination.
pub(super) fn apply_traversal_delta(state: &mut ContentState, delta: TraversalDelta) -> bool {
    let peeked = state.nav_controller.peek_delta(delta);
    // Clone the URL to drop the `nav_controller` borrow before the `&mut state` load.
    let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
        return false;
    };
    handle_navigate(state, &url, HistoryCursorOp::Commit(target_index), None)
}
