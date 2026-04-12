//! ScriptEngine trait implementation for the elidex-js VM.
//!
//! Enabled by the `engine` feature flag. Provides a thin delegation layer
//! from the ScriptEngine trait to the VM's native API.

use std::time::Instant;

use elidex_ecs::Entity;
use elidex_script_session::{DispatchEvent, EvalResult, ListenerId, ScriptContext, ScriptEngine};

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
        _listener_id: ListenerId,
        _event: &mut DispatchEvent,
        _current_target: Entity,
        _passive: bool,
        _ctx: &mut ScriptContext<'_>,
    ) {
        // Stub — event listener invocation requires full DOM integration
        // (lands with PR3 when Event/DOM wrappers become available).
    }

    fn remove_listener(&mut self, _listener_id: ListenerId) {
        // Stub — listener store lands with PR3.
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
