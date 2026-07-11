//! Navigation and history action handling for the content thread.

use std::rc::Rc;
use std::sync::Arc;

use elidex_script_session::{HistoryStepEvents, HostDriver, NavigationType};

use crate::app::navigation::resolve_nav_url;
use crate::ipc::ContentToBrowser;

use super::ContentState;

/// How a [`handle_navigate`] load moves the session-history cursor — resolved
/// BEFORE `notify_navigation` (symmetric with the fresh-nav push) so the browser
/// chrome's `NavigationState` (`can_go_back`/`can_go_forward`, derived from the
/// controller's cursor) reflects the POST-move position. A JS traversal
/// previously committed the cursor in the *caller* AFTER `handle_navigate`
/// returned, so `notify_navigation` shipped a stale pre-move state (Codex R5).
#[derive(Clone, Copy)]
pub(super) enum HistoryCursorOp {
    /// Fresh navigation: push a new entry (cursor → the new last entry).
    Push,
    /// JS traversal (`back`/`forward`/`go`): commit the peeked target index — the
    /// atomic half of peek-then-commit, moved INSIDE `handle_navigate` (before
    /// `notify_navigation`) so the chrome sees the committed position.
    Commit(usize),
    /// Reload, or a chrome-button traversal that already moved the cursor eagerly
    /// (`go_back`/`go_forward`): no cursor change here.
    Keep,
}

