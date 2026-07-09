//! History-step UA event delivery (WHATWG HTML §7.4.6.2 "update document for
//! history step application").
//!
//! A same-document history-step application — a synchronous **fragment
//! navigation** (S5-5b) or a **traversal** (S5-5c) — fires `popstate` +
//! `hashchange` at the `Window`. The **decision** (which events, with what
//! `history.state` / old-and-new URLs) is made engine-independently by the shell
//! and arrives as an [`elidex_script_session::HistoryStepEvents`]; this module
//! is the VM's marshal-only **reconstruct + fire** half (Layering mandate: no
//! classification / decision logic here).
//!
//! Split from [`super::events_extras`] (the non-UIEvent *constructor* home): the
//! constructors build a JS-facing `new PopStateEvent(...)` / `new
//! HashChangeEvent(...)`; this file is the distinct concern of *UA-firing* a
//! trusted such event at the Window and enqueuing the hashchange task.
//!
//! ## Timing (load-bearing — §7.4.6.2)
//!
//! - **popstate = SYNCHRONOUS** (step 6.4.3 "fire an event") with the
//!   reconstructed `history.state`.
//! - **hashchange = ENQUEUED** (step 6.4.5 "queue a global task on the DOM
//!   manipulation task source"), so popstate is observed **strictly before**
//!   hashchange. It rides the shared same-window task queue
//!   ([`PendingTask::HashChange`]).
//!
//! Both are plain "fire an event" (§7.4.6.2), i.e. non-bubbling, non-cancelable.

#![cfg(feature = "engine")]

use elidex_ecs::Entity;

use super::super::host_data::HostData;
use super::super::shape::ShapeId;
use super::super::value::{CallMode, JsValue, NativeContext, ObjectId, PropertyValue, StringId};
use super::super::VmInner;
use super::event_target_dispatch::dispatch_script_event;
use super::events::EventInit;
use super::pending_tasks::PendingTask;

/// A history-step UA event never bubbles and is never cancelable (§7.4.6.2 uses
/// the bare "fire an event", whose defaults are all `false`).
const HISTORY_EVENT_INIT: EventInit = EventInit {
    bubbles: false,
    cancelable: false,
    composed: false,
};

impl VmInner {
    /// Deliver a same-document history-step's popstate + hashchange (WHATWG HTML
    /// §7.4.6.2). Fires popstate **synchronously** (`popstate_state.is_some()`)
    /// then enqueues hashchange (`hashchange.is_some()`) as a task, so popstate
    /// is observed strictly before hashchange. A no-op when the VM is not bound
    /// to a browsing context (no Window entity / DOM to dispatch at).
    ///
    /// Marshal-only: the *decision* already arrived engine-independently; this
    /// reconstructs the `JsValue` state + builds/dispatches the events.
    // `Option<Option<_>>` is intentional — it mirrors the engine-independent
    // `HistoryStepEvents::popstate_state` contract and genuinely distinguishes
    // all three states clippy's help calls out: `None` (do not fire popstate),
    // `Some(None)` (fire with `state = null`, 5b), `Some(Some(bytes))` (fire with
    // `StructuredDeserialize(bytes)`, 5c).
    #[allow(clippy::option_option)]
    pub(crate) fn deliver_history_step_events(
        &mut self,
        popstate_state: Option<Option<Vec<u8>>>,
        hashchange: Option<(String, String)>,
    ) {
        // A history-step fires at the Window, which needs a bound browsing
        // context (window entity + DOM). Mirror the `deliver_media_query_changes`
        // `is_bound` gate; a bound VM always has a window entity (allocated at
        // `bind`), but resolve defensively.
        let Some(window_entity) = self
            .host_data
            .as_deref()
            .filter(|h| h.is_bound())
            .and_then(HostData::window_entity)
        else {
            return;
        };

        // 1. Restore history.state, then fire popstate — SYNCHRONOUS (§7.4.6.2
        //    step 6.3 "restore the history object's state", then step 6.4.3
        //    "fire an event").
        if let Some(state_opt) = popstate_state {
            let state = reconstruct_history_state(self, state_opt);
            // Step 6.3: restore `history.state` to the reconstructed value
            // BEFORE firing popstate — a fragment nav resets it to null
            // (§7.4.2.3.3 step 11.1 "Set history's state to null"), so a
            // synchronous popstate handler reads a `history.state` consistent
            // with the event's `state`. Without this, `history.state` keeps the
            // stale pre-nav `pushState` value while `popstate.state` is null.
            // (`JsValue` is `Copy`, so `state` is reused for the fire below.)
            self.navigation.current_state = state;
            self.fire_popstate(window_entity, state);
            // Clean up after the synchronous dispatch (perform a microtask
            // checkpoint) so a popstate listener's microtasks settle strictly
            // before the hashchange task runs.
            self.drain_microtasks();
        }

        // 2. hashchange — ENQUEUED (§7.4.6.2 step 6.4.5 "queue a global task on
        //    the DOM manipulation task source"). Enqueue then settle it within
        //    this turn, AFTER the synchronous popstate. `drain_tasks` runs a
        //    microtask checkpoint after the task and is reentrancy-guarded +
        //    idempotent, so a later shell task-drain is a safe no-op.
        if let Some((old_url, new_url)) = hashchange {
            let old_url_sid = self.strings.intern(&old_url);
            let new_url_sid = self.strings.intern(&new_url);
            self.queue_task(PendingTask::HashChange {
                old_url_sid,
                new_url_sid,
            });
            self.drain_tasks();
        }
    }

