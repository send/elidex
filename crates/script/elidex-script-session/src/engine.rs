//! Engine-agnostic script execution interface.
//!
//! Enables the shell and navigation layers to work with any script engine
//! (boa, future elidex-js) without depending on engine-specific types.

use std::time::Instant;

use elidex_ecs::{EcsDom, Entity};

use crate::event_dispatch::DispatchEvent;
use crate::event_listener::ListenerId;
use crate::mutation::MutationRecord;
use crate::navigation::{HistoryAction, NavigationRequest};
use crate::session::SessionCore;

/// Result of evaluating a script.
#[derive(Clone, Debug)]
pub struct EvalResult {
    /// `true` if the script completed without error.
    pub success: bool,
    /// Error message if the script failed, `None` if success.
    pub error: Option<String>,
}

/// Grouped context for script engine calls.
///
/// Bundles the session state, ECS DOM, and document entity that every
/// `ScriptEngine` method needs. Constructed at call sites to avoid
/// repeating the same three arguments everywhere.
pub struct ScriptContext<'a> {
    pub session: &'a mut SessionCore,
    pub dom: &'a mut EcsDom,
    pub document: Entity,
}

impl<'a> ScriptContext<'a> {
    /// Create a new script context.
    pub fn new(session: &'a mut SessionCore, dom: &'a mut EcsDom, document: Entity) -> Self {
        Self {
            session,
            dom,
            document,
        }
    }
}

