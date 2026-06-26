//! ChildNode/ParentNode mixin handlers, pre-insertion validation, and Element
//! selector methods (`matches`, `closest`).
//!
//! Implements WHATWG DOM Living Standard:
//! - `ChildNode` mixin (before, after, remove, replaceWith)
//! - `ParentNode` mixin (prepend, append, replaceChildren)
//! - Element selector methods (matches, closest)
//! - Pre-insertion validation (WHATWG DOM §4.2.3 Mutation algorithms)

mod mutations;
mod selectors;

pub use mutations::{
    After, Append, Before, ChildNodeRemove, Prepend, ReplaceChildren, ReplaceWith,
};
pub use selectors::{Closest, Matches};

use elidex_ecs::{EcsDom, Entity, NodeKind, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    convert_arg_source_records, DomApiError, DomApiErrorKind, JsObjectRef, SessionCore,
};

use crate::util::not_found_error;

// ---------------------------------------------------------------------------
// Pre-insertion validation (WHATWG DOM §4.2.3 Mutation algorithms)
// ---------------------------------------------------------------------------

/// Hierarchy request error helper.
fn hierarchy_error(message: &str) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::HierarchyRequestError,
        message: message.into(),
    }
}

/// Validate parent node kind (step 1 shared by pre-insertion and replace).
fn validate_parent_kind(parent: Entity, dom: &EcsDom) -> Result<Option<NodeKind>, DomApiError> {
    let parent_kind = dom.node_kind(parent);
    match parent_kind {
        Some(NodeKind::Document | NodeKind::DocumentFragment | NodeKind::Element) => {}
        _ => {
            if dom.world().get::<&TagType>(parent).is_err() {
                return Err(hierarchy_error(
                    "parent must be Document, DocumentFragment, or Element",
                ));
            }
        }
    }
    Ok(parent_kind)
}

/// Validate node type (step 4 shared by pre-insertion and replace).
fn validate_node_type(node: Entity, dom: &EcsDom) -> Result<Option<NodeKind>, DomApiError> {
    let node_kind = dom.node_kind(node);
    match node_kind {
        Some(
            NodeKind::DocumentFragment
            | NodeKind::DocumentType
            | NodeKind::Element
            | NodeKind::Text
            | NodeKind::ProcessingInstruction
            | NodeKind::Comment
            | NodeKind::CdataSection,
        ) => {}
        None => {
            if dom.world().get::<&TagType>(node).is_err()
                && dom.world().get::<&TextContent>(node).is_err()
            {
                return Err(hierarchy_error("node type not allowed for insertion"));
            }
        }
        _ => return Err(hierarchy_error("node type not allowed for insertion")),
    }
    Ok(node_kind)
}

/// Validate `DocumentType` and `Text` placement (step 5 shared).
fn validate_doctype_and_text(
    parent_kind: Option<NodeKind>,
    node_kind: Option<NodeKind>,
    node: Entity,
    dom: &EcsDom,
) -> Result<(), DomApiError> {
    if matches!(node_kind, Some(NodeKind::DocumentType))
        && !matches!(parent_kind, Some(NodeKind::Document))
    {
        return Err(hierarchy_error(
            "DocumentType can only be a child of a Document",
        ));
    }
    if matches!(parent_kind, Some(NodeKind::Document)) {
        let is_text = matches!(node_kind, Some(NodeKind::Text))
            || dom.world().get::<&TextContent>(node).is_ok();
        if is_text {
            return Err(hierarchy_error("cannot insert Text node under Document"));
        }
    }
    Ok(())
}

/// Check if a doctype child follows `child` in parent's child list.
fn has_doctype_following(_parent: Entity, child: Option<Entity>, dom: &EcsDom) -> bool {
    let Some(ref_child) = child else {
        return false;
    };
    let mut cursor = dom.get_next_sibling(ref_child);
    while let Some(sib) = cursor {
        if matches!(dom.node_kind(sib), Some(NodeKind::DocumentType)) {
            return true;
        }
        cursor = dom.get_next_sibling(sib);
    }
    false
}

