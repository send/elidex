//! VM-`EventTarget` dispatch (WHATWG DOM §2.9) for the non-entity
//! `EventTarget`s — `AbortSignal` / `IDBRequest` / `IDBTransaction` /
//! `IDBDatabase` / `WebSocket` / `EventSource` / `FileReader`.  The
//! get-the-parent provider + plan builder for the
//! `VmObject` half of the dispatch core; the per-listener invocation loop
//! is the SAME shared inner invoke the Node path uses
//! ([`super::event_target_dispatch::invoke_listeners_shared`]).
//!
//! Split from [`super::event_target_dispatch`] to keep both files under the
//! repo's ~1000-line convention.  A `VmObject` has no DOM-tree / shadow
//! presence, so there is no retarget / element-wrapper materialization
//! here: the propagation path is the §2.7 get-the-parent chain
//! ([`vm_event_parent`]) — flat (at-target only) for `AbortSignal` /
//! `WebSocket` / `EventSource` / `FileReader`, the
//! `IDBRequest → IDBTransaction → IDBDatabase` chain for IndexedDB —
//! `currentTarget` is the VM object itself, and `composedPath()`
//! materialization is deferred (§2.3 Q4: the slot stays unset so
//! `composedPath()` lazy-allocs `[]`).

#![cfg(feature = "engine")]

use elidex_plugin::EventPhase;
use elidex_script_session::event_dispatch::ListenerPlanEntry;
use elidex_script_session::ListenerKind;

