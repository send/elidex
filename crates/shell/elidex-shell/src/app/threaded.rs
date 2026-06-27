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
        let placement = self.viewport.placement;

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

        // Recompute the placement SoT + device facts for this frame from the settled
        // window state, then **publish + fan out atomically here** — the single
        // steady-state device-state chokepoint (C3 D2). `placement` and the cell `seq`
        // update together, so input stamped in the separate `CursorMoved`/`MouseInput`
        // arms between an event and this redraw reads a coherent `(placement, seq)`
        // pair (the F1 atomicity the `Resized`/`ScaleFactorChanged` arms preserve by
        // touching neither). The publish is idempotent: a frame with no real change
        // (animations) bumps no seq and broadcasts nothing.
        if let Some(window) = self
            .render_state
            .as_ref()
            .map(|s| std::sync::Arc::clone(&s.window))
        {
            let prev = self.viewport.placement;
            let placement = self.content_area_placement(&window);
            self.viewport.placement = Some(placement);
            let facts = Self::device_facts(placement, &window);
            let delta = self
                .viewport
                .viewport_cell
                .publish_device_state(placement.size_logical, facts);
            // Size → `SetViewport` (+`seq` bump, the C2 input-drop discipline); device
            // facts → `SetDeviceFacts` (no *size* `seq` — D3 — but its own `facts_seq`
            // generation for delivery-staleness). Each gated on its own change so a
            // pure-scale change the OS absorbs ships facts without manufacturing a
            // phantom input-drop generation.
            if delta.size_changed {
                self.broadcast_viewport();
            }
            if delta.facts_changed {
                self.broadcast_device_facts();
            }
            // Reclip the cursor on ANY real placement change — a shrunk/moved content
            // rect may have slid out from under a stationary cursor, leaving stuck
            // `:hover`. Gated on the **full** `placement != prev` comparison (origin +
            // scale + size), NOT the size-only `DeviceDelta`, so the origin axis a
            // future tab-bar-position change (#11-window-level-tab-bar-position) makes
            // live is covered by construction (C3 F1 re-check / E11).
            if prev != Some(placement) {
                self.reclip_cursor_after_placement_change(placement);
            }
        }
        let content = self
            .viewport
            .placement
            .map(|p| elidex_render::ContentPlacement {
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
    /// compositor transform (B-D2). The single input-mapping primitive: both the
    /// hit-test `point` (via `.to_f32()`) and the DOM `clientX`/`clientY`
    /// `client_point` (f64) derive from this one content-area-relative mapping,
    /// so the two coordinates the content thread receives are the *same* viewport
    /// point — the `ipc` contract (both `point` and `client_point` are
    /// content-area/viewport-relative; `clientX`/`clientY` are viewport-relative
    /// per CSSOM View Module Level 1 §10 (Extensions to the MouseEvent
    /// Interface), and the iframe router subtracts the iframe's
    /// content-relative origin from both). Computed in f64 to preserve
    /// `clientX`/`clientY` precision.
    fn cursor_to_content(
        cursor: elidex_plugin::Point<f64>,
        placement: ContentAreaPlacement,
    ) -> elidex_plugin::Point<f64> {
        let scale = f64::from(placement.scale_factor);
        elidex_plugin::Point::new(
            cursor.x / scale - f64::from(placement.origin_logical.x),
            cursor.y / scale - f64::from(placement.origin_logical.y),
        )
    }

    /// Whether a content-area CSS-px point falls inside the placed content rect
    /// `[0, size_logical)`. The compositor clips the page to `size_logical`, so a
    /// point past the right/bottom edge (e.g. a `TabBarPosition::Right` sidebar,
    /// where `x ≥ 0` but `x ≥ size_logical.width`) is over chrome, not painted
    /// page content, and must not be treated as in-content. The shared in-content
    /// gate for all three threaded pointer handlers (B-D2 / invariant I1: the
    /// input rect equals the painted rect).
    fn point_in_content(
        content: elidex_plugin::Point<f64>,
        placement: ContentAreaPlacement,
    ) -> bool {
        content.x >= 0.0
            && content.y >= 0.0
            && content.x < f64::from(placement.size_logical.width)
            && content.y < f64::from(placement.size_logical.height)
    }

    /// Whether a stationary cursor at `cursor_pos` now falls **outside** the
    /// content rect under `placement` — i.e. a placement change (a resize that
    /// shrinks/moves the content area) left a previously-in-content cursor over
    /// chrome. `None` cursor → never "left" (nothing to clear).
    fn cursor_left_content(
        cursor_pos: Option<elidex_plugin::Point<f64>>,
        placement: ContentAreaPlacement,
    ) -> bool {
        cursor_pos.is_some_and(|c| {
            !Self::point_in_content(Self::cursor_to_content(c, placement), placement)
        })
    }

    /// After the placement changes without a `CursorMoved` (e.g. an OS resize),
    /// re-run the in-content gate for the cached cursor: if it was inside content
    /// and the new content rect no longer contains it, clear `cursor_in_content`
    /// and send `CursorLeft` so `:hover` does not stay stuck until the next
    /// pointer move. No-op in legacy mode (`send_to_content` has no active tab).
    pub(super) fn reclip_cursor_after_placement_change(&mut self, placement: ContentAreaPlacement) {
        if self.cursor_in_content && Self::cursor_left_content(self.cursor_pos, placement) {
            self.cursor_in_content = false;
            self.send_to_content(BrowserToContent::CursorLeft);
        }
    }

    /// Convert a winit scroll delta to CSS-px scroll vector (browser sign
    /// convention: positive = scrollTop increases, so winit deltas are negated).
    /// `LineDelta` → 40 CSS px per line. `PixelDelta` is **physical** px, divided
    /// by `scale_factor` to CSS px (B-D2) so HiDPI scroll distance matches the
    /// cursor mapping and the painted content (the `MouseWheel` IPC contract is
    /// CSS px).
    fn scroll_delta_to_css(delta: MouseScrollDelta, scale: f32) -> elidex_plugin::Vector<f64> {
        const LINE_SCROLL_PX: f64 = 40.0;
        match delta {
            MouseScrollDelta::LineDelta(x, y) => elidex_plugin::Vector::new(
                -f64::from(x) * LINE_SCROLL_PX,
                -f64::from(y) * LINE_SCROLL_PX,
            ),
            MouseScrollDelta::PixelDelta(pos) => {
                let s = f64::from(scale);
                elidex_plugin::Vector::new(-pos.x / s, -pos.y / s)
            }
        }
    }

    fn handle_cursor_move_threaded(
        &mut self,
        client_pos: elidex_plugin::Point<f64>,
        placement: ContentAreaPlacement,
    ) {
        let content = Self::cursor_to_content(client_pos, placement);
        if Self::point_in_content(content, placement) {
            self.cursor_in_content = true;
            let placement_seq = self.current_placement_seq();
            self.send_to_content(BrowserToContent::MouseMove {
                point: content.to_f32(),
                client_point: content,
                placement_seq,
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
            let content = Self::cursor_to_content(cursor, placement);
            if Self::point_in_content(content, placement) {
                let mods = Self::to_modifier_state(self.modifiers.state());
                let placement_seq = self.current_placement_seq();
                self.send_to_content(BrowserToContent::MouseClick(crate::ipc::MouseClickEvent {
                    point: content.to_f32(),
                    client_point: content,
                    button: winit_button_to_dom(button),
                    mods,
                    placement_seq,
                }));
            }
        }
    }

    /// Resolve the content-area scroll target for a wheel event, or `None` to
    /// drop it. With a **known** cursor the in-content gate applies (a cursor over
    /// chrome must not scroll the unpainted page under it — the hover/click rule
    /// applied to the wheel). But before the first `CursorMoved` (`cursor_pos` is
    /// `None` — e.g. the first input after focus is a wheel), fall back to the
    /// content origin so the page still scrolls: `content::scroll::handle_wheel`
    /// scrolls the viewport without a hit target, so dropping the wheel here would
    /// regress wheel scrolling until the pointer is first moved.
    fn wheel_target_point(
        cursor_pos: Option<elidex_plugin::Point<f64>>,
        placement: ContentAreaPlacement,
    ) -> Option<elidex_plugin::Point> {
        match cursor_pos {
            Some(cursor) => {
                let content = Self::cursor_to_content(cursor, placement);
                Self::point_in_content(content, placement).then(|| content.to_f32())
            }
            None => Some(elidex_plugin::Point::ZERO),
        }
    }

    /// Handle `MouseWheel` in threaded mode.
    ///
    /// Converts winit scroll deltas to CSS px ([`scroll_delta_to_css`]) and sends
    /// to the content thread at the [`wheel_target_point`] (skipped only when a
    /// known cursor is over chrome).
    fn handle_mouse_wheel_threaded(
        &mut self,
        delta: MouseScrollDelta,
        placement: ContentAreaPlacement,
    ) {
        let scroll_delta = Self::scroll_delta_to_css(delta, placement.scale_factor);
        if let Some(point) = Self::wheel_target_point(self.cursor_pos, placement) {
            let placement_seq = self.current_placement_seq();
            self.send_to_content(BrowserToContent::MouseWheel {
                delta: scroll_delta,
                point,
                placement_seq,
            });
        }
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
        // Born at the real viewport (C1): the new content thread reads the shared
        // `viewport_cell` (this `Arc`-clone — a disjoint `self.viewport.viewport_cell`
        // read that coexists with `&mut mgr`) at build time. Post-`resumed` the cell
        // holds the window's published size. It is keyed to the *active* tab's chrome;
        // exact while every tab uses the default (`Top`) tab-bar position — the only
        // position ever assigned (`chrome::ChromeState::new`). A future per-tab
        // non-`Top` position would make the active tab's content size differ from this
        // new (default-chrome) tab's → slot #11-window-level-tab-bar-position.
        let viewport_cell = std::sync::Arc::clone(&self.viewport.viewport_cell);
        let thread =
            crate::content::spawn_content_thread_blank(content_ch, nh, jar, viewport_cell, wake);
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
                assert!(
                    (back.x - f64::from(c.x)).abs() < 1e-3,
                    "x {} != {}",
                    back.x,
                    c.x
                );
                assert!(
                    (back.y - f64::from(c.y)).abs() < 1e-3,
                    "y {} != {}",
                    back.y,
                    c.y
                );
            }
        }
    }

    /// `client_point` (DOM `clientX`/`clientY`) and the hit-test `point` are the
    /// **same** content-area-relative CSS-px coordinate — both come from
    /// `cursor_to_content`, so `clientX`/`clientY` are viewport-relative (chrome
    /// `origin_logical` subtracted) and scale-divided, not window-relative and
    /// not raw-physical. At a top chrome of 64 CSS px and scale 2, a physical
    /// cursor over content CSS `(10, 10)` yields `client_point (10, 10)` — not
    /// `(10, 74)` (origin kept) and not `(20, 148)` (raw physical).
    #[test]
    fn client_point_is_content_area_relative_css_px() {
        // Top chrome 64 CSS px, scale 2. Content CSS (10, 10) is painted at
        // physical ((10+0), (10+64)) × 2 = (20, 148).
        let pl = placement(Point::new(0.0, 64.0), 2.0);
        let phys = Point::<f64>::new(20.0, 148.0);
        let client = App::cursor_to_content(phys, pl);
        assert!((client.x - 10.0).abs() < 1e-9, "clientX {} != 10", client.x);
        assert!((client.y - 10.0).abs() < 1e-9, "clientY {} != 10", client.y);
    }

    /// A point past the right/bottom edge of the placed content rect is rejected
    /// (the compositor clips the page to `size_logical`, so it is over chrome,
    /// not painted content) — e.g. a `TabBarPosition::Right` sidebar at `x ≥
    /// width`. The lower edge `[0, …)` is accepted, the upper edge `size_logical`
    /// is excluded (half-open `[0, size)`).
    #[test]
    fn point_in_content_rejects_outside_placed_rect() {
        // 800×600 content rect (the `placement` helper's size_logical).
        let pl = placement(Point::new(0.0, 0.0), 1.0);
        assert!(App::point_in_content(Point::<f64>::new(0.0, 0.0), pl)); // top-left in
        assert!(App::point_in_content(Point::<f64>::new(799.0, 599.0), pl)); // inside
        assert!(!App::point_in_content(Point::<f64>::new(-1.0, 10.0), pl)); // left of origin
        assert!(!App::point_in_content(Point::<f64>::new(10.0, -1.0), pl)); // above origin
        assert!(!App::point_in_content(Point::<f64>::new(800.0, 10.0), pl)); // right edge (Right sidebar)
        assert!(!App::point_in_content(Point::<f64>::new(10.0, 600.0), pl)); // bottom edge
    }

    /// `PixelDelta` scroll is physical px → divided by `scale_factor` to CSS px
    /// (identity at scale 1, halved at scale 2), so HiDPI scroll distance matches
    /// the cursor mapping. `LineDelta` (40 px/line) is scale-independent. Both are
    /// negated (winit → browser scrollTop sign).
    #[test]
    fn scroll_delta_to_css_divides_pixeldelta_by_scale() {
        use winit::dpi::PhysicalPosition;
        use winit::event::MouseScrollDelta;
        // PixelDelta at scale 1: identity (negated).
        let d1 = App::scroll_delta_to_css(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(40.0, 80.0)),
            1.0,
        );
        assert!((d1.x + 40.0).abs() < 1e-9 && (d1.y + 80.0).abs() < 1e-9);
        // PixelDelta at scale 2: physical (40, 80) → CSS (20, 40), negated.
        let d2 = App::scroll_delta_to_css(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(40.0, 80.0)),
            2.0,
        );
        assert!((d2.x + 20.0).abs() < 1e-9 && (d2.y + 40.0).abs() < 1e-9);
        // LineDelta is scale-independent: 1 line → 40 CSS px, negated.
        let dl = App::scroll_delta_to_css(MouseScrollDelta::LineDelta(0.0, 1.0), 2.0);
        assert!((dl.y + 40.0).abs() < 1e-9);
    }

    /// Wheel target resolution: a known cursor inside the content rect scrolls at
    /// that content point; a known cursor over chrome (outside the rect) drops the
    /// wheel (`None`); but an **unknown** cursor (`None`, before the first
    /// `CursorMoved`) falls back to the content origin so the page still scrolls —
    /// the early-`None`-return regression guard.
    #[test]
    fn wheel_target_point_falls_back_before_first_cursor_move() {
        let pl = placement(Point::new(0.0, 64.0), 1.0); // 800×600 content rect
                                                        // Unknown cursor → fall back to origin (still scrolls), NOT dropped.
        assert_eq!(
            App::wheel_target_point(None, pl),
            Some(Point::new(0.0, 0.0))
        );
        // Known cursor inside content (physical (10, 74) → content (10, 10)).
        assert_eq!(
            App::wheel_target_point(Some(Point::<f64>::new(10.0, 74.0)), pl),
            Some(Point::new(10.0, 10.0))
        );
        // Known cursor over chrome (physical y=10 < origin 64 → content y=-54) → drop.
        assert_eq!(
            App::wheel_target_point(Some(Point::<f64>::new(10.0, 10.0)), pl),
            None
        );
    }

    /// A resize that shrinks the content rect under a stationary cursor must be
    /// detected so stuck `:hover` is cleared. `cursor_left_content` is true only
    /// when a known cursor falls outside the *new* placement's content rect.
    #[test]
    fn cursor_left_content_detects_shrunk_rect_under_cursor() {
        // Cursor at content (700, 500) under an 800×600 rect — inside.
        let cursor = Point::<f64>::new(700.0, 500.0); // origin 0, scale 1 → content == cursor
        let big = placement(Point::new(0.0, 0.0), 1.0); // 800×600
        assert!(!App::cursor_left_content(Some(cursor), big));
        // Shrink the content rect to 400×300 (e.g. window narrowed): the cursor
        // is now outside → it "left" content.
        let small = ContentAreaPlacement {
            origin_logical: Point::new(0.0, 0.0),
            size_logical: Size::new(400.0, 300.0),
            scale_factor: 1.0,
        };
        assert!(App::cursor_left_content(Some(cursor), small));
        // No cursor → never "left".
        assert!(!App::cursor_left_content(None, small));
    }
}
