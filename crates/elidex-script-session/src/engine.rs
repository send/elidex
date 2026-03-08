//! Engine-agnostic script execution interface.
//!
//! Enables the shell and navigation layers to work with any script engine
//! (boa, future elidex-js) without depending on engine-specific types.

use elidex_ecs::{EcsDom, Entity};

use crate::event_dispatch::DispatchEvent;
use crate::session::SessionCore;

/// Result of evaluating a script.
#[derive(Clone, Debug)]
pub struct EvalResult {
    /// `true` if the script completed without error.
    pub success: bool,
    /// Error message if the script failed, `None` if success.
    pub error: Option<String>,
}

/// Engine-agnostic script execution interface.
///
/// Navigation state methods (`set_current_url`, `take_pending_navigation`, etc.)
/// are intentionally excluded — they are engine-specific (produced by boa's
/// location/history globals) and remain as concrete methods on `JsRuntime`.
pub trait ScriptEngine {
    /// Evaluate a JavaScript source string.
    fn eval(
        &mut self,
        source: &str,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> EvalResult;

    /// Dispatch a DOM event through the propagation path.
    /// Returns `true` if `preventDefault()` was called.
    fn dispatch_event(
        &mut self,
        event: &mut DispatchEvent,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> bool;

    /// Drain and execute all ready timers.
    fn drain_timers(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document: Entity,
    ) -> Vec<EvalResult>;
}
