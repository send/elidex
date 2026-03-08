//! winit application handler for the elidex browser shell.
//!
//! Implements [`ApplicationHandler`] to manage the window lifecycle,
//! GPU initialization, frame rendering via Vello, and user input
//! event dispatch to the DOM.
//!
//! Supports two modes:
//! - **Threaded** (`ContentHandle`): content runs on a dedicated thread,
//!   communicating via message passing.
//! - **Legacy inline** (`InteractiveState`): all processing on the main
//!   thread (used by `build_pipeline` test API).

pub(crate) mod events;
pub(crate) mod hover;
pub(crate) mod navigation;
mod render;

use std::sync::Arc;
use std::thread::JoinHandle;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Modifiers, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use elidex_ecs::{ElementState as DomElementState, Entity};
use elidex_navigation::NavigationController;
use elidex_render::DisplayList;
use wgpu::util::TextureBlitter;
use wgpu::{Instance, Surface};

use crate::ipc::{BrowserToContent, ContentToBrowser, LocalChannel, ModifierState};

use render::{handle_redraw, try_init_render_state};

/// Convert a winit mouse button to the DOM spec button number.
///
/// DOM spec: 0=primary, 1=auxiliary, 2=secondary, 3=back, 4=forward.
pub(crate) fn winit_button_to_dom(button: winit::event::MouseButton) -> u8 {
    match button {
        winit::event::MouseButton::Middle => 1,
        winit::event::MouseButton::Right => 2,
        winit::event::MouseButton::Back => 3,
        winit::event::MouseButton::Forward => 4,
        winit::event::MouseButton::Left | winit::event::MouseButton::Other(_) => 0,
    }
}

/// Render state initialized after the window is created.
pub(super) struct RenderState {
    pub(super) window: Arc<Window>,
    /// Kept alive as a precaution. While wgpu 27's `Surface` does not
    /// reference the `Instance` directly, keeping it alive ensures
    /// correctness if future wgpu versions change this.
    pub(super) _instance: Instance,
    pub(super) surface: Surface<'static>,
    pub(super) gpu: crate::gpu::GpuContext,
    pub(super) renderer: elidex_render::VelloRenderer,
    pub(super) blitter: TextureBlitter,
    pub(super) egui_ctx: egui::Context,
    pub(super) egui_state: egui_winit::State,
    pub(super) egui_renderer: egui_wgpu::Renderer,
    pub(super) a11y_adapter: accesskit_winit::Adapter,
}

/// Handle to a content thread, used by the browser thread for message passing.
///
/// Navigation state is owned by the content thread. The browser thread
/// mirrors `can_go_back`/`can_go_forward` for chrome UI via
/// [`ContentToBrowser::NavigationState`] messages.
pub(super) struct ContentHandle {
    pub(super) channel: LocalChannel<BrowserToContent, ContentToBrowser>,
    pub(super) thread: JoinHandle<()>,
    pub(super) can_go_back: bool,
    pub(super) can_go_forward: bool,
    pub(super) chrome: crate::chrome::ChromeState,
    pub(super) cursor_pos: Option<(f64, f64)>,
    pub(super) modifiers: Modifiers,
    pub(super) window_title: String,
}

impl ContentHandle {
    fn new(
        channel: LocalChannel<BrowserToContent, ContentToBrowser>,
        thread: JoinHandle<()>,
        chrome: crate::chrome::ChromeState,
        window_title: String,
    ) -> Self {
        Self {
            channel,
            thread,
            can_go_back: false,
            can_go_forward: false,
            chrome,
            cursor_pos: None,
            modifiers: Modifiers::default(),
            window_title,
        }
    }
}

