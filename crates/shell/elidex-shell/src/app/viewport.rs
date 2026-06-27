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
use winit::window::{Theme, Window};

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
    /// (browser thread, via [`crate::ipc::ViewportCell::publish_device_state`]); `Arc`-shared
    /// into every spawned content thread. Seeded with `DEFAULT` before the
    /// window exists; the first `resumed` publish bumps it to the real size at
    /// seq 1 — *unless* the real size equals `DEFAULT`, which leaves the cell at
    /// seq 0 and skips the broadcast (`publish_device_state`, plan-memo E1). One per
    /// window — all tabs share the content area.
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

/// Convert a physical-pixel extent to CSS logical px, dividing in `f64` before the
/// single narrowing to the `f32` layout space.
///
/// Dividing in `f32` (the scale cast to `f32` first) makes a pure-scale resize that
/// mathematically *preserves* the logical size round to a **different** `f32` — e.g.
/// `960px ÷ 1.2 = 799.99994` instead of `800.0` — which [`Size`]'s exact equality would
/// treat as a new viewport generation, bumping `seq` and dropping queued input (the
/// phantom generation C2's [`ViewportCell::publish_device_state`](crate::ipc::ViewportCell::publish_device_state)
/// exists to prevent, reintroduced by the `f32` round-trip). `f64` division recovers
/// integer / simple-fraction logical sizes exactly, so a logical-preserving physical size
/// narrows back to the same `f32`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn logical_px(physical: u32, scale_factor: f64) -> f32 {
    (f64::from(physical) / scale_factor) as f32
}

/// Map the winit window theme to the engine-independent
/// [`ColorScheme`](elidex_css::media::ColorScheme) the `@media (prefers-color-scheme)`
/// evaluator reads (C3). winit reports `None` on the platforms with no theme event
/// (X11 / Wayland), where `prefers-color-scheme` falls back to the `Light` UA default
/// (Media Queries L5 §12.5 — no separate "no-preference" state).
fn theme_color_scheme(window: &Window) -> elidex_css::media::ColorScheme {
    match window.theme() {
        Some(Theme::Dark) => elidex_css::media::ColorScheme::Dark,
        _ => elidex_css::media::ColorScheme::Light,
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
        let scale_factor = window.scale_factor();
        let phys = window.inner_size();
        // Divide in f64 so a logical-preserving DPI change recovers the same logical
        // size (an f32 division round-trips e.g. 960px@1.2 to 799.99994 ≠ 800.0, a
        // phantom seq generation — see `logical_px`). Scale itself stays f32 in the
        // placement (compositor transform / input ÷scale tolerate the narrowing).
        let win_logical_w = logical_px(phys.width, scale_factor);
        let win_logical_h = logical_px(phys.height, scale_factor);
        let position = self.tab_bar_position();
        ContentAreaPlacement {
            origin_logical: chrome::chrome_content_offset(position),
            size_logical: chrome::content_size(win_logical_w, win_logical_h, position),
            scale_factor: scale_factor as f32,
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
    /// `viewport_cell.publish_device_state` reported a real size change — the caller gates
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
            // The seq the producer just published (resumed / redraw-top publish
            // precedes this call). Size still comes from `placement` (the geometry
            // SoT); the publish set the cell to exactly that size, so the pair is
            // consistent.
            let seq = self.viewport.viewport_cell.read().seq;
            for tab in mgr.tabs() {
                Self::seed_tab_viewport(self.viewport.placement, seq, tab);
            }
        }
    }

    /// Fan the latest device facts (dppx / color-scheme) out to **every** tab — all
    /// share the window's display, so a DPI/theme change must reach background tabs
    /// too (their `devicePixelRatio` / `@media (resolution | prefers-color-scheme)`
    /// stay live). The C3 sibling of [`Self::broadcast_viewport`]: called from the
    /// redraw-top chokepoint **only when** [`crate::ipc::ViewportCell::publish_device_state`]
    /// reported `facts_changed` (the caller gates on that), so an unchanged-facts frame
    /// fans out nothing. **No size `seq`** — facts are orthogonal to the `size_logical`
    /// generation (D3), so a pure-scale change the OS absorbs delivers facts without
    /// dropping queued input. The delivery still carries the cell's **`facts_seq`** (the
    /// device-facts generation the publish just bumped) so a content thread can drop a
    /// fact already folded into its build's cell-read — the build-vs-broadcast staleness
    /// race `SetViewport`'s `seq` handles, here for facts. Initial/`window.open`/new tabs
    /// are instead born with the facts via the construction-input spawn (the cell read
    /// seeds the bridge before scripts), so this fan-out is a guarded no-op for them.
    /// `BrowserToContent` is not `Clone`, so each recipient gets a freshly-constructed
    /// message.
    pub(super) fn broadcast_device_facts(&self) {
        if let Some(mgr) = &self.tab_manager {
            let snapshot = self.viewport.viewport_cell.read();
            let (facts, facts_seq) = (snapshot.facts, snapshot.facts_seq);
            for tab in mgr.tabs() {
                if let Err(e) = tab.channel.send(BrowserToContent::SetDeviceFacts {
                    color_scheme: facts.color_scheme,
                    dppx: facts.dppx,
                    facts_seq,
                }) {
                    eprintln!("Failed to send device facts to content thread (disconnected): {e}");
                }
            }
        }
    }

    /// The window's current device facts (C3): `dppx` from the already-snapshotted
    /// `placement.scale_factor` (no second `scale_factor` read — the placement builder
    /// is its sole reader) + `color_scheme` from the window theme. Fed to
    /// [`crate::ipc::ViewportCell::publish_device_state`] at the redraw-top chokepoint
    /// and the `resumed` seed.
    pub(super) fn device_facts(
        placement: ContentAreaPlacement,
        window: &Window,
    ) -> crate::ipc::DeviceFacts {
        crate::ipc::DeviceFacts {
            dppx: placement.scale_factor,
            color_scheme: theme_color_scheme(window),
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
        self.viewport.viewport_cell.read().seq
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

#[cfg(test)]
mod tests {
    use super::logical_px;

    #[test]
    fn logical_px_recovers_pure_scale_size_without_phantom_generation() {
        // Codex #411 R3 (P2): 960 physical px at 1.2× DPI is exactly 800.0 CSS px, but an
        // f32 division yields 799.99994 — which `Size`'s exact-eq `seq` guard would treat
        // as a new generation, bumping seq + dropping queued input on a fractional-DPI
        // move. f64 division narrows back to 800.0.
        assert_eq!(logical_px(960, 1.2), 800.0);

        // Round-trip property: the logical-preserving physical size the OS picks on a pure
        // DPI change recovers its logical extent bit-exactly across common scales, so the
        // exact-eq guard reports "unchanged" and never manufactures a generation. (Each
        // `logical × scale` here is an exact integer — a genuine sub-pixel quantization
        // shift, by contrast, *is* a real size change and correctly bumps.)
        for &(logical, scale) in &[
            (800.0_f32, 1.0_f64),
            (800.0, 1.25),
            (800.0, 1.5),
            (800.0, 1.75),
            (800.0, 2.0),
            (1280.0, 1.25),
            (1280.0, 1.5),
        ] {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let phys = (f64::from(logical) * scale) as u32;
            assert_eq!(
                logical_px(phys, scale),
                logical,
                "logical {logical} @ {scale}×"
            );
        }
    }
}
