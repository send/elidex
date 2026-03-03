//! winit application handler for the elidex browser shell.
//!
//! Implements [`ApplicationHandler`] to manage the window lifecycle,
//! GPU initialization, frame rendering via Vello, and user input
//! event dispatch to the DOM.

mod events;
mod hover;
mod navigation;
mod render;

use std::collections::HashSet;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Modifiers, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use elidex_ecs::{ElementState as DomElementState, Entity};
use elidex_layout::hit_test;
use elidex_navigation::NavigationController;
use elidex_plugin::KeyboardEventInit;
use elidex_render::DisplayList;
use wgpu::util::TextureBlitter;
use wgpu::{Instance, Surface};

use crate::PipelineResult;

use hover::{collect_hover_chain, update_element_state};
use render::{handle_redraw, try_init_render_state};

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
}

/// Interactive state holding all data needed for event handling and re-rendering.
pub(super) struct InteractiveState {
    pub(super) pipeline: PipelineResult,
    pub(super) cursor_pos: Option<(f64, f64)>,
    pub(super) focus_target: Option<Entity>,
    /// Entities in the current hover chain (hit entity + all ancestors).
    pub(super) hovered_entities: Vec<Entity>,
    /// Entities that received `:active` state on mouse press.
    pub(super) active_entities: Vec<Entity>,
    pub(super) modifiers: Modifiers,
    pub(super) nav_controller: NavigationController,
    pub(super) window_title: String,
    pub(super) chrome: crate::chrome::ChromeState,
}

/// winit application that renders a display list to a window.
pub struct App {
    pub(super) display_list: DisplayList,
    pub(super) render_state: Option<RenderState>,
    pub(super) interactive: Option<InteractiveState>,
}

impl App {
    /// Create a new interactive application from a pipeline result.
    pub fn new_interactive(pipeline: PipelineResult) -> Self {
        let display_list = pipeline.display_list.clone();
        Self {
            display_list,
            render_state: None,
            interactive: Some(InteractiveState {
                chrome: crate::chrome::ChromeState::new(None),
                pipeline,
                cursor_pos: None,
                focus_target: None,
                hovered_entities: Vec::new(),
                active_entities: Vec::new(),
                modifiers: Modifiers::default(),
                nav_controller: NavigationController::new(),
                window_title: "elidex".to_string(),
            }),
        }
    }

