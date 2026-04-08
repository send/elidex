//! ScriptEngine trait implementation for the elidex-js VM.
//!
//! Enabled by the `engine` feature flag. Provides a thin delegation layer
//! from the ScriptEngine trait to the VM's native API.

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
        // Stub — event listener invocation requires full DOM integration.
    }

    fn remove_listener(&mut self, _listener_id: ListenerId) {
        // Stub — no listener store in the VM yet.
    }

    fn run_microtasks(&mut self, _ctx: &mut ScriptContext<'_>) {
        // Stub — no microtask queue in the VM yet.
    }

    fn drain_reactions(&mut self, _ctx: &mut ScriptContext<'_>) {
        // Stub — no custom element support in the VM yet.
    }

    fn drain_timers(&mut self, _ctx: &mut ScriptContext<'_>) -> Vec<EvalResult> {
        // Stub — timer management is not yet implemented.
        Vec::new()
    }
}