/// Check if an element child precedes `child` in parent's child list.
fn has_element_preceding(parent: Entity, child: Option<Entity>, dom: &EcsDom) -> bool {
    let Some(ref_child) = child else {
        // No reference child — check if any element exists among parent's children.
        return dom.children_iter(parent).any(|c| dom.is_element(c));
    };
    let mut cursor = dom.get_prev_sibling(ref_child);
    while let Some(sib) = cursor {
        if dom.is_element(sib) {
            return true;
        }
        cursor = dom.get_prev_sibling(sib);
    }
    false
}

/// Enforce Document parent child constraints (step 6 of WHATWG DOM §4.2.3).
///
/// `child` is the reference child (for pre-insert) or the child being replaced.
/// `exclude` is an additional entity to skip when counting existing children
/// (used by replace to exclude the child being replaced).
fn validate_document_element_constraint(
    parent: Entity,
    node: Entity,
    node_kind: Option<NodeKind>,
    child: Option<Entity>,
    exclude: &[Entity],
    dom: &EcsDom,
) -> Result<(), DomApiError> {
    match node_kind {
        Some(NodeKind::Element) => {
            // §4.2.3 step 6 Element: "parent has an element child" (pre-insert,
            // `exclude` = «») / "...that is not child" (replace, `exclude` = «child»).
            // The node being inserted is NOT self-excluded — re-appending the existing
            // documentElement (`document.appendChild(document.documentElement)`)
            // therefore throws, because pre-insertion validity runs BEFORE the
            // pre-insert self-reference adjustment (pre-insert step 3) and the
            // documentElement is still an element child of the document when validity
            // runs. Only `exclude` (the replaced child) is skipped, never `node`
            // ("the above statements differ from the pre-insert algorithm"). [Codex
            // B1.2b-3 R1 caught the prior `c != node` self-exclusion as spec-wrong.]
            let has_element = dom
                .children_iter(parent)
                .any(|c| dom.is_element(c) && !exclude.contains(&c));
            if has_element {
                return Err(hierarchy_error("Document already has an element child"));
            }
            // child is a doctype, or a doctype follows child?
            if let Some(ref_child) = child {
                if matches!(dom.node_kind(ref_child), Some(NodeKind::DocumentType))
                    && !exclude.contains(&ref_child)
                {
                    return Err(hierarchy_error(
                        "cannot insert Element before a DocumentType",
                    ));
                }
            }
            if has_doctype_following(parent, child, dom) {
                return Err(hierarchy_error(
                    "cannot insert Element before a DocumentType",
                ));
            }
        }
        Some(NodeKind::DocumentFragment) => {
            // Fragment must not contain Text children under Document parent.
            let has_text = dom
                .children_iter(node)
                .any(|c| matches!(dom.node_kind(c), Some(NodeKind::Text)));
            if has_text {
                return Err(hierarchy_error(
                    "DocumentFragment has Text child nodes; cannot insert under Document",
                ));
            }
            let elem_count = dom
                .children_iter(node)
                .filter(|c| dom.is_element(*c))
                .count();
            if elem_count > 1 {
                return Err(hierarchy_error(
                    "DocumentFragment has more than one element child for Document parent",
                ));
            }
            if elem_count == 1 {
                // Same "parent has an element child (that is not `child`)" test as the
                // Element branch — `exclude` carries the replaced child; the inserted
                // node is never self-excluded (a fragment is not itself a child of
                // `parent`, so this only differs for the Element branch in practice).
                let has_element = dom
                    .children_iter(parent)
                    .any(|c| dom.is_element(c) && !exclude.contains(&c));
                if has_element {
                    return Err(hierarchy_error("Document already has an element child"));
                }
                if let Some(ref_child) = child {
                    if matches!(dom.node_kind(ref_child), Some(NodeKind::DocumentType))
                        && !exclude.contains(&ref_child)
                    {
                        return Err(hierarchy_error(
                            "cannot insert Element before a DocumentType",
                        ));
                    }
                }
                if has_doctype_following(parent, child, dom) {
                    return Err(hierarchy_error(
                        "cannot insert Element before a DocumentType",
                    ));
                }
            }
        }
        Some(NodeKind::DocumentType) => {
            // §4.2.3 step 6 DocumentType: "parent has a doctype child" (pre-insert) /
            // "...that is not child" (replace, via `exclude`). No self-exclusion of the
            // inserted node (mirrors the Element branch) — re-appending an existing
            // doctype throws, same as the documentElement case above.
            let has_doctype = dom.children_iter(parent).any(|c| {
                !exclude.contains(&c) && matches!(dom.node_kind(c), Some(NodeKind::DocumentType))
            });
            if has_doctype {
                return Err(hierarchy_error("Document already has a DocumentType child"));
            }
            // An element child precedes the reference child?
            if has_element_preceding(parent, child, dom) {
                return Err(hierarchy_error(
                    "cannot insert DocumentType after an Element",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validate that a pre-insertion operation is permitted per WHATWG DOM §4.2.3.
pub(crate) fn ensure_pre_insertion_validity(
    parent: Entity,
    node: Entity,
    child: Option<Entity>,
    dom: &EcsDom,
) -> Result<(), DomApiError> {
    let parent_kind = validate_parent_kind(parent, dom)?;

    // Step 2: node must not be a host-including inclusive ancestor of parent.
    if dom.is_host_including_ancestor_or_self(node, parent) {
        return Err(hierarchy_error(
            "node is an ancestor of parent (would create cycle)",
        ));
    }

    // Step 3: child must be a child of parent.
    if let Some(ref_child) = child {
        if dom.get_parent(ref_child) != Some(parent) {
            return Err(not_found_error("reference child is not a child of parent"));
        }
    }

    let node_kind = validate_node_type(node, dom)?;
    validate_doctype_and_text(parent_kind, node_kind, node, dom)?;

    // Step 6: Document parent element constraints.
    if matches!(parent_kind, Some(NodeKind::Document)) {
        validate_document_element_constraint(parent, node, node_kind, child, &[], dom)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Node conversion helpers
// ---------------------------------------------------------------------------

/// Convert `JsValue` arguments into entities.
///
/// - `String` values are converted to new text nodes.
/// - `ObjectRef` values are resolved via the session identity map.
/// - `Null` and `Undefined` values are skipped.
pub(crate) fn collect_nodes(
    args: &[JsValue],
    session: &mut SessionCore,
    dom: &mut EcsDom,
) -> Result<Vec<Entity>, DomApiError> {
    let mut entities = Vec::with_capacity(args.len());
    for arg in args {
        match arg {
            JsValue::String(s) => {
                let text_node = dom.create_text(s.as_str());
                entities.push(text_node);
            }
            JsValue::ObjectRef(id) => {
                let (entity, _) = session
                    .identity_map()
                    .get(JsObjectRef::from_raw(*id))
                    .ok_or_else(|| not_found_error("node not found in identity map"))?;
                // A ShadowRoot is bound to its host by the immovable host edge (§4.8)
                // and is not a movable node. Reject it HERE — before `convert_nodes_into_node`
                // wraps the variadic args via the raw, non-expanding `EcsDom::append_child`,
                // which would reparent the shadow root into the temporary fragment and on
                // into the destination (the `apply_*` ShadowRoot guard only sees the temp
                // fragment, not its shadow-root child). `collect_nodes` is the shared
                // engine-independent resolution point for every ChildNode/ParentNode mixin
                // arg (VM + boa + wasm), so the rejection covers all runtimes. (Codex PR393 R6.)
                if dom.is_shadow_root(entity) {
                    return Err(hierarchy_error(
                        "a ShadowRoot cannot be inserted into the light DOM",
                    ));
                }
                entities.push(entity);
            }
            JsValue::Null | JsValue::Undefined => {}
            other => {
                // Coerce other primitives to string (matching DOMString IDL).
                let text_node = dom.create_text(other.to_string());
                entities.push(text_node);
            }
        }
    }
    Ok(entities)
}

/// If there is a single node, return it directly. If multiple, wrap them in a
/// `DocumentFragment` and return that.
///
/// **Side effects**: When multiple nodes are provided, all are reparented under
/// a newly created `DocumentFragment`. Callers must compute any position-dependent
/// values (e.g., `viable_prev_sibling`) **after** calling this function, because
/// the DOM tree changes during reparenting.
///
/// Returns `(entity, is_temp)` where `is_temp` is true if a temporary
/// `DocumentFragment` was created.
///
/// For the multi-node (wrapper) case, each arg is appended to the temp fragment per
/// WHATWG DOM §4.2.6 step 4 ("append node to fragment" = the DOM mutation "append" =
/// a §4.2.3 insert into the wrapper). The wrapper is transient/unobserved so its
/// destination records are irrelevant, BUT the spec also queues unsuppressed **source**
/// records the observer DOES see — an already-parented arg's §4.5 adopt source-parent
/// removal, and a `DocumentFragment` arg's §4.2.3 step-4.2 fragment record — so those
/// are pushed to `session` here ([`convert_arg_source_records`], captured pre-move).
/// (The single-node case returns the arg directly and lets `apply_*` produce its
/// records, so no source push is needed there.)
pub(crate) fn convert_nodes_into_node(
    nodes: Vec<Entity>,
    session: &mut SessionCore,
    dom: &mut EcsDom,
) -> (Entity, bool) {
    if nodes.len() == 1 {
        return (nodes[0], false);
    }
    let fragment = dom.create_document_fragment();
    for node in nodes {
        // Queue the unsuppressed source records (adopt removal / fragment record)
        // BEFORE the raw move detaches the arg (which would destroy the pre-move
        // sibling/child state the records capture).
        for record in convert_arg_source_records(dom, node) {
            session.push_notify_record(record);
        }
        // A `DocumentFragment` arg is expanded one level by hand (the raw
        // `EcsDom::append_child` does not expand) so the wrapper stays flat — a
        // fragment's children are never fragments (B1.2-fragment "never nests"); without
        // this `append(frag1, frag2)` would nest fragments under the wrapper.
        if dom.is_document_fragment(node) {
            for child in dom.child_list_uncapped(node) {
                let ok = dom.append_child(fragment, child);
                debug_assert!(ok, "append_child to fresh fragment should not fail");
            }
        } else {
            let ok = dom.append_child(fragment, node);
            debug_assert!(ok, "append_child to fresh fragment should not fail");
        }
    }
    (fragment, true)
}

/// Walk next siblings of `entity`, skipping any in `exclude`. Returns the first
/// sibling not in the exclude list, or `None`.
pub(crate) fn viable_next_sibling(
    entity: Entity,
    exclude: &[Entity],
    dom: &EcsDom,
) -> Option<Entity> {
    // Walk the **exposed** sibling chain (`next_exposed_sibling` skips internal
    // `ShadowRoot` entities, §4.8) so a shadow host's shadow root can never become a
    // reference child — otherwise it would leak into `MutationRecord.nextSibling`,
    // which the Node accessors hide from `firstChild`/`nextSibling`. (Codex PR393 R2.)
    let mut current = dom.next_exposed_sibling(entity);
    while let Some(sibling) = current {
        if !exclude.contains(&sibling) {
            return Some(sibling);
        }
        current = dom.next_exposed_sibling(sibling);
    }
    None
}

/// Walk previous siblings of `entity`, skipping any in `exclude` (and internal
/// `ShadowRoot` entities — see [`viable_next_sibling`]). Returns the first viable
/// exposed sibling, or `None`.
pub(crate) fn viable_prev_sibling(
    entity: Entity,
    exclude: &[Entity],
    dom: &EcsDom,
) -> Option<Entity> {
    let mut current = dom.prev_exposed_sibling(entity);
    while let Some(sibling) = current {
        if !exclude.contains(&sibling) {
            return Some(sibling);
        }
        current = dom.prev_exposed_sibling(sibling);
    }
    None
}

/// Validate that a replace operation is permitted per WHATWG DOM §4.2.3.
pub(crate) fn ensure_replace_validity(
    parent: Entity,
    node: Entity,
    child: Entity,
    dom: &EcsDom,
) -> Result<(), DomApiError> {
    let parent_kind = validate_parent_kind(parent, dom)?;

    if dom.is_host_including_ancestor_or_self(node, parent) {
        return Err(hierarchy_error(
            "node is an ancestor of parent (would create cycle)",
        ));
    }

    if dom.get_parent(child) != Some(parent) {
        return Err(not_found_error("child is not a child of parent"));
    }

    let node_kind = validate_node_type(node, dom)?;
    validate_doctype_and_text(parent_kind, node_kind, node, dom)?;

    // Step 6: Document parent constraints (excluding `child` from counts).
    if matches!(parent_kind, Some(NodeKind::Document)) {
        validate_document_element_constraint(parent, node, node_kind, Some(child), &[child], dom)?;
    }

    Ok(())
}
#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests;
