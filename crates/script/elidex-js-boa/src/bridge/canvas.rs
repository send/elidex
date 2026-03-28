//! Canvas 2D context methods for `HostBridge`.

use elidex_ecs::EcsDom;
use elidex_web_canvas::Canvas2dContext;

use super::HostBridge;

impl HostBridge {
    /// Get or create a Canvas 2D context for an entity.
    ///
    /// Returns `true` if a new context was created (first call for this entity).
    pub fn ensure_canvas_context(&self, entity_bits: u64, width: u32, height: u32) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.canvas_contexts.contains_key(&entity_bits) {
            return false;
        }
        if let Some(ctx) = Canvas2dContext::new(width, height) {
            inner.canvas_contexts.insert(entity_bits, ctx);
            true
        } else {
            false
        }
    }

    /// Access a canvas context for the duration of a closure.
    ///
    /// Returns `None` if no context exists for the entity.
    pub fn with_canvas<R>(
        &self,
        entity_bits: u64,
        f: impl FnOnce(&mut Canvas2dContext) -> R,
    ) -> Option<R> {
        let mut inner = self.inner.borrow_mut();
        inner.canvas_contexts.get_mut(&entity_bits).map(f)
    }

    /// Mark a canvas as dirty (modified since last frame sync).
    pub fn mark_canvas_dirty(&self, entity_bits: u64) {
        self.inner.borrow_mut().dirty_canvases.insert(entity_bits);
    }

    /// Sync all dirty canvas pixel buffers to their ECS `ImageData` components.
    ///
    /// Called once per frame from the content thread loop, replacing per-draw-call syncs.
    /// Takes `&mut EcsDom` directly so this can be called outside of JS eval context
    /// (no `bind()` required).
    pub fn sync_dirty_canvases(&self, dom: &mut EcsDom) {
        let dirty: Vec<u64> = {
            let mut inner = self.inner.borrow_mut();
            inner.dirty_canvases.drain().collect()
        };
        for entity_bits in dirty {
            let Some((width, height, pixels)) = self.with_canvas(entity_bits, |ctx| {
                (ctx.width(), ctx.height(), ctx.to_rgba8_straight())
            }) else {
                continue;
            };
            let image_data = elidex_ecs::ImageData {
                pixels: std::sync::Arc::new(pixels),
                width,
                height,
            };
            let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) else {
                continue;
            };
            let _ = dom.world_mut().insert_one(entity, image_data);
        }
    }
}
