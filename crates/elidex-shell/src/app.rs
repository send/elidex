//! winit application handler for the elidex browser shell.
//!
//! Implements [`ApplicationHandler`] to manage the window lifecycle,
//! GPU initialization, and frame rendering via Vello.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use elidex_render::{DisplayList, VelloRenderer};
use wgpu::util::TextureBlitter;
use wgpu::{Instance, InstanceDescriptor, Surface};

use crate::gpu::GpuContext;

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

/// winit application that renders a display list to a window.
pub struct App {
    display_list: DisplayList,
    render_state: Option<RenderState>,
}

impl App {
    /// Create a new application with the given display list to render.
    pub fn new(display_list: DisplayList) -> Self {
        Self {
            display_list,
            render_state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_state.is_some() {
            return; // Already initialized.
        }

        let window = match event_loop.create_window(
            WindowAttributes::default()
                .with_title("elidex")
                .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("Failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        let instance = Instance::new(&InstanceDescriptor::default());
        let surface = match instance.create_surface(Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create wgpu surface: {e}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let Some(gpu) =
            crate::gpu::create_gpu_context(&instance, &surface, size.width, size.height)
        else {
            eprintln!("Failed to initialize GPU context (no compatible adapter or device)");
            event_loop.exit();
            return;
        };

        let renderer = match VelloRenderer::new(&gpu.device) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to create Vello renderer: {e}");
                event_loop.exit();
                return;
            }
        };
        let blitter = TextureBlitter::new(&gpu.device, gpu.surface_format);

        self.render_state = Some(RenderState {
            window,
            _instance: instance,
            surface,
            gpu,
            renderer,
            blitter,
        });

        // Request initial redraw via the render state.
        if let Some(state) = &self.render_state {
            state.window.request_redraw();
        }
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
                if new_size.width > 0 && new_size.height > 0 {
                    state.gpu.surface_config.width = new_size.width;
                    state.gpu.surface_config.height = new_size.height;
                    state
                        .surface
                        .configure(&state.gpu.device, &state.gpu.surface_config);
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let width = state.gpu.surface_config.width;
                let height = state.gpu.surface_config.height;

                if width == 0 || height == 0 {
                    return;
                }

                // Render the display list to an intermediate texture.
                let texture = match state.renderer.render(
                    &state.gpu.device,
                    &state.gpu.queue,
                    &self.display_list,
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
                        state
                            .surface
                            .configure(&state.gpu.device, &state.gpu.surface_config);
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
                let mut encoder =
                    state
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
            _ => {}
        }
    }
}
