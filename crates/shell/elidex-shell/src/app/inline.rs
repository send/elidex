//! Legacy inline mode event handling for the `App`.
//!
//! All processing runs on the main thread (used by `build_pipeline` test API).

use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use elidex_ecs::ElementState as DomElementState;
use elidex_render::DisplayList;

use super::hover;
use super::render::handle_redraw;
use super::App;

impl App {
    pub(super) fn handle_window_event_inline(
        &mut self,
        _event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) {
        // Always process state-tracking events before egui routing.
        match &event {
            WindowEvent::ModifiersChanged(new_modifiers) => {
                if let Some(interactive) = &mut self.interactive {
                    interactive.modifiers = *new_modifiers;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_move_inline(position.x, position.y);
            }
            WindowEvent::CursorLeft { .. } => {
                self.handle_cursor_left_inline();
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
                self.handle_redraw_inline();
            }
            _ if egui_consumed => {}
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                self.handle_mouse_press_inline(button);
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            } => {
                self.handle_mouse_release_inline();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.handle_keyboard_inline(&key_event, address_focused);
            }
            _ => {}
        }
    }

    /// Handle `CursorMoved` in legacy inline mode (hover tracking).
    fn handle_cursor_move_inline(&mut self, px: f64, py: f64) {
        let hover_changed = if let Some(interactive) = &mut self.interactive {
            interactive.cursor_pos = Some((px, py));

            #[allow(clippy::cast_possible_truncation)]
            let (x, y) = (px as f32, (py as f32) - crate::chrome::CHROME_HEIGHT);
            let new_chain = if y >= 0.0 {
                elidex_layout::hit_test(&interactive.pipeline.dom, (x, y))
                    .map(|hit| hover::collect_hover_chain(&interactive.pipeline.dom, hit.entity))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            if new_chain == interactive.hover_chain {
                false
            } else {
                let old_chain = std::mem::take(&mut interactive.hover_chain);
                hover::apply_hover_diff(&mut interactive.pipeline.dom, &old_chain, &new_chain);
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

    /// Handle `CursorLeft` in legacy inline mode (clear hover/active state).
    fn handle_cursor_left_inline(&mut self) {
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

    /// Handle `RedrawRequested` in legacy inline mode.
    fn handle_redraw_inline(&mut self) {
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
        if let (Some(state), Some(interactive)) = (&mut self.render_state, &self.interactive) {
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

    /// Handle `MouseInput::Pressed` in legacy inline mode.
    fn handle_mouse_press_inline(&mut self, button: winit::event::MouseButton) {
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

    /// Handle `MouseInput::Released` in legacy inline mode.
    fn handle_mouse_release_inline(&mut self) {
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

    /// Handle `KeyboardInput` in legacy inline mode.
    fn handle_keyboard_inline(
        &mut self,
        key_event: &winit::event::KeyEvent,
        address_focused: bool,
    ) {
        if key_event.state == ElementState::Pressed {
            let mods = self
                .interactive
                .as_ref()
                .map(|i| i.modifiers.state())
                .unwrap_or_default();
            if mods.alt_key() {
                let nav_url = match &key_event.logical_key {
                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => self
                        .interactive
                        .as_mut()
                        .and_then(|i| i.nav_controller.go_back().cloned()),
                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => self
                        .interactive
                        .as_mut()
                        .and_then(|i| i.nav_controller.go_forward().cloned()),
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

        if key_event.state == ElementState::Pressed || key_event.state == ElementState::Released {
            let event_type = if key_event.state == ElementState::Pressed {
                "keydown"
            } else {
                "keyup"
            };
            let (key, code) =
                crate::key_map::winit_key_to_dom(&key_event.logical_key, key_event.physical_key);
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
}
