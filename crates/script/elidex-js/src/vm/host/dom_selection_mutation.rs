// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `Selection` mutating methods (Selection API Living Standard §3,
//! "Selection interface").
//!
//! Split from sibling [`super::dom_selection_proto`] (1000-line
//! touch-time convention, surfaced on PR #418 B1.2d-ii) to keep that
//! module focused on the constructor install / accessors / non-mutating
//! reads.  All natives here are wired into `Selection.prototype` from
//! `register_selection_global` in `dom_selection_proto.rs`, mirroring the
//! `range_proto.rs` / `range_proto_mutation.rs` precedent.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate": no spec algorithms live here.  The
//! mutating natives marshal WebIDL args (JS → Rust), brand-check `this`
//! via [`super::dom_selection_proto::require_selection_receiver`], then
//! dispatch into [`elidex_dom_api::SelectionState`] for the actual state
//! mutation / direction derivation / validity gates.  The two shared
//! borrow-split helpers ([`mutate_selection`] / [`delete_selection_contents`])
//! own only the HostData borrow choreography + dirty-bit / MutationRecord
//! delivery, never spec prose.

#![cfg(feature = "engine")]

use elidex_dom_api::{SelectionError, SelectionState};
use elidex_ecs::Entity;

use super::super::value::{JsValue, NativeContext, VmError};

use super::dom_selection_proto::{
    arg_node_required, arg_offset_or_default, arg_offset_required, arg_range, map_selection_error,
    require_selection_receiver,
};

// ---------------------------------------------------------------------------
// Mutating state-access helpers
// ---------------------------------------------------------------------------

/// Mutable access pattern for Selection methods that mutate state +
/// registry + (optionally) read DOM.  Sets `selectionchange_pending`
/// to `true` on success.  Engine-indep errors returned by the closure
/// (as [`SelectionError`]) are mapped to the corresponding
/// `DOMException` here so the engine-indep crate stays free of
/// VM-side intern dependencies.
///
/// Copilot R1 IMP-3 (registry-leak cleanup): when the closure
/// replaces `active_range_id` with a different `RangeId`, the
/// displaced `RangeId` is unregistered from `LiveRangeRegistry`
/// **only if** no JS `Range` wrapper exists for it in
/// `range_instances`.  This preserves the previous-range-survival
/// contract — user-held `r = sel.getRangeAt(0)` followed by
/// `sel.collapse(...)` keeps `r` live-tracked because its cached
/// wrapper id is in `range_instances` — while preventing unbounded
/// growth of registry entries in tight `sel.collapse(n,0);
/// sel.collapse(n,1); ...` loops that never materialise a wrapper.
fn mutate_selection<F, R>(
    ctx: &mut NativeContext<'_>,
    method: &'static str,
    f: F,
) -> Result<R, VmError>
where
    F: FnOnce(
        &mut SelectionState,
        &mut elidex_dom_api::LiveRangeRegistry,
        &elidex_ecs::EcsDom,
        Entity,
    ) -> Result<R, SelectionError>,
{
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Selection': host environment is not initialised"
            ),
        ));
    }
    // Snapshot the pre-call RangeId so we can detect replacement
    // and free the displaced registry entry if no wrapper exists.
    let prev_id = ctx
        .host()
        .selection_state
        .as_ref()
        .and_then(SelectionState::current_range_id);
    // Confine all HostData borrows to this inner block so we can
    // re-borrow `ctx` afterwards for `map_selection_error`,
    // `selectionchange_pending` mutation, and the displaced-id
    // unregister check.
    let outcome = {
        let host = ctx.host();
        let doc = host.document();
        if host.selection_state.is_none() {
            host.selection_state = Some(SelectionState::new());
        }
        let (dom, registry, sel_slot) = host.split_dom_live_ranges_and_selection();
        let state = sel_slot.as_mut().expect("just initialised");
        f(state, registry, dom, doc)
    };
    let value = outcome.map_err(|e| map_selection_error(ctx, e, method))?;
    let new_id = ctx
        .host()
        .selection_state
        .as_ref()
        .and_then(SelectionState::current_range_id);
    if let Some(old) = prev_id {
        if Some(old) != new_id {
            let host = ctx.host();
            if !host.range_instances.contains_key(&old.0) {
                host.live_range_registry.unregister(old);
            }
        }
    }
    ctx.host().selectionchange_pending = true;
    Ok(value)
}

