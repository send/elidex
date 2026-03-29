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

    // --- Device/screen properties (Gaps 10-12) ---

    /// Get device pixel ratio.
    pub fn device_pixel_ratio(&self) -> f32 {
        self.inner.borrow().device_pixel_ratio
    }

    /// Set device pixel ratio (called by content thread from winit `scale_factor`).
    pub fn set_device_pixel_ratio(&self, dpr: f32) {
        self.inner.borrow_mut().device_pixel_ratio = dpr;
    }

    /// Get window screen position X.
    pub fn screen_x(&self) -> i32 {
        self.inner.borrow().screen_x
    }

    /// Get window screen position Y.
    pub fn screen_y(&self) -> i32 {
        self.inner.borrow().screen_y
    }

    /// Set window screen position (called by content thread from winit).
    pub fn set_screen_position(&self, x: i32, y: i32) {
        let mut inner = self.inner.borrow_mut();
        inner.screen_x = x;
        inner.screen_y = y;
    }

    /// Get monitor width in CSS pixels.
    pub fn monitor_width(&self) -> f32 {
        self.inner.borrow().monitor_width
    }

    /// Get monitor height in CSS pixels.
    pub fn monitor_height(&self) -> f32 {
        self.inner.borrow().monitor_height
    }

    /// Set monitor dimensions (called by content thread from winit).
    pub fn set_monitor_dimensions(&self, width: f32, height: f32) {
        let mut inner = self.inner.borrow_mut();
        inner.monitor_width = width;
        inner.monitor_height = height;
    }

    /// Get screen color depth in bits.
    pub fn color_depth(&self) -> u32 {
        self.inner.borrow().color_depth
    }

    /// Set screen color depth (called by content thread from GPU surface format).
    pub fn set_color_depth(&self, depth: u32) {
        self.inner.borrow_mut().color_depth = depth;
    }

    /// Set tab visibility state (called by content thread on `VisibilityChanged`).
    pub fn set_visibility(&self, visible: bool) {
        self.inner.borrow_mut().tab_hidden = !visible;
    }

    /// Returns `true` when the tab is hidden (not the active tab or window occluded).
    pub fn is_tab_hidden(&self) -> bool {
        self.inner.borrow().tab_hidden
    }

    // --- Window focus ---

    /// Request window focus (from `window.focus()`).
    ///
    /// Sets a pending flag that the content thread picks up and sends via IPC.
    pub fn request_focus(&self) {
        self.inner.borrow_mut().pending_focus = true;
    }

    /// Take (remove) the pending focus request, if any.
    pub fn take_pending_focus(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        let val = inner.pending_focus;
        inner.pending_focus = false;
        val
    }
}
