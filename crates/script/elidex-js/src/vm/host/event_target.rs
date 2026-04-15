//! `EventTarget.prototype` intrinsic — holds the three native methods
//! (`addEventListener`, `removeEventListener`, `dispatchEvent`) that every
//! DOM wrapper inherits.
//!
//! ## Why a shared prototype?
//!
//! The alternative — registering the three methods directly on each
//! element wrapper at creation time — would allocate N × 3 native-function
//! objects for N elements.  A single shared prototype matches the spec
//! (WHATWG DOM §2.7 `EventTarget` interface) and aligns with how
//! `Promise.prototype` / `Array.prototype` are structured elsewhere in
//! the VM.
//!
//! ## Stub status
//!
//! - `addEventListener` — implemented (PR3 C7).
//! - `removeEventListener` — stub (lands in PR3 C8).
//! - `dispatchEvent`: **deferred to PR5a** alongside `Event` constructors,
//!   which are the only meaningful way to pass a JS-constructed event
//!   into a synchronous dispatch from script.  Until then the stub is a
//!   no-op; `dispatchEvent` is still resolvable via the prototype chain,
//!   which is enough for scripts that only feature-test its existence.

use super::super::value::{JsValue, NativeContext, VmError};
#[cfg(feature = "engine")]
use super::super::value::{ObjectId, ObjectKind, PropertyKey};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Populate `self.event_target_prototype` with the `EventTarget`
    /// interface methods (WHATWG DOM §2.7).
    ///
    /// Called from `register_globals()` after `Object.prototype` is in
    /// place (every DOM wrapper's prototype chain terminates in
    /// `Object.prototype`, so this intrinsic sits one level above it).
    ///
    /// The three method bodies are **stubs** at C0 — see module doc for
    /// the per-method replacement schedule.
    pub(in crate::vm) fn register_event_target_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            (
                "addEventListener",
                native_event_target_add_event_listener as NativeFn,
            ),
            (
                "removeEventListener",
                native_event_target_remove_event_listener,
            ),
            ("dispatchEvent", native_event_target_dispatch_event),
        ]);
        self.event_target_prototype = Some(proto_id);
    }
}

