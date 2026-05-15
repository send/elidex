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

use super::range_proto::{arg_node, require_range_receiver};

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
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'deleteContents' on 'Range': \
                     the Range has been detached",
                )
            })?
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
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'extractContents' on 'Range': \
                     the Range has been detached",
                )
            })?
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
    let mut range = {
        let host = ctx.host();
        let (dom, registry) = host.split_dom_and_live_ranges();
        registry
            .with_range(id, dom, |r, _| r.clone())
            .ok_or_else(|| {
                VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_invalid_state_error,
                    "Failed to execute 'insertNode' on 'Range': \
                     the Range has been detached",
                )
            })?
    };
    let host = ctx.host();
    let dom = host.dom();
    range.insert_node(dom, node);
    commit_range_after_mutation(ctx, id, "insertNode", &range)?;
    Ok(JsValue::Undefined)
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
