// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! Range mutating methods + Phase-A stubs (WHATWG DOM §4.4 §6).
//!
//! Split from sibling [`super::range_proto`] (Copilot R3 MIN — 1000-line
//! convention) to keep that module focused on the constructor /
//! accessors / non-mutating methods.  All natives here are wired
//! into `Range.prototype` from `register_range_global` in
//! `range_proto.rs`.
//!
//! The 3 mutating methods (`deleteContents` / `extractContents` /
//! `insertNode`) snapshot the registered Range, run the engine-indep
//! mutating impl, then commit the post-op boundary state back via
//! [`commit_range_after_mutation`] — mutation hooks alone cannot
//! restore the spec-required collapse on `deleteContents` (§4.4 step
//! 3).  The 3 stubs (`cloneContents` / `surroundContents` /
//! `createContextualFragment`) all brand-check `this` then throw
//! `NotSupportedError` pending the `#11-range-full-impl` slot.

#![cfg(feature = "engine")]

use elidex_dom_api::{Range, RangeId};

use super::super::value::{JsValue, NativeContext, VmError};

use super::range_proto::{arg_node, detached_range_error, require_range_receiver};

/// Snapshot the registered Range, then commit `mutated` back over it.
/// Copilot R1: `deleteContents` / `extractContents` / `insertNode`
/// engine-indep impls perform boundary updates (notably the
/// post-delete collapse to the start point per WHATWG §4.4
/// `deleteContents` step 3) that are NOT recoverable from the
/// mutation hooks alone — `set_text_data` hooks only clamp offsets to
/// the new length, missing the spec-required collapse.  Persist the
/// post-op boundary state explicitly.
fn commit_range_after_mutation(
    ctx: &mut NativeContext<'_>,
    id: RangeId,
    method: &'static str,
    mutated: &Range,
) -> Result<(), VmError> {
    // Caller has already gated entry on `host_if_bound`, but the
    // engine-indep mutation (`delete_contents` / etc.) routes
    // through `dom.remove_child` etc. which is a no-op if the VM
    // got unbound mid-operation (no realistic concurrent path).
    // Re-gate defensively.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, method));
    }
    let host = ctx.host();
    let (dom, registry) = host.split_dom_and_live_ranges();
    let applied = registry.with_range_mut(id, dom, |r, _| {
        r.start_container = mutated.start_container;
        r.start_offset = mutated.start_offset;
        r.end_container = mutated.end_container;
        r.end_offset = mutated.end_offset;
    });
    if applied.is_none() {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on 'Range': \
                 the Range has been detached"
            ),
        ));
    }
    Ok(())
}

pub(super) fn native_range_delete_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "deleteContents")?;
    // Mutating Range methods need `&mut EcsDom` + `&mut LiveRangeRegistry`
    // — the with_range_mut split gives `&EcsDom` only.  Snapshot the
    // Range, run the deletion through `&mut EcsDom`, then commit the
    // post-op boundary state back to the registry (Copilot R1).
    // Copilot R6: post-`Vm::unbind()` `split_dom_and_live_ranges`
    // asserts on null `dom_ptr`; gate on `host_if_bound`.
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, "deleteContents"));
    }
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| detached_range_error(ctx, "deleteContents"))?
    };
    let host = ctx.host();
    let dom = host.dom();
    range.delete_contents(dom);
    commit_range_after_mutation(ctx, id, "deleteContents", &range)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_range_extract_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "extractContents")?;
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, "extractContents"));
    }
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| detached_range_error(ctx, "extractContents"))?
    };
    let host = ctx.host();
    let dom = host.dom();
    let fragment = range.extract_contents(dom);
    commit_range_after_mutation(ctx, id, "extractContents", &range)?;
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(fragment)))
}

