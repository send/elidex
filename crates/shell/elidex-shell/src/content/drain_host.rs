//! Content-mode realization of the shared [`DrainHost`] drain adapter
//! (`docs/plans/2026-07-session-history-slice-A-content-phase-separation.md` §4).
//!
//! Carved out of `content/navigation.rs` at the drain-adapter cohesion seam
//! (touch-time 1000-line split, Codex PR#469 R5): the `impl DrainHost for
//! ContentState` phase-drain seams (`route_window_opens` / `take_pending_history` /
//! `handle_history_action` / `normalize_deferred_sync_update` / `classify_traversal`
//! / `pending_traversal` / `handle_navigation` / `apply_traversal` / `ship_frame`)
//! plus the three free functions that ONLY serve those seams: the Phase-2
//! traversal-apply body [`apply_traversal_delta`], the deferred-`SyncUpdate` URL
//! normalizer [`normalize_deferred_history_url`], and the interim reentrancy-guard
//! [`dispatch_or_buffer_reentrant`]. The sibling `content/navigation.rs` keeps the
//! pipeline-rebuild body (`handle_navigate`), the same-document-step primitive,
//! `window.open` routing, the §7.4.4 sync-update body (`handle_history_action`), and
//! URL normalization. Behavior-neutral move (no logic change).

use elidex_navigation::{
    DrainHost, PendingTraversal, TraversalApplyOutcome, TraversalDelta, TraversalKind,
    TraversalQueue, UserInvolvement,
};
use elidex_script_session::{HistoryAction, HostDriver, NavigationType};

use crate::app::navigation::resolve_nav_url;
use crate::ipc::BrowserToContent;

use super::navigation::{
    handle_history_action, handle_navigate, route_window_opens, HistoryCursorOp,
};
use super::ContentState;

