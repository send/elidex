//! GPU initialization and frame rendering.

use std::sync::Arc;

use accesskit::ActionRequest;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;

use elidex_render::DisplayList;
use wgpu::{Instance, InstanceDescriptor};

use super::RenderState;

/// Create the window, GPU context, and Vello renderer.
///
/// Returns `None` (with a message printed to stderr) if any step fails.
pub(super) fn try_init_render_state(event_loop: &ActiveEventLoop) -> Option<RenderState> {
    // Window must be initially invisible for AccessKit adapter initialization.
    let window = event_loop
        .create_window(
            WindowAttributes::default()
                .with_title("elidex")
                .with_visible(false)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    crate::DEFAULT_VIEWPORT_WIDTH,
                    crate::DEFAULT_VIEWPORT_HEIGHT,
                )),
        )
        .inspect_err(|e| eprintln!("Failed to create window: {e}"))
        .ok()
        .map(Arc::new)?;

    // Initialize AccessKit adapter before showing the window.
    let a11y_adapter = accesskit_winit::Adapter::with_direct_handlers(
        event_loop,
        &window,
        NoopActivationHandler,
        NoopActionHandler,
        NoopDeactivationHandler,
    );

    // Now show the window.
    window.set_visible(true);

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

    let renderer = elidex_render::VelloRenderer::new(&gpu.device)
        .inspect_err(|e| eprintln!("Failed to create Vello renderer: {e}"))
        .ok()?;
    let blitter = wgpu::util::TextureBlitter::new(&gpu.device, gpu.surface_format);

    // egui initialization.
    let egui_ctx = egui::Context::default();
    #[allow(clippy::cast_possible_truncation)]
    let scale = window.scale_factor() as f32;
    let egui_state = egui_winit::State::new(
        egui_ctx.clone(),
        egui::ViewportId::ROOT,
        &*window,
        Some(scale),
        None,
        Some(gpu.device.limits().max_texture_dimension_2d as usize),
    );
    let egui_renderer = egui_wgpu::Renderer::new(
        &gpu.device,
        gpu.surface_format,
        egui_wgpu::RendererOptions {
            depth_stencil_format: None,
            msaa_samples: 1,
            dithering: true,
            ..Default::default()
        },
    );

    Some(RenderState {
        window,
        _instance: instance,
        surface,
        gpu,
        renderer,
        blitter,
        egui_ctx,
        egui_state,
        egui_renderer,
        a11y_adapter,
    })
}

/// Stub activation handler — returns `None` so the platform adapter
/// uses a placeholder tree until we send the first real update.
struct NoopActivationHandler;

impl accesskit::ActivationHandler for NoopActivationHandler {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        None
    }
}

/// Stub action handler — logs requests but doesn't act on them (MVP).
///
/// TODO(Phase 4): Handle Focus and other action requests from ATs.
struct NoopActionHandler;

impl accesskit::ActionHandler for NoopActionHandler {
    fn do_action(&mut self, _request: ActionRequest) {
        // MVP: ignore AT action requests.
    }
}

/// Stub deactivation handler — nothing to clean up.
struct NoopDeactivationHandler;

impl accesskit::DeactivationHandler for NoopDeactivationHandler {
    fn deactivate_accessibility(&mut self) {
        // No-op.
    }
}

/// Build and render the egui chrome overlay on top of existing surface content.
///
/// Runs the chrome UI, tessellates, uploads textures, and draws via a
/// `LoadOp::Load` render pass to preserve the Vello blit underneath.
pub(super) fn render_egui_overlay(
    state: &mut RenderState,
    encoder: &mut wgpu::CommandEncoder,
    frame_view: &wgpu::TextureView,
    chrome: &mut crate::chrome::ChromeState,
    can_go_back: bool,
    can_go_forward: bool,
) -> Option<crate::chrome::ChromeAction> {
    let width = state.gpu.surface_config.width;
    let height = state.gpu.surface_config.height;
    let mut chrome_action = None;
    let raw_input = state.egui_state.take_egui_input(&state.window);
    let full_output = state.egui_ctx.run(raw_input, |ctx| {
        chrome_action = chrome.build(ctx, can_go_back, can_go_forward);
    });

    state
        .egui_state
        .handle_platform_output(&state.window, full_output.platform_output);

    let clipped_primitives = state
        .egui_ctx
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point: full_output.pixels_per_point,
    };

    for (id, image_delta) in &full_output.textures_delta.set {
        state
            .egui_renderer
            .update_texture(&state.gpu.device, &state.gpu.queue, *id, image_delta);
    }

    let user_cmd_bufs = state.egui_renderer.update_buffers(
        &state.gpu.device,
        &state.gpu.queue,
        encoder,
        &clipped_primitives,
        &screen_descriptor,
    );
    if !user_cmd_bufs.is_empty() {
        state.gpu.queue.submit(user_cmd_bufs);
    }

    // `forget_lifetime()` erases the render pass lifetime (safe since
    // wgpu 22+ render passes don't actually borrow the encoder).
    {
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            })
            .forget_lifetime();

        state
            .egui_renderer
            .render(&mut render_pass, &clipped_primitives, &screen_descriptor);
    }

    for id in &full_output.textures_delta.free {
        state.egui_renderer.free_texture(id);
    }

    chrome_action
}

/// Render the display list to the window surface, optionally with an egui chrome overlay.
///
/// When `chrome` is provided, the chrome UI is built and rendered on top of
/// the Vello content using `LoadOp::Load` to preserve the blit output.
/// Returns any [`ChromeAction`] requested by the user.
pub(super) fn handle_redraw(
    state: &mut RenderState,
    display_list: &DisplayList,
    chrome: Option<&mut crate::chrome::ChromeState>,
    can_go_back: bool,
    can_go_forward: bool,
) -> Option<crate::chrome::ChromeAction> {
    let width = state.gpu.surface_config.width;
    let height = state.gpu.surface_config.height;

    if width == 0 || height == 0 {
        return None;
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
            return None;
        }
    };

    // Get the surface frame.
    let frame = match state.surface.get_current_texture() {
        Ok(f) => f,
        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
            state.gpu.reconfigure(&state.surface);
            state.window.request_redraw();
            return None;
        }
        Err(wgpu::SurfaceError::Timeout) => {
            state.window.request_redraw();
            return None;
        }
        Err(e) => {
            eprintln!("Surface error: {e}");
            return None;
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

    // Render egui chrome overlay if present.
    let chrome_action = if let Some(chrome) = chrome {
        render_egui_overlay(
            state,
            &mut encoder,
            &frame_view,
            chrome,
            can_go_back,
            can_go_forward,
        )
    } else {
        None
    };

    state.gpu.queue.submit(std::iter::once(encoder.finish()));
    frame.present();

    chrome_action
}
