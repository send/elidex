//! Content-area placement + viewport delivery + deferred initial spawn (C1).
//!
//! Extracted from `app/mod.rs` to keep that lifecycle module under the project's
//! 1000-line guideline (Axis 5): this is the cohesive cluster that builds the
//! [`ContentAreaPlacement`] SoT, fans the viewport out to content threads, and
//! performs the window-deferred initial-tab spawn. [`ViewportProducer`] groups
//! the three state fields; the placement type, `PendingSpawn` intent, and `App`
//! struct itself stay in `app/mod.rs`.

use std::sync::Arc;

use elidex_plugin::Size;
use winit::window::Window;

use crate::chrome;
use crate::ipc::{BrowserToContent, ContentToBrowser};

use super::tab::Tab;
use super::{App, ContentAreaPlacement, PendingSpawn};

/// Viewport-producer state: the three fields that the viewport mechanism methods
/// act on, sub-structured from [`super::App`] to keep `app/mod.rs` under the
/// project's 1000-line guideline.
pub(super) struct ViewportProducer {
    /// Cached content-area placement SoT (size↔origin↔scale), the single
    /// descriptor the producer/compositor/input mapper all read.
    ///
    /// `Some` once the window exists — seeded in `resumed` together with
    /// `render_state` and recomputed at redraw top + on `Resized`. `None`
    /// before the first `resumed` / after `suspended`.
    pub(super) placement: Option<ContentAreaPlacement>,
    /// The initial tab's content-thread spawn, deferred until `resumed` (C1).
    /// `Some` between `new_threaded*` and the first `resumed`; `None` after
    /// (inline mode is always `None`).
    pub(super) pending_initial_spawn: Option<PendingSpawn>,
    /// Browser-published latest content-area viewport + monotonic seq — the
    /// **pull** source every content thread reads at build time. Single writer
    /// (browser thread, via [`crate::ipc::ViewportCell::publish_if_changed`]); `Arc`-shared
    /// into every spawned content thread. Seeded with `DEFAULT` before the
    /// window exists; the first `resumed` publish bumps it to the real size at
    /// seq 1. One per window — all tabs share the content area.
    pub(super) viewport_cell: Arc<crate::ipc::ViewportCell>,
}

impl ViewportProducer {
    /// Construct, seeding the viewport cell with the given default size until
    /// `resumed` publishes the window's real size.
    pub(super) fn new(default_w: f32, default_h: f32) -> Self {
        Self {
            placement: None,
            pending_initial_spawn: None,
            viewport_cell: crate::ipc::ViewportCell::new(Size::new(default_w, default_h)),
        }
    }
}

