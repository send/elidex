//! Event-handler IDL attribute machinery (WHATWG HTML §8.1.8).
//!
//! Implements `el.onclick = fn` / `el.onclick` getter / inline
//! `<button onclick="...">` / `<body>.onbeforeunload`→Window delegation
//! on top of the engine-independent [`EventListeners`] component.
//!
//! ## Design (ECS-native)
//!
//! An event handler is a *special kind of event listener* (§8.1.8.1):
//! it lives as one entry in the target's [`EventListeners`] component
//! (`ListenerKind::EventHandler`), so dispatch walks it in registration
//! order alongside `addEventListener` listeners. The single source of
//! truth is that engine-independent component — there is **no** VM-side
//! reverse map. The IDL setter (here), the inline-attribute consumer
//! (`elidex-script-session`), the getter (here), and the dispatch walk
//! (`event_target_dispatch.rs`) all read/write the same component.
//!
//! ## Shared backend, bound key (prereq #211)
//!
//! All ~108 handler attributes install via two backend fn pairs over
//! [`VmInner::install_bound_accessor_pair`], parametrized by the
//! *event type* SID as the bound key (recovered at call time through
//! [`NativeContext::bound_key`]) — not one monomorphized fn per
//! attribute:
//!
//! - **normal pair** ([`native_event_handler_get`] / [`native_event_handler_set`]):
//!   `entity_from_this` is the target. Used for GlobalEventHandlers /
//!   WindowEventHandlers (on Window) / DocumentAndElementEventHandlers /
//!   Document-specific attributes.
//! - **body-delegation pair** ([`native_body_weh_get`] / [`native_body_weh_set`]):
//!   redirects the target to the Window entity (WHATWG HTML §8.1.8.2 —
//!   `<body>.onbeforeunload` reads/writes the Window's handler). Used
//!   only for the WindowEventHandlers overrides installed on
//!   `HTMLBodyElement.prototype`.
//!
//! ## Lazy compile
//!
//! Inline content-attribute handlers are stored as uncompiled source
//! (`ListenerKind::EventHandler { uncompiled: Some(..) }`) by the
//! engine-independent consumer, which never compiles (layering). The
//! source is compiled lazily — at first read (getter) or first dispatch
//! ([`lazy_compile_handler`]) — and an `uncompiled = Some` source always
//! takes precedence over any stale compiled callable (last-write-wins,
//! §8.1.8.1 "getting the current value of the event handler").

#![cfg(feature = "engine")]

