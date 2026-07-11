//! Window management and event loop shell for elidex.
//!
//! Provides the top-level integration that ties together parsing, styling,
//! layout, and rendering into a windowed application.
//!
//! # Usage
//!
//! ```ignore
//! elidex_shell::run("<h1>Hello</h1>", "h1 { color: red; }").unwrap();
//! ```

/// Maximum rounds of CE callback stabilization after `re_render` flush.
const MAX_CE_STABILIZATION_ROUNDS: usize = 8;

pub(crate) mod animation;
mod app;
pub(crate) mod chrome;
mod content;
mod gpu;
pub mod ipc;
pub(crate) mod key_map;
mod pipeline;
pub use pipeline::{
    build_pipeline_from_loaded, build_pipeline_from_url, build_pipeline_interactive,
    PreEvalFrameInputs, PreEvalFrameState,
};
// `pub(crate)` builders keep their crate-only visibility across the move — the
// re-export mirrors the original in-`lib.rs` `pub(crate) fn` reach so every
// `crate::build_pipeline_*` call site stays identical.
pub(crate) use pipeline::{
    build_pipeline_interactive_shared, build_pipeline_interactive_with_network,
};
pub mod quota;

#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::sync::Arc;

use elidex_css::Stylesheet;
use elidex_css_anim::engine::AnimationEngine;
use elidex_dom_compat::{
    get_presentational_hints, legacy_ua_stylesheet, parse_compat_stylesheet_with_registry,
};
use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_js::ElidexJsEngine;
use elidex_layout::layout_tree;
use elidex_plugin::{EngineMode, Size, Vector, ViewportOverflow};
use elidex_render::{build_display_list_with_scroll, DisplayList};
use elidex_script_session::{HostDriver, ScriptContext, ScriptEngine, SessionCore};
use elidex_style::resolve_styles_with_compat;
use elidex_text::FontDatabase;
use winit::event_loop::EventLoop;

use animation::{
    apply_active_animations, collect_computed_without_anim, collect_old_anim_styles,
    detect_and_start_transitions, sync_css_animations,
};

use app::App;

/// User event delivered to the winit event loop to schedule a repaint.
///
/// The content thread runs the browser loop under `ControlFlow::Wait`; a
/// content-initiated frame (timers / rAF / animation / async DOM / the
/// `SetViewport` round-trip's corrected frame) would otherwise paint only on the
/// next OS-driven event. Sending this wakes the loop so the produced frame
/// reaches a rendering opportunity (WHATWG HTML §8.1.7.3). Browser-internal: the
/// content thread never references this type — it holds only a [`WakeHandle`].
#[derive(Debug, Clone, Copy)]
pub enum WakeEvent {
    /// Request a redraw of the active window (drains pending content messages).
    Repaint,
}

/// A windowing-agnostic "wake the browser to repaint" callback handed to a
/// content thread at spawn.
///
/// Keeps `content/` free of `winit` types (the content thread is the CSS/renderer
/// owner per *concurrency-by-ownership*): it calls `wake()` after a
/// display/chrome-affecting send, knowing only "notify the host", not the winit
/// `EventLoopProxy`. Each content thread owns its own boxed closure (built from a
/// cloned `EventLoopProxy<WakeEvent>` in the browser half), so `Send` suffices.
pub type WakeHandle = Box<dyn Fn() + Send>;

/// Build the CSS property registry with all standard property handlers.
///
/// Delegates to [`elidex_style::create_css_property_registry`].
#[must_use]
pub fn create_css_property_registry() -> elidex_plugin::CssPropertyRegistry {
    elidex_style::create_css_property_registry()
}

/// Default viewport width for the initial layout pass.
const DEFAULT_VIEWPORT_WIDTH: f32 = 1024.0;
/// Default viewport height for the initial layout pass.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 768.0;

/// HTML content for a blank new-tab page.
const BLANK_TAB_HTML: &str = "<html><body><h1>New Tab</h1></body></html>";
/// CSS for the blank new-tab page.
const BLANK_TAB_CSS: &str = "body { background-color: #ffffff; color: #333333; font-family: sans-serif; } h1 { text-align: center; margin-top: 200px; }";

