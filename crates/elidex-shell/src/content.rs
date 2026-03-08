//! Content thread: owns the DOM, JS runtime, and rendering pipeline.
//!
//! The content thread runs a message loop, processing events from the browser
//! thread and sending back display list updates.

use std::rc::Rc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;

use elidex_ecs::ElementState as DomElementState;
use elidex_ecs::Entity;
use elidex_layout::hit_test;
use elidex_navigation::NavigationController;
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit};
use elidex_script_session::DispatchEvent;

use crate::app::events::find_link_ancestor;
use crate::app::hover::{apply_hover_diff, collect_hover_chain, update_element_state};
use crate::app::navigation::resolve_nav_url;
use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel, ModifierState};
use crate::PipelineResult;

/// Default poll interval when no timers are pending.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// State owned by the content thread.
///
/// `hover_chain` and `active_chain` are bounded by [`elidex_ecs::MAX_ANCESTOR_DEPTH`]
/// (the depth limit enforced by [`collect_hover_chain`]).
struct ContentState {
    pipeline: PipelineResult,
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    nav_controller: NavigationController,
    hover_chain: Vec<Entity>,
    active_chain: Vec<Entity>,
    focus_target: Option<Entity>,
}

impl ContentState {
    /// Send the current display list to the browser thread.
    fn send_display_list(&self) {
        let _ = self.channel.send(ContentToBrowser::DisplayListReady(
            self.pipeline.display_list.clone(),
        ));
    }

    /// Send current navigation state (`can_go_back`/`can_go_forward`) to the browser thread.
    fn send_navigation_state(&self) {
        let _ = self.channel.send(ContentToBrowser::NavigationState {
            can_go_back: self.nav_controller.can_go_back(),
            can_go_forward: self.nav_controller.can_go_forward(),
        });
    }

    /// Send URL change notification to the browser thread.
    fn send_url_changed(&self, url: &url::Url) {
        let _ = self.channel.send(ContentToBrowser::UrlChanged(url.clone()));
    }

    /// Send all post-navigation notifications to the browser thread
    /// (title, URL, navigation state, display list).
    fn notify_navigation(&self, url: &url::Url) {
        let title = format!("elidex \u{2014} {url}");
        let _ = self.channel.send(ContentToBrowser::TitleChanged(title));
        self.send_url_changed(url);
        self.send_navigation_state();
        self.send_display_list();
    }

    /// Push or replace a URL in the navigation controller.
    fn push_or_replace(&mut self, url: url::Url, replace: bool) {
        if replace {
            self.nav_controller.replace(url);
        } else {
            self.nav_controller.push(url);
        }
    }

    /// Create a new `ContentState` from an initialized pipeline and channel.
    fn new(
        channel: LocalChannel<ContentToBrowser, BrowserToContent>,
        nav_controller: NavigationController,
        pipeline: PipelineResult,
    ) -> Self {
        Self {
            channel,
            nav_controller,
            hover_chain: Vec::new(),
            active_chain: Vec::new(),
            focus_target: None,
            pipeline,
        }
    }
}

/// Spawn the content thread with initial HTML/CSS.
///
/// Returns a `JoinHandle` for the thread. The thread will run until it
/// receives `Shutdown` or the channel disconnects.
pub(crate) fn spawn_content_thread(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    html: String,
    css: String,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main(channel, &html, &css);
    })
}

/// Spawn the content thread with a URL to load.
///
/// Returns a `JoinHandle` for the thread.
pub(crate) fn spawn_content_thread_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    url: url::Url,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        content_thread_main_url(channel, &url);
    })
}

fn content_thread_main(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    html: &str,
    css: &str,
) {
    let pipeline = crate::build_pipeline_interactive(html, css);
    let mut state = ContentState::new(channel, NavigationController::new(), pipeline);
    state.send_display_list();
    run_event_loop(&mut state);
}

fn content_thread_main_url(
    channel: LocalChannel<ContentToBrowser, BrowserToContent>,
    url: &url::Url,
) {
    let pipeline = match crate::build_pipeline_from_url(url) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Content thread: failed to load {url}: {e}");
            let _ = channel.send(ContentToBrowser::NavigationFailed {
                url: url.clone(),
                error: format!("{e}"),
            });
            return;
        }
    };

    let mut nav_controller = NavigationController::new();
    nav_controller.push(url.clone());

    let mut state = ContentState::new(channel, nav_controller, pipeline);
    state.notify_navigation(url);
    run_event_loop(&mut state);
}

