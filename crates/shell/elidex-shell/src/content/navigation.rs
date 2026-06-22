//! Navigation and history action handling for the content thread.

use std::rc::Rc;
use std::sync::Arc;

use crate::app::navigation::resolve_nav_url;
use crate::ipc::ContentToBrowser;

use super::ContentState;

/// Navigate to a URL, loading the document and updating state.
///
/// When `is_history_nav` is `true`, the URL is not pushed to the navigation
/// controller (it was already moved by `go_back`/`go_forward`).
///
/// When `request` is `Some`, that request is sent instead of a default GET
/// (used for POST form submissions).
#[allow(clippy::too_many_lines)]
pub(super) fn handle_navigate(
    state: &mut ContentState,
    url: &url::Url,
    is_history_nav: bool,
    request: Option<elidex_net::Request>,
) {
    // WHATWG SW Handle Fetch — skip SW interception in these cases:
    // 1. Fragment-only navigation (same-document, no network fetch).
    let is_fragment_only =
        state
            .pipeline
            .runtime
            .bridge()
            .current_url()
            .is_some_and(|ref current| {
                current.as_str().split('#').next() == url.as_str().split('#').next()
                    && url.fragment().is_some()
            });

    // 2. embed/object destination — always skip (SW spec Handle Fetch §1).
    // 3. Shift+reload — skip (not yet tracked in this code path).
    // These are handled by the browser thread for subresource requests.

    if !is_fragment_only {
        if let Some(sw_scope) = state.pipeline.runtime.bridge().sw_controller_scope() {
            if elidex_api_sw::matches_scope(&sw_scope, url) {
                // Send FetchEvent relay request to browser thread.
                static FETCH_ID: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(1);
                let fetch_id = FETCH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                let (method, headers, body) = match &request {
                    Some(req) => (req.method.clone(), req.headers.clone(), req.body.to_vec()),
                    None => ("GET".into(), vec![], vec![]),
                };
                let sw_request = elidex_api_sw::SwRequest {
                    url: url.clone(),
                    method,
                    headers,
                    body,
                    mode: "navigate".into(),
                    destination: "document".into(),
                    integrity: None,
                    redirect: "follow".into(),
                    referrer: "about:client".into(),
                    referrer_policy: String::new(),
                    cache_mode: "default".into(),
                    keepalive: false,
                };

                let client_id = state.pipeline.runtime.bridge().client_id();
                let _ = state
                    .channel
                    .send(crate::ipc::ContentToBrowser::SwFetchRequest {
                        fetch_id,
                        request: Box::new(sw_request),
                        client_id,
                        // The resulting document's client ID (for FetchEvent.resultingClientId).
                        resulting_client_id: uuid::Uuid::new_v4().to_string(),
                    });

                // Wait for SW response. This blocks the content thread; fully async
                // navigation interception requires M4-10 (elidex-js VM event loop).
                // Loop to avoid consuming unrelated messages.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
                loop {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        break; // Timeout — fall through to normal fetch.
                    }
                    match state.channel.recv_timeout(remaining) {
                        Ok(crate::ipc::BrowserToContent::SwFetchResponse {
                            fetch_id: resp_id,
                            response: Some(resp),
                        }) if resp_id == fetch_id => {
                            tracing::debug!(
                                url = %url,
                                status = resp.status,
                                "SW intercepted navigation"
                            );
                            // TODO: construct document from SW response body.
                            break;
                        }
                        Ok(crate::ipc::BrowserToContent::SwFetchResponse {
                            fetch_id: resp_id,
                            response: None,
                        }) if resp_id == fetch_id => {
                            break; // Passthrough.
                        }
                        Ok(other) => {
                            // Re-dispatch non-matching message (including
                            // SwFetchResponse with wrong fetch_id).
                            super::event_loop::handle_message_public(other, state);
                        }
                        Err(_) => break, // Timeout or disconnected.
                    }
                }
            }
        }
    }

    let network_handle = Rc::clone(&state.pipeline.network_handle);
    let font_db = Arc::clone(&state.pipeline.font_db);

    match elidex_navigation::load_document(url, &network_handle, request) {
        Ok(loaded) => {
            // Shut down WebSocket/SSE connections before replacing the pipeline.
            state.pipeline.runtime.bridge().shutdown_all_realtime();
            // Preserve cookie jar across navigations.
            let cookie_jar = state.pipeline.runtime.bridge().cookie_jar_clone();
            // Rebuild at the tab's CURRENT viewport (not `DEFAULT`) so the new
            // document's initial scripts + layout see the real `innerWidth`/`@media`
            // (C1; the new runtime's JS bridge is seeded from this viewport inside
            // the builder). A window resize delivered *while `load_document` was
            // blocking* sits queued as a `SetViewport`, so the old pipeline's last
            // processed size can be stale — drain the channel and fold the latest
            // queued viewport in. Non-viewport messages are buffered and replayed
            // onto the new document after commit (below), exactly where the normal
            // event loop would have delivered them, so this corrects only the build
            // size and preserves message ordering (e.g. a queued `Navigate`).
            let (viewport, queued_during_load) =
                drain_viewport_queued_during_load(&state.channel, state.pipeline.viewport);
            let new_pipeline = crate::build_pipeline_from_loaded(
                loaded,
                network_handle,
                font_db,
                cookie_jar,
                viewport,
            );
            state.pipeline = new_pipeline;
            // Focus lives in the new pipeline's `EcsDom` (empty by construction
            // — a fresh document); no field to reset, no blur to dispatch.
            state.hover_chain.clear();
            state.active_chain.clear();
            state.focusable_cache = None;
            state.viewport_scroll = elidex_ecs::ScrollState::default();
            super::scroll::update_viewport_scroll_dimensions(state);

            if !is_history_nav {
                state.nav_controller.push(url.clone());
            }
            state.pipeline.runtime.set_current_url(Some(url.clone()));
            state
                .pipeline
                .runtime
                .set_history_length(state.nav_controller.len());
            state.notify_navigation(url);
            // Replay every message that arrived during the blocking load onto the
            // now-committed new document, in arrival order — the same destination
            // and order the normal event loop would have processed them on. The
            // `SetViewport`s replay too (the final one a no-op since the document
            // is born at that size) so a queued input still hit-tests against the
            // intermediate layout it was mapped for. A queued `Shutdown` flags the
            // exit for `run_event_loop` (its unload already ran inside the replay).
            for msg in queued_during_load {
                if !super::event_loop::handle_message_public(msg, state) {
                    state.pending_shutdown = true;
                    break;
                }
            }
        }
        Err(e) => {
            eprintln!("Content thread navigation error: {e}");
            let _ = state.channel.send(ContentToBrowser::NavigationFailed {
                url: url.clone(),
                error: format!("{e}"),
            });
        }
    }
}

