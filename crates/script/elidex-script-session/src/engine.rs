//! Engine-agnostic script execution interface.
//!
//! Enables the shell and navigation layers to work with any script engine
//! (boa, future elidex-js) without depending on engine-specific types.

use elidex_ecs::{EcsDom, Entity};

use crate::event_dispatch::DispatchEvent;
use crate::event_listener::ListenerId;
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

/// Engine-agnostic script execution interface.
///
/// Navigation state methods (`set_current_url`, `take_pending_navigation`, etc.)
/// are intentionally excluded — they are engine-specific (produced by boa's
/// location/history globals) and remain as concrete methods on `JsRuntime`.
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
