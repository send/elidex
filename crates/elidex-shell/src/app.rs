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

use elidex_ecs::{Attributes, Entity, TagType};
use elidex_layout::hit_test;
use elidex_navigation::NavigationController;
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
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

/// Interactive state holding all data needed for event handling and re-rendering.
struct InteractiveState {
    pipeline: PipelineResult,
    cursor_pos: Option<(f64, f64)>,
    focus_target: Option<Entity>,
    modifiers: Modifiers,
    nav_controller: NavigationController,
    window_title: String,
    chrome: crate::chrome::ChromeState,
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
                chrome: crate::chrome::ChromeState::new(None),
                pipeline,
                cursor_pos: None,
                focus_target: None,
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
                modifiers: Modifiers::default(),
                nav_controller,
                window_title: title,
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
    fn handle_click(&mut self, button: MouseButton) {
        let Some(interactive) = &mut self.interactive else {
            return;
        };
        let Some((cx, cy)) = interactive.cursor_pos else {
            return;
        };
        #[allow(clippy::cast_possible_truncation)]
        let x = cx as f32;
        // Offset Y by chrome bar height so hit testing is relative to content.
        #[allow(clippy::cast_possible_truncation)]
        let y = (cy as f32) - crate::chrome::CHROME_HEIGHT;
        if y < 0.0 {
            return; // Click is within the chrome bar.
        }

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
        // TODO(Phase 3): dispatch auxclick for non-primary buttons.
        let event_types: &[&str] = if button_num == 0 {
            &["mousedown", "mouseup", "click"]
        } else {
            &["mousedown", "mouseup"]
        };
        let mut click_prevented = false;
        for event_type in event_types {
            let mut event = DispatchEvent::new(*event_type, hit.entity);
            event.payload = EventPayload::Mouse(mouse_init.clone());
            let prevented = pipeline.runtime.dispatch_event(
                &mut event,
                &mut pipeline.session,
                &mut pipeline.dom,
                pipeline.document,
            );
            if *event_type == "click" {
                click_prevented = prevented;
            }
        }

        // Re-render after event handling.
        crate::re_render(pipeline);
        self.display_list = pipeline.display_list.clone();

        // Check for pending JS navigation (location.assign, etc.).
        if let Some(nav_req) = pipeline.runtime.take_pending_navigation() {
            let resolved = resolve_nav_url(pipeline.url.as_ref(), &nav_req.url);
            if let Some(target_url) = resolved {
                self.navigate(&target_url, nav_req.replace);
                return;
            }
        }

        // Check for pending JS history action.
        if let Some(action) = pipeline.runtime.take_pending_history() {
            self.handle_history_action(action);
            return;
        }

        // Link navigation: if click was not prevented, check for <a href>.
        let nav_target = if button_num == 0 && !click_prevented {
            find_link_ancestor(&pipeline.dom, hit.entity).and_then(|href| {
                if let Some(base_url) = &pipeline.url {
                    base_url.join(&href).ok()
                } else {
                    url::Url::parse(&href).ok()
                }
            })
        } else {
            None
        };

        if let Some(target_url) = nav_target {
            self.navigate(&target_url, false);
        }
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

        // TODO(Phase 3): Check default_prevented to suppress default keyboard actions.
        let _default_prevented = pipeline.runtime.dispatch_event(
            &mut event,
            &mut pipeline.session,
            &mut pipeline.dom,
            pipeline.document,
        );

        crate::re_render(pipeline);
        self.display_list = pipeline.display_list.clone();

        // Check for pending JS navigation or history action.
        if let Some(nav_req) = pipeline.runtime.take_pending_navigation() {
            let resolved = resolve_nav_url(pipeline.url.as_ref(), &nav_req.url);
            if let Some(target_url) = resolved {
                self.navigate(&target_url, nav_req.replace);
                return;
            }
        }
        if let Some(action) = pipeline.runtime.take_pending_history() {
            self.handle_history_action(action);
        }
    }

    /// Navigate to a new URL, replacing the current pipeline.
    ///
    /// When `replace` is `true`, the current history entry is replaced
    /// (matching `location.replace()` semantics). Otherwise a new entry
    /// is pushed onto the history stack.
    fn navigate(&mut self, url: &url::Url, replace: bool) {
        if !self.load_url_into_pipeline(url) {
            return;
        }
        let interactive = self.interactive.as_mut().unwrap();
        if replace {
            interactive.nav_controller.replace(url.clone());
        } else {
            interactive.nav_controller.push(url.clone());
        }
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        if let Some(state) = &self.render_state {
            state.window.set_title(&interactive.window_title);
        }
    }

