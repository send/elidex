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
//! ## Method status
//!
//! - `addEventListener` — fully implemented (PR3 C7); options-object
//!   form, capture/once/passive, duplicate check, ECS + listener_store
//!   wiring.
//! - `removeEventListener` — fully implemented (PR3 C8); WHATWG
//!   §2.7.7 (type, callback, capture) match + ECS / listener_store
//!   cleanup, options parsed via `parse_capture_only` (no spurious
//!   `once`/`passive` getter calls — see PR3 R7).
//! - `dispatchEvent` — **deferred to PR5a** alongside `Event`
//!   constructors, which are the only meaningful way to pass a
//!   JS-constructed event into a synchronous dispatch from script.
//!   Until then the stub returns `false` (the spec default for
//!   "event not dispatched"); the method itself is still resolvable
//!   via the prototype chain, which is enough for scripts that only
//!   feature-test its existence.

use super::super::value::{JsValue, NativeContext, VmError};
#[cfg(feature = "engine")]
use super::super::value::{ObjectId, ObjectKind, PropertyKey, PropertyStorage, PropertyValue};
use super::super::{NativeFn, VmInner};
#[cfg(feature = "engine")]
use elidex_script_session::event_dispatch::{
    apply_retarget, build_dispatch_plan, build_propagation_path, DispatchEvent, DispatchFlags,
};

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
        // `create_object_with_methods` interns the literal names; the
        // resulting StringIds match `WellKnownStrings`'s pre-cached
        // entries by construction (`StringPool::intern` is
        // deduplicating), so `AbortSignal.prototype`'s shadow
        // installer can use the well-known IDs and land on the same
        // shape slots as the override target.
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
        // Node-level accessors live on `Node.prototype` (one level
        // up), installed by `register_node_prototype` during
        // `register_globals`.
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
///    - object → `{capture, once, passive, signal}` properties read.
///      Missing booleans default to `false`; missing/`null`/`undefined`
///      `signal` means no AbortSignal is bound.  A non-AbortSignal
///      `signal` value throws `TypeError` (WebIDL `AbortSignal?`).
///    - undefined / missing → all flags `false`, no signal.
/// 5. WHATWG §2.6.3 step 3: an already-aborted `signal` short-circuits
///    registration entirely — no ECS write, no listener_store entry,
///    no back-ref.
/// 6. Duplicate check (§2.6 step 4): `(type, callback, capture)` —
///    `once` and `passive` are NOT part of the identity tuple.  A
///    second `addEventListener` with the same triple is silently
///    discarded.
/// 7. The new listener is recorded in three places:
///    - ECS `EventListeners` component on `entity` (metadata: type,
///      capture, once, passive).
///    - `HostData::listener_store` (`ListenerId` → JS function
///      `ObjectId`).  Both are GC-rooted via
///      `HostData::gc_root_object_ids()`.
///    - When `{signal}` is provided: an entry in
///      `abort_signal_states[signal_id].bound_listener_removals`
///      plus a reverse-index entry in
///      `VmInner::abort_listener_back_refs`, so `controller.abort()`
///      can detach in O(1) and `removeEventListener` /
///      `{once}` auto-removal can prune the back-ref symmetrically.
///
/// Deferred:
/// - Object-form callback with `handleEvent` method (§2.7 step 8) —
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

    // WHATWG §2.6.3 step 3: an already-aborted signal short-circuits
    // registration entirely — listener is never added.  Subsequent
    // `controller.abort()` is a no-op for one-shot semantics, so the
    // listener would have nothing to detach anyway.  Read-only borrow
    // keeps this off the hot path for the common `signal === None` case.
    if let Some(signal_id) = options.signal {
        if ctx
            .vm
            .abort_signal_states
            .get(&signal_id)
            .is_some_and(|s| s.aborted)
        {
            return Ok(JsValue::Undefined);
        }
    }

    // Duplicate check: WHATWG §2.6 step 4 excludes `once` / `passive`
    // from identity, so we look up by (type, capture, callback).
    if find_listener_id(ctx, entity, &event_type, options.capture, listener_obj_id).is_some() {
        return Ok(JsValue::Undefined);
    }

    // Register in the ECS component (creating the component if absent).
    // `insert_one` can fail if the entity has been despawned between
    // `entity_from_this`'s extraction and now (e.g. a concurrent DOM
    // mutation observer removed it).  In that case, skip the
    // listener_store insert entirely — storing it would create an
    // orphan entry that ECS could never reach for dispatch and that
    // would hold the JS function ObjectId rooted via gc until the
    // VM dies.
    let listener_id: Option<elidex_script_session::ListenerId> = {
        let dom = ctx.host().dom();
        if dom
            .world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .is_ok()
        {
            Some(
                dom.world_mut()
                    .get::<&mut elidex_script_session::EventListeners>(entity)
                    .expect("just-checked component must exist")
                    .add_with_options(event_type, options.capture, options.once, options.passive),
            )
        } else {
            let mut listeners = elidex_script_session::EventListeners::new();
            let id = listeners.add_with_options(
                event_type,
                options.capture,
                options.once,
                options.passive,
            );
            if dom.world_mut().insert_one(entity, listeners).is_ok() {
                Some(id)
            } else {
                None
            }
        }
    };

    let Some(listener_id) = listener_id else {
        return Ok(JsValue::Undefined);
    };

    // Stash the JS function ObjectId so dispatch can look it up.
    ctx.host().store_listener(listener_id, listener_obj_id);

    // Tie this listener to the AbortSignal back-ref list so
    // `controller.abort()` can detach it from the host's
    // `EventListeners` component.  Done after `store_listener` so
    // `detach_bound_listeners` (in `host::abort`) can clean up both
    // the ECS slot and the JS function root in lockstep.  Also write
    // a reverse index entry so `removeEventListener` can prune the
    // back-ref in O(1) when the listener is dropped before abort —
    // without that prune, a long-lived signal would accumulate stale
    // entries across add/remove cycles (Copilot R2 finding).
    if let Some(signal_id) = options.signal {
        if let Some(state) = ctx.vm.abort_signal_states.get_mut(&signal_id) {
            state.bound_listener_removals.insert(listener_id, entity);
            ctx.vm
                .abort_listener_back_refs
                .insert(listener_id, signal_id);
        }
    }

    Ok(JsValue::Undefined)
}

