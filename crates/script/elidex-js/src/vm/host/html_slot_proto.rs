//! `HTMLSlotElement.prototype` intrinsic (WHATWG HTML §4.12.4).
//!
//! Members:
//! - `name` accessor — reflects the `name` content attribute (string,
//!   default `""`).
//! - `assign(...nodes)` — manual-mode slot assignment.  Each argument is
//!   WebIDL-union-coerced to `(Element or Text)`; non-matching arguments
//!   throw `TypeError` per spec §4.2.2.5 step 1.  Engine validation
//!   errors (`NotManualMode`, `NotHostChild`, ...) are silent no-ops to
//!   match browser behaviour.
//! - `assignedNodes(options?)` — returns a fresh Array of distributed
//!   nodes.  `options.flatten=true` currently degrades to the
//!   non-flatten path (deferred to slot `#11-shadow-slot-flatten`).
//! - `assignedElements(options?)` — same as `assignedNodes` filtered to
//!   Element nodes.
//!
//! Inherits `HTMLElement.prototype`.  Identity for `<slot>` wrappers
//! reuses the generic element-wrapper identity backed by
//! `HostData::wrapper_cache` (no per-slot wrapper cache needed —
//! `create_element_wrapper` is already identity-stable per
//! `(entity, ComponentKind::Element)`).
//!
//! ## Deferred
//!
//! `slotchange` currently fires only on explicit `slot.assign()` —
//! named-mode redistribution detection (light-DOM mutation triggering
//! re-distribution) is deferred to
//! `#11-shadow-slotchange-named-mode-detection`.  `assignedNodes`
//! `flatten=true` (nested-slot recursive expansion) degrades to the
//! non-flatten path, deferred to `#11-shadow-slot-flatten`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  Distribution
//! state lives in [`elidex_ecs::EcsDom::slot_assign`] /
//! [`elidex_ecs::EcsDom::assigned_nodes`]; this file only coerces JS
//! values and dispatches.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, PropertyKey, VmError};
use super::super::VmInner;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api, wrap_entities_as_array};

impl VmInner {
    /// Append `slot` to the signal-slots set and ensure the
    /// "notify mutation observers" microtask is queued (WHATWG DOM
    /// §4.2.2.5 + §4.3 step 1).  Deduplicated linear scan — the
    /// set is typically tiny (a handful of slots per microtask
    /// burst) so avoiding `HashSet` overhead pays off in the common
    /// case.  The microtask is coalesced via
    /// [`Self::mutation_observer_microtask_queued`]: the first signal
    /// of a tick enqueues a `Microtask::NotifyMutationObservers`;
    /// subsequent signals piggy-back on the same checkpoint without
    /// re-enqueuing.  Ordering with `Promise.then` reactions is
    /// preserved because the microtask lands in `microtask_queue`
    /// at signal time, NOT at drain-tail.
    pub(crate) fn signal_slot_change(&mut self, slot: Entity) {
        if !self.pending_slot_change_signals.contains(&slot) {
            self.pending_slot_change_signals.push_back(slot);
        }
        if !self.mutation_observer_microtask_queued {
            self.mutation_observer_microtask_queued = true;
            self.microtask_queue
                .push_back(super::super::natives_promise::Microtask::NotifyMutationObservers);
        }
    }

    /// Allocate `HTMLSlotElement.prototype` chained to
    /// `HTMLElement.prototype` and install the `name` accessor plus
    /// `assign` / `assignedNodes` / `assignedElements` methods.
    pub(in crate::vm) fn register_html_slot_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_slot_prototype before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_slot_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        let name_sid = self.well_known.name;
        self.install_accessor_pair(
            proto_id,
            name_sid,
            slot_get_name,
            Some(slot_set_name),
            attrs,
        );

        let assign_sid = self.strings.intern("assign");
        self.install_native_method(
            proto_id,
            assign_sid,
            slot_assign,
            shape::PropertyAttrs::METHOD,
        );

        let assigned_nodes_sid = self.well_known.assigned_nodes;
        self.install_native_method(
            proto_id,
            assigned_nodes_sid,
            slot_assigned_nodes,
            shape::PropertyAttrs::METHOD,
        );

        let assigned_elements_sid = self.well_known.assigned_elements;
        self.install_native_method(
            proto_id,
            assigned_elements_sid,
            slot_assigned_elements,
            shape::PropertyAttrs::METHOD,
        );
    }
}

// -------------------------------------------------------------------------
// Brand check
// -------------------------------------------------------------------------

fn require_slot_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    // Pre-throw for non-wrapper receivers (`{}` / primitives /
    // unrelated Object) so the WebIDL brand check fires before
    // falling into the unbound-wrapper silent-no-op branch.
    // `require_receiver`'s `Ok(None)` collapses non-wrapper AND
    // unbound-wrapper into one case; only the latter should silent-
    // no-op per the elidex unbound-receiver policy.
    if !super::event_target::this_is_node_wrapper(ctx.vm, this) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLSlotElement': Illegal invocation"
        )));
    }
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLSlotElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "slot") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLSlotElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// -------------------------------------------------------------------------
// `name` accessor (reflects the `name` content attribute)
// -------------------------------------------------------------------------

