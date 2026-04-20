//! ParentNode mixin — `prepend` / `append` / `replaceChildren`
//! (WHATWG DOM §5.2.4).
//!
//! In the DOM spec, ParentNode is mixed into Element, Document, and
//! DocumentFragment.  In this implementation, these native fns are
//! currently installed only on `Element.prototype` and on the
//! document wrapper at bind time (Document has no shared prototype
//! the way Element does — its wrapper is patched per-bind by
//! [`install_document_methods_if_needed`](super::document)).
//! DocumentFragment wrappers still chain via `Node.prototype` and
//! therefore do not expose `prepend`/`append`/`replaceChildren`
//! yet; that gap lands together with the `DocumentFragment.prototype`
//! work in a later PR.
//!
//! Argument normalisation reuses
//! [`super::childnode::convert_nodes_to_single_node_or_fragment`] so
//! the Phase 2 (spec §4.2.5) rules are identical.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, PropertyKey, PropertyValue, VmError};
use super::super::{NativeFn, VmInner};
use super::childnode::{convert_nodes_to_single_node_or_fragment, finalize_pair};
use super::dom_bridge::nodes_to_insert;

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

/// Thin wrapper over [`super::dom_exception::hierarchy_request_error`]
/// that fills in the `'ParentNode'` interface label.  Mixin is
/// installed on `Element.prototype` and the document wrapper, so
/// `document.append(...)` and `element.prepend(...)` both surface
/// through this factory.
fn hierarchy_request_error(ctx: &NativeContext<'_>, method: &str) -> VmError {
    super::dom_exception::hierarchy_request_error(
        ctx.vm.well_known.dom_exc_hierarchy_request_error,
        "ParentNode",
        method,
        "the new child node cannot be inserted.",
    )
}

/// ParentNode mixin receivers per WHATWG §4.2.6 — Element,
/// Document, DocumentFragment.  DocumentFragment wrappers don't
/// currently receive the mixin install (the prototype install
/// happens on Element.prototype and the document wrapper only),
/// but a `Function.call` reroute can still hit these natives with
/// a Fragment receiver — accept it so we don't falsely throw.
fn is_parent_node_kind(k: NodeKind) -> bool {
    matches!(
        k,
        NodeKind::Element | NodeKind::Document | NodeKind::DocumentFragment
    )
}

fn native_parent_node_prepend(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "prepend",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Pre-insertion validity (WHATWG §4.2.3): reject ancestor
    // cycles and self-insert BEFORE mutating the tree so a throw
    // leaves the parent unchanged.  Same pattern as
    // `replaceChildren` / `replaceWith`.
    for &child in &children {
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, pair, false);
            return Err(hierarchy_request_error(ctx, "prepend"));
        }
    }
    // Track the "reference child" we insert before.  Starts as the
    // parent's current first child; if we'd insert a node as its own
    // reference, advance to that node's next sibling (WHATWG
    // pre-insert no-op).  Snapshotting is correct because
    // `insert_before` leaves the reference child's position intact
    // relative to nodes inserted in front of it.
    let mut ref_child = ctx.host().dom().children_iter(parent).next();
    let mut err = None;
    for child in children {
        if ref_child == Some(child) {
            ref_child = ctx.host().dom().get_next_sibling(child);
            continue;
        }
        let ok = match ref_child {
            Some(r) => ctx.host().dom().insert_before(parent, child, r),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error(ctx, "prepend"));
            break;
        }
    }
    finalize_pair(ctx, pair, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_parent_node_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "append",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Pre-insertion validity (WHATWG §4.2.3): cycle / self-insert
    // rejection BEFORE any mutation, matching the rest of the
    // ParentNode / ChildNode mixin.
    for &child in &children {
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, pair, false);
            return Err(hierarchy_request_error(ctx, "append"));
        }
    }
    let mut err = None;
    for child in children {
        if !ctx.host().dom().append_child(parent, child) {
            err = Some(hierarchy_request_error(ctx, "append"));
            break;
        }
    }
    finalize_pair(ctx, pair, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_parent_node_replace_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "replaceChildren",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    // WHATWG §4.2.3 order: "convert nodes into a node" → "ensure
    // pre-insertion validity" → "replace all".  If validity fails,
    // the replace-all step never runs, so the tree is unchanged.
    // Our earlier implementation inverted the validity check (tried
    // the insertion and rolled back on failure), which broke when an
    // argument was one of `parent`'s own children: normalisation
    // already reparented it into the wrapper fragment, so the
    // rollback snapshot was incomplete and the original tree could
    // not be restored.  We now validate up-front and only mutate
    // after we know every child is insertable.
    let pair = convert_nodes_to_single_node_or_fragment(ctx, args)?;
    if let Some(p) = pair {
        let to_insert = nodes_to_insert(ctx, p.0);
        // Pre-validate every leaf — `EcsDom::append_child` rejects
        // self-insert (`child == parent`) and ancestor cycles
        // (`child` is an ancestor of `parent`).  Checking with
        // `is_light_tree_ancestor_or_self` covers both in one call.
        for &child in &to_insert {
            if ctx
                .host()
                .dom()
                .is_light_tree_ancestor_or_self(child, parent)
            {
                finalize_pair(ctx, p, false);
                return Err(hierarchy_request_error(ctx, "replaceChildren"));
            }
        }
        let existing: Vec<Entity> = ctx.host().dom().children_iter(parent).collect();
        for child in existing {
            let _ = ctx.host().dom().remove_child(parent, child);
        }
        let mut err = None;
        for child in to_insert {
            // Pre-validation covers `EcsDom::append_child`'s documented
            // rejection modes (self-insert, ancestor cycle), so this
            // branch is unreachable under the current invariants.
            // Surface an error anyway as defence-in-depth: silently
            // dropping a child would hide any future ECS invariant
            // regression and still leave the parent half-populated,
            // which is worse than an explicit throw.
            if !ctx.host().dom().append_child(parent, child) {
                err = Some(hierarchy_request_error(ctx, "replaceChildren"));
                break;
            }
        }
        finalize_pair(ctx, p, err.is_none());
        if let Some(e) = err {
            return Err(e);
        }
    } else {
        // No args — clear the parent.
        let existing: Vec<Entity> = ctx.host().dom().children_iter(parent).collect();
        for child in existing {
            let _ = ctx.host().dom().remove_child(parent, child);
        }
    }
    Ok(JsValue::Undefined)
}
