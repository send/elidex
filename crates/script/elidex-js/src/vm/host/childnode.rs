//! ChildNode mixin — `before` / `after` / `replaceWith` / `remove`
//! (WHATWG DOM §4.2.8).
//!
//! The mixin is installed on `Element.prototype` and
//! `CharacterData.prototype`, so Element, Text, and Comment wrappers
//! share these natives.  WHATWG also defines the mixin on
//! `DocumentType`, but elidex has no JS surface for creating
//! DocumentType wrappers yet — install on `DocumentType.prototype`
//! lands alongside `document.doctype` / `DOMImplementation`.
//!
//! # Convergence (B1.2b)
//!
//! These natives are **thin dispatchers**: they brand-check the receiver,
//! normalise each variadic argument (the WebIDL `(Node or DOMString)` union +
//! ShadowRoot/detached rejection — VM-side marshalling), then route to the
//! engine-independent dom-api handler via [`super::dom_bridge::invoke_dom_api`]
//! (`before`/`after`/`replaceWith`/`prepend`/`append`/`replaceChildren`/`remove`).
//! The handler owns the algorithm — "convert nodes into a node" (§4.2.6, incl. the
//! temp-`DocumentFragment` build), viable-sibling capture, pre-insertion validity,
//! the insert/replace, and `MutationRecord` production via the `apply_*` primitives.
//! This is the same path boa already uses (One-issue-one-way); the prior VM-side
//! re-implementation (`flatten_into`/`convert_nodes_to_single_node_or_fragment`/…)
//! is gone.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Install `before` / `after` / `replaceWith` / `remove` onto
    /// `proto_id` (Element.prototype or CharacterData.prototype —
    /// WHATWG ChildNode mixin is shared between Element / CharacterData
    /// / DocumentType).  Re-installing `remove` on
    /// CharacterData.prototype lets Text / Comment wrappers call it
    /// without duplicating dispatch logic.
    pub(in crate::vm) fn install_child_node_mixin(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (self.well_known.before, native_child_node_before as NativeFn),
            (self.well_known.after, native_child_node_after),
            (self.well_known.replace_with, native_child_node_replace_with),
            (self.well_known.remove, native_child_node_remove),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

// ---------------------------------------------------------------------------
// Argument normalisation — WebIDL `(Node or DOMString)` union (VM marshalling)
// ---------------------------------------------------------------------------

/// Normalise one variadic mixin argument into a `JsValue` the dom-api handler's
/// `collect_nodes` understands: a valid Node wrapper is kept as its `Object` (the
/// bridge's `prepare_arg` resolves it to the entity), anything else is
/// ToString-coerced to a `String` (the handler builds the Text node — one
/// string→Text home). **No tree mutation** happens here, so a `ToString` throw
/// (e.g. a Symbol arg) leaves the DOM untouched — the algorithm's atomicity is
/// preserved without the old pre-validation/wrapper-fragment dance.
///
/// Two brand rejections stay VM-side (engine-bound marshalling): a **ShadowRoot**
/// arg cannot be moved into the light DOM (its host edge is immovable, §4.2.3 —
/// mirrors `node_proto::reject_shadow_root_insertion` for `appendChild`); and a
/// wrapper whose entity bits decode but no longer exist is a **detached** node
/// (coercing it to text would mask the bug, matching `require_node_arg`).
fn normalize_mixin_arg(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = val {
        if let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind {
            if let Some(entity) = Entity::from_bits(entity_bits) {
                if super::event_target::is_shadow_root_entity(ctx.vm, entity) {
                    let hierarchy_request = ctx.vm.well_known.dom_exc_hierarchy_request_error;
                    return Err(VmError::dom_exception(
                        hierarchy_request,
                        "Failed to execute mixin insertion: a ShadowRoot cannot be moved into the light DOM"
                            .to_string(),
                    ));
                }
                if !ctx.host().dom().contains(entity) {
                    return Err(VmError::type_error(
                        "Failed to execute argument conversion: \
                         the node is detached (invalid entity).",
                    ));
                }
                // Accept any DOM node (incl. legacy entities missing `NodeKind` but
                // carrying DOM payload). Window is an `EventTarget` but not a Node, so
                // it falls through to text coercion.
                let inferred = ctx.host().dom().node_kind_inferred(entity);
                if matches!(inferred, Some(k) if k.is_node()) {
                    return Ok(val);
                }
            }
        }
    }
    // Non-Node → DOMString branch (`to_string` may throw on Symbol).
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    Ok(JsValue::String(sid))
}

/// Normalise the whole variadic argument list (see [`normalize_mixin_arg`]).
fn normalize_mixin_args(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<Vec<JsValue>, VmError> {
    let mut out = Vec::with_capacity(args.len());
    for &arg in args {
        out.push(normalize_mixin_arg(ctx, arg)?);
    }
    Ok(out)
}

/// Shared dispatcher for the variadic ChildNode/ParentNode mixin methods
/// (`before`/`after`/`replaceWith`/`prepend`/`append`/`replaceChildren`): brand-check
/// the receiver, normalise the args (VM marshalling), then route to the
/// engine-independent dom-api handler. The handler owns the algorithm + records.
pub(super) fn dispatch_child_parent_mixin(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    interface: &str,
    method: &'static str,
    is_kind: fn(NodeKind) -> bool,
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, interface, method, is_kind)?
    else {
        return Ok(JsValue::Undefined);
    };
    let normalized = normalize_mixin_args(ctx, args)?;
    super::dom_bridge::invoke_dom_api(ctx, method, entity, &normalized)
}

// ---------------------------------------------------------------------------
// Natives (thin dispatchers)
// ---------------------------------------------------------------------------

/// ChildNode mixin receivers must be Element, Text, Comment, or
/// other CharacterData kinds (DocumentType is also valid per spec
/// but its prototype wrapper isn't installed yet — see module doc).
fn is_child_node_kind(k: NodeKind) -> bool {
    matches!(
        k,
        NodeKind::Element
            | NodeKind::Text
            | NodeKind::Comment
            | NodeKind::CdataSection
            | NodeKind::ProcessingInstruction
    )
}

fn native_child_node_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_child_parent_mixin(ctx, this, args, "ChildNode", "before", is_child_node_kind)
}

fn native_child_node_after(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_child_parent_mixin(ctx, this, args, "ChildNode", "after", is_child_node_kind)
}

fn native_child_node_replace_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_child_parent_mixin(
        ctx,
        this,
        args,
        "ChildNode",
        "replaceWith",
        is_child_node_kind,
    )
}

fn native_child_node_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = super::event_target::require_receiver(
        ctx,
        this,
        "ChildNode",
        "remove",
        is_child_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    super::dom_bridge::invoke_dom_api(ctx, "remove", entity, &[])
}
