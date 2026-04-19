//! ChildNode mixin — `before` / `after` / `replaceWith` / `remove`
//! (WHATWG DOM §5.2.2).
//!
//! The mixin is installed on `Element.prototype` and
//! `CharacterData.prototype`, so Element, Text, and Comment wrappers
//! share these natives.  WHATWG also defines the mixin on
//! `DocumentType`, but elidex has no JS surface for creating
//! DocumentType wrappers yet — install on `DocumentType.prototype`
//! lands alongside `document.doctype` / `DOMImplementation`.
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
                // A HostObject whose `entity_bits` decodes but no
                // longer exists in the DOM world is a detached /
                // destroyed node — matches `require_node_arg`'s
                // "detached (invalid entity)" brand failure.
                // Silently coercing a stale Node wrapper to a Text
                // node would hide the bug.
                if !ctx.host().dom().contains(entity) {
                    return Err(VmError::type_error(
                        "Failed to execute argument conversion: \
                         the node is detached (invalid entity).",
                    ));
                }
                // Accept any DOM node (including legacy entities
                // missing `NodeKind` but carrying DOM payload) via
                // `node_kind_inferred`.  Window is an `EventTarget`
                // but not a Node in WHATWG so it falls through to
                // text coercion.
                let inferred = ctx.host().dom().node_kind_inferred(entity);
                if matches!(inferred, Some(k) if k != NodeKind::Window) {
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
    // Multi-arg: wrap in a freshly allocated DocumentFragment.  The
    // fragment is fresh and the children are either freshly-created
    // Text nodes or user-supplied Node entities, so `append_child`
    // is expected to succeed — but we surface a TypeError on failure
    // so the caller never silently drops arguments.  On error we
    // also destroy the half-built wrapper to prevent ECS entity
    // leaks.
    let fragment = ctx.host().dom().create_document_fragment();
    for &arg in args {
        let child = match normalize_single_arg(ctx, arg) {
            Ok(c) => c,
            Err(e) => {
                let _ = ctx.host().dom().destroy_entity(fragment);
                return Err(e);
            }
        };
        if !ctx.host().dom().append_child(fragment, child) {
            let _ = ctx.host().dom().destroy_entity(fragment);
            return Err(VmError::type_error(
                "Failed to build wrapper DocumentFragment: \
                 argument rejected by the DOM.",
            ));
        }
    }
    Ok(Some((fragment, true)))
}