impl App {
    /// Build the content-area placement SoT from the current window + active-tab
    /// chrome position.
    ///
    /// The **only** caller of `window.scale_factor()`, [`chrome::content_size`],
    /// and [`chrome::chrome_content_offset`] (egui's own DPI read at render-init
    /// excepted) — the three primitives are snapshotted atomically (one
    /// `scale_factor` read per build). Callers cache the result in
    /// [`App::placement`]; nothing else recomputes a content-area size/origin.
    // Window dimensions (< 2^23 px) and the scale factor lose no meaningful
    // precision narrowing to the `f32` the layout/CSS coordinate space uses.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub(super) fn content_area_placement(&self, window: &Window) -> ContentAreaPlacement {
        let scale_factor = window.scale_factor() as f32;
        let phys = window.inner_size();
        let win_logical_w = phys.width as f32 / scale_factor;
        let win_logical_h = phys.height as f32 / scale_factor;
        let position = self.tab_bar_position();
        ContentAreaPlacement {
            origin_logical: chrome::chrome_content_offset(position),
            size_logical: chrome::content_size(win_logical_w, win_logical_h, position),
            scale_factor,
        }
    }

    /// Send the cached content-area size (CSS logical px) to **one** tab's content
    /// thread — the single per-tab viewport-delivery primitive (C1).
    ///
    /// `seq` is the [`crate::ipc::ViewportCell`] sequence the broadcast size was
    /// published at, so the recipient can drop it if its build already consumed that
    /// seq (the resume-time re-delivery to a just-spawned tab is the canonical case).
    /// Content stays device-agnostic: only `size_logical` crosses the IPC, never
    /// `scale_factor`. An associated fn (not `&self`) so it composes with an active
    /// `&mut self.tab_manager` borrow (mirrors [`Self::wake_or_noop`]). No-op until
    /// `placement` is seeded. `BrowserToContent` is not `Clone`, so each recipient
    /// gets a freshly-constructed message.
    pub(super) fn seed_tab_viewport(placement: Option<ContentAreaPlacement>, seq: u64, tab: &Tab) {
        if let Some(placement) = placement {
            if let Err(e) = tab.channel.send(BrowserToContent::SetViewport {
                width: placement.size_logical.width,
                height: placement.size_logical.height,
                seq,
            }) {
                eprintln!("Failed to send viewport to content thread (disconnected): {e}");
            }
        }
    }

    /// Fan the cached viewport out to **every** tab — all share the window's
    /// content area, so a resize must reach background tabs too (their
    /// `innerWidth`/`matchMedia` stay spec-correct). Called on `Resized` and on
    /// re-`resumed` (plan-memo Q3), but **only when** the matching
    /// `viewport_cell.publish_if_changed` reported a real size change — the caller gates
    /// this on that return (C2), so the cell's current seq tags the delivery and an
    /// unchanged size fans out nothing. (A pure DPI/scale `Resized` keeps `size_logical`,
    /// so it publishes nothing and skips this — see `App::window_event`.)
    /// Initial/`window.open`/new tabs are instead born at the real size via
    /// the construction-input spawn (C1, the cell read), so they need no seed message.
    /// The cached `placement` is keyed to the active tab's chrome; one size fits every
    /// tab while all use the default (`Top`) tab-bar position → slot
    /// #11-window-level-tab-bar-position.
    pub(super) fn broadcast_viewport(&self) {
        if let Some(mgr) = &self.tab_manager {
            // The seq the producer just published (resumed/Resized publish precedes
            // this call). Size still comes from `placement` (the geometry SoT); the
            // publish set the cell to exactly that size, so the pair is consistent.
            let (_, seq) = self.viewport.viewport_cell.read();
            for tab in mgr.tabs() {
                Self::seed_tab_viewport(self.viewport.placement, seq, tab);
            }
        }
    }

    /// The current [`crate::ipc::ViewportCell`] placement-seq — the seq that a
    /// coordinate-bearing input event's coordinates are mapped against when sent
    /// **now**. It corresponds to `self.viewport.placement` by construction: a resize
    /// republishes the cell (bumping the seq) right after caching the new placement
    /// (`resumed`/`Resized`), and the browser thread is the single writer, so no
    /// resize can interleave between an input handler's placement read and this
    /// cell read. Stamped onto `MouseClick`/`MouseMove`/`MouseWheel` so the content
    /// thread can drop input mapped against a placement its build/runtime has since
    /// superseded (`content/event_loop.rs`, plan-memo §10).
    pub(super) fn current_placement_seq(&self) -> u64 {
        self.viewport.viewport_cell.read().1
    }

    /// Spawn the deferred initial content thread (C1), now that the window exists. The
    /// thread reads its build size from the shared `viewport_cell` (already published
    /// with the real size by the `resumed` caller), so it is born at the real viewport
    /// without a size argument. No-op if there is no pending spawn (already spawned,
    /// or inline mode) or no network process. The minted [`crate::WakeHandle`] comes
    /// from the disjoint `wake_proxy` field (mirrors the `window.open` /
    /// `open_new_tab` spawn sites).
    pub(super) fn spawn_pending_initial_tab(&mut self) {
        let Some(pending) = self.viewport.pending_initial_spawn.take() else {
            return;
        };
        // `pending` is `Some` only in threaded mode (set by `new_threaded*`), and
        // `from_tab_manager` guarantees `network_process` + `tab_manager` are then
        // `Some`. Surface a broken invariant rather than silently dropping the
        // initial tab (a blank window) — mirrors the `tab_manager.expect` in
        // `handle_redraw_threaded`.
        let np = self
            .network_process
            .as_ref()
            .expect("threaded-mode initial spawn requires a network process");
        let nh = np.create_renderer_handle();
        let jar = Arc::clone(np.cookie_jar());
        let wake = Self::wake_or_noop(self.wake_proxy.as_ref());
        let (browser_ch, content_ch) =
            crate::ipc::channel_pair::<BrowserToContent, ContentToBrowser>();
        let viewport_cell = Arc::clone(&self.viewport.viewport_cell);
        let (thread, chrome, title) = match pending {
            PendingSpawn::Html { html, css } => (
                crate::content::spawn_content_thread(
                    content_ch,
                    nh,
                    jar,
                    html,
                    css,
                    viewport_cell,
                    wake,
                ),
                crate::chrome::ChromeState::new(None),
                "elidex".to_string(),
            ),
            PendingSpawn::Url(url) => {
                let title = format!("elidex \u{2014} {url}");
                let chrome = crate::chrome::ChromeState::new(Some(&url));
                let thread = crate::content::spawn_content_thread_url(
                    content_ch,
                    nh,
                    jar,
                    url,
                    viewport_cell,
                    wake,
                );
                (thread, chrome, title)
            }
        };
        self.tab_manager
            .as_mut()
            .expect("threaded mode requires a tab manager")
            .create_tab(browser_ch, thread, chrome, title);
    }
}
