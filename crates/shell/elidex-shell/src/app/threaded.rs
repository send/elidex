//! Threaded mode event handling for the `App`.
//!
//! Each tab runs on a dedicated content thread, communicating via message
//! passing. This module handles all window events in that mode.

use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use elidex_render::DisplayList;

use super::render::{handle_redraw, handle_redraw_with_tabs};
use super::{winit_button_to_dom, App, ContentAreaPlacement};
use crate::chrome::ChromeAction;
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
            self.cursor_pos = Some(elidex_plugin::Point::new(position.x, position.y));
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

        // Content-area origin + scale for input mapping come from the cached
        // placement SoT — no second `chrome_content_offset` call (the builder
        // `content_area_placement` is its single caller). `None` before the
        // window exists; the input arms below skip mapping in that case.
        let placement = self.placement;

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
                if let Some(placement) = placement {
                    self.handle_cursor_move_threaded(
                        elidex_plugin::Point::new(position.x, position.y),
                        placement,
                    );
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
                if let Some(placement) = placement {
                    self.handle_mouse_press_threaded(button, placement);
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
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(placement) = placement {
                    self.handle_mouse_wheel_threaded(delta, placement);
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.handle_keyboard_threaded(event_loop, &key_event, address_focused);
            }
            WindowEvent::Occluded(occluded) => {
                // Page Visibility §4.1: dispatch visibilitychange when window
                // becomes occluded/unoccluded.
                self.send_to_content(BrowserToContent::VisibilityChanged { visible: !occluded });
                needs_redraw = false;
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
        self.sync_cookies_if_dirty();

        // Recompute the placement SoT for this frame from the current window —
        // the authoritative per-frame snapshot (§2.3 / F8 coherence with the
        // `surface_config` written on `Resized`). Then derive the compositor's
        // physical-px content placement (offset/size/scale) for this frame.
        if let Some(window) = self
            .render_state
            .as_ref()
            .map(|s| std::sync::Arc::clone(&s.window))
        {
            self.placement = Some(self.content_area_placement(&window));
        }
        let content = self.placement.map(|p| elidex_render::ContentPlacement {
            offset: p.origin_phys(),
            size: p.size_phys(),
            scale: p.scale_factor,
        });

        // Apply pending window.focus() request.
        if self.pending_focus {
            self.pending_focus = false;
            if let Some(state) = &self.render_state {
                state.window.focus_window();
            }
        }

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
                    content,
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
    /// Map a physical-px cursor position to content-area CSS px:
    /// `(cursor ÷ scale_factor) − origin_logical` — the exact inverse of the
    /// compositor transform (B-D2). The single input-mapping primitive shared by
    /// all three threaded pointer handlers.
    fn cursor_to_content(
        cursor: elidex_plugin::Point<f64>,
        placement: ContentAreaPlacement,
    ) -> elidex_plugin::Point {
        let p = cursor.to_f32();
        let scale = placement.scale_factor;
        elidex_plugin::Point::new(
            p.x / scale - placement.origin_logical.x,
            p.y / scale - placement.origin_logical.y,
        )
    }

    /// Map a physical-px cursor to window-logical CSS px (`cursor ÷ scale`) for
    /// the DOM `clientX`/`clientY` fields. Scale-only — like [`cursor_to_content`]
    /// it removes the device pixel ratio so content receives CSS px (B-D2), but
    /// it keeps the *window-relative* origin the `client_point` field already
    /// used (the content-area-relative origin convention for `clientX` is a
    /// pre-existing, separate concern, untouched here). Identity at scale 1.
    fn cursor_to_window_css(
        cursor: elidex_plugin::Point<f64>,
        scale: f32,
    ) -> elidex_plugin::Point<f64> {
        let s = f64::from(scale);
        elidex_plugin::Point::new(cursor.x / s, cursor.y / s)
    }

    fn handle_cursor_move_threaded(
        &mut self,
        client_pos: elidex_plugin::Point<f64>,
        placement: ContentAreaPlacement,
    ) {
        let content_pos = Self::cursor_to_content(client_pos, placement);
        if content_pos.x >= 0.0 && content_pos.y >= 0.0 {
            self.cursor_in_content = true;
            self.send_to_content(BrowserToContent::MouseMove {
                point: content_pos,
                client_point: Self::cursor_to_window_css(client_pos, placement.scale_factor),
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
        placement: ContentAreaPlacement,
    ) {
        if let Some(cursor) = self.cursor_pos {
            let content_pos = Self::cursor_to_content(cursor, placement);
            if content_pos.x >= 0.0 && content_pos.y >= 0.0 {
                let mods = Self::to_modifier_state(self.modifiers.state());
                self.send_to_content(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
                    point: content_pos,
                    client_point: Self::cursor_to_window_css(cursor, placement.scale_factor),
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
        placement: ContentAreaPlacement,
    ) {
        const LINE_SCROLL_PX: f64 = 40.0;
        let scroll_delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => elidex_plugin::Vector::new(
                -f64::from(x) * LINE_SCROLL_PX,
                -f64::from(y) * LINE_SCROLL_PX,
            ),
            MouseScrollDelta::PixelDelta(pos) => elidex_plugin::Vector::new(-pos.x, -pos.y),
        };
        let point = self
            .cursor_pos
            .map_or(elidex_plugin::Point::ZERO, |cursor| {
                Self::cursor_to_content(cursor, placement)
            });
        self.send_to_content(BrowserToContent::MouseWheel {
            delta: scroll_delta,
            point,
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
                    // Notify the old active tab that it is now hidden.
                    if let Some(old_tab) = mgr.active_tab() {
                        let _ = old_tab
                            .channel
                            .send(BrowserToContent::VisibilityChanged { visible: false });
                    }
                    mgr.set_active(id);
                    // Notify the new active tab that it is now visible.
                    if let Some(new_tab) = mgr.active_tab() {
                        let _ = new_tab
                            .channel
                            .send(BrowserToContent::VisibilityChanged { visible: true });
                    }
                }
            }
        }
    }

    /// Open a new blank tab.
    fn open_new_tab(&mut self) {
        let Some(mgr) = &mut self.tab_manager else {
            return;
        };
        let Some(np) = &self.network_process else {
            return;
        };
        let nh = np.create_renderer_handle();
        let jar = std::sync::Arc::clone(np.cookie_jar());
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        // Mint via the disjoint `wake_proxy` field (an associated fn, not `&self`)
        // so it coexists with the active `&mut mgr` borrow.
        let wake = Self::wake_or_noop(self.wake_proxy.as_ref());
        let thread = crate::content::spawn_content_thread_blank(content_ch, nh, jar, wake);
        mgr.create_tab(
            browser_ch,
            thread,
            crate::chrome::ChromeState::new(None),
            "New Tab".to_string(),
        );
    }
}

#[cfg(test)]
mod placement_input_tests {
    use super::{App, ContentAreaPlacement};
    use elidex_plugin::{Point, Size};

    fn placement(origin: Point, scale: f32) -> ContentAreaPlacement {
        ContentAreaPlacement {
            origin_logical: origin,
            size_logical: Size::new(800.0, 600.0),
            scale_factor: scale,
        }
    }

    #[test]
    fn placement_phys_derivations_scale_by_factor() {
        // origin_phys = origin_logical × scale; size_phys = size_logical × scale.
        let pl = placement(Point::new(200.0, 36.0), 2.0);
        let o = pl.origin_phys();
        assert!((o.x - 400.0).abs() < f32::EPSILON);
        assert!((o.y - 72.0).abs() < f32::EPSILON);
        let s = pl.size_phys();
        assert!((s.width - 1600.0).abs() < f32::EPSILON);
        assert!((s.height - 1200.0).abs() < f32::EPSILON);
    }

    /// A click at the physical pixel where the compositor paints a content
    /// CSS-px point round-trips back to that exact CSS point — the input mapper
    /// `(cursor ÷ scale) − origin_logical` is the exact inverse of the
    /// compositor transform `content × scale + origin_phys` (invariant I3), at
    /// scale 1 and 2 for Top/Left/Right chrome.
    #[test]
    fn cursor_to_content_inverts_compositor_transform() {
        // (origin_logical, scale): Top@1, Top@2, Left@2, Right@1.
        let cases = [
            (Point::new(0.0, 64.0), 1.0_f32),
            (Point::new(0.0, 64.0), 2.0),
            (Point::new(200.0, 36.0), 2.0),
            (Point::new(0.0, 36.0), 1.0),
        ];
        let content_points = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(799.0, 599.0),
        ];
        for (origin, scale) in cases {
            let pl = placement(origin, scale);
            for c in content_points {
                // Compositor: a content CSS point `c` is painted at physical
                // `(c + origin_logical) × scale`.
                let phys = Point::<f64>::new(
                    f64::from((c.x + origin.x) * scale),
                    f64::from((c.y + origin.y) * scale),
                );
                let back = App::cursor_to_content(phys, pl);
                assert!((back.x - c.x).abs() < 1e-3, "x {} != {}", back.x, c.x);
                assert!((back.y - c.y).abs() < 1e-3, "y {} != {}", back.y, c.y);
            }
        }
    }

    /// `client_point` (DOM `clientX`/`clientY`) is scaled to CSS px (`÷ scale`)
    /// like `point`, not left raw-physical — so it does not double at scale 2.
    /// Scale-only (window-relative origin preserved), identity at scale 1.
    #[test]
    fn cursor_to_window_css_removes_device_scale() {
        // scale 1: identity.
        let p1 = App::cursor_to_window_css(Point::<f64>::new(120.0, 200.0), 1.0);
        assert!((p1.x - 120.0).abs() < 1e-9 && (p1.y - 200.0).abs() < 1e-9);
        // scale 2: physical (120, 200) → CSS (60, 100), not left at 2×.
        let p2 = App::cursor_to_window_css(Point::<f64>::new(120.0, 200.0), 2.0);
        assert!((p2.x - 60.0).abs() < 1e-9 && (p2.y - 100.0).abs() < 1e-9);
    }
}