/// `EventTarget.prototype.addEventListener` — non-engine-feature stub.
///
/// Without the `engine` feature there is no `HostData` / DOM to record
/// the listener against; addEventListener becomes a no-op so JS that
/// merely feature-tests the method's existence still works.
#[cfg(not(feature = "engine"))]
pub(super) fn native_event_target_add_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.addEventListener(type, listener, options)`.
///
/// WHATWG DOM §2.6 / §2.7.  Behaviour:
///
/// 1. `this` must be a `HostObject` — extract the entity.  Calls on
///    non-HostObject receivers are silent no-ops, matching browsers'
///    "doesn't do anything visible" handling for objects that
///    feature-test the prototype method.
/// 2. `type` is `ToString(arg[0])`.
/// 3. `listener` (`arg[1]`) — `null`/`undefined` returns silently
///    (spec §2.6 step 2: "If callback is null, then return.").
///    Otherwise must be callable; non-callable throws `TypeError`.
/// 4. `options` (`arg[2]`):
///    - boolean → `capture` flag.
///    - object → `{capture, once, passive}` properties read as
///      booleans.  Missing keys default to `false`.
///    - undefined / missing → all flags `false`.
/// 5. Duplicate check (§2.6 step 4): `(type, callback, capture)` —
///    `once` and `passive` are NOT part of the identity tuple.  A
///    second `addEventListener` with the same triple is silently
///    discarded.
/// 6. The new listener is recorded in two places:
///    - ECS `EventListeners` component on `entity` (metadata: type,
///      capture, once, passive).
///    - `HostData::listener_store` (`ListenerId` → JS function
///      `ObjectId`).  Both are GC-rooted via
///      `HostData::gc_root_object_ids()`.
///
/// Deferred:
/// - `signal` option (AbortSignal) → PR4 once `AbortController` lands.
/// - Object-form callback with `handleEvent` method (§2.7 step 8) →
///   not yet a hot-path.
#[cfg(feature = "engine")]
pub(super) fn native_event_target_add_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };

    // Argument 1: callback — null/undefined silently returns.
    let callback_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let listener_obj_id = match callback_arg {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Undefined),
        JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'addEventListener' on 'EventTarget': \
                 parameter 2 is not of type 'EventListener'.",
            ));
        }
    };

    // Argument 0: type — coerced via ToString (a non-string type
    // argument is rare but spec-allowed).
    let type_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    // Materialise the type as a Rust String for ECS storage; the
    // ECS component owns it independently of the StringPool's WTF-16
    // representation.
    let event_type = ctx.vm.strings.get_utf8(type_sid);

    // Argument 2: options — boolean (capture only) or object form.
    let options = parse_listener_options(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;

    // Duplicate check: walk existing listeners on `entity` matching
    // (type, capture); for each, look up its function ObjectId in
    // listener_store and compare to the candidate.  WHATWG §2.6
    // step 4 explicitly excludes `once` and `passive` from identity.
    if listener_already_registered(ctx, entity, &event_type, options.capture, listener_obj_id) {
        return Ok(JsValue::Undefined);
    }

    // Register in the ECS component (creating the component if absent).
    let host = ctx.host();
    let dom = host.dom();
    let listener_id = {
        if dom
            .world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .is_ok()
        {
            dom.world_mut()
                .get::<&mut elidex_script_session::EventListeners>(entity)
                .expect("just-checked component must exist")
                .add_with_options(event_type, options.capture, options.once, options.passive)
        } else {
            let mut listeners = elidex_script_session::EventListeners::new();
            let id = listeners.add_with_options(
                event_type,
                options.capture,
                options.once,
                options.passive,
            );
            // `insert_one` returns Err only on already-despawned
            // entities, which would also have failed the world.get()
            // probe above — silently ignore the result.
            let _ = dom.world_mut().insert_one(entity, listeners);
            id
        }
    };

    // Stash the JS function ObjectId so dispatch can look it up.
    ctx.host().store_listener(listener_id, listener_obj_id);

    Ok(JsValue::Undefined)
}

/// Extract the ECS Entity from `this` if it is a `HostObject` wrapper.
/// Returns `None` for any other receiver — addEventListener on a
/// non-HostObject is a silent no-op.
#[cfg(feature = "engine")]
fn entity_from_this(ctx: &NativeContext<'_>, this: JsValue) -> Option<elidex_ecs::Entity> {
    let JsValue::Object(id) = this else {
        return None;
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return None;
    };
    elidex_ecs::Entity::from_bits(entity_bits)
}

/// Parsed listener options — capture / once / passive.  AbortSignal is
/// not represented yet (PR4).
#[cfg(feature = "engine")]
#[derive(Default)]
pub(super) struct ListenerOptions {
    pub(super) capture: bool,
    pub(super) once: bool,
    pub(super) passive: bool,
}

/// WHATWG DOM §2.6 step 1 — extract listener flags from the third
/// argument of add/removeEventListener.
///
/// - undefined / missing → all flags false.
/// - boolean → capture only (legacy form).
/// - object  → read `capture`, `once`, `passive` as booleans (missing
///   keys default to false).
#[cfg(feature = "engine")]
fn parse_listener_options(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<ListenerOptions, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(ListenerOptions::default()),
        JsValue::Boolean(capture) => Ok(ListenerOptions {
            capture,
            ..Default::default()
        }),
        JsValue::Object(opts_id) => {
            let mut out = ListenerOptions::default();
            for (rust_name, slot) in [
                ("capture", &mut out.capture),
                ("once", &mut out.once),
                ("passive", &mut out.passive),
            ] {
                let key_sid = ctx.vm.strings.intern(rust_name);
                let v = ctx
                    .vm
                    .get_property_value(opts_id, PropertyKey::String(key_sid))?;
                *slot = super::super::coerce::to_boolean(ctx.vm, v);
            }
            Ok(out)
        }
        // Other types coerce via ToBoolean per WHATWG (treated as
        // capture flag).  Browsers accept e.g. `true === 1` style
        // calls; mirror that.
        other => Ok(ListenerOptions {
            capture: super::super::coerce::to_boolean(ctx.vm, other),
            ..Default::default()
        }),
    }
}

/// Walk existing listeners on `entity` matching (`event_type`,
/// `capture`); return `true` if any of them maps (via listener_store)
/// to `candidate_obj_id`.
#[cfg(feature = "engine")]
fn listener_already_registered(
    ctx: &mut NativeContext<'_>,
    entity: elidex_ecs::Entity,
    event_type: &str,
    capture: bool,
    candidate_obj_id: ObjectId,
) -> bool {
    let host = ctx.host();
    let dom = host.dom();
    let existing_ids: Vec<_> = dom
        .world()
        .get::<&elidex_script_session::EventListeners>(entity)
        .ok()
        .map(|listeners| {
            listeners
                .matching_all(event_type)
                .iter()
                .filter(|e| e.capture == capture)
                .map(|e| e.id)
                .collect()
        })
        .unwrap_or_default();
    let _ = dom; // release the borrow before re-grabbing host
    let host = ctx.host();
    existing_ids
        .iter()
        .any(|id| host.get_listener(*id) == Some(candidate_obj_id))
}

/// `EventTarget.prototype.removeEventListener(type, listener, options)` — stub.
///
/// Real implementation arrives in PR3 C8.
pub(super) fn native_event_target_remove_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.dispatchEvent(event)` — stub.
///
/// Real implementation is deferred to **PR5a** (which lands `new Event(...)`
/// and the Event constructor family).  Until then, invoking this returns
/// `false` (the spec default for "event not dispatched").
pub(super) fn native_event_target_dispatch_event(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(false))
}
