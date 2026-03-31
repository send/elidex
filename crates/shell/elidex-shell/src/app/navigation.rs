//! URL navigation, history actions, and chrome action handling.

use super::App;
use super::InteractiveState;

impl App {
    /// Check for and process any pending JS navigation or history action.
    ///
    /// Called after event dispatch + re-render. Returns `true` if a navigation
    /// or history action was processed, so the caller can skip further default
    /// actions (e.g. link navigation).
    pub(super) fn process_pending_navigation(&mut self) -> bool {
        let Some(interactive) = &mut self.interactive else {
            return false;
        };
        let pipeline = &mut interactive.pipeline;

        if let Some(nav_req) = pipeline.runtime.take_pending_navigation() {
            let resolved = resolve_nav_url(pipeline.url.as_ref(), &nav_req.url);
            if let Some(target_url) = resolved {
                self.navigate(&target_url, nav_req.replace);
                return true;
            }
        }

        // Re-borrow interactive since self.navigate may have consumed it above.
        let Some(interactive) = &mut self.interactive else {
            return false;
        };
        if let Some(action) = interactive.pipeline.runtime.take_pending_history() {
            self.handle_history_action(&action);
            return true;
        }

        false
    }

    /// Navigate to a new URL, replacing the current pipeline.
    ///
    /// When `replace` is `true`, the current history entry is replaced
    /// (matching `location.replace()` semantics). Otherwise a new entry
    /// is pushed onto the history stack.
    pub(super) fn navigate(&mut self, url: &url::Url, replace: bool) {
        if !self.load_url_into_pipeline(url) {
            return;
        }
        let Some(interactive) = self.interactive.as_mut() else {
            return;
        };
        if replace {
            interactive.nav_controller.replace(url.clone());
        } else {
            interactive.nav_controller.push(url.clone());
        }
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        if let Some(state) = &self.render_state {
            state.window.set_title(&interactive.window_title);
        }
    }

    /// Navigate to a URL from the history (back/forward).
    pub(super) fn navigate_to_history_url(&mut self, url: &url::Url) {
        if !self.load_url_into_pipeline(url) {
            return;
        }
        let Some(interactive) = self.interactive.as_mut() else {
            return;
        };
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        if let Some(state) = &self.render_state {
            state.window.set_title(&interactive.window_title);
        }
    }

    /// Load a URL into the current pipeline, updating interactive state.
    ///
    /// Shared by `navigate` and `navigate_to_history_url`.
    /// Returns `true` on success, `false` on error.
    fn load_url_into_pipeline(&mut self, url: &url::Url) -> bool {
        let Some(interactive) = &mut self.interactive else {
            return false;
        };
        let network_handle = std::rc::Rc::clone(&interactive.pipeline.network_handle);
        let font_db = std::sync::Arc::clone(&interactive.pipeline.font_db);
        match elidex_navigation::load_document(url, &network_handle, None) {
            Ok(loaded) => {
                let cookie_jar = interactive.pipeline.runtime.bridge().cookie_jar_clone();
                let new_pipeline =
                    crate::build_pipeline_from_loaded(loaded, network_handle, font_db, cookie_jar);
                interactive.pipeline = new_pipeline;
                interactive
                    .pipeline
                    .runtime
                    .set_current_url(Some(url.clone()));
                interactive.window_title = format!("elidex \u{2014} {url}");
                interactive.focus_target = None;
                interactive.hover_chain.clear();
                interactive.active_chain.clear();
                interactive.chrome.set_url(url);
                true
            }
            Err(e) => {
                eprintln!("Navigation error: {e}");
                false
            }
        }
    }

    /// Handle a pending history action from JS.
    pub(super) fn handle_history_action(&mut self, action: &elidex_navigation::HistoryAction) {
        let Some(interactive) = &mut self.interactive else {
            return;
        };

        match action {
            elidex_navigation::HistoryAction::Back | elidex_navigation::HistoryAction::Forward => {
                let url = if matches!(action, elidex_navigation::HistoryAction::Back) {
                    interactive.nav_controller.go_back().cloned()
                } else {
                    interactive.nav_controller.go_forward().cloned()
                };
                if let Some(url) = url {
                    self.navigate_to_history_url(&url);
                }
            }
            elidex_navigation::HistoryAction::Go(delta) => {
                if let Some(url) = interactive.nav_controller.go(*delta).cloned() {
                    self.navigate_to_history_url(&url);
                }
            }
            elidex_navigation::HistoryAction::PushState { url, .. }
            | elidex_navigation::HistoryAction::ReplaceState { url, .. } => {
                let replace = matches!(
                    action,
                    elidex_navigation::HistoryAction::ReplaceState { .. }
                );
                if let Some(resolved_url) =
                    resolve_state_url(interactive.pipeline.url.as_ref(), url.as_deref())
                {
                    apply_state_change(interactive, &resolved_url, replace);
                    interactive.window_title = format!("elidex \u{2014} {resolved_url}");
                    if let Some(state) = &self.render_state {
                        state.window.set_title(&interactive.window_title);
                    }
                }
            }
        }
    }

