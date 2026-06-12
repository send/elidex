//! ScriptEngine trait implementation for the elidex-js VM.
//!
//! Enabled by the `engine` feature flag. Provides a thin delegation layer
//! from the ScriptEngine trait to the VM's native API.

use std::time::Instant;

use elidex_ecs::Entity;
use elidex_script_session::{
    DispatchEvent, EvalResult, HistoryAction, ListenerId, NavigationRequest, ScriptContext,
    ScriptEngine,
};

use crate::vm::host_data::HostData;
use crate::vm::value::{JsValue, ObjectKind};
use crate::vm::Vm;

/// elidex-js VM backed `ScriptEngine` implementation.
pub struct ElidexJsEngine {
    vm: Vm,
    /// Whether a batch bracket currently has the VM bound (BATCH-BIND model).
    ///
    /// `Vm::bind`/`unbind` are heavy browsing-context-cycle operations, so
    /// the shell brackets each engine-driving *batch* with one
    /// [`bind`](Self::bind)/[`unbind`](Self::unbind); the trait methods
    /// (`eval` / `drain_*`) run **assuming bound**. Tracked only to
    /// `debug_assert` the non-re-entrancy invariant (brackets must not nest).
    bound: bool,
}

impl ElidexJsEngine {
    /// Create a new engine with a fresh VM.
    pub fn new() -> Self {
        Self {
            vm: Vm::new(),
            bound: false,
        }
    }

    /// Access the underlying VM (e.g., for setting globals from host).
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

    // ---- Shell-facing security context (S1b boa→VM cutover) ----
    //
    // These mirror boa's `HostBridge` accessors (`set_origin`/`origin`,
    // `forms_allowed`/`popups_allowed`, `sandbox_flags`,
    // `iframe_depth`/`set_iframe_depth`).  The shell consumes them today on
    // boa's `bridge()`; the S5 flip rewrites `runtime.bridge().X()` to
    // `runtime.X()` against this engine.  Until then they are exercised by
    // S1b's own tests (boa stays live).  Each None-defaults on an
    // un-`HostData`-installed VM, exactly like `scripts_allowed` above.

    /// Install the document's security origin (WHATWG HTML §7.1.1).  The
    /// embedder's load path computes it (`SecurityOrigin::from_url`, or the
    /// opaque sandbox origin via the shell's `apply_sandbox_origin_from_flags`)
    /// and installs it before scripts run.  No-op without an installed
    /// `HostData`.
    pub fn set_origin(&mut self, origin: elidex_plugin::SecurityOrigin) {
        if let Some(hd) = self.vm.host_data() {
            hd.set_origin(origin);
        }
    }

    /// The document's security origin — the resolved value (the installed
    /// override, else derived from `current_url`).  Parity with boa's
    /// `bridge().origin()`; the shell reads it to compute a child iframe's
    /// origin from its parent (`iframe/lifecycle.rs`).
    #[must_use]
    pub fn origin(&self) -> elidex_plugin::SecurityOrigin {
        self.vm.inner.document_origin()
    }

    /// Install the sandbox flags for this document's browsing context (the
    /// shell's iframe load path parses `sandbox=""` → `IframeSandboxFlags` and
    /// drives this).  No-op without an installed `HostData`.  (The underlying
    /// `HostData::set_sandbox_flags` shipped with S1a's `scripts_allowed` gate;
    /// this is the shell-facing forwarder.)
    pub fn set_sandbox_flags(&mut self, flags: Option<elidex_plugin::IframeSandboxFlags>) {
        if let Some(hd) = self.vm.host_data() {
            hd.set_sandbox_flags(flags);
        }
    }

    // The read accessors take `&self` (boa's `bridge()` getters are `&self`,
    // so the S5 shell read-sites — `event_handlers.rs` popups / `form_input.rs`
    // forms / `iframe/lifecycle.rs` depth — need only a shared borrow). They
    // read `self.vm.inner.host_data` directly, like `origin()` above, rather
    // than the `&mut`-returning `Vm::host_data()`.

    /// The sandbox flags for this document's browsing context, if sandboxed.
    #[must_use]
    pub fn sandbox_flags(&self) -> Option<elidex_plugin::IframeSandboxFlags> {
        self.vm
            .inner
            .host_data
            .as_deref()
            .and_then(HostData::sandbox_flags)
    }

    /// Whether form submission is allowed (sandbox `allow-forms`; §7.1.5).
    /// `true` on an un-`HostData`-installed / unsandboxed VM.
    #[must_use]
    pub fn forms_allowed(&self) -> bool {
        self.vm
            .inner
            .host_data
            .as_deref()
            .is_none_or(HostData::forms_allowed)
    }

    /// Whether popups are allowed (sandbox `allow-popups`; §7.1.5).
    /// `true` on an un-`HostData`-installed / unsandboxed VM.
    #[must_use]
    pub fn popups_allowed(&self) -> bool {
        self.vm
            .inner
            .host_data
            .as_deref()
            .is_none_or(HostData::popups_allowed)
    }