/// Drain messages that queued while a blocking `load_document` ran, returning
/// (a) the latest valid `SetViewport` size folded onto `current` — for *building*
/// the replacement document at the real, possibly-just-resized viewport (C1 "real
/// viewport before scripts") — and (b) the **full** message buffer in arrival
/// order, for the caller to replay onto the committed document.
///
/// The buffer keeps **every** message, `SetViewport` included: the drain only
/// *peeks* the latest viewport for the build; message processing must otherwise
/// stay identical to the normal event loop. Replaying the `SetViewport`s in order
/// is what keeps a queued input event hit-testing against the layout it was
/// mapped for — a `[resize→A, click, resize→B]` sequence must apply `A`, process
/// the click against `A`, then apply `B`, not collapse to `B` (the click was
/// mapped by the browser using placement `A`). The final `SetViewport` replays as
/// an idempotent no-op since the document is already born at that size (CSSOM
/// View §13.1), and a degenerate one is rejected by the consumer.
///
/// Shared by the two top-level content-thread paths that block on
/// `load_document` then build at a captured viewport: the navigation rebuild
/// (here) and the initial URL-backed spawn (`content_thread_main_url`).
pub(super) fn drain_viewport_queued_during_load(
    channel: &crate::ipc::LocalChannel<ContentToBrowser, crate::ipc::BrowserToContent>,
    current: elidex_plugin::Size,
) -> (elidex_plugin::Size, Vec<crate::ipc::BrowserToContent>) {
    let mut viewport = current;
    let mut buffered = Vec::new();
    while let Ok(msg) = channel.try_recv() {
        if let crate::ipc::BrowserToContent::SetViewport { width, height } = &msg {
            if *width > 0.0 && width.is_finite() && *height > 0.0 && height.is_finite() {
                viewport = elidex_plugin::Size::new(*width, *height);
            }
        }
        buffered.push(msg);
    }
    (viewport, buffered)
}

