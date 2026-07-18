//! Content thread event loop.
//!
//! Extracted from `content/mod.rs` to keep file sizes manageable. The
//! `BrowserToContent` message dispatch lives in the sibling `message_dispatch.rs`.

use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use crossbeam_channel::{RecvTimeoutError, TryRecvError};

use elidex_script_session::HostDriver;

use crate::ipc::{BrowserToContent, ContentToBrowser};

use super::message_dispatch::handle_message;
use super::{
    animation, dispatch_message_event, iframe, parent_message_allowed, ContentState,
    DEFAULT_POLL_INTERVAL, FRAME_INTERVAL,
};

pub(super) fn run_event_loop(state: &mut ContentState) {
    let mut last_frame = Instant::now();
    // One `pump_turn` per event-loop turn; `Break` stops the loop (Shutdown /
    // channel disconnect). Extracted so a test can drive turns one at a time and
    // observe the Phase-1-enqueue / Phase-2-apply task boundary across turns
    // (`pump_turn_applies_enqueued_traversal_on_a_later_turn`).
    while pump_turn(state, &mut last_frame).is_continue() {}
}

/// Process ONE event-loop turn as the message-held skeleton (plan
/// `docs/plans/2026-07-session-history-slice-A-pump-turn-drain-unification.md` §4):
/// (1) intake ONE message and HOLD it (buffer-first, else one `recv_timeout`);
/// (2) if it is a `Shutdown`, tear down before any Phase-2 work (:49); (3) apply the
/// deferred traversal + settle its `popstate`-staged SAME-document sync intent
/// (`drain_synchronous_updates`, NOT a cross-document nav — that defers to step 6)
/// (:416 / :73); (4) dispatch the held message; (5) frame tick; (6) the R9 bottom
/// full drain (the sole cross-document-nav drain). One ordered intake, no separate
/// replay channel. Returns
/// [`ControlFlow::Break`] to stop the loop (Shutdown / channel disconnect),
/// [`ControlFlow::Continue`] otherwise.
#[allow(clippy::too_many_lines)] // Event loop with iframe integration.
pub(super) fn pump_turn(state: &mut ContentState, last_frame: &mut Instant) -> ControlFlow<()> {
    {
        // === Step 0: entry invariant (DOCUMENTED, not a live guard) ===
        // Teardown safety is enforced at the SEAM BOUNDARY, not by the post-drain checks
        // below: every pipeline-mutating `DrainHost` seam fails closed on
        // `shutdown_requested` at entry (see the `impl DrainHost for ContentState` doc
        // comment for the enumerated seams + the `handle_navigation` cause-not-victim
        // exception). Because `DrainCoordinator` touches the pipeline ONLY through those
        // seams, a `Shutdown` handled mid-drain (`drain_host::dispatch_or_buffer_reentrant`,
        // an SW-wait re-dispatch) cannot re-render / ship / rebuild from the torn-down
        // pipeline no matter where in a compound drain it fires — the completeness a
        // post-drain check cannot give (a check placed AFTER a compound drain always
        // leaves a "the next seam ran before the check" hole; Codex PR#469 R14).
        //
        // The pump's own `shutdown_requested` checks (steps 3/4/6) are therefore NOT the
        // teardown-safety mechanism. They (a) promptly EXIT the loop once a mid-drain
        // teardown has set the flag, and (b) guard the DIRECT, non-coordinator pump work
        // — the step-4 held `handle_message` and the step-5 frame tick — which does not
        // pass through a seam. So a turn that returns `Continue` leaves the flag false,
        // and `pump_turn` is only ever re-entered with it clear. This ENTRY invariant is
        // ASSERTED, not live-checked: a runtime `if shutdown_requested { Break }` here
        // would re-add the per-phase shutdown accretion the restructure removed, for a
        // by-construction-unreachable state. The `debug_assert` documents (and, in debug
        // builds, enforces) it without that cost.
        debug_assert!(
            !state.shutdown_requested,
            "pump_turn entry invariant: teardown safety is seam-guarded (every \
             pipeline-mutating DrainHost seam fails closed at entry) and every \
             shutdown_requested set-site Breaks same-turn (steps 3/4/6), so Continue ⟹ flag false"
        );

        // === Step 1: intake ONE message, HELD (plan §4 message-held skeleton) ===
        // A single intake point per turn — the SOLE reader of the channel/buffer, so
        // exactly ONE message is read and held (never a `try_recv` probe that could
        // drop it — crossbeam has no peek/putback; plan §4 IMP-2). Buffer-first: a
        // reentrant message the SW-fetch wait loop deferred while a Phase-2 apply held
        // the peek→commit window (`drain_host::dispatch_or_buffer_reentrant`) is
        // re-delivered ONE per turn through this same path, so it inherits the turn's
        // Phase-2 apply (step 3, before it) and the R9 bottom drain (step 6, after it)
        // — retiring the old top-of-turn replay batch (a 2nd dispatch channel; §0).
        let msg: Option<BrowserToContent> = if state.deferred_reentrant_messages.is_empty() {
            // Wait-duration only (NOT an ordering gate): fold a pending deferred
            // traversal into the timeout as `0` — like a due timer — so an idle turn
            // with a queued traversal returns immediately and step 3 applies it without
            // a poll-interval delay (plan §4.1 Liveness). Same category as the existing
            // animation/timer timeout inputs.
            let timeout = if state.traversal_queue.is_empty() {
                let now_for_timeout = Instant::now();
                let timer_timeout = state
                    .pipeline
                    .runtime
                    .next_timer_deadline()
                    .map(|d| d.saturating_duration_since(now_for_timeout));
                if state.pipeline.animation_engine.has_running() {
                    let next_frame = *last_frame + FRAME_INTERVAL;
                    let frame_remaining = next_frame
                        .saturating_duration_since(now_for_timeout)
                        .max(Duration::from_millis(1));
                    timer_timeout.map_or(frame_remaining, |t| frame_remaining.min(t))
                } else {
                    timer_timeout.unwrap_or(DEFAULT_POLL_INTERVAL)
                }
            } else {
                Duration::ZERO
            };
            match state.channel.recv_timeout(timeout) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return ControlFlow::Break(()),
            }
        } else {
            // Buffer non-empty: FIFO one-per-turn re-delivery. A channel `Shutdown` must NOT
            // be starved behind the buffer OR behind earlier channel work (teardown-priority,
            // step 2) — so DRAIN the channel non-blocking until `Shutdown` or `Empty`,
            // buffering every non-`Shutdown` to the buffer BACK (FIFO preserved — crossbeam has
            // no putback), THEN deliver the buffer FRONT. A single probe would observe only the
            // channel HEAD, leaving a `Shutdown` behind other channel messages starved for later
            // turns (Codex PR#469 R15). The buffer holds ≥1 (this arm's precondition), so the
            // `remove(0)` after an `Empty` is always valid.
            loop {
                match state.channel.try_recv() {
                    Ok(BrowserToContent::Shutdown) => break Some(BrowserToContent::Shutdown),
                    Ok(other) => state.deferred_reentrant_messages.push(other),
                    Err(TryRecvError::Empty) => {
                        break Some(state.deferred_reentrant_messages.remove(0))
                    }
                    Err(TryRecvError::Disconnected) => return ControlFlow::Break(()),
                }
            }
        };

        // === Step 2: teardown-priority (plan §4 :49) ===
        // A `Shutdown` already in hand is handled BEFORE any Phase-2 work — no
        // `popstate` handler / cross-document load runs on a closing tab. Decided on
        // the ALREADY-HELD message, so no channel peek is needed (`Shutdown` is a unit
        // variant — reconstruct it for dispatch). elidex teardown-priority policy,
        // permitted by §8.1.7.3 step 2.1 (the event loop picks "one such task queue,
        // chosen in an implementation-defined manner").
        // `handle_message(Shutdown)` returns `false` iff it ACTUALLY tore down (unload
        // ran) → Break; a `beforeunload`-CANCELED `Shutdown` returns `true` (keep
        // running) — do NOT force the exit, matching the recv-path `Shutdown` contract:
        // consume it and continue this turn message-less (msg → `None`).
        let msg = if matches!(msg, Some(BrowserToContent::Shutdown)) {
            if !handle_message(BrowserToContent::Shutdown, state) {
                return ControlFlow::Break(());
            }
            None
        } else {
            msg
        };

        // === Step 3: Phase-2 apply + popstate SAME-document sync settle ===
        // Phase 2 (§7.4.6.1 *apply the history step*) applies ONLY the traversals a
        // PRIOR turn enqueued (I1: a genuine LATER task — Phase 2 does not itself
        // enqueue; only the drains do, all at/after this point). A same-document
        // traversal fires `popstate` SYNCHRONOUSLY here, whose handler may stage a
        // same-document `pushState`/`replaceState` (`pending_history`) AND/OR a
        // CROSS-document `location.*` (`pending_navigation`).
        //
        // The top drain is `drain_synchronous_updates` — Phase 1a (window-opens) +
        // Phase 1b (same-document `pending_history`), but NOT Phase 1c
        // (`handle_navigation`, the cross-document `pending_navigation` apply). This
        // settles the popstate-staged SAME-document `pushState` into the
        // NavigationController BEFORE step 4's held message (the :73 property — the
        // committed entry survives a held-Navigate rebuild), while a popstate-staged
        // CROSS-document navigation is NEVER drained at the top (step 3): it is drained
        // at step 4's input handler (in-task, AFTER the event dispatched) or the step-6
        // bottom `drain_synchronous_phase` (Phase 1c) — both AFTER the held input
        // dispatches. A blocking cross-document load rebuilds `state.pipeline`; running
        // it here (before step 4) would make a held `MouseClick`/`KeyDown` hit the WRONG
        // document. Per spec a `location.assign` completes in a LATER task, so an
        // already-pending input (older task) must process against the pre-navigation
        // document — hence the cross-document nav is never applied before the input.
        // `run_deferred_traversals` drains only the traversal queue (NOT the
        // history/nav FIFO), so this updates drain is what commits the popstate
        // same-document intent.
        let _ = elidex_navigation::DrainCoordinator::run_deferred_traversals(state);
        let _ = elidex_navigation::DrainCoordinator::drain_synchronous_updates(state);
        // The step-3 drains can reach `handle_navigate`'s SW-wait (via a Phase-2 apply
        // or the top drain), where a re-dispatched `Shutdown` runs teardown + sets
        // `shutdown_requested`. Those drains' seams are now seam-guarded (fail closed at
        // entry — no post-teardown pipeline mutation), so this check is NOT the
        // teardown-safety mechanism: it is a prompt loop-exit, breaking before step 4's
        // DIRECT held-message dispatch touches the torn-down pipeline (Codex PR#469 R14).
        if state.shutdown_requested {
            return ControlFlow::Break(());
        }

        // === Step 4: dispatch the HELD message (after Phase-2 + same-doc settle) ===
        // `msg` is non-`Shutdown` here (step 2 exited on `Shutdown`). It dispatches
        // AFTER the queued traversal applied (step 3) — so a direct `Navigate` can
        // never overtake a queued traversal (:416) — and after the popstate
        // same-document intent was settled — so a rebuild here cannot sever it (:73).
        // Because step 3 did NOT run Phase 1c, a popstate-staged CROSS-document
        // navigation is still pending here, so a held input hits the pre-nav document;
        // that nav applies below at step 6.
        if let Some(msg) = msg {
            // `handle_message` returns `false` on `Shutdown` (already excluded) — the
            // `false`/`Break` path stays for the recv contract's completeness.
            if !handle_message(msg, state) {
                return ControlFlow::Break(());
            }
            // The step-4 held `handle_message` is DIRECT pump work (not a coordinator
            // seam), so it is guarded HERE, not at a seam: a fresh navigation's
            // (non-nested) SW-wait can see a re-dispatched `Shutdown` — `handle_navigate`
            // ran teardown + set `shutdown_requested` and returned without applying, but
            // its caller `handle_message` returned `true`. Break so the thread exits
            // without a frame tick on the torn-down pipeline (prompt loop-exit +
            // direct-work guard; Codex PR#469 R14).
            if state.shutdown_requested {
                return ControlFlow::Break(());
            }
            if state.pipeline.animation_engine.has_active() {
                state.pipeline.prune_dead_animation_entities();
            }
        }

        // === Step 5: frame tick (animations / timers / iframe / network / render) ===
        // Every JS phase that may STAGE a nav intent (drained by the step-6 bottom
        // drain): `drain_timers`, `dispatch_message_event`, `tick_network`,
        // `drain_worker_messages`, plus the re-render.
        let now = Instant::now();
        let dt = now.duration_since(*last_frame);
        let mut needs_render = false;

        if state.pipeline.animation_engine.has_active() && dt > Duration::ZERO {
            let dt_secs = dt.min(FRAME_INTERVAL * 2).as_secs_f64();
            let events = state.pipeline.animation_engine.tick(dt_secs);
            animation::dispatch_animation_events(&events, state);
            *last_frame = now;
            needs_render = true;
        }

        if state
            .pipeline
            .runtime
            .next_timer_deadline()
            .is_some_and(|d| d <= now)
        {
            state.pipeline.drain_timers();
            needs_render = true;
        }

        // --- Iframe + messaging frame tick ---
        // Under the VM a depth-0 `window.postMessage` self-delivers internally
        // (with the §9.3.3 step 8.1 gate applied inline in `dispatch_post_message`),
        // so the shell no longer drains top-level self-messages. Iframe→parent
        // messages are drained/gated below (2f4-c). Message-handler DOM mutations
        // that need a re-render ride the §4.3.8 inclusive-descendants version-delta
        // (below), the same signal the realtime/worker drains use.
        // §9.3.3 step 8.1: gate every iframe→parent message against the parent
        // window's origin key at THIS single chokepoint (fail-closed — a message
        // whose resolved `targetOrigin` is neither `*` nor the parent key is
        // dropped). Both transports (OOP IPC + in-process) are normalised onto
        // one `Vec<ParentMessage>` by `drain_parent_messages`, so the gate sees
        // one input shape. `parent_key` is the parent's `storage_origin_key`
        // (byte-identical to the send-side resolution).
        let parent_key = state.pipeline.runtime.storage_origin_key();
        for msg in state.iframes.drain_parent_messages() {
            if parent_message_allowed(&parent_key, &msg.target_origin) {
                dispatch_message_event(state, &msg.data, &msg.origin);
            }
        }

        for change in state.pipeline.runtime.take_pending_storage_changes() {
            let _ = state.channel.send(ContentToBrowser::StorageChanged {
                origin: change.origin,
                key: change.key,
                old_value: change.old_value,
                new_value: change.new_value,
                url: change.url,
            });
        }

        for req in state
            .pipeline
            .runtime
            .take_pending_idb_versionchange_requests()
        {
            let _ = state
                .channel
                .send(ContentToBrowser::IdbVersionChangeRequest {
                    request_id: req.request_id,
                    origin: req.origin,
                    db_name: req.db_name,
                    old_version: req.old_version,
                    new_version: req.new_version,
                });
        }

        // Drain all four `ServiceWorkerContainer`/`ServiceWorkerRegistration`/
        // `ServiceWorker` client requests and route each onto its browser IPC.
        // The VM queues all four arms and settles their promises via
        // `deliver_sw_client_update`, so dropping any arm would HANG its
        // promise (§8). A page with no current_url cannot own SW requests.
        if let Some(current_url) = state.pipeline.runtime.current_url() {
            for req in state.pipeline.runtime.drain_sw_client_requests() {
                match req {
                    elidex_api_sw::SwClientRequest::Register {
                        script_url,
                        scope,
                        update_via_cache,
                    } => {
                        // URLs are already resolved against the doc base
                        // (canonical) — parse verbatim, no join/default_scope.
                        let (Ok(script_url), Ok(scope)) =
                            (url::Url::parse(&script_url), url::Url::parse(&scope))
                        else {
                            continue;
                        };
                        let origin = current_url.origin().unicode_serialization();
                        let _ = state.channel.send(ContentToBrowser::SwRegister {
                            script_url,
                            scope,
                            origin,
                            page_url: current_url.clone(),
                            update_via_cache,
                        });
                    }
                    elidex_api_sw::SwClientRequest::Update { scope } => {
                        let Ok(scope) = url::Url::parse(&scope) else {
                            continue;
                        };
                        let _ = state.channel.send(ContentToBrowser::SwUpdate { scope });
                    }
                    elidex_api_sw::SwClientRequest::Unregister { scope } => {
                        let Ok(scope) = url::Url::parse(&scope) else {
                            continue;
                        };
                        let _ = state.channel.send(ContentToBrowser::SwUnregister { scope });
                    }
                    elidex_api_sw::SwClientRequest::PostMessage { scope, data } => {
                        let Ok(scope) = url::Url::parse(&scope) else {
                            continue;
                        };
                        let origin = current_url.origin().unicode_serialization();
                        let client_id = state.client_id.clone();
                        let _ = state.channel.send(ContentToBrowser::SwPostMessage {
                            scope,
                            data,
                            origin,
                            client_id,
                        });
                    }
                }
            }
        }

        // Persist any `localStorage` origins this turn's scripts dirtied to disk.
        // The VM-native Storage natives write through the shell-owned
        // `WebStorageManager` (installed at the pipeline construction seam); this
        // per-turn flush is the disk-persistence half (F14 / §4.3.3). Replaced the
        // pre-flip boa `flush_dirty_stores()` (which flushed the boa registry the VM
        // never writes to); the boa bridge was removed in D-26 PR7.
        state.web_storage.flush_dirty();

        // window.open — route the ordered tab-creation / named-navigation
        // queue (a user-visible chrome action, so we wake: a pure-async
        // window.open with no DOM change would otherwise stall under Wait).
        // Drained via the engine-agnostic session trait surface, not the boa
        // bridge — the S5-6 flip swaps the runtime type here too (memo §4.3.2 /
        // edge E4). Same ordered routing as the Phase-1a `route_window_opens` seam
        // (driven by `drain_synchronous_phase` on an input turn): a named-target
        // open from a pure-async turn (timer / postMessage) MUST drain here too,
        // not only `_blank`, or it would strand forever. A routed named HIT
        // re-navigates an iframe → re-render.
        let window_opens = state.pipeline.runtime.take_pending_window_opens();
        needs_render |= super::navigation::route_window_opens(state, window_opens).navigated_iframe;

        // NOTE: Phase 2 (§7.4.6.1 *apply the history step*) is applied at step 3 of
        // the turn, NOT here — so a traversal enqueued by THIS turn's input handler
        // (`handle_message` → `drain_synchronous_phase`) applies on the NEXT pump
        // turn, a genuine later task (plan §4.5 I1). See the step-3 apply.

        if state.pipeline.runtime.take_pending_focus() {
            state.notify_browser(ContentToBrowser::FocusWindow);
        }

        // §4.3.8: the VM-native network turn — settle fetch(), dispatch WS/SSE,
        // run the microtask checkpoint. `needs_render` for any realtime/fetch-
        // driven DOM mutation is restored by the inclusive-descendants version-
        // delta below (stage 2d-2), which subsumed the boa per-drain `has_js_events`
        // bool this block used to carry.
        state.pipeline.tick_network();

        // §4.3.8: worker-drive needs_render now comes from the version-delta (stage 2d-2).
        state.pipeline.drain_worker_messages();

        iframe::tick_iframe_timers(state);

        if state.update_caret_blink() {
            needs_render = true;
        }

        // §4.3.8: any DOM-tree mutation this turn (worker/timer/dispatch-driven) moved the
        // document-root version — restore the needs_render signal the boa per-drain bools carried.
        if state
            .pipeline
            .dom
            .inclusive_descendants_version(state.pipeline.document)
            != state.last_render_dom_version
        {
            needs_render = true;
        }

        // §4.3.8: two render effects bump NO DOM-tree version, so the delta above
        // misses a turn whose ONLY visible effect is one of them — restore the
        // explicit render-dirty signal the pre-flip loop carried. Both drain
        // INSIDE `re_render` (scroll via `take_pending_scroll`, canvas via
        // `sync_dirty_canvases`), so a pure-async handler (WS/SSE/worker/
        // postMessage) that only `scrollTo`s or draws a canvas would otherwise
        // leave `needs_render` false and stall until a later render/interaction.
        // PEEK, don't consume — `re_render` stays the single drain point.
        if state.pipeline.runtime.has_pending_scroll() || state.pipeline.has_dirty_canvas() {
            needs_render = true;
        }

        if needs_render {
            state.re_render();
            state.send_display_list();
        }

        // === Step 6: R9 bottom drain (FULL Phase 1, incl. 1c cross-document nav) ===
        // Phase 1 (§7.4.2 last-wins navigation / §7.4.4 synchronous history
        // updates) — the R9 BOTTOM drain, the FULL `drain_synchronous_phase` (1a
        // window-opens + 1b same-document sync + 1c CROSS-document nav), run AFTER
        // every frame-tick JS phase above: `drain_timers`, `dispatch_message_event`
        // (postMessage), `tick_network` (fetch callbacks), `drain_worker_messages`,
        // and any non-input step-4 held message handler (e.g. a
        // `resize`/`visibilitychange` listener). Any of those callbacks may call
        // `location.assign()` / `history.pushState()` / `history.back()`, staging
        // intents in the VM `pending_navigation` / `pending_history` buffers.
        //
        // The drain partition (top `drain_synchronous_updates` vs bottom
        // `drain_synchronous_phase`) is ASYMMETRIC, not just temporal:
        //   · top (step 3) = 1a + 1b only — same-document `pushState`/`replaceState`
        //     (the popstate intent the :73 property protects) + window-opens.
        //   · bottom (step 6) = 1a + 1b + 1c — runs Phase 1c (`handle_navigation`), the
        //     CROSS-document `pending_navigation` drain.
        // A popstate-staged cross-document `location.assign` is NEVER drained at the
        // top (step 3): it is drained at step 4's input handler (in-task Phase 1c, AFTER
        // the event dispatched) or here at step 6 — both AFTER the step-4 held input
        // dispatched, so the input hits the pre-navigation document (spec:
        // `location.assign` completes in a later task than an already-pending input) and
        // the cross-document nav applies as that later task. The single VM FIFO stays
        // the ordering SoT; the same-document
        // `pending_history` is drained by whichever of top/bottom first observes it
        // (take-consumed, no double-apply). Without this drain a nav staged by a
        // timer/fetch/worker callback sits UNPROCESSED until an unrelated later INPUT
        // turn drained it — the "navigation stuck" bug (Codex PR#469 R9). The
        // callback-staged nav applies in-task (a `location.*` / `pushState`), or a
        // `Back`/`Forward`/`Go` is ENQUEUED for a LATER turn's step-3
        // `run_deferred_traversals` — NOT applied this turn (`drain_synchronous_*`
        // only enqueues traversals, never applies them; the apply is Phase 2, at step
        // 3 of a later turn), preserving the §4.5 I1 task boundary.
        //
        // Window-open routing stays EXACTLY-ONCE across THREE drain points this turn,
        // each consuming via `take_pending_window_opens` (a take, not a peek), so an
        // open is routed by whichever point first observes it and never twice: (1) the
        // step-3 TOP `drain_synchronous_updates` Phase-1a seam (opens staged by the
        // Phase-2 popstate apply); (2) the frame-tick `route_window_opens` above (opens
        // staged by the held message / `drain_timers` / postMessage phases); (3) this
        // bottom drain's Phase-1a seam (`DrainHost::route_window_opens`), taking only
        // the LEFTOVER opens staged by the later `tick_network` / `drain_worker_messages`
        // phases. A temporal partition of one take-consumed queue, never the same intent
        // twice. The returned `DrainOutcome` is ignored: a pump turn has no `<a href>`
        // default to suppress, and the drain ships its own frames (`ship_if_needed` /
        // `handle_navigate`).
        let _ = elidex_navigation::DrainCoordinator::drain_synchronous_phase(state);

        // The step-6 bottom drain (`drain_synchronous_phase`) can reach
        // `handle_navigate`'s SW-wait via a callback-staged `location.*` nav, where a
        // re-dispatched `Shutdown` runs teardown + sets `shutdown_requested`
        // (`dispatch_or_buffer_reentrant`). Its seams are seam-guarded (fail closed at
        // entry), so this is NOT the "check after EVERY phase" catch-all it once was: it
        // is a prompt loop-exit — break after a bottom-drain teardown so the next turn
        // does not re-enter on the torn-down pipeline (before its `recv_timeout` blocks
        // the poll interval). Nothing direct runs after this drain THIS turn; step 5's
        // frame-tick work already ran on the live pipeline (gated by the step-4 check —
        // step 5 stages navs for this drain but cannot itself reach the SW-wait teardown)
        // (Codex PR#469 R14).
        if state.shutdown_requested {
            return ControlFlow::Break(());
        }
    }

    ControlFlow::Continue(())
}

