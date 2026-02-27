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
