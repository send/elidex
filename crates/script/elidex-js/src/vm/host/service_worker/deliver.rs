//! The `navigator.serviceWorker` back-channel deliver (DR-B'; WHATWG SW
//! §3.1/§3.4; D-19 PR-3).
//!
//! The register/state/controller/message/unregister updates arrive **inbound**
//! from the shell coordinator — they are not produced by the synchronous
//! native that created the promise (unlike `CacheDeliver`, which queues its own
//! same-tick outcome).  So the seam is a public `Vm::deliver_sw_client_update`
//! (the window-realm twin of PR-2's SW-thread recv loop, the 7th member of the
//! `vm_api.rs` `deliver_*` family); this module is its body.  It reuses the
//! existing seams — `settle_promise`, `wrapper_intern` identity, the
//! `fire_vm_*` VmObject §2.9 dispatch core — and embeds **no** lifecycle state
//! machine (that stays shell-side).
//!
//! Ordering invariants:
//! - **intern-before-settle (NG-3)**: a `Registered` deliver interns the
//!   registration + worker objects *before* settling `register()`, so a
//!   `.then` reading `.installing` + adding `onstatechange` attaches to the
//!   same object the later `StateChanged` dispatches on.
//! - **mutate-before-dispatch**: a `StateChanged` mutates the registry worker
//!   state in place (identity preserved, `#update-worker-state`) before firing
//!   `statechange`.

#![cfg(feature = "engine")]

use elidex_api_sw::{SwClientUpdate, SwRegisterError, SwState, SwWorkerSnapshot, UpdateViaCache};
use url::Url;

use super::super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::super::VmInner;
use super::super::event_target_dispatch_vm::{dispatch_vm_simple_event, fire_vm_message_event};
use super::{
    map_sw_register_error, registration_object, resolve_sw_ready, worker_object,
    SwRegistrationEntry,
};

/// Dispatch an inbound [`SwClientUpdate`] to the window-realm
/// `navigator.serviceWorker` client (the body of `Vm::deliver_sw_client_update`).
pub(crate) fn deliver_sw_client_update(ctx: &mut NativeContext<'_>, update: SwClientUpdate) {
    match update {
        SwClientUpdate::Registered {
            scope,
            success,
            error,
            worker,
        } => deliver_registered(ctx, &scope, success, error, worker),
        SwClientUpdate::StateChanged { scope, state } => deliver_state_changed(ctx, &scope, state),
        SwClientUpdate::ControllerSet { scope } => deliver_controller_set(ctx, scope),
        SwClientUpdate::Message { data, source_scope } => {
            deliver_message(ctx, &data, source_scope.as_str());
        }
        SwClientUpdate::Unregistered { scope, success } => {
            deliver_unregistered(ctx, &scope, success);
        }
    }
}

// ---------------------------------------------------------------------------
// Registered — §3.4.3 settle (register / update)
// ---------------------------------------------------------------------------

