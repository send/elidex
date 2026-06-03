//! `ExtendableEvent` / `FetchEvent` interfaces + their `waitUntil` /
//! `respondWith` natives (WHATWG SW §4.4 / §4.6; D-19 PR-2).
//!
//! Both events are ordinary `ObjectKind::Event`s (so they dispatch through
//! the shared `dispatch_script_event`, the `MessageEvent` precedent); the
//! [`FetchEventState`] / [`ExtendableEventState`] side-stores are their
//! brands + the mutable native state the SW loop (`vm/sw_thread.rs`) reads
//! back after dispatch.

#![cfg(feature = "engine")]

use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::{NativeFn, VmInner};
use super::super::event_target_dispatch::dispatch_script_event;
use super::{install_interface, ExtendableEventState, FetchEventState};

// ---------------------------------------------------------------------------
// Interface registration
// ---------------------------------------------------------------------------

/// Allocate `ExtendableEvent.prototype` (chains to `Event.prototype`),
/// install `waitUntil`, and expose the `ExtendableEvent` interface.
pub(crate) fn register_extendable_event_interface(vm: &mut VmInner) {
    let parent = vm.event_prototype;
    let proto = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: parent,
        extensible: true,
    });
    let methods: &[(&str, NativeFn)] = &[("waitUntil", native_extendable_wait_until)];
    vm.install_methods(proto, methods);
    vm.extendable_event_prototype = Some(proto);
    install_interface(vm, proto, "ExtendableEvent");
}

/// Allocate `FetchEvent.prototype` (chains to `ExtendableEvent.prototype`),
/// install `respondWith`, and expose the `FetchEvent` interface.  Must run
/// after [`register_extendable_event_interface`].
pub(crate) fn register_fetch_event_interface(vm: &mut VmInner) {
    let parent = vm
        .extendable_event_prototype
        .expect("register_fetch_event_interface called before register_extendable_event_interface");
    let proto = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    let methods: &[(&str, NativeFn)] = &[("respondWith", native_fetch_event_respond_with)];
    vm.install_methods(proto, methods);
    vm.fetch_event_prototype = Some(proto);
    install_interface(vm, proto, "FetchEvent");
}

// ---------------------------------------------------------------------------
// Event construction (called by the SW loop, not from script)
// ---------------------------------------------------------------------------

/// Allocate an `install` / `activate` `ExtendableEvent` (SW §4.4) over
/// `ExtendableEvent.prototype` + register its [`ExtendableEventState`] brand.
/// The returned event is dispatched via [`dispatch_event_at_sw_scope`].
pub(crate) fn create_extendable_event(vm: &mut VmInner, event_type: &str) -> ObjectId {
    let type_sid = vm.strings.intern(event_type);
    let proto = vm.extendable_event_prototype;
    let id = alloc_sw_event(vm, type_sid, proto);
    vm.extendable_event_states.insert(
        id,
        ExtendableEventState {
            lifetime_promises: Vec::new(),
        },
    );
    id
}

/// Allocate a `fetch` `FetchEvent` (SW §4.6) over `FetchEvent.prototype`
/// with its `request` / `clientId` / `resultingClientId` own attributes +
/// register both the [`FetchEventState`] (respondWith) and
/// [`ExtendableEventState`] (inherited waitUntil) brands.
///
/// `request_obj` must already be rooted by the caller (it becomes an
/// own-data prop of the returned event, which the caller roots across
/// dispatch).
pub(crate) fn create_fetch_event(
    vm: &mut VmInner,
    request_obj: ObjectId,
    client_id: &str,
    resulting_client_id: &str,
) -> ObjectId {
    let type_sid = vm.strings.intern("fetch");
    let proto = vm.fetch_event_prototype;
    let id = alloc_sw_event(vm, type_sid, proto);

    let cid = vm.strings.intern(client_id);
    let rcid = vm.strings.intern(resulting_client_id);
    define_ro(vm, id, "request", JsValue::Object(request_obj));
    define_ro(vm, id, "clientId", JsValue::String(cid));
    define_ro(vm, id, "resultingClientId", JsValue::String(rcid));

    vm.fetch_event_states.insert(
        id,
        FetchEventState {
            responded: false,
            response_promise: None,
        },
    );
    vm.extendable_event_states.insert(
        id,
        ExtendableEventState {
            lifetime_promises: Vec::new(),
        },
    );
    id
}

