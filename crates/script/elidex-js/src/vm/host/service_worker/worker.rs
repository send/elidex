//! `ServiceWorker` (WHATWG SW §3.1; D-19 PR-3) — a registration's
//! `installing`/`waiting`/`active` worker and `navigator.serviceWorker.controller`.
//!
//! Own brand `ObjectKind::ServiceWorker`; the canonical scope is recovered
//! from `VmInner::service_worker_states`, and the worker's `scriptURL`/`state`
//! live in the `sw_registrations` registry entry for that scope.  Identity is
//! one-per-scope via the `wrapper_intern` seam (`worker_object`) and preserved
//! across state transitions (the deliver mutates the registry state in place,
//! never re-mints), so `reg.active === controller` and a captured worker keeps
//! firing `statechange` (`#update-worker-state`).

#![cfg(feature = "engine")]

use elidex_api_sw::{SwClientRequest, SwState};

use super::super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::super::super::{NativeFn, VmInner};
use super::{alloc_client_prototype, install_interface, install_ro_getter};

// ---------------------------------------------------------------------------
// Interface registration
// ---------------------------------------------------------------------------

/// Allocate `ServiceWorker.prototype`, install the `scriptURL` / `state`
/// accessors, the `postMessage` method, and the `onstatechange` handler attr.
pub(crate) fn register_service_worker_interface(vm: &mut VmInner) {
    let proto = alloc_client_prototype(vm);
    let methods: &[(&str, NativeFn)] = &[("postMessage", native_worker_post_message)];
    vm.install_methods(proto, methods);

    install_ro_getter(vm, proto, "scriptURL", native_worker_get_script_url);
    install_ro_getter(vm, proto, "state", native_worker_get_state);

    vm.install_vm_object_handler_attrs(proto, &["onstatechange"]);
    vm.sw_worker_prototype = Some(proto);
    install_interface(vm, proto, "ServiceWorker");
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Brand-check that `this` is a `ServiceWorker` and return its canonical scope
/// string (the key into `sw_registrations`).
fn require_worker_scope(ctx: &NativeContext<'_>, this: JsValue) -> Result<String, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::ServiceWorker) {
            if let Some(scope) = ctx.vm.service_worker_states.get(&id) {
                return Ok(scope.clone());
            }
        }
    }
    Err(VmError::type_error(
        "Illegal invocation: receiver is not a ServiceWorker",
    ))
}

// ---------------------------------------------------------------------------
// scriptURL / state — §3.1.2 / §3.1.3
// ---------------------------------------------------------------------------

/// `ServiceWorker.scriptURL` getter (SW §3.1.2) — immutable per spec; read
/// from the registry entry (empty once the registration is gone / redundant).
fn native_worker_get_script_url(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = require_worker_scope(ctx, this)?;
    let url = ctx
        .vm
        .sw_registrations
        .get(&scope)
        .and_then(|e| e.worker.as_ref())
        .map(|w| w.script_url.clone())
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(&url);
    Ok(JsValue::String(sid))
}

/// `ServiceWorker.state` getter (SW §3.1.3) — read from the registry entry; a
/// worker whose registration was removed reads `redundant`.
fn native_worker_get_state(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = require_worker_scope(ctx, this)?;
    let state = ctx
        .vm
        .sw_registrations
        .get(&scope)
        .and_then(|e| e.worker.as_ref())
        .map_or(SwState::Redundant, |w| w.state);
    let sid = ctx.vm.strings.intern(state.as_str());
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// postMessage(message) — §3.1.4
// ---------------------------------------------------------------------------

/// `ServiceWorker.postMessage(message)` (SW §3.1.4): serialize the message
/// (StructuredClone parity — circular refs throw `DataCloneError`) and stage a
/// `PostMessage` request routed to this worker's scope.  Returns `undefined`.
fn native_worker_post_message(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = require_worker_scope(ctx, this)?;
    let data = args.first().copied().unwrap_or(JsValue::Undefined);
    let serialized = super::super::worker_scope::serialize_message(ctx, data)?;
    ctx.vm
        .queue_sw_client_request(SwClientRequest::PostMessage {
            scope,
            data: serialized,
        });
    Ok(JsValue::Undefined)
}
