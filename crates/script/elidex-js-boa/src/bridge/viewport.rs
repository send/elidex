//! Viewport and scroll state methods for `HostBridge`.

use super::HostBridge;

impl HostBridge {
    /// Update cached viewport dimensions (called by content thread on `SetViewport`).
    pub fn set_viewport(&self, width: f32, height: f32) {
        let mut inner = self.inner.borrow_mut();
        inner.viewport_width = width;
        inner.viewport_height = height;
    }

    /// Get cached viewport width.
    pub fn viewport_width(&self) -> f32 {
        self.inner.borrow().viewport_width
    }

    /// Get cached viewport height.
    pub fn viewport_height(&self) -> f32 {
        self.inner.borrow().viewport_height
    }

    /// Update cached scroll offset (called by content thread before re-render).
    pub fn set_scroll_offset(&self, x: f32, y: f32) {
        let mut inner = self.inner.borrow_mut();
        inner.scroll_x = x;
        inner.scroll_y = y;
    }

    /// Get cached horizontal scroll offset.
    pub fn scroll_x(&self) -> f32 {
        self.inner.borrow().scroll_x
    }

    /// Get cached vertical scroll offset.
    pub fn scroll_y(&self) -> f32 {
        self.inner.borrow().scroll_y
    }

    /// Set a pending scroll offset from JS `scrollTo`/`scrollBy`.
    ///
    /// The content thread picks this up on the next frame and applies it
    /// to the viewport scroll state, then syncs back via `set_scroll_offset`.
    pub fn set_pending_scroll(&self, x: f32, y: f32) {
        self.inner.borrow_mut().pending_scroll = Some((x, y));
    }

    /// Take (remove) the pending scroll offset, if any.
    pub fn take_pending_scroll(&self) -> Option<(f32, f32)> {
        self.inner.borrow_mut().pending_scroll.take()
    }
}
