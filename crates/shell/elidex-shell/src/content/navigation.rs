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
            // Rebuild at the tab's CURRENT viewport + device facts (not `DEFAULT`) so
            // the new document's initial scripts + layout see the real
            // `innerWidth`/`@media`/`devicePixelRatio` (C1/C3; the new runtime's JS
            // bridge is seeded from this snapshot inside the builder — the fresh
            // document's bridge would otherwise default to 1×/Light). Read the
            // **latest browser-published** snapshot from the viewport cell *after* the
            // blocking `load_document` above returns: a resize/scale change that landed
            // during the load is observed by construction, where the old
            // `state.pipeline.viewport` snapshot would be stale. `seq` re-bases this
            // document's high-water mark below.
            let snapshot = state.viewport_cell.read();
            let (viewport, seq, facts_seq) = (snapshot.size, snapshot.seq, snapshot.facts_seq);
            let new_pipeline = crate::build_pipeline_from_loaded(
                loaded,
                network_handle,
                font_db,
                cookie_jar,
                viewport,
                snapshot.facts,
                // Top-level document: no frame security (URL-derived origin).
                None,
            );
            state.pipeline = new_pipeline;
            // Focus lives in the new pipeline's `EcsDom` (empty by construction
            // — a fresh document); no field to reset, no blur to dispatch.
            state.hover_chain.clear();
            state.active_chain.clear();
            state.focusable_cache = None;
            state.viewport_scroll = elidex_ecs::ScrollState::default();
            // Re-base the viewport + facts high-water marks to the rebuild's cell-read
            // generations, in the per-pipeline reset cluster: every rebuild re-anchors
            // them so a queued `SetViewport` / `SetDeviceFacts` is judged against THIS
            // document's build, not the prior document's (else a post-nav resize / DPI
            // change is mis-dropped as stale, or a stale pre-nav delivery mis-applies).
            // Unconditional — the new document consumed exactly `seq` / `facts_seq`.
            state.applied_viewport_seq = seq;
            state.applied_facts_seq = facts_seq;
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

    // window.open(_blank) → send OpenNewTab to browser thread. Drained via the
    // engine-agnostic session trait surface (`take_pending_open_tabs`), not the
    // boa bridge — the S5-6 flip swaps the runtime type without touching this
    // site (memo §4.3.2 / edge E4). The enqueue is popup-gated at the native, so
    // a sandbox-blocked popup never reaches this drain.
    let open_tabs = state.pipeline.runtime.take_pending_open_tabs();
    if !open_tabs.is_empty() {
        state.send_display_list();
        for req in open_tabs {
            if let Ok(url) = url::Url::parse(&req.url) {
                state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(url));
            }
        }
        return true;
    }

    // window.focus() is handled in content/mod.rs drain loop — do not duplicate here.

    // window.open with named target → route to matching iframe or (gated) new tab.
    let navigate_iframes = state.pipeline.runtime.take_pending_frame_navigations();
    if !navigate_iframes.is_empty() {
        route_frame_navigations(state, navigate_iframes);
        state.re_render();
        state.send_display_list();
        return true;
    }

    false
}

/// Route the drained named-target `window.open` navigations (WHATWG HTML
/// §7.3.1.7) against the current document's iframe tree.
///
/// - **HIT** (`find_iframe_by_name` matches a descendant iframe) →
///   `navigate_iframe`, **ungated**. This is spec-correct: `find_iframe_by_name`
///   (`content/iframe/lifecycle.rs`) searches only the current document's
///   iframes, so the source is an ancestor of the target ⇒ HTML §7.4.2.4 step 2
///   ("If source is an ancestor of target, then return true") discharges the
///   *allowed by sandboxing to navigate* check unconditionally. Revisit if the
///   lookup ever widens beyond descendants (folded into slot
///   `#11-browsing-context-model-window-open-postmessage`).
/// - **MISS** → promote to a new tab **only when** the payload's call-time
///   `aux_nav_allowed` snapshot permits (§7.3.1.7 step 3 snapshots the
///   sandboxing flag set at call time — never re-read live flags here). A
///   sandboxed no-`allow-popups` document's miss is dropped silently (HTML
///   §7.3.1.7 step 8 sandboxed-auxiliary-navigation case — "may report to a
///   developer console"). The previously-ungated promotion was the sandbox
///   bypass this slice closes.
pub(crate) fn route_frame_navigations(
    state: &mut ContentState,
    navigations: Vec<elidex_script_session::NamedFrameNavigation>,
) {
    for nav in navigations {
        if let Some(iframe_entity) = super::iframe::find_iframe_by_name(state, &nav.name) {
            if let Ok(url) = url::Url::parse(&nav.url) {
                super::iframe::navigate_iframe(state, iframe_entity, &url);
            }
        } else if nav.aux_nav_allowed {
            if let Ok(url) = url::Url::parse(&nav.url) {
                state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(url));
            }
        }
        // else: MISS without an aux-nav grant → drop (sandboxed auxiliary
        // navigation, §7.3.1.7 step 8).
    }
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