pub(super) fn native_range_insert_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_range_receiver(ctx, this, "insertNode")?;
    let node = arg_node(ctx, args.first().copied(), "insertNode")?;
    if ctx.host_if_bound().is_none() {
        return Err(detached_range_error(ctx, "insertNode"));
    }
    // Copilot R13 (#2): read only the fields needed for the
    // engine-indep call; the snapshot+commit pattern is unsafe for
    // `insertNode` because the DOM ops fire `after_split_text` /
    // `after_insert` hooks that adjust the **registered** range, and
    // a clone-and-commit would overwrite those adjustments.
    let snapshot = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| {
                (r.start_container, r.start_offset, r.collapsed())
            })
            .ok_or_else(|| detached_range_error(ctx, "insertNode"))?
    };
    let (start_container, start_offset, was_collapsed) = snapshot;

    // Build a transient `Range` purely as the argument carrier — the
    // engine-indep `insert_node` takes `&self` and reads only
    // `start_container` / `start_offset` from it.
    let mut transient = Range::new(start_container);
    transient.set_start(start_container, start_offset);
    transient.set_end(start_container, start_offset);

    let host = ctx.host();
    let dom = host.dom();
    let outcome = transient.insert_node(dom, node);

    match outcome {
        None => {
            // WHATWG §4.4 step 6 (pre-insertion validity) failed
            // (cycle / orphan parent) — surface as
            // `HierarchyRequestError`.  No DOM mutation happened
            // (Copilot R13 #1: validity check runs before split).
            Err(VmError::dom_exception(
                ctx.vm.well_known.dom_exc_hierarchy_request_error,
                "Failed to execute 'insertNode' on 'Range': \
                 the node could not be inserted at the start boundary \
                 (cycle, missing reference node, or orphan parent).",
            ))
        }
        Some((parent, new_offset)) => {
            // WHATWG §4.4 step 13: when the range was collapsed,
            // set the end to (parent, newOffset).  Apply directly
            // to the registered range so the §5.10/§4.2.3 hook
            // adjustments to start remain intact.  Non-collapsed
            // ranges need no explicit commit: hooks already
            // migrated both boundaries.
            if was_collapsed {
                let host = ctx.host();
                let (dom, registry) = host.split_dom_and_live_ranges();
                let applied = registry.with_range_mut(id, dom, |r, _| {
                    r.end_container = parent;
                    r.end_offset = new_offset;
                });
                if applied.is_none() {
                    return Err(detached_range_error(ctx, "insertNode"));
                }
            }
            Ok(JsValue::Undefined)
        }
    }
}

// ---------------------------------------------------------------------------
// Stubs: cloneContents / surroundContents / createContextualFragment
// ---------------------------------------------------------------------------

/// elidex-specific `NotSupportedError` placeholder pending deep-clone
/// infrastructure; spec returns a `DocumentFragment` per WHATWG §4.4.
/// Full impl tracked at `#11-range-full-impl`.
pub(super) fn native_range_clone_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_range_receiver(ctx, this, "cloneContents")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "Failed to execute 'cloneContents' on 'Range': \
         deep-clone of Range contents is not yet implemented.",
    ))
}

/// elidex-specific `NotSupportedError` placeholder pending deep-clone
/// infrastructure; spec wraps the contents in `newParent` per WHATWG §4.4.
/// Full impl tracked at `#11-range-full-impl`.
pub(super) fn native_range_surround_contents(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_range_receiver(ctx, this, "surroundContents")?;
    // Validate new_parent shape per spec step 2 (Element / Text / Comment /
    // ProcessingInstruction allowed; Document / DocumentType /
    // DocumentFragment rejected).  Currently unimplemented past brand
    // check.
    let _ = arg_node(ctx, args.first().copied(), "surroundContents")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "Failed to execute 'surroundContents' on 'Range': \
         surround-with-parent is not yet implemented.",
    ))
}

/// elidex-specific `NotSupportedError` placeholder pending parser
/// wiring; spec returns a `DocumentFragment` per WHATWG §3.2 step 7.
/// Full impl tracked at `#11-range-full-impl`.
pub(super) fn native_range_create_contextual_fragment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_range_receiver(ctx, this, "createContextualFragment")?;
    let _ = args.first().copied().unwrap_or(JsValue::Undefined);
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "Failed to execute 'createContextualFragment' on 'Range': \
         HTML-parser wiring for fragment parsing is not yet implemented.",
    ))
}