/// Extract the ECS Entity from `this` if it is a `HostObject` wrapper
/// AND `HostData` is currently bound.  Returns `None` (silent no-op)
/// for either failure mode, so callers can early-return without
/// further conditionals.
///
/// The bound-check covers a real scenario: JS code retains a
/// `HostObject` wrapper (e.g. saves `document` to a global, then
/// `Vm::unbind()` runs at end of the tick) and later invokes a
/// method on it across the unbind boundary.  Without the check,
/// the subsequent `host.dom()` call would panic on null pointers.
/// Treating it as a no-op matches browser behaviour for unattached
/// document references.
#[cfg(feature = "engine")]
pub(super) fn entity_from_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
) -> Option<elidex_ecs::Entity> {
    if !ctx.vm.host_data.as_deref().is_some_and(|h| h.is_bound()) {
        return None;
    }
    let JsValue::Object(id) = this else {
        return None;
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return None;
    };
    elidex_ecs::Entity::from_bits(entity_bits)
}

/// WebIDL branded receiver extraction.  Returns:
/// - `Ok(Some(entity))` when `this` is a bound HostObject wrapping
///   an entity whose inferred `NodeKind` matches `kind_matches`.
/// - `Ok(None)` when `this` is not a HostObject or the VM is
///   unbound — silent no-op, matches elidex's unbound-receiver
///   policy (post-unbind retained references must not panic).
/// - `Err(TypeError)` when `this` IS a HostObject but its kind
///   does NOT match — the WebIDL "Illegal invocation" brand
///   check that distinguishes `Function.prototype.call`-style
///   misuse from a silently invalid receiver.
///
/// `interface` / `method` are embedded in the error message
/// ("Failed to execute `<method>` on `<interface>`: Illegal
/// invocation"), mirroring browser DOMException text.
///
/// Uses `node_kind_inferred`, so legacy entities that carry DOM
/// payload without an explicit `NodeKind` are accepted when their
/// payload-derived kind matches.
#[cfg(feature = "engine")]
pub(super) fn require_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    interface: &str,
    method: &str,
    kind_matches: impl FnOnce(elidex_ecs::NodeKind) -> bool,
) -> Result<Option<elidex_ecs::Entity>, super::super::value::VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(None);
    };
    let dom = ctx.host().dom();
    // Differentiate a destroyed/invalid entity from a wrong-
    // interface receiver so the error message matches the actual
    // failure mode (`require_node_arg` makes the same split for
    // argument brand checks).
    if !dom.contains(entity) {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': \
             the node is detached (invalid entity)."
        )));
    }
    let Some(kind) = dom.node_kind_inferred(entity) else {
        return Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )));
    };
    if kind_matches(kind) {
        Ok(Some(entity))
    } else {
        Err(super::super::value::VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )))
    }
}