#[cfg(test)]
mod tests {
    use elidex_script_session::HostDriver;

    use crate::build_pipeline_interactive;

    /// Regression (S5-6b flip): a turn whose ONLY visible effect is a
    /// `window.scrollTo` moves NO DOM-tree version, so the event loop's
    /// inclusive-descendants version-delta render-dirty check (event_loop.rs
    /// `if needs_render` gate) would miss it and the scroll would stall. The
    /// scroll drains INSIDE `re_render` (`take_pending_scroll`), so the loop must
    /// peek `has_pending_scroll()` to schedule that render. Proves the peek fires
    /// while the DOM version is unchanged, and that peeking does NOT consume.
    #[test]
    fn scroll_only_turn_flags_render_without_dom_version_bump() {
        let mut pipeline = build_pipeline_interactive(
            "<body><div id='a'>hi</div><script>window.scrollTo(0, 100);</script></body>",
            "",
        );
        let doc = pipeline.document;
        // The scrollTo ran during build; its request is pending (undrained — the
        // build path never calls take_pending_scroll).
        let version_after_scroll = pipeline.dom.inclusive_descendants_version(doc);

        // The event-loop render-dirty peek picks it up...
        assert!(
            pipeline.runtime.has_pending_scroll(),
            "scrollTo must leave a pending scroll the render-dirty gate can see"
        );
        // ...and the peek is NON-consuming (unlike take_pending_scroll).
        assert!(
            pipeline.runtime.has_pending_scroll(),
            "has_pending_scroll must not consume the pending scroll"
        );
        // The version-delta signal alone would MISS this turn: the peek did not
        // move the DOM-tree version.
        assert_eq!(
            version_after_scroll,
            pipeline.dom.inclusive_descendants_version(doc),
            "a scroll-only turn bumps no DOM-tree version"
        );

        // The gate expression the event loop evaluates fires.
        let needs_render = pipeline.runtime.has_pending_scroll() || pipeline.has_dirty_canvas();
        assert!(needs_render, "scroll-only turn must schedule re_render");

        // re_render's drain (take_pending_scroll) clears the signal.
        assert_eq!(pipeline.runtime.take_pending_scroll(), Some((0.0, 100.0)));
        assert!(!pipeline.runtime.has_pending_scroll());
    }

