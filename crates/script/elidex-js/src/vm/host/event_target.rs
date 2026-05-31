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
use super::super::value::{ObjectId, ObjectKind, PropertyKey};
use super::super::{NativeFn, VmInner};
#[cfg(feature = "engine")]
use super::dispatch_target::{target_from_this, DispatchTarget};
#[cfg(feature = "engine")]
use super::event_target_dispatch::dispatch_script_event;
#[cfg(feature = "engine")]
use super::event_target_dispatch_vm::dispatch_vm_event;

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
    let Some(target) = target_from_this(ctx, this) else {
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
    if find_listener_id(ctx, target, &event_type, options.capture, listener_obj_id).is_some() {
        return Ok(JsValue::Undefined);
    }

    // Register in the target's listener home (the ECS `EventListeners`
    // component for a `Node`, `vm_event_listeners` for a `VmObject`),
    // lazily creating it.  `None` means a `Node` entity was despawned
    // between receiver resolution and now (insert failed) — skip the
    // paired `store_listener` so no orphan rooted ObjectId is left.
    let listener_id = target.with_listeners_mut_or_insert(ctx, |listeners| {
        listeners.add_with_options(event_type, options.capture, options.once, options.passive)
    });

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
            state.bound_listener_removals.insert(listener_id, target);
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
    if !ctx
        .vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
    {
        return None;
    }
    let JsValue::Object(id) = this else {
        return None;
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => elidex_ecs::Entity::from_bits(entity_bits),
        _ => None,
    }
}

/// Test-only / brand-check helper: returns `true` when `this`
/// resolves to a DOM wrapper (`HostObject`) regardless of bind state.
/// Callers that need to enforce WebIDL "Illegal invocation" semantics
/// for non-wrapper receivers (e.g. `Element.prototype.shadowRoot.call({})`)
/// should pre-throw via this check BEFORE invoking [`require_receiver`],
/// because `require_receiver` collapses non-wrapper receivers and
/// unbound HostObject receivers into the same `Ok(None)` branch (the
/// elidex unbound-receiver policy requires silent no-op for the
/// latter).
///
/// Returns `false` for primitive receivers and any `Object` whose
/// kind isn't `HostObject` (Function, Array, plain Ordinary, etc.).
/// Does NOT attempt to decode the entity bits — the pre-throw is a
/// wrapper-kind brand check, not an entity-existence probe; callers
/// that need entity existence run [`entity_from_this`] (which
/// performs the decode AND the bind-state check) before invoking
/// any DOM operation.
#[cfg(feature = "engine")]
pub(super) fn this_is_node_wrapper(vm: &super::super::VmInner, this: JsValue) -> bool {
    let JsValue::Object(id) = this else {
        return false;
    };
    matches!(vm.get_object(id).kind, ObjectKind::HostObject { .. })
}

/// Engine-component brand check: `entity` carries an
/// `elidex_ecs::ShadowRoot` component in the currently-bound DOM.
/// Returns `false` when unbound (silent — callers fall through to
/// the standard non-shadow path).
///
/// This is the post-H-migration replacement for matching against
/// `ObjectKind::ShadowRoot` ([feedback_objectkind-resolution-uniformity]).
/// Use this whenever code needs to distinguish a shadow-root wrapper
/// from any other DOM wrapper — both are `HostObject { entity_bits }`
/// now, only the ECS component differentiates them.  Delegates to
/// [`HostData::is_shadow_root_entity`] which encapsulates the
/// unsafe `dom_ptr` deref + aliasing contract (sibling of
/// [`HostData::is_element_entity`]).
#[cfg(feature = "engine")]
pub(super) fn is_shadow_root_entity(
    vm: &super::super::VmInner,
    entity: elidex_ecs::Entity,
) -> bool {
    vm.host_data
        .as_deref()
        .is_some_and(|hd| hd.is_shadow_root_entity(entity))
}