/// Resolve styles under the engine-wide [`EngineMode`]'s style-compat policy.
///
/// `BrowserCompat` applies the compat cascade — the legacy UA stylesheet + HTML
/// presentational hints (WHATWG HTML §15.2) + the CSS property registry for
/// handler-based `is_inherited()` / `initial_value()` / `get_computed()`
/// dispatch. `BrowserCore` / `App` resolve against the modern UA baseline only
/// (no legacy UA sheet, no presentational hints).
///
/// The style-compat policy is derived **in parallel** to the Web-API
/// `SpecLevelPolicy` from the same one [`EngineMode`], so the CSS pipeline carries
/// no dependency on a Web-API classification enum (web-api-compat-split design
/// §5 / R3-6). `medium` reaches both arms so no `@media` is dropped in either mode.
fn resolve_with_mode(
    dom: &mut EcsDom,
    author_stylesheets: &[&Stylesheet],
    registry: &elidex_plugin::CssPropertyRegistry,
    viewport: Size,
    medium: elidex_css::media::Medium,
    engine_mode: EngineMode,
) -> ViewportOverflow {
    if engine_mode.style_compat_policy().presentational_compat() {
        let legacy_ua = legacy_ua_stylesheet();
        resolve_styles_with_compat(
            dom,
            author_stylesheets,
            &[legacy_ua],
            &get_presentational_hints,
            viewport,
            medium,
            Some(registry),
        )
    } else {
        // Core (BrowserCore/App): modern UA baseline only — no legacy UA sheet,
        // no presentational hints. This is the `elidex_style::resolve_styles`
        // surface, threaded with the call site's `medium` so paged/print output
        // keeps `@media print` even in core mode. The modern baseline (core
        // `ua_stylesheet`) carries the standard §15.3 conforming rendering since
        // the UA-sheet reclassification (#408), so dropping the legacy sheet drops
        // only obsolete-element rendering (strike/big/center) + presentational
        // hints — standard rendering (e.g. `<strong>` bold) survives in core.
        resolve_styles_with_compat(
            dom,
            author_stylesheets,
            &[],
            &no_hints,
            viewport,
            medium,
            None,
        )
    }
}

/// No-op presentational-hint generator for the core (non-compat) cascade.
///
/// The compat arm uses [`get_presentational_hints`]; `BrowserCore`/`App` supply
/// no hints (modern UA baseline). Mirrors `elidex_style`'s private `no_hints`.
fn no_hints(_entity: Entity, _dom: &EcsDom) -> Vec<elidex_css::Declaration> {
    Vec::new()
}

/// Run the full browser pipeline and display the result in a window.
///
/// Parses HTML, applies CSS, computes layout, builds a display list,
/// and opens a window rendering the result via Vello + wgpu.
///
/// Content processing (DOM, JS, style, layout) runs on a dedicated thread,
/// communicating with the browser thread via message passing.
///
/// This function blocks until the window is closed.
pub fn run(html: &str, css: &str) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<WakeEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new_threaded(html.to_string(), css.to_string(), proxy);
    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Execute the rendering pipeline without opening a window.
///
/// Useful for testing the parse → style → layout → display list chain.
/// Includes script execution phase: `<script>` tags are evaluated after
/// initial style resolution, followed by re-resolution and layout.
///
/// Delegates to [`build_pipeline_interactive`] and returns only the display list.
#[must_use]
pub fn build_pipeline(html: &str, css: &str) -> elidex_render::DisplayList {
    build_pipeline_interactive(html, css).display_list
}