/// Parsed listener options — capture / once / passive / signal.
///
/// `signal` — `Some(signal_id)` when the caller passed an
/// `AbortSignal` via `{signal}`.  When that signal aborts, the
/// runtime detaches this listener from its host's
/// `EventListeners` component (see
/// [`super::abort::AbortSignalState::bound_listener_removals`]).
#[cfg(feature = "engine")]
#[derive(Default)]
pub(super) struct ListenerOptions {
    pub(super) capture: bool,
    pub(super) once: bool,
    pub(super) passive: bool,
    pub(super) signal: Option<ObjectId>,
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
            // Property names pre-interned in `WellKnownStrings` to avoid
            // a HashMap lookup per addEventListener options object.
            let mut out = ListenerOptions::default();
            for (key_sid, slot) in [
                (ctx.vm.well_known.capture, &mut out.capture),
                (ctx.vm.well_known.once, &mut out.once),
                (ctx.vm.well_known.passive, &mut out.passive),
            ] {
                let v = ctx
                    .vm
                    .get_property_value(opts_id, PropertyKey::String(key_sid))?;
                *slot = super::super::coerce::to_boolean(ctx.vm, v);
            }
            // `signal` (WebIDL `AbortSignal? signal`): `null` /
            // `undefined` / missing key all mean "no signal"; any
            // other non-AbortSignal value is a WebIDL conversion
            // failure that throws TypeError, matching browsers.
            let signal_val = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(ctx.vm.well_known.signal))?;
            out.signal = match signal_val {
                JsValue::Undefined | JsValue::Null => None,
                JsValue::Object(id)
                    if matches!(ctx.vm.get_object(id).kind, ObjectKind::AbortSignal) =>
                {
                    Some(id)
                }
                _ => {
                    return Err(VmError::type_error(
                        "Failed to execute 'addEventListener' on 'EventTarget': \
                         member signal is not of type 'AbortSignal'.",
                    ));
                }
            };
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

/// Extract ONLY the `capture` flag from `removeEventListener`'s
/// third argument.
///
/// WHATWG DOM §2.7.7's "flatten options" step for removal is
/// narrower than addEventListener's: it only consults the `capture`
/// property.  Reading `once` / `passive` (which `parse_listener_options`
/// does for addEventListener) would be observable through user
/// getters or Proxy traps, deviating from browser behaviour.  This
/// helper preserves that observability invariant by touching only
/// the capture slot.
#[cfg(feature = "engine")]
fn parse_capture_only(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<bool, VmError> {
    match val {
        JsValue::Undefined | JsValue::Null => Ok(false),
        JsValue::Boolean(capture) => Ok(capture),
        JsValue::Object(opts_id) => {
            let key_sid = ctx.vm.well_known.capture;
            let v = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(key_sid))?;
            Ok(super::super::coerce::to_boolean(ctx.vm, v))
        }
        // Other primitive types coerce via ToBoolean per WHATWG —
        // browsers accept e.g. `el.removeEventListener('click', f, 1)`.
        other => Ok(super::super::coerce::to_boolean(ctx.vm, other)),
    }
}

/// Find the `ListenerId` on `entity` whose (type, capture, callback)
/// triple matches the given arguments — returns the id if such a
/// listener is registered, `None` otherwise.
///
/// Used by both `addEventListener` (`.is_some()` for the §2.6 step 4
/// duplicate check) and `removeEventListener` (the actual id to drop).
/// Keeping a single helper prevents the two sites from drifting on
/// what counts as a "match" — historically `removeEventListener`
/// picked the first (type, capture) entry and bailed if its callback
/// didn't match, missing later matching entries when an element had
/// multiple listeners of the same type+capture.
///
/// **Precondition**: caller must have verified `HostData` is bound
/// (typically by passing `entity` from a successful
/// [`entity_from_this`] call).  This helper calls `ctx.host().dom()`
/// directly, which would panic on a null dom pointer.
#[cfg(feature = "engine")]
fn find_listener_id(
    ctx: &mut NativeContext<'_>,
    entity: elidex_ecs::Entity,
    event_type: &str,
    capture: bool,
    candidate_obj_id: ObjectId,
) -> Option<elidex_script_session::ListenerId> {
    // Two-pass: collect candidate ids while holding the world borrow
    // (scoped to the inner block), then cross-reference each against
    // `listener_store` (which goes through `host_data` and would
    // otherwise conflict with the world borrow).
    let candidate_ids: Vec<_> = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_script_session::EventListeners>(entity)
            .ok()
            .map(|listeners| {
                // `iter_matching` skips the `Vec<&ListenerEntry>`
                // intermediate alloc that `matching_all` would
                // require — `addEventListener` (duplicate check) and
                // `removeEventListener` (find target) call this on
                // every invocation, so the alloc savings add up.
                listeners
                    .iter_matching(event_type)
                    .filter(|e| e.capture == capture)
                    .map(|e| e.id)
                    .collect()
            })
            .unwrap_or_default()
    };
    let host = ctx.host();
    candidate_ids
        .into_iter()
        .find(|id| host.get_listener(*id) == Some(candidate_obj_id))
}

