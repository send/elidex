//! ScriptEngine trait implementation for the elidex-js VM.
//!
//! Enabled by the `engine` feature flag. Provides a thin delegation layer
//! from the current ScriptEngine trait to the VM's ideal API (`eval`/`call`).
//!
//! When boa is removed (M4-12), the ScriptEngine trait itself will be
//! refactored to match the VM's native API, eliminating this wrapper.

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{DispatchEvent, EvalResult, ScriptEngine, SessionCore};

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
    fn eval(
        &mut self,
        source: &str,
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
        _document: Entity,
    ) -> EvalResult {
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

    fn dispatch_event(
        &mut self,
        _event: &mut DispatchEvent,
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
        _document: Entity,
    ) -> bool {
        // M4-10: Stub — event dispatch requires DOM listener integration
        // which is handled by elidex-js-boa. Full implementation in M4-12
        // when boa is replaced.
        false
    }

    fn drain_timers(
        &mut self,
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
        _document: Entity,
    ) -> Vec<EvalResult> {
        // M4-10: Stub — timer management remains in elidex-js-boa.
        Vec::new()
    }
}
