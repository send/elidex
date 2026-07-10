//! IndexedDB UA event firing (W3C IDB §5.9 "fire a success event" / §5.10
//! "fire an error event" / §4.2 `IDBVersionChangeEvent`).
//!
//! Post `#11-eventtarget-dispatch-core` the `IDBRequest` / `IDBTransaction`
//! / `IDBDatabase` EventTargets are full members of the unified dispatch
//! core: their `addEventListener` listeners + `on<event>` handlers live in
//! the shared [`VmInner::vm_event_listeners`] home, and dispatch (capture /
//! at-target / bubble along the `IDBRequest → IDBTransaction → IDBDatabase`
//! get-the-parent chain) runs through
//! [`super::super::event_target_dispatch_vm::dispatch_vm_event`].  This
//! module is now just the **UA-fire seam**: it builds the event object
//! (with the §4.2 `IDBVersionChangeEvent` shape when needed) and adapts the
//! generic dispatch outcome to the [`FireResult`] the §5.9/§5.10
//! transaction-lifecycle steps consume.

#![cfg(feature = "engine")]

use super::super::super::host_data::HostData;
use super::super::super::shape::PropertyAttrs;
use super::super::super::value::{
    CallMode, JsValue, NativeContext, ObjectId, PropertyKey, PropertyValue, StringId,
};
use super::super::super::VmInner;
use super::super::event_target_dispatch_vm::{dispatch_vm_event, vm_path_has_listener};
use super::super::events::EventInit;

/// Outcome of dispatching an IDB event, consumed by the §5.9 / §5.10
/// transaction lifecycle steps.
pub(crate) struct FireResult {
    /// A handler / listener threw (§5.9 step 8.2 / §5.10 step 8.2 →
    /// abort the transaction with an `"AbortError"`).
    pub(crate) threw: bool,
    /// `event.preventDefault()` was called during dispatch (§5.10 step 8.3
    /// canceled-flag check — when false the error aborts the transaction).
    pub(crate) canceled: bool,
}

/// Fire `event_type` at `target` (W3C IDB §5.9 / §5.10 dispatch step)
/// through the shared EventTarget core, returning whether a listener threw
/// and whether the default was prevented so the caller can run the
/// transaction lifecycle steps.  A `bubbles` event propagates along the
/// IDB get-the-parent chain (request → transaction → database).
pub(crate) fn fire_idb_event(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    cancelable: bool,
    bubbles: bool,
) -> FireResult {
    fire_idb_event_with_props(ctx, target, event_type, cancelable, bubbles, None, &[])
}

/// Fire an `IDBVersionChangeEvent` (§4.2) at `target` — a base `Event`
/// with own `oldVersion` / `newVersion` data properties + the
/// `IDBVersionChangeEvent.prototype`.  Used for `upgradeneeded` /
/// `versionchange` / `blocked`.  `new_version` is `null` for a
/// `deleteDatabase` versionchange.
pub(crate) fn fire_version_change_event(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    old_version: u64,
    new_version: Option<u64>,
) -> FireResult {
    #[allow(clippy::cast_precision_loss)]
    let new_v = new_version.map_or(JsValue::Null, |v| JsValue::Number(v as f64));
    let old_sid = ctx.vm.well_known.old_version;
    let new_sid = ctx.vm.well_known.new_version;
    #[allow(clippy::cast_precision_loss)]
    let props = [
        (old_sid, JsValue::Number(old_version as f64)),
        (new_sid, new_v),
    ];
    let proto = ctx.vm.idb_version_change_event_prototype;
    fire_idb_event_with_props(ctx, target, event_type, false, false, proto, &props)
}