/// `EventTarget.prototype.removeEventListener` — non-engine-feature stub.
#[cfg(not(feature = "engine"))]
pub(super) fn native_event_target_remove_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.removeEventListener(type, listener, options)`.
///
/// WHATWG DOM §2.7.7: locate any listener whose (type, callback,
/// capture) tuple matches and remove it from the entity's
/// `EventListeners` component, `HostData::listener_store`, and any
/// `AbortSignal` back-ref pointing at it (the last via the shared
/// [`super::super::VmInner::remove_listener_and_prune_back_ref`]
/// helper, which is also called from the `{once}` auto-removal
/// path).
///
/// Behaviour parallels [`native_event_target_add_event_listener`]:
/// - Non-HostObject `this` → silent no-op.
/// - `null` / `undefined` callback → silent no-op (§2.7.7 step 2:
///   "If callback is null, then return.").
/// - Other non-callable callback → throws `TypeError` (matches
///   `addEventListener`'s WebIDL `EventListener?` conversion;
///   silently dropping `el.removeEventListener('click', 42)`
///   would mask user bugs).
/// - Options third arg: only `capture` is read from the object form
///   (`once` / `passive` are not part of identity per §2.6 step 4
///   and reading them would fire user getters that browsers don't
///   trigger here — see `parse_capture_only`).
#[cfg(feature = "engine")]
pub(super) fn native_event_target_remove_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };

    // WebIDL `EventListener? callback`: null/undefined are explicitly
    // allowed and become a silent no-op (§2.7.7 step 2); any other
    // non-callable value is a conversion failure that throws
    // `TypeError`, matching `addEventListener` and browser behaviour
    // (silently dropping `el.removeEventListener('click', 42)` would
    // mask user bugs).
    let callback_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let listener_obj_id = match callback_arg {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Undefined),
        JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'removeEventListener' on 'EventTarget': \
                 parameter 2 is not of type 'EventListener'.",
            ));
        }
    };

    let type_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    let event_type = ctx.vm.strings.get_utf8(type_sid);

    // Spec §2.7.7: only `capture` is read from options for removal.
    // `parse_listener_options` would also read `once` / `passive`,
    // which would fire user getters / Proxy traps that browsers
    // don't trigger here.
    let capture = parse_capture_only(ctx, args.get(2).copied().unwrap_or(JsValue::Undefined))?;

    // Locate the matching listener via the shared (type, capture,
    // callback) lookup.  WHATWG §2.6 step 4 forbids duplicates so at
    // most one match exists; if none matches, it's a silent no-op.
    let Some(listener_id) = find_listener_id(ctx, entity, &event_type, capture, listener_obj_id)
    else {
        return Ok(JsValue::Undefined);
    };

    // Remove from ECS component first (scoped block so the world
    // borrow is released before the centralized listener-retirement
    // helper re-grabs the host).
    {
        let dom = ctx.host().dom();
        if let Ok(mut listeners) = dom
            .world_mut()
            .get::<&mut elidex_script_session::EventListeners>(entity)
        {
            listeners.remove(listener_id);
        }
    }
    // Single helper that drops the listener from both
    // `HostData::listener_store` AND any AbortSignal back-ref
    // (`abort_listener_back_refs` + the per-signal HashMap).  Same
    // helper is invoked by `Engine::remove_listener` for the {once}
    // auto-removal path so both retirement routes stay in sync.
    ctx.vm.remove_listener_and_prune_back_ref(listener_id);

    Ok(JsValue::Undefined)
}

// Node-level accessors / methods live on `Node.prototype`
// (`vm/host/node_proto.rs`), which chains to `EventTarget.prototype`.
// Splitting them out keeps non-Node EventTargets like `window`
// (Window.prototype → EventTarget.prototype) from exposing
// `parentNode` / `nodeType` / `textContent`, matching the Web
// platform where Window is an EventTarget but not a Node.

// (Node-level natives moved to vm/host/node_proto.rs)

/// `EventTarget.prototype.dispatchEvent(event)` — non-engine stub.
///
/// Without `engine`, there is no DOM to dispatch against; return `false`
/// (spec default for "event not dispatched") and skip the arg /
/// receiver brand-checks the engine path performs.
#[cfg(not(feature = "engine"))]
pub(super) fn native_event_target_dispatch_event(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(false))
}

/// `EventTarget.prototype.dispatchEvent(event)` — WHATWG DOM §2.9.
///
/// Runs the script-side dispatch algorithm on a user-constructed
/// `Event` (from `new Event(...)` / `CustomEvent` / a specialized
/// constructor).  Unlike the UA-initiated path, which rebuilds a
/// fresh JS event per listener invocation, this path **reuses the
/// caller's single event object**: `target` / `currentTarget` /
/// `eventPhase` are mutated in-place on its core-9 shape slots as
/// the dispatch walks the tree, so all listeners see the same
/// Event identity (`e === e` inside every handler, required by
/// WHATWG §2.9).
///
/// Returns `true` if no listener called `preventDefault()` and
/// the event was cancelable; `false` otherwise.  Errors are
/// surfaced only for the three precondition throws (non-Event arg
/// → TypeError, dispatch-flag set → `InvalidStateError`
/// `DOMException`, and the WebIDL receiver brand check is a
/// silent no-op per elidex's detach-tolerant convention).
///
/// Listener body throws are caught and ignored — this matches
/// WHATWG §2.10 step 10 "report the exception" and keeps dispatch
/// advancing through the remainder of the plan.
#[cfg(feature = "engine")]
pub(super) fn native_event_target_dispatch_event(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // ---- 1. WebIDL arg binding: `event` must be an `Event` ----
    // WebIDL `Event event` throws TypeError before any of §2.9's
    // receiver logic runs.  Missing arg is also a TypeError (no
    // default).  Validate first so wrong-arg / missing-arg error
    // paths stay observable regardless of receiver state.
    let event_id = match args.first().copied() {
        Some(JsValue::Object(id))
            if matches!(ctx.vm.get_object(id).kind, ObjectKind::Event { .. }) =>
        {
            id
        }
        Some(_) | None => {
            return Err(VmError::type_error(
                "Failed to execute 'dispatchEvent' on 'EventTarget': \
                 parameter 1 is not of type 'Event'.",
            ));
        }
    };

    // ---- 2. Receiver brand check ----
    // `entity_from_this` returns None for unbound / non-HostObject
    // receivers, matching `addEventListener`'s silent-no-op policy
    // (detach-tolerant: JS that retained `document` across
    // `Vm::unbind()` gets `false` instead of a panic).
    let Some(target_entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(true));
    };

    // ---- 3. Dispatch-flag check (WHATWG §2.9 step 1) ----
    // `dispatched_events` tracks in-flight membership only — the
    // set-and-clear is bracketed across the walk, so a sequential
    // `dispatchEvent(e); dispatchEvent(e);` succeeds both times
    // while a re-entrant `e.currentTarget.dispatchEvent(e)` from
    // inside a listener for `e` throws `InvalidStateError`.
    if ctx.vm.dispatched_events.contains(&event_id) {
        let name_sid = ctx.vm.well_known.dom_exc_invalid_state_error;
        return Err(super::dom_exception::invalid_state_error(
            name_sid,
            "EventTarget",
            "dispatchEvent",
            "The event is already being dispatched.",
        ));
    }

    // ---- 4. Perform the dispatch ----
    // Bracketed with `dispatched_events` insert/remove so a throw
    // from any post-precondition step (very rare — listener
    // throws are caught inside the walk, but a VM-level error
    // path could still hit `?`) still clears membership.
    ctx.vm.dispatched_events.insert(event_id);
    let result = dispatch_script_event(ctx, event_id, target_entity);
    ctx.vm.dispatched_events.remove(&event_id);
    result.map(JsValue::Boolean)
}

/// Inner dispatch walker — assumed preconditions (caller-validated):
/// - `event_id` names an `ObjectKind::Event` with the PR3.6
///   precomputed-shape layout.
/// - `target_entity` is a bound HostObject's backing entity.
/// - `ctx.vm.dispatched_events` already has `event_id` inserted.
///
/// Return contract: `Ok(!default_prevented)` on normal completion;
/// errors are surfaced only if the VM itself cannot continue
/// (e.g. allocator failure).  Listener-body throws are caught and
/// ignored (spec §2.10 "report the exception") so the walk
/// advances past them.
#[cfg(feature = "engine")]
fn dispatch_script_event(
    ctx: &mut NativeContext<'_>,
    event_id: ObjectId,
    target_entity: elidex_ecs::Entity,
) -> Result<bool, VmError> {
    use super::events::{
        set_event_slot_raw, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_EVENT_PHASE, EVENT_SLOT_TARGET,
    };
    use elidex_plugin::EventPhase;

    // ---- A. Extract the event's invariant attributes ----
    // The `type`, `bubbles`, `cancelable`, `composed` slots never
    // change across dispatch (they are WebIDL `readonly` per §2.2),
    // so one read up front suffices.  `cancelable` is read from
    // the internal slot (same source of truth as
    // `Event.prototype.preventDefault` consults) rather than the
    // data slot — both agree but the internal read is cheaper.
    let (event_type_str, bubbles, cancelable, composed) = {
        let obj = ctx.vm.get_object(event_id);
        let ObjectKind::Event { cancelable, .. } = obj.kind else {
            // Unreachable — caller's brand check established this.
            unreachable!("dispatch_script_event: receiver is not ObjectKind::Event");
        };
        let PropertyStorage::Shaped { slots, .. } = &obj.storage else {
            unreachable!("Event must use Shaped storage");
        };
        let type_sid = match slots[0] {
            PropertyValue::Data(JsValue::String(sid)) => sid,
            _ => unreachable!("Event slot 0 (type) must be a String"),
        };
        let bubbles = matches!(slots[1], PropertyValue::Data(JsValue::Boolean(true)));
        let composed = matches!(slots[7], PropertyValue::Data(JsValue::Boolean(true)));
        (
            ctx.vm.strings.get_utf8(type_sid),
            bubbles,
            cancelable,
            composed,
        )
    };

    // ---- B. Build the local DispatchEvent shim ----
    // The session crate's `build_dispatch_plan` / `apply_retarget`
    // walk over a `DispatchEvent` Rust struct.  We project the
    // user's JS event into one (shallow projection — no payload,
    // no composed_path yet) so the shared helpers are reused.
    // Flag bits are loaded from the internal slots so that a
    // user who constructed the event with `default_prevented:
    // true` (impossible via ctor, but possible via direct slot
    // mutation) is respected.
    let initial_flags = {
        let ObjectKind::Event {
            default_prevented,
            propagation_stopped,
            immediate_propagation_stopped,
            ..
        } = ctx.vm.get_object(event_id).kind
        else {
            unreachable!();
        };
        DispatchFlags {
            default_prevented,
            propagation_stopped,
            immediate_propagation_stopped,
        }
    };
    // `DispatchEvent` is `#[non_exhaustive]` (cross-crate boundary —
    // the session crate owns it), so direct struct literal is
    // rejected.  `new_untrusted` sets `is_trusted = false` plus
    // `EventPayload::None` + `EventPhase::None`; we override the
    // user-facing invariants below.  `build_dispatch_plan` /
    // `apply_retarget` only read target / composed / flags, so the
    // default payload is correct.
    let mut local = DispatchEvent::new_untrusted(event_type_str, target_entity);
    local.bubbles = bubbles;
    local.cancelable = cancelable;
    local.composed = composed;
    local.flags = initial_flags;
    local.dispatch_flag = true;

    // ---- C. Build dispatch plan + composed path ----
    // Scoped DOM borrow so subsequent `create_element_wrapper`
    // calls (which need `&mut ctx.vm`) don't overlap.
    let plan = {
        let dom = ctx.host().dom();
        let p = build_dispatch_plan(dom, &local);
        local.composed_path = build_propagation_path(dom, local.target, local.composed);
        p
    };

    // ---- D. Seed the user event's `composed_path` internal slot ----
    // Build one Array of wrappers mirroring `create_event_object`'s
    // per-listener UA path (PR3 D4).  `composedPath()` is
    // identity-preserving during dispatch (§2.9 "same Array"
    // requirement) — the cached slot wins on subsequent calls.
    // After dispatch completes, the slot is cleared (see Step G)
    // so post-dispatch `composedPath()` returns `[]` via the
    // lazy-alloc fallback in `natives_event::native_event_composed_path`.
    let saved_target_wrapper_id = ctx.vm.create_element_wrapper(target_entity);
    {
        // Guard `saved_target_wrapper_id` across wrapper allocations
        // for the composed-path entries; it's already rooted via
        // `wrapper_cache`, so this is belt-and-braces in case GC
        // trims the cache between allocations.
        let mut g = ctx
            .vm
            .push_temp_root(JsValue::Object(saved_target_wrapper_id));
        let elements: Vec<JsValue> = local
            .composed_path
            .iter()
            .map(|&entity| JsValue::Object(g.create_element_wrapper(entity)))
            .collect();
        let arr_id = g.create_array_object(elements);
        if let ObjectKind::Event { composed_path, .. } = &mut g.get_object_mut(event_id).kind {
            *composed_path = Some(arr_id);
        }
        drop(g);
    }

    // ---- E. Seed `target` slot to the original target wrapper ----
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_TARGET,
        JsValue::Object(saved_target_wrapper_id),
    );

    // ---- F. Walk the three phases ----
    let saved_target = local.target;
    // Phase 1: Capture (root → target, exclusive).
    walk_phase(
        ctx,
        event_id,
        &plan.capture,
        EventPhase::Capturing,
        &mut local,
        saved_target,
    )?;

    // Phase 2: At-target.
    if !local.flags.propagation_stopped && !local.flags.immediate_propagation_stopped {
        if let Some(at_target) = plan.at_target.as_ref() {
            local.target = saved_target;
            local.original_target = None;
            walk_phase(
                ctx,
                event_id,
                std::slice::from_ref(at_target),
                EventPhase::AtTarget,
                &mut local,
                saved_target,
            )?;
        }
    }

    // Phase 3: Bubble (target → root, exclusive, reversed).
    if bubbles && !local.flags.propagation_stopped && !local.flags.immediate_propagation_stopped {
        walk_phase(
            ctx,
            event_id,
            &plan.bubble,
            EventPhase::Bubbling,
            &mut local,
            saved_target,
        )?;
    }

    // ---- G. Finalise — §2.9 steps 27-31 ----
    // Unset dispatch flag + propagation flags (default_prevented
    // is NOT reset — it is the canceled-flag bit that the caller
    // inspects via the return value).  Restore target to its
    // original (pre-retarget) wrapper.  Clear currentTarget +
    // eventPhase so post-dispatch reads see the "no longer
    // dispatching" state (§2.9 step 30-31).  Clear
    // `composed_path` internal slot so a subsequent
    // `composedPath()` call returns `[]` via the lazy-alloc
    // branch in `natives_event::native_event_composed_path`.
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_TARGET,
        JsValue::Object(saved_target_wrapper_id),
    );
    set_event_slot_raw(ctx.vm, event_id, EVENT_SLOT_CURRENT_TARGET, JsValue::Null);
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_EVENT_PHASE,
        JsValue::Number(0.0),
    );
    if let ObjectKind::Event {
        propagation_stopped,
        immediate_propagation_stopped,
        composed_path,
        ..
    } = &mut ctx.vm.get_object_mut(event_id).kind
    {
        *propagation_stopped = false;
        *immediate_propagation_stopped = false;
        *composed_path = None;
    }

    // ---- H. Return `!default_prevented` ----
    let prevented = matches!(
        ctx.vm.get_object(event_id).kind,
        ObjectKind::Event {
            default_prevented: true,
            ..
        }
    );
    Ok(!prevented)
}