    /// Build + synchronously dispatch a trusted `PopStateEvent` (with `state`
    /// initialized to `state`) at the Window (§7.4.6.2 step 6.4.3).
    fn fire_popstate(&mut self, window_entity: Entity, state: JsValue) {
        // Read the (immutable) fire inputs into locals first, so the subsequent
        // `&mut self` fire borrow does not overlap the field reads.
        let type_sid = self.well_known.popstate_event_type;
        let shape = self
            .precomputed_event_shapes
            .as_ref()
            .expect("precomputed_event_shapes built during VM init")
            .pop_state_event;
        let proto = self.pop_state_event_prototype;
        self.fire_window_event(
            window_entity,
            type_sid,
            shape,
            proto,
            vec![PropertyValue::Data(state)],
        );
    }

    /// Build a trusted subclass `Event` (via the shared
    /// [`VmInner::create_fresh_event_object`]) and dispatch it at the Window
    /// entity through [`dispatch_script_event`] — the entity/Node analogue of
    /// [`super::event_target_dispatch_vm::fire_vm_event`] (which targets
    /// `VmObject`s). `proto_override` reparents to the subclass prototype
    /// (`PopStateEvent` / `HashChangeEvent`); `None` keeps `Event.prototype`.
    fn fire_window_event(
        &mut self,
        window_entity: Entity,
        type_sid: StringId,
        shape: ShapeId,
        proto_override: Option<ObjectId>,
        payload_slots: Vec<PropertyValue>,
    ) {
        // Build + GC-root the event inside a `push_stack_scope`: pin any
        // Object-valued payload slot (a 5c `history.state` object; 5b's null +
        // hashchange strings have none) across `create_fresh_event_object`'s
        // internal alloc, then release once the event holds the slots AND is
        // bracketed in `dispatched_events` (a real GC root). Mirrors
        // `fire_vm_event_unchecked`.
        let event_id = {
            let mut frame = self.push_stack_scope();
            for slot in &payload_slots {
                if let PropertyValue::Data(v @ JsValue::Object(_)) = slot {
                    frame.stack.push(*v);
                }
            }
            // Call-mode + `Undefined` receiver allocates a fresh trusted `Event`
            // (`is_trusted = true`) and appends `payload_slots` in `shape` order.
            let id = frame.create_fresh_event_object(
                JsValue::Undefined,
                type_sid,
                HISTORY_EVENT_INIT,
                shape,
                payload_slots,
                true,
                CallMode::Call,
            );
            if let Some(proto) = proto_override {
                frame.get_object_mut(id).prototype = Some(proto);
            }
            // §2.9 step 1 dispatch flag + GC root for the dispatch window.
            frame.dispatched_events.insert(id);
            id
        };

        // `dispatch_script_event` seeds `target`/`currentTarget` to the Window
        // wrapper and clears them at finalize, so the `Null` seed is correct.
        let mut ctx = NativeContext::new_call(self);
        let _ = dispatch_script_event(&mut ctx, event_id, window_entity);
        self.dispatched_events.remove(&event_id);
    }
}

/// Reconstruct the `history.state` JS value a popstate fires with (the *restore
/// the history object state* step, WHATWG HTML §7.4.6.2 step 6.3 → its step 2
/// `StructuredDeserialize`).
///
/// - `None` = the entry carries **no** state — a **fragment navigation** (§7.4.2.3.3
///   step 11.1 "Set history's state to null"), a plain-navigation entry, or any
///   entry a boa-`None` pushState produced ⇒ `history.state = null`.
/// - `Some(bytes)` = a **traversal** to a pushState'd entry: the restored state,
///   `StructuredDeserialize(bytes)` (a decode failure → `null` per the restore
///   step's "If this throws an exception, catch it and let state be null").
fn reconstruct_history_state(vm: &mut VmInner, state: Option<Vec<u8>>) -> JsValue {
    match state {
        None => JsValue::Null,
        Some(bytes) => super::structured_serialize::structured_deserialize(vm, &bytes),
    }
}

/// Drain step for [`PendingTask::HashChange`] (§7.4.6.2 step 6.4.5): build a
/// trusted `HashChangeEvent { oldURL, newURL }` and fire it at the Window. Runs
/// on the same-window task queue, so it is observed strictly after the
/// synchronously-fired popstate. Re-resolves the Window entity at drain time
/// (mirroring [`super::pending_tasks`]'s `dispatch_post_message`); a no-op if the
/// VM lost its binding between enqueue and drain.
pub(super) fn dispatch_hashchange_task(
    vm: &mut VmInner,
    old_url_sid: StringId,
    new_url_sid: StringId,
) {
    let Some(window_entity) = vm
        .host_data
        .as_deref()
        .filter(|h| h.is_bound())
        .and_then(HostData::window_entity)
    else {
        return;
    };
    let type_sid = vm.well_known.hashchange_event_type;
    let shape = vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .hash_change;
    let proto = vm.hash_change_event_prototype;
    // Slot order matches `event_shapes.rs::hash_change` + the constructor:
    // `oldURL`, then `newURL`.
    let payload = vec![
        PropertyValue::Data(JsValue::String(old_url_sid)),
        PropertyValue::Data(JsValue::String(new_url_sid)),
    ];
    vm.fire_window_event(window_entity, type_sid, shape, proto, payload);
}