/// `deleteFromDocument` needs `&mut EcsDom` (the spec algorithm
/// mutates the tree) **plus** `&mut LiveRangeRegistry` + `&mut
/// SelectionState`.  Per CLAUDE.md layering mandate (Copilot R1
/// IMP-2), the spec algorithm lives in the engine-indep
/// [`SelectionState::delete_from_document`] — this VM-side function
/// owns only the borrow split + dirty-bit flip.
fn delete_selection_contents(
    ctx: &mut NativeContext<'_>,
    method: &'static str,
) -> Result<(), VmError> {
    if ctx.host_if_bound().is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Selection': host environment is not initialised"
            ),
        ));
    }
    let host = ctx.host();
    if host.selection_state.is_none() {
        host.selection_state = Some(SelectionState::new());
    }
    let records = {
        let (dom_mut, registry, sel_slot) = host.split_dom_mut_live_ranges_and_selection();
        let state = sel_slot.as_mut().expect("just initialised");
        state.delete_from_document(registry, dom_mut)
    };
    // Single delivery mechanism: route the childList records the engine-indep
    // `delete_from_document` produced through the same chokepoint as the Range
    // natives (the `host`/split borrow above has ended). One-issue-one-way —
    // a record-producing primitive's records are never silently dropped.
    ctx.vm.commit_notify_records(records);
    ctx.host().selectionchange_pending = true;
    Ok(())
}

// ---------------------------------------------------------------------------
// Mutating methods (11)
// ---------------------------------------------------------------------------

pub(super) fn native_selection_add_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "addRange")?;
    let range_id = arg_range(ctx, args.first().copied(), "addRange")?;
    // We need the range's owner document AND the selection's owner
    // document to decide the no-op case.  Pull both from registered
    // state.
    let host = ctx.host();
    let sel_owner = host.document();
    let (dom, registry, sel_slot) = host.split_dom_live_ranges_and_selection();
    if sel_slot.is_none() {
        *sel_slot = Some(SelectionState::new());
    }
    let range_owner = registry
        .with_range(range_id, dom, |r, _| r.owner_document)
        .ok_or_else(|| {
            VmError::type_error(
                "Failed to execute 'addRange' on 'Selection': parameter is not of type 'Range'.",
            )
        })?;
    let changed =
        sel_slot
            .as_mut()
            .expect("just initialised")
            .add_range(range_owner, sel_owner, range_id);
    if changed {
        ctx.host().selectionchange_pending = true;
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_remove_range(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "removeRange")?;
    let range_id = arg_range(ctx, args.first().copied(), "removeRange")?;
    mutate_selection(ctx, "removeRange", |s, _reg, _dom, _doc| {
        s.remove_range(range_id)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_remove_all_ranges(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "removeAllRanges")?;
    mutate_selection(ctx, "removeAllRanges", |s, _reg, _dom, _doc| {
        s.remove_all_ranges();
        Ok(())
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_empty(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "empty")?;
    mutate_selection(ctx, "empty", |s, _reg, _dom, _doc| {
        s.empty();
        Ok(())
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_collapse(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "collapse")?;
    // WebIDL §3.7.6 — arg conversion in declared order.
    let node = arg_node_required(ctx, args.first().copied(), "collapse")?;
    let offset = arg_offset_or_default(ctx, args.get(1).copied())?;
    mutate_selection(ctx, "collapse", |s, reg, dom, doc| {
        s.collapse(reg, dom, doc, node, offset)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_collapse_to_start(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "collapseToStart")?;
    mutate_selection(ctx, "collapseToStart", |s, reg, dom, _doc| {
        s.collapse_to_start(reg, dom)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_collapse_to_end(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "collapseToEnd")?;
    mutate_selection(ctx, "collapseToEnd", |s, reg, dom, _doc| {
        s.collapse_to_end(reg, dom)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_extend(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "extend")?;
    let node = arg_node_required(ctx, args.first().copied(), "extend")?;
    let offset = arg_offset_or_default(ctx, args.get(1).copied())?;
    mutate_selection(ctx, "extend", |s, reg, dom, doc| {
        s.extend(reg, dom, doc, node, offset)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_set_base_and_extent(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "setBaseAndExtent")?;
    // WebIDL declared-order coercion (lesson #245): brand-check both
    // node args + ToUint32 both offset args BEFORE any state probe.
    let anchor = arg_node_required(ctx, args.first().copied(), "setBaseAndExtent")?;
    let anchor_offset = arg_offset_required(ctx, args.get(1).copied(), "setBaseAndExtent")?;
    let focus = arg_node_required(ctx, args.get(2).copied(), "setBaseAndExtent")?;
    let focus_offset = arg_offset_required(ctx, args.get(3).copied(), "setBaseAndExtent")?;
    mutate_selection(ctx, "setBaseAndExtent", |s, reg, dom, doc| {
        s.set_base_and_extent(reg, dom, doc, anchor, anchor_offset, focus, focus_offset)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_select_all_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "selectAllChildren")?;
    let parent = arg_node_required(ctx, args.first().copied(), "selectAllChildren")?;
    mutate_selection(ctx, "selectAllChildren", |s, reg, dom, doc| {
        s.select_all_children(reg, dom, doc, parent)
    })?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_selection_delete_from_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_selection_receiver(ctx, this, "deleteFromDocument")?;
    delete_selection_contents(ctx, "deleteFromDocument")?;
    Ok(JsValue::Undefined)
}
