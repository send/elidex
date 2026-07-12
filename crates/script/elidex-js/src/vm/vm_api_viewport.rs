//! Public `Vm` API — the window-parity / viewport device-fact transport seam.
//!
//! Carved out of `vm_api.rs` (the >1000-line touch-time-split convention, Codex
//! R3) as the cohesive cluster the shell uses to push viewport / device facts
//! into the VM: the script-requested scroll offset + the live scroll offset
//! (CSSOM-View §4), the media environment (§4.2), the monitor dimensions (§4.3),
//! and the `VisualViewport` report-changes pass (§13.1). All business logic
//! lives in `VmInner`; these are thin delegators. Each backs the matching
//! `HostDriver` method (the shell call-sites ride the S5-6 flip; VM tests call
//! them directly).

use super::Vm;

impl Vm {
    /// Drain the pending script-requested scroll offset (CSSOM View §4),
    /// set by `window.scrollTo` / `scrollBy`, for the shell to apply to the
    /// real viewport. `None` when no script scroll is pending. Backs
    /// [`HostDriver::take_pending_scroll`](elidex_script_session::HostDriver::take_pending_scroll).
    pub fn take_pending_scroll(&mut self) -> Option<(f64, f64)> {
        self.inner.viewport.pending_scroll.take()
    }

    /// Non-consuming peek at whether a script-requested scroll (CSSOM View §4,
    /// `window.scrollTo` / `scrollBy`) is pending, WITHOUT draining it. The
    /// event loop uses this to schedule `re_render` (the sole drain point, which
    /// consumes via [`Self::take_pending_scroll`]) on a turn whose only visible
    /// effect is a scroll — a `scrollTo` bumps no DOM version, so the tree-delta
    /// render-dirty signal would otherwise miss it. Backs
    /// [`HostDriver::has_pending_scroll`](elidex_script_session::HostDriver::has_pending_scroll).
    #[must_use]
    pub fn has_pending_scroll(&self) -> bool {
        self.inner.viewport.pending_scroll.is_some()
    }

    /// Push the viewport's current scroll offset into the engine (CSSOM View
    /// §4) so `window.scrollX` / `scrollY` read the live value after a user
    /// (wheel/keyboard) scroll the shell applied. Backs
    /// [`HostDriver::set_scroll_offset`](elidex_script_session::HostDriver::set_scroll_offset).
    pub fn set_scroll_offset(&mut self, x: f64, y: f64) {
        self.inner.viewport.scroll_x = x;
        self.inner.viewport.scroll_y = y;
    }

    /// Push the window's media-query device facts (CSSOM-View §4.2) into the
    /// single `ViewportState` device-facts SoT, so `innerWidth` / `innerHeight`
    /// / `devicePixelRatio` and every live `MediaQueryList.matches` read the
    /// new values. A **pure state push** (no JS, no `change`) — the sibling of
    /// [`Self::set_scroll_offset`]; the shell runs
    /// [`Self::deliver_media_query_changes`] to report flips. Backs
    /// [`HostDriver::set_media_environment`](elidex_script_session::HostDriver::set_media_environment).
    pub fn set_media_environment(
        &mut self,
        viewport_width: f64,
        viewport_height: f64,
        device_pixel_ratio: f64,
        color_scheme: elidex_css::media::ColorScheme,
        reduced_motion: elidex_css::media::ReducedMotion,
    ) {
        let vp = &mut self.inner.viewport;
        vp.inner_width = viewport_width;
        vp.inner_height = viewport_height;
        vp.device_pixel_ratio = device_pixel_ratio;
        vp.color_scheme = color_scheme;
        vp.reduced_motion = reduced_motion;
    }

    /// Run the CSSOM-View §4.2 "evaluate media queries and report changes"
    /// pass — re-evaluate every live `MediaQueryList` against the current
    /// environment and fire `change` at each whose result flipped since the
    /// last delivery. Backs
    /// [`HostDriver::deliver_media_query_changes`](elidex_script_session::HostDriver::deliver_media_query_changes);
    /// VM tests call it directly after [`Self::set_media_environment`].
    pub fn deliver_media_query_changes(&mut self) {
        self.inner.deliver_media_query_changes();
    }

    /// Push the **monitor** (display) dimensions in CSS px into the single
    /// `ViewportState` device-facts SoT, so `screen.width` / `.height` /
    /// `.availWidth` / `.availHeight` (CSSOM-View §4.3) read the new values. A
    /// **pure state push, no delivery turn** — monitor dims are NOT a
    /// `MediaEnvironment` input (no media feature reads them) and there is no
    /// `change` event for `screen`, so this does NOT route through
    /// `set_media_environment` / `deliver_media_query_changes`. The sibling of
    /// [`Self::set_media_environment`] for a different device-fact axis. Backs
    /// [`HostDriver::set_screen_dimensions`](elidex_script_session::HostDriver::set_screen_dimensions);
    /// VM tests call it directly. The CSS-px conversion from physical
    /// `current_monitor().size()` happens at the shell producer (which rides the
    /// S5-6 flip), keeping this a pure transport.
    pub fn set_screen_dimensions(
        &mut self,
        width: f64,
        height: f64,
        avail_width: f64,
        avail_height: f64,
    ) {
        let vp = &mut self.inner.viewport;
        vp.screen_width = width;
        vp.screen_height = height;
        vp.avail_width = avail_width;
        vp.avail_height = avail_height;
    }

    /// Run the CSSOM-View §13.1 `VisualViewport` report-changes pass — diff the
    /// current viewport size against the producer's stored prior and fire
    /// `resize` (a `(width, height)` change) at the `visualViewport` singleton.
    /// It does NOT fire `scroll`/`scrollend`: §13.2 fires those only on a
    /// visual-viewport *offset* change (pinch-zoom), which elidex does not model,
    /// so an ordinary layout scroll is a document scroll, not a visual-viewport
    /// scroll. The first deliver after a bind fires nothing (the prior is seeded
    /// at `Vm::bind`, the load-time baseline). Backs
    /// [`HostDriver::deliver_visual_viewport_events`](elidex_script_session::HostDriver::deliver_visual_viewport_events);
    /// VM tests call it directly after a geometry change.
    pub fn deliver_visual_viewport_events(&mut self) {
        self.inner.deliver_visual_viewport_events();
    }
}