use elidex_script_session::{
    event_handler_attr_event_type, EventListeners, HandlerScope, ListenerId, EVENT_HANDLER_ATTRS,
};

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::{NativeFn, VmInner};
use super::event_target::entity_from_this;

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install the event-handler IDL attribute accessors whose
    /// [`HandlerScope`] is in `scopes` onto `target` (a prototype or a
    /// per-entity wrapper). Each attribute installs one real accessor
    /// over the shared [`native_event_handler_get`] /
    /// [`native_event_handler_set`] backend pair, keyed by its event
    /// type SID (the bound key). WHATWG HTML §8.1.8.2 / §8.1.8.2.1
    /// (GlobalEventHandlers / DocumentAndElementEventHandlers /
    /// WindowEventHandlers).
    pub(in crate::vm) fn install_event_handler_attrs(
        &mut self,
        target: ObjectId,
        scopes: &[HandlerScope],
    ) {
        self.install_handler_attr_family(
            target,
            scopes,
            native_event_handler_get as NativeFn,
            native_event_handler_set as NativeFn,
        );
    }

    /// Install the WindowEventHandlers (18 attrs) onto
    /// `HTMLBodyElement.prototype` as **delegation** accessors: their
    /// getter/setter redirect to the Window object rather than the body
    /// element (WHATWG HTML §8.1.8.2 — body/frameset delegate
    /// WindowEventHandlers to the Window). GlobalEventHandlers are
    /// inherited from `HTMLElement.prototype` and not re-installed here.
    pub(in crate::vm) fn install_body_weh_delegation(&mut self, target: ObjectId) {
        self.install_handler_attr_family(
            target,
            &[HandlerScope::Window],
            native_body_weh_get as NativeFn,
            native_body_weh_set as NativeFn,
        );
    }

    /// Shared install loop: for every [`EVENT_HANDLER_ATTRS`] row whose
    /// scope is in `scopes`, intern the attribute-name SID (property key)
    /// and the event-type SID (bound key — derived through the single
    /// SoT helper [`event_handler_attr_event_type`], never an inline
    /// slice), then install the `get`/`set` accessor pair.
    fn install_handler_attr_family(
        &mut self,
        target: ObjectId,
        scopes: &[HandlerScope],
        get: NativeFn,
        set: NativeFn,
    ) {
        for (attr_name, scope) in EVENT_HANDLER_ATTRS {
            if !scopes.contains(scope) {
                continue;
            }
            let event_type = event_handler_attr_event_type(attr_name)
                .expect("EVENT_HANDLER_ATTRS row must be a known event-handler attribute");
            let attr_name_sid = self.strings.intern(attr_name);
            let event_type_sid = self.strings.intern(event_type);
            self.install_bound_accessor_pair(
                target,
                attr_name_sid,
                get,
                Some(set),
                event_type_sid,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shared backend (normal): target = `entity_from_this`
// ---------------------------------------------------------------------------

/// Shared getter for every event-handler IDL attribute (WHATWG HTML
/// §8.1.8.1 "the event handler IDL attributes" — getter / "getting the
/// current value of the event handler"; WebIDL §3.7.6). Recovers its
/// event type from `ctx.bound_key()`. Returns the current callable, or
/// `null` (never `undefined` — an unset handler attribute reads as
/// `null` per the WebIDL `EventHandler?` nullable type).
fn native_event_handler_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    Ok(read_event_handler(ctx, entity, &event_type))
}

/// Shared setter for every event-handler IDL attribute (WHATWG HTML
/// §8.1.8.1 — setter / "activate an event handler"; WebIDL §3.7.6). A
/// callable value activates the handler; any non-callable value
/// (including `null`/`undefined`) clears it to `null` silently (the
/// WebIDL `EventHandler?` conversion does not throw).
fn native_event_handler_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let callable = callable_arg(ctx, args);
    activate_event_handler(ctx, entity, &event_type, callable);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Body-delegation backend: target = Window entity (HTML §8.1.8.2)
// ---------------------------------------------------------------------------

/// `HTMLBodyElement.prototype` WindowEventHandlers getter — delegates to
/// the Window object (WHATWG HTML §8.1.8.2). No-op (`null`) if no Window
/// is bound.
fn native_body_weh_get(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(window_entity) = ctx.host().window_entity() else {
        return Ok(JsValue::Null);
    };
    Ok(read_event_handler(ctx, window_entity, &event_type))
}

/// `HTMLBodyElement.prototype` WindowEventHandlers setter — delegates to
/// the Window object (WHATWG HTML §8.1.8.2). No-op if no Window is bound.
fn native_body_weh_set(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(window_entity) = ctx.host().window_entity() else {
        return Ok(JsValue::Undefined);
    };
    let callable = callable_arg(ctx, args);
    activate_event_handler(ctx, window_entity, &event_type, callable);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Shared core
// ---------------------------------------------------------------------------

/// The event type this accessor serves, materialized from the bound key
/// (`ctx.bound_key()` = event-type SID, installed by
/// `install_bound_accessor_pair`) to an owned `String` for the
/// engine-independent `&str`-keyed [`EventListeners`] lookups.
fn bound_event_type(ctx: &NativeContext<'_>) -> String {
    let sid = ctx
        .bound_key()
        .expect("event-handler accessor missing bound_key");
    ctx.get_utf8(sid)
}

/// Store `callable` as the compiled callable for handler listener `id`,
/// replacing any previous one. Event-handler reassignment intentionally
/// overwrites the slot (unlike `addEventListener`, whose `store_listener`
/// asserts uniqueness): remove the old entry first so the insert does not
/// trip that guard. The `listener_store` map is itself the GC root set,
/// so the dropped callable is correctly unrooted and the new one rooted.
pub(super) fn set_handler_callable(
    ctx: &mut NativeContext<'_>,
    id: ListenerId,
    callable: ObjectId,
) {
    let host = ctx.host();
    let _ = host.remove_listener(id);
    host.store_listener(id, callable);
}

/// `args[0]` if it is a callable object, else `None` (the WebIDL
/// `EventHandler?` setter treats any non-callable as `null`).
fn callable_arg(ctx: &NativeContext<'_>, args: &[JsValue]) -> Option<ObjectId> {
    match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(obj) if ctx.vm.get_object(obj).kind.is_callable() => Some(obj),
        _ => None,
    }
}

/// Read the current value of the `(entity, event_type)` handler (WHATWG
/// HTML §8.1.8.1 "getting the current value of the event handler"). An
/// `uncompiled = Some` source takes precedence over any stale compiled
/// callable (last-write-wins): it is drained, compiled, and the result
/// overwrites the stored callable. A parse failure clears the handler to
/// `null`.
fn read_event_handler(
    ctx: &mut NativeContext<'_>,
    entity: elidex_ecs::Entity,
    event_type: &str,
) -> JsValue {
    // Pass 1: locate the handler listener and drain any pending inline
    // source (scoped so the DOM/world borrow drops before compiling).
    let (id, uncompiled) = {
        let dom = ctx.host().dom();
        let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) else {
            return JsValue::Null;
        };
        let Some(id) = listeners.find_event_handler(event_type) else {
            return JsValue::Null;
        };
        (id, listeners.take_uncompiled(id).map(|u| u.source))
    };

    if let Some(source) = uncompiled {
        if let Some(callable) = lazy_compile_handler(ctx, &source) {
            set_handler_callable(ctx, id, callable);
        } else {
            // §8.1.8.1: "if body is not parsable" → handler value is null.
            ctx.vm.remove_listener_and_prune_back_ref(id);
            return JsValue::Null;
        }
    }

    ctx.host()
        .get_listener(id)
        .map_or(JsValue::Null, JsValue::Object)
}

/// Activate (`Some`) or clear (`None`) the `(entity, event_type)` event
/// handler (WHATWG HTML §8.1.8.1 "activate an event handler" / setter).
/// The listener entry is added at most once per `(target, event type)`
/// and reused on subsequent writes — the stored callable is what
/// changes. Clearing keeps the entry (registration-order stability) but
/// drops the callable so dispatch skips it.
fn activate_event_handler(
    ctx: &mut NativeContext<'_>,
    entity: elidex_ecs::Entity,
    event_type: &str,
    callable: Option<ObjectId>,
) {
    if let Some(obj) = callable {
        let id = {
            let dom = ctx.host().dom();
            if dom.world().get::<&EventListeners>(entity).is_err() {
                let _ = dom.world_mut().insert_one(entity, EventListeners::new());
            }
            let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) else {
                return;
            };
            let id = listeners
                .find_event_handler(event_type)
                .unwrap_or_else(|| listeners.add_event_handler(event_type.to_string()));
            // IDL write is the last write: any stale inline source is
            // superseded by this fresh compiled callable.
            listeners.clear_uncompiled(id);
            id
        };
        set_handler_callable(ctx, id, obj);
    } else {
        let id = {
            let dom = ctx.host().dom();
            let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) else {
                return;
            };
            let Some(id) = listeners.find_event_handler(event_type) else {
                return;
            };
            listeners.clear_uncompiled(id);
            id
        };
        ctx.vm.remove_listener_and_prune_back_ref(id);
    }
}

