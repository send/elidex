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

/// Classification of a variadic argument after a side-effect-free
/// validation pass.  A second materialisation pass consumes the Vec
/// to allocate Text children + build the wrapper fragment.
enum ClassifiedArg {
    Node(Entity),
    Text(super::super::value::StringId),
}

/// Side-effect-free classification of a single arg.  Throws on
/// detached Node wrappers and on Symbol ToString, without ever
/// detaching / allocating.
fn classify_arg_without_mutation(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<ClassifiedArg, VmError> {
    if let JsValue::Object(id) = val {
        if let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind {
            if let Some(entity) = Entity::from_bits(entity_bits) {
                if !ctx.host().dom().contains(entity) {
                    return Err(VmError::type_error(
                        "Failed to execute argument conversion: \
                         the node is detached (invalid entity).",
                    ));
                }
                let inferred = ctx.host().dom().node_kind_inferred(entity);
                if matches!(inferred, Some(k) if k != NodeKind::Window) {
                    return Ok(ClassifiedArg::Node(entity));
                }
            }
        }
    }
    // Primitive / coercible to string — `to_string` may throw on
    // Symbol.  Doing this in the pre-validation pass forces the
    // throw to happen BEFORE any user Node is detached into a
    // wrapper fragment (WHATWG §4.2.5 semantics — the "convert
    // nodes into a node" algorithm validates before re-parenting).
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    Ok(ClassifiedArg::Text(sid))
}

/// Normalise a variadic argument list.  Returns `None` for empty
/// input, `Some((entity, was_wrapped))` otherwise — if
/// `was_wrapped == true`, the caller must destroy `entity` after
/// consuming it.
///
/// Multi-arg path is **side-effect-free until every arg is
/// validated**: a pre-pass classifies each arg (detached check,
/// ToString coerce, Symbol throw) without allocating a
/// DocumentFragment or detaching any user Node.  Only after the
/// pre-pass succeeds do we allocate the fragment and move children
/// into it.  This guarantees that a throw during conversion cannot
/// strand a user-supplied Node inside a fragment that's about to be
/// destroyed (silent data-loss scenario).
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
    // Pre-validation pass — no allocation, no tree mutation.
    let mut classified = Vec::with_capacity(args.len());
    for &arg in args {
        classified.push(classify_arg_without_mutation(ctx, arg)?);
    }
    // Materialisation pass — every arg has passed validation; the
    // only remaining failure mode is EcsDom rejecting
    // `append_child`, which would be a genuine DOM invariant
    // violation (fragment was just allocated, user nodes already
    // checked).  Still cleaned up defensively.
    let fragment = ctx.host().dom().create_document_fragment();
    for c in classified {
        let child = match c {
            ClassifiedArg::Node(e) => e,
            ClassifiedArg::Text(sid) => {
                let s = ctx.vm.strings.get_utf8(sid);
                ctx.host().dom().create_text(&s)
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

/// Thin wrapper over [`super::dom_exception::hierarchy_request_error`]
/// that fills in the `'ChildNode'` interface label.  This mixin is
/// installed on both Element and CharacterData wrappers, so the
/// interface surface is `ChildNode` regardless of concrete receiver.
fn hierarchy_request_error(ctx: &NativeContext<'_>, method: &str) -> VmError {
    super::dom_exception::hierarchy_request_error(
        ctx.vm.well_known.dom_exc_hierarchy_request_error,
        "ChildNode",
        method,
        "the new child node cannot be inserted.",
    )
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
    // Snapshot ALL of `entity`'s preceding siblings BEFORE
    // argument normalisation.  WHATWG §5.2.2 defines
    // viablePreviousSibling as "last preceding sibling of this not
    // in nodes" — computed against the ORIGINAL sibling chain, not
    // whatever state `parent`'s children end up in after
    // `convert_nodes_to_single_node_or_fragment` detaches some of
    // them into a wrapper fragment.
    let preceding_siblings: Vec<Entity> = {
        let dom = ctx.host().dom();
        let mut out: Vec<Entity> = Vec::new();
        for sib in dom.children_iter(parent) {
            if sib == entity {
                break;
            }
            out.push(sib);
        }
        out
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Pre-insertion validity (WHATWG §4.2.3): reject ancestor
    // cycles / self-insert-into-parent BEFORE any mutation.  The
    // receiver itself (`child == entity`) is a valid arg — it
    // just re-inserts at its original position via the
    // `child == anchor` advance below.
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
            return Err(hierarchy_request_error(ctx, "before"));
        }
    }
    // viablePreviousSibling = last snapshotted preceding sibling
    // that isn't being re-inserted.  Insertion anchor follows the
    // spec: `viablePreviousSibling.nextSibling` if it's defined,
    // otherwise `parent`'s first child.  Reading nextSibling in
    // the CURRENT (post-normalisation) tree is correct because
    // viable_prev itself is not in children → it's still attached
    // to parent → its nextSibling reflects the post-detach state
    // we're inserting into.
    let viable_prev = preceding_siblings
        .iter()
        .rev()
        .find(|&&sib| !children.iter().any(|&c| c == sib))
        .copied();
    let mut anchor = match viable_prev {
        Some(prev) => ctx.host().dom().get_next_sibling(prev),
        None => ctx.host().dom().children_iter(parent).next(),
    };
    let mut err = None;
    for child in children {
        // WHATWG "insert" step 2: if referenceChild is node,
        // advance it to node.nextSibling.  Converts
        // `insert_before(parent, x, x)` (which `EcsDom` rejects)
        // into a no-op in-place move.
        if Some(child) == anchor {
            anchor = ctx.host().dom().get_next_sibling(child);
        }
        let ok = match anchor {
            Some(r) => ctx.host().dom().insert_before(parent, child, r),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error(ctx, "before"));
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
            return Err(hierarchy_request_error(ctx, "after"));
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
            err = Some(hierarchy_request_error(ctx, "after"));
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
            return Err(hierarchy_request_error(ctx, "replaceWith"));
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
            err = Some(hierarchy_request_error(ctx, "replaceWith"));
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
