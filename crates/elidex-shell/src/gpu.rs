//! GPU device initialization for wgpu.
//!
//! Provides synchronous wgpu device/queue creation using `pollster` to
//! block on the async adapter and device requests.

use wgpu::{
    Device, DeviceDescriptor, Instance, Queue, RequestAdapterOptions, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages,
};

/// GPU context holding wgpu device, queue, and surface configuration.
pub struct GpuContext {
    pub device: Device,
    pub queue: Queue,
    pub surface_config: SurfaceConfiguration,
    pub surface_format: TextureFormat,
}

/// Create a wgpu device and queue compatible with the given surface.
///
/// The `instance` must be the same one used to create `surface`.
/// Uses `pollster` to synchronously block on async wgpu operations.
///
/// Returns `None` if no compatible GPU adapter or device is found, or
/// if the surface reports no supported formats.
pub fn create_gpu_context(
    instance: &Instance,
    surface: &Surface<'_>,
    width: u32,
    height: u32,
) -> Option<GpuContext> {
    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(surface),
        force_fallback_adapter: false,
    }))
    .ok()?;

    let (device, queue) =
        pollster::block_on(adapter.request_device(&DeviceDescriptor::default())).ok()?;

    let capabilities = surface.get_capabilities(&adapter);
    let surface_format = capabilities
        .formats
        .iter()
        .copied()
        .find(|f| {
            matches!(
                f,
                TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb
            )
        })
        .or_else(|| capabilities.formats.first().copied())?;

    let surface_config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
    };

    surface.configure(&device, &surface_config);

    Some(GpuContext {
        device,
        queue,
        surface_config,
        surface_format,
    })
}
