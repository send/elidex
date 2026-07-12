//! Content thread event loop and message dispatch.
//!
//! Extracted from `content/mod.rs` to keep file sizes manageable.

use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;

use elidex_api_sw::SwClientUpdate;
use elidex_script_session::HostDriver;

use crate::ipc::{BrowserToContent, ContentToBrowser};

use super::{
    animation, dispatch_message_event, dispatch_storage_event, iframe, navigation,
    parent_message_allowed, scroll, ContentState, DEFAULT_POLL_INTERVAL, FRAME_INTERVAL,
};
use super::{event_handlers, ime};

#[allow(clippy::too_many_lines)] // Event loop with iframe integration.
pub(super) fn run_event_loop(state: &mut ContentState) {
    let mut last_frame = Instant::now();

    loop {
        let animations_running = state.pipeline.animation_engine.has_running();

        let now_for_timeout = Instant::now();
        let timer_timeout = state
            .pipeline
            .runtime
            .next_timer_deadline()
            .map(|d| d.saturating_duration_since(now_for_timeout));
        let timeout = if animations_running {
            let next_frame = last_frame + FRAME_INTERVAL;
            let frame_remaining = next_frame
                .saturating_duration_since(now_for_timeout)
                .max(Duration::from_millis(1));
            timer_timeout.map_or(frame_remaining, |t| frame_remaining.min(t))
        } else {
            timer_timeout.unwrap_or(DEFAULT_POLL_INTERVAL)
        };

        match state.channel.recv_timeout(timeout) {
            Ok(msg) => {
                // `handle_message` returns `false` on `Shutdown` (after dispatching
                // unload) — stop the loop before this iteration's frame tick so no
                // script/render work runs after unload.
                if !handle_message(msg, state) {
                    break;
                }
                if state.pipeline.animation_engine.has_active() {
                    state.pipeline.prune_dead_animation_entities();
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // --- Frame tick: animations + timers ---
        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        let mut needs_render = false;

        if state.pipeline.animation_engine.has_active() && dt > Duration::ZERO {
            let dt_secs = dt.min(FRAME_INTERVAL * 2).as_secs_f64();
            let events = state.pipeline.animation_engine.tick(dt_secs);
            animation::dispatch_animation_events(&events, state);
            last_frame = now;
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

        elidex_js_boa::bridge::local_storage::flush_dirty_stores();

        // window.open — route the ordered tab-creation / named-navigation
        // queue (a user-visible chrome action, so we wake: a pure-async
        // window.open with no DOM change would otherwise stall under Wait).
        // Drained via the engine-agnostic session trait surface, not the boa
        // bridge — the S5-6 flip swaps the runtime type here too (memo §4.3.2 /
        // edge E4). Same ordered routing as `process_pending_actions`: a
        // named-target open from a pure-async turn (timer / postMessage) MUST
        // drain here too, not only `_blank`, or it would strand forever. A
        // routed named HIT re-navigates an iframe → re-render.
        let window_opens = state.pipeline.runtime.take_pending_window_opens();
        needs_render |= super::navigation::route_window_opens(state, window_opens).navigated_iframe;

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

        if needs_render {
            state.re_render();
            state.send_display_list();
        }
    }
}

/// Whether a coordinate-bearing input event mapped against `placement_seq` is stale:
/// the browser hit-mapped its coordinates against a viewport the build/runtime has
/// since **superseded** (its seq is below the high-water mark `applied_viewport_seq`).
/// This happens when a resize lands during a blocking load, input is sent against that
/// resize's placement, then a newer resize (or the build) advances the mark past it —
/// the `SetViewport` staleness guard drops the intermediate resize, so its coordinates
/// no longer match any applied layout. Such input is dropped rather than hit-tested
/// against the current layout (which would target the wrong element). The input half
/// of the `ViewportCell` seq reconciliation (plan-memo §10), completing the
/// `SetViewport` viewport-staleness guard above; see `BrowserToContent::MouseMove`.
fn input_placement_stale(placement_seq: u64, state: &ContentState) -> bool {
    placement_seq < state.applied_viewport_seq
}

/// Apply device facts (dppx / `prefers-color-scheme`) carried by a delivery to the JS
/// bridge, guarded by the `facts_seq` high-water mark — the facts analog of the
/// `SetViewport` `seq` guard. Returns `true` iff the facts were **fresh and actually
/// changed** a bridge value (so the caller must re-evaluate media queries); `false` for a
/// stale or value-unchanged delivery (no re-eval needed).
///
/// Two guards, mirroring `SetViewport`'s pair:
/// - **Staleness** (`facts_seq ≤ applied_facts_seq`): an older DPI/theme change already
///   folded into the facts the build read from the cell. Re-applying would flash the
///   document backward (and fire a spurious MQL `change`) before a later queued fact
///   restores the latest. The value-guard cannot catch this — an older fact that
///   *differs* from the freshly-seeded bridge passes it.
/// - **Value-guard**: a construction-seeded tab's resume-time fan-out repeats facts it
///   was born with; only a real change needs a re-eval.
///
/// The high-water mark advances on **any** fresh generation (even a value-unchanged one)
/// so a later equal-`facts_seq` delivery is correctly judged stale (the `SetViewport`
/// seq-bookkeeping discipline). Shared by the `SetDeviceFacts` arm (facts-only frame) and
/// the `SetViewport` arm (which carries the settled facts so a frame changing both size
/// and facts applies them atomically — one re-eval, no intermediate; C3 R2).
fn apply_device_facts(
    state: &mut ContentState,
    color_scheme: elidex_css::media::ColorScheme,
    dppx: f64,
    facts_seq: u64,
) -> bool {
    if facts_seq <= state.applied_facts_seq {
        return false;
    }
    state.applied_facts_seq = facts_seq;
    // Value-guard against the shell-owned facts SoT (B20 — the getterless VM can no
    // longer answer). On a real change, fold the new values into `state.device_facts`
    // so the caller's `set_media_environment` push reads them; the actual VM push +
    // MQL re-eval happen in the arms, not here.
    if dppx != state.device_facts.dppx || color_scheme != state.device_facts.color_scheme {
        state.device_facts.dppx = dppx;
        state.device_facts.color_scheme = color_scheme;
        true
    } else {
        false
    }
}

/// Handle a single message. Returns `false` for Shutdown.
///
/// Also exposed as `handle_message_public` for re-dispatch from navigation.rs.
#[allow(clippy::too_many_lines)]
pub(super) fn handle_message_public(msg: BrowserToContent, state: &mut ContentState) -> bool {
    handle_message(msg, state)
}

#[allow(clippy::too_many_lines)]
fn handle_message(msg: BrowserToContent, state: &mut ContentState) -> bool {
    match msg {
        BrowserToContent::Shutdown => {
            let proceed = crate::pipeline::dispatch_unload_events(
                &mut state.pipeline.runtime,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            if !proceed {
                return true;
            }
            state.iframes.shutdown_all();
            // Document teardown (WHATWG HTML §10.2.4 / WebSockets §7): force-close
            // WS/SSE + terminate dedicated workers AFTER unload handlers ran,
            // before the content thread exits. Unifies the former separate boa
            // `shutdown_all_realtime` + `shutdown_all_workers` calls.
            state.pipeline.teardown_document();
            return false;
        }

        BrowserToContent::Navigate(url) => {
            // Only a CROSS-document address-bar nav dispatches unload/beforeunload.
            // A same-page `#fragment` address-bar nav is same-document (*navigate to
            // a fragment*, `isSameDocument = true`): it fires NO unload and takes the
            // Fragment branch's no-rebuild path in `handle_navigate` (§6.3
            // caller-audit — this address-bar `Push` caller is the one site with an
            // unconditional pre-unload). A missing current URL classifies as
            // cross-document (a full nav) — fire unload. Uses the same classifier +
            // current-URL source (`state.pipeline.url`) as the Fragment branch, so
            // the unload-skip and the no-rebuild decision agree.
            let is_cross_document = state.pipeline.url.as_ref().is_none_or(|current| {
                elidex_navigation::classify_navigation(current, &url)
                    == elidex_navigation::NavClass::CrossDocument
            });
            if is_cross_document {
                let proceed = crate::pipeline::dispatch_unload_events(
                    &mut state.pipeline.runtime,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if !proceed {
                    return true;
                }
            }
            navigation::handle_navigate(state, &url, navigation::HistoryCursorOp::Push, None);
        }

        BrowserToContent::MouseClick(ref click) => {
            if input_placement_stale(click.placement_seq, state) {
                return true;
            }
            event_handlers::handle_click(state, click);
        }

        BrowserToContent::MouseRelease { button: _ } => {
            event_handlers::handle_mouse_release(state);
        }

        BrowserToContent::MouseMove {
            point,
            placement_seq,
            ..
        } => {
            if input_placement_stale(placement_seq, state) {
                return true;
            }
            event_handlers::handle_mouse_move(state, point);
        }

        BrowserToContent::CursorLeft => {
            event_handlers::handle_cursor_left(state);
        }

        BrowserToContent::KeyDown {
            ref key,
            ref code,
            repeat,
            mods,
        } => {
            event_handlers::handle_key(state, "keydown", key, code, repeat, mods);
        }

        BrowserToContent::KeyUp {
            ref key,
            ref code,
            repeat,
            mods,
        } => {
            event_handlers::handle_key(state, "keyup", key, code, repeat, mods);
        }

        BrowserToContent::SetViewport {
            width,
            height,
            seq,
            color_scheme,
            dppx,
            facts_seq,
        } => {
            // Reconcile this FIFO delivery with the build's cell-read (the `ViewportCell`
            // high-water mark), then with CSSOM View. Size and device facts are applied
            // from this **one** message, each by its own staleness guard, then a **single**
            // re-eval reflects both — so a DPI move that alters the logical size *and* the
            // facts never produces an inconsistent new-size + old-facts intermediate MQL
            // fire (C3 R2; the producer puts the facts here, not in a separate
            // `SetDeviceFacts`, precisely when both change in a frame).
            //
            // (i) Size staleness: drop any size whose `seq` is `≤` the seq the current
            // document built at (or last applied) — an intermediate resize already folded
            // into the size the build read; re-applying would flash backward. **Unlike
            // facts, this returns** (the whole delivery's size half is moot), but the
            // facts half rides its own `facts_seq` and the producer only co-delivers facts
            // with a *fresh* size, so a stale-seq `SetViewport` carries no fresh facts.
            if seq <= state.applied_viewport_seq {
                return true;
            }
            // (ii) Advance the size high-water mark on a *fresh* seq **unconditionally** —
            // even when the size is unchanged below — so a later equal-seq delivery is
            // correctly judged stale. Seq bookkeeping and resize-event firing (CSSOM View
            // §13.1) are orthogonal; collapsing them would leave the mark behind on a
            // seq-newer/value-same delivery.
            state.applied_viewport_seq = seq;

            // (iii) Apply the carried device facts (the `facts_seq` staleness + value
            // guards live in the shared helper). Returns whether they actually changed —
            // the single re-eval below must run if **either** the size or the facts did.
            let facts_changed = apply_device_facts(state, color_scheme, dppx, facts_seq);

            // (iv) Size value-guard — per CSSOM View §13.1 "run the resize steps"
            // (#document-run-the-resize-steps) step 1, a `resize` event fires only when the
            // width/height changed **since the last run**. The producer gates
            // `broadcast_viewport` on a real size change, but this guard remains
            // load-bearing for the build-vs-broadcast staleness race: a just-spawned tab
            // born at exactly the broadcast size (its build read the same cell value) still
            // receives the fan-out and must not fire a spurious `resize` / double-paint.
            let size_changed = width > 0.0
                && width.is_finite()
                && height > 0.0
                && height.is_finite()
                && (width != state.pipeline.viewport.width
                    || height != state.pipeline.viewport.height);

            // Nothing to apply (a same-size, same-facts fan-out) → no re-eval, no repaint.
            if !size_changed && !facts_changed {
                return true;
            }

            if size_changed {
                state.pipeline.viewport = elidex_plugin::Size::new(width, height);
            }

            // Push the new size + the facts `apply_device_facts` already folded into
            // `state.device_facts` to the engine (B20 model inversion — the shell owns
            // the facts SoT, the VM evaluates). Setter form: survives unbind, called
            // OUTSIDE any bracket. `matchMedia().matches` DERIVES live from this stored
            // environment on every get, so right after this push a `resize` listener
            // reads the post-change `matches` with no cache to refresh (C3 R2 atomicity
            // is preserved by pushing size + facts in one call before any event fires).
            state.pipeline.runtime.set_media_environment(
                f64::from(state.pipeline.viewport.width),
                f64::from(state.pipeline.viewport.height),
                state.device_facts.dppx,
                state.device_facts.color_scheme,
                state.device_facts.reduced_motion,
            );

            // HTML "update the rendering" (§8.1.7.3 Processing model): step 8 "run the
            // resize steps" runs **before** step 10 "evaluate media queries and report
            // changes". Fire `resize` first (iff the size changed) — its listener reads
            // the current (already-pushed) `matchMedia` — then report the MQL `change`
            // flips (CSSOM View §4.2) in one batch bracket.
            if size_changed {
                // `resize` fires on Window (CSSOM-View §13.1) — target the VM's
                // dedicated Window entity, NOT the Document: `window.addEventListener
                // ('resize', …)` records its listener against the Window entity
                // (window.rs), so a document-targeted dispatch misses it. Falls back
                // to the document entity pre-bind (window_entity == None).
                let window_target = state
                    .pipeline
                    .runtime
                    .window_entity()
                    .unwrap_or(state.pipeline.document);
                let mut resize_event =
                    elidex_script_session::DispatchEvent::new_composed("resize", window_target);
                resize_event.bubbles = false;
                resize_event.cancelable = false;
                state.pipeline.dispatch_event(&mut resize_event);
            }

            state.pipeline.deliver_media_query_changes();

            state.re_render();
            state.send_display_list();
        }

        BrowserToContent::SetDeviceFacts {
            color_scheme,
            dppx,
            facts_seq,
        } => {
            // Per-window device facts for a **facts-only** frame (the producer sends
            // `SetViewport` instead, carrying the facts atomically, when the size also
            // changes — so this never co-occurs with a size change). Activates
            // `window.devicePixelRatio` + `prefers-color-scheme` and re-evaluates
            // `@media (resolution | prefers-color-scheme)`. The `facts_seq` staleness +
            // value guards live in the shared `apply_device_facts`; a real change needs
            // one re-eval + repaint (no `resize` — the size is unchanged).
            if apply_device_facts(state, color_scheme, dppx, facts_seq) {
                // Push the settled facts (viewport unchanged — carry the current size)
                // to the engine, then report MQL `change` flips (CSSOM View §4.2) in one
                // batch bracket. No `resize` — the size did not change. `matchMedia`
                // derives live from the pushed environment (B20).
                state.pipeline.runtime.set_media_environment(
                    f64::from(state.pipeline.viewport.width),
                    f64::from(state.pipeline.viewport.height),
                    state.device_facts.dppx,
                    state.device_facts.color_scheme,
                    state.device_facts.reduced_motion,
                );
                state.pipeline.deliver_media_query_changes();
                state.re_render();
                state.send_display_list();
            }
        }

        BrowserToContent::VisibilityChanged { visible } => {
            let mut event = elidex_script_session::DispatchEvent::new_composed(
                "visibilitychange",
                state.pipeline.document,
            );
            event.bubbles = false;
            event.cancelable = false;
            state.pipeline.runtime.set_visibility(visible);
            state.pipeline.dispatch_event(&mut event);
            state.re_render();
            state.send_display_list();
        }

        BrowserToContent::GoBack => {
            chrome_traverse(state, ChromeTraversal::Back);
        }

        BrowserToContent::GoForward => {
            chrome_traverse(state, ChromeTraversal::Forward);
        }

        BrowserToContent::Reload => {
            let proceed = crate::pipeline::dispatch_unload_events(
                &mut state.pipeline.runtime,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            if proceed {
                if let Some(url) = state.pipeline.url.clone() {
                    // Reload never moves the cursor → Keep.
                    navigation::handle_navigate(
                        state,
                        &url,
                        navigation::HistoryCursorOp::Keep,
                        None,
                    );
                }
            }
        }

        BrowserToContent::MouseWheel {
            delta,
            point,
            placement_seq,
        } => {
            if input_placement_stale(placement_seq, state) {
                return true;
            }
            scroll::handle_wheel(state, delta, point);
        }

        BrowserToContent::Ime { kind } => {
            ime::handle_ime(state, kind);
        }

        BrowserToContent::StorageEvent {
            key,
            old_value,
            new_value,
            url,
        } => {
            dispatch_storage_event(state, key, old_value, new_value, url);
        }

        BrowserToContent::IdbVersionChange {
            request_id,
            db_name,
            old_version,
            new_version,
        } => {
            state
                .pipeline
                .deliver_idb_versionchange(&db_name, old_version, new_version);
            let _ = state.channel.send(ContentToBrowser::IdbConnectionsClosed {
                request_id,
                db_name,
            });
        }

        // Settle SW client promises / fire client events via ONE self-bracketed
        // deliver (§6.2 — `handle_message` is NOT inside an open bracket).
        BrowserToContent::SwRegistered(data) => {
            state
                .pipeline
                .deliver_sw_client_update(SwClientUpdate::Registered {
                    scope: data.scope,
                    success: data.success,
                    error: data.error,
                    worker: data.worker,
                    update_via_cache: data.update_via_cache,
                });
        }
        BrowserToContent::SwUnregistered { scope, success } => {
            state
                .pipeline
                .deliver_sw_client_update(SwClientUpdate::Unregistered { scope, success });
        }
        BrowserToContent::SwStateChanged {
            scope,
            state: sw_state,
        } => {
            state
                .pipeline
                .deliver_sw_client_update(SwClientUpdate::StateChanged {
                    scope,
                    state: sw_state,
                });
        }
        BrowserToContent::SwControllerSet { scope } => {
            state
                .pipeline
                .deliver_sw_client_update(SwClientUpdate::ControllerSet { scope: Some(scope) });
        }

        BrowserToContent::IdbUpgradeReady { .. }
        | BrowserToContent::IdbBlocked { .. }
        | BrowserToContent::StorageEstimateResult { .. }
        | BrowserToContent::StoragePersistResult { .. }
        | BrowserToContent::StoragePersistedResult { .. }
        | BrowserToContent::ManifestParsed(_)
        | BrowserToContent::SwFetchResponse { .. } => {}
    }
    true
}

/// Direction of a chrome Back/Forward-button traversal.
#[derive(Clone, Copy)]
enum ChromeTraversal {
    Back,
    Forward,
}

/// Apply a chrome Back/Forward-button traversal through the SAME peek-then-commit
/// path as a JS `history.back()`/`forward()` (`navigation::handle_navigate` with
/// `Commit`), so a same-document toolbar traversal restores state + scroll and
/// fires popstate IN PLACE (no rebuild) exactly like the JS API — One-issue-one-way,
/// retiring the old eager `go_back`/`go_forward` + always-rebuild path (which also
/// non-atomically moved the cursor before the load). `beforeunload`/`unload` fire
/// ONLY for a cross-document traversal (`resolve_traversal` = `Rebuild`): a
/// same-document traversal does not destroy the document, so it must not run the
/// unload steps, and a cancelled `beforeunload` blocks only a document-destroying
/// traversal.
fn chrome_traverse(state: &mut ContentState, direction: ChromeTraversal) {
    let peeked = match direction {
        ChromeTraversal::Back => state.nav_controller.peek_back(),
        ChromeTraversal::Forward => state.nav_controller.peek_forward(),
    };
    let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
        return;
    };
    let cross_document = matches!(
        state.nav_controller.resolve_traversal(target_index),
        elidex_navigation::TraversalKind::Rebuild
    );
    if cross_document {
        let proceed = crate::pipeline::dispatch_unload_events(
            &mut state.pipeline.runtime,
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );
        if !proceed {
            return; // beforeunload cancelled the (document-destroying) traversal.
        }
    }
    navigation::handle_navigate(
        state,
        &url,
        navigation::HistoryCursorOp::Commit(target_index),
        None,
    );
}
