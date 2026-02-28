//! winit application handler for the elidex browser shell.
//!
//! Implements [`ApplicationHandler`] to manage the window lifecycle,
//! GPU initialization, frame rendering via Vello, and user input
//! event dispatch to the DOM.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Modifiers, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use elidex_ecs::Entity;
use elidex_layout::hit_test;
use elidex_plugin::{EventPayload, KeyboardEventInit, MouseEventInit};
use elidex_render::{DisplayList, VelloRenderer};
use elidex_script_session::DispatchEvent;
use wgpu::util::TextureBlitter;
use wgpu::{Instance, InstanceDescriptor, Surface};

use crate::gpu::GpuContext;
use crate::PipelineResult;

/// Render state initialized after the window is created.
struct RenderState {
    window: Arc<Window>,
    /// Kept alive as a precaution. While wgpu 27's `Surface` does not
    /// reference the `Instance` directly, keeping it alive ensures
    /// correctness if future wgpu versions change this.
    _instance: Instance,
    surface: Surface<'static>,
    gpu: GpuContext,
    renderer: VelloRenderer,
    blitter: TextureBlitter,
}

/// Interactive state holding all data needed for event handling and re-rendering.
struct InteractiveState {
    pipeline: PipelineResult,
    cursor_pos: Option<(f64, f64)>,
    focus_target: Option<Entity>,
    modifiers: Modifiers,
}

/// winit application that renders a display list to a window.
pub struct App {
    display_list: DisplayList,
    render_state: Option<RenderState>,
    interactive: Option<InteractiveState>,
}

impl App {
    /// Create a new interactive application from a pipeline result.
    pub fn new_interactive(pipeline: PipelineResult) -> Self {
        let display_list = pipeline.display_list.clone();
        Self {
            display_list,
            render_state: None,
            interactive: Some(InteractiveState {
                pipeline,
                cursor_pos: None,
                focus_target: None,
                modifiers: Modifiers::default(),
            }),
        }
    }

    /// Handle a mouse click event.
    ///
    /// Phase 2 simplification: mousedown, mouseup, and click are all
    /// dispatched synchronously on button press. Per DOM spec, mouseup
    /// should fire on button *release* (which may target a different
    /// element if the cursor moved), and click should only fire if
    /// press and release hit the same element.
    // TODO(Phase 3): split into handle_mouse_down / handle_mouse_up,
    // track press target per button, and dispatch mouseup on release.
    #[allow(clippy::cast_possible_truncation)]
    fn handle_click(&mut self, button: MouseButton) {
        let Some(interactive) = &mut self.interactive else {
            return;
        };
        let Some((cx, cy)) = interactive.cursor_pos else {
            return;
        };
        let x = cx as f32;
        let y = cy as f32;

        let pipeline = &mut interactive.pipeline;
        let Some(hit) = hit_test(&pipeline.dom, x, y) else {
            return;
        };

        // Update focus target on any click.
        interactive.focus_target = Some(hit.entity);

        // DOM spec: 0=primary, 1=auxiliary, 2=secondary, 3=back, 4=forward.
        let button_num = match button {
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            MouseButton::Back => 3,
            MouseButton::Forward => 4,
            MouseButton::Left | MouseButton::Other(_) => 0,
        };

        let mods = interactive.modifiers.state();
        let mouse_init = MouseEventInit {
            client_x: cx,
            client_y: cy,
            button: button_num,
            alt_key: mods.alt_key(),
            ctrl_key: mods.control_key(),
            meta_key: mods.super_key(),
            shift_key: mods.shift_key(),
            ..Default::default()
        };

        // Dispatch mousedown, mouseup, and (for primary button only) click.
        // DOM spec: click fires only for the primary button (button 0).
        // TODO(Phase 3): check dispatch_event return value (preventDefault)
        // to suppress default actions (e.g. link navigation on click).
        // TODO(Phase 3): dispatch auxclick for non-primary buttons.
        let event_types: &[&str] = if button_num == 0 {
            &["mousedown", "mouseup", "click"]
        } else {
            &["mousedown", "mouseup"]
        };
        for event_type in event_types {
            let mut event = DispatchEvent::new(*event_type, hit.entity);
            event.payload = EventPayload::Mouse(mouse_init.clone());
            let _default_prevented = pipeline.runtime.dispatch_event(
                &mut event,
                &mut pipeline.session,
                &mut pipeline.dom,
                pipeline.document,
            );
        }

        // Re-render after event handling.
        crate::re_render(pipeline);
        self.display_list = pipeline.display_list.clone();
    }

