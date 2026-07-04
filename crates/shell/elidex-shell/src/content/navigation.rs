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

/// Process any pending JS navigation / window.open / history action after
/// event dispatch. Returns `true` iff an **own-context** navigation occurred —
/// a `location.*` / `window.open('_self'/'_top')` navigation or a history
/// action that determines where THIS document goes. A caller uses that to
/// suppress its own-context fallback (a link's `<a href>` navigation), because
/// the JS already decided where the current context navigates.
///
/// `window.open` tab-creation / named-frame opens do NOT count: they act on
/// OTHER browsing contexts (a new tab, a child iframe) and are orthogonal to
/// the current context's navigation, so they are applied here but must NOT
/// suppress a link's default (a browser opens the popup AND navigates the
/// link). They still send their own display list when they have a real effect.
pub(super) fn process_pending_actions(state: &mut ContentState) -> bool {
    // window.open tab-creation / named-frame opens FIRST. These are effects on
    // OTHER browsing contexts that do NOT replace our pipeline, so they must be
    // drained + applied BEFORE any own-context history/navigation below —
    // otherwise a same-turn `window.open('_self')` / `location` navigation (or a
    // `history.back()` traversal) replaces the pipeline and strands the queued
    // opens (they live on the old pipeline's runtime and are lost). The channels
    // are distinct effect classes: own-context navigation is last-wins
    // (`pending_navigation`, shared with `location.*`), own-context history is
    // FIFO (`pending_history`), other-context open is FIFO-all-surface
    // (`pending_window_open`). Drained via the engine-agnostic session trait
    // surface (`take_pending_window_opens`), not the boa bridge — the S5-6 flip
    // swaps the runtime type without touching this site (memo §4.3.2 / edge E4).
    let window_opens = state.pipeline.runtime.take_pending_window_opens();
    if !window_opens.is_empty() {
        let outcome = route_window_opens(state, window_opens);
        if outcome.any_effect {
            // A real browser effect happened (a tab opened / an iframe
            // navigated) — re-render if OUR render changed, then flush once.
            // This does NOT make us report an own-context action.
            if outcome.navigated_iframe {
                state.re_render();
            }
            state.send_display_list();
        }
    }

    // Own-context HISTORY drain BEFORE the navigation drain below (WHATWG HTML
    // §7.4.4 — the URL/history update ran synchronously during the script). A
    // same-turn `pushState('/a'); location.href='/b'` enqueues both a history
    // mutation and a navigation; the pushState entry must commit to the
    // `NavigationController` BEFORE the async pipeline-replacing navigation
    // supersedes, else it is stranded (the navigation early-returns and the
    // history is never drained — the same reason window-opens drain first).
    // Iterate the drained `Vec` in FIFO order: each synchronous
    // `pushState`/`replaceState` is an independent session-history commit. boa's
    // single-slot back-channel yields a 0/1-element Vec today; the VM engine
    // yields every action of the turn — so this site is type-stable across the
    // S5-6 flip (memo §3.2 / §5.1).
    let pending_history = state.pipeline.runtime.take_pending_history();
    let history_applied = !pending_history.is_empty();
    for action in &pending_history {
        if handle_history_action(state, action) {
            // A traversal (`back`/`forward`/`go`) just rebuilt `state.pipeline`
            // (fresh runtime); the REMAINING same-turn history intents were
            // captured from the now-superseded (navigated-away) document, so they
            // must NOT be replayed onto the fresh page — e.g. a trailing
            // `pushState` after a `history.back()` must not mutate the new page's
            // URL/history (Codex R1 P2). Stop the drain here.
            break;
        }
    }

    // Own-context navigation (may replace the pipeline) — AFTER the history
    // above. For a NON-rebuild history action (pushState/replaceState — render
    // nothing, do not touch the pipeline), the entries just committed live on the
    // `NavigationController` (owned by `ContentState`, not the replaced pipeline),
    // so they survive this navigation's rebuild; and the navigation ships its own
    // display list — so the history drain intentionally does NOT send one when a
    // navigation follows (no redundant double-send). This is the pushState+nav
    // common case 5a targets.
    //
    // Caveat — if a TRAVERSAL history action (`back`/`forward`/`go`) in the loop
    // above rebuilt `state.pipeline`, the loop already BROKE (the remaining
    // same-turn history intents, captured from the superseded document, were
    // dropped — not replayed). This `take_pending_navigation()` then reads the
    // FRESH (empty) runtime, so a same-turn `pending_navigation` from the
    // pre-traversal runtime is likewise discarded and the traversal wins — the
    // bounded same-turn traversal+navigation race the plan carves as §6-E7 /
    // §8-D5 (`#11-traversal-navigation-same-turn-race`), "one wins" either way
    // (nav pre-5a, traversal post-5a), within D5's deferred envelope.
    if let Some(nav_req) = state.pipeline.runtime.take_pending_navigation() {
        let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &nav_req.url);
        if let Some(target_url) = resolved {
            state.send_display_list();
            handle_navigate(state, &target_url, false, None);
            return true;
        }
    }

    // No navigation applied — a pure-history turn ships its display list now and
    // reports the own-context action (preserving the prior single-action path
    // exactly: history-only turns render + return true).
    //
    // Caveat — for a pure TRAVERSAL turn (e.g. `history.back()` alone),
    // `handle_navigate`'s own `notify_navigation` already sent a display list, so
    // this trailing send is a second one. That double-send is PRE-EXISTING (the
    // old order ran the identical `handle_history_action(...); send_display_list()`
    // pair) and is unchanged by 5a; the no-redundant-double-send guarantee above
    // is about the pushState+navigation case, not traversal.
    if history_applied {
        state.send_display_list();
        return true;
    }

    // window.focus() is handled in content/mod.rs drain loop — do not duplicate here.

    false
}