fn slot_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_slot_receiver(ctx, this, "name")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let attr_sid = ctx.vm.well_known.name;
    invoke_dom_api(ctx, "getAttribute", entity, &[JsValue::String(attr_sid)]).map(|v| match v {
        JsValue::Null => JsValue::String(ctx.vm.well_known.empty),
        other => other,
    })
}

fn slot_set_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_slot_receiver(ctx, this, "name")? else {
        return Ok(JsValue::Undefined);
    };
    let value_sid = coerce_first_arg_to_string_id(ctx, args)?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let attr_sid = ctx.vm.well_known.name;
    invoke_dom_api(
        ctx,
        "setAttribute",
        entity,
        &[JsValue::String(attr_sid), JsValue::String(value_sid)],
    )
}

// -------------------------------------------------------------------------
// `assign(...nodes)` method
// -------------------------------------------------------------------------

/// Coerce a single JS argument to a host-tracked `Entity` of an
/// Element-or-Text node.  Returns `Err(TypeError)` per WebIDL union
/// semantics when the value isn't a host wrapper at all or its
/// `NodeKind` isn't `Element` / `Text` (HTML §4.2.2.5 step 1).
fn coerce_element_or_text(ctx: &NativeContext<'_>, value: JsValue) -> Result<Entity, VmError> {
    let entity = super::event_target::entity_from_this(ctx, value).ok_or_else(|| {
        VmError::type_error(
            "Failed to execute 'assign' on 'HTMLSlotElement': \
             argument is not of type (Element or Text)"
                .to_string(),
        )
    })?;
    // Use is_element_entity for the kind probe — `entity_from_this`
    // only confirmed HostObject wrapping; we still need a Node-kind
    // check.  Reuse the inferred-kind helper so payload-only entities
    // are classified correctly (matches `require_receiver` policy).
    let host = ctx.vm.host_data.as_deref().expect(
        "coerce_element_or_text invoked after entity_from_this returned Some, \
         which requires host_data to be bound",
    );
    let kind = host
        .dom_shared()
        .node_kind_inferred(entity)
        .ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'assign' on 'HTMLSlotElement': \
             argument is not of type (Element or Text)"
                    .to_string(),
            )
        })?;
    if !matches!(kind, NodeKind::Element | NodeKind::Text) {
        return Err(VmError::type_error(
            "Failed to execute 'assign' on 'HTMLSlotElement': \
             argument is not of type (Element or Text)"
                .to_string(),
        ));
    }
    Ok(entity)
}

fn slot_assign(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(slot) = require_slot_receiver(ctx, this, "assign")? else {
        return Ok(JsValue::Undefined);
    };
    // WebIDL union coercion happens BEFORE engine validation per
    // spec §4.2.2.5 step 1 — any non-Element/Text argument is a
    // TypeError observable to script.
    let mut nodes: Vec<Entity> = Vec::with_capacity(args.len());
    for &arg in args {
        nodes.push(coerce_element_or_text(ctx, arg)?);
    }
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // Engine validation errors (`NotASlot` is impossible here per
    // brand check, `NoShadowRoot` / `NotManualMode` / `NotHostChild`
    // / `InvalidNodeKind`) are silent no-ops to match Chrome /
    // Firefox.  The spec's `manually assigned nodes` slot is only
    // observable through `assignedNodes()`, which `slot_assign`'s
    // failure path correctly leaves untouched.
    // WHATWG DOM §4.2.2.5 "signal a slot change": signal every slot
    // whose `assigned_nodes` list changed.  The engine returns the
    // receiver AND any other slots from which the new nodes had to
    // be removed (spec step 3).  Engine validation failures (`Err`)
    // produce no observable mutation and therefore no event.
    if let Ok(changed_slots) = ctx.host().dom().slot_assign(slot, nodes) {
        for changed in changed_slots {
            ctx.vm.signal_slot_change(changed);
        }
    }
    Ok(JsValue::Undefined)
}

// -------------------------------------------------------------------------
// `assignedNodes(options?)` / `assignedElements(options?)`
// -------------------------------------------------------------------------

/// Parse the `flatten` option from an `AssignedNodesOptions`
/// dictionary.  Per WebIDL dictionary conversion (§3.2.18):
/// - missing / `undefined` / `null` → empty dict → `flatten=false`
/// - any other non-Object primitive (number / string / boolean / …)
///   → throw `TypeError`
/// - Object with no `flatten` key OR key value `undefined` → `false`
/// - Object with `flatten` key → `ToBoolean`-coerce per WebIDL
fn read_flatten_option(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<bool, VmError> {
    let Some(&first) = args.first() else {
        return Ok(false);
    };
    let opts_id = match first {
        JsValue::Undefined | JsValue::Null => return Ok(false),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'HTMLSlotElement': \
                 argument 1 is not of type 'AssignedNodesOptions'"
            )));
        }
    };
    let key = PropertyKey::String(ctx.vm.well_known.flatten);
    let raw = ctx.vm.get_property_value(opts_id, key)?;
    if matches!(raw, JsValue::Undefined) {
        return Ok(false);
    }
    Ok(super::super::coerce::to_boolean(ctx.vm, raw))
}

