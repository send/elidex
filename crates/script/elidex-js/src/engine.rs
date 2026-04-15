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
}

impl ElidexJsEngine {
    /// Create a new engine with a fresh VM.
    pub fn new() -> Self {
        Self { vm: Vm::new() }
    }

    /// Access the underlying VM (e.g., for setting globals from host).
    pub fn vm(&mut self) -> &mut Vm {
        &mut self.vm
    }
}

impl Default for ElidexJsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine for ElidexJsEngine {
    fn eval(&mut self, source: &str, _ctx: &mut ScriptContext<'_>) -> EvalResult {
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
        self.vm.with_temp_root(JsValue::Object(event_obj_id), |vm| {
            let _ = vm.call(
                listener_obj_id,
                JsValue::Object(current_wrapper),
                &[JsValue::Object(event_obj_id)],
            );
            if let ObjectKind::Event {
                default_prevented,
                propagation_stopped,
                immediate_propagation_stopped,
                ..
            } = vm.inner.get_object(event_obj_id).kind
            {
                event.flags.default_prevented = default_prevented;
                event.flags.propagation_stopped = propagation_stopped;
                event.flags.immediate_propagation_stopped = immediate_propagation_stopped;
            }
        });
    }

    fn remove_listener(&mut self, listener_id: ListenerId) {
        if let Some(host) = self.vm.host_data() {
            host.remove_listener(listener_id);
        }
    }

    fn run_microtasks(&mut self, _ctx: &mut ScriptContext<'_>) {
        // HTML §8.1.4.3 microtask checkpoint — drain Promise reactions
        // and queueMicrotask callbacks (PR2 commits 1-5 supply the queue).
        self.vm.inner.drain_microtasks();
    }

    fn drain_reactions(&mut self, _ctx: &mut ScriptContext<'_>) {
        // Stub — custom element lifecycle reactions land with PR5b.
    }

    fn drain_timers(&mut self, _ctx: &mut ScriptContext<'_>) -> Vec<EvalResult> {
        // WHATWG §8.7 timer firing.  PR2 commit 6's drain_timers fires
        // every expired entry and drains microtasks afterwards; failures
        // are reported via eprintln at the moment and do not surface
        // through the EvalResult vector.  Return an empty vec for the
        // current caller shape — the per-callback EvalResult split is
        // tracked as a PR6 follow-up once host.session().log() is
        // wired up.
        let _fired = self.vm.inner.drain_timers(Instant::now());
        Vec::new()
    }
}
