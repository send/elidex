//! `ScriptEngine` + `HostDriver` trait implementations for the elidex-js VM.
//!
//! Enabled by the `engine` feature flag. Provides a thin delegation layer from
//! the engine-agnostic `elidex-script-session` contracts ([`ScriptEngine`] =
//! "execute JS / dispatch an event"; [`HostDriver`] = "the shell pumps the loop
//! and exchanges host effects across the host boundary") to the VM's native API.
//! The contract docs live on the trait declarations; the impls here are thin
//! forwarders to the verified `Vm` / `HostData` bodies.

use std::time::Instant;

use elidex_css::media::{ColorScheme, ReducedMotion};
use elidex_ecs::Entity;
use elidex_script_session::{
    DispatchEvent, EvalResult, HistoryAction, HistoryStepEvents, HostDriver,
    IdbVersionChangeRequest, ListenerId, MutationRecord, NavigationRequest, ParentMessage,
    ScriptContext, ScriptEngine, StorageChange, WindowOpenIntent,
};

use crate::vm::host_data::HostData;
use crate::vm::value::{JsValue, ObjectKind};
use crate::vm::Vm;

/// elidex-js VM backed [`ScriptEngine`] + [`HostDriver`] implementation.
pub struct ElidexJsEngine {
    vm: Vm,
    /// Whether a batch bracket currently has the VM bound (BATCH-BIND model).
    ///
    /// `Vm::bind`/`unbind` are heavy browsing-context-cycle operations, so
    /// the shell brackets each engine-driving *batch* with one
    /// [`bind`](HostDriver::bind)/[`unbind`](HostDriver::unbind); the per-turn
    /// methods (`eval` / `drain_*`) run **assuming bound**. Tracked only to
    /// `debug_assert` the non-re-entrancy invariant (brackets must not nest).
    bound: bool,
}

impl ElidexJsEngine {
    /// Create a new engine with a fresh VM under [`EngineMode::BrowserCompat`]
    /// (the full compat surface — zero behavior change).
    ///
    /// [`EngineMode::BrowserCompat`]: elidex_plugin::EngineMode::BrowserCompat
    pub fn new() -> Self {
        let mut vm = Vm::new();
        vm.install_host_data(HostData::new());
        Self { vm, bound: false }
    }

    /// Create a new engine with a fresh VM under an explicit
    /// [`EngineMode`](elidex_plugin::EngineMode).
    ///
    /// **`#[cfg(test)]` — not a production surface (F10).** The engine-wide mode
    /// is fixed at VM construction and governs the Web-API core/compat install
    /// gate, but `BrowserCore` / `App` must not be selected for a real session
    /// until the async core storage (`#11-async-core-storage-cookiestore`) lands
    /// — see [`EngineMode`](elidex_plugin::EngineMode). Production embedders use
    /// [`ElidexJsEngine::new`] (BrowserCompat); the cfg gate enforces this by
    /// construction. The async-core PR removes the gate.
    #[cfg(test)]
    #[must_use]
    pub fn new_with_mode(engine_mode: elidex_plugin::EngineMode) -> Self {
        let mut vm = Vm::new_with_mode(engine_mode);
        vm.install_host_data(HostData::new());
        Self { vm, bound: false }
    }

    /// Access the underlying VM (e.g., for setting globals from host).
    ///
    /// This is the one remaining concrete touchpoint into VM internals; the
    /// per-turn drive surface is on the [`HostDriver`] trait. The S5 flip audits
    /// the shell so no per-turn drive path reaches through here.
    pub fn vm(&mut self) -> &mut Vm {
        &mut self.vm
    }

    /// Whether scripting is enabled for the bound browsing context
    /// (WHATWG HTML §8.1.3.4 "scripting is disabled"; the gate consulted by
    /// [`run a classic script`](https://html.spec.whatwg.org/#run-a-classic-script)
    /// §8.1.4.4). `true` when no `HostData` is installed or the context is
    /// unsandboxed / grants `allow-scripts`. Reads `HostData::sandbox_flags`,
    /// which requires `HostData` installed but **not** bound — so the gate is
    /// valid before/outside a batch bracket.
    fn scripts_allowed(&mut self) -> bool {
        self.vm.host_data().is_none_or(|hd| hd.scripts_allowed())
    }

