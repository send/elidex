//! Navigation and history action handling for the content thread.

use std::rc::Rc;

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
pub(super) fn handle_navigate(
    state: &mut ContentState,
    url: &url::Url,
    is_history_nav: bool,
    request: Option<elidex_net::Request>,
) {
    let fetch_handle = Rc::clone(&state.pipeline.fetch_handle);
    let font_db = Rc::clone(&state.pipeline.font_db);

    match elidex_navigation::load_document(url, &fetch_handle, request) {
        Ok(loaded) => {
            let new_pipeline = crate::build_pipeline_from_loaded(loaded, fetch_handle, font_db);
            state.pipeline = new_pipeline;
            state.focus_target = None;
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

    false
}

pub(super) fn handle_history_action(
    state: &mut ContentState,
    action: &elidex_navigation::HistoryAction,
) {
    match action {
        elidex_navigation::HistoryAction::Back | elidex_navigation::HistoryAction::Forward => {
            let url = if matches!(action, elidex_navigation::HistoryAction::Back) {
                state.nav_controller.go_back().cloned()
            } else {
                state.nav_controller.go_forward().cloned()
            };
            if let Some(url) = url {
                handle_navigate(state, &url, true, None);
            }
        }
        elidex_navigation::HistoryAction::Go(delta) => {
            if let Some(url) = state.nav_controller.go(*delta).cloned() {
                handle_navigate(state, &url, true, None);
            }
        }
        elidex_navigation::HistoryAction::PushState { url, .. }
        | elidex_navigation::HistoryAction::ReplaceState { url, .. } => {
            let replace = matches!(
                action,
                elidex_navigation::HistoryAction::ReplaceState { .. }
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
        let _ = state.channel.send(ContentToBrowser::TitleChanged(title));
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
