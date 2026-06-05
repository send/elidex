//! `ServiceWorkerContainer` — `navigator.serviceWorker` (WHATWG SW §3.4;
//! D-19 PR-3).
//!
//! A **singleton** whose state — controller, the registration registry, the
//! pending `register()` promises, the `ready` promise, the buffered messages —
//! is VM-level (`VmInner`), not per-instance.  Unlike the non-dispatchable
//! `Clients` façade (brand-via-prototype), the container IS an `EventTarget`
//! (`controllerchange` / `message`), so it carries its own
//! `ObjectKind::ServiceWorkerContainer` brand — uniform with the other VmObject
//! EventTargets (WebSocket / EventSource / IdbRequest), which `target_from_this`
//! routes to `vm_event_listeners` by `ObjectKind` (lesson #276: the brand is
//! justified by EventTarget-dispatch-distinctness, not a unique state shape).
//! The singleton is eagerly constructed at realm setup so its `EventListeners`
//! exist before a pre-access `controllerchange` / `message` deliver (NG-5).
//!
//! Layering: every untrusted input (script/scope URLs, options) is validated
//! by the engine-indep `elidex-api-sw` (`validate_registration` →
//! `SwRegisterError`, `default_scope`, `matches_scope`); this native only
//! marshals + maps the typed error 1:1 to a `DOMException`.

#![cfg(feature = "engine")]

use elidex_api_sw::{SwClientRequest, UpdateViaCache};
use url::Url;

use super::super::super::coerce::to_string;
use super::super::super::natives_promise::create_promise;
use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage, StringId,
    VmError,
};
use super::super::super::{NativeFn, VmInner};
use super::{
    alloc_client_prototype, install_interface, install_ro_getter, map_sw_register_error,
    promise_resolve, registration_object, reject_promise, resolve_sw_ready, worker_object,
};

// ---------------------------------------------------------------------------
// Interface registration + the eager singleton
// ---------------------------------------------------------------------------

/// Allocate `ServiceWorkerContainer.prototype`, install the methods +
/// `controller`/`ready` accessors + `onmessage`/`oncontrollerchange` handler
/// attrs, expose the interface, and **eagerly construct the singleton** (the
/// only `ServiceWorkerContainer`; stored as [`VmInner::sw_container`]).  The
/// caller installs it on `navigator.serviceWorker`.
pub(crate) fn register_service_worker_container(vm: &mut VmInner) {
    let proto = alloc_client_prototype(vm);
    let methods: &[(&str, NativeFn)] = &[
        ("register", native_container_register),
        ("getRegistration", native_container_get_registration),
        ("getRegistrations", native_container_get_registrations),
        ("startMessages", native_container_start_messages),
    ];
    vm.install_methods(proto, methods);

    install_ro_getter(vm, proto, "controller", native_container_get_controller);
    install_ro_getter(vm, proto, "ready", native_container_get_ready);

    vm.install_vm_object_handler_attrs(proto, &["onmessage", "oncontrollerchange"]);
    install_interface(vm, proto, "ServiceWorkerContainer");

    let singleton = vm.alloc_object(Object {
        kind: ObjectKind::ServiceWorkerContainer,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    vm.sw_container = Some(singleton);
}

// ---------------------------------------------------------------------------
// Brand check + Promise helpers
// ---------------------------------------------------------------------------

/// Brand-check that `this` is the `ServiceWorkerContainer` singleton (its own
/// `ObjectKind`, uniform with the other VmObject EventTargets, §3.4).
fn require_container(ctx: &NativeContext<'_>, this: JsValue) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if matches!(
            ctx.vm.get_object(id).kind,
            ObjectKind::ServiceWorkerContainer
        ) {
            return Ok(());
        }
    }
    Err(VmError::type_error(
        "Illegal invocation: receiver is not a ServiceWorkerContainer",
    ))
}

// ---------------------------------------------------------------------------
// register(scriptURL, options?) — §3.4.3
// ---------------------------------------------------------------------------

/// `navigator.serviceWorker.register(scriptURL, options?)` →
/// `Promise<ServiceWorkerRegistration>` (SW §3.4.3).  The promise is left
/// pending and settled by a later `Registered` deliver (DR-B'); the four
/// client-side validation failures reject it synchronously.
fn native_container_register(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let Err(e) = require_container(ctx, this) {
        return Ok(JsValue::Object(reject_promise(ctx.vm, &e)));
    }
    match register_outcome(ctx, args) {
        Ok(promise) => Ok(JsValue::Object(promise)),
        Err(e) => Ok(JsValue::Object(reject_promise(ctx.vm, &e))),
    }
}

