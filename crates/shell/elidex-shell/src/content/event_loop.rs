//! Content thread event loop and message dispatch.
//!
//! Extracted from `content/mod.rs` to keep file sizes manageable.

use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;

use crate::ipc::{BrowserToContent, ContentToBrowser};

use super::{
    animation, apply_script_animations, dispatch_media_query_changes, dispatch_message_event,
    dispatch_storage_event, iframe, navigation, scroll, ContentState, DEFAULT_POLL_INTERVAL,
    FRAME_INTERVAL,
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

        apply_script_animations(state);

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
        let post_messages = state.iframes.drain_oop_messages();
        for msg in &post_messages {
            dispatch_message_event(state, &msg.data, &msg.origin);
        }

        let self_messages = state.pipeline.runtime.bridge().drain_post_messages();
        for (data, origin) in &self_messages {
            dispatch_message_event(state, data, origin);
        }
        if !self_messages.is_empty() || !post_messages.is_empty() {
            needs_render = true;
        }

        for change in state.pipeline.runtime.bridge().drain_storage_changes() {
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
            .bridge()
            .drain_idb_versionchange_requests()
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

        for req in state.pipeline.runtime.bridge().drain_sw_register_requests() {
            if let Some(ref current_url) = state.pipeline.runtime.bridge().current_url() {
                let origin = current_url.origin().unicode_serialization();
                let Ok(script_url) = current_url.join(&req.script_url) else {
                    continue;
                };
                let scope = req
                    .scope
                    .as_deref()
                    .and_then(|s| current_url.join(s).ok())
                    .unwrap_or_else(|| elidex_api_sw::default_scope(&script_url));
                let _ = state.channel.send(ContentToBrowser::SwRegister {
                    script_url,
                    scope,
                    origin,
                    page_url: current_url.clone(),
                });
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

        if state.pipeline.runtime.bridge().take_pending_focus() {
            state.notify_browser(ContentToBrowser::FocusWindow);
        }

        {
            let (ws_events, sse_events) = state.pipeline.runtime.bridge().drain_realtime_events();
            let has_js_events = ws_events
                .iter()
                .any(|(_, e)| !matches!(e, elidex_net::ws::WsEvent::BytesSent(_)))
                || !sse_events.is_empty();
            if !ws_events.is_empty() || !sse_events.is_empty() {
                state.pipeline.runtime.dispatch_realtime_events(
                    ws_events,
                    sse_events,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if has_js_events {
                    needs_render = true;
                }
            }
        }

        needs_render |= state.pipeline.runtime.drain_and_dispatch_worker_events(
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );

        iframe::tick_iframe_timers(state);

        if state.update_caret_blink() {
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
    let bridge = state.pipeline.runtime.bridge();
    if dppx != bridge.device_pixel_ratio() || color_scheme != bridge.color_scheme() {
        bridge.set_device_pixel_ratio(dppx);
        bridge.set_color_scheme(color_scheme);
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
            state.pipeline.runtime.bridge().shutdown_all_realtime();
            state.pipeline.runtime.bridge().shutdown_all_workers();
            return false;
        }

        BrowserToContent::Navigate(url) => {
            let proceed = crate::pipeline::dispatch_unload_events(
                &mut state.pipeline.runtime,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            if !proceed {
                return true;
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

            let bridge = state.pipeline.runtime.bridge().clone();
            if size_changed {
                state.pipeline.viewport = elidex_plugin::Size::new(width, height);
                bridge.set_viewport(width, height);
            }

            // Refresh each `MediaQueryList`'s cached `matches` to the new viewport **and**
            // the facts applied in (iii) **before** dispatching any event (CSSOM View §4.2
            // — the `matches` getter must read the post-change value in a `change`
            // listener). One re-eval covers both inputs (atomic — C3 R2); it only refreshes
            // state + collects the changed set, the `change` *events* fire after `resize`.
            let changed = bridge.re_evaluate_media_queries(
                state.pipeline.viewport.width,
                state.pipeline.viewport.height,
            );

            // HTML "update the rendering" (§8.1.7.3 Processing model): step 8 "run the
            // resize steps" runs **before** step 10 "evaluate media queries and report
            // changes". Fire `resize` first (iff the size changed), then the MQL `change`
            // events — spec-correct order, with the cache a `resize` listener reads already
            // current (refreshed above).
            if size_changed {
                let mut resize_event = elidex_script_session::DispatchEvent::new_composed(
                    "resize",
                    state.pipeline.document,
                );
                resize_event.bubbles = false;
                resize_event.cancelable = false;
                state.pipeline.dispatch_event(&mut resize_event);
            }

            if !changed.is_empty() {
                dispatch_media_query_changes(&changed, state);
            }

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
                let bridge = state.pipeline.runtime.bridge().clone();
                // Refresh each `MediaQueryList`'s cached `matches` to the new facts BEFORE
                // firing any event (CSSOM View §4.2 — the `matches` getter must read the
                // post-change value in a `change` listener); the viewport is unchanged, so
                // pass the current cached size. Then fire the MQL `change` events + repaint.
                let changed_mqls = bridge.re_evaluate_media_queries(
                    state.pipeline.viewport.width,
                    state.pipeline.viewport.height,
                );
                if !changed_mqls.is_empty() {
                    dispatch_media_query_changes(&changed_mqls, state);
                }
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
            state.pipeline.runtime.bridge().set_visibility(visible);
            state.pipeline.dispatch_event(&mut event);
            state.re_render();
            state.send_display_list();
        }

        BrowserToContent::GoBack => {
            if state.nav_controller.can_go_back() {
                let proceed = crate::pipeline::dispatch_unload_events(
                    &mut state.pipeline.runtime,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if proceed {
                    if let Some(url) = state.nav_controller.go_back().cloned() {
                        // `go_back` already eager-moved the cursor → Keep (do NOT
                        // double-commit); `notify_navigation` reads the moved cursor.
                        navigation::handle_navigate(
                            state,
                            &url,
                            navigation::HistoryCursorOp::Keep,
                            None,
                        );
                    }
                }
            }
        }

        BrowserToContent::GoForward => {
            if state.nav_controller.can_go_forward() {
                let proceed = crate::pipeline::dispatch_unload_events(
                    &mut state.pipeline.runtime,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );
                if proceed {
                    if let Some(url) = state.nav_controller.go_forward().cloned() {
                        // `go_forward` already eager-moved the cursor → Keep.
                        navigation::handle_navigate(
                            state,
                            &url,
                            navigation::HistoryCursorOp::Keep,
                            None,
                        );
                    }
                }
            }
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
            state.pipeline.runtime.dispatch_idb_versionchange(
                &db_name,
                old_version,
                new_version,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
            let _ = state.channel.send(ContentToBrowser::IdbConnectionsClosed {
                request_id,
                db_name,
            });
        }

        BrowserToContent::IdbUpgradeReady { .. }
        | BrowserToContent::IdbBlocked { .. }
        | BrowserToContent::StorageEstimateResult { .. }
        | BrowserToContent::StoragePersistResult { .. }
        | BrowserToContent::StoragePersistedResult { .. }
        | BrowserToContent::SwRegistered(_)
        | BrowserToContent::SwControllerSet { .. }
        | BrowserToContent::SwStateChanged { .. }
        | BrowserToContent::ManifestParsed(_)
        | BrowserToContent::SwFetchResponse { .. } => {}
    }
    true
}