    /// Navigate to a URL from the history (back/forward).
    fn navigate_to_history_url(&mut self, url: &url::Url) {
        if !self.load_url_into_pipeline(url) {
            return;
        }
        let interactive = self.interactive.as_mut().unwrap();
        interactive
            .pipeline
            .runtime
            .set_history_length(interactive.nav_controller.len());
        if let Some(state) = &self.render_state {
            state.window.set_title(&interactive.window_title);
        }
    }

    /// Load a URL into the current pipeline, updating interactive state.
    ///
    /// Shared by `navigate` and `navigate_to_history_url`.
    /// Returns `true` on success, `false` on error.
    fn load_url_into_pipeline(&mut self, url: &url::Url) -> bool {
        let Some(interactive) = &mut self.interactive else {
            return false;
        };
        let fetch_handle = std::rc::Rc::clone(&interactive.pipeline.fetch_handle);
        let font_db = std::rc::Rc::clone(&interactive.pipeline.font_db);
        match elidex_navigation::load_document(url, &fetch_handle) {
            Ok(loaded) => {
                let new_pipeline = crate::build_pipeline_from_loaded(loaded, fetch_handle, font_db);
                interactive.pipeline = new_pipeline;
                interactive.window_title = format!("elidex \u{2014} {url}");
                interactive.focus_target = None;
                interactive.chrome.set_url(url);
                self.display_list = interactive.pipeline.display_list.clone();
                true
            }
            Err(e) => {
                eprintln!("Navigation error: {e}");
                false
            }
        }
    }

    /// Handle a pending history action from JS.
    fn handle_history_action(&mut self, action: elidex_navigation::HistoryAction) {
        let Some(interactive) = &mut self.interactive else {
            return;
        };

        match action {
            elidex_navigation::HistoryAction::Back => {
                if let Some(url) = interactive.nav_controller.go_back().cloned() {
                    self.navigate_to_history_url(&url);
                }
            }
            elidex_navigation::HistoryAction::Forward => {
                if let Some(url) = interactive.nav_controller.go_forward().cloned() {
                    self.navigate_to_history_url(&url);
                }
            }
            elidex_navigation::HistoryAction::Go(delta) => {
                if let Some(url) = interactive.nav_controller.go(delta).cloned() {
                    self.navigate_to_history_url(&url);
                }
            }
            elidex_navigation::HistoryAction::PushState { url, .. } => {
                if let Some(resolved_url) =
                    resolve_state_url(interactive.pipeline.url.as_ref(), url.as_deref())
                {
                    apply_state_change(interactive, &resolved_url, false);
                    interactive.window_title = format!("elidex \u{2014} {resolved_url}");
                    if let Some(state) = &self.render_state {
                        state.window.set_title(&interactive.window_title);
                    }
                }
            }
            elidex_navigation::HistoryAction::ReplaceState { url, .. } => {
                if let Some(resolved_url) =
                    resolve_state_url(interactive.pipeline.url.as_ref(), url.as_deref())
                {
                    apply_state_change(interactive, &resolved_url, true);
                    interactive.window_title = format!("elidex \u{2014} {resolved_url}");
                    if let Some(state) = &self.render_state {
                        state.window.set_title(&interactive.window_title);
                    }
                }
            }
        }
    }
}

/// Resolve a `pushState`/`replaceState` URL, enforcing same-origin.
///
/// Per the History API spec, `pushState`/`replaceState` must not change the
/// origin. Returns `None` if the URL is cross-origin or cannot be parsed.
/// If `url_str` is `None`, returns the current URL (no URL change).
fn resolve_state_url(base: Option<&url::Url>, url_str: Option<&str>) -> Option<url::Url> {
    let Some(url_str) = url_str else {
        return base.cloned();
    };
    let resolved = resolve_nav_url(base, url_str)?;
    // Same-origin check: scheme + host + port must match.
    if let Some(current) = base {
        if current.origin() != resolved.origin() {
            eprintln!(
                "SecurityError: pushState/replaceState URL {resolved} has different origin than {current}"
            );
            return None;
        }
    }
    Some(resolved)
}