    /// The iframe nesting depth of this document (`0` = top-level).
    #[must_use]
    pub fn iframe_depth(&self) -> usize {
        self.vm
            .inner
            .host_data
            .as_deref()
            .map_or(0, HostData::iframe_depth)
    }

    /// Set the iframe nesting depth (the shell's iframe load path drives it).
    pub fn set_iframe_depth(&mut self, depth: usize) {
        if let Some(hd) = self.vm.host_data() {
            hd.set_iframe_depth(depth);
        }
    }

    // ---- Shell-facing navigation back-channel (S1c boa→VM cutover) ----
    //
    // boa exposes these on its `HostBridge`/`JsRuntime`: the shell drains the
    // engine's pending navigation/history intents after each script turn (the
    // engine's `location`/`history` globals only *enqueue*) and pushes the
    // committed URL + history length back.  Excluded from the `ScriptEngine`
    // trait by design (engine-specific — `elidex-script-session/src/engine.rs`)
    // → inherent here.  The S5 flip rewrites the shell's `runtime.X()` /
    // `runtime.bridge().X()` against this engine; until then boa stays live and
    // these are exercised by S1c's own tests.  The session history of record is
    // the shell's `NavigationController` — the VM holds only a current-document
    // view.

    /// Commit the current document URL after a navigation load (WHATWG HTML
    /// §7.4.2.2).  `None` resets to `about:blank` (the "no active document"
    /// state).  Parity with boa's `bridge().set_current_url`.
    ///
    /// NB this commits **only** the URL — it does **not** resync the document
    /// origin (boa's `set_current_url` also recomputes a cached origin for
    /// storage keying).  An S5 integrator must call [`set_origin`](Self::set_origin)
    /// alongside this after a content-thread navigation, so a cross-origin
    /// navigation does not leave the S1b `document_origin` override stale →
    /// slot `#11-vm-navigation-origin-resync`.
    pub fn set_current_url(&mut self, url: Option<url::Url>) {
        self.vm.inner.navigation.set_current_url(url);
    }

    /// The current document URL — always `Some` (the VM's browsing context
    /// always has an active document, `about:blank` by default; unlike boa's
    /// `None`-when-unset, the VM is spec-faithful here).  Parity with boa's
    /// `bridge().current_url`.
    #[must_use]
    pub fn current_url(&self) -> Option<url::Url> {
        Some(self.vm.inner.navigation.current_url.clone())
    }

    /// Drain the pending navigation request enqueued by `location.assign`/`href=`
    /// /`replace`/`reload` (WHATWG HTML §7.4.2.2).  The shell runs the navigate
    /// algorithm with it, then commits the result via `set_current_url`.
    pub fn take_pending_navigation(&mut self) -> Option<NavigationRequest> {
        self.vm.inner.navigation.pending_navigation.take()
    }

    /// Drain the pending history actions enqueued by `history.back`/`forward`/`go`
    /// /`pushState`/`replaceState` (WHATWG HTML §7.2.5), in FIFO order.  The shell
    /// applies each to its `NavigationController`.  Returns a `Vec` rather than a
    /// single action because synchronous `pushState`/`replaceState` calls each
    /// commit an independent session-history mutation, so a turn may enqueue
    /// several that must all be applied in order (`pending_navigation`, async and
    /// last-wins, stays a single slot).
    pub fn take_pending_history(&mut self) -> Vec<HistoryAction> {
        std::mem::take(&mut self.vm.inner.navigation.pending_history).into()
    }

    /// Push the session-history length into the engine so `history.length` reads
    /// correctly (the shell's `NavigationController` owns the count).  Parity
    /// with boa's `bridge().set_history_length`.
    pub fn set_history_length(&mut self, len: usize) {
        self.vm.inner.navigation.history_length = len;
    }