use super::super::shape::ShapeId;
use super::super::value::{
    CallMode, JsValue, NativeContext, ObjectId, ObjectKind, PropertyValue, StringId, VmError,
};
use super::super::VmInner;
use super::dispatch_target::DispatchTarget;
use super::event_target_dispatch::invoke_listeners_shared;
use super::events::{
    set_event_slot_raw, EventInit, EVENT_SLOT_CURRENT_TARGET, EVENT_SLOT_EVENT_PHASE,
    EVENT_SLOT_TARGET,
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

/// Whether a listener (normal or event-handler) that WOULD actually run for
/// an `event_type` dispatch is registered anywhere on `target_id`'s
/// get-the-parent path.  A faithful predicate for the firing set of
/// [`build_vm_dispatch_plan`] + [`invoke_listeners_shared`], so a UA-fire can
/// skip allocating an event object for a target where nothing would fire
/// (W3C IDB fires `success` on every request; most are unobserved).  An
/// entry counts iff BOTH:
///
/// - it would run in this event's phase — the at-target node fires any
///   matching listener; an ancestor fires its **capture** listeners always
///   (the capture phase runs regardless of `event.bubbles`) and its bubble
///   listeners only when `bubbles`.  So the ancestor walk must NOT stop at
///   `!bubbles` — a non-bubbling event (`success` / `complete` /
///   `versionchange`) still runs the capture phase; and
/// - its callable is still live in `listener_store`.  A cleared
///   event-handler (`o.onX = null`) intentionally keeps its `EventListeners`
///   metadata entry (so a re-set reuses the original registration slot) but
///   retires the callable; that entry is planned by `build_vm_dispatch_plan`
///   yet skipped by `resolve_callable`, so counting it here would dispatch to
///   nobody and defeat the lazy-allocation fast-path.
pub(in crate::vm) fn vm_path_has_listener(
    vm: &VmInner,
    target_id: ObjectId,
    event_type: &str,
    bubbles: bool,
) -> bool {
    let host = vm.host_data.as_deref();
    let mut cur = Some(target_id);
    let mut depth = 0;
    while let Some(id) = cur {
        let is_target = depth == 0;
        let fires = vm.vm_event_listeners.get(&id).is_some_and(|l| {
            l.iter_matching(event_type).any(|e| {
                (is_target || e.capture || bubbles)
                    && host.is_some_and(|h| h.get_listener(e.id).is_some())
            })
        });
        if fires {
            return true;
        }
        // Match `build_vm_dispatch_plan`'s chain cap exactly (≤
        // `VM_DISPATCH_MAX_DEPTH` nodes total, target + ancestors): advance
        // `depth` BEFORE the bound check so this walk visits the SAME node
        // set the plan does — not one extra ancestor.
        depth += 1;
        if depth >= VM_DISPATCH_MAX_DEPTH {
            break;
        }
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

/// The **single VmObject UA-fire seam** (WHATWG DOM §2.9): build a
/// trusted `Event` (plain or a typed subclass) and dispatch it at a
/// `VmObject` `target_id` through the shared
/// [`dispatch_vm_event`] walk, so `addEventListener` listeners +
/// `on<type>` handlers + capture/bubble all fire from one event object.
///
/// Generalizes the proven IDB seam
/// ([`super::indexeddb::dispatch::fire_idb_event_with_props`]) so
/// `AbortSignal` (plain), `WebSocket` (MessageEvent / CloseEvent /
/// plain) and `EventSource` (MessageEvent / plain) all share one
/// construction-and-dispatch path. (IDB keeps its own constructor this
/// PR per plan-memo DR-1a — its `oldVersion`/`newVersion` use `BUILTIN`
/// attrs, not the `WEBIDL_RO` this seam's precomputed shapes carry; the
/// converge is trailing cleanup.)
///
/// Steps:
/// - **Gate**: [`vm_path_has_listener`] — fire at an unobserved target
///   allocates nothing (WHATWG fire-an-event is unobservable with no
///   listeners), returning the not-fired outcome.
/// - **Build**: one [`VmInner::create_fresh_event_object`] call (the
///   shared event builder the IDB seam also uses). In call-mode with an
///   `Undefined` receiver it allocates a fresh `Event`, seeds the core-9
///   slots (`isTrusted = true` for a UA-fired event), then appends the
///   shape-ordered `payload_slots` (empty for a plain `Event`). `shape`
///   MUST be the matching `precomputed_event_shapes` entry: `core` for
///   plain, `message` / `close_event` for the subclasses. A non-`None`
///   `proto_override` reparents to the subclass prototype; `None` keeps
///   `Event.prototype`.
/// - **Dispatch**: brackets the event in `dispatched_events` (§2.9
///   step 1 dispatch flag + GC root for the walk window) and routes
///   through [`dispatch_vm_event`].
///
/// `create_fresh_event_object` seeds `target` / `currentTarget` `Null`;
/// `dispatch_vm_event` overwrites `target` and clears `currentTarget` at
/// finalize, so the post-walk state matches regardless of the seed.
///
/// # GC safety
/// `fire_vm_event` pins any Object-valued `payload_slots` entry (a
/// `MessageEvent`'s `data` Blob / `ports` Array) on the VM stack across
/// `create_fresh_event_object`'s internal alloc, then releases them once
/// the event holds the slots and is bracketed in `dispatched_events` — so
/// callers pass freshly-built payload objects without their own rooting
/// dance.
pub(super) fn fire_vm_event(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    type_sid: StringId,
    init: EventInit,
    shape: ShapeId,
    proto_override: Option<ObjectId>,
    payload_slots: Vec<PropertyValue>,
) -> Result<VmDispatchOutcome, VmError> {
    // No observer anywhere on the (flat or ancestor) path → allocate
    // nothing (matches `fire_idb_event_with_props`'s lazy-alloc gate).
    let type_str = ctx.vm.strings.get_utf8(type_sid);
    if !vm_path_has_listener(ctx.vm, target_id, &type_str, init.bubbles) {
        return Ok(VmDispatchOutcome {
            not_prevented: true,
            threw: false,
        });
    }

    fire_vm_event_unchecked(
        ctx,
        target_id,
        type_sid,
        init,
        shape,
        proto_override,
        payload_slots,
    )
}

/// The build-and-dispatch half of [`fire_vm_event`], WITHOUT the
/// `vm_path_has_listener` gate.  The caller MUST already have confirmed an
/// observer on `target_id` for `type_sid` — the lazy-alloc invariant is then
/// honored at that earlier point, letting a gated caller build an expensive
/// payload (a `MessageEvent`'s Blob / ArrayBuffer `data`) only once it knows
/// the event will fire.  [`fire_vm_message_event`] is that gated entry; every
/// other UA-fire caller goes through [`fire_vm_event`], which gates.
fn fire_vm_event_unchecked(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    type_sid: StringId,
    init: EventInit,
    shape: ShapeId,
    proto_override: Option<ObjectId>,
    payload_slots: Vec<PropertyValue>,
) -> Result<VmDispatchOutcome, VmError> {
    // Build + GC-root the event inside a `push_stack_scope`: pin every
    // Object-valued payload slot (Blob `data`, `ports` Array) on the VM
    // stack so `create_fresh_event_object`'s internal alloc can't GC them
    // out from under the not-yet-allocated event.  The scope drops once
    // the event holds the slots AND is bracketed in `dispatched_events` (a
    // real GC root) — after that the payload is reachable through the
    // rooted event, and releasing the temporary stack roots is safe.
    let event_id = {
        let mut frame = ctx.vm.push_stack_scope();
        for slot in &payload_slots {
            if let PropertyValue::Data(v @ JsValue::Object(_)) = slot {
                frame.stack.push(*v);
            }
        }
        // Build through the single shared event builder (the same path the
        // IDB seam `fire_idb_event_with_props` takes): call-mode +
        // `Undefined` receiver allocates a fresh `Event`, seeds the core-9
        // slots with `isTrusted = true`, and appends `payload_slots` in
        // shape order.  `target` / `currentTarget` are seeded `Null` here;
        // `dispatch_vm_event` overwrites `target` + clears `currentTarget`
        // at finalize, so the post-walk state matches.
        let event_id = frame.create_fresh_event_object(
            JsValue::Undefined,
            type_sid,
            init,
            shape,
            payload_slots,
            true,
            CallMode::Call,
        );
        // Reparent to the subclass prototype (`MessageEvent` / `CloseEvent`)
        // so `e instanceof MessageEvent` holds; `None` keeps `Event.prototype`.
        if let Some(proto) = proto_override {
            frame.get_object_mut(event_id).prototype = Some(proto);
        }
        // §2.9 step 1 dispatch flag + GC root for the dispatch window.
        frame.dispatched_events.insert(event_id);
        event_id
    };

    let result = dispatch_vm_event(ctx, event_id, target_id);
    ctx.vm.dispatched_events.remove(&event_id);

    result
}

/// UA-fire a `MessageEvent(type, {data, origin, lastEventId})` at a
/// `VmObject` target — the shared WS-receive / SSE-dispatch helper (WHATWG
/// HTML §9.1 `MessageEvent`; WebSockets §4 / HTML §9.2.6).  Gates on the raw
/// `event_type` (`&str`) FIRST, so an unobserved target interns NOTHING and
/// allocates nothing: not the server-controlled `event_type` / `last_event_id`
/// (StringPool entries are permanent, so a permanent leak otherwise), not the
/// `data` payload (`build_data` — e.g. the binary caller's Blob / ArrayBuffer
/// — runs only past the gate), not the `ports` Array, not the event.
/// `source` is `null` and `ports` a fresh empty Array (no MessagePort transfer
/// on these surfaces).  Returns `Ok(true)` when the event was cancelled.
pub(super) fn fire_vm_message_event(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    event_type: &str,
    build_data: impl FnOnce(&mut NativeContext<'_>) -> JsValue,
    origin_sid: StringId,
    last_event_id: &str,
) -> Result<bool, VmError> {
    // Lazy gate on the raw `event_type` (`&str`) up-front — MessageEvents are
    // non-bubbling, and no StringPool intern is needed to decide whether to
    // fire.  An unobserved target returns here having interned / allocated
    // nothing; `fire_vm_event_unchecked` then avoids a redundant second walk.
    if !vm_path_has_listener(ctx.vm, target_id, event_type, false) {
        return Ok(false);
    }
    // Past the gate: intern the server-controlled strings now (a leak only if
    // done before the gate — see the doc).
    let type_sid = ctx.vm.strings.intern(event_type);
    let last_event_id_sid = if last_event_id.is_empty() {
        ctx.vm.well_known.empty
    } else {
        ctx.vm.strings.intern(last_event_id)
    };
    let shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .message;
    let proto = ctx.vm.message_event_prototype;
    // Build `data` now that an observer is confirmed: a cheap interned string
    // for text / SSE, or a fresh Blob / ArrayBuffer for binary.  It may have
    // no other live root, so pin it across the `ports` Array allocation;
    // `fire_vm_event_unchecked` then re-roots both Object payload slots across
    // its own event alloc (no GC-triggering work runs in between).
    let data = build_data(ctx);
    let ports_arr = {
        let mut frame = ctx.vm.push_stack_scope();
        if matches!(data, JsValue::Object(_)) {
            frame.stack.push(data);
        }
        frame.create_array_object(Vec::new())
    };
    let payload = vec![
        PropertyValue::Data(data),
        PropertyValue::Data(JsValue::String(origin_sid)),
        PropertyValue::Data(JsValue::String(last_event_id_sid)),
        PropertyValue::Data(JsValue::Null), // source
        PropertyValue::Data(JsValue::Object(ports_arr)),
    ];
    let init = EventInit {
        bubbles: false,
        cancelable: false,
        composed: false,
    };
    fire_vm_event_unchecked(ctx, target_id, type_sid, init, shape, proto, payload)
        .map(|o| !o.not_prevented)
}

/// UA-fire a `CloseEvent("close", {code, reason, wasClean})` at a `VmObject`
/// target (WHATWG WebSockets §6).  Gates FIRST, so an unobserved socket never
/// interns the server-controlled `reason` (a permanent StringPool entry) nor
/// builds the event.
pub(super) fn fire_vm_close_event(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    type_sid: StringId,
    code: u16,
    reason: &str,
    was_clean: bool,
) -> Result<bool, VmError> {
    // `close` is a fixed (non-server-controlled) type, but `reason` is
    // server-controlled and permanently interned — gate up-front so an
    // unobserved close defers (and thus skips) the `reason` intern.
    let type_str = ctx.vm.strings.get_utf8(type_sid);
    if !vm_path_has_listener(ctx.vm, target_id, &type_str, false) {
        return Ok(false);
    }
    let reason_sid = ctx.vm.strings.intern(reason);
    let shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .close_event;
    let proto = ctx.vm.close_event_prototype;
    let payload = vec![
        PropertyValue::Data(JsValue::Number(f64::from(code))),
        PropertyValue::Data(JsValue::String(reason_sid)),
        PropertyValue::Data(JsValue::Boolean(was_clean)),
    ];
    let init = EventInit {
        bubbles: false,
        cancelable: false,
        composed: false,
    };
    fire_vm_event_unchecked(ctx, target_id, type_sid, init, shape, proto, payload)
        .map(|o| !o.not_prevented)
}

/// UA-fire a `ProgressEvent(type, {lengthComputable, loaded, total})` at a
/// `VmObject` target (WHATWG XHR §5 `ProgressEvent`; W3C File API §6.4 — the
/// `FileReader` load-progress events `loadstart`/`progress`/`load`/`loadend`/
/// `abort`/`error`).  All such events are non-bubbling AND non-cancelable
/// (File API §6.4: "e.bubbles must be false … e.cancelable must be false").
/// Gates on the observer up-front — like [`fire_vm_message_event`] /
/// [`fire_vm_close_event`] — so an unobserved target skips the 3-number
/// `payload` Vec allocation (the gate still materializes the event type as a
/// transient `String` via `get_utf8`, as every `type_sid`-keyed gated helper
/// does — the saving is the payload, not literally zero allocation).  The
/// payload has no server-controlled string and no `Blob`/`ArrayBuffer` `data`,
/// so there is nothing to *intern* past the gate (unlike those two helpers);
/// the up-front gate keeps the lazy-alloc invariant uniform across every
/// payload-bearing UA-fire helper.  Always returns `Ok(false)`: these events
/// are non-cancelable (above), so default-prevention can't occur — the
/// `Result<bool>` shape just mirrors the sibling helpers (the sole caller,
/// `fire_fr_progress`, discards it).
pub(super) fn fire_vm_progress_event(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    type_sid: StringId,
    length_computable: bool,
    loaded: f64,
    total: f64,
) -> Result<bool, VmError> {
    // Lazy gate up-front (ProgressEvents are non-bubbling) so an unobserved
    // `FileReader` skips the `payload` Vec allocation below; the gate's
    // `get_utf8` type `String` is the only remaining transient.
    // `fire_vm_event_unchecked` then runs the single observer walk for
    // confirmed-observed targets.
    let type_str = ctx.vm.strings.get_utf8(type_sid);
    if !vm_path_has_listener(ctx.vm, target_id, &type_str, false) {
        return Ok(false);
    }
    let shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .progress_event;
    let proto = ctx.vm.progress_event_prototype;
    let payload = vec![
        PropertyValue::Data(JsValue::Boolean(length_computable)),
        PropertyValue::Data(JsValue::Number(loaded)),
        PropertyValue::Data(JsValue::Number(total)),
    ];
    let init = EventInit {
        bubbles: false,
        cancelable: false,
        composed: false,
    };
    fire_vm_event_unchecked(ctx, target_id, type_sid, init, shape, proto, payload)
        .map(|o| !o.not_prevented)
}

/// UA-fire a plain `Event` of `type_sid` at a `VmObject` target — the
/// no-payload wrapper over [`fire_vm_event`] (the `VmObject` sibling of
/// [`super::event_target_dispatch::dispatch_simple_event`]).  Returns
/// `Ok(true)` when the event was cancelled (default-prevented).
pub(super) fn dispatch_vm_simple_event(
    ctx: &mut NativeContext<'_>,
    target_id: ObjectId,
    type_sid: StringId,
    bubbles: bool,
    cancelable: bool,
) -> Result<bool, VmError> {
    let core_shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;
    let init = EventInit {
        bubbles,
        cancelable,
        composed: false,
    };
    fire_vm_event(ctx, target_id, type_sid, init, core_shape, None, Vec::new())
        .map(|outcome| !outcome.not_prevented)
}
