//! Shared fixtures for the `engine`-feature test modules
//! (`tests_call_listener`, `tests_dispatch_integration`, ...).

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{HostDriver, ScriptContext, SessionCore};

use crate::engine::ElidexJsEngine;

/// Run `f` with the engine bound to a fresh `ScriptContext` over `session` / `dom`
/// (ctx first, then `HostDriver::with_bound`, whose guard unbinds on panic before
/// the locals drop).
#[allow(unsafe_code)]
pub(crate) fn with_bound_ctx<R>(
    engine: &mut ElidexJsEngine,
    session: &mut SessionCore,
    dom: &mut EcsDom,
    doc: Entity,
    f: impl FnOnce(&mut ElidexJsEngine, &mut ScriptContext<'_>) -> R,
) -> R {
    let mut ctx = ScriptContext::new(session, dom, doc);
    // SAFETY: `ctx` lives for the whole bracket; `f` gets only the reborrowed ctx
    // and reaches session/dom only through the bound VM.
    unsafe { engine.with_bound(&mut ctx, f) }
}
