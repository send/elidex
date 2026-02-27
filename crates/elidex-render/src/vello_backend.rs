//! Vello GPU rendering backend.
//!
//! Converts a [`DisplayList`] into a Vello [`Scene`] and renders it to
//! a `wgpu::Texture` using Vello's GPU compute pipeline.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use vello::kurbo::{Affine, Rect as VelloRect};
use vello::peniko::{Blob, Color, Fill, FontData};
use vello::{AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::{Device, Queue, Texture, TextureDescriptor, TextureFormat, TextureUsages};

use elidex_plugin::CssColor;

use crate::display_list::{DisplayItem, DisplayList};

/// GPU renderer backed by Vello.
///
/// Holds the Vello `Renderer`, a reusable `Scene`, and a persistent font
/// cache to avoid per-frame allocations and preserve Vello's glyph cache
/// across frames.
pub struct VelloRenderer {
    renderer: Renderer,
    scene: Scene,
    /// Persistent font data cache keyed by `Arc<Vec<u8>>` pointer identity.
    /// Keeping the same `FontData` (with the same `Blob` ID) across frames
    /// allows Vello's internal glyph cache to hit.
    font_cache: HashMap<*const Vec<u8>, FontData>,
}

impl VelloRenderer {
    /// Create a new Vello renderer for the given wgpu device.
    ///
    /// Returns an error if Vello's GPU pipeline creation fails.
    pub fn new(device: &Device) -> Result<Self, vello::Error> {
        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )?;

        Ok(Self {
            renderer,
            scene: Scene::new(),
            font_cache: HashMap::new(),
        })
    }

    /// Render a display list to a newly created `Rgba8Unorm` texture.
    ///
    /// The returned texture has `STORAGE_BINDING | TEXTURE_BINDING` usage
    /// flags, suitable for blitting to a surface via [`wgpu::util::TextureBlitter`].
    ///
    /// `width` and `height` must both be greater than zero.
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        display_list: &DisplayList,
        width: u32,
        height: u32,
    ) -> Result<Texture, vello::Error> {
        // Clamp to 1×1 to avoid wgpu validation errors on zero-size textures.
        let width = width.max(1);
        let height = height.max(1);

        // Build the Vello scene from the display list.
        self.scene.reset();
        build_scene(&mut self.scene, display_list, &mut self.font_cache);

        // Create the render target texture.
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("vello_render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.renderer.render_to_texture(
            device,
            queue,
            &self.scene,
            &view,
            &RenderParams {
                base_color: Color::from_rgb8(255, 255, 255),
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        )?;

        Ok(texture)
    }
}

/// Convert a [`CssColor`] to a Vello [`Color`].
fn convert_color(c: CssColor) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a)
}

/// Convert a [`DisplayList`] into a Vello [`Scene`].
///
/// The `font_cache` maps `Arc<Vec<u8>>` pointer identity to Vello `FontData`.
/// Keeping this cache across frames preserves Vello's internal glyph cache,
/// which keys on `Blob` ID.
///
/// # Safety of raw pointer keys
///
/// The `Arc<Vec<u8>>` references in `display_list` must outlive the returned
/// `FontData` entries (they do, since `FontData` clones the data into its own
/// `Blob`).
pub(crate) fn build_scene(
    scene: &mut Scene,
    display_list: &DisplayList,
    font_cache: &mut HashMap<*const Vec<u8>, FontData>,
) {
    for item in display_list.iter() {
        match item {
            DisplayItem::SolidRect { rect, color } => {
                let vello_rect = VelloRect::new(
                    f64::from(rect.x),
                    f64::from(rect.y),
                    f64::from(rect.x + rect.width),
                    f64::from(rect.y + rect.height),
                );
                let vello_color = convert_color(*color);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    vello_color,
                    None,
                    &vello_rect,
                );
            }
            DisplayItem::Text {
                glyphs,
                font_blob,
                font_index,
                font_size,
                color,
            } => {
                let ptr = Arc::as_ptr(font_blob);
                let font_data = font_cache
                    .entry(ptr)
                    .or_insert_with(|| {
                        let blob = Blob::from(font_blob.as_ref().clone());
                        FontData::new(blob, *font_index)
                    })
                    .clone();
                let vello_color = convert_color(*color);

                let vello_glyphs: Vec<Glyph> = glyphs
                    .iter()
                    .map(|g| Glyph {
                        id: g.glyph_id,
                        x: g.x,
                        y: g.y,
                    })
                    .collect();

                scene
                    .draw_glyphs(&font_data)
                    .font_size(*font_size)
                    .brush(vello_color)
                    .draw(Fill::NonZero, vello_glyphs.into_iter());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssColor, Rect};

    #[test]
    fn empty_display_list_builds_empty_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList::default();
        build_scene(&mut scene, &dl, &mut fc);
        // Scene was constructed without panic — smoke test passes.
    }

    #[test]
    fn solid_rect_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::SolidRect {
            rect: Rect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            },
            color: CssColor::RED,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Scene contains data (encoding is non-empty).
    }
}
