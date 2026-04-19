//! ChildNode mixin — `before` / `after` / `replaceWith` / `remove`
//! (WHATWG DOM §5.2.2).
//!
//! Implemented by Element, CharacterData, and DocumentType — both
//! `Element.prototype` and `CharacterData.prototype` install these
//! same natives so the members land on Element, Text, Comment
//! wrappers simultaneously.
//!
//! # "Convert nodes into a node"
//!
//! Every variadic argument list goes through
//! [`convert_nodes_to_single_node_or_fragment`], matching WHATWG
//! §4.2.5 "convert nodes into a node":
//! - empty → `None` (no-op).
//! - single Node / DocumentFragment → `(entity, false)` —
//!   **DocumentFragment is not re-wrapped**.
//! - single primitive → a fresh Text node → `(text, false)`.
//! - 2+ arguments → new wrapper DocumentFragment with each argument
//!   converted and appended → `(fragment, true)`.  The caller destroys
//!   the wrapper once it has been consumed to prevent leaks.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::nodes_to_insert;
use super::event_target::entity_from_this;

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

// ---------------------------------------------------------------------------
// Argument normalisation — WHATWG §4.2.5 "convert nodes into a node"
// ---------------------------------------------------------------------------

/// Classify a single variadic argument, turning primitives into a
/// fresh Text node.  Returns the entity to use.
///
/// Non-Node non-primitive inputs (arbitrary object, `null`,
/// `undefined`) are coerced via ToString (matching browsers' lenient
/// behaviour on mixed-type arg lists).
fn normalize_single_arg(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<Entity, VmError> {
    if let JsValue::Object(id) = val {
        if let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind {
            if let Some(entity) = Entity::from_bits(entity_bits) {
                // Window and other non-Node EventTargets must not be
                // accepted here — treat them as text coercion below
                // via the fallthrough path.
                if !matches!(
                    ctx.host().dom().node_kind(entity),
                    None | Some(NodeKind::Window)
                ) {
                    return Ok(entity);
                }
            }
        }
    }
    // Coerce to string and allocate a Text node.
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    let text = ctx.host().dom().create_text(s);
    Ok(text)
}

/// Normalise a variadic argument list.  Returns `None` for empty
/// input, `Some((entity, was_wrapped))` otherwise — if
/// `was_wrapped == true`, the caller must destroy `entity` after
/// consuming it.
pub(super) fn convert_nodes_to_single_node_or_fragment(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<Option<(Entity, bool)>, VmError> {
    if args.is_empty() {
        return Ok(None);
    }
    if args.len() == 1 {
        return Ok(Some((normalize_single_arg(ctx, args[0])?, false)));
    }
    // Multi-arg: wrap in a freshly allocated DocumentFragment.
    let fragment = ctx.host().dom().create_document_fragment();
    for &arg in args {
        let child = normalize_single_arg(ctx, arg)?;
        let _ = ctx.host().dom().append_child(fragment, child);
    }
    Ok(Some((fragment, true)))
}

/// Consume the `(entity, was_wrapped)` pair after a ChildNode /
/// ParentNode mutation completes — destroys the wrapper fragment if
/// we allocated one AND it has no remaining children.
///
/// On the success path every child was detached from the wrapper by
/// the `append_child` / `insert_before` that moved it into the real
/// parent, so the fragment is empty and gets destroyed.  On error
/// paths (mutation loop aborted mid-way) the fragment may still hold
/// unmoved children; destroying it then would orphan those children
/// and leak them in the ECS world.  Leave the fragment intact in
/// that case — it becomes GC-unreachable from JS since the wrapper
/// cache never saw it.
pub(super) fn destroy_wrapper_fragment_if_any(ctx: &mut NativeContext<'_>, pair: (Entity, bool)) {
    let (entity, was_wrapped) = pair;
    if !was_wrapped {
        return;
    }
    let has_children = ctx.host().dom().children_iter(entity).next().is_some();
    if has_children {
        return;
    }
    let _ = ctx.host().dom().destroy_entity(entity);
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

/// Build the `HierarchyRequestError`-equivalent throw emitted when
/// `EcsDom` rejects an insertion (self-insert, ancestor cycle,
/// destroyed entity).  Matches the TypeError-surfaced pattern
/// established by `Node.appendChild` / `insertBefore` (DOMException
/// integration is deferred).  Uses `'ChildNode'` as the interface
/// label because this mixin is installed on both Element and
/// CharacterData wrappers.
fn hierarchy_request_error(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'ChildNode': \
         the new child node cannot be inserted."
    ))
}

fn native_child_node_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // Parent-less receiver is a no-op (spec).
    let Some(parent) = ctx.host().dom().get_parent(entity) else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    let mut err = None;
    for child in children {
        // WHATWG ChildNode.before: `el.before(el)` is a no-op per the
        // spec (the receiver would be its own "viable previous
        // sibling").  `EcsDom::insert_before` rejects
        // `new_child == ref_child`, so treat the self-reference as an
        // explicit skip instead of letting it surface as a throw.
        if child == entity {
            continue;
        }
        if !ctx.host().dom().insert_before(parent, child, entity) {
            err = Some(hierarchy_request_error("before"));
            break;
        }
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_child_node_after(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let Some(parent) = ctx.host().dom().get_parent(entity) else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Track the "viable next sibling" starting at `entity.nextSibling`.
    // Per WHATWG pre-insert: if we'd insert a node as its own
    // reference, advance the reference past it (no-op, preserves the
    // node's current position).
    let mut ref_next = ctx.host().dom().get_next_sibling(entity);
    let mut err = None;
    for child in children {
        if child == entity {
            continue;
        }
        if ref_next == Some(child) {
            ref_next = ctx.host().dom().get_next_sibling(child);
            continue;
        }
        let ok = match ref_next {
            Some(r) => ctx.host().dom().insert_before(parent, child, r),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error("after"));
            break;
        }
    }
    destroy_wrapper_fragment_if_any(ctx, pair);
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_child_node_replace_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let Some(parent) = ctx.host().dom().get_parent(entity) else {
        return Ok(JsValue::Undefined);
    };
    // Insert new nodes before `entity`, then remove `entity`.  This
    // is the simplest implementation of replaceWith that handles the
    // empty-args case (→ detach) correctly.
    let pair = convert_nodes_to_single_node_or_fragment(ctx, args)?;
    let mut err = None;
    if let Some(p) = pair {
        let children = nodes_to_insert(ctx, p.0);
        for child in children {
            if !ctx.host().dom().insert_before(parent, child, entity) {
                err = Some(hierarchy_request_error("replaceWith"));
                break;
            }
        }
        destroy_wrapper_fragment_if_any(ctx, p);
    }
    if let Some(e) = err {
        return Err(e);
    }
    let _ = ctx.host().dom().remove_child(parent, entity);
    Ok(JsValue::Undefined)
}

fn native_child_node_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let dom = ctx.host().dom();
    if let Some(parent) = dom.get_parent(entity) {
        let _ = dom.remove_child(parent, entity);
    }
    Ok(JsValue::Undefined)
}
