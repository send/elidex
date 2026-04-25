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
    let (type_sid, bubbles, cancelable, composed) = match ctx.vm.get_object(event_id).kind {
        ObjectKind::Event {
            type_sid,
            bubbles,
            cancelable,
            composed,
            ..
        } => (type_sid, bubbles, cancelable, composed),
        _ => unreachable!("dispatch_script_event: receiver is not ObjectKind::Event"),
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