/// Process any pending JS navigation or history action after event dispatch.
///
/// Returns `true` if an action was processed (display list already sent).
pub(super) fn process_pending_actions(state: &mut ContentState) -> bool {
    if let Some(nav_req) = state.pipeline.runtime.take_pending_navigation() {
        let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &nav_req.url);
        if let Some(target_url) = resolved {
            state.send_display_list();
            handle_navigate(state, &target_url, false, None);
            return true;
        }
    }

    if let Some(action) = state.pipeline.runtime.take_pending_history() {
        handle_history_action(state, &action);
        state.send_display_list();
        return true;
    }

    // window.open(_blank) → send OpenNewTab to browser thread.
    let open_tabs = state.pipeline.runtime.bridge().drain_pending_open_tabs();
    if !open_tabs.is_empty() {
        state.send_display_list();
        for url in open_tabs {
            state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(url));
        }
        return true;
    }

    // window.focus() is handled in content/mod.rs drain loop — do not duplicate here.

    // window.open with named target → navigate matching iframe or open new tab.
    let navigate_iframes = state
        .pipeline
        .runtime
        .bridge()
        .drain_pending_navigate_iframe();
    if !navigate_iframes.is_empty() {
        for (name, url) in navigate_iframes {
            if let Some(iframe_entity) = super::iframe::find_iframe_by_name(state, &name) {
                super::iframe::navigate_iframe(state, iframe_entity, &url);
            } else {
                // No matching iframe → open in new tab.
                state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(url));
            }
        }
        state.re_render();
        state.send_display_list();
        return true;
    }

    false
}

pub(super) fn handle_history_action(
    state: &mut ContentState,
    action: &elidex_script_session::HistoryAction,
) {
    match action {
        elidex_script_session::HistoryAction::Back
        | elidex_script_session::HistoryAction::Forward => {
            let url = if matches!(action, elidex_script_session::HistoryAction::Back) {
                state.nav_controller.go_back().cloned()
            } else {
                state.nav_controller.go_forward().cloned()
            };
            if let Some(url) = url {
                handle_navigate(state, &url, true, None);
            }
        }
        elidex_script_session::HistoryAction::Go(delta) => {
            if let Some(url) = state.nav_controller.go(*delta).cloned() {
                handle_navigate(state, &url, true, None);
            }
        }
        elidex_script_session::HistoryAction::PushState { url, .. }
        | elidex_script_session::HistoryAction::ReplaceState { url, .. } => {
            let replace = matches!(
                action,
                elidex_script_session::HistoryAction::ReplaceState { .. }
            );
            apply_push_replace_state(state, url.as_deref(), replace);
        }
    }
}

/// Apply a `pushState`/`replaceState` history action.
///
/// Resolves the URL (if any), enforces same-origin, updates the pipeline URL,
/// navigation controller, and notifies the browser thread.
fn apply_push_replace_state(state: &mut ContentState, url_str: Option<&str>, replace: bool) {
    if let Some(url_str) = url_str {
        let Some(resolved_url) = resolve_nav_url(state.pipeline.url.as_ref(), url_str) else {
            return;
        };
        // Same-origin check (scheme + host + port).
        if let Some(current) = &state.pipeline.url {
            let current_origin: url::Origin = current.origin();
            let resolved_origin: url::Origin = resolved_url.origin();
            if current_origin != resolved_origin {
                eprintln!(
                    "SecurityError: pushState/replaceState URL {resolved_url} has different origin than {current}"
                );
                return;
            }
        }
        state.pipeline.url = Some(resolved_url.clone());
        state.push_or_replace(resolved_url.clone(), replace);
        state
            .pipeline
            .runtime
            .set_current_url(state.pipeline.url.clone());
        state
            .pipeline
            .runtime
            .set_history_length(state.nav_controller.len());

        let title = format!("elidex \u{2014} {resolved_url}");
        state.send_title(title);
        state.send_url_changed(&resolved_url);
        state.send_navigation_state();
    } else {
        // No URL change — just update history.
        let Some(current) = state.pipeline.url.clone() else {
            return;
        };
        state.push_or_replace(current, replace);
        state
            .pipeline
            .runtime
            .set_history_length(state.nav_controller.len());
        state.send_navigation_state();
    }
}

