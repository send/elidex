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

/// Recursively flatten `node` into the list of real nodes to insert.
/// `DocumentFragment` at any depth expands to its light-tree
/// descendants; every other `NodeKind` becomes a single leaf entry.
///
/// **Side-effect free**: the walk reads children without mutating
/// the source tree.  Fragment emptying happens separately in
/// [`finalize_pair`] AFTER the insertion loop succeeds — draining
/// during the walk would orphan leaves whenever a pre-insertion
/// validity check (`replaceChildren` / `replaceWith`) later throws,
/// because the detach would already have happened.
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
    let children: Vec<Entity> = ctx.host().dom().children_iter(node).collect();
    for child in children {
        flatten_into(ctx, child, out);
    }
}

/// Recursively detach every `DocumentFragment` descendant of `root`
/// from its fragment parent.  Called on the **success path** of an
/// insertion to finalise WHATWG §4.2.3's "fragment becomes empty
/// after pre-insert" contract — leaves already moved during the
/// insert loop, this pass empties the intermediate fragment
/// scaffolding that the leaves were originally parented to.
///
/// Must NOT be called on an error path: some leaves may still be
/// linked to the fragment tree, and detaching their fragment
/// parents would leave them stranded in orphan fragments.
pub(super) fn drain_fragment_descendants(ctx: &mut NativeContext<'_>, root: Entity) {
    if !matches!(
        ctx.host().dom().node_kind(root),
        Some(NodeKind::DocumentFragment)
    ) {
        return;
    }
    let children: Vec<Entity> = ctx.host().dom().children_iter(root).collect();
    for child in children {
        if matches!(
            ctx.host().dom().node_kind(child),
            Some(NodeKind::DocumentFragment)
        ) {
            drain_fragment_descendants(ctx, child);
            let _ = ctx.host().dom().remove_child(root, child);
        }
        // Non-fragment (leaf) children shouldn't linger on the
        // success path — leaves already moved during the insert
        // loop.  If one does stay (e.g. the caller skipped a
        // leaf), leaving it attached is safer than an aggressive
        // detach.
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

/// Pre-order DFS collecting every descendant of `root` whose tag name
/// matches `tag` (ASCII case-insensitively), or every element when
/// `tag == "*"`.  `root` itself is never a candidate, matching the
/// WHATWG §4.2.6 scope rule ("every descendant element of node").
///
/// Shared by `document.getElementsByTagName` and
/// `Element.prototype.getElementsByTagName` so the two surfaces cannot
/// drift on `*` handling or case folding.
pub(super) fn collect_descendants_by_tag_name(
    dom: &EcsDom,
    root: Entity,
    tag: &str,
) -> Vec<Entity> {
    use elidex_ecs::TagType;
    let mut out = Vec::new();
    let match_all = tag == "*";
    dom.traverse_descendants(root, |entity| {
        if match_all {
            if dom.world().get::<&TagType>(entity).is_ok() {
                out.push(entity);
            }
        } else if let Ok(tt) = dom.world().get::<&TagType>(entity) {
            if tt.0.eq_ignore_ascii_case(tag) {
                out.push(entity);
            }
        }
        true
    });
    out
}

/// Pre-order DFS collecting every descendant of `root` whose `class`
/// attribute contains all tokens in `target_classes`.  Empty
/// `target_classes` produces an empty Vec per WHATWG §4.2.6.2 step 2
/// ("if classes is the empty set, return an empty HTMLCollection").
/// `root` itself is never a match candidate.
pub(super) fn collect_descendants_by_class_name(
    dom: &EcsDom,
    root: Entity,
    target_classes: &[&str],
) -> Vec<Entity> {
    use elidex_ecs::Attributes;
    if target_classes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            if let Some(cls) = attrs.get("class") {
                if target_classes
                    .iter()
                    .all(|tc| cls.split_whitespace().any(|ec| ec == *tc))
                {
                    out.push(entity);
                }
            }
        }
        true
    });
    out
}
