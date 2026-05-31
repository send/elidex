//! VM-`EventTarget` dispatch (WHATWG DOM §2.9) for the non-entity
//! `EventTarget`s — `AbortSignal` / `IDBRequest` / `IDBTransaction` /
//! `IDBDatabase`.  The get-the-parent provider + plan builder for the
//! `VmObject` half of the dispatch core; the per-listener invocation loop
//! is the SAME shared inner invoke the Node path uses
//! ([`super::event_target_dispatch::invoke_listeners_shared`]).
//!
//! Split from [`super::event_target_dispatch`] to keep both files under the
//! repo's ~1000-line convention.  A `VmObject` has no DOM-tree / shadow
//! presence, so there is no retarget / element-wrapper materialization
//! here: the propagation path is the §2.7 get-the-parent chain
//! ([`vm_event_parent`]) — flat (at-target only) for `AbortSignal`, the
//! `IDBRequest → IDBTransaction → IDBDatabase` chain for IndexedDB —
//! `currentTarget` is the VM object itself, and `composedPath()`
//! materialization is deferred (§2.3 Q4: the slot stays unset so
//! `composedPath()` lazy-allocs `[]`).

#![cfg(feature = "engine")]

use elidex_plugin::EventPhase;
use elidex_script_session::event_dispatch::ListenerPlanEntry;
use elidex_script_session::ListenerKind;

use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, PropertyValue, StringId,
    VmError,
};
use super::super::VmInner;
use super::dispatch_target::DispatchTarget;
use super::event_target_dispatch::invoke_listeners_shared;
use super::events::{
    set_event_slot_raw, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_EVENT_PHASE, EVENT_SLOT_TARGET,
};

/// Bound on the VM get-the-parent walk (the IDB chain is depth ≤ 2;
/// `AbortSignal` is flat).  Guards against an accidentally cyclic
/// ancestor relation in a side-store.
const VM_DISPATCH_MAX_DEPTH: usize = 16;

/// WHATWG DOM §2.7 **get the parent** for a `VmObject` EventTarget — the
/// propagation chain the dispatch plan bubbles along:
///
/// - `IDBRequest` → its owning `IDBTransaction` (W3C IDB §5.9/§5.10
///   bubbling so a request `error` reaches `tx.onerror` / `db.onerror`).
/// - `IDBTransaction` → its owning `IDBDatabase`.
/// - everything else (`IDBDatabase`, `AbortSignal`, …) → `None` (the
///   §2.7 default: no parent → flat, at-target only).
fn vm_event_parent(vm: &VmInner, id: ObjectId) -> Option<ObjectId> {
    match vm.get_object(id).kind {
        ObjectKind::IdbRequest => vm.idb_request_states.get(&id).and_then(|s| s.transaction),
        ObjectKind::IdbTransaction => vm.idb_transaction_states.get(&id).and_then(|s| s.db),
        _ => None,
    }
}

/// Whether any listener (normal or event-handler) is registered for
/// `event_type` anywhere on `target_id`'s get-the-parent path — the
/// at-target node always, plus the bubbling ancestors when `bubbles`.
/// Lets a UA-fire skip allocating an event object for an unobserved
/// target (W3C IDB fires `success` on every request; most are unobserved).
pub(super) fn vm_path_has_listener(
    vm: &VmInner,
    target_id: ObjectId,
    event_type: &str,
    bubbles: bool,
) -> bool {
    let mut cur = Some(target_id);
    let mut depth = 0;
    while let Some(id) = cur {
        if vm
            .vm_event_listeners
            .get(&id)
            .is_some_and(|l| l.iter_matching(event_type).next().is_some())
        {
            return true;
        }
        if !bubbles || depth >= VM_DISPATCH_MAX_DEPTH {
            break;
        }
        depth += 1;
        cur = vm_event_parent(vm, id);
    }
    false
}

/// `(node, listeners)` buckets for one VM dispatch phase.
type VmPhase = Vec<(ObjectId, Vec<ListenerPlanEntry>)>;

/// Pre-collected VM dispatch plan (the `VmObject` analogue of the
/// session crate's `Entity`-typed `DispatchPlan`).
struct VmDispatchPlan {
    capture: VmPhase,
    at_target: Option<(ObjectId, Vec<ListenerPlanEntry>)>,
    bubble: VmPhase,
}