    /// `history.length` — the shell-pushed session-history entry count.
    #[must_use]
    pub fn history_length(&self) -> usize {
        self.vm.inner.navigation.history_length
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

    /// Open a batch bracket: bind the VM to `ctx` for a run of engine calls
    /// (BATCH-BIND model). The shell calls this **once** at the start of a
    /// batch (script-exec / event dispatch / frame drain) and the paired
    /// [`unbind`](Self::unbind) at the end; `eval` / `drain_*` in between
    /// assume the VM is bound. Binding is **per-batch, never per-call** — the
    /// VM `unbind` is heavy teardown (non-Node wrapper / live-collection / IDB
    /// cleanup) that would corrupt cross-script identity if run between the
    /// evals of one document.
    ///
    /// **Non-re-entrant**: batch brackets must not nest (`Vm::bind` installs a
    /// single `ConsumerDispatcher`). The `bound` flag debug-asserts this with a
    /// clear message ahead of the lower-level dispatcher assert.
    ///
    /// # Safety
    ///
    /// `ctx.session` / `ctx.dom` must stay valid and **unaliased** until the
    /// paired [`unbind`](Self::unbind): while bound, the VM holds raw pointers
    /// to them, so neither the caller nor any trait method may access
    /// `ctx.session` / `ctx.dom` through a `&mut` (the trait methods do not
    /// touch `ctx` — they use the bound pointers). The method is `unsafe`
    /// because the type system cannot enforce this; `Vm::bind` is itself
    /// `unsafe` for the same reason, and a safe wrapper would silently expose
    /// that precondition to safe callers.
    ///
    /// **Known soundness gap — event dispatch (slot
    /// `#11-bound-safe-dispatch-dom-aliasing`).** Driving event dispatch under
    /// a batch bracket relies on the bound `*mut dom` and the `&mut ctx.dom`
    /// reborrows inside the shared `script_dispatch_event`
    /// (`elidex-script-session` — dispatch-path build, retarget, and `{once}`
    /// removal between `call_listener` calls) referring to the same `EcsDom`.
    /// That is a Stacked-Borrows aliasing violation — pre-existing in the VM
    /// dispatch path since the dispatch-integration tests landed (`Vm::bind` +
    /// `script_dispatch_event`), not introduced here. It does not miscompile
    /// today but is unsound under strict aliasing. The principled fix is a
    /// bound-safe dispatch API that does not reborrow the bound DOM, designed
    /// when the shell wires dispatch bracketing (S5); until then, only `eval` /
    /// `drain_*` bracketing — the assume-bound trait methods that never touch
    /// `ctx` — is fully sound. Do not treat dispatch bracketing as settled by
    /// this contract.
    #[allow(unsafe_code)]
    pub unsafe fn bind(&mut self, ctx: &mut ScriptContext<'_>) {
        debug_assert!(
            !self.bound,
            "ElidexJsEngine::bind: already bound — batch brackets must not nest"
        );
        // SAFETY: the caller's contract (above) keeps `ctx.session`/`ctx.dom`
        // valid + unaliased until `unbind`. `Vm::bind` no-ops without an
        // installed `HostData`.
        unsafe {
            self.vm.bind(
                std::ptr::from_mut(ctx.session),
                std::ptr::from_mut(ctx.dom),
                ctx.document,
            );
        }
        self.bound = true;
    }

    /// Close the batch bracket opened by [`bind`](Self::bind), running the VM's
    /// browsing-context-cycle teardown. Safe to call when not bound (the VM
    /// no-ops), so it doubles as the `Drop`-guard hook in
    /// [`with_bound`](Self::with_bound).
    pub fn unbind(&mut self) {
        self.vm.unbind();
        self.bound = false;
    }

    /// RAII sugar over [`bind`](Self::bind)/[`unbind`](Self::unbind): binds,
    /// runs `f`, then unbinds **even if `f` panics** (a `Drop` guard runs
    /// `unbind` on unwind — the VM equivalent of boa's `UnbindGuard`).
    ///
    /// `f` receives the bound engine plus `ctx` (so it can satisfy the
    /// `eval`/`drain_*` signatures — those ignore `ctx` under the assume-bound
    /// model, so re-passing it does not disturb the bound raw pointers).
    ///
    /// # Safety
    ///
    /// Same contract as [`bind`](Self::bind), and `unsafe` for the same reason:
    /// while bound, the VM holds raw pointers into `ctx.session`/`ctx.dom`, so
    /// **`f` must NOT access `ctx.session`/`ctx.dom` directly** (only via the
    /// bound engine) — reborrowing those fields while bound would invalidate
    /// those pointers. The method hands the same `ctx` back to arbitrary
    /// closure code, so the caller must uphold this for `f`. The shell's
    /// interleaved eval+dispatch+drain batch uses the explicit `bind`/`unbind`
    /// pair instead (`script_dispatch_event` takes `engine` and `ctx`
    /// separately, outside any closure); `with_bound` serves tests and
    /// single-closure batches.
    #[allow(unsafe_code)]
    pub unsafe fn with_bound<R>(
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
        // SAFETY: forwarded to the caller via this method's own `# Safety`
        // contract — `ctx` stays valid + unaliased for the bracket and `f`
        // does not touch `ctx.session`/`ctx.dom` directly.
        unsafe { self.bind(ctx) };
        let guard = UnbindGuard(self);
        f(&mut *guard.0, ctx)
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
        _ctx: &mut ScriptContext<'_>,
    ) {
        // HTML §8.1.8.1: bring an event-handler IDL attribute backing up to
        // date (lazy-compile a pending inline source / drop a cleared one)
        // before resolving its callable, so UA-initiated dispatch honours
        // inline `<body onload="...">`-style handlers identically to the
        // script dispatch walk. No-op for `addEventListener` listeners. It is
        // also the scripting-disabled chokepoint (HTML §8.1.8.1 "getting the
        // current value of the event handler" step 3.2): when scripting is
        // disabled it does NOT compile a raw inline handler, so the
        // `get_listener` read below returns `None` and the handler does not run
        // — invocation suppressed by construction, no per-path gate, and no
        // destructive delete (a compiled callable's value is preserved).
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
}