fn slot_assigned_nodes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(slot) = require_slot_receiver(ctx, this, "assignedNodes")? else {
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    };
    let flatten = read_flatten_option(ctx, args, "assignedNodes")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    }
    let entities = ctx.host().dom().assigned_nodes(slot, flatten);
    // `create_element_wrapper` dispatches on `NodeKind` internally,
    // so a Text entity here chains through `Text.prototype` — the
    // helper name is historical, not Element-only.
    Ok(wrap_entities_as_array(ctx.vm, &entities))
}

fn slot_assigned_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(slot) = require_slot_receiver(ctx, this, "assignedElements")? else {
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    };
    let flatten = read_flatten_option(ctx, args, "assignedElements")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    }
    let raw = ctx.host().dom().assigned_nodes(slot, flatten);
    // Filter to Element entities BEFORE wrapper allocation to avoid
    // allocating wrappers for nodes that won't be returned.
    let dom = ctx.host().dom();
    let elements: Vec<Entity> = raw
        .into_iter()
        .filter(|&e| matches!(dom.node_kind_inferred(e), Some(NodeKind::Element)))
        .collect();
    Ok(wrap_entities_as_array(ctx.vm, &elements))
}

// -------------------------------------------------------------------------
// slotchange dispatch (microtask checkpoint tail)
// -------------------------------------------------------------------------

/// Drain [`VmInner::pending_slot_change_signals`] and fire a
/// `slotchange` Event (bubbles=true, composed=false) at each slot.
///
/// Dispatched by the [`super::super::natives_promise::Microtask::NotifyMutationObservers`]
/// microtask variant — enqueued at signal time by
/// [`VmInner::signal_slot_change`] and coalesced per-tick via
/// [`VmInner::mutation_observer_microtask_queued`].  Drain pass
/// processes it FIFO with `Promise.then` reactions and
/// `queueMicrotask` callbacks, so a `Promise.then(cb)` registered
/// AFTER `slot.assign()` observes the post-slotchange state, while
/// one registered BEFORE the assign still fires first (WHATWG DOM
/// §4.3 step 1).  Mutation observer callbacks themselves remain
/// embedder-driven via `Vm::deliver_mutation_records`; the
/// slotchange half is fully spec-correct.
///
/// The signal-slots set is **snapshotted before dispatch** per spec
/// §4.3 "notify mutation observers" steps 4–5 + 7 ("let signalSet be
/// a clone of signal slots; empty signal slots; ... for each slot of
/// signalSet: fire an event named `slotchange` at slot").  Signals
/// enqueued by a `slotchange` listener body (re-entrant
/// `slot.assign()` calls) re-arm the coalescing flag and enqueue a
/// fresh notify-MO microtask, which runs later in the SAME drain
/// pass.  Returns the number of events actually fired (telemetry /
/// tests).
pub(in crate::vm) fn dispatch_pending_slotchange_signals(vm: &mut VmInner) -> usize {
    if vm.pending_slot_change_signals.is_empty() {
        return 0;
    }
    if !vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
    {
        // Defensive — `Vm::unbind` clears `pending_slot_change_signals`
        // (see `vm_api.rs`), so this branch is only reachable on a
        // torn-down VM that somehow re-enters `drain_microtasks`.
        // Drop the queue silently rather than panic in `host()` /
        // `dom()`.
        vm.pending_slot_change_signals.clear();
        return 0;
    }
    let type_sid = vm.well_known.slotchange_event;
    // Snapshot the signal-slots set BEFORE dispatch (WHATWG DOM
    // §4.3 "notify mutation observers" steps 4–5 + 7: clone signal
    // slots, empty signal slots, then fire `slotchange` for each slot
    // in the clone).  Slots signaled by a `slotchange` listener body
    // during this pass land on the live queue and fire in the next
    // microtask checkpoint — NOT re-entrantly inside this dispatch.
    let snapshot: Vec<Entity> = vm.pending_slot_change_signals.drain(..).collect();
    let mut fired = 0usize;
    for slot in snapshot {
        // `slot.assign()` followed by tree mutation
        // (`parent.removeChild(slot)`) before the microtask drains
        // can destroy the entity; drop without firing.
        if !vm
            .host_data
            .as_deref()
            .is_some_and(|hd| hd.dom_shared().contains(slot))
        {
            continue;
        }
        let mut ctx = NativeContext::new_call(vm);
        let _ = super::event_target_dispatch::dispatch_simple_event(
            &mut ctx, slot, type_sid, true, false,
        );
        fired += 1;
    }
    fired
}
