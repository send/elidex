//! URL navigation, history actions, and chrome action handling.

use elidex_script_session::{HistoryStepEvents, HostDriver, NavigationType};

use super::App;
use super::InteractiveState;

/// Which same-document history-step application [`App::same_document_step`] is
/// applying — the app-mode mirror of `content/navigation.rs::SameDocStep`. One
/// primitive, parameterized over the two consumers (a fresh fragment navigation
/// vs a same-document traversal); NOT a fork.
enum AppSameDocStep {
    /// A fresh fragment navigation (WHATWG HTML §7.4.2.3.3): push/replace per the
    /// nav-type (or replace for a URL equal to the active entry's, §7.4.2.2 step
    /// 13); popstate `state = null`; scroll resolved from the target fragment.
    FragmentNav(NavigationType),
    /// A same-document traversal (§7.4.6.2 step 6.4): commit the cursor to the
    /// peeked target; popstate `state` = the target entry's serialized state (the
    /// general form — a `None`-state entry ⇒ `null`); scroll = the target entry's
    /// persisted `scroll_position` (step 6.4.4).
    Traversal {
        target_index: usize,
        popstate_state: Option<Vec<u8>>,
        scroll_position: Option<(f64, f64)>,
    },
}

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
                // A traversal handled the turn — a cross-document rebuild that
                // loaded, OR a same-document traversal applied in place (restored +
                // fired popstate). Return IMMEDIATELY rather than falling through to
                // `take_pending_navigation()` below, which (on the rebuild path)
                // would drain a `location.*` the freshly-loaded page's initial
                // scripts queued onto the FRESH runtime (Codex #283). A no-target /
                // failed-load traversal returns `false` and does NOT reach
                // here, so the loop CONTINUES and trailing same-turn intents still
                // apply (Codex R1 P2 / R2). Mirrors
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
                self.navigate(&target_url, nav_req.nav_type);
                return true;
            }
        }

        history_applied
    }

    /// Navigate to a new URL, rebuilding the current pipeline.
    ///
    /// App-mode **honors** the [`NavigationType`] (unlike thread-mode, whose
    /// drain collapses `Replace` → `Push` for the cursor op, §10-D6):
    /// - `Push` (`href=`/`assign`/`<a href>`) pushes a new history entry;
    /// - `Replace` (`location.replace()`) replaces the current entry in place;
    /// - `Reload` (`location.reload()`) rebuilds with **no** cursor move — a
    ///   fragment-URL reload must neither push an entry nor (Phase 2b) take the
    ///   fragment no-rebuild path (§7.4.3 reload, `isSameDocument = false`). The
    ///   enum distinguishes `Reload` from `Replace`, which a `replace: bool`
    ///   could not.
    pub(super) fn navigate(&mut self, url: &url::Url, nav_type: NavigationType) {
        // Capture the departing entry's scroll BEFORE any same-document gate or
        // rebuild (WHATWG HTML §7.4.6.1 *activate history entry* step 1 — the common
        // save-persisted-state chokepoint on leaving ANY entry, CR-6/F4), so a later
        // `back()` to it restores the scroll.
        self.capture_scroll_on_leave();
        // --- Same-document (fragment) navigation (WHATWG HTML §7.4.2.2 navigate
        // step 15) --- App-mode is GET-only (no `documentResource` — that step-15
        // conjunct is vacuous) and honors the nav-type directly, so the fresh-nav
        // conjunct is `nav_type != Reload` (a `Reload` rebuilds with no cursor move,
        // §7.4.3 `isSameDocument = false`; the enum distinguishes it from a same-page
        // `replace()`). Take the no-rebuild path iff the URL classifies SameDocument
        // AND it is not a reload.
        if nav_type != NavigationType::Reload {
            if let Some(current) = self
                .interactive
                .as_ref()
                .and_then(|i| i.pipeline.url.clone())
            {
                if elidex_navigation::classify_navigation(&current, url)
                    == elidex_navigation::NavClass::SameDocument
                {
                    self.same_document_step(&current, url, AppSameDocStep::FragmentNav(nav_type));
                    return;
                }
            }
        }

        if !self.load_url_into_pipeline(url) {
            return;
        }
        let Some(interactive) = self.interactive.as_mut() else {
            return;
        };
        match nav_type {
            // Cross-document (the fragment path early-returned above): a fresh
            // navigation and `location.replace()` are both NEW-document events.
            NavigationType::Push => interactive.nav_controller.push(url.clone()),
            NavigationType::Replace => interactive.nav_controller.replace(url.clone()),
            // A reload replaces the navigable's *document* without moving the cursor
            // — re-stamp the current entry's document identity (else a neighbor entry
            // sharing its pre-reload `document_sequence` mis-classifies same-document
            // on a later traversal).
            NavigationType::Reload => {
                interactive.nav_controller.restamp_current_document();
                // The pre-reload scroll offset was captured onto the current entry
                // (`capture_scroll_on_leave`) and `load_url_into_pipeline` reset
                // `pipeline.scroll_offset` to 0; reapply it so `location.reload()` /
                // chrome Reload lands where the user was (browsers preserve scroll on
                // reload). Mirror of the content-thread `HistoryCursorOp::Keep` restore
                // — the cursor did not move, so the CURRENT entry holds the offset.
                if let Some((x, y)) = interactive.nav_controller.current_scroll_position() {
                    interactive.pipeline.scroll_offset =
                        crate::content::scroll::scroll_offset_from_position((x, y));
                    crate::re_render(&mut interactive.pipeline);
                }
            }
        }
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        if let Some(state) = &self.render_state {
            state.window.set_title(&interactive.window_title);
        }
    }

    /// Capture the current viewport scroll offset onto the entry being LEFT, BEFORE
    /// a navigation moves the cursor or rebuilds the pipeline (WHATWG HTML §7.4.6.1
    /// *activate history entry* step 1 — save persisted state on leaving any entry;
    /// CR-6). App-mode's mirror of `content/navigation.rs::capture_scroll_on_leave`.
    fn capture_scroll_on_leave(&mut self) {
        if let Some(interactive) = self.interactive.as_mut() {
            let off = interactive.pipeline.scroll_offset;
            interactive
                .nav_controller
                .set_current_scroll((f64::from(off.x), f64::from(off.y)));
        }
    }

    /// Apply a same-document history-step IN PLACE (no pipeline rebuild) — app-mode's
    /// mirror of `content/navigation.rs::same_document_step`, the shared primitive for
    /// BOTH a fresh fragment navigation (WHATWG HTML §7.4.2.3.3) and a same-document
    /// traversal (§7.4.6.2), selected by `step`. `current` is the pre-step URL,
    /// `target` the destination. No rebuild, so the document + its `EcsDom` (incl.
    /// focus) persist; the document origin stays correct BY CONSTRUCTION
    /// (`set_current_url` re-derives the same URL-tuple origin — only the
    /// fragment/entry changed). App-mode HONORS the nav-type push-vs-replace
    /// distinction (thread-mode collapses `Replace` → `Push`, §10-D6).
    fn same_document_step(&mut self, current: &url::Url, target: &url::Url, step: AppSameDocStep) {
        let Some(interactive) = self.interactive.as_mut() else {
            return;
        };
        // Update the current document URL (shell copy + runtime `current_url`) —
        // no `load_url_into_pipeline`, no rebuild.
        interactive.pipeline.url = Some(target.clone());
        interactive
            .pipeline
            .runtime
            .set_current_url(Some(target.clone()));
        // Move the session-history cursor. A FRESH fragment nav to the URL the
        // active entry ALREADY has (incl. fragment) REPLACES it regardless of
        // nav-type — §7.4.2.2 step 13 resolves `historyHandling` to "replace" for
        // an equal URL (so re-navigating to the current `#id` does not grow
        // `history.length`); otherwise honor push-vs-replace (`Reload` is excluded
        // by the caller — fail loud). A TRAVERSAL commits the peeked target entry.
        match &step {
            AppSameDocStep::FragmentNav(nav_type) => {
                // A fragment navigation stays in the CURRENT document → inherit its
                // `document_sequence` (same-document push/replace).
                if current == target {
                    interactive
                        .nav_controller
                        .replace_same_document(target.clone());
                } else {
                    match nav_type {
                        NavigationType::Push => {
                            interactive
                                .nav_controller
                                .push_same_document(target.clone());
                        }
                        NavigationType::Replace => {
                            interactive
                                .nav_controller
                                .replace_same_document(target.clone());
                        }
                        NavigationType::Reload => {
                            unreachable!(
                                "reload never takes the same-document fragment path (excluded by `navigate`)"
                            )
                        }
                    }
                }
                // A fragment navigation sets `history.state` to null (§7.4.2.3.3 step
                // 11.1 — the popstate below fires `Some(None)`); a same-URL replace
                // would keep the prior `pushState`'d state, so clear it (F4).
                interactive.nav_controller.set_current_state(None);
            }
            AppSameDocStep::Traversal { target_index, .. } => {
                interactive.nav_controller.commit_index(*target_index);
            }
        }
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        interactive.window_title = format!("elidex \u{2014} {target}");
        interactive.chrome.set_url(target);
        // Resolve the scroll offset against the current (pre-step) layout, BEFORE
        // firing popstate (below) so it reads live geometry. App-mode has no
        // viewport-scroll application seam (unlike the content thread's `re_render`
        // clamp/echo/`ScrollState` path), so the offset is set on the pipeline for
        // the free `re_render` to read; the missing clamp/echo is an app-mode-wide
        // gap (folds into the driver-unification backlog, cluster §8-D4), not a
        // 5c one. A FRAGMENT nav resolves from the target fragment (§7.4.6.4, the
        // shared engine-independent helper — One-issue-one-way with the content
        // thread); a TRAVERSAL restores the target entry's persisted scroll
        // (§7.4.6.2 step 6.4.4).
        let offset = match &step {
            AppSameDocStep::FragmentNav(_) => crate::content::scroll::scroll_offset_for_fragment(
                &interactive.pipeline.dom,
                interactive.pipeline.document,
                target.fragment().unwrap_or_default(),
                interactive.pipeline.scroll_offset,
                interactive.pipeline.viewport.width,
            ),
            AppSameDocStep::Traversal {
                scroll_position, ..
            } => scroll_position.map(crate::content::scroll::scroll_offset_from_position),
        };
        // §7.4.6.2 step 6.3 + 6.4.3 (also §7.4.2.3.3 step 14): restore
        // `history.state` then fire popstate SYNCHRONOUSLY, BEFORE the scroll — a
        // synchronous popstate handler observes the PRE-scroll position. popstate
        // is state-AGNOSTIC (fires whenever the entry changed): a fragment nav
        // carries `null` (`Some(None)`), a traversal the target entry's serialized
        // state (`Some(Some(bytes))`/`Some(None)`). hashchange is a queued task
        // that fires AFTER the scroll (below). The VM fires; boa stubs — flip-inert
        // until S5-6.
        // Consume `step` here (moving the serialized state out — no clone).
        let popstate_state = match step {
            AppSameDocStep::FragmentNav(_) => None,
            AppSameDocStep::Traversal { popstate_state, .. } => popstate_state,
        };
        interactive
            .pipeline
            .runtime
            .deliver_history_step_events(HistoryStepEvents {
                popstate_state: Some(popstate_state),
                hashchange: None,
            });
        // step 6.4.4 / §7.4.2.3.3 step 15: scroll.
        if let Some(offset) = offset {
            interactive.pipeline.scroll_offset = offset;
        }
        crate::re_render(&mut interactive.pipeline);
        // §7.4.6.2 step 6.4.5: the hashchange task, queued at the fire above, runs
        // as a LATER task — after the synchronous scroll — iff the fragment
        // differs. Delivering it in a second call keeps the spec order
        // popstate → scroll → hashchange (and popstate strictly-before-hashchange).
        if let Some(hashchange) = (current.fragment() != target.fragment())
            .then(|| (current.to_string(), target.to_string()))
        {
            interactive
                .pipeline
                .runtime
                .deliver_history_step_events(HistoryStepEvents {
                    popstate_state: None,
                    hashchange: Some(hashchange),
                });
        }
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
                    // App-mode does not seed the rebuilt document's `history.state`
                    // on a cross-document traversal (its value is flip-inert — boa
                    // passes `None` on every pushState; the seed-threading lands
                    // only on the content thread, plan §5.5). Same-document
                    // traversals still restore + fire in place (`handle_history_action`).
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
                // Peek the target WITHOUT moving the cursor; `traverse_to` commits
                // it only after a successful cross-document load, or restores +
                // fires in place for a same-document target. Clone the URL to drop
                // the `interactive` borrow before the `&mut self` traversal below.
                let peeked = if matches!(action, elidex_script_session::HistoryAction::Back) {
                    interactive.nav_controller.peek_back()
                } else {
                    interactive.nav_controller.peek_forward()
                };
                let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
                    return false;
                };
                self.traverse_to(target_index, &url)
            }
            elidex_script_session::HistoryAction::Go(delta) => {
                let Some((target_index, url)) = interactive
                    .nav_controller
                    .peek_go(*delta)
                    .map(|(i, u)| (i, u.clone()))
                else {
                    return false;
                };
                self.traverse_to(target_index, &url)
            }
            elidex_script_session::HistoryAction::PushState {
                url,
                serialized_state,
                ..
            }
            | elidex_script_session::HistoryAction::ReplaceState {
                url,
                serialized_state,
                ..
            } => {
                let replace = matches!(
                    action,
                    elidex_script_session::HistoryAction::ReplaceState { .. }
                );
                if let Some(resolved_url) =
                    resolve_state_url(interactive.pipeline.url.as_ref(), url.as_deref())
                {
                    apply_state_change(
                        interactive,
                        &resolved_url,
                        replace,
                        serialized_state.clone(),
                    );
                    interactive.window_title = format!("elidex \u{2014} {resolved_url}");
                    if let Some(state) = &self.render_state {
                        state.window.set_title(&interactive.window_title);
                    }
                }
                false
            }
        }
    }

    /// Apply a session-history traversal to `target_index` (WHATWG HTML §7.4.6.1),
    /// classified by document identity (`resolve_traversal`). Returns `true` iff it
    /// handled the turn — a cross-document rebuild that loaded (cursor committed;
    /// a `go(0)` reload is this arm), OR a same-document traversal applied in place.
    /// Returns `false` only on a cross-document load FAILURE (old document still
    /// active, no rollback — the cursor never moved). Mirrors
    /// `content/navigation.rs::handle_navigate`'s Commit arm.
    pub(super) fn traverse_to(&mut self, target_index: usize, target: &url::Url) -> bool {
        // Scoped borrow: capture scroll-on-leave, then classify the traversal by
        // DOCUMENT IDENTITY (`resolve_traversal`, §7.4.6.1 step 14.10 — NOT URL, so
        // `pushState` routing across different URLs is same-document, and a `go(0)`
        // reload is `Rebuild`). Drop the borrow before the `&mut self`
        // restore/rebuild below.
        // Capture scroll-on-leave onto the entry being LEFT, BEFORE the cursor moves
        // or a cross-document rebuild resets `pipeline.scroll_offset` (DR-4/CR-6).
        self.capture_scroll_on_leave();
        let (kind, current) = {
            let Some(interactive) = self.interactive.as_mut() else {
                return false;
            };
            let current = interactive.pipeline.url.clone();
            let kind = interactive.nav_controller.resolve_traversal(target_index);
            (kind, current)
        };

        match kind {
            // Same-document → restore state + scroll and fire popstate in place.
            elidex_navigation::TraversalKind::SameDocument {
                state: popstate_state,
                scroll: scroll_position,
            } => {
                // A same-document traversal implies a current document URL.
                let current =
                    current.expect("same-document traversal implies a current document URL");
                self.same_document_step(
                    &current,
                    target,
                    AppSameDocStep::Traversal {
                        target_index,
                        popstate_state,
                        scroll_position,
                    },
                );
                return true;
            }
            // Cross-document (incl. a `go(0)` reload) → fall through to the rebuild.
            elidex_navigation::TraversalKind::Rebuild => {}
        }

        // Cross-document → rebuild + commit the cursor only on a successful load
        // (atomic peek-then-commit — a failed load leaves the cursor on the still-
        // active document, no rollback). The rebuilt target is a FRESH document, so
        // re-stamp its identity (mirror of the content-thread Commit-rebuild arm) —
        // else the rebuilt target keeps the `document_sequence` it shared with its
        // former pushState/fragment siblings and a later traversal to such a sibling
        // mis-classifies same-document. App-mode does NOT seed the rebuilt
        // document's `history.state` on a cross-document traversal — an ADDRESSABLE
        // gap (the target entry carries the state; app-mode could read it exactly
        // like content-mode), deliberately deferred because it is flip-inert (boa
        // passes `None` on every pushState) AND app-mode's rebuild path is folded
        // into the §8-D4 driver-unification audit, which threads it once at S5-6
        // rather than duplicating content-mode's seed plumbing here now.
        // Read the target entry's persisted scroll BEFORE the rebuild resets
        // `pipeline.scroll_offset`; reapply it after the commit (mirror of the
        // content-thread `restored_scroll`, R2-F3). App-mode's scroll seam is a plain
        // set-offset + `re_render` — no clamp/echo (the §8-D4 gap), but the offset
        // still lands instead of the page staying at the top.
        let restored_scroll = self
            .interactive
            .as_ref()
            .and_then(|i| i.nav_controller.entry(target_index))
            .and_then(|e| e.scroll_position);
        if self.navigate_to_history_url(target) {
            if let Some(interactive) = self.interactive.as_mut() {
                interactive.nav_controller.commit_index(target_index);
                interactive.nav_controller.restamp_current_document();
                if let Some((x, y)) = restored_scroll {
                    interactive.pipeline.scroll_offset =
                        crate::content::scroll::scroll_offset_from_position((x, y));
                    crate::re_render(&mut interactive.pipeline);
                }
            }
            true
        } else {
            false
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
                    Ok(url) => self.navigate(&url, NavigationType::Push),
                    Err(e) => eprintln!("Invalid URL: {e}"),
                }
            }
            crate::chrome::ChromeAction::Back | crate::chrome::ChromeAction::Forward => {
                // Route the toolbar Back/Forward through the SAME peek-then-commit
                // path as JS `history.back()`/`forward()` (`traverse_to` →
                // `resolve_traversal`), so a same-document toolbar traversal restores
                // state + scroll and fires popstate in place (One-issue-one-way with
                // the JS API), and the eager `go_back`/`go_forward` non-atomic commit
                // is retired.
                let is_back = matches!(action, crate::chrome::ChromeAction::Back);
                let peeked = self.interactive.as_ref().and_then(|i| {
                    if is_back {
                        i.nav_controller.peek_back()
                    } else {
                        i.nav_controller.peek_forward()
                    }
                    .map(|(idx, u)| (idx, u.clone()))
                });
                if let Some((target_index, url)) = peeked {
                    self.traverse_to(target_index, &url);
                }
            }
            crate::chrome::ChromeAction::Reload => {
                let url = self
                    .interactive
                    .as_ref()
                    .and_then(|i| i.pipeline.url.clone());
                if let Some(url) = url {
                    // Chrome reload is `Reload`, NOT `Replace` (the old `true`
                    // meant reload) — else a fragment-URL (`/a#x`) chrome-reload
                    // would take Phase 2b's fragment no-rebuild path and skip the
                    // rebuild (the exact `reload()`-vs-`replace()` collision the
                    // enum fixes).
                    self.navigate(&url, NavigationType::Reload);
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
fn apply_state_change(
    interactive: &mut InteractiveState,
    url: &url::Url,
    replace: bool,
    serialized_state: Option<Vec<u8>>,
) {
    interactive.pipeline.url = Some(url.clone());
    // `pushState`/`replaceState` are SAME-document mutations → inherit the current
    // document's `document_sequence` (a later traversal between this entry and its
    // document-siblings restores in place, not a rebuild).
    if replace {
        interactive
            .nav_controller
            .replace_same_document(url.clone());
    } else {
        // A `pushState` (non-replace) pushes a NEW entry, leaving the current one —
        // capture its scroll first (§7.4.6.1 save-persisted-state; CR-6/F1) so a
        // later Back to it restores scroll. App-mode's mirror of the content path.
        let off = interactive.pipeline.scroll_offset;
        interactive
            .nav_controller
            .set_current_scroll((f64::from(off.x), f64::from(off.y)));
        interactive.nav_controller.push_same_document(url.clone());
    }
    // Store the StructuredSerializeForStorage bytes on the just-committed entry
    // (§7.4.4 step 3) for a later cross-document traversal to restore.
    interactive
        .nav_controller
        .set_current_state(serialized_state);
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
