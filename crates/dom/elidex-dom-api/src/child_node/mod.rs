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
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom, Entity, TextContent};
    use elidex_plugin::JsValue;
    use elidex_script_session::{
        ComponentKind, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
    };

    /// Create a simple DOM: doc > body > [div, span, p]
    fn setup() -> (EcsDom, Entity, Entity, Entity, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let body = dom.create_element("body", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(body, span);
        dom.append_child(body, p);

        let mut session = SessionCore::new();
        session.get_or_create_wrapper(body, ComponentKind::Element);
        session.get_or_create_wrapper(div, ComponentKind::Element);
        session.get_or_create_wrapper(span, ComponentKind::Element);
        session.get_or_create_wrapper(p, ComponentKind::Element);

        (dom, body, div, span, p, session)
    }

    // ---- before ----

    #[test]
    fn before_single() {
        let (mut dom, body, div, span, _p, mut session) = setup();
        let new_el = dom.create_element("em", Attributes::default());
        let new_ref = session
            .get_or_create_wrapper(new_el, ComponentKind::Element)
            .to_raw();

        let handler = Before;
        handler
            .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children.len(), 4);
        assert_eq!(children[0], div);
        assert_eq!(children[1], new_el);
        assert_eq!(children[2], span);
    }

    #[test]
    fn before_multiple() {
        let (mut dom, body, div, span, _p, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let a_ref = session
            .get_or_create_wrapper(a, ComponentKind::Element)
            .to_raw();
        let b = dom.create_element("b", Attributes::default());
        let b_ref = session
            .get_or_create_wrapper(b, ComponentKind::Element)
            .to_raw();

        let handler = Before;
        handler
            .invoke(
                span,
                &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children[0], div);
        assert_eq!(children[1], a);
        assert_eq!(children[2], b);
        assert_eq!(children[3], span);
    }

    #[test]
    fn before_string_creates_text() {
        let (mut dom, body, _div, span, _p, mut session) = setup();

        let handler = Before;
        handler
            .invoke(
                span,
                &[JsValue::String("hello".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children.len(), 4);
        // The text node should be before span.
        let text_entity = children[1];
        let tc = dom.world().get::<&TextContent>(text_entity).unwrap();
        assert_eq!(tc.0, "hello");
    }

    #[test]
    fn before_orphan_noop() {
        let mut dom = EcsDom::new();
        let orphan = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();

        let handler = Before;
        let result = handler.invoke(
            orphan,
            &[JsValue::String("text".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_ok());
    }

    // ---- after ----

    #[test]
    fn after_single() {
        let (mut dom, body, div, span, p, mut session) = setup();
        let new_el = dom.create_element("em", Attributes::default());
        let new_ref = session
            .get_or_create_wrapper(new_el, ComponentKind::Element)
            .to_raw();

        let handler = After;
        handler
            .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children.len(), 4);
        assert_eq!(children[0], div);
        assert_eq!(children[1], span);
        assert_eq!(children[2], new_el);
        assert_eq!(children[3], p);
    }

    #[test]
    fn after_multiple() {
        let (mut dom, body, div, span, p, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let a_ref = session
            .get_or_create_wrapper(a, ComponentKind::Element)
            .to_raw();
        let b = dom.create_element("b", Attributes::default());
        let b_ref = session
            .get_or_create_wrapper(b, ComponentKind::Element)
            .to_raw();

        let handler = After;
        handler
            .invoke(
                div,
                &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children[0], div);
        assert_eq!(children[1], a);
        assert_eq!(children[2], b);
        assert_eq!(children[3], span);
        assert_eq!(children[4], p);
    }

    // ---- remove ----

    #[test]
    fn remove_attached() {
        let (mut dom, body, div, span, p, mut session) = setup();

        let handler = ChildNodeRemove;
        handler.invoke(span, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(body);
        assert_eq!(children, vec![div, p]);
    }

    #[test]
    fn remove_orphan_noop() {
        let mut dom = EcsDom::new();
        let orphan = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();

        let handler = ChildNodeRemove;
        let result = handler.invoke(orphan, &[], &mut session, &mut dom);
        assert!(result.is_ok());
    }

    // ---- replaceWith ----

    #[test]
    fn replace_with_single() {
        let (mut dom, body, div, span, p, mut session) = setup();
        let new_el = dom.create_element("em", Attributes::default());
        let new_ref = session
            .get_or_create_wrapper(new_el, ComponentKind::Element)
            .to_raw();

        let handler = ReplaceWith;
        handler
            .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children, vec![div, new_el, p]);
    }

    #[test]
    fn replace_with_multiple() {
        let (mut dom, body, div, span, p, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let a_ref = session
            .get_or_create_wrapper(a, ComponentKind::Element)
            .to_raw();
        let b = dom.create_element("b", Attributes::default());
        let b_ref = session
            .get_or_create_wrapper(b, ComponentKind::Element)
            .to_raw();

        let handler = ReplaceWith;
        handler
            .invoke(
                span,
                &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children, vec![div, a, b, p]);
    }

    // ---- prepend ----

    #[test]
    fn prepend_single() {
        let (mut dom, body, div, _span, _p, mut session) = setup();
        let new_el = dom.create_element("em", Attributes::default());
        let new_ref = session
            .get_or_create_wrapper(new_el, ComponentKind::Element)
            .to_raw();

        let handler = Prepend;
        handler
            .invoke(body, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children[0], new_el);
        assert_eq!(children[1], div);
    }

    #[test]
    fn prepend_multiple() {
        let (mut dom, body, div, _span, _p, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let a_ref = session
            .get_or_create_wrapper(a, ComponentKind::Element)
            .to_raw();
        let b = dom.create_element("b", Attributes::default());
        let b_ref = session
            .get_or_create_wrapper(b, ComponentKind::Element)
            .to_raw();

        let handler = Prepend;
        handler
            .invoke(
                body,
                &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children[0], a);
        assert_eq!(children[1], b);
        assert_eq!(children[2], div);
    }

    #[test]
    fn prepend_empty() {
        let (mut dom, body, div, span, p, mut session) = setup();

        let handler = Prepend;
        handler.invoke(body, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(body);
        assert_eq!(children, vec![div, span, p]);
    }

    // ---- append ----

    #[test]
    fn append_single() {
        let (mut dom, body, _div, _span, _p, mut session) = setup();
        let new_el = dom.create_element("em", Attributes::default());
        let new_ref = session
            .get_or_create_wrapper(new_el, ComponentKind::Element)
            .to_raw();

        let handler = Append;
        handler
            .invoke(body, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children.last(), Some(&new_el));
        assert_eq!(children.len(), 4);
    }

    #[test]
    fn append_multiple() {
        let (mut dom, body, _div, _span, _p, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let a_ref = session
            .get_or_create_wrapper(a, ComponentKind::Element)
            .to_raw();
        let b = dom.create_element("b", Attributes::default());
        let b_ref = session
            .get_or_create_wrapper(b, ComponentKind::Element)
            .to_raw();

        let handler = Append;
        handler
            .invoke(
                body,
                &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children.len(), 5);
        assert_eq!(children[3], a);
        assert_eq!(children[4], b);
    }

    // ---- replaceChildren ----

    #[test]
    fn replace_children() {
        let (mut dom, body, _div, _span, _p, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let a_ref = session
            .get_or_create_wrapper(a, ComponentKind::Element)
            .to_raw();
        let b = dom.create_element("b", Attributes::default());
        let b_ref = session
            .get_or_create_wrapper(b, ComponentKind::Element)
            .to_raw();

        let handler = ReplaceChildren;
        handler
            .invoke(
                body,
                &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children, vec![a, b]);
    }

    // ---- matches ----

    #[test]
    fn matches_tag() {
        let (mut dom, _body, div, _span, _p, mut session) = setup();

        let handler = Matches;
        let result = handler
            .invoke(
                div,
                &[JsValue::String("div".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn matches_class() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "active");
        let el = dom.create_element("div", attrs);
        let mut session = SessionCore::new();

        let handler = Matches;
        let result = handler
            .invoke(
                el,
                &[JsValue::String(".active".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn matches_no_match() {
        let (mut dom, _body, div, _span, _p, mut session) = setup();

        let handler = Matches;
        let result = handler
            .invoke(
                div,
                &[JsValue::String("span".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    #[test]
    fn matches_invalid_selector() {
        let (mut dom, _body, div, _span, _p, mut session) = setup();

        let handler = Matches;
        let result = handler.invoke(
            div,
            &[JsValue::String(">>>".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    // ---- closest ----

    #[test]
    fn closest_self() {
        let (mut dom, _body, div, _span, _p, mut session) = setup();

        let handler = Closest;
        let result = handler
            .invoke(
                div,
                &[JsValue::String("div".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        // Should return an ObjectRef for `div` itself.
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn closest_ancestor() {
        let (mut dom, body, _div, span, _p, mut session) = setup();

        let handler = Closest;
        let result = handler
            .invoke(
                span,
                &[JsValue::String("body".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        match result {
            JsValue::ObjectRef(id) => {
                let (entity, _) = session
                    .identity_map()
                    .get(JsObjectRef::from_raw(id))
                    .unwrap();
                assert_eq!(entity, body);
            }
            _ => panic!("expected ObjectRef"),
        }
    }

    #[test]
    fn closest_none() {
        let (mut dom, _body, div, _span, _p, mut session) = setup();

        let handler = Closest;
        let result = handler
            .invoke(
                div,
                &[JsValue::String("article".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn closest_skips_text() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("hello");
        dom.append_child(div, text);
        let mut session = SessionCore::new();

        let handler = Closest;
        // Starting from text node, closest("div") should find the parent div.
        let result = handler
            .invoke(
                text,
                &[JsValue::String("div".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    // ---- viable_next_sibling ----

    #[test]
    fn viable_sibling_basic() {
        let (dom, _body, div, span, p, _session) = setup();

        // Next sibling of div, excluding nothing.
        assert_eq!(viable_next_sibling(div, &[], &dom), Some(span));

        // Next sibling of div, excluding span -> p.
        assert_eq!(viable_next_sibling(div, &[span], &dom), Some(p));

        // Next sibling of div, excluding span and p -> None.
        assert_eq!(viable_next_sibling(div, &[span, p], &dom), None);
    }

    #[test]
    fn self_in_args_skipped() {
        // When `before` is called on a node that is also in the args,
        // viable_next_sibling should skip it.
        let (dom, _body, div, span, p, _session) = setup();
        assert_eq!(viable_next_sibling(div, &[span], &dom), Some(p));
    }

    // ---- ensure_pre_insertion_validity ----

    #[test]
    fn ensure_validity_invalid_parent() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");
        let child = dom.create_element("span", Attributes::default());

        let result = ensure_pre_insertion_validity(text, child, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_ancestor_cycle() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);

        // Trying to insert parent under child should be rejected.
        let result = ensure_pre_insertion_validity(child, parent, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_ref_child_not_child_of_parent() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let node = dom.create_element("span", Attributes::default());
        let unrelated = dom.create_element("p", Attributes::default());

        let result = ensure_pre_insertion_validity(parent, node, Some(unrelated), &dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::NotFoundError);
    }

    #[test]
    fn ensure_validity_valid_insertion() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let node = dom.create_element("span", Attributes::default());
        let existing = dom.create_element("p", Attributes::default());
        dom.append_child(parent, existing);

        let result = ensure_pre_insertion_validity(parent, node, Some(existing), &dom);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_validity_document_rejects_text() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let text = dom.create_text("hello");

        let result = ensure_pre_insertion_validity(doc, text, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    // ---- ensure_pre_insertion_validity: step 5 DocumentType ----

    #[test]
    fn ensure_validity_doctype_under_element_rejected() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let doctype = dom.create_document_type("html", "", "");
        let result = ensure_pre_insertion_validity(parent, doctype, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_doctype_under_document_ok() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let doctype = dom.create_document_type("html", "", "");
        let result = ensure_pre_insertion_validity(doc, doctype, None, &dom);
        assert!(result.is_ok());
    }

    // ---- ensure_pre_insertion_validity: step 6 Document constraints ----

    #[test]
    fn ensure_validity_document_rejects_second_element() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        dom.append_child(doc, html);
        let second = dom.create_element("body", Attributes::default());
        let result = ensure_pre_insertion_validity(doc, second, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_document_allows_first_element() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let result = ensure_pre_insertion_validity(doc, html, None, &dom);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_validity_document_rejects_element_before_doctype() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let doctype = dom.create_document_type("html", "", "");
        dom.append_child(doc, doctype);
        let html = dom.create_element("html", Attributes::default());
        // Inserting element before doctype should fail.
        let result = ensure_pre_insertion_validity(doc, html, Some(doctype), &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_document_rejects_second_doctype() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let dt1 = dom.create_document_type("html", "", "");
        dom.append_child(doc, dt1);
        let dt2 = dom.create_document_type("html", "", "");
        let result = ensure_pre_insertion_validity(doc, dt2, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_document_rejects_doctype_after_element() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        dom.append_child(doc, html);
        let doctype = dom.create_document_type("html", "", "");
        // Appending doctype after element should fail.
        let result = ensure_pre_insertion_validity(doc, doctype, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn ensure_validity_document_fragment_rejects_text_child() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let frag = dom.create_document_fragment();
        let text = dom.create_text("hello");
        dom.append_child(frag, text);
        let result = ensure_pre_insertion_validity(doc, frag, None, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    // ---- ensure_replace_validity ----

    #[test]
    fn replace_validity_ok() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let old = dom.create_element("span", Attributes::default());
        let new_el = dom.create_element("p", Attributes::default());
        dom.append_child(parent, old);
        let result = ensure_replace_validity(parent, new_el, old, &dom);
        assert!(result.is_ok());
    }

    #[test]
    fn replace_validity_cycle_rejected() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);
        // Trying to replace child with parent (ancestor) should be rejected.
        let result = ensure_replace_validity(parent, parent, child, &dom);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::HierarchyRequestError
        );
    }

    #[test]
    fn replace_validity_child_not_found() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let node = dom.create_element("span", Attributes::default());
        let unrelated = dom.create_element("p", Attributes::default());
        let result = ensure_replace_validity(parent, node, unrelated, &dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::NotFoundError);
    }

    // ---- viable_prev_sibling ----

    #[test]
    fn viable_prev_basic() {
        let (dom, _body, div, span, p, _session) = setup();
        // Previous of p, excluding nothing -> span.
        assert_eq!(viable_prev_sibling(p, &[], &dom), Some(span));
        // Previous of p, excluding span -> div.
        assert_eq!(viable_prev_sibling(p, &[span], &dom), Some(div));
        // Previous of p, excluding span and div -> None.
        assert_eq!(viable_prev_sibling(p, &[span, div], &dom), None);
        // Previous of div -> None (first child).
        assert_eq!(viable_prev_sibling(div, &[], &dom), None);
    }

    // ---- ReplaceChildren validates before removing (H3) ----

    #[test]
    fn replace_children_validates_before_removing() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");
        // Cannot insert nodes under a text node.
        let child = dom.create_element("span", Attributes::default());
        let mut session = SessionCore::new();
        let child_ref = session
            .get_or_create_wrapper(child, ComponentKind::Element)
            .to_raw();
        // Calling replaceChildren on text should fail validation.
        let handler = ReplaceChildren;
        let result = handler.invoke(
            text,
            &[JsValue::ObjectRef(child_ref)],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    // ---- before with viable_prev_sibling ----

    #[test]
    fn before_self_in_nodes() {
        let (mut dom, body, div, span, _p, mut session) = setup();
        // Insert a new element before span.
        let new_el = dom.create_element("em", Attributes::default());
        let new_ref = session
            .get_or_create_wrapper(new_el, ComponentKind::Element)
            .to_raw();

        let handler = Before;
        handler
            .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
            .unwrap();

        let children = dom.children(body);
        assert_eq!(children[0], div);
        assert_eq!(children[1], new_el);
        assert_eq!(children[2], span);
    }

    // ---- after with validation ----

    #[test]
    fn after_validates_insertion() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);
        let mut session = SessionCore::new();

        // Try to insert parent after child (would create cycle).
        let parent_ref = session
            .get_or_create_wrapper(parent, ComponentKind::Element)
            .to_raw();
        let handler = After;
        let result = handler.invoke(
            child,
            &[JsValue::ObjectRef(parent_ref)],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    // ---- replaceWith with validation (M4) ----

    #[test]
    fn replace_with_empty_removes() {
        let (mut dom, body, div, span, p, mut session) = setup();

        let handler = ReplaceWith;
        handler.invoke(span, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(body);
        assert_eq!(children, vec![div, p]);
    }
}