/// Consume the `(entity, was_wrapped)` pair after a ChildNode /
/// ParentNode mutation completes.
///
/// `succeeded = true` (every insertion landed): drain every
/// `DocumentFragment` descendant from `entity` (finalising WHATWG
/// §4.2.3 "fragment becomes empty after pre-insert" for both the
/// wrapper and any user-supplied nested fragments), then destroy
/// the wrapper if we allocated it.
///
/// `succeeded = false` (an insert aborted mid-loop): skip the drain
/// — unmoved leaves may still be parented to fragments and
/// detaching those fragments would strand the leaves in an orphan
/// fragment.  We still destroy the wrapper if it happens to be
/// empty (best-effort cleanup that can't strand anything).
pub(super) fn finalize_pair(ctx: &mut NativeContext<'_>, pair: (Entity, bool), succeeded: bool) {
    let (entity, was_wrapped) = pair;
    if succeeded {
        super::dom_bridge::drain_fragment_descendants(ctx, entity);
    }
    if !was_wrapped {
        return;
    }
    if ctx.host().dom().children_iter(entity).next().is_some() {
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
    let Some(entity) = super::event_target::require_receiver(
        ctx,
        this,
        "ChildNode",
        "before",
        is_child_node_kind,
    )?
    else {
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
    // Pre-insertion validity (WHATWG §4.2.3): reject ancestor
    // cycles / self-insert-into-parent BEFORE any mutation.  The
    // `child == entity` no-op is allowed — inserting the receiver
    // before itself leaves the tree unchanged and is NOT an
    // ancestor-of-parent case (entity is parent's child, not
    // parent's ancestor).
    for &child in &children {
        if child == entity {
            continue;
        }
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, pair, false);
            return Err(hierarchy_request_error("before"));
        }
    }
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
    finalize_pair(ctx, pair, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_child_node_after(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "ChildNode", "after", is_child_node_kind)?
    else {
        return Ok(JsValue::Undefined);
    };
    let Some(parent) = ctx.host().dom().get_parent(entity) else {
        return Ok(JsValue::Undefined);
    };
    // Snapshot ALL of `entity`'s following siblings BEFORE
    // argument normalisation.  WHATWG §5.2.2 defines
    // viableNextSibling as "first following sibling of this not in
    // nodes" — computed against the ORIGINAL sibling chain, not
    // whatever state `parent`'s children are in after
    // `convert_nodes_to_single_node_or_fragment` has detached some
    // of them into a wrapper fragment.
    let following_siblings: Vec<Entity> = {
        let dom = ctx.host().dom();
        let mut out = Vec::new();
        let mut cur = dom.get_next_sibling(entity);
        while let Some(sib) = cur {
            out.push(sib);
            cur = dom.get_next_sibling(sib);
        }
        out
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Pre-insertion validity (WHATWG §4.2.3): cycle / self-insert
    // rejection BEFORE any mutation, mirroring `before` above.
    for &child in &children {
        if child == entity {
            continue;
        }
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, pair, false);
            return Err(hierarchy_request_error("after"));
        }
    }
    // viableNextSibling = first sibling in the pre-normalisation
    // snapshot that is NOT one of the nodes being inserted.
    // Append (viableNextSibling == None) otherwise.
    let viable_next = following_siblings
        .iter()
        .find(|&&sib| !children.iter().any(|&c| c == sib))
        .copied();
    let mut err = None;
    for child in children {
        if child == entity {
            continue;
        }
        let ok = match viable_next {
            Some(r) => ctx.host().dom().insert_before(parent, child, r),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error("after"));
            break;
        }
    }
    finalize_pair(ctx, pair, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_child_node_replace_with(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = super::event_target::require_receiver(
        ctx,
        this,
        "ChildNode",
        "replaceWith",
        is_child_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    let Some(parent) = ctx.host().dom().get_parent(entity) else {
        return Ok(JsValue::Undefined);
    };
    // Snapshot ALL of `entity`'s following siblings BEFORE
    // argument normalisation — see the `after` rationale for why
    // we cannot walk `get_next_sibling(cand)` after normalisation
    // (normalisation detaches Node args into a wrapper fragment
    // so the chain read off `cand` would follow the fragment's
    // child list, not `parent`'s original children).
    let following_siblings: Vec<Entity> = {
        let dom = ctx.host().dom();
        let mut out = Vec::new();
        let mut cur = dom.get_next_sibling(entity);
        while let Some(sib) = cur {
            out.push(sib);
            cur = dom.get_next_sibling(sib);
        }
        out
    };
    // WHATWG DOM §5.2.2 `replaceWith`:
    // 1. viableNextSibling = first following sibling of `this` not
    //    in `nodes`; otherwise null.
    // 2. Let node = "convert nodes into a node".
    // 3. Remove `this`.
    // 4. Insert node into parent before viableNextSibling.
    //
    // The spec's remove-then-insert order is what makes
    // `node.replaceWith(node)` a no-op: `node` is detached by step 3
    // then re-inserted at its original position (viableNextSibling)
    // in step 4.  Inserting first would trip
    // `EcsDom::insert_before`'s `new_child == ref_child` rejection
    // and throw.
    let pair = convert_nodes_to_single_node_or_fragment(ctx, args)?;
    let Some(p) = pair else {
        // Zero args: detach only.
        let _ = ctx.host().dom().remove_child(parent, entity);
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, p.0);
    // Pre-insertion validity BEFORE detaching `entity`: reject
    // ancestor cycles / self-insert so the throw path leaves the
    // tree untouched (WHATWG §5.2.2 step 3 runs "ensure pre-
    // insertion validity" before the remove+insert in step 5).
    // Same pattern as `replaceChildren` after R12 F3+F4 —
    // inserting first and rolling back loses nodes that argument
    // normalisation already detached.
    for &child in &children {
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, p, false);
            return Err(hierarchy_request_error("replaceWith"));
        }
    }
    // viableNextSibling = first sibling in the pre-normalisation
    // snapshot that is NOT one of the nodes being inserted.
    let viable_next = following_siblings
        .iter()
        .find(|&&sib| !children.iter().any(|&c| c == sib))
        .copied();
    let _ = ctx.host().dom().remove_child(parent, entity);
    let mut err = None;
    for child in children {
        let ok = match viable_next {
            Some(r) => ctx.host().dom().insert_before(parent, child, r),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error("replaceWith"));
            break;
        }
    }
    finalize_pair(ctx, p, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
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
    let dom = ctx.host().dom();
    if let Some(parent) = dom.get_parent(entity) {
        let _ = dom.remove_child(parent, entity);
    }
    Ok(JsValue::Undefined)
}