/// Apply a `pushState`/`replaceState` URL change to interactive state.
///
/// When `replace` is `true`, the current history entry is replaced;
/// otherwise a new entry is pushed.
fn apply_state_change(interactive: &mut InteractiveState, url: &url::Url, replace: bool) {
    interactive.pipeline.url = Some(url.clone());
    if replace {
        interactive.nav_controller.replace(url.clone());
    } else {
        interactive.nav_controller.push(url.clone());
    }
    interactive.chrome.set_url(url);
    interactive
        .pipeline
        .runtime
        .set_current_url(Some(url.clone()));
    interactive
        .pipeline
        .runtime
        .set_history_length(interactive.nav_controller.len());
}

/// Resolve a navigation URL string against the current page URL.
fn resolve_nav_url(base: Option<&url::Url>, url_str: &str) -> Option<url::Url> {
    if let Some(base_url) = base {
        base_url.join(url_str).ok()
    } else {
        url::Url::parse(url_str).ok()
    }
}

/// Find the nearest `<a href="...">` ancestor of an entity (including itself).
///
/// Depth-limited to 10,000 to guard against cycles (consistent with
/// `build_propagation_path` and other tree walkers in the codebase).
fn find_link_ancestor(dom: &elidex_ecs::EcsDom, entity: Entity) -> Option<String> {
    let mut current = Some(entity);
    let mut depth = 0;
    while let Some(e) = current {
        if depth > 10_000 {
            break;
        }
        if let Ok(tag) = dom.world().get::<&TagType>(e) {
            if tag.0 == "a" {
                if let Ok(attrs) = dom.world().get::<&Attributes>(e) {
                    if let Some(href) = attrs.get("href") {
                        if !href.is_empty() {
                            return Some(href.to_string());
                        }
                    }
                }
            }
        }
        current = dom.get_parent(e);
        depth += 1;
    }
    None
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
    })
}

/// Build and render the egui chrome overlay on top of existing surface content.
///
/// Runs the chrome UI, tessellates, uploads textures, and draws via a
/// `LoadOp::Load` render pass to preserve the Vello blit underneath.
fn render_egui_overlay(
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
fn handle_redraw(
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

impl App {
    /// Handle a chrome action (navigation, back, forward, reload).
    fn handle_chrome_action(&mut self, action: crate::chrome::ChromeAction) {
        match action {
            crate::chrome::ChromeAction::Navigate(url_str) => {
                // Try parsing as-is first, then with https:// prefix.
                let parsed = url::Url::parse(&url_str)
                    .or_else(|_| url::Url::parse(&format!("https://{url_str}")));
                match parsed {
                    // navigate() calls chrome.set_url() on success internally.
                    Ok(url) => self.navigate(&url, false),
                    Err(e) => eprintln!("Invalid URL: {e}"),
                }
            }
            crate::chrome::ChromeAction::Back => {
                let url = self
                    .interactive
                    .as_mut()
                    .and_then(|i| i.nav_controller.go_back().cloned());
                if let Some(url) = url {
                    self.navigate_to_history_url(&url);
                }
            }
            crate::chrome::ChromeAction::Forward => {
                let url = self
                    .interactive
                    .as_mut()
                    .and_then(|i| i.nav_controller.go_forward().cloned());
                if let Some(url) = url {
                    self.navigate_to_history_url(&url);
                }
            }
            crate::chrome::ChromeAction::Reload => {
                let url = self
                    .interactive
                    .as_ref()
                    .and_then(|i| i.pipeline.url.clone());
                if let Some(url) = url {
                    self.navigate(&url, true);
                }
            }
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
                if let Some(interactive) = &mut self.interactive {
                    interactive.cursor_pos = Some((position.x, position.y));
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
                self.handle_click(button);
                if let Some(s) = &self.render_state {
                    s.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Alt+Left/Right: back/forward navigation (always active,
                // even when the address bar is focused — matches browser UX).
                if key_event.state == ElementState::Pressed {
                    let mods = self
                        .interactive
                        .as_ref()
                        .map(|i| i.modifiers.state())
                        .unwrap_or_default();
                    if mods.alt_key() {
                        let nav_url = match &key_event.logical_key {
                            winit::keyboard::Key::Named(
                                winit::keyboard::NamedKey::ArrowLeft,
                            ) => self
                                .interactive
                                .as_mut()
                                .and_then(|i| i.nav_controller.go_back().cloned()),
                            winit::keyboard::Key::Named(
                                winit::keyboard::NamedKey::ArrowRight,
                            ) => self
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

                // Address bar focused — don't dispatch keyboard events to
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