    /// Create a new interactive application from a URL-loaded pipeline result.
    pub fn new_interactive_with_url(pipeline: PipelineResult, title: String) -> Self {
        let display_list = pipeline.display_list.clone();
        let chrome = crate::chrome::ChromeState::new(pipeline.url.as_ref());
        let mut nav_controller = NavigationController::new();
        if let Some(url) = &pipeline.url {
            nav_controller.push(url.clone());
        }
        Self {
            display_list,
            render_state: None,
            interactive: Some(InteractiveState {
                chrome,
                pipeline,
                cursor_pos: None,
                focus_target: None,
                hovered_entities: Vec::new(),
                active_entities: Vec::new(),
                modifiers: Modifiers::default(),
                nav_controller,
                window_title: title,
            }),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_state.is_some() {
            return; // Already initialized.
        }

        let Some(state) = try_init_render_state(event_loop) else {
            event_loop.exit();
            return;
        };

        // Set initial window title from interactive state.
        if let Some(interactive) = &self.interactive {
            state.window.set_title(&interactive.window_title);
        }

        state.window.request_redraw();
        self.render_state = Some(state);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Drop GPU resources when the application is suspended.
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

        // Handle events that always need processing regardless of egui.
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(new_size) => {
                let state = self.render_state.as_mut().unwrap();
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

        // Always process state-tracking events before egui routing,
        // so content state stays consistent regardless of egui consumption.
        match &event {
            WindowEvent::ModifiersChanged(new_modifiers) => {
                if let Some(interactive) = &mut self.interactive {
                    interactive.modifiers = *new_modifiers;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let hover_changed = if let Some(interactive) = &mut self.interactive {
                    interactive.cursor_pos = Some((position.x, position.y));

                    // Update hover state: hit-test and compare with previous hover chain.
                    #[allow(clippy::cast_possible_truncation)]
                    let x = position.x as f32;
                    #[allow(clippy::cast_possible_truncation)]
                    let y = (position.y as f32) - crate::chrome::CHROME_HEIGHT;
                    let new_chain = if y >= 0.0 {
                        hit_test(&interactive.pipeline.dom, x, y)
                            .map(|hit| collect_hover_chain(&interactive.pipeline.dom, hit.entity))
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    if new_chain == interactive.hovered_entities {
                        false
                    } else {
                        let old_chain =
                            std::mem::replace(&mut interactive.hovered_entities, new_chain.clone());
                        let new_set: HashSet<Entity> = new_chain.iter().copied().collect();
                        let old_set: HashSet<Entity> = old_chain.iter().copied().collect();
                        // Remove HOVER from entities no longer hovered.
                        for &e in &old_chain {
                            if !new_set.contains(&e) {
                                update_element_state(&mut interactive.pipeline.dom, e, |s| {
                                    s.remove(DomElementState::HOVER);
                                });
                            }
                        }
                        // Add HOVER to newly hovered entities.
                        for &e in &new_chain {
                            if !old_set.contains(&e) {
                                update_element_state(&mut interactive.pipeline.dom, e, |s| {
                                    s.insert(DomElementState::HOVER);
                                });
                            }
                        }
                        // Re-render to apply :hover style changes.
                        crate::re_render(&mut interactive.pipeline);
                        true
                    }
                } else {
                    false
                };
                if hover_changed {
                    if let Some(interactive) = &self.interactive {
                        self.display_list = interactive.pipeline.display_list.clone();
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                // Clear all hover and active state when cursor leaves the window.
                if let Some(interactive) = &mut self.interactive {
                    let had_hover = !interactive.hovered_entities.is_empty();
                    let had_active = !interactive.active_entities.is_empty();
                    // Clear ACTIVE from press-time entities (may differ from hover chain).
                    for &e in &std::mem::take(&mut interactive.active_entities) {
                        update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    // Clear HOVER (and any remaining ACTIVE) from hover chain.
                    for &e in &std::mem::take(&mut interactive.hovered_entities) {
                        update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::HOVER);
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    if had_hover || had_active {
                        crate::re_render(&mut interactive.pipeline);
                    }
                }
                if let Some(interactive) = &self.interactive {
                    self.display_list = interactive.pipeline.display_list.clone();
                }
            }
            _ => {}
        }

        // Pass events to egui first (interactive mode only).
        let egui_consumed = if self.interactive.is_some() {
            let state = self.render_state.as_mut().unwrap();
            let response = state.egui_state.on_window_event(&state.window, &event);
            if response.repaint {
                state.window.request_redraw();
            }
            response.consumed
        } else {
            false
        };

        // Check if the address bar has focus (suppress content keyboard events).
        let address_focused = self
            .interactive
            .as_ref()
            .is_some_and(|i| i.chrome.address_focused);

        match event {
            WindowEvent::RedrawRequested => {
                // Use an inner block so field borrows are released before
                // handle_chrome_action needs `&mut self`.
                let chrome_action = {
                    let Self {
                        display_list,
                        render_state,
                        interactive,
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
                if let Some(action) = chrome_action {
                    self.handle_chrome_action(action);
                    if let Some(s) = &self.render_state {
                        s.window.request_redraw();
                    }
                }
            }
            _ if egui_consumed => {
                // egui consumed this event; skip content handling.
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                // Set ACTIVE state on hover chain and remember which
                // entities received it (cursor may move before release).
                if let Some(interactive) = &mut self.interactive {
                    interactive.active_entities = interactive.hovered_entities.clone();
                    for &e in &interactive.active_entities {
                        update_element_state(&mut interactive.pipeline.dom, e, |s| {
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
                // Clear ACTIVE state from the entities that were active at
                // press time (not the current hover chain, which may differ).
                if let Some(interactive) = &mut self.interactive {
                    let active = std::mem::take(&mut interactive.active_entities);
                    for &e in &active {
                        update_element_state(&mut interactive.pipeline.dom, e, |s| {
                            s.remove(DomElementState::ACTIVE);
                        });
                    }
                    crate::re_render(&mut interactive.pipeline);
                }
                if let Some(interactive) = &self.interactive {
                    self.display_list = interactive.pipeline.display_list.clone();
                }
                if let Some(s) = &self.render_state {
                    s.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Alt+Left/Right: back/forward navigation (always active,
                // even when the address bar is focused -- matches browser UX).
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

                // Address bar focused -- don't dispatch keyboard events to
                // content. Just request redraw for egui updates.
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
                    let init = KeyboardEventInit {
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