fn deliver_registered(
    ctx: &mut NativeContext<'_>,
    scope: &Url,
    success: bool,
    error: Option<SwRegisterError>,
    worker: Option<SwWorkerSnapshot>,
) {
    let canonical = scope.as_str().to_owned();

    if !success {
        let waiters = ctx
            .vm
            .pending_registration_promises
            .remove(&canonical)
            .unwrap_or_default();
        if waiters.is_empty() {
            return;
        }
        let exc = match error {
            Some(e) => map_sw_register_error(ctx.vm, &e),
            None => VmError::type_error("Service Worker registration failed"),
        };
        let reason = ctx.vm.vm_error_to_thrown(&exc);
        for promise in waiters {
            settle_reject(ctx.vm, promise, reason);
        }
        return;
    }

    // Seed the registry authoritatively (F1) — the write-path `.installing` /
    // `.waiting` / `.active` read from at resolve.
    let scope_sid = ctx.vm.strings.intern(&canonical);
    let prev_state = {
        let entry = ctx
            .vm
            .sw_registrations
            .entry(canonical.clone())
            .or_insert_with(|| SwRegistrationEntry {
                scope_sid,
                update_via_cache: UpdateViaCache::default(),
                worker: None,
            });
        let prev = entry.worker.as_ref().map(|w| w.state);
        if let Some(w) = worker {
            entry.worker = Some(w);
        }
        prev
    };

    // Intern the registration (+ worker) BEFORE settling (NG-3).
    let reg = registration_object(ctx.vm, &canonical, scope_sid);
    let cur_state = ctx
        .vm
        .sw_registrations
        .get(&canonical)
        .and_then(|e| e.worker.as_ref().map(|w| w.state));
    if cur_state.is_some() {
        let _ = worker_object(ctx.vm, &canonical, scope_sid);
    }

    // A freshly-installing worker is an `updatefound` (SW §3.2.10).
    if matches!(cur_state, Some(SwState::Installing))
        && !matches!(prev_state, Some(SwState::Installing))
    {
        fire_updatefound(ctx, reg);
    }

    // Settle every waiter (D2 — concurrent same-scope register all resolve).
    let waiters = ctx
        .vm
        .pending_registration_promises
        .remove(&canonical)
        .unwrap_or_default();
    for promise in waiters {
        settle_resolve(ctx.vm, promise, JsValue::Object(reg));
    }

    if matches!(cur_state, Some(SwState::Activating | SwState::Activated)) {
        resolve_sw_ready(ctx.vm, &canonical, scope_sid);
    }
}

// ---------------------------------------------------------------------------
// StateChanged — §3.1.2 / #update-worker-state
// ---------------------------------------------------------------------------

fn deliver_state_changed(ctx: &mut NativeContext<'_>, scope: &Url, state: SwState) {
    let canonical = scope.as_str().to_owned();
    let Some(scope_sid) = ctx.vm.sw_registrations.get(&canonical).map(|e| e.scope_sid) else {
        return;
    };
    let prev_state = ctx
        .vm
        .sw_registrations
        .get(&canonical)
        .and_then(|e| e.worker.as_ref().map(|w| w.state));

    // Mutate the worker state in place (identity preserved, never re-minted).
    {
        let entry = ctx
            .vm
            .sw_registrations
            .get_mut(&canonical)
            .expect("registry entry checked above");
        match entry.worker.as_mut() {
            Some(w) => w.state = state,
            None => {
                entry.worker = Some(SwWorkerSnapshot {
                    script_url: String::new(),
                    state,
                });
            }
        }
    }

    let worker = worker_object(ctx.vm, &canonical, scope_sid);
    fire_statechange(ctx, worker);

    // A worker newly entering `installing` is an `updatefound` (an update).
    if matches!(state, SwState::Installing) && !matches!(prev_state, Some(SwState::Installing)) {
        let reg = registration_object(ctx.vm, &canonical, scope_sid);
        fire_updatefound(ctx, reg);
    }

    if state.is_active() {
        resolve_sw_ready(ctx.vm, &canonical, scope_sid);
    }
}

// ---------------------------------------------------------------------------
// ControllerSet — §3.4.1
// ---------------------------------------------------------------------------

fn deliver_controller_set(ctx: &mut NativeContext<'_>, scope: Option<Url>) {
    ctx.vm.sw_controller_scope = scope.map(|s| s.as_str().to_owned());
    if let Some(container) = ctx.vm.sw_container {
        fire_controllerchange(ctx, container);
    }
}

// ---------------------------------------------------------------------------
// Message — §3.4 (buffered until startMessages / first onmessage)
// ---------------------------------------------------------------------------

fn deliver_message(ctx: &mut NativeContext<'_>, data: &str, source_scope: &str) {
    if ctx.vm.sw_messages_enabled {
        fire_message(ctx, data, source_scope);
    } else {
        ctx.vm
            .sw_message_buffer
            .push((data.to_owned(), source_scope.to_owned()));
    }
}