/// Allocate a UA-fired `ObjectKind::Event` with the given type + prototype +
/// the **core-9 event slots** (type / bubbles / cancelable / eventPhase /
/// target / currentTarget / timeStamp / composed / isTrusted) installed via
/// the `core` precomputed shape — exactly the `dispatch_message_event_at`
/// pattern.  Without the core slots, `dispatch_script_event`'s slot writes
/// (e.g. `target` at index 4) panic on the empty shape.  SW events are
/// non-bubbling / non-cancelable / non-composed (preventDefault is not their
/// response mechanism — `respondWith` / `waitUntil` are) and trusted.
fn alloc_sw_event(
    vm: &mut VmInner,
    type_sid: super::super::super::value::StringId,
    proto: Option<ObjectId>,
) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: false,
            passive: false,
            type_sid,
            bubbles: false,
            composed: false,
            composed_path: None,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let core_shape = vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;
    let timestamp_ms = vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    // Slot order MUST match the core-9 layout (see
    // `events::create_fresh_event_object`): target / currentTarget are seeded
    // `Null` and filled by `dispatch_script_event`.
    let slots = vec![
        PropertyValue::Data(JsValue::String(type_sid)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Number(0.0)),
        PropertyValue::Data(JsValue::Null),
        PropertyValue::Data(JsValue::Null),
        PropertyValue::Data(JsValue::Number(timestamp_ms)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(true)),
    ];
    vm.define_with_precomputed_shape(id, core_shape, slots);
    id
}

/// Define a readonly own-data attribute on an SW event object.
fn define_ro(vm: &mut VmInner, id: ObjectId, name: &str, value: JsValue) {
    let key = PropertyKey::String(vm.strings.intern(name));
    vm.define_shaped_property(
        id,
        key,
        PropertyValue::Data(value),
        PropertyAttrs::WEBIDL_RO,
    );
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Fire `event_id` at the SW global scope entity through the shared
/// `dispatch_script_event` walker (so the matching `on*` handler and every
/// `addEventListener` listener run).  Resolves the scope entity from the
/// `globalThis` `HostObject` (the SW loop's `bind_worker` promoted it).
pub(crate) fn dispatch_event_at_sw_scope(
    vm: &mut VmInner,
    event_id: ObjectId,
) -> Result<(), VmError> {
    let global_id = vm.global_object;
    let ObjectKind::HostObject {
        entity_bits: target_bits,
    } = vm.get_object(global_id).kind
    else {
        return Err(VmError::type_error(
            "Service Worker global scope is not bound to an entity",
        ));
    };
    let Some(target_entity) = elidex_ecs::Entity::from_bits(target_bits) else {
        return Err(VmError::type_error("invalid SW scope entity"));
    };
    vm.dispatched_events.insert(event_id);
    let result = {
        let mut ctx = NativeContext::new_call(vm);
        dispatch_script_event(&mut ctx, event_id, target_entity)
    };
    vm.dispatched_events.remove(&event_id);
    result.map(|_| ())
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

/// `Promise.resolve(value)` as an `ObjectId` — a fresh promise that adopts
/// `value` (a real promise is followed; a plain value fulfils immediately).
/// Reactions fire on the next `drain_microtasks` in the SW loop.  `value` is
/// rooted across `create_promise` (which can GC) defensively, independent of
/// the caller's argument-stack rooting.
fn promise_resolve(vm: &mut VmInner, value: JsValue) -> ObjectId {
    let mut g = vm.push_temp_root(value);
    let promise = super::super::super::natives_promise::create_promise(&mut g);
    let mut g2 = g.push_temp_root(JsValue::Object(promise));
    super::super::blob::resolve_promise_sync(&mut g2, promise, value);
    drop(g2);
    drop(g);
    promise
}

/// `ExtendableEvent.waitUntil(f)` (SW §4.4.1 + Install/Activate `#install` /
/// `#activate`): add `Promise.resolve(f)` to the event's lifetime-promise
/// list; the lifecycle loop holds `LifecycleComplete` until all settle (a
/// rejection / timeout → `success:false`).
fn native_extendable_wait_until(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(
            "Illegal invocation: receiver is not an ExtendableEvent",
        ));
    };
    if !ctx.vm.extendable_event_states.contains_key(&id) {
        return Err(VmError::type_error(
            "Illegal invocation: receiver is not an ExtendableEvent",
        ));
    }
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let promise = promise_resolve(ctx.vm, value);
    if let Some(state) = ctx.vm.extendable_event_states.get_mut(&id) {
        state.lifetime_promises.push(promise);
    }
    Ok(JsValue::Undefined)
}

/// `FetchEvent.respondWith(r)` (SW §4.6.7): store `Promise.resolve(r)` as
/// the event's response promise (the SW loop drains it to a `SwResponse`).
/// A second call — or any call after dispatch settled the event — throws
/// `InvalidStateError`.
fn native_fetch_event_respond_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(
            "Illegal invocation: receiver is not a FetchEvent",
        ));
    };
    let Some(state) = ctx.vm.fetch_event_states.get(&id) else {
        return Err(VmError::type_error(
            "Illegal invocation: receiver is not a FetchEvent",
        ));
    };
    if state.responded {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            "FetchEvent.respondWith: respondWith() was already called",
        ));
    }
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let promise = promise_resolve(ctx.vm, value);
    if let Some(state) = ctx.vm.fetch_event_states.get_mut(&id) {
        state.responded = true;
        state.response_promise = Some(promise);
    }
    Ok(JsValue::Undefined)
}