impl VmInner {
    /// Deliver a cross-context version change to this VM (S5-6a, B21 —
    /// IndexedDB-3 §4.2 Event interfaces, dfn *fire a version change
    /// event*): fire `versionchange` at every OPEN `IDBDatabase` connection
    /// to `db_name`, the receive half of the wire whose emit half is the
    /// `indexedDB.open()` upgrade branch's cross-context request
    /// (`factory.rs`).  `new_version` is `None` for a database-deletion
    /// version change (`IDBVersionChangeEvent.newVersion` = null).
    ///
    /// Marshal-only: another context made the decision; this reuses the
    /// in-VM `IDBVersionChangeEvent` UA-fire seam
    /// ([`fire_version_change_event`]).  Fires only — per the spec (and boa
    /// parity), closing the connection is the page's `versionchange`
    /// handler's job, never automatic.  A no-op when no open connection to
    /// `db_name` exists, or when the VM is not bound to a browsing context
    /// (mirroring `deliver_history_step_events`' defensive gate — listener
    /// bodies may touch the DOM).
    pub(crate) fn deliver_idb_versionchange(
        &mut self,
        db_name: &str,
        old_version: u64,
        new_version: Option<u64>,
    ) {
        if !self.host_data.as_deref().is_some_and(HostData::is_bound) {
            return;
        }
        let targets: Vec<ObjectId> = self
            .idb_database_states
            .iter()
            .filter(|(_, s)| s.db_name == db_name && !s.closed)
            .map(|(id, _)| *id)
            .collect();
        if targets.is_empty() {
            return;
        }
        let versionchange_sid = self.well_known.versionchange;
        let mut ctx = NativeContext::new_call(self);
        for db_id in targets {
            // A throwing handler is contained by the dispatch core
            // (report-an-exception); the remaining connections still hear
            // their event.
            let _ = fire_version_change_event(
                &mut ctx,
                db_id,
                versionchange_sid,
                old_version,
                new_version,
            );
        }
    }
}

/// Shared UA-fire seam.  `proto_override` reparents the event to a subclass
/// prototype (e.g. `IDBVersionChangeEvent.prototype`); `extra_props`
/// installs own data properties before dispatch.  Builds the event lazily
/// — only when a node on the propagation path actually has a listener /
/// handler (a fire at an unobserved target allocates nothing) — brackets it
/// in `dispatched_events` (the §2.9 dispatch flag + GC root) for the walk,
/// and routes through the shared [`dispatch_vm_event`].
fn fire_idb_event_with_props(
    ctx: &mut NativeContext<'_>,
    target: ObjectId,
    event_type: StringId,
    cancelable: bool,
    bubbles: bool,
    proto_override: Option<ObjectId>,
    extra_props: &[(StringId, JsValue)],
) -> FireResult {
    // No observer anywhere on the path → no event object is allocated
    // (matches the node UA-fire only insofar as IDB fires `success` on
    // every request; this keeps an unobserved request allocation-free).
    let type_str = ctx.vm.strings.get_utf8(event_type);
    if !vm_path_has_listener(ctx.vm, target, &type_str, bubbles) {
        return FireResult {
            threw: false,
            canceled: false,
        };
    }

    let shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes missing — register_globals did not run")
        .core;
    let init = EventInit {
        bubbles,
        cancelable,
        composed: false,
    };
    let event_id = ctx.vm.create_fresh_event_object(
        JsValue::Undefined,
        event_type,
        init,
        shape,
        Vec::new(),
        true,
        CallMode::Call,
    );
    if let Some(proto) = proto_override {
        ctx.vm.get_object_mut(event_id).prototype = Some(proto);
    }
    for &(key, value) in extra_props {
        ctx.vm.define_shaped_property(
            event_id,
            PropertyKey::String(key),
            PropertyValue::Data(value),
            PropertyAttrs::BUILTIN,
        );
    }

    // §2.9 step 1 dispatch flag + GC root for the walk window.
    ctx.vm.dispatched_events.insert(event_id);
    let outcome = dispatch_vm_event(ctx, event_id, target);
    ctx.vm.dispatched_events.remove(&event_id);

    match outcome {
        Ok(o) => FireResult {
            threw: o.threw,
            canceled: !o.not_prevented,
        },
        // A VM-level dispatch failure is rare (listener throws go through
        // report-an-exception, not `Err`); treat it as "threw" so the
        // transaction aborts rather than committing under a hidden error.
        Err(_) => FireResult {
            threw: true,
            canceled: false,
        },
    }
}