/// Legacy inline interactive state (all processing on the main thread).
///
/// Kept for backward compatibility with `build_pipeline()` test API.
pub(super) struct InteractiveState {
    pub(super) pipeline: crate::PipelineResult,
    pub(super) cursor_pos: Option<(f64, f64)>,
    pub(super) focus_target: Option<Entity>,
    pub(super) hover_chain: Vec<Entity>,
    pub(super) active_chain: Vec<Entity>,
    pub(super) modifiers: Modifiers,
    pub(super) nav_controller: NavigationController,
    pub(super) window_title: String,
    pub(super) chrome: crate::chrome::ChromeState,
}

/// winit application that renders a display list to a window.
pub struct App {
    pub(super) display_list: DisplayList,
    pub(super) render_state: Option<RenderState>,
    /// Threaded content handle (new architecture).
    pub(super) content_handle: Option<ContentHandle>,
    /// Legacy inline interactive state.
    pub(super) interactive: Option<InteractiveState>,
}

impl App {
    /// Synchronize the top-level display list from the interactive pipeline (legacy mode).
    fn sync_display_list(&mut self) {
        if let Some(interactive) = &self.interactive {
            self.display_list = interactive.pipeline.display_list.clone();
        }
    }

    /// Create a new threaded application from HTML/CSS.
    pub fn new_threaded(html: String, css: String) -> Self {
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let thread = crate::content::spawn_content_thread(content_ch, html, css);

        Self {
            display_list: DisplayList::default(),
            render_state: None,
            content_handle: Some(ContentHandle::new(
                browser_ch,
                thread,
                crate::chrome::ChromeState::new(None),
                "elidex".to_string(),
            )),
            interactive: None,
        }
    }

    /// Create a new threaded application from a URL.
    pub fn new_threaded_url(url: url::Url) -> Self {
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let title = format!("elidex \u{2014} {url}");
        let chrome = crate::chrome::ChromeState::new(Some(&url));
        let thread = crate::content::spawn_content_thread_url(content_ch, url);

        Self {
            display_list: DisplayList::default(),
            render_state: None,
            content_handle: Some(ContentHandle::new(browser_ch, thread, chrome, title)),
            interactive: None,
        }
    }

    /// Create a new legacy (inline) interactive application from a pipeline result.
    #[allow(dead_code)]
    pub fn new_interactive(pipeline: crate::PipelineResult) -> Self {
        let display_list = pipeline.display_list.clone();
        Self {
            display_list,
            render_state: None,
            content_handle: None,
            interactive: Some(InteractiveState {
                chrome: crate::chrome::ChromeState::new(None),
                pipeline,
                cursor_pos: None,
                focus_target: None,
                hover_chain: Vec::new(),
                active_chain: Vec::new(),
                modifiers: Modifiers::default(),
                nav_controller: NavigationController::new(),
                window_title: "elidex".to_string(),
            }),
        }
    }

    /// Create a new legacy (inline) interactive application from a URL-loaded pipeline result.
    #[allow(dead_code)]
    pub fn new_interactive_with_url(pipeline: crate::PipelineResult, title: String) -> Self {
        let display_list = pipeline.display_list.clone();
        let chrome = crate::chrome::ChromeState::new(pipeline.url.as_ref());
        let mut nav_controller = NavigationController::new();
        if let Some(url) = &pipeline.url {
            nav_controller.push(url.clone());
        }
        Self {
            display_list,
            render_state: None,
            content_handle: None,
            interactive: Some(InteractiveState {
                chrome,
                pipeline,
                cursor_pos: None,
                focus_target: None,
                hover_chain: Vec::new(),
                active_chain: Vec::new(),
                modifiers: Modifiers::default(),
                nav_controller,
                window_title: title,
            }),
        }
    }

