//! ParentNode mixin — `prepend` / `append` / `replaceChildren`
//! (WHATWG DOM §5.2.4).
//!
//! Implemented by Element, Document, and DocumentFragment.  We install
//! the same native fns on `Element.prototype` and on the document
//! wrapper at bind time (Document has no shared prototype the way
//! Element does — its wrapper is patched per-bind by
//! [`install_document_methods_if_needed`](super::document)).
//!
//! Argument normalisation reuses
//! [`super::childnode::convert_nodes_to_single_node_or_fragment`] so
//! the Phase 2 (spec §4.2.5) rules are identical.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, PropertyKey, PropertyValue, VmError};
use super::super::{NativeFn, VmInner};
use super::childnode::{convert_nodes_to_single_node_or_fragment, destroy_wrapper_fragment_if_any};
use super::event_target::entity_from_this;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Install `prepend` / `append` / `replaceChildren` on
    /// `proto_id` (Element.prototype or a document wrapper).
    pub(in crate::vm) fn install_parent_node_mixin(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (
                self.well_known.prepend,
                native_parent_node_prepend as NativeFn,
            ),
            (self.well_known.append, native_parent_node_append),
            (
                self.well_known.replace_children,
                native_parent_node_replace_children,
            ),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                shape::PropertyAttrs::METHOD,
            );
        }
    }
}

/// Flatten a DocumentFragment entity (whether caller-supplied or
/// internally wrapped) into its children, otherwise return `[node]`.
fn nodes_to_insert(ctx: &mut NativeContext<'_>, node: Entity) -> Vec<Entity> {
    if matches!(
        ctx.host().dom().node_kind(node),
        Some(NodeKind::DocumentFragment)
    ) {
        ctx.host().dom().children_iter(node).collect()
    } else {
        vec![node]
    }
}

fn native_parent_node_prepend(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let first = ctx.host().dom().children_iter(parent).next();
    let children = nodes_to_insert(ctx, pair.0);
    for child in children {
        match first {
            Some(f) => {
                let _ = ctx.host().dom().insert_before(parent, child, f);
            }
            None => {
                let _ = ctx.host().dom().append_child(parent, child);
            }
        }
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    Ok(JsValue::Undefined)
}

fn native_parent_node_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    for child in children {
        let _ = ctx.host().dom().append_child(parent, child);
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    Ok(JsValue::Undefined)
}

fn native_parent_node_replace_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // Remove every existing child first (snapshot to avoid iterator
    // invalidation under the mutation).
    let existing: Vec<Entity> = ctx.host().dom().children_iter(parent).collect();
    for child in existing {
        let _ = ctx.host().dom().remove_child(parent, child);
    }
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    for child in children {
        let _ = ctx.host().dom().append_child(parent, child);
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    Ok(JsValue::Undefined)
}
