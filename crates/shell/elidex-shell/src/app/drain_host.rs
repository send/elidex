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
//! [`DrainCoordinator::drain_same_turn`]. The shells differ ONLY in WHEN Phase 2 is
//! pumped: content-mode on a later async-pump turn
//! ([`run_deferred_traversals`](DrainCoordinator::run_deferred_traversals)),
//! app-mode back-to-back inside the input handler
//! ([`drain_same_turn`](DrainCoordinator::drain_same_turn)) — app-mode has no async
//! pump, so its Phase 2 is a *degenerate* later task (Q-SCHED option (i)). That
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
//! has no pump, so a mid-apply-serialized step would strand until the next input
//! event. It cannot happen here, because the inline path has **no reentrancy
//! vector at all**:
//!
//! 1. This drive runs EXCLUSIVELY on the legacy-inline `InteractiveState` path
//!    ([`App::new_interactive`] / [`App::new_interactive_with_url`]), reached only
//!    from `events::handle_click` / `events::handle_keyboard`. Threaded mode uses a
//!    different method set that messages the content thread, which runs its own
//!    content-mode `DrainHost`.
//! 2. The inline path has NO service-worker machinery: `new_interactive*` set
//!    `network_process: None` and `origin_storage: None`.
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
//!    the [`TraversalQueue`] only on the NEXT drain (the next input turn), never
//!    re-entering the current one. Any future change to an apply body MUST preserve
//!    this — eagerly re-draining pending nav from inside an apply would re-open the
//!    mid-apply re-enqueue vector.
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

/// The `.expect()` message for every per-seam reach-through to
/// [`App::interactive`](super::App::interactive).
///
/// An **unreachable panic**, not a fallible unwrap: the sole drive site
/// ([`App::process_pending_navigation`]) enters only when `interactive.is_some()`,
/// and nothing in the crate ever clears the field afterwards (see its
/// never-cleared invariant in `app/mod.rs`). Reaching it would mean a second,
/// unguarded coordinator drive was introduced — which must fail loudly rather than
/// silently no-op half a drain.
pub(super) const INTERACTIVE_DRIVE_ONLY: &str =
    "the DrainCoordinator is driven only from process_pending_navigation, which enters \
     behind an `interactive.is_some()` guard, and `interactive` is never cleared";

impl App {
    /// Drive one **whole-turn** session-history / navigation drain for the
    /// legacy-inline shell — the app-mode leg of the shared phase-partition.
    ///
    /// Called at the end of an input handler (`events::handle_click` /
    /// `events::handle_keyboard`), after event dispatch + re-render. Runs
    /// [`DrainCoordinator::drain_same_turn`], whose body sequences **Phase 1**
    /// (window-opens §7.2.2.1 → §7.4.4 synchronous `pushState`/`replaceState`
    /// updates applied in-task, with §7.4.3 `Back`/`Forward`/`Go` traversals merely
    /// *enqueued* → §7.4.2 last-wins own-context navigation) strictly BEFORE
    /// **Phase 2** (the §7.4.6.1 deferred traversal apply), then ships at most one
    /// frame. That call ordering IS app-mode's realization of the task boundary
    /// (I1, app leg): every Phase-1 write to the entry list lands before any
    /// Phase-2 apply reads it.
    ///
    /// **This is the SOLE site that drives the coordinator in app-mode** — the
    /// `interactive.is_some()` guard here is what makes every per-seam
    /// [`INTERACTIVE_DRIVE_ONLY`] `expect` an unreachable panic.
    ///
    /// Returns the coordinator's [`DrainOutcome`] (the shared summary both shells
    /// return) rather than the retired ad-hoc `bool`. Callers read the field they
    /// need: `handle_click` consumes
    /// [`suppress_default`](DrainOutcome::suppress_default) to drop the `<a href>`
    /// default navigation; `handle_keyboard` calls for effect and ignores it. When
    /// `interactive` is absent (threaded mode) no drain runs and the default
    /// outcome — every field `false`, i.e. "nothing happened, suppress nothing" —
    /// is returned.
    pub(super) fn process_pending_navigation(&mut self) -> DrainOutcome {
        if self.interactive.is_none() {
            return DrainOutcome::default();
        }
        DrainCoordinator::drain_same_turn(self)
    }
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
/// `self.interactive.as_mut().expect(`[`INTERACTIVE_DRIVE_ONLY`]`)` — a bounded,
/// provably-safe wrinkle (unreachable panic), not an ownership gap.
///
/// **Layering.** The coordinator owns the phase ordering + the I1/I2/I3 invariants;
/// these seams own the irreducibly shell-specific bodies. `App` /
/// `InteractiveState` / the pipeline / `EcsDom` / the winit window stay **behind the
/// trait** and never cross the `elidex-navigation` crate boundary — the coordinator
/// touches the OS window only through [`ship_frame`](Self::ship_frame).
///
/// **No teardown guards.** Content-mode fails every pipeline-mutating seam closed on
/// `shutdown_requested`, because its `Shutdown` can be handled mid-drain at the
/// SW-wait reentrancy vector. App-mode has no message pump and no SW-wait inside a
/// drain (module doc, premises 2–4), so there is no mid-drain teardown to guard
/// against — adding one would be a guard for an unreachable state.
impl DrainHost for App {
    fn traversal_queue(&mut self) -> &mut TraversalQueue {
        &mut self
            .interactive
            .as_mut()
            .expect(INTERACTIVE_DRIVE_ONLY)
            .traversal_queue
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
            .interactive
            .as_mut()
            .expect(INTERACTIVE_DRIVE_ONLY)
            .pipeline
            .runtime
            .take_pending_window_opens();
    }