/// Engine-agnostic script execution interface — the reusable JS-execution
/// primitive (also the model workers / service workers run on directly).
///
/// The shell's per-turn *host-drive* surface — pumping the event loop,
/// exchanging host effects (mutation records, network ticks, navigation
/// intents), and the security/navigation context — lives in the sibling
/// [`HostDriver`] trait, not here: executing a script and driving the shell
/// pipeline are two different kinds of processing with two cohesive homes.
pub trait ScriptEngine {
    /// Evaluate a JavaScript source string.
    fn eval(&mut self, source: &str, ctx: &mut ScriptContext<'_>) -> EvalResult;

    /// Invoke a single event listener by ID.
    ///
    /// Called by the shared `script_dispatch_event` function for each
    /// matching listener during the 3-phase dispatch loop. The engine
    /// creates the JS event object, calls the JS function, and syncs
    /// `event.flags` back after the call.
    ///
    /// `passive` indicates whether the listener was registered with
    /// `{ passive: true }` — if so, `preventDefault()` must be a no-op.
    fn call_listener(
        &mut self,
        listener_id: ListenerId,
        event: &mut DispatchEvent,
        current_target: Entity,
        passive: bool,
        ctx: &mut ScriptContext<'_>,
    );

    /// Remove the engine-side callback for a listener (e.g. from `HostBridge`).
    ///
    /// Called by the shared dispatch function after removing a `{ once: true }`
    /// listener from `EventListeners` to prevent leaking the JS function object.
    fn remove_listener(&mut self, listener_id: ListenerId);

    /// Drain the microtask queue (Promise .then(), queueMicrotask, etc.).
    fn run_microtasks(&mut self, ctx: &mut ScriptContext<'_>);

    /// Drain queued events and custom element lifecycle reactions.
    fn drain_reactions(&mut self, ctx: &mut ScriptContext<'_>);

    /// Drain and execute all ready timers.
    fn drain_timers(&mut self, ctx: &mut ScriptContext<'_>) -> Vec<EvalResult>;
}

/// The shell↔engine host-drive contract — how the main-thread shell pipeline
/// pumps the event loop and exchanges host effects with the script engine.
///
/// Sibling to [`ScriptEngine`] (which is "execute JS / dispatch an event").
/// `HostDriver` is "the shell drives the loop + configures the browsing
/// context across the host boundary": batch lifecycle (binding the engine to a
/// run of calls), per-turn host→engine deliveries (mutation records, network
/// ticks, observer firings), engine→host drains (worker / service-worker
/// requests, the next timer deadline), the navigation/history back-channel, the
/// per-browsing-context security context, and one-time host-resource install.
///
/// Each method is a distinct host-boundary exchange the engine genuinely
/// performs — the size is intrinsic to the contract, not a God-object grab-bag.
/// The trait reflects the **engine-native** event-loop model, not any single
/// engine's internal shape: `tick_network` is one fused step (fetch settlement,
/// WebSocket/`EventSource` dispatch, and a microtask checkpoint), realtime/worker
/// shutdown is folded into [`unbind`](Self::unbind) rather than exposed as
/// separate methods, and same-window `postMessage` is internalized — so this
/// surface is *smaller* than a per-effect-drain contract in those places.
///
/// **Object-safety**: [`with_bound`](Self::with_bound) is generic, so the trait
/// is intentionally not object-safe. There is exactly one engine type, so the
/// shell pipeline is generic `E: ScriptEngine + HostDriver` (monomorphised),
/// never `dyn HostDriver`.
///
/// **Accretion**: the contract grows one cohesive method-group per capability
/// as the engine gains features (media queries, visibility/scroll/focus,
/// `window.open` + cross-context `postMessage`, Web Animations) — one home,
/// incremental membership, never two ways.
pub trait HostDriver {
    // ── batch lifecycle (BATCH-BIND model) ────────────────────────────────
    //
    // The engine's bind/unbind are heavy browsing-context-cycle operations, so
    // the shell brackets each engine-driving *batch* (a script-exec loop, a UA
    // event dispatch, a frame drain) with one bind/unbind; the per-turn methods
    // (`ScriptEngine::eval` / `drain_*`, and the deliver/drain methods below)
    // run **assuming bound**. Binding is per-batch, never per-call — a per-call
    // unbind would tear down cross-script wrapper / live-collection / open-IDB
    // state mid-batch.

    /// Open a batch bracket: bind the engine to `ctx` for a run of calls.
    ///
    /// Call **once** at the start of a batch, paired with [`unbind`](Self::unbind)
    /// at the end. Non-re-entrant — batch brackets must not nest.
    ///
    /// # Safety
    ///
    /// `ctx.session` / `ctx.dom` must stay valid and **unaliased** until the
    /// paired [`unbind`](Self::unbind): while bound, the engine may hold raw
    /// pointers to them, so neither the caller nor any trait method may access
    /// `ctx.session` / `ctx.dom` through a `&mut` (the per-turn methods do not
    /// touch `ctx` — they use the bound pointers). The type system cannot
    /// enforce this, hence `unsafe`.
    #[allow(unsafe_code)]
    unsafe fn bind(&mut self, ctx: &mut ScriptContext<'_>);

    /// Close the batch bracket opened by [`bind`](Self::bind), running the
    /// engine's browsing-context-cycle teardown. Safe (and a no-op) when not
    /// bound, so it doubles as the panic-safe `Drop` hook in
    /// [`with_bound`](Self::with_bound).
    fn unbind(&mut self);

    /// RAII sugar over [`bind`](Self::bind)/[`unbind`](Self::unbind): binds, runs
    /// `f`, then unbinds **even if `f` panics**. `f` receives the bound engine
    /// plus `ctx` (the per-turn methods ignore `ctx` under the assume-bound
    /// model). The interleaved shell batch (eval + dispatch + drain, where
    /// dispatch takes the engine and `ctx` separately) uses the explicit
    /// `bind`/`unbind` pair instead; `with_bound` serves tests and single-closure
    /// batches.
    ///
    /// # Safety
    ///
    /// Same contract as [`bind`](Self::bind): `ctx` stays valid + unaliased for
    /// the bracket and **`f` must not access `ctx.session` / `ctx.dom` directly**
    /// (only via the bound engine). The method hands the same `ctx` back to
    /// arbitrary closure code, so the caller must uphold this for `f`.
    #[allow(unsafe_code)]
    unsafe fn with_bound<R>(
        &mut self,
        ctx: &mut ScriptContext<'_>,
        f: impl FnOnce(&mut Self, &mut ScriptContext<'_>) -> R,
    ) -> R;

    // ── host → engine deliver (per-turn; WHATWG HTML §8.1.7.3) ─────────────

    /// Deliver the layout-derived mutation records the shell collected this turn
    /// to any registered `MutationObserver` (the "update the rendering" observer
    /// steps).
    fn deliver_mutation_records(&mut self, records: &[MutationRecord]);

    /// Fire queued `ResizeObserver` callbacks for boxes whose size changed.
    fn deliver_resize_observations(&mut self);

    /// Fire queued `IntersectionObserver` callbacks for targets whose
    /// intersection changed.
    fn deliver_intersection_observations(&mut self);

    /// Advance the network/event-loop turn: settle resolved `fetch()` promises,
    /// dispatch `WebSocket` / `EventSource` messages, and run a microtask
    /// checkpoint — one fused step (the engine-native model).
    fn tick_network(&mut self);

    /// Flush every dirty `<canvas>` into its display-list source (HTML §4.12.5),
    /// called each frame alongside [`tick_network`](Self::tick_network).
    fn sync_dirty_canvases(&mut self);

    /// Deliver an inbound `navigator.serviceWorker` client update (controller
    /// change / message) staged by the service-worker coordinator.
    fn deliver_sw_client_update(&mut self, update: elidex_api_sw::SwClientUpdate);

    /// Seed the initial `navigator.serviceWorker` controller + registrations the
    /// page is controlled by AT navigation (WHATWG SW §3.4.1), before any runtime
    /// [`deliver_sw_client_update`](Self::deliver_sw_client_update). An
    /// uncontrolled page passes `None` + an empty slice.
    fn seed_sw_client(
        &mut self,
        controller: Option<url::Url>,
        registrations: &[(url::Url, elidex_api_sw::SwWorkerSnapshot)],
    );

    // ── engine → host drain / read (per-turn) ──────────────────────────────

    /// Deliver any parent-side `postMessage` from dedicated/shared workers that
    /// arrived since the last turn.
    fn drain_worker_messages(&mut self);

    /// Take the outbound `navigator.serviceWorker` client requests
    /// (register / update / unregister / postMessage) the page staged this turn,
    /// for the shell to forward to the service-worker coordinator.
    fn drain_sw_client_requests(&mut self) -> Vec<elidex_api_sw::SwClientRequest>;

    /// The earliest pending timer deadline (WHATWG HTML §8.7) — the shell
    /// event-loop scheduler's next-wake hint, or `None` when no timer is
    /// scheduled. The next timer that will *actually* fire (lazily-cancelled
    /// timers are skipped), not merely the heap head.
    #[must_use]
    fn next_timer_deadline(&self) -> Option<Instant>;

    /// The `navigator.serviceWorker.controller`'s registration scope
    /// (WHATWG SW §3.4.1), or `None` when the page is uncontrolled.
    #[must_use]
    fn sw_controller_scope(&self) -> Option<url::Url>;

    // ── navigation / history back-channel ──────────────────────────────────
    //
    // The engine's `location` / `history` globals only *enqueue* intents; the
    // shell drains them after each turn, runs the navigate algorithm against its
    // `NavigationController` (the session-history SoT), and pushes the committed
    // URL + history position back. The engine holds only a current-document view.

    /// Commit the current document URL after a navigation load (WHATWG HTML
    /// §7.4.2.2). `None` resets to `about:blank` (the "no active document"
    /// state). Commits **only** the URL — an integrator must call
    /// [`set_origin`](Self::set_origin) alongside it after a cross-origin
    /// navigation so the document origin does not go stale.
    fn set_current_url(&mut self, url: Option<url::Url>);

    /// The current document URL (always `Some` — the engine's browsing context
    /// always has an active document, `about:blank` by default).
    #[must_use]
    fn current_url(&self) -> Option<url::Url>;

    /// Drain the pending navigation request enqueued by `location.assign` / `href=`
    /// / `replace` / `reload` (WHATWG HTML §7.4.2.2). The shell runs the navigate
    /// algorithm with it, then commits via [`set_current_url`](Self::set_current_url).
    fn take_pending_navigation(&mut self) -> Option<NavigationRequest>;

    /// Drain the pending history actions enqueued by `history.back` / `forward` /
    /// `go` / `pushState` / `replaceState` (WHATWG HTML §7.2.5), in FIFO order —
    /// a `Vec` because several synchronous `pushState`/`replaceState` calls in one
    /// turn each commit an independent session-history mutation.
    fn take_pending_history(&mut self) -> Vec<HistoryAction>;

    /// Push the authoritative session-history position — the current entry's
    /// 0-based `index` and total `length` — together (so they never desync) after
    /// a navigation/traversal commit, so `history.length` reads correctly and the
    /// synchronous `pushState` length update (`index + 1`) starts from the right
    /// index.
    fn set_session_history(&mut self, index: usize, length: usize);

    /// `history.length` — the session-history entry count.
    #[must_use]
    fn history_length(&self) -> usize;

    /// Install the navigation referrer exposed as `document.referrer` / the
    /// `Referer` header (already stripped of fragment + credentials per the
    /// referrer-serialisation rules).
    fn set_navigation_referrer(&mut self, referrer: Option<url::Url>);

    // ── security context (per-browsing-context) ────────────────────────────

    /// Install the document's security origin (WHATWG HTML §7.1.1) — the
    /// embedder's load path computes it (`SecurityOrigin::from_url`, or the opaque
    /// sandbox origin) and installs it before scripts run.
    fn set_origin(&mut self, origin: elidex_plugin::SecurityOrigin);

    /// The document's resolved security origin (the installed override, else
    /// derived from `current_url`).
    #[must_use]
    fn origin(&self) -> elidex_plugin::SecurityOrigin;

    /// Install the sandbox flags for this document's browsing context (the
    /// embedder parses `sandbox=""` → `IframeSandboxFlags`).
    fn set_sandbox_flags(&mut self, flags: Option<elidex_plugin::IframeSandboxFlags>);

    /// The sandbox flags for this document's browsing context, if sandboxed.
    #[must_use]
    fn sandbox_flags(&self) -> Option<elidex_plugin::IframeSandboxFlags>;

    /// Whether form submission is allowed (sandbox `allow-forms`; §7.1.5).
    /// `true` on an unsandboxed / un-configured engine.
    #[must_use]
    fn forms_allowed(&self) -> bool;

    /// Whether popups are allowed (sandbox `allow-popups`; §7.1.5).
    /// `true` on an unsandboxed / un-configured engine.
    #[must_use]
    fn popups_allowed(&self) -> bool;

    /// The iframe nesting depth of this document (`0` = top-level).
    #[must_use]
    fn iframe_depth(&self) -> usize;

    /// Set the iframe nesting depth (the embedder's iframe load path drives it).
    fn set_iframe_depth(&mut self, depth: usize);

    // ── page visibility / scroll transport (per-window; S2) ────────────────
    //
    // Visibility is a per-browsing-context UA fact; scroll is per-window
    // viewport geometry. Both are shell-driven transport (not per-entity DOM
    // facts), so they live behind the engine boundary, exchanged here.

    /// Set the page-visibility state (WHATWG HTML §6.2) — the shell drives this
    /// on tab show/hide / window occlusion. `visible = false` ⇒ `document.hidden`
    /// is `true` and `document.visibilityState` is `"hidden"`.
    fn set_visibility(&mut self, visible: bool);

    /// Drain the scroll offset a script requested via `window.scrollTo` /
    /// `scrollBy` (CSSOM View §4) since the last turn, for the shell to apply to
    /// the viewport and then echo back via [`set_scroll_offset`](Self::set_scroll_offset).
    /// `None` when no script scroll is pending.
    #[must_use]
    fn take_pending_scroll(&mut self) -> Option<(f64, f64)>;

    /// Push the viewport's current scroll offset into the engine (CSSOM View §4)
    /// so `window.scrollX` / `scrollY` read the live value after a user
    /// (wheel/keyboard) scroll the shell applied.
    fn set_scroll_offset(&mut self, x: f64, y: f64);

    // ── host-resource install (construction-adjacent) ──────────────────────

    /// Install the `NetworkHandle` the `fetch()` host global uses. Without one,
    /// every `fetch()` rejects with a `TypeError`.
    fn install_network_handle(&mut self, handle: std::rc::Rc<elidex_net::broker::NetworkHandle>);

    /// Install the per-origin IndexedDB backend. When none is installed, the
    /// `indexedDB` host code lazily creates an in-memory backend on first use.
    fn install_idb_backend(&mut self, backend: std::rc::Rc<elidex_indexeddb::IdbBackend>);

    /// Install the cookie jar backing `document.cookie`. Requires a host context
    /// to already be installed on the engine (a no-op otherwise).
    fn install_cookie_jar(&mut self, jar: std::sync::Arc<elidex_net::CookieJar>);
}
