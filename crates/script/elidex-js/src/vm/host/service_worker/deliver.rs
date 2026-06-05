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

use super::super::super::value::{JsValue, NativeContext, ObjectId, StringId, VmError};
use super::super::super::wrapper_intern::{WrapperKey, WrapperKind};
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
            update_via_cache,
        } => deliver_registered(ctx, &scope, success, error, worker, update_via_cache),
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
    update_via_cache: UpdateViaCache,
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
            super::settle_rooted(ctx.vm, promise, true, reason);
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
                update_via_cache,
                worker: None,
            });
        // A re-register / update may change updateViaCache (SW §3.2.7).
        entry.update_via_cache = update_via_cache;
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

    // Settle every waiter (D2 — concurrent same-scope register all resolve).
    // Drain the pending list BEFORE firing any event handler: an `updatefound`
    // handler that synchronously re-`register()`s the same scope pushes a NEW
    // pending promise, which must wait for its own round-trip rather than being
    // settled by this deliver.  (settle_promise queues the `.then` as a
    // microtask, so the handler below still runs before register() resolves.)
    let waiters = ctx
        .vm
        .pending_registration_promises
        .remove(&canonical)
        .unwrap_or_default();
    for promise in waiters {
        super::settle_rooted(ctx.vm, promise, false, JsValue::Object(reg));
    }

    // A freshly-installing worker is an `updatefound` (SW §3.2.10).
    fire_updatefound_if_new_installing(ctx, &canonical, scope_sid, prev_state, cur_state);

    if cur_state.is_some_and(|s| s.is_active_slot()) {
        resolve_sw_ready(ctx.vm, &canonical, scope_sid);
    }
}

// ---------------------------------------------------------------------------
// StateChanged — §3.1.2 / #update-worker-state
// ---------------------------------------------------------------------------

fn deliver_state_changed(ctx: &mut NativeContext<'_>, scope: &Url, state: SwState) {
    let canonical = scope.as_str().to_owned();
    // One `get_mut`: read scope_sid + prev_state and mutate the worker state in
    // place (identity preserved, never re-minted).  Early-return for an
    // out-of-scope deliver (no registry entry).
    let (scope_sid, prev_state) = {
        let Some(entry) = ctx.vm.sw_registrations.get_mut(&canonical) else {
            return;
        };
        let scope_sid = entry.scope_sid;
        let prev_state = entry.worker.as_ref().map(|w| w.state);
        match entry.worker.as_mut() {
            Some(w) => w.state = state,
            None => {
                entry.worker = Some(SwWorkerSnapshot {
                    script_url: String::new(),
                    state,
                });
            }
        }
        (scope_sid, prev_state)
    };

    let worker = worker_object(ctx.vm, &canonical, scope_sid);
    fire_simple(ctx, worker, "statechange");

    // A worker newly entering `installing` is an `updatefound` (an update).
    fire_updatefound_if_new_installing(ctx, &canonical, scope_sid, prev_state, Some(state));

    if state.is_active_slot() {
        resolve_sw_ready(ctx.vm, &canonical, scope_sid);
    }
}

/// Fire `updatefound` on the registration when a worker newly enters the
/// `installing` state (SW §3.2.10) — shared by the register + statechange
/// delivers so the edge predicate has one definition.
fn fire_updatefound_if_new_installing(
    ctx: &mut NativeContext<'_>,
    canonical: &str,
    scope_sid: StringId,
    prev_state: Option<SwState>,
    new_state: Option<SwState>,
) {
    if matches!(new_state, Some(SwState::Installing))
        && !matches!(prev_state, Some(SwState::Installing))
    {
        let reg = registration_object(ctx.vm, canonical, scope_sid);
        fire_simple(ctx, reg, "updatefound");
    }
}

// ---------------------------------------------------------------------------
// ControllerSet — §3.4.1
// ---------------------------------------------------------------------------

fn deliver_controller_set(ctx: &mut NativeContext<'_>, scope: Option<Url>) {
    // The shell broadcasts `ControllerSet` to every same-origin tab; this client
    // must adopt the controller only if it is actually controlled by that
    // registration — the registration is known to this realm AND its scope
    // contains the document URL (SW "Match Service Worker Registration",
    // `#match-service-worker-registration`; with
    // multiple same-origin registrations a non-controlling one must be ignored).
    let new_scope = match scope {
        Some(s) => {
            let canonical = s.as_str().to_owned();
            if !ctx.vm.sw_registrations.contains_key(&canonical)
                || !elidex_api_sw::matches_scope(&s, &ctx.vm.navigation.current_url)
            {
                return;
            }
            Some(canonical)
        }
        None => None,
    };
    // `controllerchange` fires only on an actual change (SW §3.4.1).
    if ctx.vm.sw_controller_scope == new_scope {
        return;
    }
    ctx.vm.sw_controller_scope = new_scope;
    if let Some(container) = ctx.vm.sw_container {
        fire_simple(ctx, container, "controllerchange");
    }
}

// ---------------------------------------------------------------------------
// Message — §3.4.6 (buffered until the client message queue is enabled)
// ---------------------------------------------------------------------------

fn deliver_message(ctx: &mut NativeContext<'_>, data: &str, source_scope: &str) {
    // Adding a `message` listener (`onmessage = …` or addEventListener) enables
    // the client message queue, the same as `startMessages()` (SW §3.4.6) —
    // latch it (which flushes any already-buffered messages) on first sight.
    if !ctx.vm.sw_messages_enabled {
        if let Some(container) = ctx.vm.sw_container {
            if super::super::event_target_dispatch_vm::vm_path_has_listener(
                ctx.vm, container, "message", false,
            ) {
                let _ = enable_sw_messages(ctx);
            }
        }
    }
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
        super::settle_rooted(ctx.vm, promise, false, JsValue::Boolean(success));
    }
}

// ---------------------------------------------------------------------------
// Dispatch + settle helpers
// ---------------------------------------------------------------------------

/// UA-fire a plain, non-bubbling, non-cancelable `Event` (`statechange` /
/// `updatefound` / `controllerchange`) at a VmObject target.
fn fire_simple(ctx: &mut NativeContext<'_>, target: ObjectId, event_type: &str) {
    let sid = ctx.vm.strings.intern(event_type);
    let _ = dispatch_vm_simple_event(ctx, target, sid, false, false);
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