    fn take_pending_history(&mut self) -> Vec<HistoryAction> {
        // The VM `pending_history` FIFO in issue order (each synchronous
        // `pushState`/`replaceState` an independent session-history commit;
        // `Back`/`Forward`/`Go` staged as enqueue-only). Q-VM-MODEL: the staging
        // model is unchanged and identical to content's — only the shell drain
        // re-times.
        self.interactive
            .as_mut()
            .expect(INTERACTIVE_DRIVE_ONLY)
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
    fn classify_traversal(&mut self, delta: TraversalDelta) -> Option<PendingTraversal> {
        let in_range = self
            .interactive
            .as_ref()
            .expect(INTERACTIVE_DRIVE_ONLY)
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
    /// an earlier one), **drain-and-DISCARD**: the slot IS drained (this is its only
    /// drain) so a suppressed `location.*` cannot re-fire a turn late, but the
    /// request is dropped without applying — a queued traversal supersedes it
    /// (§7.4.2.2 *Beginning navigation* step 19, "Any attempts to navigate a
    /// navigable that is currently traversing are ignored").
    ///
    /// Returns `true` iff a navigation applied. This is where the retired
    /// hand-rolled drain's `nav-applied` early `return true` now lives: "a
    /// navigation applied" flows through
    /// [`DrainOutcome::own_context_action`]/[`shipped`](DrainOutcome::shipped)
    /// instead of short-circuiting the drain. `navigate` performs the shell's own
    /// output for this leg (rebuild/same-document apply + `set_title`), so the
    /// coordinator's trailing [`ship_frame`](Self::ship_frame) is correctly
    /// suppressed for it.
    fn handle_navigation(&mut self, suppress: bool) -> bool {
        let interactive = self.interactive.as_mut().expect(INTERACTIVE_DRIVE_ONLY);
        let Some(nav_req) = interactive.pipeline.runtime.take_pending_navigation() else {
            return false;
        };
        if suppress {
            return false;
        }
        let Some(target_url) = resolve_nav_url(interactive.pipeline.url.as_ref(), &nav_req.url)
        else {
            return false;
        };
        self.navigate(&target_url, nav_req.nav_type);
        true
    }

    /// **Phase 2** — apply ONE deferred traversal (§7.4.6.1 *Updating the
    /// traversable*) via the shared peek-then-commit body, returning `true` iff it
    /// applied and shipped. Called inside the coordinator's nested-apply guard
    /// bracket (inert in app-mode — module doc).
    fn apply_traversal(&mut self, traversal: &PendingTraversal) -> bool {
        apply_traversal_delta(self, traversal.delta)
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
    /// On a drain turn the dispatch layer redraws too — it always has, and it covers
    /// the input handler's own effects (hover/active state, the dispatched event's
    /// re-render, the `<a href>` default navigation), not the drain's. winit
    /// coalesces concurrent requests into one `RedrawRequested`, so that is a second
    /// *request*, never a second frame; keeping the seam's own output here is what
    /// makes the drain self-sufficient instead of dependent on its caller's dispatch
    /// layer — the property `ContentState::ship_frame` has via `send_display_list`.
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
/// A deferred traversal carries its delta **un-resolved** (§7.4.6.1 resolves the
/// target step at *apply* time against the possibly-Phase-1-mutated entry list), so
/// this resolves it here — `peek_delta` → `(target_index, url)` — and hands the
/// resolved pair to the index-keyed [`App::traverse_to`], which owns the actual
/// peek-then-commit apply. `None` is a no-op traversal (out of range, or a stacked
/// `back(); back()` whose cursor already moved) and returns `false` without touching
/// the document, so the coordinator marks no own-context action and the caller's
/// default is not over-suppressed.
///
/// **One body, three entry points — no duplication.** `traverse_to` stays index-keyed
/// because it also serves the two **non-drain** traversal callers, which already
/// hold a resolved `(index, url)` and are fenced OUT of the coordinator this slice:
/// the chrome toolbar Back/Forward (`navigation.rs::handle_chrome_action`) and
/// Alt+←/→ (`inline.rs::handle_keyboard_inline`). This is the exact shape of the
/// content mirror, where the delta-keyed `apply_traversal_delta` resolves and then
/// calls the shared index/op-keyed `handle_navigate`.
pub(super) fn apply_traversal_delta(app: &mut App, delta: TraversalDelta) -> bool {
    let peeked = app
        .interactive
        .as_ref()
        .expect(INTERACTIVE_DRIVE_ONLY)
        .nav_controller
        .peek_delta(delta);
    // Clone the URL to drop the `nav_controller` borrow before the `&mut app` apply.
    let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
        return false;
    };
    app.traverse_to(target_index, &url)
}
