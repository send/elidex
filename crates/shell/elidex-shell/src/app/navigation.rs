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

        // Drain the `window.open` back-channel FIRST so it cannot leak across a
        // navigation (a queue left un-drained would surface on the next task).
        // Legacy inline mode has no new-tab capability (`ChromeAction::NewTab`
        // is threaded-mode only, see `handle_chrome_action`) and no iframe
        // registry (`InteractiveState` carries no iframes — iframes are a
        // content-thread facility), so the whole ordered window.open queue is
        // drained-and-dropped here. Draining first (unconditional, mirroring the
        // content thread's `process_pending_actions`) also closes the prior leak
        // where an early navigation/history return skipped the drop. Threaded
        // content mode does the real routing in
        // `content/navigation.rs::process_pending_actions`.
        let _ = interactive.pipeline.runtime.take_pending_window_opens();

        // Own-context HISTORY drain BEFORE navigation (WHATWG HTML §7.4.4), FIFO
        // — mirrors the content thread: a synchronous `pushState`/`replaceState`
        // must commit its session-history entry before an async
        // pipeline-replacing navigation supersedes, else a same-turn
        // `pushState('/a'); location.href='/b'` strands `/a`. boa yields a
        // 0/1-element Vec; the VM yields every action of the turn (type-stable
        // across the S5-6 flip).
        let pending_history = interactive.pipeline.runtime.take_pending_history();
        let history_applied = !pending_history.is_empty();
        for action in &pending_history {
            if self.handle_history_action(action) {
                // A traversal's load SUCCEEDED and rebuilt the pipeline — return
                // IMMEDIATELY rather than falling through to
                // `take_pending_navigation()` below, which would drain a
                // `location.*` the freshly-loaded page's initial scripts queued
                // onto the FRESH runtime (Codex #283). A no-op or failed-load
                // traversal returns `false` and does NOT reach here, so the loop
                // CONTINUES and trailing same-turn intents still apply (Codex R1
                // P2 / R2). Mirrors
                // `content/navigation.rs::process_pending_actions`.
                return true;
            }
        }

        // Re-borrow required by the borrow-checker: the loop above called
        // `handle_history_action` (`&mut self`), so `self.interactive` needs a
        // fresh borrow here. It stays `Some` — no path ever clears it
        // (`navigate`/`navigate_to_history_url`/`load_url_into_pipeline` replace
        // `interactive.pipeline` IN PLACE, never `self.interactive = None`) — so
        // the `else` is an unreachable destructuring formality, not a real
        // "interactive was dropped" path.
        let Some(interactive) = &mut self.interactive else {
            return history_applied;
        };

        // Own-context navigation — AFTER the history above.
        if let Some(nav_req) = interactive.pipeline.runtime.take_pending_navigation() {
            let resolved = resolve_nav_url(interactive.pipeline.url.as_ref(), &nav_req.url);
            if let Some(target_url) = resolved {
                self.navigate(&target_url, nav_req.replace);
                return true;
            }
        }

        history_applied
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

    /// Navigate to a URL from the history (back/forward). Returns `true` iff the
    /// pipeline was **replaced (the load succeeded)** — the inline mirror of
    /// `content/navigation.rs::handle_navigate`'s success signal, so a
    /// failed-load traversal does NOT supersede the same-turn history drain
    /// (Codex R2).
    pub(super) fn navigate_to_history_url(&mut self, url: &url::Url) -> bool {
        if !self.load_url_into_pipeline(url) {
            return false;
        }
        let Some(interactive) = self.interactive.as_mut() else {
            return false;
        };
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        if let Some(state) = &self.render_state {
            state.window.set_title(&interactive.window_title);
        }
        true
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
                // Rebuild at the current viewport + device facts, not `DEFAULT`
                // (C1/C3 — same as the content-thread navigation path). The fresh
                // document's bridge would default to 1×/Light, so carry the current
                // facts forward like the cookie jar (legacy inline mode has no
                // viewport-cell; the bridge is the SoT here).
                let viewport = interactive.pipeline.viewport;
                let device_facts = crate::ipc::DeviceFacts {
                    dppx: interactive.pipeline.runtime.bridge().device_pixel_ratio(),
                    color_scheme: interactive.pipeline.runtime.bridge().color_scheme(),
                };
                let new_pipeline = crate::build_pipeline_from_loaded(
                    loaded,
                    network_handle,
                    font_db,
                    cookie_jar,
                    viewport,
                    device_facts,
                    // Top-level document: no frame security (URL-derived origin).
                    None,
                );
                interactive.pipeline = new_pipeline;
                interactive
                    .pipeline
                    .runtime
                    .set_current_url(Some(url.clone()));
                interactive.window_title = format!("elidex \u{2014} {url}");
                // Focus lives in the new pipeline's `EcsDom` (fresh document,
                // empty by construction) — no field to reset.
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

    /// Handle a pending history action from JS. Returns `true` iff it
    /// **superseded the document via a pipeline-rebuilding traversal that
    /// LOADED** — a `Back`/`Forward`/`Go` whose `NavigationController` peek
    /// yielded a target AND whose `navigate_to_history_url` load succeeded
    /// (replaced the pipeline). Returns `false` for `PushState`/`ReplaceState`,
    /// for a no-op traversal (no target), and for a traversal whose load FAILED
    /// (old document still active). Mirrors
    /// `content/navigation.rs::handle_history_action`: the drain loop returns
    /// only on a genuine rebuild, so a no-op / failed-load traversal leaves the
    /// current document active and the remaining same-turn intents still apply
    /// (Codex R1 P2 / R2). The traversal is atomic by construction
    /// (peek-then-commit — Codex R3): the cursor is committed (`commit_index`)
    /// only after the load succeeds, so a failed load never moves it (no rollback
    /// path).
    pub(super) fn handle_history_action(
        &mut self,
        action: &elidex_script_session::HistoryAction,
    ) -> bool {
        let Some(interactive) = &mut self.interactive else {
            return false;
        };

        match action {
            elidex_script_session::HistoryAction::Back
            | elidex_script_session::HistoryAction::Forward => {
                // Peek the target WITHOUT moving the cursor, then commit the move
                // ONLY on a successful load — an atomic traversal (Codex R3,
                // mirrors `content/navigation.rs`). A failed load leaves the
                // cursor on the still-active document (no rollback needed — it
                // never moved). Clone the URL to drop the `interactive` borrow
                // before the `&mut self` load below.
                let peeked = if matches!(action, elidex_script_session::HistoryAction::Back) {
                    interactive.nav_controller.peek_back()
                } else {
                    interactive.nav_controller.peek_forward()
                };
                let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
                    return false;
                };
                if self.navigate_to_history_url(&url) {
                    if let Some(interactive) = self.interactive.as_mut() {
                        interactive.nav_controller.commit_index(target_index);
                    }
                    true
                } else {
                    false
                }
            }
            elidex_script_session::HistoryAction::Go(delta) => {
                let Some((target_index, url)) = interactive
                    .nav_controller
                    .peek_go(*delta)
                    .map(|(i, u)| (i, u.clone()))
                else {
                    return false;
                };
                if self.navigate_to_history_url(&url) {
                    if let Some(interactive) = self.interactive.as_mut() {
                        interactive.nav_controller.commit_index(target_index);
                    }
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
                if let Some(resolved_url) =
                    resolve_state_url(interactive.pipeline.url.as_ref(), url.as_deref())
                {
                    apply_state_change(interactive, &resolved_url, replace);
                    interactive.window_title = format!("elidex \u{2014} {resolved_url}");
                    if let Some(state) = &self.render_state {
                        state.window.set_title(&interactive.window_title);
                    }
                }
                false
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
