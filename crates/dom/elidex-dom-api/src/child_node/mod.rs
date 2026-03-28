//! ChildNode/ParentNode mixin handlers, pre-insertion validation, and Element
//! selector methods (`matches`, `closest`).
//!
//! Implements WHATWG DOM Living Standard:
//! - `ChildNode` mixin (before, after, remove, replaceWith)
//! - `ParentNode` mixin (prepend, append, replaceChildren)
//! - Element selector methods (matches, closest)
//! - Pre-insertion validation (WHATWG DOM 4.2.4)

mod mutations;
mod selectors;

pub use mutations::{
    After, Append, Before, ChildNodeRemove, Prepend, ReplaceChildren, ReplaceWith,
};
pub use selectors::{Closest, Matches};

use elidex_ecs::{EcsDom, Entity, NodeKind, TagType, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind, JsObjectRef, SessionCore};

use crate::util::not_found_error;

// ---------------------------------------------------------------------------
// Pre-insertion validation (WHATWG DOM 4.2.4)
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

/// Enforce Document parent child constraints (step 6 of WHATWG DOM §4.2.4).
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
            // Document already has an element child (other than child being replaced)?
            let has_element = dom
                .children_iter(parent)
                .any(|c| c != node && dom.is_element(c) && !exclude.contains(&c));
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
                let has_element = dom
                    .children_iter(parent)
                    .any(|c| c != node && dom.is_element(c) && !exclude.contains(&c));
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
            // Document already has a doctype child (other than the one being replaced)?
            let has_doctype = dom.children_iter(parent).any(|c| {
                !exclude.contains(&c)
                    && c != node
                    && matches!(dom.node_kind(c), Some(NodeKind::DocumentType))
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

/// Validate that a pre-insertion operation is permitted per WHATWG DOM 4.2.4.
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
pub(crate) fn convert_nodes_into_node(nodes: Vec<Entity>, dom: &mut EcsDom) -> (Entity, bool) {
    if nodes.len() == 1 {
        return (nodes[0], false);
    }
    let fragment = dom.create_document_fragment();
    for node in nodes {
        let ok = dom.append_child(fragment, node);
        debug_assert!(ok, "append_child to fresh fragment should not fail");
    }
    (fragment, true)
}

/// Insert a node before `ref_child` under `parent`, expanding `DocumentFragment`
/// contents.
///
/// If `node` is a `DocumentFragment`, its children are moved one-by-one into
/// `parent` before `ref_child`. Otherwise `node` itself is inserted/appended.
pub(crate) fn insert_node_expanding_fragment(
    parent: Entity,
    node: Entity,
    ref_child: Option<Entity>,
    dom: &mut EcsDom,
) -> Result<(), DomApiError> {
    let is_fragment = matches!(dom.node_kind(node), Some(NodeKind::DocumentFragment));

    if is_fragment {
        // Collect fragment children first to avoid iterator invalidation.
        let children = dom.children(node);
        for child in children {
            match ref_child {
                Some(ref_entity) => {
                    if !dom.insert_before(parent, child, ref_entity) {
                        return Err(DomApiError {
                            kind: DomApiErrorKind::HierarchyRequestError,
                            message: "failed to insert fragment child".into(),
                        });
                    }
                }
                None => {
                    if !dom.append_child(parent, child) {
                        return Err(DomApiError {
                            kind: DomApiErrorKind::HierarchyRequestError,
                            message: "failed to append fragment child".into(),
                        });
                    }
                }
            }
        }
    } else {
        match ref_child {
            Some(ref_entity) => {
                if !dom.insert_before(parent, node, ref_entity) {
                    return Err(DomApiError {
                        kind: DomApiErrorKind::HierarchyRequestError,
                        message: "failed to insert node".into(),
                    });
                }
            }
            None => {
                if !dom.append_child(parent, node) {
                    return Err(DomApiError {
                        kind: DomApiErrorKind::HierarchyRequestError,
                        message: "failed to append node".into(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Walk next siblings of `entity`, skipping any in `exclude`. Returns the first
/// sibling not in the exclude list, or `None`.
pub(crate) fn viable_next_sibling(
    entity: Entity,
    exclude: &[Entity],
    dom: &EcsDom,
) -> Option<Entity> {
    let mut current = dom.get_next_sibling(entity);
    while let Some(sibling) = current {
        if !exclude.contains(&sibling) {
            return Some(sibling);
        }
        current = dom.get_next_sibling(sibling);
    }
    None
}

/// Walk previous siblings of `entity`, skipping any in `exclude`. Returns the
/// first sibling not in the exclude list, or `None`.
pub(crate) fn viable_prev_sibling(
    entity: Entity,
    exclude: &[Entity],
    dom: &EcsDom,
) -> Option<Entity> {
    let mut current = dom.get_prev_sibling(entity);
    while let Some(sibling) = current {
        if !exclude.contains(&sibling) {
            return Some(sibling);
        }
        current = dom.get_prev_sibling(sibling);
    }
    None
}

/// Validate that a replace operation is permitted per WHATWG DOM 4.2.4.
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
