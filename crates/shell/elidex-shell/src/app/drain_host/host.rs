//! The app-mode `impl DrainHost for App` seam bodies — the CALL-BACK half of the
//! drive site next door.
//!
//! Split from the drive site at its own cohesion seam (touch-time 1000-line
//! discipline), mirroring how `elidex-navigation` splits
//! `traversal_queue/{coordinator,host}.rs`. The two halves run in opposite
//! directions and have different readers: [`super`] owns the SCHEDULE (the
//! turn-completion loop, its cap, the quiescence predicate and the swap marker)
//! and calls INTO the coordinator; this module owns the phase-drain
//! BODIES the coordinator calls BACK into, plus the one free function that only
//! serves them — the Phase-2 traversal-apply body [`apply_traversal_delta`].
//!
//! Everything the seams here rest on — the schedule, the reentrancy premises, the
//! `debug_assert` pair — is stated once, in the parent module's doc.

use elidex_navigation::{
    DrainHost, PendingTraversal, TraversalDelta, TraversalQueue, UserInvolvement,
};
use elidex_script_session::{HistoryAction, HostDriver};

use crate::app::navigation::{handle_history_action, resolve_nav_url};
use crate::app::App;

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
    /// [`own_context_action`](elidex_navigation::DrainOutcome::own_context_action) / [`shipped`](elidex_navigation::DrainOutcome::shipped)
    /// instead of short-circuiting the drain.
    ///
    /// **⚠ That `true` is UNCONDITIONAL — the known applied/shipped conflation**
    /// (slot `#11-nav-applied-shipped-decouple`, carved on PR #469 R15).
    /// [`App::navigate`](super::App::navigate) returns `()` and early-returns when
    /// `load_url_into_pipeline` fails, so a **failed** load still reports `true`
    /// here — setting BOTH [`own_context_action`](elidex_navigation::DrainOutcome::own_context_action)
    /// (→ the `<a href>` click default is suppressed) AND
    /// [`shipped`](elidex_navigation::DrainOutcome::shipped) (→ the coordinator's trailing
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
pub(in crate::app) fn apply_traversal_delta(app: &mut App, delta: TraversalDelta) -> bool {
    let peeked = app.inline_state().nav_controller.peek_delta(delta);
    // Clone the URL to drop the `nav_controller` borrow before the `&mut app` apply.
    let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
        return false;
    };
    app.traverse_to(target_index, &url)
}