    /// Handle a keyboard event.
    fn handle_keyboard(&mut self, event_type: &str, init: KeyboardEventInit) {
        let Some(interactive) = &mut self.interactive else {
            return;
        };
        let Some(target) = interactive.focus_target else {
            return;
        };

        let pipeline = &mut interactive.pipeline;
        if !pipeline.dom.contains(target) {
            interactive.focus_target = None;
            return;
        }

        let mut event = DispatchEvent::new(event_type, target);
        event.payload = EventPayload::Keyboard(init);

        let _default_prevented = pipeline.runtime.dispatch_event(
            &mut event,
            &mut pipeline.session,
            &mut pipeline.dom,
            pipeline.document,
        );

        crate::re_render(pipeline);
        self.display_list = pipeline.display_list.clone();
    }
}

/// Create the window, GPU context, and Vello renderer.
///
/// Returns `None` (with a message printed to stderr) if any step fails.
fn try_init_render_state(event_loop: &ActiveEventLoop) -> Option<RenderState> {
    let window = event_loop
        .create_window(
            WindowAttributes::default()
                .with_title("elidex")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    crate::DEFAULT_VIEWPORT_WIDTH,
                    crate::DEFAULT_VIEWPORT_HEIGHT,
                )),
        )
        .inspect_err(|e| eprintln!("Failed to create window: {e}"))
        .ok()
        .map(Arc::new)?;

    let instance = Instance::new(&InstanceDescriptor::default());
    let surface = instance
        .create_surface(Arc::clone(&window))
        .inspect_err(|e| eprintln!("Failed to create wgpu surface: {e}"))
        .ok()?;

    let size = window.inner_size();
    let gpu = crate::gpu::create_gpu_context(&instance, &surface, size.width, size.height)
        .or_else(|| {
            eprintln!("Failed to initialize GPU context (no compatible adapter or device)");
            None
        })?;

    let renderer = VelloRenderer::new(&gpu.device)
        .inspect_err(|e| eprintln!("Failed to create Vello renderer: {e}"))
        .ok()?;
    let blitter = TextureBlitter::new(&gpu.device, gpu.surface_format);

    Some(RenderState {
        window,
        _instance: instance,
        surface,
        gpu,
        renderer,
        blitter,
    })
}

/// Render the display list to the window surface.
fn handle_redraw(state: &mut RenderState, display_list: &DisplayList) {
    let width = state.gpu.surface_config.width;
    let height = state.gpu.surface_config.height;

    if width == 0 || height == 0 {
        return;
    }

    // Render the display list to an intermediate texture.
    let texture = match state.renderer.render(
        &state.gpu.device,
        &state.gpu.queue,
        display_list,
        width,
        height,
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Vello render error: {e}");
            return;
        }
    };

    // Get the surface frame.
    let frame = match state.surface.get_current_texture() {
        Ok(f) => f,
        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
            state.gpu.reconfigure(&state.surface);
            state.window.request_redraw();
            return;
        }
        Err(wgpu::SurfaceError::Timeout) => {
            // Transient: request another frame.
            state.window.request_redraw();
            return;
        }
        Err(e) => {
            eprintln!("Surface error: {e}");
            return;
        }
    };

    let frame_view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let source_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Blit the Vello output to the surface.
    let mut encoder = state
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("blit_encoder"),
        });
    state
        .blitter
        .copy(&state.gpu.device, &mut encoder, &source_view, &frame_view);
    state.gpu.queue.submit(std::iter::once(encoder.finish()));

    frame.present();
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

        state.window.request_redraw();
        self.render_state = Some(state);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Drop GPU resources when the application is suspended.
        self.render_state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.render_state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                state
                    .gpu
                    .resize(&state.surface, new_size.width, new_size.height);
                if new_size.width > 0 && new_size.height > 0 {
                    state.window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                if let Some(interactive) = &mut self.interactive {
                    interactive.modifiers = new_modifiers;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(interactive) = &mut self.interactive {
                    interactive.cursor_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                self.handle_click(button);
                if let Some(s) = &self.render_state {
                    s.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
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
                        &key_event.physical_key,
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
            WindowEvent::RedrawRequested => {
                handle_redraw(state, &self.display_list);
            }
            _ => {}
        }
    }
}