/// The observable outcome of routing a `window.open` intent batch — what the
/// caller needs to decide render + "did an action happen".
pub(crate) struct WindowOpenOutcome {
    /// A named HIT re-navigated an iframe → OUR render changed, so the caller
    /// must re-render before flushing the display list.
    pub navigated_iframe: bool,
    /// Any REAL browser effect occurred — a tab was opened (`OpenNewTab`) or an
    /// iframe was navigated — as opposed to every intent being a dropped no-op
    /// (a sandbox-blocked named MISS, an empty-url HIT, a blocked-scheme URL).
    /// Callers that suppress a fallback (e.g. a link's default navigation when
    /// an onclick called `window.open`) gate on THIS, not on the queue being
    /// non-empty — a no-op `window.open` must not swallow the default action.
    pub any_effect: bool,
}

/// Route the drained ordered `window.open` intents (WHATWG HTML §7.2.2.1) in
/// call order — popup and named opens interleaved on ONE queue so a later
/// `_blank` never surfaces before an earlier named MISS. Shared by BOTH drain
/// pumps (`process_pending_actions` and the async `run_event_loop`) so the
/// routing has one home — a named open from a pure-async turn (a timer /
/// postMessage with no later user input) reaches the same routing as an
/// input-driven one (edge E4).
///
/// Every produced tab / iframe URL is run through the shell navigation
/// chokepoint [`resolve_nav_url`] (same as link / location navigation), so a
/// `javascript:` / `vbscript:` `window.open` URL is blocked rather than
/// forwarded as an `OpenNewTab` for a scheme the normal paths reject.
///
/// Per intent:
/// - **[`WindowOpenIntent::Popup`]** → `OpenNewTab` (blocked-scheme-filtered).
/// - **[`WindowOpenIntent::NamedFrame`]** resolved against the current
///   document's iframe tree:
///   - **HIT** (`find_iframe_by_name` matches a descendant iframe) →
///     `navigate_iframe`, **ungated**. Spec-correct: `find_iframe_by_name`
///     (`content/iframe/lifecycle.rs`) searches only the current document's
///     iframes, so the source is an ancestor of the target ⇒ HTML §7.4.2.4
///     step 2 discharges the *allowed by sandboxing to navigate* check
///     unconditionally. Revisit if the lookup ever widens beyond descendants
///     (folded into slot `#11-browsing-context-model-window-open-postmessage`).
///   - **MISS** → promote to a new tab **only when** the payload's call-time
///     `aux_nav_allowed` snapshot permits (§7.3.1.7 step 3 snapshots the
///     sandboxing flag set at call time — never re-read live flags). A
///     sandboxed no-`allow-popups` MISS is dropped silently (§7.3.1.7 step 8
///     sandboxed auxiliary navigation — "may report to a developer console").
///     The previously-ungated promotion was the sandbox bypass this slice
///     closes.
///   - `url == None` (empty-url open, null urlRecord): a HIT is a NO-OP
///     (§7.2.2.1 step 16.1 navigates only for a non-null urlRecord); a MISS
///     defaults to about:blank (step 15.3).
pub(crate) fn route_window_opens(
    state: &mut ContentState,
    intents: Vec<elidex_script_session::WindowOpenIntent>,
) -> WindowOpenOutcome {
    use elidex_script_session::WindowOpenIntent;
    let base = state.pipeline.url.clone();
    let mut navigated_iframe = false;
    let mut any_effect = false;
    for intent in intents {
        match intent {
            WindowOpenIntent::Popup(req) => {
                if let Some(url) = resolve_nav_url(base.as_ref(), &req.url) {
                    state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(url));
                    any_effect = true;
                }
            }
            WindowOpenIntent::NamedFrame(nav) => {
                if let Some(iframe_entity) = super::iframe::find_iframe_by_name(state, &nav.name) {
                    // HIT — existing navigable: navigate only for a non-null
                    // urlRecord (§7.2.2.1 step 16.1); an empty-url open is a no-op.
                    if let Some(url) = nav
                        .url
                        .as_deref()
                        .and_then(|u| resolve_nav_url(base.as_ref(), u))
                    {
                        super::iframe::navigate_iframe(state, iframe_entity, &url);
                        navigated_iframe = true;
                        any_effect = true;
                    }
                } else if nav.aux_nav_allowed {
                    // MISS — new navigable: a null urlRecord defaults to
                    // about:blank (§7.2.2.1 step 15.3).
                    let url_str = nav.url.as_deref().unwrap_or("about:blank");
                    if let Some(url) = resolve_nav_url(base.as_ref(), url_str) {
                        state.notify_browser(crate::ipc::ContentToBrowser::OpenNewTab(url));
                        any_effect = true;
                    }
                }
                // else: MISS without an aux-nav grant → drop (sandboxed
                // auxiliary navigation, §7.3.1.7 step 8).
            }
        }
    }
    WindowOpenOutcome {
        navigated_iframe,
        any_effect,
    }
}