/// Collect listener entries matching (`event_type`, `capture`) from a VM
/// target's `vm_event_listeners` home.  `capture == None` → both phases
/// (at-target).  A missing home (zero listeners) yields an empty Vec.
fn vm_collect_listeners(
    vm: &VmInner,
    id: ObjectId,
    event_type: &str,
    capture: Option<bool>,
) -> Vec<ListenerPlanEntry> {
    vm.vm_event_listeners
        .get(&id)
        .map(|listeners| {
            listeners
                .iter_matching(event_type)
                .filter(|e| capture.is_none_or(|cap| e.capture == cap))
                .map(|e| ListenerPlanEntry {
                    id: e.id,
                    once: e.once,
                    passive: e.passive,
                    is_handler: matches!(e.kind, ListenerKind::EventHandler { .. }),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Build the §2.9 dispatch plan for a `VmObject` target: walk
/// [`vm_event_parent`] to the root, then bucket capture (root → target
/// exclusive), at-target (all listeners), and bubble (target exclusive →
/// root) listener entries.  The DOM-borrow-free analogue of
/// `build_dispatch_plan` — no shadow / retarget (no shadow tree exists
/// for a VM object).
fn build_vm_dispatch_plan(
    vm: &VmInner,
    target_id: ObjectId,
    event_type: &str,
    bubbles: bool,
) -> VmDispatchPlan {
    // Ancestor chain, target → root.
    let mut chain = vec![target_id];
    let mut cur = target_id;
    while let Some(parent) = vm_event_parent(vm, cur) {
        if chain.len() >= VM_DISPATCH_MAX_DEPTH {
            break;
        }
        chain.push(parent);
        cur = parent;
    }
    chain.reverse(); // root → target
    let target_idx = chain.len() - 1;

    let capture: VmPhase = chain[..target_idx]
        .iter()
        .map(|&id| (id, vm_collect_listeners(vm, id, event_type, Some(true))))
        .collect();
    let at_target = Some((
        chain[target_idx],
        vm_collect_listeners(vm, chain[target_idx], event_type, None),
    ));
    let bubble: VmPhase = if bubbles {
        chain[..target_idx]
            .iter()
            .rev()
            .map(|&id| (id, vm_collect_listeners(vm, id, event_type, Some(false))))
            .collect()
    } else {
        VmPhase::new()
    };

    VmDispatchPlan {
        capture,
        at_target,
        bubble,
    }
}

/// `stopPropagation()` / `stopImmediatePropagation()` halts the walk
/// between path nodes.
fn vm_propagation_halted(vm: &VmInner, event_id: ObjectId) -> bool {
    matches!(
        vm.get_object(event_id).kind,
        ObjectKind::Event {
            propagation_stopped: true,
            ..
        } | ObjectKind::Event {
            immediate_propagation_stopped: true,
            ..
        }
    )
}

/// Walk one VM dispatch phase: for each path node set `currentTarget`
/// (the VM object itself — no element wrapper) + `eventPhase`, then run
/// the shared §2.9 inner invoke over its listener entries.  Returns
/// whether any listener threw (OR-accumulated across nodes).
fn vm_walk_phase(
    ctx: &mut NativeContext<'_>,
    event_id: ObjectId,
    entries: &[(ObjectId, Vec<ListenerPlanEntry>)],
    phase: EventPhase,
) -> bool {
    let mut threw = false;
    for (node_id, listener_entries) in entries {
        if vm_propagation_halted(ctx.vm, event_id) {
            break;
        }
        set_event_slot_raw(
            ctx.vm,
            event_id,
            EVENT_SLOT_CURRENT_TARGET,
            JsValue::Object(*node_id),
        );
        set_event_slot_raw(
            ctx.vm,
            event_id,
            EVENT_SLOT_EVENT_PHASE,
            JsValue::Number(f64::from(phase as u8)),
        );
        threw |= invoke_listeners_shared(
            ctx,
            event_id,
            JsValue::Object(*node_id),
            DispatchTarget::VmObject(*node_id),
            listener_entries,
            // VM UA-fire: defer the microtask checkpoint to post-dispatch
            // (preserves the IDB transaction-active window / AbortSignal
            // one-shot timing).
            false,
        );
    }
    threw
}

/// Outcome of a `VmObject` dispatch.
pub(super) struct VmDispatchOutcome {
    /// `!default_prevented` (the `dispatchEvent` return value).
    pub(super) not_prevented: bool,
    /// Whether any listener threw an uncaught exception (W3C IDB
    /// §5.9/§5.10 step 8.2 aborts the transaction when true).
    pub(super) threw: bool,
}

/// Dispatch `event_id` on a `VmObject` target (WHATWG DOM §2.9), the
/// `VmObject` arm of `EventTarget.prototype.dispatchEvent` + the UA-fire
/// helpers.  Preconditions (caller-validated, mirroring
/// [`super::event_target_dispatch::dispatch_script_event`]): `event_id`
/// is an `ObjectKind::Event`, and `ctx.vm.dispatched_events` already has
/// `event_id` inserted (the §2.9 step 1 dispatch flag + GC root).
pub(super) fn dispatch_vm_event(
    ctx: &mut NativeContext<'_>,
    event_id: ObjectId,
    target_id: ObjectId,
) -> Result<VmDispatchOutcome, VmError> {
    let ObjectKind::Event {
        type_sid, bubbles, ..
    } = ctx.vm.get_object(event_id).kind
    else {
        unreachable!("dispatch_vm_event: receiver is not ObjectKind::Event")
    };
    let event_type = ctx.vm.strings.get_utf8(type_sid);

    let plan = build_vm_dispatch_plan(ctx.vm, target_id, &event_type, bubbles);

    // §2.9: `target` is set for the whole dispatch and stays observable
    // after — the VM object itself (no element wrapper).
    set_event_slot_raw(
        ctx.vm,
        event_id,
        EVENT_SLOT_TARGET,
        JsValue::Object(target_id),
    );

    let mut threw = false;
    // Phase 1: Capture (root → target, exclusive).
    threw |= vm_walk_phase(ctx, event_id, &plan.capture, EventPhase::Capturing);
    // Phase 2: At-target.
    if !vm_propagation_halted(ctx.vm, event_id) {
        if let Some(at_target) = plan.at_target.as_ref() {
            threw |= vm_walk_phase(
                ctx,
                event_id,
                std::slice::from_ref(at_target),
                EventPhase::AtTarget,
            );
        }
    }
    // Phase 3: Bubble (target → root, exclusive, reversed).
    if bubbles && !vm_propagation_halted(ctx.vm, event_id) {
        threw |= vm_walk_phase(ctx, event_id, &plan.bubble, EventPhase::Bubbling);
    }

    // Finalize (§2.9 steps 27-31): clear `currentTarget` + `eventPhase` +
    // the propagation flags so a captured event reads "not dispatching"
    // and a re-dispatch starts clean.  `target` stays set;
    // `default_prevented` (the canceled bit the caller inspects) is
    // preserved.
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
        ..
    } = &mut ctx.vm.get_object_mut(event_id).kind
    {
        *propagation_stopped = false;
        *immediate_propagation_stopped = false;
    }

    let prevented = matches!(
        ctx.vm.get_object(event_id).kind,
        ObjectKind::Event {
            default_prevented: true,
            ..
        }
    );
    Ok(VmDispatchOutcome {
        not_prevented: !prevented,
        threw,
    })
}

/// UA-fire a plain `Event` of `type_sid` at a `VmObject` target (the
/// `VmObject` sibling of
/// [`super::event_target_dispatch::dispatch_simple_event`]).  Allocates
/// the event, installs its core-9 own-data slots, brackets it in
/// `dispatched_events` for the dispatch window (GC root + §2.9 dispatch
/// flag), walks via [`dispatch_vm_event`], and unbrackets.  Returns
/// `Ok(true)` when the event was cancelled (default-prevented).
pub(super) fn dispatch_vm_simple_event(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    type_sid: StringId,
    bubbles: bool,
    cancelable: bool,
) -> Result<bool, VmError> {
    let event_proto = ctx.vm.event_prototype;
    let core_shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;

    let event_id = ctx.vm.alloc_object(Object {
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
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: event_proto,
        extensible: true,
    });

    // GC-root `event_id` immediately by inserting into `dispatched_events`
    // (a real GC root) — covers the slot install + any transitive alloc
    // inside `dispatch_vm_event` (listener-fired user code).  The matching
    // `.remove` runs on the normal-return path; see
    // `dispatch_simple_event` for the panic-safety rationale.
    ctx.vm.dispatched_events.insert(event_id);

    let timestamp_ms = ctx.vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    // Core-9 slot order: type / bubbles / cancelable / eventPhase /
    // target / currentTarget / timeStamp / composed / isTrusted.
    // `target` / `currentTarget` are the VM object itself (no wrapper).
    let slots: Vec<PropertyValue> = vec![
        PropertyValue::Data(JsValue::String(type_sid)),
        PropertyValue::Data(JsValue::Boolean(bubbles)),
        PropertyValue::Data(JsValue::Boolean(cancelable)),
        PropertyValue::Data(JsValue::Number(0.0)),
        PropertyValue::Data(JsValue::Object(target_id)),
        PropertyValue::Data(JsValue::Object(target_id)),
        PropertyValue::Data(JsValue::Number(timestamp_ms)),
        PropertyValue::Data(JsValue::Boolean(false)),
        PropertyValue::Data(JsValue::Boolean(true)), // isTrusted (UA-fired)
    ];
    ctx.vm
        .define_with_precomputed_shape(event_id, core_shape, slots);

    let result = dispatch_vm_event(ctx, event_id, target_id);
    ctx.vm.dispatched_events.remove(&event_id);

    result.map(|outcome| !outcome.not_prevented)
}