    /// Handle a chrome action (navigation, back, forward, reload).
    pub(super) fn handle_chrome_action(&mut self, action: crate::chrome::ChromeAction) {
        match action {
            crate::chrome::ChromeAction::Navigate(url_str) => {
                // Try parsing as-is first, then with https:// prefix.
                let parsed = url::Url::parse(&url_str)
                    .or_else(|_| url::Url::parse(&format!("https://{url_str}")));
                match parsed {
                    // navigate() calls chrome.set_url() on success internally.
                    Ok(url) => self.navigate(&url, false),
                    Err(e) => eprintln!("Invalid URL: {e}"),
                }
            }
            crate::chrome::ChromeAction::Back | crate::chrome::ChromeAction::Forward => {
                let is_back = matches!(action, crate::chrome::ChromeAction::Back);
                let url = self.interactive.as_mut().and_then(|i| {
                    if is_back {
                        i.nav_controller.go_back().cloned()
                    } else {
                        i.nav_controller.go_forward().cloned()
                    }
                });
                if let Some(url) = url {
                    self.navigate_to_history_url(&url);
                }
            }
            crate::chrome::ChromeAction::Reload => {
                let url = self
                    .interactive
                    .as_ref()
                    .and_then(|i| i.pipeline.url.clone());
                if let Some(url) = url {
                    self.navigate(&url, true);
                }
            }
            // Tab actions are only handled in threaded mode.
            crate::chrome::ChromeAction::NewTab
            | crate::chrome::ChromeAction::CloseTab(_)
            | crate::chrome::ChromeAction::SwitchTab(_) => {}
        }
    }
}

/// Resolve a `pushState`/`replaceState` URL, enforcing same-origin.
///
/// Per the History API spec, `pushState`/`replaceState` must not change the
/// origin. Returns `None` if the URL is cross-origin or cannot be parsed.
/// If `url_str` is `None`, returns the current URL (no URL change).
fn resolve_state_url(base: Option<&url::Url>, url_str: Option<&str>) -> Option<url::Url> {
    let Some(url_str) = url_str else {
        return base.cloned();
    };
    let resolved = resolve_nav_url(base, url_str)?;
    // Same-origin check: scheme + host + port must match.
    if let Some(current) = base {
        if current.origin() != resolved.origin() {
            eprintln!(
                "SecurityError: pushState/replaceState URL {resolved} has different origin than {current}"
            );
            return None;
        }
    }
    Some(resolved)
}

/// Apply a `pushState`/`replaceState` URL change to interactive state.
///
/// When `replace` is `true`, the current history entry is replaced;
/// otherwise a new entry is pushed.
fn apply_state_change(interactive: &mut InteractiveState, url: &url::Url, replace: bool) {
    interactive.pipeline.url = Some(url.clone());
    if replace {
        interactive.nav_controller.replace(url.clone());
    } else {
        interactive.nav_controller.push(url.clone());
    }
    interactive.chrome.set_url(url);
    interactive
        .pipeline
        .runtime
        .set_current_url(Some(url.clone()));
    interactive
        .pipeline
        .runtime
        .set_history_length(interactive.nav_controller.len());
}

/// URL schemes that must not be navigated to.
///
/// `javascript:` and `vbscript:` URLs execute code in the current context
/// rather than navigating. Allowing them would bypass security boundaries.
const BLOCKED_NAV_SCHEMES: &[&str] = &["javascript", "vbscript"];

/// Resolve a navigation URL string against the current page URL.
///
/// Returns `None` if the URL cannot be parsed or has a blocked scheme
/// (e.g. `javascript:`, `vbscript:`).
pub(crate) fn resolve_nav_url(base: Option<&url::Url>, url_str: &str) -> Option<url::Url> {
    let url = if let Some(base_url) = base {
        base_url.join(url_str).ok()?
    } else {
        url::Url::parse(url_str).ok()?
    };
    if BLOCKED_NAV_SCHEMES.contains(&url.scheme()) {
        eprintln!("Blocked navigation to {}: scheme not allowed", url.scheme());
        return None;
    }
    Some(url)
}
