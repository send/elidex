//! Engine-agnostic script execution interface.
//!
//! Enables the shell and navigation layers to work with any script engine
//! (boa, future elidex-js) without depending on engine-specific types.

use std::time::Instant;

use elidex_css::media::{ColorScheme, ReducedMotion};
use elidex_ecs::{EcsDom, Entity};

use crate::event_dispatch::DispatchEvent;
use crate::event_listener::ListenerId;
use crate::host_effects::{IdbVersionChangeRequest, ParentMessage, StorageChange};
use crate::mutation::MutationRecord;
use crate::navigation::{HistoryAction, HistoryStepEvents, NavigationRequest, WindowOpenIntent};
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
    ///
    /// `is_handler` is the dispatch plan's
    /// [`ListenerPlanEntry::is_handler`](crate::event_dispatch::ListenerPlanEntry::is_handler)
    /// snapshot — `true` for event-handler-derived listeners, which the
    /// engine gates per HTML §8.1.8.1 "the event handler processing
    /// algorithm" step 1 (plain `addEventListener` listeners are never
    /// scripting-gated).
    fn call_listener(
        &mut self,
        listener_id: ListenerId,
        event: &mut DispatchEvent,
        current_target: Entity,
        passive: bool,
        is_handler: bool,
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

    /// Fire `versionchange` at this engine's open IndexedDB connections to
    /// `db_name` (IndexedDB-3 §4.2 Event interfaces, dfn *fire a version
    /// change event*) — the receive half of the cross-context version-change
    /// wire whose emit half is
    /// [`take_pending_idb_versionchange_requests`](Self::take_pending_idb_versionchange_requests):
    /// another context's upgrade-opening engine enqueued the request, the
    /// shell broadcast it, and this call delivers it.  `new_version` is
    /// `None` for a database-deletion version change (the event's
    /// `newVersion` member is null).  A no-op when this engine holds no open
    /// connection to `db_name`.  Runs assuming bound, like the other
    /// `deliver_*` methods.
    fn deliver_idb_versionchange(
        &mut self,
        db_name: &str,
        old_version: u64,
        new_version: Option<u64>,
    );

    // ── engine → host drain / read (per-turn) ──────────────────────────────

    /// Deliver any parent-side `postMessage` from dedicated/shared workers that
    /// arrived since the last turn.
    fn drain_worker_messages(&mut self);

    /// Take the outbound `navigator.serviceWorker` client requests
    /// (register / update / unregister / postMessage) the page staged this turn,
    /// for the shell to forward to the service-worker coordinator.
    fn drain_sw_client_requests(&mut self) -> Vec<elidex_api_sw::SwClientRequest>;

    // ── cross-context effect drains (per-turn) ─────────────────────────────
    //
    // Effects a bound engine cannot deliver itself because the receiver is
    // another browsing context or the OS window: localStorage `storage`
    // broadcasts, cross-tab IndexedDB `versionchange` requests, `window.focus()`
    // requests, and iframe→parent `postMessage`.  Each is enqueued as an
    // intent (the navigation back-channel model) and drained here in FIFO
    // order; the shell routes them through its own IPC / window machinery.

    /// Take the `localStorage` mutation broadcasts staged this turn (WHATWG
    /// HTML §12.2.1 — `setItem` step 7 / `removeItem` step 5 / `clear` step 3
    /// "Broadcast this…"), in mutation order, for the shell to fan out to the
    /// OTHER same-origin contexts (§12.2.1 *broadcast a Storage object* step 3
    /// excludes the originating storage, so the shell never routes one back to
    /// this engine).  The engine enqueues only actual changes — a same-value
    /// `setItem` (step 3.2 "If oldValue is value, then return"), a
    /// `removeItem` of an absent key (step 1), and a `clear` of an empty map
    /// (step 1) all broadcast nothing.
    fn take_pending_storage_changes(&mut self) -> Vec<StorageChange>;

    /// Take the cross-context IndexedDB version-change requests staged this
    /// turn (IndexedDB-3 §4.2, dfn *fire a version change event*) — one per
    /// `indexedDB.open()` that needed an upgrade — for the shell to broadcast
    /// to the other same-origin contexts, whose engines deliver via
    /// [`deliver_idb_versionchange`](Self::deliver_idb_versionchange) (the
    /// receive half of the same wire).
    fn take_pending_idb_versionchange_requests(&mut self) -> Vec<IdbVersionChangeRequest>;

    /// Take the pending `window.focus()` request (WHATWG HTML §6.6.6 Focus
    /// management APIs, the `Window` `focus()` method — `#dom-window-focus`),
    /// draining it: `true` at most once per staged request, then `false`
    /// until a script calls `window.focus()` again.  The engine only relays
    /// the flag — the shell owns focusing the OS window; the §6.6.6 *window
    /// focusing steps*' fidelity (focus stealing gates etc.) is the focus
    /// program's scope, not this transport's.
    fn take_pending_focus(&mut self) -> bool;

    /// Take the iframe→parent `postMessage` intents staged this turn (WHATWG
    /// HTML §9.3.3 Posting messages — `#dom-window-postmessage-options`), in
    /// call order, for the shell to forward to the parent document.  Each
    /// carries its `targetOrigin` verbatim because the §9.3.3 origin gate
    /// compares against the TARGET (parent) window's origin, which only the
    /// receiving side knows — see [`ParentMessage`].  Only an iframe-depth
    /// engine enqueues here; a top-level engine's `postMessage` self-delivers
    /// internally (boa-parity context routing, superseded at S5-8/B1 by the
    /// real `WindowProxy` model).
    fn take_pending_parent_messages(&mut self) -> Vec<ParentMessage>;

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

    /// Drain the `window.open` tab-creation / named-navigation intents
    /// (WHATWG HTML §7.2.2.1) as ONE ordered [`Vec`] in **call order** —
    /// popup (`_blank`) and named opens interleaved on a single FIFO, because
    /// both become user-visible browser actions and the page's issue order
    /// must be preserved (two separate queues would let a later `_blank`
    /// surface before an earlier named MISS). The shell drains this each pump
    /// (see [`WindowOpenIntent`](crate::WindowOpenIntent) for how each variant
    /// routes); the enqueue is popup-gated (a sandbox-blocked popup never
    /// enters the queue). boa's private bridge channels coexist with this
    /// drain only until the S5-6 flip deletes the crate.
    fn take_pending_window_opens(&mut self) -> Vec<WindowOpenIntent>;

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

    /// Seed `history.state` on document construction from the current
    /// session-history entry's serialized state (WHATWG HTML §7.4.6.2 step 6.3
    /// "restore the history object state" — via `StructuredDeserialize`). This is
    /// **restore-WITHOUT-fire**: step 6.3 runs regardless of `documentIsNew`, and
    /// precedes step 8.4 "scripts may run", so a **cross-document traversal** to a
    /// pushState'd entry rebuilds the pipeline (a fresh engine) whose *initial*
    /// scripts must read the restored `history.state` — but fires **no** popstate
    /// (step 6.4 is gated on `documentIsNew=false`; a fresh document is
    /// `documentIsNew=true`). Distinct from [`deliver_history_step_events`], which
    /// FIRES popstate. Installed at the pre-eval chokepoint (alongside the origin /
    /// referrer / viewport seeds). `None` = null state (the common case, and the
    /// **boa** engine, which passes `None` — light-touch).
    ///
    /// [`deliver_history_step_events`]: Self::deliver_history_step_events
    fn set_history_state(&mut self, serialized_state: Option<Vec<u8>>);

    // ── history-step event delivery (per-navigation; WHATWG HTML §7.4.6.2) ──
    //
    // A same-document history-step application (fragment nav — 5b; traversal —
    // 5c) fires popstate + hashchange at the Window. The shell decides WHICH
    // events fire (its session-history entry model, engine-independent) and
    // hands the decision here as a [`HistoryStepEvents`]; the engine
    // reconstructs `history.state` and fires. Mirrors the media group's
    // decision-then-deliver split, but the decision is a per-navigation value,
    // not a stored environment — so this is a single deliver method, not a
    // `set_*` + `deliver_*` pair.

    /// Deliver the popstate / hashchange of a same-document history-step
    /// application (WHATWG HTML §7.4.6.2 "update document for history step
    /// application"). popstate fires **synchronously** with the reconstructed
    /// `history.state` (step 6.4.3); hashchange is **enqueued** as a task
    /// (step 6.4.5), so popstate is observed strictly before hashchange. A
    /// no-op if `ev` fires neither.
    fn deliver_history_step_events(&mut self, ev: HistoryStepEvents);

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
    /// `true` on an unsandboxed / un-configured engine. Implementations
    /// answer via the canonical predicate home `elidex_plugin::sandbox`
    /// over their stored flags.
    #[must_use]
    fn forms_allowed(&self) -> bool;

    /// Whether popups are allowed (sandbox `allow-popups` = the §7.1.5
    /// *sandboxed auxiliary navigation* flag). `true` on an unsandboxed /
    /// un-configured engine. Implementations answer via the canonical
    /// predicate home `elidex_plugin::sandbox` over their stored flags.
    #[must_use]
    fn popups_allowed(&self) -> bool;

    // `modals_allowed` is intentionally NOT on this trait: unlike
    // `forms_allowed` / `popups_allowed` (consulted shell-side for the
    // form-submit / link-target gates), the *sandboxed modals flag* (§7.1.5)
    // is enforced entirely inside the engine's `alert`/`confirm`/`prompt`
    // natives (HTML §8.9.1 *cannot show simple dialogs* step 1) — the shell
    // has no modal gate to drive. So it lives only as the engine-internal
    // predicate (`HostData::modals_allowed` → `elidex_plugin::sandbox`),
    // matching the `scripts_allowed` precedent (also engine-internal, off
    // this trait). Adding it here would be an unconsumed trait surface.

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

    // ── media-query environment transport (per-window; S2 Slice 2b) ────────
    //
    // The window's device facts the media-query evaluator reads (CSSOM-View
    // §4 / Media Queries L4): viewport geometry + resolution + the
    // `prefers-color-scheme` / `prefers-reduced-motion` user preferences.
    // Like visibility/scroll, these are shell-driven transport (not per-entity
    // DOM facts). Split into a state push + a delivery turn, mirroring
    // scroll's `set_scroll_offset` + the observer `deliver_*` pair: the shell
    // pushes facts whenever they change (winit `Resized` / `ScaleFactorChanged`
    // / `ThemeChanged`), then runs the report-changes pass once per
    // update-the-rendering step. (Producer wiring beyond viewport geometry is
    // carved to `#11-media-prefers-features`; the VM path is exercised by VM
    // tests and goes live with the boa→VM cutover, S5.)

    /// Push the window's media-query device facts (CSSOM-View §4.2). Updates
    /// the engine's environment so `window.innerWidth` / `innerHeight` /
    /// `devicePixelRatio` and every live `MediaQueryList.matches` read the new
    /// values. Does NOT fire `change` on its own — the shell calls
    /// [`deliver_media_query_changes`](Self::deliver_media_query_changes) at
    /// the update-the-rendering step to report flips (mirroring
    /// `set_scroll_offset` + the observer deliver split).
    fn set_media_environment(
        &mut self,
        viewport_width: f64,
        viewport_height: f64,
        device_pixel_ratio: f64,
        color_scheme: ColorScheme,
        reduced_motion: ReducedMotion,
    );

    /// Run the CSSOM-View §4.2 "evaluate media queries and report changes"
    /// pass: re-evaluate every live `MediaQueryList` against the current
    /// environment and fire `change` at each whose result flipped since the
    /// last delivery. Idempotent and cheap when nothing flips. The shell calls
    /// this once per update-the-rendering step (the media sibling of
    /// [`deliver_resize_observations`](Self::deliver_resize_observations)).
    fn deliver_media_query_changes(&mut self);

    // ── monitor-dimensions transport (per-window; S5-2 window parity) ───────
    //
    // `screen.width` / `.height` / `.availWidth` / `.availHeight` (CSSOM-View
    // §4.3) report the MONITOR (display) CSS-px size — a device fact DISTINCT
    // from the layout viewport (`innerWidth`). Unlike `set_media_environment`,
    // monitor dims are NOT a media-query input and have NO `change` event, so
    // this is a pure state push with NO paired delivery turn (the shell observes
    // `current_monitor()` and pushes; the producer wiring rides the boa→VM
    // cutover, S5-6; the VM path is exercised by VM tests).

    /// Push the monitor (display) dimensions in CSS px (CSSOM-View §4.3) so
    /// `screen.width` / `.height` / `.availWidth` / `.availHeight` read the new
    /// values. A pure state push — there is no `change` event for `screen` and
    /// monitor dims are not a media input, so (unlike
    /// [`set_media_environment`](Self::set_media_environment)) there is no paired
    /// delivery method. `avail_*` is the OS-chrome-excluded available area (the
    /// full monitor dims until a work-area source lands).
    fn set_screen_dimensions(
        &mut self,
        width: f64,
        height: f64,
        avail_width: f64,
        avail_height: f64,
    );

    /// Run the CSSOM-View §13.1 `VisualViewport` report-changes pass: diff the
    /// current viewport size against the producer's stored prior and fire
    /// `resize` (a `(width, height)` change) at the `visualViewport` singleton.
    /// It does NOT fire `scroll`/`scrollend`: per §13.2 those fire only on a
    /// visual-viewport *offset* change (pinch-zoom pan), which elidex does not
    /// model, so an ordinary layout-viewport scroll is a document scroll
    /// (delivered as `window`/document `scroll`), not a visual-viewport scroll.
    /// The first deliver after a bind fires nothing (the prior is seeded at
    /// `Vm::bind`, the load-time baseline). The shell calls this from its
    /// update-the-rendering step after a resize (the `VisualViewport` sibling of
    /// [`deliver_media_query_changes`](Self::deliver_media_query_changes)).
    fn deliver_visual_viewport_events(&mut self);

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

    /// Install the shared `WebStorageManager` backing `localStorage` (WHATWG
    /// HTML §12.2 — origin-keyed, persistent).  The shell owns ONE
    /// process-wide manager (a shared cross-cutting session resource, the
    /// cookie-jar precedent) and installs it at pipeline construction; an
    /// engine without one falls back to a per-engine in-memory store (the
    /// hermetic test / unconfigured path — data is lost with the engine).
    /// Requires a host context to already be installed (a no-op otherwise),
    /// like [`install_cookie_jar`](Self::install_cookie_jar).
    ///
    /// Feature-gated on `web-storage` (unlike its siblings) because the
    /// backend type carries the A2 absence guarantee — an app-profile build
    /// compiles the whole Web Storage family out; see this crate's
    /// `Cargo.toml` `[features]` note.
    #[cfg(feature = "web-storage")]
    fn install_web_storage(
        &mut self,
        manager: std::sync::Arc<elidex_storage_core::WebStorageManager>,
    );
}
