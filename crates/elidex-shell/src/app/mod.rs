//! winit application handler for the elidex browser shell.
//!
//! Implements [`ApplicationHandler`] to manage the window lifecycle,
//! GPU initialization, frame rendering via Vello, and user input
//! event dispatch to the DOM.
//!
//! Supports two modes:
//! - **Threaded** (`TabManager`): each tab runs on a dedicated content thread,
//!   communicating via message passing.
//! - **Legacy inline** (`InteractiveState`): all processing on the main
//!   thread (used by `build_pipeline` test API).

pub(crate) mod events;
pub(crate) mod hover;
pub(crate) mod navigation;
mod render;
pub(crate) mod tab;

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Modifiers, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use elidex_ecs::{ElementState as DomElementState, Entity};
use elidex_navigation::NavigationController;
use elidex_render::DisplayList;
use wgpu::util::TextureBlitter;
use wgpu::{Instance, Surface};

use crate::chrome::{self, ChromeAction, TabBarInfo};
use crate::ipc::{BrowserToContent, ContentToBrowser, ModifierState};

use render::{handle_redraw, try_init_render_state};
use tab::TabManager;

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
    pub(super) render_state: Option<RenderState>,
    /// Multi-tab manager (threaded mode).
    tab_manager: Option<TabManager>,
    /// Window-level cursor position (shared across tabs).
    pub(super) cursor_pos: Option<(f64, f64)>,
    /// Window-level modifier state (shared across tabs).
    pub(super) modifiers: Modifiers,
    /// Whether the cursor was in the content area on the last move event.
    /// Used to send exactly one `CursorLeft` when the cursor moves into the chrome area.
    cursor_in_content: bool,
    /// Legacy inline interactive state.
    pub(super) interactive: Option<InteractiveState>,
}

impl App {
    /// Create a threaded-mode `App` from a pre-initialized `TabManager`.
    fn from_tab_manager(mgr: TabManager) -> Self {
        Self {
            render_state: None,
            tab_manager: Some(mgr),
            cursor_pos: None,
            modifiers: Modifiers::default(),
            cursor_in_content: false,
            interactive: None,
        }
    }

    /// Create a new threaded application from HTML/CSS.
    pub fn new_threaded(html: String, css: String) -> Self {
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let thread = crate::content::spawn_content_thread(content_ch, html, css);

        let mut mgr = TabManager::new();
        mgr.create_tab(
            browser_ch,
            thread,
            crate::chrome::ChromeState::new(None),
            "elidex".to_string(),
        );

        Self::from_tab_manager(mgr)
    }

    /// Create a new threaded application from a URL.
    pub fn new_threaded_url(url: url::Url) -> Self {
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let title = format!("elidex \u{2014} {url}");
        let chrome = crate::chrome::ChromeState::new(Some(&url));
        let thread = crate::content::spawn_content_thread_url(content_ch, url);

        let mut mgr = TabManager::new();
        mgr.create_tab(browser_ch, thread, chrome, title);

        Self::from_tab_manager(mgr)
    }