/// Enable the client message queue (`startMessages()`, SW §3.4.6) and flush
/// buffered `message` events.  Idempotent.
pub(crate) fn enable_sw_messages(ctx: &mut NativeContext<'_>) -> Result<(), VmError> {
    if ctx.vm.sw_messages_enabled {
        return Ok(());
    }
    ctx.vm.sw_messages_enabled = true;
    let buffered = std::mem::take(&mut ctx.vm.sw_message_buffer);
    for (data, source_scope) in buffered {
        fire_message(ctx, &data, &source_scope);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unregistered — §3.2.9 registry removal
// ---------------------------------------------------------------------------

fn deliver_unregistered(ctx: &mut NativeContext<'_>, scope: &Url, success: bool) {
    let canonical = scope.as_str().to_owned();
    // Remove from the registry + drop the interned wrappers, so `getRegistration`
    // stops returning the registration and GC un-roots its wrappers (R2-2).
    if let Some(entry) = ctx.vm.sw_registrations.remove(&canonical) {
        let scope_sid = entry.scope_sid;
        let _ = ctx.vm.remove_wrapper_keyed(WrapperKey::scope(
            scope_sid,
            WrapperKind::ServiceWorkerRegistration,
        ));
        let _ = ctx
            .vm
            .remove_wrapper_keyed(WrapperKey::scope(scope_sid, WrapperKind::ServiceWorker));
    }
    let waiters = ctx
        .vm
        .pending_unregister_promises
        .remove(&canonical)
        .unwrap_or_default();
    for promise in waiters {
        settle_resolve(ctx.vm, promise, JsValue::Boolean(success));
    }
}

// ---------------------------------------------------------------------------
// Dispatch + settle helpers
// ---------------------------------------------------------------------------

fn fire_statechange(ctx: &mut NativeContext<'_>, worker: ObjectId) {
    let sid = ctx.vm.strings.intern("statechange");
    let _ = dispatch_vm_simple_event(ctx, worker, sid, false, false);
}

fn fire_updatefound(ctx: &mut NativeContext<'_>, reg: ObjectId) {
    let sid = ctx.vm.strings.intern("updatefound");
    let _ = dispatch_vm_simple_event(ctx, reg, sid, false, false);
}

fn fire_controllerchange(ctx: &mut NativeContext<'_>, container: ObjectId) {
    let sid = ctx.vm.strings.intern("controllerchange");
    let _ = dispatch_vm_simple_event(ctx, container, sid, false, false);
}

fn fire_message(ctx: &mut NativeContext<'_>, data: &str, source_scope: &str) {
    let Some(container) = ctx.vm.sw_container else {
        return;
    };
    let origin = Url::parse(source_scope)
        .ok()
        .map(|u| u.origin().ascii_serialization())
        .unwrap_or_default();
    let origin_sid = ctx.vm.strings.intern(&origin);
    let data_owned = data.to_owned();
    let _ = fire_vm_message_event(
        ctx,
        container,
        "message",
        move |ctx| {
            super::super::super::natives_json::parse_json_str(ctx.vm, &data_owned)
                .unwrap_or(JsValue::Undefined)
        },
        origin_sid,
        "",
    );
}

/// Resolve `promise` with `value`, rooting both across `settle_promise`
/// (which runs reactions / may GC).
fn settle_resolve(vm: &mut VmInner, promise: ObjectId, value: JsValue) {
    let mut g = vm.push_temp_root(value);
    let mut g2 = g.push_temp_root(JsValue::Object(promise));
    let _ = super::super::super::natives_promise::settle_promise(&mut g2, promise, false, value);
    drop(g2);
    drop(g);
}

/// Reject `promise` with `reason`, rooting both across `settle_promise`.
fn settle_reject(vm: &mut VmInner, promise: ObjectId, reason: JsValue) {
    let mut g = vm.push_temp_root(reason);
    let mut g2 = g.push_temp_root(JsValue::Object(promise));
    let _ = super::super::super::natives_promise::settle_promise(&mut g2, promise, true, reason);
    drop(g2);
    drop(g);
}