/// Result of the interactive rendering pipeline.
///
/// Contains all state needed to handle user events and re-render.
pub struct PipelineResult {
    /// The initial display list.
    pub display_list: DisplayList,
    /// The ECS DOM.
    pub dom: EcsDom,
    /// The document root entity.
    pub document: Entity,
    /// The script session state.
    pub session: SessionCore,
    /// The JavaScript runtime (S5-6b flip: boa `JsRuntime` → VM `ElidexJsEngine`).
    pub runtime: ElidexJsEngine,
    /// All parsed CSS stylesheets.
    pub stylesheets: Vec<Stylesheet>,
    /// The font database (shared across navigations to avoid re-scanning).
    pub font_db: Arc<FontDatabase>,
    /// The URL of the current page, if loaded from a URL.
    pub url: Option<url::Url>,
    /// Network handle for communicating with the Network Process broker.
    /// `disconnected()` when no broker is available (standalone tests).
    pub network_handle: Rc<elidex_net::broker::NetworkHandle>,
    /// Keeps the broker thread alive for standalone pipelines.
    /// `None` when the App owns the broker (normal tab mode).
    #[allow(dead_code)]
    pub(crate) broker_keepalive: Option<elidex_net::broker::NetworkProcessHandle>,
    /// CSS property registry (cached to avoid re-creation on each re-render).
    /// `Arc`-wrapped so it can be shared with child iframe pipelines.
    pub registry: Arc<elidex_plugin::CssPropertyRegistry>,
    /// CSS animation/transition engine.
    pub animation_engine: AnimationEngine,
    /// Current viewport dimensions for layout.
    pub viewport: Size,
    /// Whether the text input caret should be visible in the display list.
    ///
    /// Set by the content thread's caret blink timer. Defaults to `true`.
    pub caret_visible: bool,
    /// Cached form ancestor lookups (invalidated on DOM mutation).
    pub ancestor_cache: elidex_form::AncestorCache,
    /// Viewport-level overflow propagated from root/body element.
    pub viewport_overflow: ViewportOverflow,
    /// Viewport scroll offset synced from content thread before re-render.
    pub scroll_offset: Vector,
    /// Engine-wide operating mode — the embedder's single mode authority.
    ///
    /// The shell is a browser embedder, so a production session is
    /// [`EngineMode::BrowserCompat`] (the byte-identical full compat surface).
    /// Read by `re_render` and the pipeline builders to derive the style-compat
    /// policy ([`EngineMode::style_compat_policy`]); the *same* field will feed VM
    /// construction at the boa→elidex-js-VM cutover, so there is one mode authority,
    /// not a style-only one. `BrowserCore`/`App` await the async-core storage
    /// precondition (`#11-async-core-storage-cookiestore`) before a real session
    /// selects them — until then they are exercised by tests only.
    pub engine_mode: EngineMode,
}