    /// Create a new legacy (inline) interactive application from a pipeline result.
    #[allow(dead_code)]
    pub fn new_interactive(pipeline: crate::PipelineResult) -> Self {
        Self {
            render_state: None,
            tab_manager: None,
            cursor_pos: None,
            modifiers: Modifiers::default(),
            cursor_in_content: false,
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
        let chrome = crate::chrome::ChromeState::new(pipeline.url.as_ref());
        let mut nav_controller = NavigationController::new();
        if let Some(url) = &pipeline.url {
            nav_controller.push(url.clone());
        }
        Self {
            render_state: None,
            tab_manager: None,
            cursor_pos: None,
            modifiers: Modifiers::default(),
            cursor_in_content: false,
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

    /// Maximum messages to drain per tab per frame.
    ///
    /// Prevents a runaway content thread from monopolizing the browser thread's
    /// event loop. Any remaining messages will be drained on the next frame.
    const MAX_DRAIN_PER_TAB: usize = 1000;

    /// Drain all pending messages from all tabs.
    fn drain_content_messages(&mut self) {
        let Some(mgr) = &mut self.tab_manager else {
            return;
        };
        for tab in mgr.tabs_mut() {
            let mut drained = 0;
            while drained < Self::MAX_DRAIN_PER_TAB {
                let Ok(msg) = tab.channel.try_recv() else {
                    break;
                };
                drained += 1;
                match msg {
                    ContentToBrowser::DisplayListReady(dl) => {
                        tab.display_list = dl;
                    }
                    ContentToBrowser::TitleChanged(title) => {
                        tab.window_title = title;
                    }
                    ContentToBrowser::NavigationState {
                        can_go_back,
                        can_go_forward,
                    } => {
                        tab.can_go_back = can_go_back;
                        tab.can_go_forward = can_go_forward;
                    }
                    ContentToBrowser::UrlChanged(url) => {
                        tab.chrome.set_url(&url);
                    }
                    ContentToBrowser::NavigationFailed { url, error } => {
                        eprintln!("Navigation to {url} failed: {error}");
                    }
                }
            }
        }

        // Update window title only when the active tab's title changed.
        if let Some(tab) = mgr.active_tab() {
            if let Some(state) = &self.render_state {
                if state.window.title() != tab.window_title {
                    state.window.set_title(&tab.window_title);
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

    /// Send a message to the active tab's content thread.
    fn send_to_content(&self, msg: BrowserToContent) {
        if let Some(mgr) = &self.tab_manager {
            if let Some(tab) = mgr.active_tab() {
                if let Err(e) = tab.channel.send(msg) {
                    eprintln!("Failed to send to content thread (disconnected): {e}");
                }
            }
        }
    }

    /// Get the tab bar position from the active tab's chrome state.
    fn tab_bar_position(&self) -> chrome::TabBarPosition {
        self.tab_manager
            .as_ref()
            .and_then(|mgr| mgr.active_tab())
            .map_or(chrome::TabBarPosition::Top, |tab| {
                tab.chrome.tab_bar_position
            })
    }

    /// Build tab bar info for all tabs.
    fn tab_bar_infos(&self) -> Vec<TabBarInfo> {
        let Some(mgr) = &self.tab_manager else {
            return Vec::new();
        };
        let active_id = mgr.active_id();
        mgr.tabs()
            .iter()
            .map(|tab| TabBarInfo {
                id: tab.id,
                title: tab.window_title.clone(),
                is_active: Some(tab.id) == active_id,
            })
            .collect()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(mgr) = &mut self.tab_manager {
            mgr.shutdown_all();
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
        if let Some(mgr) = &self.tab_manager {
            if let Some(tab) = mgr.active_tab() {
                state.window.set_title(&tab.window_title);
            }
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
        if self.tab_manager.is_some() {
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
    fn handle_window_event_threaded(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        // Track modifier state on browser side.
        if let WindowEvent::ModifiersChanged(new_modifiers) = &event {
            self.modifiers = *new_modifiers;
        }

        // Track cursor position.
        if let WindowEvent::CursorMoved { position, .. } = &event {
            self.cursor_pos = Some((position.x, position.y));
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
            .tab_manager
            .as_ref()
            .and_then(|mgr| mgr.active_tab())
            .is_some_and(|tab| tab.chrome.address_focused);

        let position = self.tab_bar_position();
        let (x_offset, y_offset) = chrome::chrome_content_offset(position);

        // Most event arms need a redraw; track exceptions explicitly.
        let mut needs_redraw = true;

        match event {
            WindowEvent::RedrawRequested => {
                needs_redraw = false;
                self.drain_content_messages();

                let tab_infos = self.tab_bar_infos();

                let chrome_actions = {
                    let Self {
                        render_state,
                        tab_manager,
                        ..
                    } = &mut *self;
                    let Some(state) = render_state.as_mut() else {
                        return;
                    };
                    // tab_manager is always Some in threaded mode.
                    let mgr = tab_manager
                        .as_mut()
                        .expect("threaded mode requires tab_manager");
                    if let Some(tab) = mgr.active_tab_mut() {
                        render::handle_redraw_with_tabs(
                            state,
                            &tab.display_list,
                            &mut tab.chrome,
                            tab.can_go_back,
                            tab.can_go_forward,
                            &tab_infos,
                            position,
                        )
                    } else {
                        let empty = DisplayList::default();
                        handle_redraw(state, &empty, None, false, false);
                        Vec::new()
                    }
                };

                // TODO(Phase 4): Update accessibility tree in threaded mode.
                // The DOM lives on the content thread, so we can't call
                // `a11y_adapter.update_if_active()` here (unlike legacy mode).
                // Requires a new IPC message (e.g. ContentToBrowser::A11yTreeReady).

                for action in chrome_actions {
                    self.handle_chrome_action_threaded(event_loop, action);
                    needs_redraw = true;
                }
            }
            _ if egui_consumed => {
                needs_redraw = false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                #[allow(clippy::cast_possible_truncation)]
                let x = (position.x as f32) - x_offset;
                #[allow(clippy::cast_possible_truncation)]
                let y = (position.y as f32) - y_offset;
                // Only send to content thread when cursor is in the content area.
                // When cursor first moves into chrome/tab bar, send CursorLeft
                // to clear hover state (otherwise :hover stays stuck).
                if x >= 0.0 && y >= 0.0 {
                    self.cursor_in_content = true;
                    self.send_to_content(BrowserToContent::MouseMove {
                        x,
                        y,
                        client_x: position.x,
                        client_y: position.y,
                    });
                } else if self.cursor_in_content {
                    self.cursor_in_content = false;
                    self.send_to_content(BrowserToContent::CursorLeft);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor_in_content = false;
                self.send_to_content(BrowserToContent::CursorLeft);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if let Some((cx, cy)) = self.cursor_pos {
                    #[allow(clippy::cast_possible_truncation)]
                    let x = (cx as f32) - x_offset;
                    #[allow(clippy::cast_possible_truncation)]
                    let y = (cy as f32) - y_offset;
                    if y >= 0.0 && x >= 0.0 {
                        let mods = Self::to_modifier_state(self.modifiers.state());
                        self.send_to_content(BrowserToContent::MouseClick(
                            crate::ipc::MouseClickEvent {
                                x,
                                y,
                                client_x: cx,
                                client_y: cy,
                                button: winit_button_to_dom(button),
                                mods,
                            },
                        ));
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
                let mut handled = false;
                if key_event.state == ElementState::Pressed {
                    let mods = self.modifiers.state();

                    // Tab keyboard shortcuts (Cmd/Ctrl+T/W, Ctrl+Tab, Ctrl+1-9).
                    if let Some(action) = self.check_tab_shortcut(&key_event, mods) {
                        self.handle_chrome_action_threaded(event_loop, action);
                        handled = true;
                    }

                    // Alt+Left/Right: back/forward navigation.
                    if !handled && mods.alt_key() {
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
                    let ipc_mods = Self::to_modifier_state(self.modifiers.state());

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

    /// Check for tab-related keyboard shortcuts.
    fn check_tab_shortcut(
        &self,
        key_event: &winit::event::KeyEvent,
        mods: winit::keyboard::ModifiersState,
    ) -> Option<ChromeAction> {
        // Cmd (macOS) or Ctrl (other) as the primary modifier.
        let cmd_or_ctrl = if cfg!(target_os = "macos") {
            mods.super_key()
        } else {
            mods.control_key()
        };

        if cmd_or_ctrl {
            match &key_event.logical_key {
                winit::keyboard::Key::Character(c) if c.as_str() == "t" => {
                    return Some(ChromeAction::NewTab);
                }
                winit::keyboard::Key::Character(c) if c.as_str() == "w" => {
                    let mgr = self.tab_manager.as_ref()?;
                    let id = mgr.active_id()?;
                    return Some(ChromeAction::CloseTab(id));
                }
                // Ctrl/Cmd+1-8: switch to nth tab. Ctrl/Cmd+9: last tab
                // (Chrome/Firefox convention).
                winit::keyboard::Key::Character(c) => {
                    if let Some(digit) = c.as_str().chars().next().and_then(|ch| ch.to_digit(10)) {
                        if (1..=9).contains(&digit) {
                            let mgr = self.tab_manager.as_ref()?;
                            let id = if digit == 9 {
                                // Cmd/Ctrl+9 always selects the last tab.
                                let count = mgr.tabs().len();
                                mgr.nth_tab_id(count.saturating_sub(1))?
                            } else {
                                mgr.nth_tab_id((digit - 1) as usize)?
                            };
                            return Some(ChromeAction::SwitchTab(id));
                        }
                    }
                }
                _ => {}
            }
        }

        // Ctrl+Tab / Ctrl+Shift+Tab: cycle tabs.
        // NOTE: macOS HIG uses Cmd+Shift+]/[ for tab cycling, but Chrome/Firefox
        // use Ctrl+Tab on all platforms. We follow the Chrome/Firefox convention.
        if mods.control_key() {
            if let winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab) =
                &key_event.logical_key
            {
                let mgr = self.tab_manager.as_ref()?;
                let id = if mods.shift_key() {
                    mgr.prev_tab_id()?
                } else {
                    mgr.next_tab_id()?
                };
                return Some(ChromeAction::SwitchTab(id));
            }
        }

        None
    }

    /// Handle chrome actions in threaded mode.
    fn handle_chrome_action_threaded(
        &mut self,
        event_loop: &ActiveEventLoop,
        action: ChromeAction,
    ) {
        match action {
            ChromeAction::Navigate(url_str) => {
                let parsed = url::Url::parse(&url_str)
                    .or_else(|_| url::Url::parse(&format!("https://{url_str}")));
                match parsed {
                    Ok(url) => {
                        self.send_to_content(BrowserToContent::Navigate(url));
                    }
                    Err(e) => eprintln!("Invalid URL: {e}"),
                }
            }
            ChromeAction::Back => {
                self.send_to_content(BrowserToContent::GoBack);
            }
            ChromeAction::Forward => {
                self.send_to_content(BrowserToContent::GoForward);
            }
            ChromeAction::Reload => {
                self.send_to_content(BrowserToContent::Reload);
            }
            ChromeAction::NewTab => {
                self.open_new_tab();
            }
            ChromeAction::CloseTab(id) => {
                if let Some(mgr) = &mut self.tab_manager {
                    let has_tabs = mgr.close_tab(id);
                    if !has_tabs {
                        event_loop.exit();
                    }
                }
            }
            ChromeAction::SwitchTab(id) => {
                if let Some(mgr) = &mut self.tab_manager {
                    mgr.set_active(id);
                }
            }
        }
    }

    /// Open a new blank tab.
    fn open_new_tab(&mut self) {
        let Some(mgr) = &mut self.tab_manager else {
            return;
        };
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let thread = crate::content::spawn_content_thread_blank(content_ch);
        mgr.create_tab(
            browser_ch,
            thread,
            crate::chrome::ChromeState::new(None),
            "New Tab".to_string(),
        );
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
                    if let Some(s) = &self.render_state {
                        s.window.request_redraw();
                    }
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
                        if let Some(s) = &self.render_state {
                            s.window.request_redraw();
                        }
                    }
                }
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
                            &interactive.pipeline.display_list,
                            Some(&mut interactive.chrome),
                            can_back,
                            can_fwd,
                        )
                    } else {
                        handle_redraw(state, &DisplayList::default(), None, false, false)
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
                    // Clear stale ACTIVE from a previous press.
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