    /// Regression (S5-6b flip): a turn whose ONLY visible effect is a canvas draw
    /// inserts a `CanvasDirty` marker but bumps NO DOM-tree version, so the
    /// version-delta gate would miss it and the draw would stall until a later
    /// render. The pixels flush INSIDE `re_render` (`sync_dirty_canvases`), so the
    /// loop must peek `has_dirty_canvas()`. Proves the peek fires while the DOM
    /// version is unchanged.
    #[test]
    fn canvas_only_turn_flags_render_without_dom_version_bump() {
        let mut pipeline = build_pipeline_interactive("<body><canvas id='c'></canvas></body>", "");
        let doc = pipeline.document;
        let canvas = pipeline.dom.query_by_tag("canvas")[0];

        let version_before = pipeline.dom.inclusive_descendants_version(doc);
        assert!(
            !pipeline.has_dirty_canvas(),
            "no canvas is dirty before the draw"
        );

        // A draw marks the canvas dirty (HTML §4.12.5) without touching the DOM
        // tree — set the marker directly (the raster op path is exercised by the
        // canvas crate's own tests).
        elidex_api_canvas::mark_dirty(&mut pipeline.dom, canvas);

        assert_eq!(
            version_before,
            pipeline.dom.inclusive_descendants_version(doc),
            "marking a canvas dirty bumps no DOM-tree version"
        );

        // The event-loop render-dirty gate now picks it up where the version-delta
        // signal alone would not.
        let needs_render = pipeline.runtime.has_pending_scroll() || pipeline.has_dirty_canvas();
        assert!(
            needs_render,
            "canvas-only turn must schedule re_render via has_dirty_canvas()"
        );
    }
}