#[cfg(test)]
mod tests {
    use super::drain_viewport_queued_during_load;
    use crate::ipc::{channel_pair, BrowserToContent, ContentToBrowser};
    use elidex_plugin::Size;

    /// F5/F10: a resize that lands while `load_document` blocks (queued as a
    /// `SetViewport`) is folded into the build viewport — the latest wins — while
    /// the **full** sequence is buffered in arrival order (SetViewports included)
    /// so a replayed input still hit-tests against its intermediate layout.
    #[test]
    fn drain_folds_latest_viewport_and_buffers_everything_in_order() {
        let (browser, content) = channel_pair::<BrowserToContent, ContentToBrowser>();
        // Interleave two resizes (640 is latest) with two non-viewport messages.
        browser
            .send(BrowserToContent::SetViewport {
                width: 800.0,
                height: 600.0,
            })
            .unwrap();
        browser
            .send(BrowserToContent::MouseRelease { button: 0 })
            .unwrap();
        browser
            .send(BrowserToContent::SetViewport {
                width: 640.0,
                height: 480.0,
            })
            .unwrap();
        browser.send(BrowserToContent::CursorLeft).unwrap();

        let (viewport, buffered) =
            drain_viewport_queued_during_load(&content, Size::new(1024.0, 768.0));

        assert_eq!(
            viewport,
            Size::new(640.0, 480.0),
            "the latest queued SetViewport must win the build viewport"
        );
        assert_eq!(
            buffered.len(),
            4,
            "every message is buffered in arrival order (SetViewports included)"
        );
        assert!(matches!(buffered[0], BrowserToContent::SetViewport { .. }));
        assert!(matches!(buffered[1], BrowserToContent::MouseRelease { .. }));
        assert!(matches!(buffered[2], BrowserToContent::SetViewport { .. }));
        assert!(matches!(buffered[3], BrowserToContent::CursorLeft));
    }

    /// No queued messages → the current (pre-navigation) viewport is kept and
    /// nothing is buffered (the common no-resize-during-load case).
    #[test]
    fn drain_empty_queue_keeps_current_viewport() {
        let (_browser, content) = channel_pair::<BrowserToContent, ContentToBrowser>();
        let (viewport, buffered) =
            drain_viewport_queued_during_load(&content, Size::new(1024.0, 768.0));
        assert_eq!(viewport, Size::new(1024.0, 768.0));
        assert!(buffered.is_empty());
    }

    /// A degenerate (zero / non-finite) `SetViewport` does not change the build
    /// viewport, but is still buffered for replay (the consumer rejects it on
    /// replay) so ordering with any interleaved input stays faithful.
    #[test]
    fn drain_ignores_degenerate_setviewport_for_build_but_buffers_it() {
        let (browser, content) = channel_pair::<BrowserToContent, ContentToBrowser>();
        browser
            .send(BrowserToContent::SetViewport {
                width: 0.0,
                height: 600.0,
            })
            .unwrap();
        browser
            .send(BrowserToContent::SetViewport {
                width: f32::NAN,
                height: 600.0,
            })
            .unwrap();

        let (viewport, buffered) =
            drain_viewport_queued_during_load(&content, Size::new(1024.0, 768.0));
        assert_eq!(
            viewport,
            Size::new(1024.0, 768.0),
            "degenerate SetViewport must not change the build viewport"
        );
        assert_eq!(
            buffered.len(),
            2,
            "degenerate SetViewport is still buffered (rejected by the consumer on replay)"
        );
    }
}
