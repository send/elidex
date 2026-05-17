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
    /// Append `slot` to the signal-slots set (WHATWG DOM §4.2.2.5
    /// "signal a slot change").  Deduplicated linear scan — the set
    /// is typically tiny (a handful of slots per microtask burst) so
    /// avoiding `HashSet` overhead pays off in the common case.
    pub(crate) fn signal_slot_change(&mut self, slot: Entity) {
        if self.pending_slot_change_signals.contains(&slot) {
            return;
        }
        self.pending_slot_change_signals.push_back(slot);
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
    let assignment_applied = ctx.host().dom().slot_assign(slot, nodes).is_ok();
    if assignment_applied {
        // WHATWG DOM §4.2.2.5 "signal a slot change": queue this slot
        // for the next microtask checkpoint so a `slotchange` event
        // fires at it.  Only successful assignments signal — engine
        // validation failures leave the assignment untouched and so
        // produce no observable change.
        ctx.vm.signal_slot_change(slot);
    }
    Ok(JsValue::Undefined)
}

// -------------------------------------------------------------------------
// `assignedNodes(options?)` / `assignedElements(options?)`
// -------------------------------------------------------------------------

/// Parse the `flatten` option from an `AssignedNodesOptions`
/// dictionary.  Missing arg / non-object / missing key all yield
/// `false`; otherwise the value is `ToBoolean`-coerced per WebIDL.
fn read_flatten_option(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<bool, VmError> {
    let Some(&first) = args.first() else {
        return Ok(false);
    };
    let JsValue::Object(opts_id) = first else {
        return Ok(false);
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
    let flatten = read_flatten_option(ctx, args)?;
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
    let flatten = read_flatten_option(ctx, args)?;
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
/// Called from `drain_microtasks` after `process_pending_rejections`,
/// matching the spec's "notify mutation observers" microtask
/// checkpoint (WHATWG DOM §4.3.4): mutation observer callbacks run,
/// then any signaled slots get their slotchange dispatched.  We do
/// not currently queue the MO microtask via the Promise / queueMicrotask
/// queue — `Vm::deliver_mutation_records` is embedder-driven — so
/// firing slotchange at the drain tail is the closest correct timing
/// for script-initiated assignments.
///
/// New signals enqueued by a slotchange listener body (re-entrant
/// `slot.assign()` calls) are picked up on the next iteration of the
/// while loop, preserving spec ordering ("set signal slots is empty"
/// happens before each listener runs).  Returns the number of events
/// actually fired (telemetry / tests).
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
    // §4.3.4 "notify mutation observers" step 3: clone signal slots,
    // empty signal slots, then fire `slotchange` for each slot in
    // the clone).  Slots signaled by a `slotchange` listener body
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
        let mut ctx = NativeContext { vm };
        let _ = super::event_target_dispatch::dispatch_simple_event(
            &mut ctx, slot, type_sid, true, false,
        );
        fired += 1;
    }
    fired
}