/// Apply a single history action. Returns `true` iff it **superseded the
/// document via a pipeline-rebuilding traversal** — a `Back`/`Forward`/`Go`
/// whose `NavigationController` move returned a target URL and drove
/// `handle_navigate(is_history_nav)`. Returns `false` for
/// `PushState`/`ReplaceState` (no rebuild) AND for a **no-op traversal** (an
/// out-of-range `go` / empty `go_back`/`go_forward` returning `None` → no
/// `handle_navigate`, so the drain loop must CONTINUE past it). The FIFO drain
/// loop keys on this to STOP replaying the remaining same-turn intents once a
/// traversal navigates away — they belong to the now-superseded document (Codex
/// R1 P2). (Whether the traversal's subsequent load ultimately succeeds does not
/// change the answer: the branch was taken, the document's intent stream is
/// superseded.)
pub(super) fn handle_history_action(
    state: &mut ContentState,
    action: &elidex_script_session::HistoryAction,
) -> bool {
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
                true
            } else {
                false
            }
        }
        elidex_script_session::HistoryAction::Go(delta) => {
            if let Some(url) = state.nav_controller.go(*delta).cloned() {
                handle_navigate(state, &url, true, None);
                true
            } else {
                false
            }
        }
        elidex_script_session::HistoryAction::PushState { url, .. }
        | elidex_script_session::HistoryAction::ReplaceState { url, .. } => {
            let replace = matches!(
                action,
                elidex_script_session::HistoryAction::ReplaceState { .. }
            );
            apply_push_replace_state(state, url.as_deref(), replace);
            false
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