/// Route a `BrowserToContent` message re-dispatched from the SW-fetch wait loop
/// ([`handle_navigate`](super::navigation::handle_navigate)) — the **interim
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
/// `KeyDown`) would mutate the [`NavigationController`] entry list/cursor BETWEEN
/// the in-flight traversal's peek (`apply_traversal_delta`) and its
/// `commit_index`, committing a stale index against a mutated list — the reachable
/// corruption window. So BUFFER the message into
/// [`ContentState::deferred_reentrant_messages`] instead; the event loop replays
/// it at the top of a later `pump_turn`, once `is_applying()` has cleared and the
/// apply fully committed (see `event_loop::replay_deferred_reentrant_messages`).
///
/// This is the bounded interim guard — it consumes `is_applying()` at the single
/// reentrancy vector. The FULL canonical serialization (routing EVERY nav-mutating
/// step through the traversal queue with per-step apply-time context, WHATWG HTML
/// §7.4.1.3) is Slice 4.
pub(super) fn dispatch_or_buffer_reentrant(state: &mut ContentState, msg: BrowserToContent) {
    if state.traversal_queue.is_applying() {
        state.deferred_reentrant_messages.push(msg);
    } else {
        super::event_loop::handle_message_public(msg, state);
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
        handle_history_action(self, action);
    }

    /// **T3** (Codex PR#469 R3) — bind a to-be-deferred `SyncUpdate`'s URL to the
    /// CALL-TIME document URL. `history.pushState(state, '')` / `replaceState` with
    /// the URL omitted stages `url: None`; a relative URL is base-relative. Once
    /// deferred behind a same-turn traversal, Phase-2's `apply_push_replace_state`
    /// resolves the URL against the **post-traversal** `pipeline.url` — so
    /// `back(); pushState(null, '')` from `/a` would record the back target (base),
    /// not `/a`. Resolve the effective URL NOW against the current document URL
    /// (the call-time base): `None` → the current document URL; a relative/absolute
    /// URL → its absolute call-time resolution. Phase 2 then records the call-time
    /// target regardless of any same-document traversal that moved `pipeline.url`
    /// first. Traversals never defer as a `SyncUpdate`, so they pass through
    /// unchanged.
    fn normalize_deferred_sync_update(&self, action: HistoryAction) -> HistoryAction {
        let base = self.pipeline.url.as_ref();
        match action {
            HistoryAction::PushState {
                url,
                title,
                serialized_state,
            } => HistoryAction::PushState {
                url: normalize_deferred_history_url(base, url),
                title,
                serialized_state,
            },
            HistoryAction::ReplaceState {
                url,
                title,
                serialized_state,
            } => HistoryAction::ReplaceState {
                url: normalize_deferred_history_url(base, url),
                title,
                serialized_state,
            },
            // Not a sync update — never deferred as a `SyncUpdate`; return verbatim.
            other => other,
        }
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
    /// step*) via the shared peek-then-commit body, reporting `shipped` +
    /// `changed_document` (the latter drives the coordinator's Resolution-D
    /// `SyncUpdate` cancellation).
    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> TraversalApplyOutcome {
        apply_traversal_delta(self, traversal.delta)
    }

    fn ship_frame(&mut self) {
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
/// Returns a [`TraversalApplyOutcome`]: `shipped` iff `handle_navigate` applied
/// (a rebuild that replaced the pipeline, or a same-document apply-in-place), and
/// `changed_document` iff the applied traversal landed a **fresh document** — a
/// §7.4.6.1 [`TraversalKind::Rebuild`] that actually loaded (`shipped`). The
/// `Rebuild`-ness is read via [`NavigationController::resolve_traversal`] BEFORE
/// `handle_navigate` commits/re-stamps the cursor, then ANDed with `shipped` so a
/// **failed** rebuild (old document still active) reports `changed_document =
/// false` and a trailing deferred `SyncUpdate` still applies (plan §1 D). A no-op
/// (no target — e.g. a stacked `back(); back()` whose cursor already moved, or an
/// out-of-range `go`) reports the default (`shipped = false`, `changed_document =
/// false`).
pub(super) fn apply_traversal_delta(
    state: &mut ContentState,
    delta: TraversalDelta,
) -> TraversalApplyOutcome {
    let peeked = state.nav_controller.peek_delta(delta);
    // Clone the URL to drop the `nav_controller` borrow before the `&mut state` load.
    let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
        return TraversalApplyOutcome::default();
    };
    // Read the cross-document classification BEFORE `handle_navigate` commits +
    // re-stamps the document identity (else it would compare against the moved
    // cursor). `changed_document` is only true when the rebuild actually landed.
    let is_rebuild = matches!(
        state.nav_controller.resolve_traversal(target_index),
        TraversalKind::Rebuild
    );
    let shipped = handle_navigate(state, &url, HistoryCursorOp::Commit(target_index), None);
    TraversalApplyOutcome {
        shipped,
        changed_document: shipped && is_rebuild,
    }
}

/// Resolve a to-be-deferred `SyncUpdate`'s URL to an ABSOLUTE **call-time** URL
/// (T3). Called at Phase-1b enqueue time, when `base` is the CALL-TIME document
/// URL (`state.pipeline.url` before any deferred traversal applies):
/// - `None` (an omitted `pushState` / `replaceState` URL) → the current document
///   URL (§7.2.5: an absent/empty url resolves against the document's URL at call
///   time — the entry keeps the call-time URL).
/// - `Some(u)` → its absolute resolution against the call-time base; left verbatim
///   only if it does not resolve (Phase-2's apply re-checks scheme/origin).
///
/// This makes the deferred update base-INDEPENDENT at apply time, so a same-turn
/// same-document traversal that moved `pipeline.url` first cannot rebind it to the
/// back target.
fn normalize_deferred_history_url(base: Option<&url::Url>, url: Option<String>) -> Option<String> {
    match url {
        None => base.map(url::Url::to_string),
        Some(u) => Some(
            resolve_nav_url(base, &u)
                .map(|resolved| resolved.to_string())
                .unwrap_or(u),
        ),
    }
}