impl PipelineResult {
    /// Dispatch a DOM event through the propagation path.
    ///
    /// Returns `true` if `preventDefault()` was called.
    ///
    /// S5-6b batch-bind bracket (§4.1 "UA / lifecycle event dispatch"): build
    /// `ScriptContext` ONCE, bind through it, drive `script_dispatch_event` +
    /// `drain_reactions` under the SAME `&mut ctx`, unbind. `with_bound`'s RAII
    /// guard makes the bracket panic-safe and unpaired-form-unrepresentable. The
    /// driving-order invariant (never reconstruct `ScriptContext::new(&mut dom,…)`
    /// mid-bracket — a fresh `&mut dom` invalidates the bound `*mut dom`) is held
    /// by threading the single `ctx` the closure receives.
    #[allow(unsafe_code)]
    pub fn dispatch_event(&mut self, event: &mut elidex_script_session::DispatchEvent) -> bool {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: `ctx` outlives the bracket and neither this method nor the
        // trait methods touch `ctx.session` / `ctx.dom` through a `&mut` while
        // bound (they use the bound pointers) — the `with_bound` contract.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                let prevented = elidex_script_session::script_dispatch_event(engine, event, ctx);
                // Post-dispatch microtask + CE-reaction checkpoint (HTML §8.1.4.4
                // *clean up after running script*), per the bracket contract.
                engine.drain_reactions(ctx);
                prevented
            })
        }
    }

    /// Evaluate a JavaScript source string (bracketed, §4.1).
    #[allow(unsafe_code)]
    pub fn eval_script(&mut self, source: &str) -> elidex_script_session::EvalResult {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        unsafe {
            self.runtime
                .with_bound(&mut ctx, |engine, ctx| engine.eval(source, ctx))
        }
    }

    /// Drain and execute all ready timers (bracketed, §4.1).
    #[allow(unsafe_code)]
    pub fn drain_timers(&mut self) -> Vec<elidex_script_session::EvalResult> {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        unsafe {
            self.runtime
                .with_bound(&mut ctx, |engine, ctx| engine.drain_timers(ctx))
        }
    }

    /// Deliver a batch of flushed `MutationRecord`s to the engine and drain
    /// the resulting custom-element reactions, in ONE batch bracket (§4.1 +
    /// §4.3.1).
    ///
    /// `deliver_mutation_records` runs the record→CE enqueue (S5-6b §4.3.1,
    /// the single `elidex_custom_elements` classification) followed by
    /// `MutationObserver` delivery; `drain_reactions` then drains the CE queue
    /// (callbacks the enqueue produced). Both are assume-bound, so they run
    /// inside the `with_bound` bracket. `records` is external data (the flush
    /// output) — the driving-order invariant only forbids touching
    /// `ctx.session` / `ctx.dom` while bound, which neither method does.
    #[allow(unsafe_code)]
    fn deliver_records_and_drain(&mut self, records: &[elidex_script_session::MutationRecord]) {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        // `records` is owned outside the bracket and does not alias `ctx`.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                engine.deliver_mutation_records(records);
                engine.drain_reactions(ctx);
            });
        }
    }

    /// Flush every dirty `<canvas>` into its display-list source (HTML §4.12.5),
    /// in ONE batch bracket (§4.1). Bracketed because `sync_dirty_canvases` is
    /// assume-bound (it reads the bound `EcsDom` to reach each canvas's backing
    /// store). `drain_reactions` follows the bracket contract — a canvas sync
    /// fires no JS itself, so the checkpoint is a no-op on an empty queue, but it
    /// keeps the post-deliver microtask/CE-reaction drain uniform with
    /// `dispatch_event` / `deliver_records_and_drain`.
    #[allow(unsafe_code)]
    pub fn sync_dirty_canvases(&mut self) {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                engine.sync_dirty_canvases();
                engine.drain_reactions(ctx);
            });
        }
    }

    /// Deliver the queued `ResizeObserver` + `IntersectionObserver` callbacks in
    /// ONE batch bracket (§4.1 "no per-channel brackets" for the contiguous
    /// pair). Both are assume-bound (they read the bound state — intersection
    /// reads the viewport from it, no longer a passed `Rect`) and fire in this
    /// spec order (resize before intersection). `drain_reactions` follows the
    /// bracket contract, draining the microtask + CE reactions the observer
    /// callbacks produced.
    #[allow(unsafe_code)]
    pub fn deliver_layout_observations(&mut self) {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                engine.deliver_resize_observations();
                engine.deliver_intersection_observations();
                engine.drain_reactions(ctx);
            });
        }
    }

    /// Fire `versionchange` at this engine's open IndexedDB connections to
    /// `db_name` (IndexedDB-3 §4.2), in ONE batch bracket (§4.1). Assume-bound
    /// (it reaches the bound VM's open connections); `drain_reactions` follows
    /// the bracket contract, draining the microtask + CE reactions the
    /// `versionchange` handler produced.
    #[allow(unsafe_code)]
    pub fn deliver_idb_versionchange(
        &mut self,
        db_name: &str,
        old_version: u64,
        new_version: Option<u64>,
    ) {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        // `db_name` is borrowed external data and does not alias `ctx`.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                engine.deliver_idb_versionchange(db_name, old_version, new_version);
                engine.drain_reactions(ctx);
            });
        }
    }

    /// Deliver any parent-side dedicated/shared worker `postMessage`s that
    /// arrived since the last turn, in ONE batch bracket (§4.1). Assume-bound
    /// (it dispatches at the bound VM's `Worker` objects); `drain_reactions`
    /// follows the bracket contract, draining the microtask + CE reactions the
    /// `message` handlers produced.
    #[allow(unsafe_code)]
    pub fn drain_worker_messages(&mut self) {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                engine.drain_worker_messages();
                engine.drain_reactions(ctx);
            });
        }
    }

    /// Deliver the popstate / hashchange of a same-document history-step
    /// application (WHATWG HTML §7.4.6.2), in ONE batch bracket (§4.1).
    /// Assume-bound (it fires at the bound VM's `Window` and reconstructs
    /// `history.state`); it was previously called UNBRACKETED and so was a
    /// silent no-op. `drain_reactions` follows the bracket contract, draining
    /// the microtask + CE reactions the popstate/hashchange handlers produced.
    #[allow(unsafe_code)]
    pub fn deliver_history_step_events(&mut self, ev: elidex_script_session::HistoryStepEvents) {
        let mut ctx = ScriptContext::new(&mut self.session, &mut self.dom, self.document);
        // SAFETY: see `dispatch_event` — the `with_bound` unaliased contract.
        // `ev` is owned external data and does not alias `ctx`.
        unsafe {
            self.runtime.with_bound(&mut ctx, |engine, ctx| {
                engine.deliver_history_step_events(ev);
                engine.drain_reactions(ctx);
            });
        }
    }

    /// Remove animation/transition state for entities that no longer exist in the DOM.
    pub(crate) fn prune_dead_animation_entities(&mut self) {
        self.animation_engine.prune_dead_entities(&|entity_id| {
            Entity::from_bits(entity_id).is_some_and(|entity| self.dom.world().contains(entity))
        });
        self.animation_engine.prune_unused_keyframes();
    }
}