fn run_event_loop(state: &mut ContentState) {
    loop {
        let timeout = state
            .pipeline
            .runtime
            .next_timer_deadline()
            .map_or(DEFAULT_POLL_INTERVAL, |d| {
                d.saturating_duration_since(Instant::now())
            });

        match state.channel.recv_timeout(timeout) {
            Ok(msg) => {
                if !handle_message(msg, state) {
                    break; // Shutdown
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // Only drain timers and re-render if timers are actually ready.
                if state
                    .pipeline
                    .runtime
                    .next_timer_deadline()
                    .is_some_and(|d| d <= Instant::now())
                {
                    state.pipeline.runtime.drain_timers(
                        &mut state.pipeline.session,
                        &mut state.pipeline.dom,
                        state.pipeline.document,
                    );
                    crate::re_render(&mut state.pipeline);
                    state.send_display_list();
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Handle a single message. Returns `false` for Shutdown.
fn handle_message(msg: BrowserToContent, state: &mut ContentState) -> bool {
    match msg {
        BrowserToContent::Shutdown => return false,

        BrowserToContent::Navigate(url) => {
            handle_navigate(state, &url, false);
        }

        BrowserToContent::MouseClick {
            x,
            y,
            client_x,
            client_y,
            button,
            mods,
        } => {
            handle_click(state, x, y, client_x, client_y, button, mods);
        }

        BrowserToContent::MouseRelease { button: _ } => {
            handle_mouse_release(state);
        }

        BrowserToContent::MouseMove { x, y, .. } => {
            handle_mouse_move(state, x, y);
        }

        BrowserToContent::CursorLeft => {
            handle_cursor_left(state);
        }

        BrowserToContent::KeyDown {
            key,
            code,
            repeat,
            mods,
        } => {
            handle_key(state, "keydown", key, code, repeat, mods);
        }

        BrowserToContent::KeyUp {
            key,
            code,
            repeat,
            mods,
        } => {
            handle_key(state, "keyup", key, code, repeat, mods);
        }

        BrowserToContent::SetViewport { .. } => {
            // TODO: update viewport dimensions and re-layout
        }

        BrowserToContent::GoBack => {
            if let Some(url) = state.nav_controller.go_back().cloned() {
                handle_navigate(state, &url, true);
            }
        }

        BrowserToContent::GoForward => {
            if let Some(url) = state.nav_controller.go_forward().cloned() {
                handle_navigate(state, &url, true);
            }
        }

        BrowserToContent::Reload => {
            if let Some(url) = state.pipeline.url.clone() {
                handle_navigate(state, &url, true);
            }
        }
    }
    true
}

/// Navigate to a URL, loading the document and updating state.
///
/// When `is_history_nav` is `true`, the URL is not pushed to the navigation
/// controller (it was already moved by `go_back`/`go_forward`).
fn handle_navigate(state: &mut ContentState, url: &url::Url, is_history_nav: bool) {
    let fetch_handle = Rc::clone(&state.pipeline.fetch_handle);
    let font_db = Rc::clone(&state.pipeline.font_db);

    match elidex_navigation::load_document(url, &fetch_handle) {
        Ok(loaded) => {
            let new_pipeline = crate::build_pipeline_from_loaded(loaded, fetch_handle, font_db);
            state.pipeline = new_pipeline;
            state.focus_target = None;
            state.hover_chain.clear();
            state.active_chain.clear();

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

#[allow(clippy::too_many_arguments)]
fn handle_click(
    state: &mut ContentState,
    x: f32,
    y: f32,
    client_x: f64,
    client_y: f64,
    button: u8,
    mods: ModifierState,
) {
    let Some(hit) = hit_test(&state.pipeline.dom, x, y) else {
        return;
    };
    let hit_entity = hit.entity;

    // Update focus.
    if state.focus_target != Some(hit_entity) {
        if let Some(old_focus) = state.focus_target {
            update_element_state(&mut state.pipeline.dom, old_focus, |s| {
                s.remove(DomElementState::FOCUS);
            });
        }
        update_element_state(&mut state.pipeline.dom, hit_entity, |s| {
            s.insert(DomElementState::FOCUS);
        });
        state.focus_target = Some(hit_entity);
    }

    // Set ACTIVE state on press. Per UI Events spec, :active applies from
    // mousedown to mouseup — cleared in handle_mouse_release().
    // Clear any stale ACTIVE from a previous press (e.g. MouseRelease lost
    // due to window focus change).
    for &e in &state.active_chain {
        update_element_state(&mut state.pipeline.dom, e, |s| {
            s.remove(DomElementState::ACTIVE);
        });
    }
    state.active_chain = state.hover_chain.clone();
    for &e in &state.active_chain {
        update_element_state(&mut state.pipeline.dom, e, |s| {
            s.insert(DomElementState::ACTIVE);
        });
    }

    // Use viewport-relative coordinates for DOM event properties (clientX/clientY).
    let mouse_init = MouseEventInit {
        client_x,
        client_y,
        button: i16::from(button),
        alt_key: mods.alt,
        ctrl_key: mods.ctrl,
        meta_key: mods.meta,
        shift_key: mods.shift,
        ..Default::default()
    };

    let event_types: &[&str] = if button == 0 {
        &["mousedown", "mouseup", "click"]
    } else {
        &["mousedown", "mouseup"]
    };

    let mut click_prevented = false;
    for event_type in event_types {
        let mut event = DispatchEvent::new_composed(*event_type, hit_entity);
        event.payload = EventPayload::Mouse(mouse_init.clone());
        let prevented = state.pipeline.runtime.dispatch_event(
            &mut event,
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );
        if *event_type == "click" {
            click_prevented = prevented;
        }
    }

    crate::re_render(&mut state.pipeline);

    if process_pending_actions(state) {
        return;
    }

    // Link navigation: if click was not prevented, check for <a href>.
    if button == 0 && !click_prevented {
        if let Some(href) = find_link_ancestor(&state.pipeline.dom, hit_entity) {
            let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &href);
            if let Some(target_url) = resolved {
                state.send_display_list();
                handle_navigate(state, &target_url, false);
                return;
            }
        }
    }

    state.send_display_list();
}

/// Handle mouse button release — clear `:active` state.
///
/// Per UI Events spec, `:active` applies from mousedown to mouseup.
fn handle_mouse_release(state: &mut ContentState) {
    if state.active_chain.is_empty() {
        return;
    }
    let active = std::mem::take(&mut state.active_chain);
    for &e in &active {
        update_element_state(&mut state.pipeline.dom, e, |s| {
            s.remove(DomElementState::ACTIVE);
        });
    }
    crate::re_render(&mut state.pipeline);
    state.send_display_list();
}

fn handle_mouse_move(state: &mut ContentState, x: f32, y: f32) {
    let new_chain = if y >= 0.0 {
        hit_test(&state.pipeline.dom, x, y)
            .map(|hit| collect_hover_chain(&state.pipeline.dom, hit.entity))
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if new_chain == state.hover_chain {
        return;
    }

    let old_chain = std::mem::take(&mut state.hover_chain);
    apply_hover_diff(&mut state.pipeline.dom, &old_chain, &new_chain);
    state.hover_chain = new_chain;

    crate::re_render(&mut state.pipeline);
    state.send_display_list();
}

fn handle_cursor_left(state: &mut ContentState) {
    let had_hover = !state.hover_chain.is_empty();
    let had_active = !state.active_chain.is_empty();

    for &e in &std::mem::take(&mut state.active_chain) {
        update_element_state(&mut state.pipeline.dom, e, |s| {
            s.remove(DomElementState::ACTIVE);
        });
    }
    for &e in &std::mem::take(&mut state.hover_chain) {
        update_element_state(&mut state.pipeline.dom, e, |s| {
            s.remove(DomElementState::HOVER);
            s.remove(DomElementState::ACTIVE);
        });
    }

    if had_hover || had_active {
        crate::re_render(&mut state.pipeline);
        state.send_display_list();
    }
}

fn handle_key(
    state: &mut ContentState,
    event_type: &str,
    key: String,
    code: String,
    repeat: bool,
    mods: ModifierState,
) {
    let Some(target) = state.focus_target else {
        return;
    };
    if !state.pipeline.dom.contains(target) {
        state.focus_target = None;
        return;
    }

    let init = KeyboardEventInit {
        key,
        code,
        repeat,
        alt_key: mods.alt,
        ctrl_key: mods.ctrl,
        meta_key: mods.meta,
        shift_key: mods.shift,
    };

    let mut event = DispatchEvent::new_composed(event_type, target);
    event.payload = EventPayload::Keyboard(init);

    let _default_prevented = state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );

    crate::re_render(&mut state.pipeline);

    if !process_pending_actions(state) {
        state.send_display_list();
    }
}

/// Process any pending JS navigation or history action after event dispatch.
///
/// Returns `true` if an action was processed (display list already sent).
fn process_pending_actions(state: &mut ContentState) -> bool {
    if let Some(nav_req) = state.pipeline.runtime.take_pending_navigation() {
        let resolved = resolve_nav_url(state.pipeline.url.as_ref(), &nav_req.url);
        if let Some(target_url) = resolved {
            state.send_display_list();
            handle_navigate(state, &target_url, false);
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

fn handle_history_action(state: &mut ContentState, action: &elidex_navigation::HistoryAction) {
    match action {
        elidex_navigation::HistoryAction::Back | elidex_navigation::HistoryAction::Forward => {
            let url = if matches!(action, elidex_navigation::HistoryAction::Back) {
                state.nav_controller.go_back().cloned()
            } else {
                state.nav_controller.go_forward().cloned()
            };
            if let Some(url) = url {
                handle_navigate(state, &url, true);
            }
        }
        elidex_navigation::HistoryAction::Go(delta) => {
            if let Some(url) = state.nav_controller.go(*delta).cloned() {
                handle_navigate(state, &url, true);
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
            .set_current_url(Some(resolved_url.clone()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{self, BrowserToContent, ContentToBrowser};
    use std::time::Duration;

    #[test]
    fn content_thread_startup_and_shutdown() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(
            content,
            "<div>Hello</div>".to_string(),
            "div { display: block; }".to_string(),
        );

        // Should receive initial DisplayListReady.
        let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

        // Send shutdown.
        browser.send(BrowserToContent::Shutdown).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn content_thread_mouse_move() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(
            content,
            "<div style=\"background-color: red; width: 200px; height: 100px;\">Test</div>"
                .to_string(),
            "div { display: block; }".to_string(),
        );

        // Drain initial display list.
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Send mouse move.
        browser
            .send(BrowserToContent::MouseMove {
                x: 50.0,
                y: 50.0,
                client_x: 50.0,
                client_y: 86.0,
            })
            .unwrap();

        // Should get a DisplayListReady (hover state change triggers re-render).
        let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

        browser.send(BrowserToContent::Shutdown).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn content_thread_click() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(
            content,
            "<div style=\"background-color: blue; width: 200px; height: 100px;\">Click</div>"
                .to_string(),
            "div { display: block; }".to_string(),
        );

        // Drain initial display list.
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Send click.
        browser
            .send(BrowserToContent::MouseClick {
                x: 50.0,
                y: 50.0,
                client_x: 50.0,
                client_y: 86.0,
                button: 0,
                mods: ModifierState::default(),
            })
            .unwrap();

        // Should get a DisplayListReady.
        let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

        browser.send(BrowserToContent::Shutdown).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn content_thread_mouse_release_clears_active() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(
            content,
            "<div style=\"background-color: blue; width: 200px; height: 100px;\">Active</div>"
                .to_string(),
            "div { display: block; }".to_string(),
        );

        // Drain initial display list.
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Move cursor to set hover chain.
        browser
            .send(BrowserToContent::MouseMove {
                x: 50.0,
                y: 50.0,
                client_x: 50.0,
                client_y: 86.0,
            })
            .unwrap();
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Click (sets ACTIVE).
        browser
            .send(BrowserToContent::MouseClick {
                x: 50.0,
                y: 50.0,
                client_x: 50.0,
                client_y: 86.0,
                button: 0,
                mods: ModifierState::default(),
            })
            .unwrap();
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Release (clears ACTIVE).
        browser
            .send(BrowserToContent::MouseRelease { button: 0 })
            .unwrap();
        let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

        browser.send(BrowserToContent::Shutdown).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn content_thread_disconnect() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(content, "<div>Hello</div>".to_string(), "".to_string());

        // Drain initial display list.
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Drop browser end — content thread should exit cleanly.
        drop(browser);
        handle.join().unwrap();
    }

    #[test]
    fn content_thread_with_script() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(
            content,
            "<div id=\"btn\" style=\"background-color: blue; width: 200px; height: 100px;\">Click</div>\
             <script>\
               document.getElementById('btn').addEventListener('click', function(e) {\
                 e.target.style.setProperty('background-color', 'red');\
               });\
             </script>".to_string(),
            "div { display: block; }".to_string(),
        );

        // Drain initial display list.
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Click on the element.
        browser
            .send(BrowserToContent::MouseClick {
                x: 50.0,
                y: 50.0,
                client_x: 50.0,
                client_y: 86.0,
                button: 0,
                mods: ModifierState::default(),
            })
            .unwrap();

        // Should get updated display list.
        let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

        browser.send(BrowserToContent::Shutdown).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn content_thread_keyboard() {
        let (browser, content) = ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let handle = spawn_content_thread(
            content,
            "<div id=\"box\" style=\"width: 100px; height: 100px;\">Key</div>\
             <script>\
               document.getElementById('box').addEventListener('keydown', function(e) {\
                 console.log('key=' + e.key);\
               });\
             </script>"
                .to_string(),
            "div { display: block; }".to_string(),
        );

        // Drain initial display list.
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Click first to set focus.
        browser
            .send(BrowserToContent::MouseClick {
                x: 50.0,
                y: 50.0,
                client_x: 50.0,
                client_y: 86.0,
                button: 0,
                mods: ModifierState::default(),
            })
            .unwrap();
        let _ = browser.recv_timeout(Duration::from_secs(5)).unwrap();

        // Send key down.
        browser
            .send(BrowserToContent::KeyDown {
                key: "a".to_string(),
                code: "KeyA".to_string(),
                repeat: false,
                mods: ModifierState::default(),
            })
            .unwrap();

        let msg = browser.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(msg, ContentToBrowser::DisplayListReady(_)));

        browser.send(BrowserToContent::Shutdown).unwrap();
        handle.join().unwrap();
    }
}