fn register_outcome(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<ObjectId, VmError> {
    let Some(&script_arg) = args.first() else {
        return Err(VmError::type_error(
            "Failed to execute 'register' on 'ServiceWorkerContainer': \
             1 argument required, but only 0 present.",
        ));
    };
    let script_sid = to_string(ctx.vm, script_arg)?;
    let raw_script = ctx.vm.strings.get_utf8(script_sid);
    let (raw_scope, update_via_cache) =
        read_register_options(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;

    // Resolve the URLs against the document base URL (the same join the
    // content thread / coordinator use, so the canonical scope key matches the
    // coordinator's `SwRegistered.scope` — R2-1).
    let base = ctx.vm.navigation.current_url.clone();
    let Ok(script_url) = base.join(&raw_script) else {
        return Err(VmError::type_error(
            "Failed to register a ServiceWorker: the script URL could not be parsed",
        ));
    };
    let scope_url = match raw_scope {
        Some(s) => base.join(&s).map_err(|_| {
            VmError::type_error(
                "Failed to register a ServiceWorker: the scope URL could not be parsed",
            )
        })?,
        None => elidex_api_sw::default_scope(&script_url),
    };

    // All scheme/origin/scope-path/secure validation lives in the crate; map
    // its typed error 1:1 to a DOMException (SW §3.1).
    if let Err(e) = elidex_api_sw::validate_registration(&script_url, &scope_url, &base) {
        return Err(map_sw_register_error(ctx.vm, &e));
    }

    let canonical = scope_url.as_str().to_owned();
    let promise = create_promise(ctx.vm);
    ctx.vm
        .pending_registration_promises
        .entry(canonical.clone())
        .or_default()
        .push(promise);
    ctx.vm.queue_sw_client_request(SwClientRequest::Register {
        script_url: script_url.as_str().to_owned(),
        scope: canonical,
        update_via_cache,
    });
    Ok(promise)
}

/// Read the `RegistrationOptions` dictionary (SW §3.4.3): `scope` (a
/// `USVString`, default `None`) + `updateViaCache` (default `"imports"`).
fn read_register_options(
    ctx: &mut NativeContext<'_>,
    options: JsValue,
) -> Result<(Option<String>, UpdateViaCache), VmError> {
    let JsValue::Object(opts_id) = options else {
        return Ok((None, UpdateViaCache::default()));
    };
    let scope = read_string_member(ctx, opts_id, "scope")?;
    let update_via_cache = match read_string_member(ctx, opts_id, "updateViaCache")? {
        Some(s) => UpdateViaCache::parse(&s).ok_or_else(|| {
            VmError::type_error(
                "Failed to register a ServiceWorker: updateViaCache must be \
                 \"imports\", \"all\", or \"none\"",
            )
        })?,
        None => UpdateViaCache::default(),
    };
    Ok((scope, update_via_cache))
}

/// Read a `USVString` dictionary member, `None` when absent/nullish.
fn read_string_member(
    ctx: &mut NativeContext<'_>,
    opts_id: ObjectId,
    name: &str,
) -> Result<Option<String>, VmError> {
    let key = PropertyKey::String(ctx.vm.strings.intern(name));
    let val = ctx.get_property_value(opts_id, key)?;
    if matches!(val, JsValue::Undefined | JsValue::Null) {
        return Ok(None);
    }
    let sid = to_string(ctx.vm, val)?;
    Ok(Some(ctx.vm.strings.get_utf8(sid)))
}

// ---------------------------------------------------------------------------
// getRegistration / getRegistrations — §3.4.4 / §3.4.5
// ---------------------------------------------------------------------------

/// `getRegistration(clientURL?)` → `Promise<ServiceWorkerRegistration | undefined>`
/// (SW §3.4.4): the registration whose scope matches `clientURL` (longest
/// scope wins), or `undefined`.
fn native_container_get_registration(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let Err(e) = require_container(ctx, this) {
        return Ok(JsValue::Object(reject_promise(ctx.vm, &e)));
    }
    match get_registration_outcome(ctx, args) {
        Ok(value) => Ok(JsValue::Object(promise_resolve(ctx.vm, value))),
        Err(e) => Ok(JsValue::Object(reject_promise(ctx.vm, &e))),
    }
}

fn get_registration_outcome(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let base = ctx.vm.navigation.current_url.clone();
    let client_url = match args.first().copied() {
        Some(arg) if !matches!(arg, JsValue::Undefined) => {
            let sid = to_string(ctx.vm, arg)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            base.join(&raw).map_err(|_| {
                VmError::type_error("Failed to get a ServiceWorker registration: invalid clientURL")
            })?
        }
        _ => base.clone(),
    };

    // Longest matching in-scope registration (SW "Match Service Worker
    // Registration" `#match-service-worker-registration` / matches_scope).
    let mut best: Option<(String, StringId, usize)> = None;
    for (scope_str, entry) in &ctx.vm.sw_registrations {
        let Ok(scope_url) = Url::parse(scope_str) else {
            continue;
        };
        if elidex_api_sw::matches_scope(&scope_url, &client_url) {
            let len = scope_url.path().len();
            if best.as_ref().is_none_or(|(_, _, best_len)| len > *best_len) {
                best = Some((scope_str.clone(), entry.scope_sid, len));
            }
        }
    }

    Ok(match best {
        Some((scope, scope_sid, _)) => {
            JsValue::Object(registration_object(ctx.vm, &scope, scope_sid))
        }
        None => JsValue::Undefined,
    })
}

/// `getRegistrations()` → `Promise<sequence<ServiceWorkerRegistration>>`
/// (SW §3.4.5): every registration for this realm's origin.  The registry is
/// already origin-scoped (a back-channel deliver only adds this client's
/// origin), so all entries qualify.
fn native_container_get_registrations(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let Err(e) = require_container(ctx, this) {
        return Ok(JsValue::Object(reject_promise(ctx.vm, &e)));
    }
    let regs: Vec<(String, StringId)> = ctx
        .vm
        .sw_registrations
        .iter()
        .map(|(scope, entry)| (scope.clone(), entry.scope_sid))
        .collect();

    // GC-safety: each `registration_object` may allocate (and GC); root the
    // growing element set on the VM stack across the accumulation (the
    // `clients.matchAll` precedent).
    let mut frame = ctx.vm.push_stack_scope();
    let base = frame.saved_len();
    for (scope, scope_sid) in &regs {
        let obj = registration_object(&mut frame, scope, *scope_sid);
        frame.stack.push(JsValue::Object(obj));
    }
    let elements: Vec<JsValue> = frame.stack[base..].to_vec();
    let arr = frame.create_array_object(elements);
    drop(frame);
    Ok(JsValue::Object(promise_resolve(
        ctx.vm,
        JsValue::Object(arr),
    )))
}

// ---------------------------------------------------------------------------
// controller / ready accessors — §3.4.1 / §3.4.2
// ---------------------------------------------------------------------------

/// `navigator.serviceWorker.controller` getter (SW §3.4.1): the active
/// `ServiceWorker` controlling this client, or `null`.
fn native_container_get_controller(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_container(ctx, this)?;
    let Some(scope) = ctx.vm.sw_controller_scope.clone() else {
        return Ok(JsValue::Null);
    };
    let Some(scope_sid) = ctx.vm.sw_registrations.get(&scope).map(|e| e.scope_sid) else {
        return Ok(JsValue::Null);
    };
    Ok(JsValue::Object(worker_object(ctx.vm, &scope, scope_sid)))
}

/// `navigator.serviceWorker.ready` getter (SW §3.4.2): one coalesced
/// `[SameObject]` promise per realm (the `whenDefined` idiom), minted lazily,
/// resolved once with the registration that has an active worker.
fn native_container_get_ready(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_container(ctx, this)?;
    if let Some(p) = ctx.vm.sw_ready_promise {
        return Ok(JsValue::Object(p));
    }
    let promise = create_promise(ctx.vm);
    ctx.vm.sw_ready_promise = Some(promise);
    if let Some((scope, scope_sid)) = active_registration(ctx.vm) {
        resolve_sw_ready(ctx.vm, &scope, scope_sid);
    }
    Ok(JsValue::Object(promise))
}

/// The first registration with a worker in the `active` slot (`ready`'s
/// resolution target — SW §3.4.2 resolves once `registration.active` is set,
/// i.e. a worker reached `activating`, matching the runtime deliver predicate).
fn active_registration(vm: &VmInner) -> Option<(String, StringId)> {
    vm.sw_registrations.iter().find_map(|(scope, entry)| {
        entry
            .worker
            .as_ref()
            .filter(|w| w.state.is_active_slot())
            .map(|_| (scope.clone(), entry.scope_sid))
    })
}

// ---------------------------------------------------------------------------
// startMessages — §3.4.6
// ---------------------------------------------------------------------------

/// `navigator.serviceWorker.startMessages()` (SW §3.4.6): enable the client
/// message queue and flush any buffered `message` events.
fn native_container_start_messages(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_container(ctx, this)?;
    super::deliver::enable_sw_messages(ctx)?;
    Ok(JsValue::Undefined)
}
