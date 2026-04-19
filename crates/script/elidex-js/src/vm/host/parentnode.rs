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
use super::dom_bridge::nodes_to_insert;
use super::event_target::entity_from_this;

use elidex_ecs::Entity;

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

/// TypeError-surfaced HierarchyRequestError for the ParentNode
/// mixin — mirrors the pattern `Node.appendChild` / `insertBefore`
/// use (DOMException integration is deferred).  Uses `'ParentNode'`
/// as the interface label because this mixin is installed on both
/// `Element.prototype` and the document wrapper (so the method can
/// throw for `document.append(...)` too).
fn hierarchy_request_error(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'ParentNode': \
         the new child node cannot be inserted."
    ))
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
    let mut err = None;
    for child in children {
        let ok = match first {
            Some(f) => ctx.host().dom().insert_before(parent, child, f),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error("prepend"));
            break;
        }
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    err.map_or(Ok(JsValue::Undefined), Err)
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
    let mut err = None;
    for child in children {
        if !ctx.host().dom().append_child(parent, child) {
            err = Some(hierarchy_request_error("append"));
            break;
        }
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_parent_node_replace_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // WHATWG §4.2.3: convert the variadic arguments BEFORE clearing
    // the parent so a ToString / HierarchyRequestError throw leaves
    // the tree untouched.  Mirror `replaceChildren` step 3 of the
    // "replace all" algorithm.
    let pair = convert_nodes_to_single_node_or_fragment(ctx, args)?;
    let existing: Vec<Entity> = ctx.host().dom().children_iter(parent).collect();
    for child in existing {
        let _ = ctx.host().dom().remove_child(parent, child);
    }
    let mut err = None;
    if let Some(p) = pair {
        let children = nodes_to_insert(ctx, p.0);
        for child in children {
            if !ctx.host().dom().append_child(parent, child) {
                err = Some(hierarchy_request_error("replaceChildren"));
                break;
            }
        }
        destroy_wrapper_fragment_if_any(ctx, p);
    }
    err.map_or(Ok(JsValue::Undefined), Err)
}
