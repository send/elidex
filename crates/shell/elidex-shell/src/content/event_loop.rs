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
            state.pipeline.runtime.drain_timers(
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
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

        for url in state.pipeline.runtime.bridge().drain_pending_open_tabs() {
            let _ = state.channel.send(ContentToBrowser::OpenNewTab(url));
        }

        if state.pipeline.runtime.bridge().take_pending_focus() {
            let _ = state.channel.send(ContentToBrowser::FocusWindow);
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

/// Handle a single message. Returns `false` for Shutdown.
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
            navigation::handle_navigate(state, &url, false, None);
        }

        BrowserToContent::MouseClick(ref click) => {
            event_handlers::handle_click(state, click);
        }

        BrowserToContent::MouseRelease { button: _ } => {
            event_handlers::handle_mouse_release(state);
        }

        BrowserToContent::MouseMove { point, .. } => {
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

        BrowserToContent::SetViewport { width, height } => {
            if width > 0.0 && width.is_finite() && height > 0.0 && height.is_finite() {
                state.pipeline.viewport = elidex_plugin::Size::new(width, height);
                let bridge = state.pipeline.runtime.bridge().clone();
                bridge.set_viewport(width, height);

                let changed = bridge.re_evaluate_media_queries(width, height);
                if !changed.is_empty() {
                    dispatch_media_query_changes(&changed, state);
                }

                let mut resize_event = elidex_script_session::DispatchEvent::new_composed(
                    "resize",
                    state.pipeline.document,
                );
                resize_event.bubbles = false;
                resize_event.cancelable = false;
                state.pipeline.runtime.dispatch_event(
                    &mut resize_event,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );

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
            state.pipeline.runtime.dispatch_event(
                &mut event,
                &mut state.pipeline.session,
                &mut state.pipeline.dom,
                state.pipeline.document,
            );
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
                        navigation::handle_navigate(state, &url, true, None);
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
                        navigation::handle_navigate(state, &url, true, None);
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
                    navigation::handle_navigate(state, &url, true, None);
                }
            }
        }

        BrowserToContent::MouseWheel { delta, point } => {
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
        | BrowserToContent::ManifestParsed(_)
        | BrowserToContent::SwFetchResponse { .. } => {}
    }
    true
}
