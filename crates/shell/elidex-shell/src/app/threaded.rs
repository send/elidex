//! Threaded mode event handling for the `App`.
//!
//! Each tab runs on a dedicated content thread, communicating via message
//! passing. This module handles all window events in that mode.

use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use elidex_render::DisplayList;

use super::render::{handle_redraw, handle_redraw_with_tabs};
use super::{winit_button_to_dom, App};
use crate::chrome::{self, ChromeAction};
use crate::ipc::{BrowserToContent, ContentToBrowser};

impl App {
    pub(super) fn handle_window_event_threaded(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) {
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
                needs_redraw = self.handle_redraw_threaded(event_loop);
            }
            _ if egui_consumed => {
                needs_redraw = false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_move_threaded(position.x, position.y, x_offset, y_offset);
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
                self.handle_mouse_press_threaded(button, x_offset, y_offset);
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
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel_threaded(delta, x_offset, y_offset);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.handle_keyboard_threaded(event_loop, &key_event, address_focused);
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

    /// Handle `RedrawRequested` in threaded mode.
    ///
    /// Returns `true` if a redraw is needed (due to chrome actions).
    fn handle_redraw_threaded(&mut self, event_loop: &ActiveEventLoop) -> bool {
        self.drain_content_messages();

        let tab_infos = self.tab_bar_infos();
        let position = self.tab_bar_position();

        let chrome_actions = {
            let Self {
                render_state,
                tab_manager,
                ..
            } = &mut *self;
            let Some(state) = render_state.as_mut() else {
                return false;
            };
            // tab_manager is always Some in threaded mode.
            let mgr = tab_manager
                .as_mut()
                .expect("threaded mode requires tab_manager");
            if let Some(tab) = mgr.active_tab_mut() {
                handle_redraw_with_tabs(
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

        // Accessibility tree update in threaded mode:
        //
        // In legacy (single-thread) mode, the a11y tree is built directly from
        // the ECS DOM via `elidex_a11y::build_tree_update()` and pushed to the
        // platform adapter. In threaded mode, the DOM lives on the content thread
        // and cannot be accessed from the browser thread.
        //
        // To support this, the content thread should:
        // 1. Build the `accesskit::TreeUpdate` after each layout pass
        //    (using `elidex_a11y::build_tree_update()`)
        // 2. Send it via a new `ContentToBrowser::A11yTreeReady(TreeUpdate)` message
        // 3. The browser thread receives it in `drain_content_messages()` and calls
        //    `a11y_adapter.update_if_active(|| tree_update)`
        //
        // This mirrors the existing `DisplayListReady` pattern. The `TreeUpdate`
        // type is `Send` (it contains only owned data), so it can cross threads.
        // Implementation deferred until accessibility testing infrastructure is
        // available.

        let mut needs_redraw = false;
        for action in chrome_actions {
            self.handle_chrome_action_threaded(event_loop, action);
            needs_redraw = true;
        }
        needs_redraw
    }

    /// Handle `CursorMoved` in threaded mode.
    ///
    /// Only sends to content thread when cursor is in the content area.
    /// When cursor first moves into chrome/tab bar, sends `CursorLeft`
    /// to clear hover state (otherwise `:hover` stays stuck).
    fn handle_cursor_move_threaded(
        &mut self,
        client_x: f64,
        client_y: f64,
        x_offset: f32,
        y_offset: f32,
    ) {
        #[allow(clippy::cast_possible_truncation)]
        let x = (client_x as f32) - x_offset;
        #[allow(clippy::cast_possible_truncation)]
        let y = (client_y as f32) - y_offset;
        if x >= 0.0 && y >= 0.0 {
            self.cursor_in_content = true;
            self.send_to_content(BrowserToContent::MouseMove {
                x,
                y,
                client_x,
                client_y,
            });
        } else if self.cursor_in_content {
            self.cursor_in_content = false;
            self.send_to_content(BrowserToContent::CursorLeft);
        }
    }

    /// Handle `MouseInput::Pressed` in threaded mode.
    fn handle_mouse_press_threaded(
        &mut self,
        button: winit::event::MouseButton,
        x_offset: f32,
        y_offset: f32,
    ) {
        if let Some((cx, cy)) = self.cursor_pos {
            #[allow(clippy::cast_possible_truncation)]
            let x = (cx as f32) - x_offset;
            #[allow(clippy::cast_possible_truncation)]
            let y = (cy as f32) - y_offset;
            if y >= 0.0 && x >= 0.0 {
                let mods = Self::to_modifier_state(self.modifiers.state());
                self.send_to_content(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
                    x,
                    y,
                    client_x: cx,
                    client_y: cy,
                    button: winit_button_to_dom(button),
                    mods,
                }));
            }
        }
    }

    /// Handle `MouseWheel` in threaded mode.
    ///
    /// Converts winit scroll deltas to CSS pixels and sends to content thread.
    /// Winit convention: positive = content moves right/down (natural scroll).
    /// Browser convention: positive delta = scrollTop increases (scroll down).
    /// These are opposite, so deltas are negated.
    /// `LineDelta` is multiplied by 40px per line (typical browser behavior);
    /// `PixelDelta` is used as-is (already in CSS pixels on most platforms).
    fn handle_mouse_wheel_threaded(
        &mut self,
        delta: MouseScrollDelta,
        x_offset: f32,
        y_offset: f32,
    ) {
        const LINE_SCROLL_PX: f64 = 40.0;
        let (delta_x, delta_y) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (
                -f64::from(x) * LINE_SCROLL_PX,
                -f64::from(y) * LINE_SCROLL_PX,
            ),
            MouseScrollDelta::PixelDelta(pos) => (-pos.x, -pos.y),
        };
        let (x, y) = self.cursor_pos.map_or((0.0_f32, 0.0_f32), |(cx, cy)| {
            #[allow(clippy::cast_possible_truncation)]
            ((cx as f32) - x_offset, (cy as f32) - y_offset)
        });
        self.send_to_content(BrowserToContent::MouseWheel {
            delta_x,
            delta_y,
            x,
            y,
        });
    }

    /// Handle `KeyboardInput` in threaded mode.
    fn handle_keyboard_threaded(
        &mut self,
        event_loop: &ActiveEventLoop,
        key_event: &winit::event::KeyEvent,
        address_focused: bool,
    ) {
        let mut handled = false;
        if key_event.state == ElementState::Pressed {
            let mods = self.modifiers.state();

            // Tab keyboard shortcuts (Cmd/Ctrl+T/W, Ctrl+Tab, Ctrl+1-9).
            if let Some(action) = self.check_tab_shortcut(key_event, mods) {
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
            let (key, code) =
                crate::key_map::winit_key_to_dom(&key_event.logical_key, key_event.physical_key);
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