    /// Drain all pending messages from the content thread.
    fn drain_content_messages(&mut self) {
        let Some(ch) = &mut self.content_handle else {
            return;
        };
        while let Ok(msg) = ch.channel.try_recv() {
            match msg {
                ContentToBrowser::DisplayListReady(dl) => {
                    self.display_list = dl;
                }
                ContentToBrowser::TitleChanged(title) => {
                    ch.window_title = title;
                    if let Some(state) = &self.render_state {
                        state.window.set_title(&ch.window_title);
                    }
                }
                ContentToBrowser::NavigationState {
                    can_go_back,
                    can_go_forward,
                } => {
                    ch.can_go_back = can_go_back;
                    ch.can_go_forward = can_go_forward;
                }
                ContentToBrowser::UrlChanged(url) => {
                    ch.chrome.set_url(&url);
                }
                ContentToBrowser::NavigationFailed { url, error } => {
                    eprintln!("Navigation to {url} failed: {error}");
                }
            }
        }
    }

    /// Convert winit modifier state to IPC `ModifierState`.
    fn to_modifier_state(mods: winit::keyboard::ModifiersState) -> ModifierState {
        ModifierState {
            alt: mods.alt_key(),
            ctrl: mods.control_key(),
            meta: mods.super_key(),
            shift: mods.shift_key(),
        }
    }

