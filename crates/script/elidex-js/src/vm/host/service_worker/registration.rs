//! `ServiceWorkerRegistration` (WHATWG SW §3.2; D-19 PR-3).
//!
//! Own brand `ObjectKind::ServiceWorkerRegistration`; the canonical scope is
//! recovered from `VmInner::sw_registration_states` (the brand-check +
//! reverse map), and the registration data lives in the per-realm
//! `sw_registrations` registry keyed by scope.  Identity is one-per-scope via
//! the `wrapper_intern` seam (`registration_object`), so `reg ===
//! getRegistration()`.
//!
//! The `installing`/`waiting`/`active` getters return the registration's one
//! worker (the shell's single-`SwState` model) iff its state matches the slot
//! (SW §3.2.1-3): `installing` ↔ `installing`, `waiting` ↔ `installed`,
//! `active` ↔ {`activating`, `activated`}; else `null`.

#![cfg(feature = "engine")]

use elidex_api_sw::{SwClientRequest, SwState};

use super::super::super::natives_promise::create_promise;
use super::super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::super::super::{NativeFn, VmInner};
use super::{
    alloc_client_prototype, install_interface, install_ro_getter, reject_promise, worker_object,
};

// ---------------------------------------------------------------------------
// Interface registration
// ---------------------------------------------------------------------------

/// Allocate `ServiceWorkerRegistration.prototype`, install the `scope` /
/// `installing` / `waiting` / `active` / `updateViaCache` accessors, the
/// `update` / `unregister` methods, and the `onupdatefound` handler attr.
pub(crate) fn register_service_worker_registration_interface(vm: &mut VmInner) {
    let proto = alloc_client_prototype(vm);
    let methods: &[(&str, NativeFn)] = &[
        ("update", native_registration_update),
        ("unregister", native_registration_unregister),
    ];
    vm.install_methods(proto, methods);

    install_ro_getter(vm, proto, "scope", native_registration_get_scope);
    install_ro_getter(vm, proto, "installing", native_registration_get_installing);
    install_ro_getter(vm, proto, "waiting", native_registration_get_waiting);
    install_ro_getter(vm, proto, "active", native_registration_get_active);
    install_ro_getter(
        vm,
        proto,
        "updateViaCache",
        native_registration_get_update_via_cache,
    );

    vm.install_vm_object_handler_attrs(proto, &["onupdatefound"]);
    vm.sw_registration_prototype = Some(proto);
    install_interface(vm, proto, "ServiceWorkerRegistration");
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Brand-check that `this` is a `ServiceWorkerRegistration` and return its
/// canonical scope string (the key into `sw_registrations`).
fn require_registration_scope(ctx: &NativeContext<'_>, this: JsValue) -> Result<String, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(
            ctx.vm.get_object(id).kind,
            ObjectKind::ServiceWorkerRegistration
        ) {
            if let Some(scope) = ctx.vm.sw_registration_states.get(&id) {
                return Ok(scope.clone());
            }
        }
    }
    Err(VmError::type_error(
        "Illegal invocation: receiver is not a ServiceWorkerRegistration",
    ))
}

// ---------------------------------------------------------------------------
// scope / installing / waiting / active / updateViaCache — §3.2.1-6
// ---------------------------------------------------------------------------

/// `registration.scope` getter (SW §3.2.1).
fn native_registration_get_scope(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = require_registration_scope(ctx, this)?;
    let sid = ctx.vm.strings.intern(&scope);
    Ok(JsValue::String(sid))
}

/// Shared `installing`/`waiting`/`active` getter: the registration's worker
/// iff its state satisfies `slot_matches`, else `null`.
fn worker_slot(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    slot_matches: fn(SwState) -> bool,
) -> Result<JsValue, VmError> {
    let scope = require_registration_scope(ctx, this)?;
    let Some(entry) = ctx.vm.sw_registrations.get(&scope) else {
        return Ok(JsValue::Null);
    };
    let scope_sid = entry.scope_sid;
    let matches = entry.worker.as_ref().is_some_and(|w| slot_matches(w.state));
    if matches {
        Ok(JsValue::Object(worker_object(ctx.vm, &scope, scope_sid)))
    } else {
        Ok(JsValue::Null)
    }
}

/// `registration.installing` getter (SW §3.2.2): the worker iff `installing`.
fn native_registration_get_installing(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    worker_slot(ctx, this, |s| matches!(s, SwState::Installing))
}

/// `registration.waiting` getter (SW §3.2.3): the worker iff `installed`.
fn native_registration_get_waiting(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    worker_slot(ctx, this, |s| matches!(s, SwState::Installed))
}

/// `registration.active` getter (SW §3.2.4): the worker iff `activating` or
/// `activated`.
fn native_registration_get_active(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    worker_slot(ctx, this, |s| {
        matches!(s, SwState::Activating | SwState::Activated)
    })
}

/// `registration.updateViaCache` getter (SW §3.2.6).
fn native_registration_get_update_via_cache(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = require_registration_scope(ctx, this)?;
    let uvc = ctx
        .vm
        .sw_registrations
        .get(&scope)
        .map(|e| e.update_via_cache)
        .unwrap_or_default();
    let sid = ctx.vm.strings.intern(uvc.as_str());
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// update() / unregister() — §3.2.8 / §3.2.9
// ---------------------------------------------------------------------------

/// `registration.update()` → `Promise<ServiceWorkerRegistration>` (SW §3.2.8):
/// stage an `Update` request and leave the promise pending until a
/// `Registered`-shaped deliver carries the updated worker.
fn native_registration_update(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = match require_registration_scope(ctx, this) {
        Ok(s) => s,
        Err(e) => return Ok(JsValue::Object(reject_promise(ctx.vm, &e))),
    };
    let promise = create_promise(ctx.vm);
    ctx.vm
        .pending_registration_promises
        .entry(scope.clone())
        .or_default()
        .push(promise);
    ctx.vm
        .queue_sw_client_request(SwClientRequest::Update { scope });
    Ok(JsValue::Object(promise))
}

/// `registration.unregister()` → `Promise<boolean>` (SW §3.2.9): stage an
/// `Unregister` request and leave the promise pending until an `Unregistered`
/// deliver settles it with the removed/not-removed boolean.
fn native_registration_unregister(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let scope = match require_registration_scope(ctx, this) {
        Ok(s) => s,
        Err(e) => return Ok(JsValue::Object(reject_promise(ctx.vm, &e))),
    };
    let promise = create_promise(ctx.vm);
    ctx.vm
        .pending_unregister_promises
        .entry(scope.clone())
        .or_default()
        .push(promise);
    ctx.vm
        .queue_sw_client_request(SwClientRequest::Unregister { scope });
    Ok(JsValue::Object(promise))
}