/// One-phase walk over `entries`, invoking each listener with the
/// user's event object after in-place mutation of target /
/// currentTarget / eventPhase slots.
///
/// `phase` drives the `eventPhase` slot write (see `EventPhase` repr);
/// `saved_target` is the pre-retarget target used by
/// `apply_retarget`; `local` carries the session-crate flag bits
/// that gate the outer phase switches.
///
/// Listener errors are caught and ignored; internal flags (default_prevented,
/// propagation_stopped, immediate_propagation_stopped) are synced
/// from the event object back to `local` after each invocation so
/// the per-phase propagation gates respond to listener mutations
/// made via `Event.prototype.{preventDefault, stopPropagation,
/// stopImmediatePropagation}`.
#[cfg(feature = "engine")]
fn walk_phase(
    ctx: &mut NativeContext<'_>,
    event_id: ObjectId,
    entries: &[(
        elidex_ecs::Entity,
        Vec<elidex_script_session::event_dispatch::ListenerPlanEntry>,
    )],
    phase: elidex_plugin::EventPhase,
    local: &mut DispatchEvent,
    saved_target: elidex_ecs::Entity,
) -> Result<(), VmError> {
    use super::events::{
        set_event_slot_raw, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_EVENT_PHASE, EVENT_SLOT_TARGET,
    };
    use elidex_script_session::EventListeners;

    for (entity, listener_entries) in entries {
        if local.flags.propagation_stopped || local.flags.immediate_propagation_stopped {
            break;
        }

        // Retarget (§2.5) — updates `local.target` which we mirror
        // into the event's `target` slot.  Retarget is a no-op for
        // the common case (no shadow host crossing), matching the
        // `retarget(A, B)` algorithm's identity short-circuit.
        {
            let dom = ctx.host().dom();
            apply_retarget(local, *entity, saved_target, dom);
        }
        local.current_target = Some(*entity);
        local.phase = phase;

        // Update the JS event slots to match the current phase
        // state.  `currentTarget` always changes per entity; the
        // target slot only needs a rewrite when `apply_retarget`
        // actually moved it (common case: no shadow crossing, so
        // `local.target == saved_target` across the whole walk
        // and the per-entity wrapper resolve is skippable).
        let target_wrapper = ctx.vm.create_element_wrapper(local.target);
        let current_wrapper = ctx.vm.create_element_wrapper(*entity);
        set_event_slot_raw(
            ctx.vm,
            event_id,
            EVENT_SLOT_TARGET,
            JsValue::Object(target_wrapper),
        );
        set_event_slot_raw(
            ctx.vm,
            event_id,
            EVENT_SLOT_CURRENT_TARGET,
            JsValue::Object(current_wrapper),
        );
        set_event_slot_raw(
            ctx.vm,
            event_id,
            EVENT_SLOT_EVENT_PHASE,
            JsValue::Number(f64::from(phase as u8)),
        );

        for entry in listener_entries {
            if local.flags.immediate_propagation_stopped {
                break;
            }

            // §2.10 step 15: remove `once` listeners BEFORE
            // invocation so re-entrant dispatch sees them gone.
            // The corresponding `listener_store` + AbortSignal
            // back-ref cleanup happens after `call_value`
            // returns so it also runs for listeners that were
            // NOT `once` but whose invocation threw.
            if entry.once {
                let dom = ctx.host().dom();
                if let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(*entity) {
                    listeners.remove(entry.id);
                }
            }

            // Resolve the JS function; a miss means the listener
            // was removed between plan-freeze and now (addEventListener
            // inside an earlier listener can't add to this plan
            // but removeEventListener / abort signals can drop
            // planned entries).  Silent continue matches §2.10
            // step 5.4.
            let Some(host) = ctx.vm.host_data.as_deref() else {
                continue;
            };
            let Some(func_obj_id) = host.get_listener(entry.id) else {
                continue;
            };

            // Invoke the listener.  `this` is the currentTarget
            // wrapper per §2.10 step 15 (matches WebIDL callback
            // `this` binding).  Throw propagation is swallowed
            // (session crate parity — `script_dispatch_event_core`
            // ignores engine.call_listener's discarded Result).
            let _ = ctx.call_value(
                JsValue::Object(func_obj_id),
                JsValue::Object(current_wrapper),
                &[JsValue::Object(event_id)],
            );

            // Sync internal flag state back to the local walker.
            // Capture BEFORE the `once` cleanup below because
            // retiring the ListenerId drops the `listener_store`
            // entry but doesn't touch the Event flags.
            if let ObjectKind::Event {
                default_prevented,
                propagation_stopped,
                immediate_propagation_stopped,
                ..
            } = ctx.vm.get_object(event_id).kind
            {
                local.flags.default_prevented = default_prevented;
                local.flags.propagation_stopped = propagation_stopped;
                local.flags.immediate_propagation_stopped = immediate_propagation_stopped;
            }

            // Post-listener cleanup: drop the engine-side
            // function store entry + any AbortSignal back-ref.
            // Shared with `removeEventListener` to keep the
            // back-ref indexes bounded across `{once}` +
            // `{signal}` combinations.
            if entry.once {
                ctx.vm.remove_listener_and_prune_back_ref(entry.id);
            }

            // HTML §8.1.7.3 microtask checkpoint — drain
            // Promise reactions queued by the listener before
            // invoking the next listener.  Matches the session
            // crate's UA-initiated dispatch walk.
            ctx.vm.drain_microtasks();
        }
    }
    Ok(())
}