    /// Send a message to the content thread (if in threaded mode).
    fn send_to_content(&self, msg: BrowserToContent) {
        if let Some(ch) = &self.content_handle {
            if let Err(e) = ch.channel.send(msg) {
                eprintln!("Failed to send to content thread (disconnected): {e}");
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(ch) = self.content_handle.take() {
            let _ = ch.channel.send(BrowserToContent::Shutdown);
            if let Err(e) = ch.thread.join() {
                eprintln!("Content thread panicked: {e:?}");
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_state.is_some() {
            return;
        }

        let Some(state) = try_init_render_state(event_loop) else {
            event_loop.exit();
            return;
        };

        // Set initial window title.
        if let Some(ch) = &self.content_handle {
            state.window.set_title(&ch.window_title);
        } else if let Some(interactive) = &self.interactive {
            state.window.set_title(&interactive.window_title);
        }

        state.window.request_redraw();
        self.render_state = Some(state);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.render_state = None;
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.render_state.is_none() {
            return;
        }

        // Process AccessKit events first.
        if let Some(state) = &mut self.render_state {
            state.a11y_adapter.process_event(&state.window, &event);
        }

        // Handle events that always need processing regardless of egui.
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(new_size) => {
                let Some(state) = self.render_state.as_mut() else {
                    return;
                };
                state
                    .gpu
                    .resize(&state.surface, new_size.width, new_size.height);
                if new_size.width > 0 && new_size.height > 0 {
                    state.window.request_redraw();
                }
                return;
            }
            _ => {}
        }

        // ---- Threaded mode ----
        if self.content_handle.is_some() {
            self.handle_window_event_threaded(event_loop, event);
            return;
        }

        // ---- Legacy inline mode ----
        self.handle_window_event_inline(event_loop, event);
    }
}

// --- Threaded mode event handling ---
impl App {
    #[allow(clippy::too_many_lines)]
    fn handle_window_event_threaded(&mut self, _event_loop: &ActiveEventLoop, event: WindowEvent) {
        // Track modifier state on browser side.
        if let WindowEvent::ModifiersChanged(new_modifiers) = &event {
            if let Some(ch) = &mut self.content_handle {
                ch.modifiers = *new_modifiers;
            }
        }

        // Track cursor position for click offset calculation.
        if let WindowEvent::CursorMoved { position, .. } = &event {
            if let Some(ch) = &mut self.content_handle {
                ch.cursor_pos = Some((position.x, position.y));
            }
        }

        // Pass to egui first.
        let egui_consumed = if let Some(state) = self.render_state.as_mut() {
            let response = state.egui_state.on_window_event(&state.window, &event);
            if response.repaint {
                state.window.request_redraw();
            }
            response.consumed
        } else {
            false
        };

        let address_focused = self
            .content_handle
            .as_ref()
            .is_some_and(|ch| ch.chrome.address_focused);

        // Most event arms need a redraw; track exceptions explicitly.
        let mut needs_redraw = true;

        match event {
            WindowEvent::RedrawRequested => {
                needs_redraw = false;
                self.drain_content_messages();

                let chrome_action = {
                    let Self {
                        display_list,
                        render_state,
                        content_handle,
                        ..
                    } = &mut *self;
                    let Some(state) = render_state.as_mut() else {
                        return;
                    };
                    if let Some(ch) = content_handle.as_mut() {
                        handle_redraw(
                            state,
                            display_list,
                            Some(&mut ch.chrome),
                            ch.can_go_back,
                            ch.can_go_forward,
                        )
                    } else {
                        handle_redraw(state, display_list, None, false, false)
                    }
                };

                // TODO(Phase 4): Update AccessKit tree in threaded mode.
                // Currently a11y tree is only updated in legacy inline mode
                // because it requires DOM access (which lives on the content thread).

                if let Some(action) = chrome_action {
                    self.handle_chrome_action_threaded(action);
                    needs_redraw = true;
                }
            }
            _ if egui_consumed => {
                needs_redraw = false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                #[allow(clippy::cast_possible_truncation)]
                let y = (position.y as f32) - crate::chrome::CHROME_HEIGHT;
                #[allow(clippy::cast_possible_truncation)]
                let x = position.x as f32;
                self.send_to_content(BrowserToContent::MouseMove {
                    x,
                    y,
                    client_x: position.x,
                    client_y: position.y,
                });
            }
            WindowEvent::CursorLeft { .. } => {
                self.send_to_content(BrowserToContent::CursorLeft);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if let Some(ch) = self.content_handle.as_ref() {
                    if let Some((cx, cy)) = ch.cursor_pos {
                        #[allow(clippy::cast_possible_truncation)]
                        let x = cx as f32;
                        #[allow(clippy::cast_possible_truncation)]
                        let y = (cy as f32) - crate::chrome::CHROME_HEIGHT;
                        if y >= 0.0 {
                            let mods = Self::to_modifier_state(ch.modifiers.state());
                            self.send_to_content(BrowserToContent::MouseClick {
                                x,
                                y,
                                client_x: cx,
                                client_y: cy,
                                button: winit_button_to_dom(button),
                                mods,
                            });
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => {
                self.send_to_content(BrowserToContent::MouseRelease {
                    button: winit_button_to_dom(button),
                });
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Alt+Left/Right: back/forward navigation.
                let mut handled = false;
                if key_event.state == ElementState::Pressed {
                    let mods = self
                        .content_handle
                        .as_ref()
                        .map(|ch| ch.modifiers.state())
                        .unwrap_or_default();
                    if mods.alt_key() {
                        let nav_msg = match &key_event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                                Some(BrowserToContent::GoBack)
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                                Some(BrowserToContent::GoForward)
                            }
                            _ => None,
                        };
                        if let Some(msg) = nav_msg {
                            self.send_to_content(msg);
                            handled = true;
                        }
                    }
                }

                if !handled
                    && !address_focused
                    && (key_event.state == ElementState::Pressed
                        || key_event.state == ElementState::Released)
                {
                    let event_type = if key_event.state == ElementState::Pressed {
                        "keydown"
                    } else {
                        "keyup"
                    };
                    let (key, code) = crate::key_map::winit_key_to_dom(
                        &key_event.logical_key,
                        key_event.physical_key,
                    );
                    let mods = self
                        .content_handle
                        .as_ref()
                        .map(|ch| ch.modifiers.state())
                        .unwrap_or_default();
                    let ipc_mods = Self::to_modifier_state(mods);

                    let msg = if event_type == "keydown" {
                        BrowserToContent::KeyDown {
                            key,
                            code,
                            repeat: key_event.repeat,
                            mods: ipc_mods,
                        }
                    } else {
                        BrowserToContent::KeyUp {
                            key,
                            code,
                            repeat: key_event.repeat,
                            mods: ipc_mods,
                        }
                    };
                    self.send_to_content(msg);
                }
            }
            _ => {
                needs_redraw = false;
            }
        }

        if needs_redraw {
            if let Some(s) = &self.render_state {
                s.window.request_redraw();
            }
        }
    }

    /// Handle chrome actions in threaded mode.
    ///
    /// Navigation state is managed by the content thread. The browser sends
    /// `Navigate`/`GoBack`/`GoForward` messages; the content thread responds
    /// with `UrlChanged` + `NavigationState` after completing navigation.
    fn handle_chrome_action_threaded(&mut self, action: crate::chrome::ChromeAction) {
        match action {
            crate::chrome::ChromeAction::Navigate(url_str) => {
                let parsed = url::Url::parse(&url_str)
                    .or_else(|_| url::Url::parse(&format!("https://{url_str}")));
                match parsed {
                    Ok(url) => {
                        self.send_to_content(BrowserToContent::Navigate(url));
                    }
                    Err(e) => eprintln!("Invalid URL: {e}"),
                }
            }
            crate::chrome::ChromeAction::Back => {
                self.send_to_content(BrowserToContent::GoBack);
            }
            crate::chrome::ChromeAction::Forward => {
                self.send_to_content(BrowserToContent::GoForward);
            }
            crate::chrome::ChromeAction::Reload => {
                self.send_to_content(BrowserToContent::Reload);
            }
        }
    }
}

// --- Legacy inline mode event handling ---
impl App {
    #[allow(clippy::too_many_lines)]
    fn handle_window_event_inline(&mut self, _event_loop: &ActiveEventLoop, event: WindowEvent) {
        // Always process state-tracking events before egui routing.
        match &event {
            WindowEvent::ModifiersChanged(new_modifiers) => {
                if let Some(interactive) = &mut self.interactive {
                    interactive.modifiers = *new_modifiers;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let hover_changed = if let Some(interactive) = &mut self.interactive {
                    interactive.cursor_pos = Some((position.x, position.y));

                    #[allow(clippy::cast_possible_truncation)]
                    let (x, y) = (
                        position.x as f32,
                        (position.y as f32) - crate::chrome::CHROME_HEIGHT,
                    );
                    let new_chain = if y >= 0.0 {
                        elidex_layout::hit_test(&interactive.pipeline.dom, x, y)
                            .map(|hit| {
                                hover::collect_hover_chain(&interactive.pipeline.dom, hit.entity)
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    if new_chain == interactive.hover_chain {
                        false
                    } else {
                        let old_chain = std::mem::take(&mut interactive.hover_chain);
                        hover::apply_hover_diff(
                            &mut interactive.pipeline.dom,
                            &old_chain,
                            &new_chain,
                        );
                        interactive.hover_chain = new_chain;
                        crate::re_render(&mut interactive.pipeline);
                        true
                    }
                } else {
                    false
                };
                if hover_changed {
                    self.sync_display_list();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(interactive) = &mut self.interactive {
                    let had_hover = !interactive.hover_chain.is_empty();
                    let had_active = !interactive.active_chain.is_empty();
                    for &e in &std::mem::take(&mut interactive.active_chain) {
                        hover::update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    for &e in &std::mem::take(&mut interactive.hover_chain) {
                        hover::update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::HOVER);
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    if had_hover || had_active {
                        crate::re_render(&mut interactive.pipeline);
                    }
                }
                self.sync_display_list();
            }
            _ => {}
        }

        // Pass events to egui.
        let egui_consumed =
            if let (Some(_), Some(state)) = (&self.interactive, self.render_state.as_mut()) {
                let response = state.egui_state.on_window_event(&state.window, &event);
                if response.repaint {
                    state.window.request_redraw();
                }
                response.consumed
            } else {
                false
            };

        let address_focused = self
            .interactive
            .as_ref()
            .is_some_and(|i| i.chrome.address_focused);

        match event {
            WindowEvent::RedrawRequested => {
                let chrome_action = {
                    let Self {
                        display_list,
                        render_state,
                        interactive,
                        ..
                    } = &mut *self;
                    let Some(state) = render_state.as_mut() else {
                        return;
                    };
                    if let Some(interactive) = interactive.as_mut() {
                        let can_back = interactive.nav_controller.can_go_back();
                        let can_fwd = interactive.nav_controller.can_go_forward();
                        handle_redraw(
                            state,
                            display_list,
                            Some(&mut interactive.chrome),
                            can_back,
                            can_fwd,
                        )
                    } else {
                        handle_redraw(state, display_list, None, false, false)
                    }
                };
                // Update accessibility tree after rendering.
                if let (Some(state), Some(interactive)) =
                    (&mut self.render_state, &self.interactive)
                {
                    let dom = &interactive.pipeline.dom;
                    let document = interactive.pipeline.document;
                    let focus = interactive.focus_target.filter(|e| dom.contains(*e));
                    state
                        .a11y_adapter
                        .update_if_active(|| elidex_a11y::build_tree_update(dom, document, focus));
                }

                if let Some(action) = chrome_action {
                    self.handle_chrome_action(action);
                    if let Some(s) = &self.render_state {
                        s.window.request_redraw();
                    }
                }
            }
            _ if egui_consumed => {}
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if let Some(interactive) = &mut self.interactive {
                    // Clear stale ACTIVE from a previous press (e.g. MouseRelease
                    // lost due to window focus change).
                    for &e in &interactive.active_chain {
                        hover::update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    interactive.active_chain = interactive.hover_chain.clone();
                    for &e in &interactive.active_chain {
                        hover::update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.insert(DomElementState::ACTIVE);
                        });
                    }
                }
                self.handle_click(button);
                if let Some(s) = &self.render_state {
                    s.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            } => {
                if let Some(interactive) = &mut self.interactive {
                    let active = std::mem::take(&mut interactive.active_chain);
                    for &e in &active {
                        hover::update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    crate::re_render(&mut interactive.pipeline);
                }
                self.sync_display_list();
                if let Some(s) = &self.render_state {
                    s.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state == ElementState::Pressed {
                    let mods = self
                        .interactive
                        .as_ref()
                        .map(|i| i.modifiers.state())
                        .unwrap_or_default();
                    if mods.alt_key() {
                        let nav_url = match &key_event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                                self.interactive
                                    .as_mut()
                                    .and_then(|i| i.nav_controller.go_back().cloned())
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                                self.interactive
                                    .as_mut()
                                    .and_then(|i| i.nav_controller.go_forward().cloned())
                            }
                            _ => None,
                        };
                        if let Some(url) = nav_url {
                            self.navigate_to_history_url(&url);
                            if let Some(s) = &self.render_state {
                                s.window.request_redraw();
                            }
                            return;
                        }
                    }
                }

                if address_focused {
                    if let Some(s) = &self.render_state {
                        s.window.request_redraw();
                    }
                    return;
                }

                if key_event.state == ElementState::Pressed
                    || key_event.state == ElementState::Released
                {
                    let event_type = if key_event.state == ElementState::Pressed {
                        "keydown"
                    } else {
                        "keyup"
                    };
                    let (key, code) = crate::key_map::winit_key_to_dom(
                        &key_event.logical_key,
                        key_event.physical_key,
                    );
                    let mods = self
                        .interactive
                        .as_ref()
                        .map(|i| i.modifiers.state())
                        .unwrap_or_default();
                    let init = elidex_plugin::KeyboardEventInit {
                        key,
                        code,
                        repeat: key_event.repeat,
                        alt_key: mods.alt_key(),
                        ctrl_key: mods.control_key(),
                        meta_key: mods.super_key(),
                        shift_key: mods.shift_key(),
                    };
                    self.handle_keyboard(event_type, init);
                    if let Some(s) = &self.render_state {
                        s.window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}