    /// Settle the same-window task queue, then custom-element reactions — the
    /// post-turn checkpoint that [`Vm::eval`] runs internally (`interpreter.rs`:
    /// `drain_tasks` → `flush_ce_reactions`) but that the **non-`eval` JS turns**
    /// reach only through the trait `drain_*` methods.
    ///
    /// boa gets this for free by routing event dispatch and timer callbacks
    /// through `eval` (whose drain covers both); the VM fires listeners /
    /// timers via the call path, not `eval`, so without this the tasks an event
    /// handler or timer callback enqueues (`postMessage`, IndexedDB
    /// completions, coalesced `selectionchange`) and the custom-element
    /// reactions its DOM mutations enqueue would sit pending until some later
    /// `eval` instead of settling on this turn. Order mirrors `eval`:
    /// `drain_tasks` (which runs a microtask checkpoint between tasks) before
    /// the CE flush (a task body may enqueue reactions). Both are
    /// reentrancy-guarded, so re-running them after an `eval`-only batch (whose
    /// queues are already empty) is an idempotent no-op.
    fn settle_tasks_and_reactions(&mut self) {
        self.vm.inner.drain_tasks();
        self.vm.inner.flush_ce_reactions();
    }
}

impl Default for ElidexJsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine for ElidexJsEngine {
    fn eval(&mut self, source: &str, _ctx: &mut ScriptContext<'_>) -> EvalResult {
        // HTML §8.1.4.4 "run a classic script": if scripting is disabled for
        // this browsing context (a sandboxed iframe without `allow-scripts`,
        // §8.1.3.4), the script does not run — a silent success, not an error.
        if !self.scripts_allowed() {
            return EvalResult {
                success: true,
                error: None,
            };
        }
        // Assume-bound (BATCH-BIND model): the caller's batch bracket
        // (`bind`/`unbind` or `with_bound`) has bound the VM; `_ctx` is ignored
        // here so the bound raw pointers stay valid. `vm.eval` runs the full
        // post-script microtask + task + CE checkpoint internally (HTML §8.1.4.4
        // "clean up after running script").
        match self.vm.eval(source) {
            Ok(_) => EvalResult {
                success: true,
                error: None,
            },
            Err(e) => EvalResult {
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    fn call_listener(
        &mut self,
        listener_id: ListenerId,
        event: &mut DispatchEvent,
        current_target: Entity,
        passive: bool,
        is_handler: bool,
        _ctx: &mut ScriptContext<'_>,
    ) {
        // §8.1.8.1 event handler processing algorithm step 1 — see
        // `VmInner::scripting_disabled_for_platform_object`. Precedes step
        // 2's "getting the current value" (the reconcile/compile below), so
        // a suppressed target's raw inline handler source is never compiled
        // during dispatch.
        if is_handler
            && self
                .vm
                .inner
                .scripting_disabled_for_platform_object(Some(current_target))
        {
            return;
        }

        // HTML §8.1.8.1: bring an event-handler IDL attribute backing up to
        // date (lazy-compile a pending inline source / drop a cleared one)
        // before resolving its callable, so UA-initiated dispatch honours
        // inline `<body onload="...">`-style handlers identically to the
        // script dispatch walk. No-op for `addEventListener` listeners. It is
        // also the scripting-disabled COMPILE chokepoint (HTML §8.1.8.1
        // "getting the current value of the event handler" step 3.2): when
        // scripting is disabled it does NOT compile a raw inline handler, so
        // the `get_listener` read below returns `None` and the handler does
        // not run — no destructive delete (a compiled callable's value is
        // preserved).
        self.vm
            .inner
            .ensure_event_handler_current(current_target, listener_id);

        // 1. Resolve the listener function ObjectId from HostData's
        //    listener_store.  A miss means the listener was removed
        //    between dispatch-plan freezing and this invocation —
        //    silently no-op (matches WHATWG DOM §2.9 "inner invoke"
        //    step 2, which processes only listeners "whose removed is
        //    false").
        let Some(host) = self.vm.host_data() else {
            return;
        };
        let Some(listener_obj_id) = host.get_listener(listener_id) else {
            return;
        };

        // 2. Build target / currentTarget wrappers via the cached
        //    `create_element_wrapper`.  Both end up rooted in
        //    `wrapper_cache`, so they survive any GC triggered by the
        //    listener body.
        let target_wrapper = self.vm.inner.create_element_wrapper(event.target);
        let current_wrapper = self.vm.inner.create_element_wrapper(current_target);

        // Build the per-listener event object, root it across the
        // listener call (it has no other GC root until the call's
        // arg slot becomes a stack frame), invoke the listener,
        // then sync flag fields back into `event.flags` so the next
        // listener / outer dispatch loop sees prior preventDefault /
        // stopPropagation calls.
        //
        // Microtask checkpoint is NOT performed here — the shared
        // dispatch loop in
        // `elidex_script_session::event_dispatch::script_dispatch_event_core`
        // calls `engine.run_microtasks(ctx)` after every listener
        // invocation (HTML §8.1.7.3).  Timer drain is similarly host
        // event-loop driven, not per-listener.
        let event_obj_id =
            self.vm
                .inner
                .create_event_object(event, target_wrapper, current_wrapper, passive);
        let mut g = self.vm.push_temp_root(JsValue::Object(event_obj_id));
        let _ = g.call(
            listener_obj_id,
            JsValue::Object(current_wrapper),
            &[JsValue::Object(event_obj_id)],
        );
        if let ObjectKind::Event {
            default_prevented,
            propagation_stopped,
            immediate_propagation_stopped,
            ..
        } = g.get_object(event_obj_id).kind
        {
            event.flags.default_prevented = default_prevented;
            event.flags.propagation_stopped = propagation_stopped;
            event.flags.immediate_propagation_stopped = immediate_propagation_stopped;
        }
        // `g` drops here; restores stack to pre-push length, even
        // if the listener body panicked under `catch_unwind`.
    }

    fn remove_listener(&mut self, listener_id: ListenerId) {
        // Goes through `VmInner::remove_listener_and_prune_back_ref`
        // (not `host.remove_listener` directly) so that any
        // AbortSignal back-ref to this listener is also dropped.
        // The {once} auto-removal path that `event_dispatch` invokes
        // through this trait method would otherwise leak both the
        // back-ref entry and the reverse-index slot — see
        // `VmInner::remove_listener_and_prune_back_ref`'s doc.
        self.vm
            .inner
            .remove_listener_and_prune_back_ref(listener_id);
    }

    fn run_microtasks(&mut self, _ctx: &mut ScriptContext<'_>) {
        // HTML §8.1.7.3 perform a microtask checkpoint — drain Promise reactions
        // and queueMicrotask callbacks (PR2 commits 1-5 supply the queue).
        self.vm.inner.drain_microtasks();
    }

    fn drain_reactions(&mut self, _ctx: &mut ScriptContext<'_>) {
        // The post-dispatch checkpoint `script_dispatch_event` runs after the
        // 3-phase listener walk (WHATWG DOM §2.9 "Dispatching events"): deliver
        // the same-window
        // tasks the listeners enqueued (`postMessage`, IndexedDB completions,
        // coalesced `selectionchange`) AND drain the custom-element reactions
        // (WHATWG HTML §4.13.6) their DOM mutations enqueued. boa's
        // `drain_reactions` does exactly this (`drain_queued_events` then CE
        // reactions); the VM reaches both via `settle_tasks_and_reactions`.
        // This is NOT CE-only: listeners run via `call_listener` (the call
        // path, not `eval`), so unlike a script the dispatch turn has no
        // internal `drain_tasks` — it happens here. Assume-bound (runs within
        // the batch bracket); re-draining empty queues after an `eval`-only
        // batch is a no-op. A throwing reaction is contained inside the CE
        // flush and does not propagate (matching the `()` return).
        self.settle_tasks_and_reactions();
    }

    fn drain_timers(&mut self, _ctx: &mut ScriptContext<'_>) -> Vec<EvalResult> {
        // WHATWG HTML §8.7 — fire every expired setTimeout/setInterval callback
        // through the bound VM, one `EvalResult` per callback. Assume-bound:
        // runs within the shell's batch bracket. `inner.drain_timers` runs the
        // full post-callback checkpoint (microtask + same-window task + CE
        // reaction drain) **per fired timer** — matching boa, which runs each
        // ready timer through `eval` — so a `postMessage` / `checkValidity` /
        // DOM mutation an earlier callback made is settled before a later
        // expired timer observes it. The `scripts_allowed` gate is transitive
        // (a script-disabled context never ran the `setTimeout`).
        self.vm
            .inner
            .drain_timers(Instant::now())
            .into_iter()
            .map(|r| match r {
                Ok(()) => EvalResult {
                    success: true,
                    error: None,
                },
                Err(e) => EvalResult {
                    success: false,
                    error: Some(e.to_string()),
                },
            })
            .collect()
    }

    fn bound_dom_mut(&mut self) -> Option<&mut elidex_ecs::EcsDom> {
        // The single-derivation-chain source for bound-path dispatch: when a
        // batch bracket has the VM bound (`self.bound` && HostData `dom_ptr`
        // non-null), hand out the bound `EcsDom` reconstructed from `dom_ptr`
        // so the shared dispatch loop routes dom access through the same raw
        // pointer the VM's natives hold — never a fresh `ctx.dom` reborrow that
        // would invalidate `dom_ptr` under Stacked Borrows
        // (`#11-bound-safe-dispatch-dom-aliasing`). `None` when unbound (no
        // bracket) → the caller falls back to `ctx.dom`.
        if self.bound {
            self.vm.bound_dom_mut()
        } else {
            None
        }
    }
}

impl HostDriver for ElidexJsEngine {
    // ── batch lifecycle (BATCH-BIND; consolidated from the S1a inherent pair) ─

    #[allow(unsafe_code)]
    unsafe fn bind(&mut self, ctx: &mut ScriptContext<'_>) {
        debug_assert!(
            !self.bound,
            "ElidexJsEngine::bind: already bound — batch brackets must not nest"
        );
        // SAFETY: the caller's contract (see `HostDriver::bind`) keeps
        // `ctx.session`/`ctx.dom` valid + unaliased until `unbind`. `Vm::bind`
        // no-ops without an installed `HostData`.
        //
        // Known soundness gap — event dispatch (slot
        // `#11-bound-safe-dispatch-dom-aliasing`). Driving event dispatch under a
        // batch bracket relies on the bound `*mut dom` and the `&mut ctx.dom`
        // reborrows inside the shared `script_dispatch_event` referring to the
        // same `EcsDom` — a Stacked-Borrows aliasing violation, pre-existing in
        // the VM dispatch path (not introduced here). The principled fix is a
        // bound-safe dispatch API designed when the shell wires dispatch
        // bracketing (S5); until then only `eval` / `drain_*` bracketing — the
        // assume-bound trait methods that never touch `ctx` — is fully sound.
        unsafe {
            self.vm.bind(
                std::ptr::from_mut(ctx.session),
                std::ptr::from_mut(ctx.dom),
                ctx.document,
            );
        }
        self.bound = true;
    }

    fn unbind(&mut self) {
        self.vm.unbind();
        self.bound = false;
    }

    #[allow(unsafe_code)]
    unsafe fn with_bound<R>(
        &mut self,
        ctx: &mut ScriptContext<'_>,
        f: impl FnOnce(&mut Self, &mut ScriptContext<'_>) -> R,
    ) -> R {
        // Declared before any statement to satisfy clippy::items_after_statements.
        struct UnbindGuard<'a>(&'a mut ElidexJsEngine);
        impl Drop for UnbindGuard<'_> {
            fn drop(&mut self) {
                self.0.unbind();
            }
        }
        // SAFETY: forwarded to the caller via `HostDriver::with_bound`'s own
        // `# Safety` contract — `ctx` stays valid + unaliased for the bracket
        // and `f` does not touch `ctx.session`/`ctx.dom` directly.
        unsafe { self.bind(ctx) };
        let guard = UnbindGuard(self);
        f(&mut *guard.0, ctx)
    }

    // ── host → engine deliver (per-turn) ──────────────────────────────────

    fn deliver_mutation_records(&mut self, records: &[MutationRecord]) {
        self.vm.deliver_mutation_records(records);
    }

    fn deliver_resize_observations(&mut self) {
        self.vm.deliver_resize_observations();
    }

    fn deliver_intersection_observations(&mut self) {
        self.vm.deliver_intersection_observations();
    }

    fn tick_network(&mut self) {
        self.vm.tick_network();
    }

    fn sync_dirty_canvases(&mut self) {
        self.vm.sync_dirty_canvases();
    }

    fn deliver_sw_client_update(&mut self, update: elidex_api_sw::SwClientUpdate) {
        self.vm.deliver_sw_client_update(update);
    }

    fn seed_sw_client(
        &mut self,
        controller: Option<url::Url>,
        registrations: &[(url::Url, elidex_api_sw::SwWorkerSnapshot)],
    ) {
        self.vm.seed_sw_client(controller, registrations);
    }

    fn deliver_idb_versionchange(
        &mut self,
        db_name: &str,
        old_version: u64,
        new_version: Option<u64>,
    ) {
        // Marshal-only forward to the VM's reconstruct+fire body (the in-VM
        // IDBVersionChangeEvent UA-fire seam), reaching `vm.inner` directly
        // like `deliver_history_step_events`.
        self.vm
            .inner
            .deliver_idb_versionchange(db_name, old_version, new_version);
    }

    // ── engine → host drain / read (per-turn) ─────────────────────────────

    fn drain_worker_messages(&mut self) {
        self.vm.drain_worker_messages();
    }

    fn drain_sw_client_requests(&mut self) -> Vec<elidex_api_sw::SwClientRequest> {
        self.vm.drain_sw_client_requests()
    }

    // ── cross-context effect drains (per-turn; S5-6a) ─────────────────────
    //
    // All four queues live on the per-VM `HostData` (the transient
    // event-queue standing of the navigation back-channel) and survive
    // `unbind`, so the shell can drain them after the batch bracket closes.
    // Each drains empty on an engine without an installed host context.

    fn take_pending_storage_changes(&mut self) -> Vec<StorageChange> {
        // A2: the Web Storage family (queue included) is compat-webapi-gated;
        // an app-profile build has no storage natives to enqueue, so the
        // drain is a constant empty Vec there.
        #[cfg(feature = "compat-webapi")]
        {
            self.vm
                .host_data()
                .map_or_else(Vec::new, HostData::take_pending_storage_changes)
        }
        #[cfg(not(feature = "compat-webapi"))]
        {
            Vec::new()
        }
    }

    fn take_pending_idb_versionchange_requests(&mut self) -> Vec<IdbVersionChangeRequest> {
        self.vm
            .host_data()
            .map_or_else(Vec::new, HostData::take_pending_idb_versionchange_requests)
    }

    fn take_pending_focus(&mut self) -> bool {
        self.vm
            .host_data()
            .is_some_and(HostData::take_pending_focus)
    }

    fn take_pending_parent_messages(&mut self) -> Vec<ParentMessage> {
        self.vm
            .host_data()
            .map_or_else(Vec::new, HostData::take_pending_parent_messages)
    }

    fn next_timer_deadline(&self) -> Option<Instant> {
        self.vm.inner.next_timer_deadline()
    }

    fn sw_controller_scope(&self) -> Option<url::Url> {
        self.vm.inner.sw_controller_scope_url()
    }

    // ── navigation / history back-channel (consolidated from S1c inherent) ─
    //
    // The session history of record is the shell's `NavigationController`; the
    // VM holds only a current-document view. `current_url` is always `Some`
    // (the VM's browsing context always has an active document, `about:blank`
    // by default — unlike boa's `None`-when-unset). `set_current_url` commits
    // only the URL — an S5 integrator must call `set_origin` alongside it after
    // a cross-origin navigation so the S1b `document_origin` override is not
    // left stale (slot `#11-vm-navigation-origin-resync`).

    fn set_current_url(&mut self, url: Option<url::Url>) {
        self.vm.inner.navigation.set_current_url(url);
    }

    fn current_url(&self) -> Option<url::Url> {
        Some(self.vm.inner.navigation.current_url.clone())
    }

    fn take_pending_navigation(&mut self) -> Option<NavigationRequest> {
        self.vm.inner.navigation.pending_navigation.take()
    }

    fn take_pending_history(&mut self) -> Vec<HistoryAction> {
        std::mem::take(&mut self.vm.inner.navigation.pending_history).into()
    }

    fn take_pending_window_opens(&mut self) -> Vec<WindowOpenIntent> {
        std::mem::take(&mut self.vm.inner.navigation.pending_window_open).into()
    }

    fn set_session_history(&mut self, index: usize, length: usize) {
        // index + length pushed together so they never desync (a `back` moves
        // the index without changing length, so pushing only length would leave
        // synchronous `pushState`'s `length = index + 1` over-counting).
        self.vm.inner.navigation.current_index = index;
        self.vm.inner.navigation.history_length = length;
    }

    fn history_length(&self) -> usize {
        self.vm.inner.navigation.history_length
    }

    fn set_navigation_referrer(&mut self, referrer: Option<url::Url>) {
        self.vm.set_navigation_referrer(referrer);
    }

    fn set_history_state(&mut self, serialized_state: Option<Vec<u8>>) {
        // Restore-WITHOUT-fire (§7.4.6.2 step 6.3): StructuredDeserialize the
        // bytes → `history.state`, no popstate (the cross-document traversal case
        // is `documentIsNew=true`). Marshal-only; reaches `vm.inner` directly.
        self.vm.inner.seed_history_state(serialized_state);
    }

    // ── history-step event delivery (per-navigation; §7.4.6.2) ────────────

    fn deliver_history_step_events(&mut self, ev: HistoryStepEvents) {
        // Marshal-only: decompose the engine-independent decision and forward to
        // the VM's reconstruct+fire body (popstate SYNC, hashchange ENQUEUED),
        // reaching `vm.inner` directly like the navigation back-channel above.
        self.vm
            .inner
            .deliver_history_step_events(ev.popstate_state, ev.hashchange);
    }

    // ── security context (consolidated from S1b inherent) ─────────────────
    //
    // The read accessors take `&self` and read `self.vm.inner.host_data`
    // directly (like `origin`), rather than the `&mut`-returning
    // `Vm::host_data()`, so the S5 shell read-sites need only a shared borrow.
    // Each defaults permissive on an un-`HostData`-installed VM (so the absence
    // of a security context never silently denies), exactly like
    // `scripts_allowed`.

    fn set_origin(&mut self, origin: elidex_plugin::SecurityOrigin) {
        if let Some(hd) = self.vm.host_data() {
            hd.set_origin(origin);
        }
    }

    fn origin(&self) -> elidex_plugin::SecurityOrigin {
        self.vm.inner.document_origin()
    }

    fn set_sandbox_flags(&mut self, flags: Option<elidex_plugin::IframeSandboxFlags>) {
        if let Some(hd) = self.vm.host_data() {
            hd.set_sandbox_flags(flags);
        }
    }

    fn sandbox_flags(&self) -> Option<elidex_plugin::IframeSandboxFlags> {
        self.vm
            .inner
            .host_data
            .as_deref()
            .and_then(HostData::sandbox_flags)
    }

    fn forms_allowed(&self) -> bool {
        self.vm
            .inner
            .host_data
            .as_deref()
            .is_none_or(HostData::forms_allowed)
    }

    fn popups_allowed(&self) -> bool {
        self.vm
            .inner
            .host_data
            .as_deref()
            .is_none_or(HostData::popups_allowed)
    }

    fn iframe_depth(&self) -> usize {
        self.vm
            .inner
            .host_data
            .as_deref()
            .map_or(0, HostData::iframe_depth)
    }

    fn set_iframe_depth(&mut self, depth: usize) {
        if let Some(hd) = self.vm.host_data() {
            hd.set_iframe_depth(depth);
        }
    }

    // ── page visibility / scroll transport (per-window; S2) ───────────────

    fn set_visibility(&mut self, visible: bool) {
        // Visibility is browsing-context UA state on `HostData`; a no-op when
        // no host context is installed (like the other `HostData`-backed
        // setters).
        if let Some(hd) = self.vm.host_data() {
            hd.set_visibility(visible);
        }
    }

    fn take_pending_scroll(&mut self) -> Option<(f64, f64)> {
        self.vm.take_pending_scroll()
    }

    fn set_scroll_offset(&mut self, x: f64, y: f64) {
        self.vm.set_scroll_offset(x, y);
    }

    fn set_media_environment(
        &mut self,
        viewport_width: f64,
        viewport_height: f64,
        device_pixel_ratio: f64,
        color_scheme: ColorScheme,
        reduced_motion: ReducedMotion,
    ) {
        self.vm.set_media_environment(
            viewport_width,
            viewport_height,
            device_pixel_ratio,
            color_scheme,
            reduced_motion,
        );
    }

    fn deliver_media_query_changes(&mut self) {
        self.vm.deliver_media_query_changes();
    }

    fn set_screen_dimensions(
        &mut self,
        width: f64,
        height: f64,
        avail_width: f64,
        avail_height: f64,
    ) {
        self.vm
            .set_screen_dimensions(width, height, avail_width, avail_height);
    }

    fn deliver_visual_viewport_events(&mut self) {
        self.vm.deliver_visual_viewport_events();
    }

    // ── host-resource install (construction-adjacent) ─────────────────────

    fn install_network_handle(&mut self, handle: std::rc::Rc<elidex_net::broker::NetworkHandle>) {
        self.vm.install_network_handle(handle);
    }

    fn install_idb_backend(&mut self, backend: std::rc::Rc<elidex_indexeddb::IdbBackend>) {
        self.vm.install_idb_backend(backend);
    }

    fn install_cookie_jar(&mut self, jar: std::sync::Arc<elidex_net::CookieJar>) {
        // The cookie jar lives on `HostData` (a shared cross-cutting host
        // resource), so a host context must already be installed; a no-op
        // otherwise, like the other `HostData`-backed setters above.
        if let Some(hd) = self.vm.host_data() {
            hd.install_cookie_jar(jar);
        }
    }

    // The trait gates this method on elidex-script-session's `web-storage`
    // feature, which this crate's `compat-webapi` enables (Cargo.toml) — so
    // the method and this impl always appear together.
    #[cfg(feature = "compat-webapi")]
    fn install_web_storage(
        &mut self,
        manager: std::sync::Arc<elidex_storage_core::WebStorageManager>,
    ) {
        // Like the cookie jar: a shared cross-cutting session resource on
        // `HostData`; a no-op without an installed host context.  Without an
        // installed manager the Storage natives fall back to the per-VM
        // in-memory `fallback_local_storage` (the hermetic test /
        // unconfigured path).
        if let Some(hd) = self.vm.host_data() {
            hd.install_web_storage(manager);
        }
    }
}
