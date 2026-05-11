//! Script-initiated `dispatchEvent` walk (WHATWG DOM §2.9-§2.10).
//!
//! Split from [`super::event_target`] to keep both files below the
//! 1000-line convention (cleanup tranche 2).  The entry point
//! [`dispatch_script_event`] is invoked by
//! [`super::event_target::native_event_target_dispatch_event`] after
//! receiver brand check + dispatch-flag tracking; this module owns
//! the actual three-phase walk (capture → at-target → bubble) and
//! the per-phase listener invocation loop ([`walk_phase`]).
//!
//! See [`super::event_target`]'s module doc for the higher-level
//! `addEventListener` / `removeEventListener` registration path
//! that ultimately routes through this dispatcher when scripts call
//! `target.dispatchEvent(evt)`.

#![cfg(feature = "engine")]

use elidex_plugin::EventPhase;
use elidex_script_session::event_dispatch::{
    apply_retarget, build_dispatch_plan, build_propagation_path, DispatchEvent, DispatchFlags,
};
use elidex_script_session::EventListeners;

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};
use super::events::{
    set_event_slot_raw, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_EVENT_PHASE, EVENT_SLOT_TARGET,
};

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
#[allow(clippy::too_many_lines)] // single capture/target/bubble walk — splitting would scatter §2.10 flow
pub(super) fn dispatch_script_event(
    ctx: &mut NativeContext<'_>,
    event_id: ObjectId,
    target_entity: elidex_ecs::Entity,
) -> Result<bool, VmError> {
    // ---- A. Extract the event's invariant attributes ----
    // The `type`, `bubbles`, `cancelable`, `composed` slots never
    // change across dispatch (they are WebIDL `readonly` per §2.2),
    // so one read up front suffices.  `cancelable` is read from
    // the internal slot (same source of truth as
    // `Event.prototype.preventDefault` consults) rather than the
    // data slot — both agree but the internal read is cheaper.
    // Read type / bubbles / cancelable / composed from the
    // authoritative `ObjectKind::Event` internal slots — not from
    // the JS-visible data properties.  WebIDL specifies these as
    // readonly IDL attributes backed by internal slots; browsers
    // use the slot value even when a user did `delete evt.type` or
    // shadowed the accessor on the instance.  Elidex mirrors this
    // by keeping the internal slot authoritative; the data
    // property is a mirror installed for enumeration / ergonomic
    // access but cannot hijack dispatch.
    let ObjectKind::Event {
        type_sid,
        bubbles,
        cancelable,
        composed,
        ..
    } = ctx.vm.get_object(event_id).kind
    else {
        unreachable!("dispatch_script_event: receiver is not ObjectKind::Event")
    };
    let event_type_str = ctx.vm.strings.get_utf8(type_sid);

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
    // `last_written_target` mirrors the `target` slot state across
    // the whole walk.  Seeded to `saved_target` because Step E just
    // wrote `saved_target_wrapper_id` into the slot; walk_phase
    // only rewrites when `local.target` diverges from this tracker
    // (common case: no shadow crossing → never diverges → slot
    // stays at saved_target without per-listener wrapper lookups).
    let mut last_written_target = saved_target;
    // Phase 1: Capture (root → target, exclusive).
    walk_phase(
        ctx,
        event_id,
        &plan.capture,
        EventPhase::Capturing,
        &mut local,
        saved_target,
        &mut last_written_target,
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
                &mut last_written_target,
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
            &mut last_written_target,
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
fn walk_phase(
    ctx: &mut NativeContext<'_>,
    event_id: ObjectId,
    entries: &[(
        elidex_ecs::Entity,
        Vec<elidex_script_session::event_dispatch::ListenerPlanEntry>,
    )],
    phase: EventPhase,
    local: &mut DispatchEvent,
    saved_target: elidex_ecs::Entity,
    last_written_target: &mut elidex_ecs::Entity,
) -> Result<(), VmError> {
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
        // state.  `currentTarget` always changes per entity and is
        // always rewritten.  `target` only needs a rewrite when
        // `apply_retarget` moved it — the common case (no shadow
        // crossing) has `local.target == saved_target` across the
        // whole walk, so the tracker skips the wrapper_cache lookup
        // plus slot write for every listener after the first.  The
        // tracker threads across phases through `dispatch_script_event`
        // so a phase-1 retarget that gets reverted in phase 2 still
        // restores the slot correctly.
        if *last_written_target != local.target {
            let target_wrapper = ctx.vm.create_element_wrapper(local.target);
            set_event_slot_raw(
                ctx.vm,
                event_id,
                EVENT_SLOT_TARGET,
                JsValue::Object(target_wrapper),
            );
            *last_written_target = local.target;
        }
        let current_wrapper = ctx.vm.create_element_wrapper(*entity);
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

            // WHATWG DOM §2.10 step 15: passive is a per-listener
            // bit, not a per-event one — the event's `in passive
            // listener flag` (our `ObjectKind::Event.passive`
            // internal slot) is temporarily set to the invoking
            // listener's `passive` for the call and cleared
            // afterward so `preventDefault()` can observe it.
            // UA-dispatch handles this by rebuilding the JS event
            // object per listener with the correct `passive`;
            // script-dispatch reuses the user's event object, so
            // we toggle the slot around each call_value instead.
            if let ObjectKind::Event { passive, .. } = &mut ctx.vm.get_object_mut(event_id).kind {
                *passive = entry.passive;
            }
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
            // Restore the slot to `false` so post-dispatch
            // `preventDefault()` (if called from outside any
            // listener — e.g. a microtask scheduled during
            // dispatch) observes the default non-passive state.
            if let ObjectKind::Event { passive, .. } = &mut ctx.vm.get_object_mut(event_id).kind {
                *passive = false;
            }

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

/// Construct a plain `Event` with the given `type` / `bubbles` /
/// `cancelable` flags and dispatch it on `target_entity`.  Returns
/// `true` when the dispatch was cancelled (default-prevented).
///
/// Shared helper for UA-initiated synthetic Event dispatch:
/// `form.reset()` fires `reset` (bubbles=true, cancelable=true),
/// `checkValidity()` fires `invalid` (bubbles=false,
/// cancelable=true).  Lifecycle bracket matches
/// [`super::pending_tasks::deliver_post_message`]: alloc the
/// Event, install core-9 own-data slots immediately so a GC
/// triggered by the slot install cannot collect the freshly-
/// returned id, register in `dispatched_events` for the dispatch
/// window, walk via [`dispatch_script_event`], unregister.
pub(super) fn dispatch_simple_event(
    ctx: &mut NativeContext<'_>,
    target_entity: elidex_ecs::Entity,
    type_sid: super::super::value::StringId,
    bubbles: bool,
    cancelable: bool,
) -> Result<bool, VmError> {
    use super::super::value::PropertyValue;

    let event_proto = ctx.vm.event_prototype;
    let target_wrapper = ctx.vm.create_element_wrapper(target_entity);
    let core_shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;

    let event_id = ctx.vm.alloc_object(super::super::value::Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable,
            passive: false,
            type_sid,
            bubbles,
            composed: false,
            composed_path: None,
        },
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: event_proto,
        extensible: true,
    });

    // GC safety — root `event_id` *immediately* after allocation
    // by inserting into `dispatched_events`, which `gc/roots.rs`
    // step (j.4) now treats as a real GC root.  This covers both
    // the slot-install phase and any transitive allocation inside
    // `dispatch_script_event` (composedPath wrappers, listener-
    // fired user code, etc.) without the borrow gymnastics needed
    // to thread a `push_temp_root` guard across the dispatch call.
    //
    // Panic safety — the workspace forbids `unsafe` (`-D
    // unsafe-code`), so an RAII guard holding a `*mut VmInner` is
    // not viable, and a safe guard cannot re-borrow `vm` while
    // `dispatch_script_event` holds the active mutable borrow via
    // `NativeContext`.  Instead, the matching `.remove` below
    // runs on the normal-return path.
    //
    // Earlier R22/R24 comments described the
    // `gc/collect.rs:558` sweep-tail block as defensive cleanup
    // if a Rust panic skipped the `.remove`, but that framing was
    // incorrect: `dispatched_events` is now itself a GC root
    // (`gc/roots.rs:215`), so a leaked id keeps its underlying
    // `Event` marked, the sweep-tail `retain(bit_get(...))` keeps
    // the entry, and the leak is permanent.  In practice the leak
    // is unreachable — listener-thrown JS exceptions go through
    // the spec §2.10 "report the exception" path (no Rust
    // unwind), and VM-level failures return `Err(VmError)`
    // instead of panicking — so the insert/remove pair always
    // pairs up.  Do not rely on the sweep-tail to recover from a
    // missed `.remove`; if a future change introduces a real
    // panic path here, refactor around `RefCell<HashSet>` /
    // `catch_unwind` first.
    ctx.vm.dispatched_events.insert(event_id);

    let timestamp_ms = ctx.vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    // Core-9 slot order per `event_shapes.rs::CORE_KEY_COUNT`:
    // type / bubbles / cancelable / eventPhase / target /
    // currentTarget / timeStamp / composed / isTrusted.
    // `defaultPrevented` is NOT a core-9 slot — it lives on
    // `ObjectKind::Event.default_prevented` and is exposed via
    // a prototype accessor, not as an own property.
    let slots: Vec<PropertyValue> = vec![
        PropertyValue::Data(JsValue::String(type_sid)), // type
        PropertyValue::Data(JsValue::Boolean(bubbles)), // bubbles
        PropertyValue::Data(JsValue::Boolean(cancelable)), // cancelable
        PropertyValue::Data(JsValue::Number(0.0)),      // eventPhase
        PropertyValue::Data(JsValue::Object(target_wrapper)), // target
        PropertyValue::Data(JsValue::Object(target_wrapper)), // currentTarget
        PropertyValue::Data(JsValue::Number(timestamp_ms)), // timeStamp
        PropertyValue::Data(JsValue::Boolean(false)),   // composed
        PropertyValue::Data(JsValue::Boolean(true)), // isTrusted (UA-fired synthetic events: reset / invalid)
    ];
    ctx.vm
        .define_with_precomputed_shape(event_id, core_shape, slots);

    let result = dispatch_script_event(ctx, event_id, target_entity);
    ctx.vm.dispatched_events.remove(&event_id);

    // `dispatch_script_event` returns `Ok(!default_prevented)` so
    // `Ok(false)` means the dispatch was cancelled.  `Err` is only
    // returned for VM-level failures (handler-loop infrastructure
    // errors, not listener-thrown JS exceptions — those go through
    // the report-an-exception path and never bubble to `Ok`/`Err`
    // here).  Propagate the VM-level `Err` rather than collapsing
    // it to `Ok(false)` so the caller (`form.reset()` etc.) does
    // not proceed under a hidden failure.
    result.map(|not_default_prevented| !not_default_prevented)
}

/// Dispatch a `toggle` ToggleEvent at `target_entity`, mirroring the
/// hand-rolled allocation pattern of [`dispatch_simple_event`] but
/// extending the slot Vec with the two ToggleEvent-specific payload
/// values (`newState`, `oldState`).
///
/// Used by the `<details>.open` setter (HTML §4.11.1.5) — both for
/// the self-fire on state change and for each sibling closed by
/// multi-disclosure exclusion.
///
/// Per spec the `toggle` event is `bubbles=false`, `cancelable=false`.
/// `is_trusted=true` since this is a UA-fired event (the setter is
/// the spec-mandated dispatch site, not a script-side `dispatchEvent`).
///
/// Returns `Ok(true)` when the event was cancelled (bubbles=false +
/// cancelable=false, so always returns `Ok(false)` in practice — but
/// the same contract is preserved for caller symmetry).  `Err` is
/// only returned for VM-level failures.
///
/// No `EventPayload::Toggle` variant is added to elidex-plugin — the
/// dispatch path here is fully VM-side, so the engine-indep
/// `EventPayload` enum stays untouched.
///
/// Parameter order is `(old_state, new_state)` to read naturally at
/// call sites ("closed → open" / "open → closed").  The internal slot
/// order is `(newState, oldState)` per Chrome DevTools enumeration
/// and the `toggle_event` shape transition.  Bindings below mirror
/// the SIGNATURE order on the lines that ToString-intern the values
/// so a reader following the parameter list down the function body
/// doesn't transpose them; the slot-population order is then made
/// explicit at the slot-Vec construction site.
pub(super) fn dispatch_toggle_event(
    ctx: &mut NativeContext<'_>,
    target_entity: elidex_ecs::Entity,
    old_state: &str,
    new_state: &str,
) -> Result<bool, VmError> {
    use super::super::value::PropertyValue;

    let toggle_type_sid = ctx.vm.well_known.toggle;
    // ToggleEvent.prototype is the brand for UA-fired toggles —
    // matches the §C-7 UA-brand fix surface.  Falls back to
    // `Event.prototype` if (somehow) the registration order changed
    // and `toggle_event_prototype` is `None`.
    let toggle_proto = ctx.vm.toggle_event_prototype.or(ctx.vm.event_prototype);
    let target_wrapper = ctx.vm.create_element_wrapper(target_entity);
    let toggle_shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .toggle_event;

    let event_id = ctx.vm.alloc_object(super::super::value::Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: false,
            passive: false,
            type_sid: toggle_type_sid,
            bubbles: false,
            composed: false,
            composed_path: None,
        },
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: toggle_proto,
        extensible: true,
    });
    // Same GC-rooting strategy as `dispatch_simple_event`: insert into
    // `dispatched_events` so the walker treats `event_id` as a strong
    // root for the duration of the dispatch + remove on the
    // normal-return path.
    ctx.vm.dispatched_events.insert(event_id);

    let timestamp_ms = ctx.vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    // Intern order matches the function signature
    // (`old_state, new_state`) — slot-population order below
    // independently rearranges to `(newState, oldState)` per shape.
    let old_state_sid = ctx.vm.strings.intern(old_state);
    let new_state_sid = ctx.vm.strings.intern(new_state);
    // Slot order: 9 core values then 2 toggle-payload values.
    // Toggle slots match the `toggle_event` shape transition:
    // newState then oldState (matches Chrome DevTools enumeration).
    let slots: Vec<PropertyValue> = vec![
        PropertyValue::Data(JsValue::String(toggle_type_sid)), // type
        PropertyValue::Data(JsValue::Boolean(false)),          // bubbles
        PropertyValue::Data(JsValue::Boolean(false)),          // cancelable
        PropertyValue::Data(JsValue::Number(0.0)),             // eventPhase
        PropertyValue::Data(JsValue::Object(target_wrapper)),  // target
        PropertyValue::Data(JsValue::Object(target_wrapper)),  // currentTarget
        PropertyValue::Data(JsValue::Number(timestamp_ms)),    // timeStamp
        PropertyValue::Data(JsValue::Boolean(false)),          // composed
        PropertyValue::Data(JsValue::Boolean(true)),           // isTrusted (UA-fired)
        PropertyValue::Data(JsValue::String(new_state_sid)),   // newState
        PropertyValue::Data(JsValue::String(old_state_sid)),   // oldState
    ];
    ctx.vm
        .define_with_precomputed_shape(event_id, toggle_shape, slots);

    let result = dispatch_script_event(ctx, event_id, target_entity);
    ctx.vm.dispatched_events.remove(&event_id);

    result.map(|not_default_prevented| !not_default_prevented)
}