/// Navigate to a URL, loading the document and updating state.
///
/// `cursor_op` selects how this load moves the session-history cursor (see
/// [`HistoryCursorOp`]) — applied in the `Ok` branch BEFORE `notify_navigation`,
/// so the chrome's `NavigationState` reflects the post-move cursor and a failed
/// load never moves it.
///
/// When `request` is `Some`, that request is sent instead of a default GET
/// (used for POST form submissions).
///
/// Returns `true` iff the step was **handled** — a cross-document load that
/// succeeded and replaced the pipeline (`Ok`, including a `go(0)` reload), OR a
/// same-document step applied in place (a fresh fragment nav, or a same-document
/// traversal that restored + fired popstate — no rebuild). Returns `false` only on
/// load failure (`Err` — a `NavigationFailed` is sent and `state.pipeline` is left
/// UNCHANGED, so the old document stays active). The history drain propagates this
/// so a failed traversal load does not supersede the current document (Codex R2).
#[allow(clippy::too_many_lines)]
pub(super) fn handle_navigate(
    state: &mut ContentState,
    url: &url::Url,
    cursor_op: HistoryCursorOp,
    request: Option<elidex_net::Request>,
) -> bool {
    // Capture the departing entry's scroll BEFORE any same-document gate or
    // rebuild reset (WHATWG HTML §7.4.6.1 *activate history entry* step 1 "save
    // persisted state to the navigable's active session history entry" is the
    // common chokepoint on leaving ANY entry — fresh nav, fragment, traversal, or
    // reload). The cursor has not moved yet (Commit/Push happen below), so this
    // writes the entry being left; `re_render`/rebuild reset `viewport_scroll`
    // after (CR-6/DR-4). No-op-safe on a failed load (writes the current scroll to
    // the still-current entry).
    capture_scroll_on_leave(state);

    // The no-rebuild same-document path, split by `cursor_op`:
    //   - `Push` (a FRESH navigation) → the URL-based fragment classifier
    //     (`classify_navigation`, navigate step 15): a `/a` → `/a#x` fragment nav.
    //     `request.is_none()` = `documentResource is null` (a POST body ⇒ rebuild).
    //   - `Commit` (a TRAVERSAL) → the DOCUMENT-IDENTITY classifier
    //     (`resolve_traversal`, §7.4.6.1 step 14.10 — NOT URL): a same-document
    //     traversal (incl. `pushState` routing across different URLs) restores
    //     state + scroll and fires popstate in place; a cross-document traversal
    //     OR a `go(0)` reload falls through to the rebuild + seed.
    //   - `Keep` (a reload) → always rebuilds.
    // A same-document step does NO fetch, so it never reaches the SW check below.
    if request.is_none() {
        match cursor_op {
            HistoryCursorOp::Push => {
                if let Some(current) = state.pipeline.url.clone() {
                    if elidex_navigation::classify_navigation(&current, url)
                        == elidex_navigation::NavClass::SameDocument
                    {
                        return same_document_step(state, &current, url, SameDocStep::FragmentNav);
                    }
                }
            }
            HistoryCursorOp::Commit(target_index) => {
                if let elidex_navigation::TraversalKind::SameDocument {
                    state: popstate_state,
                    scroll: scroll_position,
                } = state.nav_controller.resolve_traversal(target_index)
                {
                    if let Some(current) = state.pipeline.url.clone() {
                        return same_document_step(
                            state,
                            &current,
                            url,
                            SameDocStep::Traversal {
                                target_index,
                                popstate_state,
                                scroll_position,
                            },
                        );
                    }
                }
            }
            HistoryCursorOp::Keep => {}
        }
    }

    // WHATWG SW Handle Fetch — a rebuilding navigation consults the controlling
    // service worker. A fragment nav early-returned above; the other skip cases
    // (embed/object destination — §1; shift+reload) are handled by the browser
    // thread for subresource requests.
    if let Some(sw_scope) = state.pipeline.runtime.sw_controller_scope() {
        if elidex_api_sw::matches_scope(&sw_scope, url) {
            // Send FetchEvent relay request to browser thread.
            static FETCH_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
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

            let client_id = state.client_id.clone();
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

    let network_handle = Rc::clone(&state.pipeline.network_handle);
    let font_db = Arc::clone(&state.pipeline.font_db);

    match elidex_navigation::load_document(url, &network_handle, request) {
        Ok(loaded) => {
            // Shut down WebSocket/SSE connections before replacing the pipeline.
            state.pipeline.runtime.bridge().shutdown_all_realtime();
            // Preserve cookie jar across navigations.
            let cookie_jar = state.pipeline.cookie_jar.clone();
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
            // A rebuild restores the target entry's `history.state` in the rebuilt
            // document (§7.4.6.1 restore state, BEFORE "scripts may run" — J5)
            // WITHOUT firing popstate (`documentIsNew=true` — J6): a restore-only
            // seed, NOT a `deliver_history_step_events` call.
            //   - `Commit` (a cross-document traversal — the same-document case
            //     early-returned) reads the PEEKED TARGET entry (`entry(target_index)`),
            //     NEVER `current()`: the cursor commits (`commit_index`, below) only
            //     AFTER this rebuild, so `current()` still points at the departing
            //     document (DR-1).
            //   - `Keep` (a reload) re-seeds the CURRENT entry's state (a reload
            //     restores the entry's classic state — the go(0) reload's sibling).
            //   - `Push` (a fresh navigation) has no prior state.
            // Flip-inert value: boa passes `None` on every pushState, so the entry
            // stores `None` on the boa-live path; only the VM path carries a real
            // value (live at S5-6).
            let history_seed = match cursor_op {
                HistoryCursorOp::Commit(target_index) => state
                    .nav_controller
                    .entry(target_index)
                    .and_then(|e| e.classic_history_api_state.clone()),
                HistoryCursorOp::Keep => state.nav_controller.current_serialized_state(),
                HistoryCursorOp::Push => None,
            };
            // Persisted scroll to restore AFTER the rebuild is laid out (§7.4.6.1
            // "restore persisted state"; scrollRestoration=auto). `capture_scroll_on_leave`
            // stored it on the departing entry; the return trip reapplies it — for a
            // cross-document Commit traversal (the PEEKED TARGET entry — DR-1) and a
            // reload (the current entry). A fresh navigation (`Push`) lands at the top
            // (a new entry, no captured offset). Without this, a cross-document Back
            // rebuilds at the top even though the offset was captured (F3).
            let restored_scroll = match cursor_op {
                HistoryCursorOp::Commit(target_index) => state
                    .nav_controller
                    .entry(target_index)
                    .and_then(|e| e.scroll_position),
                HistoryCursorOp::Keep => state.nav_controller.current_scroll_position(),
                HistoryCursorOp::Push => None,
            };
            let new_pipeline = crate::build_pipeline_from_loaded(
                loaded,
                network_handle,
                font_db,
                cookie_jar,
                viewport,
                snapshot.facts,
                // Top-level document: no frame security (URL-derived origin).
                None,
                history_seed,
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

            // Move the session-history cursor BEFORE `notify_navigation` below,
            // so the `NavigationState` it ships reads the post-move
            // `can_go_back`/`can_go_forward` (Codex R5). A JS traversal commits
            // here (the peeked target), symmetric with the fresh-nav push — the
            // caller no longer commits after we return.
            match cursor_op {
                HistoryCursorOp::Push => state.nav_controller.push(url.clone()),
                // A `Commit` reaching this rebuild path is a CROSS-document traversal
                // (the same-document case early-returned via `same_document_step`) OR
                // a `go(0)` reload — either way the target entry was just rebuilt as
                // a FRESH document, so commit the cursor AND re-stamp its identity.
                // Without the re-stamp the rebuilt target keeps the
                // `document_sequence` it shared with its former pushState/fragment
                // siblings, so a later traversal to such a sibling mis-classifies
                // same-document and skips the required rebuild (stale document under
                // a swapped URL).
                HistoryCursorOp::Commit(index) => {
                    state.nav_controller.commit_index(index);
                    state.nav_controller.restamp_current_document();
                }
                // `Keep` uniquely means a reload (chrome Back/Forward routes through
                // `Commit` — see `event_loop.rs`): a reload replaces the navigable's
                // *document*, so re-stamp the current entry's document identity —
                // else a neighbor entry that shared its pre-reload `document_sequence`
                // would mis-classify same-document on a later traversal (§7.4.6.1
                // reload populates a new Document).
                HistoryCursorOp::Keep => state.nav_controller.restamp_current_document(),
            }
            state.pipeline.runtime.set_current_url(Some(url.clone()));
            state.pipeline.runtime.set_session_history(
                state.nav_controller.current_index(),
                state.nav_controller.len(),
            );
            // Restore the persisted scroll onto the freshly-laid-out document, then
            // `re_render` clamps it against the rebuilt content size + echoes
            // `scrollX`/`scrollY`, so the display list `notify_navigation` ships is
            // already scrolled (not an un-applied 0 offset). Only for a traversal /
            // reload with a captured offset (F3).
            if let Some((x, y)) = restored_scroll {
                state.viewport_scroll.scroll_offset =
                    super::scroll::scroll_offset_from_position((x, y));
                state.re_render();
            }
            state.notify_navigation(url);
            true
        }
        Err(e) => {
            eprintln!("Content thread navigation error: {e}");
            let _ = state.channel.send(ContentToBrowser::NavigationFailed {
                url: url.clone(),
                error: format!("{e}"),
            });
            false
        }
    }
}

/// Which same-document history-step application [`same_document_step`] is applying
/// — the two consumers of the shared no-rebuild primitive. Each parameterizes the
/// three facets that differ between a fresh fragment navigation (WHATWG HTML
/// §7.4.2.3.3) and a same-document traversal (§7.4.6.2 step 6.4): the
/// session-history cursor move, the popstate `history.state`, and the scroll
/// source. One primitive, incrementally parameterized — NOT a fork
/// (One-issue-one-way).
enum SameDocStep {
    /// A fresh fragment navigation: push the entry (or replace it for a URL equal
    /// to the active entry's, §7.4.2.2 step 13); popstate `state = null`
    /// (§7.4.2.3.3 step 11.1); scroll resolved from the target fragment against
    /// the live (un-rebuilt) layout (§7.4.6.4).
    FragmentNav,
    /// A same-document traversal (`back`/`forward`/`go`): commit the cursor to the
    /// peeked target index; popstate `state` = the target entry's serialized state
    /// (§7.4.6.2 step 6.3 → 6.4.3, the general form — a `None`-state entry ⇒
    /// popstate fires with `null`); scroll = the target entry's persisted
    /// `scroll_position` (step 6.4.4 "restore persisted state").
    Traversal {
        target_index: usize,
        popstate_state: Option<Vec<u8>>,
        scroll_position: Option<(f64, f64)>,
    },
}

/// Apply a same-document history-step IN PLACE (no pipeline rebuild) — the shared
/// primitive [`handle_navigate`] early-returns into for BOTH a fresh fragment
/// navigation (WHATWG HTML §7.4.2.3.3) and a same-document traversal (§7.4.6.2),
/// selected by `step`. `current` is the pre-step document URL, `target` the
/// destination URL.
///
/// Replicates the normal `Push` path's post-nav bookkeeping MINUS the pipeline
/// rebuild, so the existing document and its `EcsDom` — including
/// `ElementState::FOCUS` — persist (the focus-persist fix; no ad-hoc focus reset).
/// The document origin stays correct BY CONSTRUCTION: `set_current_url` re-derives
/// the same URL-tuple origin (only the fragment/entry changed) and never touches
/// an installed opaque/sandbox override, so `fetch` / `new WebSocket()` keep
/// keying on the unchanged origin. Returns `true` — a same-document step is
/// handled in place (mirrors `handle_navigate`'s success return).
fn same_document_step(
    state: &mut ContentState,
    current: &url::Url,
    target: &url::Url,
    step: SameDocStep,
) -> bool {
    // Update the current document URL: the shell copy (the relative-URL base for
    // the next navigation) + the runtime's `current_url` (`location.*` /
    // `document.URL`). No `load_document`, no pipeline rebuild.
    state.pipeline.url = Some(target.clone());
    state.pipeline.runtime.set_current_url(Some(target.clone()));
    // Move the session-history cursor. A FRESH fragment nav to the URL the active
    // entry ALREADY has (including the fragment) REPLACES it — §7.4.2.2 step 13
    // resolves `historyHandling` to "replace" for an equal URL, so
    // `location.href = location.href` / re-clicking the current `#id` does not
    // grow `history.length`; a changed/added fragment pushes. A TRAVERSAL commits
    // the already-existing peeked target entry (`commit_index`) — no push/replace.
    match &step {
        SameDocStep::FragmentNav => {
            // A fragment navigation stays in the CURRENT document, so it inherits
            // its `document_sequence` (same-document push/replace) — a later
            // traversal between it and its document-siblings restores in place.
            if current == target {
                state.nav_controller.replace_same_document(target.clone());
            } else {
                state.nav_controller.push_same_document(target.clone());
            }
            // A fragment navigation sets `history.state` to null (§7.4.2.3.3 step
            // 11.1 — the popstate below fires `Some(None)`), so the entry must carry
            // NO classic state. A push already reset it, but a replace (a same-URL
            // re-nav) would keep the prior `pushState`'d entry's state — clear it so
            // a later reload/traversal restores null, not the stale value (F4).
            state.nav_controller.set_current_state(None);
        }
        SameDocStep::Traversal { target_index, .. } => {
            state.nav_controller.commit_index(*target_index);
        }
    }
    state.pipeline.runtime.set_session_history(
        state.nav_controller.current_index(),
        state.nav_controller.len(),
    );
    // Resolve the scroll offset BEFORE firing popstate (below) so it reads live
    // geometry unaffected by a popstate handler; the existing document's layout is
    // current (no rebuild). Applied AFTER popstate via the post-layout `re_render`
    // seam. A FRAGMENT nav resolves the offset from the target fragment against
    // the current layout (§7.4.6.4); a TRAVERSAL restores the target entry's
    // persisted `scroll_position` (§7.4.6.2 step 6.4.4).
    let offset = match &step {
        SameDocStep::FragmentNav => super::scroll::scroll_offset_for_fragment(
            &state.pipeline.dom,
            state.pipeline.document,
            target.fragment().unwrap_or_default(),
            state.viewport_scroll.scroll_offset,
            state.viewport_scroll.client_size.width,
        ),
        SameDocStep::Traversal {
            scroll_position, ..
        } => scroll_position.map(super::scroll::scroll_offset_from_position),
    };
    // §7.4.6.2 step 6.3 + 6.4.3 (also §7.4.2.3.3 step 14): restore `history.state`
    // then fire popstate SYNCHRONOUSLY, BEFORE the scroll — a synchronous popstate
    // handler must observe the PRE-scroll scroll position (`window.scrollY`).
    // popstate is state-AGNOSTIC (fires whenever the entry changed, §4.5): a
    // fragment nav carries `null` (`Some(None)`), a traversal the target entry's
    // serialized state (`Some(Some(bytes))` → `StructuredDeserialize`,
    // `Some(None)` → `null`). hashchange is a queued task that fires AFTER the
    // scroll (below). The VM fires (popstate SYNC); boa stubs it — flip-inert
    // until S5-6.
    // Consume `step` here (moving the serialized state out — no clone).
    let popstate_state = match step {
        SameDocStep::FragmentNav => None,
        SameDocStep::Traversal { popstate_state, .. } => popstate_state,
    };
    state
        .pipeline
        .deliver_history_step_events(HistoryStepEvents {
            popstate_state: Some(popstate_state),
            hashchange: None,
        });
    // scroll (§7.4.6.2 step 6.4.4 / §7.4.2.3.3 step 15) through the post-layout
    // `re_render` seam — set the resolved offset on `viewport_scroll`, then
    // `re_render` applies + clamps it against the content size, echoes
    // `scrollX`/`scrollY` + the document-root `ScrollState`, flushes any popstate
    // handler's DOM mutations, and rebuilds the display list — NOT an inline set +
    // `send_display_list` (which would ship the offset un-applied).
    if let Some(offset) = offset {
        state.viewport_scroll.scroll_offset = offset;
    }
    state.re_render();
    // Ship the scrolled frame + the new title / URL / nav-state (mirrors the
    // normal `Push` path's `notify_navigation`, MINUS the rebuild).
    state.notify_navigation(target);
    // §7.4.6.2 step 6.4.5: the hashchange task, queued at the fire above, runs as
    // a LATER task — after the synchronous scroll — iff the fragment differs.
    // Delivering it in a second call keeps the spec order popstate → scroll →
    // hashchange (and popstate strictly-before-hashchange).
    if let Some(hashchange) =
        (current.fragment() != target.fragment()).then(|| (current.to_string(), target.to_string()))
    {
        state
            .pipeline
            .deliver_history_step_events(HistoryStepEvents {
                popstate_state: None,
                hashchange: Some(hashchange),
            });
    }
    true
}

/// Capture the current viewport scroll offset onto the entry being LEFT, BEFORE a
/// traversal moves the cursor or rebuilds (WHATWG HTML §7.4.6.2 step 6.4.4
/// "restore persisted state" reads it on the return trip). Called in
/// `handle_history_action` BEFORE `handle_navigate` (DR-4), whose cross-document
/// rebuild resets `viewport_scroll` to `(0,0)` (the per-pipeline reset, ABOVE the
/// `commit_index`) — so a capture placed after that reset would persist the top of
/// the page. `set_current_scroll` writes the CURRENT entry: the cursor has not
/// moved (peek does not commit). Auto mode only (the `Manual`-mode suppression is
/// `#11-history-scroll-restoration-manual-mode`).
fn capture_scroll_on_leave(state: &mut ContentState) {
    let offset = state.viewport_scroll.scroll_offset;
    state
        .nav_controller
        .set_current_scroll((f64::from(offset.x), f64::from(offset.y)));
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
            // A traversal (`back`/`forward`/`go`) just handled the turn — either a
            // CROSS-document rebuild (`state.pipeline` replaced, a fresh document
            // loaded) OR a SAME-document traversal applied in place (state + scroll
            // restored, popstate fired). Either way `handle_navigate`'s
            // `notify_navigation` already shipped its display list. Return
            // IMMEDIATELY: do NOT fall through to `take_pending_navigation()`
            // below. On the rebuild path that would read the FRESH runtime and
            // drain a `location.*` the newly-loaded page's initial scripts queued
            // (`build_pipeline_from_loaded` runs them before returning) — the
            // traversal must win, not the fresh page's own nav (Codex #283, this
            // slice's own regression: the 5a reorder created a fall-through onto
            // the freshly-rebuilt runtime). Remaining same-turn history intents,
            // captured from the now-superseded (navigated-away) document, are
            // discarded with it. A no-target (out-of-range) / failed-load traversal
            // returns `false` and does NOT reach here, so the loop CONTINUES and
            // trailing intents still apply (Codex R1 P2 / R2). The single-navigable
            // "traversal = a later task" boundary the spec models (§7.4.3 queued
            // task) is collapsed onto this synchronous return; a faithful
            // task-queued model — including the same-document-traversal +
            // same-turn-`pushState` refinement — is the reframed deferred slot
            // `#11-session-history-task-queue-model` (memo §8-D5).
            return true;
        }
    }

    // Own-context navigation (may replace the pipeline) — AFTER the history
    // above, and reached ONLY when NO same-turn traversal superseded the document
    // (a superseding traversal `return true`s from the loop above). So this
    // `take_pending_navigation()` always reads the ORIGINAL (non-superseded)
    // runtime — the pending navigation THIS document's script queued, never a
    // freshly-loaded page's. For a NON-rebuild history action
    // (pushState/replaceState — render nothing, do not touch the pipeline), the
    // entries just committed live on the `NavigationController` (owned by
    // `ContentState`, not the replaced pipeline), so they survive this
    // navigation's rebuild; and the navigation ships its own display list — so the
    // history drain intentionally does NOT send one when a navigation follows (no
    // redundant double-send). This is the pushState+nav common case 5a targets.
    if let Some(nav_req) = state.pipeline.runtime.take_pending_navigation() {
        let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &nav_req.url);
        if let Some(target_url) = resolved {
            state.send_display_list();
            // Map the nav-type to a history-cursor effect. `Reload` → `Keep`
            // (rebuild, NO cursor advance) — fixes `location.reload()` pushing a
            // spurious history entry (it previously hardcoded `Push`), and is the
            // guard Phase 2b's fragment branch uses to exclude a fragment-URL
            // reload from the no-rebuild path (`cursor_op == Push`). `Push`/
            // `Replace` → `Push`: thread-mode still collapses `Replace` → `Push`
            // for the cursor op (a pre-existing drop, deferred §10-D6 — the enum
            // only CONVEYS the distinction here).
            let cursor_op = match nav_req.nav_type {
                NavigationType::Reload => HistoryCursorOp::Keep,
                NavigationType::Push | NavigationType::Replace => HistoryCursorOp::Push,
            };
            handle_navigate(state, &target_url, cursor_op, None);
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

/// Apply a single history action. Returns `true` iff it **handled the turn as an
/// own-context traversal** — a `Back`/`Forward`/`Go` whose `NavigationController`
/// peek yielded a target AND whose `handle_navigate` (with
/// [`HistoryCursorOp::Commit`]) either (a) **rebuilt** the document on a
/// cross-document load that succeeded (replaced `state.pipeline`) OR (b) applied a
/// **same-document traversal IN PLACE** (§7.4.6.1 — no rebuild: restored state +
/// scroll and fired popstate, shipping its own display list). A `go(0)` is a
/// **reload** (History.go step 4) → the rebuild arm (also `true`). Returns `false`
/// for `PushState`/`ReplaceState` (no rebuild), for a **no-target traversal** (an
/// out-of-range `go` / empty `peek_back`/`peek_forward` returning `None` → no
/// `handle_navigate`), AND for a cross-document traversal whose **load FAILED**
/// (`handle_navigate` `Err` left the pipeline unchanged, so the old document is
/// still active). The FIFO drain loop keys on
/// this to STOP replaying remaining same-turn intents ONLY once a traversal
/// genuinely superseded the document (Codex R1 P2 / R2): a no-op or failed-load
/// traversal leaves the current document active, so the loop CONTINUES and the
/// trailing intents still apply. The traversal is **atomic on the non-reentrant
/// path** (peek-then-commit — Codex R3): the target index is peeked WITHOUT moving
/// the cursor and committed (`commit_index`, threaded INTO `handle_navigate` via
/// [`HistoryCursorOp::Commit`] so it precedes `notify_navigation` — Codex R5)
/// ONLY after the load succeeds, so a failed load never leaves the cursor
/// speculatively moved (no rollback path — a continuing trailing `pushState`
/// commits from the correct, unmoved index). **One reentrancy vector is out of
/// scope**: `handle_navigate`'s SW-fetch synchronous message pump can re-dispatch
/// a nav-mutating message during its blocking wait, staling the held `target_index`
/// before the commit — folded into the deferred `#11-session-history-task-queue-model`
/// (the task-queued model + M4-10 async event loop remove the synchronous
/// cross-wait window; unreachable today — the SW controller path is dead — and
/// `commit_index`'s `debug_assert` backstops the out-of-range case in debug/test).
pub(super) fn handle_history_action(
    state: &mut ContentState,
    action: &elidex_script_session::HistoryAction,
) -> bool {
    match action {
        elidex_script_session::HistoryAction::Back
        | elidex_script_session::HistoryAction::Forward => {
            // Peek the traversal target WITHOUT moving the cursor; `handle_navigate`
            // commits the move (via `HistoryCursorOp::Commit`) ONLY if the load
            // succeeds — an atomic traversal (Codex R3), with the commit threaded
            // into `handle_navigate` before its `notify_navigation` (Codex R5). A
            // failed load leaves the cursor on the still-active document, so a
            // trailing same-turn `pushState` commits from the correct index (no
            // rollback needed — the cursor never moved speculatively). Clone the
            // URL to drop the `nav_controller` borrow before the `&mut state` load.
            let peeked = if matches!(action, elidex_script_session::HistoryAction::Back) {
                state.nav_controller.peek_back()
            } else {
                state.nav_controller.peek_forward()
            };
            let Some((target_index, url)) = peeked.map(|(i, u)| (i, u.clone())) else {
                return false;
            };
            // Scroll capture-on-leave happens at the top of `handle_navigate` (it
            // covers every leave — fresh nav, fragment, traversal, reload — CR-6).
            // The commit lives INSIDE `handle_navigate` (its `Ok` branch, before
            // `notify_navigation`) via `HistoryCursorOp::Commit`, so the chrome's
            // `NavigationState` reads the post-move cursor and a failed load never
            // moves it — atomic peek-then-commit at the navigate seam (Codex R5).
            handle_navigate(state, &url, HistoryCursorOp::Commit(target_index), None)
        }
        elidex_script_session::HistoryAction::Go(delta) => {
            let Some((target_index, url)) = state
                .nav_controller
                .peek_go(*delta)
                .map(|(i, u)| (i, u.clone()))
            else {
                return false;
            };
            handle_navigate(state, &url, HistoryCursorOp::Commit(target_index), None)
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
            apply_push_replace_state(state, url.as_deref(), replace, serialized_state.clone());
            false
        }
    }
}

/// Apply a `pushState`/`replaceState` history action.
///
/// Resolves the URL (if any), enforces same-origin, updates the pipeline URL,
/// navigation controller, and notifies the browser thread.
fn apply_push_replace_state(
    state: &mut ContentState,
    url_str: Option<&str>,
    replace: bool,
    serialized_state: Option<Vec<u8>>,
) {
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
        // A `pushState` (non-replace) pushes a NEW entry, leaving the current one —
        // capture its scroll first (WHATWG HTML §7.4.6.1 save-persisted-state on
        // leaving any entry), else a later Back to it restores no scroll. This drain
        // path bypasses `handle_navigate`'s capture, so it needs its own (CR-6/F1).
        if !replace {
            capture_scroll_on_leave(state);
        }
        state.push_or_replace(resolved_url.clone(), replace);
        // Store the StructuredSerializeForStorage bytes on the just-committed
        // entry (§7.4.4 step 3) so a later cross-document traversal restores it.
        state.nav_controller.set_current_state(serialized_state);
        state
            .pipeline
            .runtime
            .set_current_url(state.pipeline.url.clone());
        state.pipeline.runtime.set_session_history(
            state.nav_controller.current_index(),
            state.nav_controller.len(),
        );

        let title = format!("elidex \u{2014} {resolved_url}");
        state.send_title(title);
        state.send_url_changed(&resolved_url);
        state.send_navigation_state();
    } else {
        // No URL change — just update history.
        let Some(current) = state.pipeline.url.clone() else {
            return;
        };
        if !replace {
            capture_scroll_on_leave(state);
        }
        state.push_or_replace(current, replace);
        state.nav_controller.set_current_state(serialized_state);
        state.pipeline.runtime.set_session_history(
            state.nav_controller.current_index(),
            state.nav_controller.len(),
        );
        state.send_navigation_state();
    }
}
