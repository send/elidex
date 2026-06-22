//! `Element.prototype.insertAdjacentElement` /
//! `insertAdjacentText` (WHATWG DOM §4.9).
//!
//! Split out of [`super::element_proto`] to keep that file under
//! the project's 1000-line convention.  The install-time reference
//! in `install_element_matches` reaches these natives via their
//! `pub(super)` re-export.
//!
//! # Convergence (B1.2b-2)
//!
//! These natives are **thin dispatchers**: they brand-check the receiver and
//! (for `insertAdjacentElement`) the `Element` arg — the WebIDL interface-type
//! binding check + the detached-vs-wrong-type distinction, both engine-bound
//! marshalling — then route to the engine-independent dom-api handler via
//! [`super::dom_bridge::invoke_dom_api`] (`insertAdjacentElement`/
//! `insertAdjacentText`). The handler owns the algorithm — WHATWG "insert
//! adjacent" site resolution, the SyntaxError on a bad position, the parent-null
//! no-op, the Text allocation (leak-careful), and `MutationRecord` production via
//! the `apply_*` primitives. This is the same path boa already uses
//! (One-issue-one-way); the prior VM re-implementation (`perform_adjacent_insert`
//! / `parse_adjacent_position` / `position_requires_parent`) is gone.
//!
//! The `Element` arg brand-check ([`require_element_arg`]) stays VM-side as
//! marshalling — it operates on `JsValue`/`ObjectKind` and distinguishes a
//! *detached* wrapper (recycled entity) from a *wrong-type* one, which the
//! engine-independent layer cannot message: the bridge's `materialize` rejects a
//! destroyed entity generically before any handler runs. This mirrors
//! `childnode::normalize_mixin_arg`, which likewise keeps the detached /
//! ShadowRoot brand rejections VM-side. The handler additionally guards
//! Element-kind on the resolved entity (defense-in-depth here, sole guard for
//! boa/wasm).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectKind, VmError};

use elidex_ecs::{Entity, NodeKind};

// ---------------------------------------------------------------------------
// `Element` argument brand-check — WebIDL interface-type binding (marshalling)
// ---------------------------------------------------------------------------

/// TypeError thrown when the second argument of `insertAdjacentElement`
/// is not an Element wrapper.  Matches the Blink / Gecko message form.
fn adjacent_element_arg_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'insertAdjacentElement' on 'Element': \
         parameter 2 is not of type 'Element'."
            .to_owned(),
    )
}

/// TypeError thrown when `insertAdjacentElement`'s second argument
/// is a HostObject whose entity has been destroyed / recycled.
/// Separated from [`adjacent_element_arg_error`] so stale wrappers
/// surface the "detached" failure mode rather than being misreported
/// as non-Element (matches [`super::event_target::require_receiver`]
/// which also distinguishes destroyed vs. wrong-kind receivers).
fn adjacent_element_detached_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'insertAdjacentElement' on 'Element': \
         parameter 2 is detached (invalid entity)."
            .to_owned(),
    )
}

/// Validate that a method argument is an Element wrapper (WebIDL `Element`
/// interface-type binding), throwing a `TypeError` on any non-Element value
/// (`null` / `undefined` / non-HostObject objects / HostObjects that are not
/// `NodeKind::Element`).  A HostObject whose entity has been destroyed surfaces
/// a distinct "detached" error so scripts can distinguish stale wrappers from
/// genuine type mismatches — a distinction the engine-independent handler
/// cannot make (the bridge rejects a destroyed entity generically at
/// `materialize`, before the handler runs), so this brand-check stays VM-side as
/// marshalling (mirroring `childnode::normalize_mixin_arg`).
fn require_element_arg(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<(), VmError> {
    let JsValue::Object(id) = value else {
        return Err(adjacent_element_arg_error());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(adjacent_element_arg_error());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(adjacent_element_detached_error)?;
    // Stale-entity check BEFORE the kind lookup: a destroyed entity
    // has no components, so `node_kind_inferred` would return None
    // and masquerade as "wrong type".  Catching it here keeps the
    // error message aligned with `require_receiver` (which makes the
    // same split for stale receivers).
    if !ctx.host().dom().contains(entity) {
        return Err(adjacent_element_detached_error());
    }
    match ctx.host().dom().node_kind_inferred(entity) {
        Some(NodeKind::Element) => Ok(()),
        _ => Err(adjacent_element_arg_error()),
    }
}

// ---------------------------------------------------------------------------
// Natives (thin dispatchers → dom-api handler)
// ---------------------------------------------------------------------------

/// `Element.prototype.insertAdjacentElement(where, element)` —
/// WHATWG DOM §4.9.
pub(super) fn native_element_insert_adjacent_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(target) = super::event_target::require_receiver(
        ctx,
        this,
        "Element",
        "insertAdjacentElement",
        |k| k == NodeKind::Element,
    )?
    else {
        return Ok(JsValue::Null);
    };
    // WebIDL binding order: ToString-coerce `where`, then brand-check the
    // `Element` arg — both before the engine-independent algorithm (which does
    // the position parse / SyntaxError). `where`'s String + the validated
    // element handle pass through `invoke_dom_api`; the handler resolves the
    // element through the identity map and re-guards its kind (boa parity).
    let where_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let where_sid = super::super::coerce::to_string(ctx.vm, where_arg)?;
    let element_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    require_element_arg(ctx, element_arg)?;
    super::dom_bridge::invoke_dom_api(
        ctx,
        "insertAdjacentElement",
        target,
        &[JsValue::String(where_sid), element_arg],
    )
}

/// `Element.prototype.insertAdjacentText(where, data)` —
/// WHATWG DOM §4.9.
pub(super) fn native_element_insert_adjacent_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(target) =
        super::event_target::require_receiver(ctx, this, "Element", "insertAdjacentText", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Undefined);
    };
    // WebIDL binding order: ToString-coerce `where` then `data` (both at the
    // binding boundary, before the algorithm). The handler resolves the
    // insertion site BEFORE allocating the Text, so a bad position / parent-null
    // no-op never leaks an orphan Text — the leak-careful discipline lives in
    // the canonical handler now, not here.
    let where_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let where_sid = super::super::coerce::to_string(ctx.vm, where_arg)?;
    let data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let data_sid = super::super::coerce::to_string(ctx.vm, data_arg)?;
    super::dom_bridge::invoke_dom_api(
        ctx,
        "insertAdjacentText",
        target,
        &[JsValue::String(where_sid), JsValue::String(data_sid)],
    )
}