/// WebIDL branded receiver extraction.  Returns:
/// - `Ok(Some(entity))` when `this` is a bound HostObject wrapping
///   an entity whose inferred `NodeKind` matches `kind_matches`.
/// - `Ok(None)` when `this` is not a HostObject or the VM is
///   unbound — silent no-op, matches elidex's unbound-receiver
///   policy (post-unbind retained references must not panic).
///   Callers that need WebIDL-strict TypeError for non-wrapper
///   receivers (e.g. plain `{}`) should pre-check via
///   [`this_is_node_wrapper`] before invoking this helper.
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
pub(super) fn parse_listener_options(
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
/// Works for either listener home (ECS component for a `Node`,
/// `vm_event_listeners` for a `VmObject`) via the listener-home adapter;
/// tolerant of a missing home / absent `HostData` (returns `None`).
#[cfg(feature = "engine")]
fn find_listener_id(
    ctx: &mut NativeContext<'_>,
    target: DispatchTarget,
    event_type: &str,
    capture: bool,
    candidate_obj_id: ObjectId,
) -> Option<elidex_script_session::ListenerId> {
    // Two-pass: collect candidate ids from the listener home (scoped via
    // the adapter), then cross-reference each against `listener_store`
    // (the engine-side callback map, shared by both homes).
    let candidate_ids: Vec<_> = target
        .with_listeners(ctx, |listeners| {
            // `iter_matching` skips the `Vec<&ListenerEntry>`
            // intermediate alloc that `matching_all` would require —
            // `addEventListener` (duplicate check) and
            // `removeEventListener` (find target) call this on every
            // invocation, so the alloc savings add up.
            //
            // WHATWG HTML §8.1.8.1: event-handler listeners (backing
            // `el.onclick = fn`) are NOT identity-matchable by
            // `removeEventListener(type, fn)` nor by the
            // `addEventListener` duplicate check — their callback is the
            // internal event-handler processing algorithm, not the raw
            // `fn`. Filter to `Normal` so handler listeners are excluded
            // from both paths (they are managed solely through the IDL
            // attribute / content-attribute surface).
            listeners
                .iter_matching(event_type)
                .filter(|e| {
                    e.capture == capture
                        && matches!(e.kind, elidex_script_session::ListenerKind::Normal)
                })
                .map(|e| e.id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let host = ctx.vm.host_data.as_deref();
    candidate_ids
        .into_iter()
        .find(|id| host.and_then(|h| h.get_listener(*id)) == Some(candidate_obj_id))
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
    let Some(target) = target_from_this(ctx, this) else {
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
    let Some(listener_id) = find_listener_id(ctx, target, &event_type, capture, listener_obj_id)
    else {
        return Ok(JsValue::Undefined);
    };

    // Remove from the listener home (ECS component for a `Node`,
    // `vm_event_listeners` for a `VmObject`) via the adapter.
    target.remove_listener_entry(ctx, listener_id);
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
/// WHATWG §2.9 inner-invoke "report the exception" and keeps dispatch
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
    // receivers; we fall through to the spec's "event not
    // canceled" default of `true` (matches `addEventListener`'s
    // silent-no-op policy — detach-tolerant: JS that retained
    // `document` across `Vm::unbind()` gets `true` instead of a
    // panic, and a plain-object receiver reached via
    // `dispatchEvent.call({}, ...)` behaves the same way).
    let Some(target) = target_from_this(ctx, this) else {
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
    // §2.9 get-the-parent provider: a `Node` builds its DOM-tree / shadow
    // plan; a `VmObject` builds its (flat or ancestor-chain) plan.  Both
    // feed the one shared inner invoke.
    let result = match target {
        DispatchTarget::Node(entity) => dispatch_script_event(ctx, event_id, entity),
        DispatchTarget::VmObject(id) => {
            dispatch_vm_event(ctx, event_id, id).map(|outcome| outcome.not_prevented)
        }
    };
    ctx.vm.dispatched_events.remove(&event_id);
    result.map(JsValue::Boolean)
}
