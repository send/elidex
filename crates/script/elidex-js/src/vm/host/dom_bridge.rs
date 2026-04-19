//! Helpers shared across host-side DOM natives — wrapper lifting
//! and selector parsing.
//!
//! These existed as file-local `fn`s in `document.rs` and
//! `element_proto.rs` before they grew a second consumer.  Keeping
//! them in one place avoids the near-identical copies drifting over
//! time (each had seven call sites between the two files).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, VmError};
use super::super::VmInner;
use super::event_target::entity_from_this;

use elidex_css::{parse_selector_from_str, Selector};
use elidex_ecs::{EcsDom, Entity, NodeKind};

/// Return `Option<Entity>` as a JS wrapper or `null` — no intermediate
/// `ObjectId`, so callers can chain it straight into a `Result::Ok`.
pub(super) fn wrap_entity_or_null(vm: &mut VmInner, entity: Option<Entity>) -> JsValue {
    match entity {
        Some(e) => JsValue::Object(vm.create_element_wrapper(e)),
        None => JsValue::Null,
    }
}

/// Wrap a list of entities as a JS Array of element wrappers.  One
/// allocation for the intermediate `Vec<JsValue>`, one for the
/// Array object.
pub(super) fn wrap_entities_as_array(vm: &mut VmInner, entities: &[Entity]) -> JsValue {
    let elements: Vec<JsValue> = entities
        .iter()
        .map(|&e| JsValue::Object(vm.create_element_wrapper(e)))
        .collect();
    JsValue::Object(vm.create_array_object(elements))
}

/// Parse a selector string and reject shadow-scoped pseudos.  Shared
/// by `document.querySelector*` and `Element.prototype.matches` /
/// `closest` — all four throw `SyntaxError` on invalid input and on
/// `:host` / `::slotted()`, which are only valid inside shadow-tree
/// context.
///
/// The `method` name appears in the shadow-pseudo error message so
/// callers get a call-site-accurate complaint (`… are not valid in
/// querySelector` vs `… in matches/closest`).
pub(super) fn parse_dom_selector(
    selector_str: &str,
    shadow_method_label: &str,
) -> Result<Vec<Selector>, VmError> {
    let selectors = parse_selector_from_str(selector_str)
        .map_err(|()| VmError::syntax_error(format!("Invalid selector: {selector_str}")))?;
    if selectors.iter().any(|s| s.has_shadow_pseudo()) {
        return Err(VmError::syntax_error(format!(
            ":host and ::slotted() are not valid in {shadow_method_label}"
        )));
    }
    Ok(selectors)
}

/// Coerce the first argument to a string and hand back its UTF-8
/// materialisation — the shape every selector-accepting native
/// (querySelector, matches, closest, …) starts with.
pub(super) fn coerce_first_arg_to_string(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<String, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

/// Shared body for every "map `this` through one `EcsDom` tree-nav
/// accessor and wrap-or-null" native — extracts the receiver entity,
/// runs `lookup` against the bound DOM, and lifts the result to a
/// wrapper (or `null`).  The unbound-receiver path returns `null`.
///
/// Used by both `Element.prototype` (ParentNode / sibling accessors)
/// and `Node.prototype` (parentNode / firstChild / nextSibling / …).
pub(super) fn tree_nav_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    lookup: impl FnOnce(&EcsDom, Entity) -> Option<Entity>,
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let target = lookup(ctx.host().dom(), entity);
    Ok(wrap_entity_or_null(ctx.vm, target))
}

/// Pre-order DFS over descendants of `root` looking for the first
/// element that matches any selector in `selectors`.  `root` itself is
/// **not** a match candidate — WHATWG §4.2.6 step 3.  Returns the
/// matched entity, or `None` if none found.
///
/// Shared by both `document.querySelector` and
/// `Element.prototype.querySelector`.
pub(super) fn query_selector_in_subtree_first(
    dom: &EcsDom,
    root: Entity,
    selectors: &[elidex_css::Selector],
) -> Option<Entity> {
    use elidex_ecs::TagType;
    let mut result = None;
    dom.traverse_descendants(root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|s| s.matches(entity, dom))
        {
            result = Some(entity);
            false
        } else {
            true
        }
    });
    result
}

/// Recursively flatten `node` into the list of real nodes to insert,
/// draining every `DocumentFragment` encountered so that every
/// traversed fragment ends up empty.  Non-fragment nodes are kept
/// attached to their current parent — `EcsDom::append_child` /
/// `insert_before` at insertion time reparents them.
///
/// **Not side-effect free**: fragment-to-child links are severed
/// during the walk.  This matches WHATWG §4.2.3 step 6 — after a
/// successful pre-insert, the fragment argument (and every nested
/// fragment reachable from it) becomes empty.  A purely lazy flatten
/// would leave an intermediate fragment parented to its outer
/// fragment even after its leaves moved, producing non-empty
/// `outer.childNodes` for a nested argument — a spec-visible
/// regression.
///
/// Failure mode: if a later `append_child` / `insert_before` call
/// fails, the leaves already drained are now orphan nodes (detached
/// but still reachable from user JS).  This matches the WHATWG
/// "insert" algorithm, which empties the fragment at step 4.1 before
/// the per-node append at step 6 — a partial failure there leaves
/// the fragment empty and some leaves detached.
pub(super) fn nodes_to_insert(ctx: &mut NativeContext<'_>, node: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    flatten_into(ctx, node, &mut out);
    out
}

fn flatten_into(ctx: &mut NativeContext<'_>, node: Entity, out: &mut Vec<Entity>) {
    if !matches!(
        ctx.host().dom().node_kind(node),
        Some(NodeKind::DocumentFragment)
    ) {
        out.push(node);
        return;
    }
    // Drain the fragment: snapshot children, detach each from
    // `node`, then recurse.  By the time we return, `node` is empty
    // (including any nested fragment `node` contained, because
    // each nested fragment recurses into the same drain path).
    let children: Vec<Entity> = ctx.host().dom().children_iter(node).collect();
    for child in children {
        let _ = ctx.host().dom().remove_child(node, child);
        flatten_into(ctx, child, out);
    }
}

/// Pre-order DFS collecting every descendant of `root` matching any
/// selector in `selectors`.  `root` itself is not a match candidate.
pub(super) fn query_selector_in_subtree_all(
    dom: &EcsDom,
    root: Entity,
    selectors: &[elidex_css::Selector],
) -> Vec<Entity> {
    use elidex_ecs::TagType;
    let mut out = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|s| s.matches(entity, dom))
        {
            out.push(entity);
        }
        true
    });
    out
}