/// Compile an inline handler body as `function (event) { <body> }` and
/// return the callable's `ObjectId` (WHATWG HTML §8.1.8.1 "getting the
/// current value of the event handler" — compile step). Returns `None`
/// if the body is not parsable (the caller then clears the handler to
/// `null`). Shared by the getter ([`read_event_handler`]) and the
/// dispatch walk's lazy-compile branch.
///
/// The body is wrapped in a function expression so a top-level `return`
/// inside the inline handler (`onsubmit="return false"`) is valid.
/// Compilation uses `run_script` (not `Vm::eval`) deliberately: `eval`
/// drains the microtask + same-window task queues, which could re-enter
/// event dispatch while this runs mid-dispatch; evaluating the function
/// expression only allocates the closure and never runs user code, so no
/// queues need draining.
///
/// The special inline-handler scope chain (`with(document)
/// with(form-owner) with(element)`) is deferred
/// (`#11-inline-handler-scope-chain`); `event` + `this` = currentTarget
/// cover the common case. The 5-argument `onerror` signature is deferred
/// (`#11-onerror-error-event-args`).
pub(super) fn lazy_compile_handler(ctx: &mut NativeContext<'_>, source: &str) -> Option<ObjectId> {
    let wrapped = format!("(function (event) {{\n{source}\n}})");
    let script = crate::compiler::compile_script(&wrapped).ok()?;
    match ctx.vm.run_script(script) {
        Ok(JsValue::Object(id)) if ctx.vm.get_object(id).kind.is_callable() => Some(id),
        _ => None,
    }
}
