//! ScriptEngine trait implementation for the elidex-js VM.
//!
//! Enabled by the `engine` feature flag. Provides a thin delegation layer
//! from the ScriptEngine trait to the VM's native API.

use std::time::Instant;

use elidex_ecs::Entity;
use elidex_script_session::{DispatchEvent, EvalResult, ListenerId, ScriptContext, ScriptEngine};

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
    /// # Safety contract
    ///
    /// `ctx.session` / `ctx.dom` must stay valid and **unaliased** until the
    /// paired [`unbind`](Self::unbind): while bound, the VM holds raw pointers
    /// to them, so neither the caller nor any trait method may access
    /// `ctx.session` / `ctx.dom` through a `&mut` (the trait methods do not
    /// touch `ctx` — they use the bound pointers).
    #[allow(unsafe_code)]
    pub fn bind(&mut self, ctx: &mut ScriptContext<'_>) {
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
    /// model, so re-passing it does not disturb the bound raw pointers). **`f`
    /// must NOT access `ctx.session`/`ctx.dom` directly** (only via the bound
    /// engine) — the same contract as [`bind`](Self::bind): reborrowing those
    /// fields while bound would invalidate the VM's raw pointers. The
    /// shell's interleaved eval+dispatch+drain batch uses the explicit
    /// `bind`/`unbind` pair instead (`script_dispatch_event` takes `engine` and
    /// `ctx` separately, outside any closure); `with_bound` serves tests and
    /// single-closure batches.
    pub fn with_bound<R>(
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
        self.bind(ctx);
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
        // script dispatch walk. No-op for `addEventListener` listeners.
        self.vm
            .inner
            .ensure_event_handler_current(current_target, listener_id);

        // 1. Resolve the listener function ObjectId from HostData's
        //    listener_store.  A miss means the listener was removed
        //    between dispatch-plan freezing and this invocation —
        //    silently no-op (matches WHATWG DOM §2.10 step 5.4 "if
        //    listener's removed is true, then continue").
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
        // WHATWG HTML §4.13.6 — drain custom element reactions enqueued by DOM
        // mutations the shell applied between evals (upgrade / connected /
        // disconnected / attributeChanged callbacks). Assume-bound: runs within
        // the shell's batch bracket. `vm.eval` already flushes reactions
        // enqueued *during* a script at script end (interpreter.rs); this is the
        // post-shell-mutation drain point. Re-flushing an empty queue is a
        // no-op. (Boa's `drain_reactions` also ran `drain_queued_events`; the VM
        // `eval` already does `drain_tasks` inline, so this is CE-only.) A
        // throwing reaction callback is handled inside `flush_ce_reactions` and
        // does not propagate — matching this method's `()` return (boa parity).
        self.vm.inner.flush_ce_reactions();
    }

    fn drain_timers(&mut self, _ctx: &mut ScriptContext<'_>) -> Vec<EvalResult> {
        // WHATWG HTML §8.7 — fire every expired setTimeout/setInterval callback
        // through the bound VM, one `EvalResult` per callback. Assume-bound:
        // runs within the shell's batch bracket. `inner.drain_timers` returns
        // the per-callback fire results and runs a microtask checkpoint after;
        // the `scripts_allowed` gate is transitive (a script-disabled context
        // never ran the `setTimeout` that registers a callback).
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