/// Re-render after DOM changes: re-resolve styles, re-layout, and rebuild display list.
///
/// Includes transition detection: saves old computed values for entities with
/// `AnimStyle`, re-resolves styles, compares old vs new values to detect
/// transitions, feeds them to the `AnimationEngine`, and applies animated
/// values to `ComputedStyle` before layout.
///
/// Returns the mutation records from the flush, for the shell's own record
/// consumers (focusable-cache invalidation, iframe add/remove detection). The
/// observer + CE delivery for these records is now internal (see below).
pub(crate) fn re_render(result: &mut PipelineResult) -> Vec<elidex_script_session::MutationRecord> {
    // Flush applies buffered mutations to the DOM. `flush` runs OUTSIDE any
    // batch bracket — it takes `&mut dom`, which the bound `*mut dom` aliasing
    // contract forbids overlapping (§4.1). It returns a flat record stream (a
    // childList move yields two records).
    //
    // S5-6b §4.3.1 CE-loop dissolve: under the VM, `session.flush` is usually
    // EMPTY — VM-native mutations write the `EcsDom` immediately via `apply_*`,
    // settle CE inside the VM's own checkpoints (the bind-installed dispatcher +
    // `flush_ce_reactions`), and queue their observer records internally, so
    // they never enter `SessionCore::pending`. The flushed records are the
    // EXTERNAL (shell-buffered / layout-derived) case; each bracketed
    // `deliver_mutation_records` runs the record→CE enqueue (the single
    // `elidex_custom_elements` classification) + `MutationObserver` delivery,
    // and `drain_reactions` drains the CE reactions those callbacks produce. CE
    // callbacks may record further mutations, so re-flush (unbound) → bracket
    // until stable, bounded.
    let mut mutation_records: Vec<elidex_script_session::MutationRecord> =
        result.session.flush(&mut result.dom);

    if !mutation_records.is_empty() {
        result.deliver_records_and_drain(&mutation_records);

        for round in 0..MAX_CE_STABILIZATION_ROUNDS {
            let follow_up: Vec<_> = result.session.flush(&mut result.dom);
            if follow_up.is_empty() {
                break;
            }
            result.deliver_records_and_drain(&follow_up);
            mutation_records.extend(follow_up);
            if round == MAX_CE_STABILIZATION_ROUNDS - 1 {
                eprintln!(
                    "[CE] stabilization loop hit max rounds ({MAX_CE_STABILIZATION_ROUNDS}); \
                     some mutations may be deferred to next frame"
                );
            }
        }
    }

    // Invalidate ancestor cache when DOM mutations occurred.
    if !mutation_records.is_empty() {
        result.ancestor_cache.invalidate_all();
    }

    // Prune animations/transitions for destroyed entities unconditionally.
    // JS event handlers may destroy entities without generating style mutations,
    // so conditional pruning could leak animation state.
    result.prune_dead_animation_entities();

    // Maintain the focus invariant `current_focus ⟹ is_focusable` (WHATWG HTML
    // §6.6.2): a DOM mutation this frame may have made the focused element
    // non-focusable (its `hidden`/`disabled` landed, `<input type>` flipped to a
    // non-focusable kind, or it lost the `tabindex`/`contenteditable`/`href` that
    // made it focusable) with focus still on it. Silently reset focus (no events,
    // like the §2.1.4 removal reset). Gated on "any mutation occurred" rather than
    // a focusability-attribute allow-list so every focusability-affecting change is
    // caught without a hand-maintained, drift-prone trigger list. Lives at this
    // single `re_render` chokepoint so the parent document, in-process iframes
    // (`content::iframe::render::re_render_all_iframes`) and OOP iframes
    // (`content::iframe::thread::iframe_thread_main`) all reconcile — every
    // pipeline funnels through this one function. Runs BEFORE style resolution /
    // layout / display-list build (not after) so the cleared `FOCUS` bit is
    // reflected in `:focus` styling, form-control caret state and the display
    // list this same frame — otherwise the render builder (which reads
    // `ElementState::FOCUS`) would paint stale `:focus` for a frame after
    // `activeElement` was already cleared, with nothing scheduling a repair. Also
    // before observer delivery, so a `MutationObserver` callback sees the
    // reconciled `activeElement`.
    if !mutation_records.is_empty() {
        elidex_dom_api::focus::reconcile_focus(&mut result.dom, result.document);
    }

    // Phase 1: Save old computed values for entities with AnimStyle (transition detection).
    // Also snapshot entities without AnimStyle but with ComputedStyle, so that
    // entities gaining AnimStyle in this render cycle have a baseline for transitions.
    let old_styles = collect_old_anim_styles(&result.dom);
    let old_computed_no_anim = collect_computed_without_anim(&result.dom);

    // Phase 2: Re-resolve styles.
    //
    // DOM-as-truth CSSOM (S5-6 §4.2): re-collect the cascade-input stylesheets
    // from the live DOM's `<style>` / loaded `<link rel="stylesheet">` owners
    // every frame instead of reading a shell-side shadow copy. The VM's
    // `insertRule` / `deleteRule` write back to those owner sources
    // (`elidex-dom-api::cssom_sheet`), and this re-collection picks the changed
    // owners up (version-compared, re-parsing only what diverged) — the
    // replacement for the deleted boa CSSOM shadow-sync. The compat/registry
    // parser is dependency-injected (F3) so vendor-prefix normalisation +
    // `transition-*`/`animation-*` handler dispatch survive a script re-parse,
    // matching the build-time cascade's `parse_compat_stylesheet_with_registry`.
    let registry = &result.registry;
    let collected =
        elidex_dom_api::collect_document_stylesheets(result.document, &mut result.dom, |css| {
            std::sync::Arc::new(parse_compat_stylesheet_with_registry(
                css,
                elidex_css::Origin::Author,
                Some(registry),
            ))
        });
    let stylesheet_refs: Vec<&Stylesheet> = collected.iter().map(|s| s.as_ref()).collect();
    result.viewport_overflow = resolve_with_mode(
        &mut result.dom,
        &stylesheet_refs,
        &result.registry,
        result.viewport,
        elidex_css::media::Medium::Screen,
        result.engine_mode,
    );

    // Phase 3: Detect transitions by comparing old vs new computed values.
    // Includes entities that newly gained AnimStyle (transition-* properties).
    detect_and_start_transitions(result, &old_styles, &old_computed_no_anim);

    // Phase 3b: Start/cancel CSS animations based on animation-name changes.
    // CSS Animations L1 §4.2: when animation-name changes, old names are cancelled
    // and new names are started.
    sync_css_animations(result, &old_styles);

    // Phase 4: Apply animated values from active transitions/animations to ComputedStyle.
    apply_active_animations(result);

    layout_tree(&mut result.dom, result.viewport, &result.font_db);

    result.display_list = build_display_list_with_scroll(
        &result.dom,
        &result.font_db,
        result.caret_visible,
        result.scroll_offset,
    );

    mutation_records
}

/// Run the browser from a URL string, opening a window.
///
/// Parses the URL, fetches the page and its resources, executes scripts,
/// renders the result, and runs the event loop.
///
/// Content processing runs on a dedicated thread.
///
/// This function blocks until the window is closed.
pub fn run_url(url_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = url::Url::parse(url_str)
        .map_err(|e| elidex_navigation::LoadError::InvalidUrl(format!("{url_str}: {e}")))?;

    let event_loop = EventLoop::<WakeEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new_threaded_url(url, proxy);
    event_loop.run_app(&mut app)?;

    Ok(())
}
