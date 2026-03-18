//! Node interface method implementations as `DomApiHandler` trait impls.
//!
//! Covers: `contains`, `compareDocumentPosition`, `cloneNode`, `normalize`,
//! `isConnected`, `getRootNode`, `textContent` (NodeKind-aware), `nodeValue`,
//! `ownerDocument`, `isSameNode`, `isEqualNode`.

use elidex_ecs::{
    Attributes, CommentData, DocTypeData, EcsDom, Entity, NodeKind, ShadowRoot, TagType,
    TextContent,
};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::{not_found_error, require_object_ref_arg};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Collect concatenated text content from an entity and its descendants.
fn descendant_text_content(entity: Entity, dom: &EcsDom) -> String {
    crate::element::collect_text_content(entity, dom)
}

/// Resolve an optional `ObjectRef` arg to an Entity. Returns `None` if the arg
/// is `Null`, `Undefined`, or missing.
fn resolve_optional_entity(
    args: &[JsValue],
    index: usize,
    session: &SessionCore,
) -> Result<Option<Entity>, DomApiError> {
    match args.get(index) {
        None | Some(JsValue::Null | JsValue::Undefined) => Ok(None),
        Some(JsValue::ObjectRef(id)) => {
            let (entity, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(*id))
                .ok_or_else(|| not_found_error("entity not found"))?;
            Ok(Some(entity))
        }
        _ => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a Node or null"),
        }),
    }
}

/// Walk ancestors to find the tree root, stopping at `ShadowRoot` boundaries
/// (non-composed root). Delegates to `EcsDom::find_tree_root`.
fn find_root_non_composed(entity: Entity, dom: &EcsDom) -> Entity {
    dom.find_tree_root(entity)
}

/// Walk ancestors to find the composed tree root (crosses shadow boundaries).
fn find_root(entity: Entity, dom: &EcsDom) -> Entity {
    let mut current = entity;
    let mut depth = 0;
    while let Some(parent) = dom.get_parent(current) {
        current = parent;
        depth += 1;
        if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
            break;
        }
    }
    current
}

/// Check deep structural equality of two nodes.
fn nodes_equal(a: Entity, b: Entity, dom: &EcsDom) -> bool {
    let kind_a = dom.node_kind(a);
    let kind_b = dom.node_kind(b);
    if kind_a != kind_b {
        return false;
    }

    match kind_a {
        Some(NodeKind::Element) => {
            // Same tag name.
            let tag_a = dom.world().get::<&TagType>(a).ok().map(|t| t.0.clone());
            let tag_b = dom.world().get::<&TagType>(b).ok().map(|t| t.0.clone());
            if tag_a != tag_b {
                return false;
            }
            // Same attributes (count + values).
            let attrs_a = dom.world().get::<&Attributes>(a).ok();
            let attrs_b = dom.world().get::<&Attributes>(b).ok();
            match (&attrs_a, &attrs_b) {
                (Some(a_ref), Some(b_ref)) => {
                    if a_ref.len() != b_ref.len() {
                        return false;
                    }
                    for (name, val) in a_ref.iter() {
                        if b_ref.get(name) != Some(val) {
                            return false;
                        }
                    }
                }
                (None, None) => {}
                _ => return false,
            }
        }
        Some(NodeKind::Text | NodeKind::CdataSection) => {
            let ta = dom.world().get::<&TextContent>(a).ok().map(|t| t.0.clone());
            let tb = dom.world().get::<&TextContent>(b).ok().map(|t| t.0.clone());
            if ta != tb {
                return false;
            }
        }
        Some(NodeKind::Comment) => {
            let ca = dom.world().get::<&CommentData>(a).ok().map(|c| c.0.clone());
            let cb = dom.world().get::<&CommentData>(b).ok().map(|c| c.0.clone());
            if ca != cb {
                return false;
            }
        }
        Some(NodeKind::DocumentType) => {
            let da = dom
                .world()
                .get::<&DocTypeData>(a)
                .ok()
                .map(|d| (d.name.clone(), d.public_id.clone(), d.system_id.clone()));
            let db = dom
                .world()
                .get::<&DocTypeData>(b)
                .ok()
                .map(|d| (d.name.clone(), d.public_id.clone(), d.system_id.clone()));
            if da != db {
                return false;
            }
        }
        Some(NodeKind::Document | NodeKind::DocumentFragment) => {
            // No data to compare beyond children.
        }
        _ => {
            // Unknown or missing NodeKind — not equal if both are None.
            if kind_a.is_none() {
                return false;
            }
        }
    }

    // Compare children recursively.
    let children_a = dom.children(a);
    let children_b = dom.children(b);
    if children_a.len() != children_b.len() {
        return false;
    }
    children_a
        .iter()
        .zip(children_b.iter())
        .all(|(&ca, &cb)| nodes_equal(ca, cb, dom))
}

// ---------------------------------------------------------------------------
// 1. Contains
// ---------------------------------------------------------------------------

/// `node.contains(other)` — check if this node contains the argument node.
pub struct Contains;

impl DomApiHandler for Contains {
    fn method_name(&self) -> &str {
        "contains"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let target = resolve_optional_entity(args, 0, session)?;
        let Some(target) = target else {
            return Ok(JsValue::Bool(false));
        };
        Ok(JsValue::Bool(dom.is_ancestor_or_self(this, target)))
    }
}

// ---------------------------------------------------------------------------
// 2. CompareDocumentPosition
// ---------------------------------------------------------------------------

/// Bitmask constants for `compareDocumentPosition`.
const DOCUMENT_POSITION_DISCONNECTED: u32 = 1;
const DOCUMENT_POSITION_PRECEDING: u32 = 2;
const DOCUMENT_POSITION_FOLLOWING: u32 = 4;
const DOCUMENT_POSITION_CONTAINS: u32 = 8;
const DOCUMENT_POSITION_CONTAINED_BY: u32 = 16;
const DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: u32 = 32;

/// `node.compareDocumentPosition(other)` — returns a bitmask.
pub struct CompareDocumentPosition;

impl DomApiHandler for CompareDocumentPosition {
    fn method_name(&self) -> &str {
        "compareDocumentPosition"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let other_ref = require_object_ref_arg(args, 0)?;
        let (other, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(other_ref))
            .ok_or_else(|| not_found_error("other node not found"))?;

        if this == other {
            return Ok(JsValue::Number(0.0));
        }

        // Check if they share a common root.
        let root_this = find_root(this, dom);
        let root_other = find_root(other, dom);

        if root_this != root_other {
            // Disconnected — use entity bits for consistent ordering.
            let dir = if this.to_bits() < other.to_bits() {
                DOCUMENT_POSITION_PRECEDING
            } else {
                DOCUMENT_POSITION_FOLLOWING
            };
            return Ok(JsValue::Number(f64::from(
                DOCUMENT_POSITION_DISCONNECTED | DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC | dir,
            )));
        }

        // Check containment.
        if dom.is_ancestor_or_self(other, this) && other != this {
            // `other` contains `this` — other is an ancestor of this.
            return Ok(JsValue::Number(f64::from(
                DOCUMENT_POSITION_CONTAINS | DOCUMENT_POSITION_PRECEDING,
            )));
        }
        if dom.is_ancestor_or_self(this, other) && this != other {
            // `this` contains `other` — this is an ancestor of other.
            return Ok(JsValue::Number(f64::from(
                DOCUMENT_POSITION_CONTAINED_BY | DOCUMENT_POSITION_FOLLOWING,
            )));
        }

        // Use tree order comparison.
        match dom.tree_order_cmp(this, other) {
            std::cmp::Ordering::Less => Ok(JsValue::Number(f64::from(DOCUMENT_POSITION_FOLLOWING))),
            std::cmp::Ordering::Greater => {
                Ok(JsValue::Number(f64::from(DOCUMENT_POSITION_PRECEDING)))
            }
            std::cmp::Ordering::Equal => Ok(JsValue::Number(0.0)),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. CloneNode
// ---------------------------------------------------------------------------

/// `node.cloneNode(deep?)` — clone a node (optionally deep).
pub struct CloneNode;

impl CloneNode {
    /// Clone a single entity (shallow) and return the new entity.
    fn clone_single(entity: Entity, dom: &mut EcsDom) -> Result<Entity, DomApiError> {
        let kind = dom.node_kind(entity);

        // ShadowRoot cannot be cloned.
        if dom.world().get::<&ShadowRoot>(entity).is_ok() {
            return Err(DomApiError {
                kind: DomApiErrorKind::NotSupportedError,
                message: "cloneNode: ShadowRoot cannot be cloned".into(),
            });
        }

        match kind {
            Some(NodeKind::Element) => {
                let tag = dom
                    .world()
                    .get::<&TagType>(entity)
                    .map(|t| t.0.clone())
                    .unwrap_or_default();
                let attrs = dom
                    .world()
                    .get::<&Attributes>(entity)
                    .ok()
                    .map(|a| (*a).clone())
                    .unwrap_or_default();
                // Note: InlineStyle is NOT copied per DOM spec (cloneNode does not
                // copy the CSSOM-level style object).
                Ok(dom.create_element(tag, attrs))
            }
            Some(NodeKind::Text) => {
                let text = dom
                    .world()
                    .get::<&TextContent>(entity)
                    .map(|t| t.0.clone())
                    .unwrap_or_default();
                Ok(dom.create_text(text))
            }
            Some(NodeKind::Comment) => {
                let data = dom
                    .world()
                    .get::<&CommentData>(entity)
                    .map(|c| c.0.clone())
                    .unwrap_or_default();
                Ok(dom.create_comment(data))
            }
            Some(NodeKind::DocumentType) => {
                let dt = dom
                    .world()
                    .get::<&DocTypeData>(entity)
                    .ok()
                    .map(|d| (d.name.clone(), d.public_id.clone(), d.system_id.clone()));
                if let Some((name, public_id, system_id)) = dt {
                    Ok(dom.create_document_type(name, public_id, system_id))
                } else {
                    Ok(dom.create_document_type("", "", ""))
                }
            }
            Some(NodeKind::DocumentFragment) => Ok(dom.create_document_fragment()),
            Some(NodeKind::Document) => {
                // Cloning a Document creates a new document root.
                Ok(dom.create_document_root())
            }
            _ => Err(DomApiError {
                kind: DomApiErrorKind::NotSupportedError,
                message: "cloneNode: unsupported node kind".into(),
            }),
        }
    }

    /// Deep clone: clone entity and recursively clone all children.
    fn clone_deep(entity: Entity, dom: &mut EcsDom) -> Result<Entity, DomApiError> {
        let clone = Self::clone_single(entity, dom)?;
        let children = dom.children(entity);
        for child in children {
            let child_clone = Self::clone_deep(child, dom)?;
            let _ = dom.append_child(clone, child_clone);
        }
        Ok(clone)
    }
}

impl DomApiHandler for CloneNode {
    fn method_name(&self) -> &str {
        "cloneNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let deep = matches!(args.first(), Some(JsValue::Bool(true)));

        let cloned = if deep {
            Self::clone_deep(this, dom)?
        } else {
            Self::clone_single(this, dom)?
        };

        // Register the new entity in the session (new identity, not copied).
        let kind = match dom.node_kind(cloned) {
            Some(nk) => ComponentKind::from_node_kind(nk),
            None => ComponentKind::Element,
        };
        let obj_ref = session.get_or_create_wrapper(cloned, kind);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// 4. Normalize
// ---------------------------------------------------------------------------

/// `node.normalize()` — merge adjacent text nodes, remove empty text nodes.
pub struct Normalize;

impl Normalize {
    fn normalize_entity(entity: Entity, dom: &mut EcsDom) {
        let mut current = dom.get_first_child(entity);
        let mut prev_text: Option<Entity> = None;

        while let Some(child) = current {
            let next = dom.get_next_sibling(child);
            let is_text = dom.node_kind(child) == Some(NodeKind::Text);

            if is_text {
                let text = dom
                    .world()
                    .get::<&TextContent>(child)
                    .map(|t| t.0.clone())
                    .unwrap_or_default();

                if text.is_empty() {
                    // Remove empty text node.
                    let ok = dom.remove_child(entity, child);
                    debug_assert!(ok, "remove_child: child from get_first_child walk");
                    dom.rev_version(entity);
                    current = next;
                    continue;
                }

                if let Some(prev) = prev_text {
                    // Merge with previous text node.
                    let prev_text_val = dom
                        .world()
                        .get::<&TextContent>(prev)
                        .map(|t| t.0.clone())
                        .unwrap_or_default();
                    let merged = prev_text_val + &text;
                    if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(prev) {
                        tc.0 = merged;
                    }
                    let ok = dom.remove_child(entity, child);
                    debug_assert!(ok, "remove_child: child from get_first_child walk");
                    dom.rev_version(entity);
                    current = next;
                    continue;
                }

                prev_text = Some(child);
            } else {
                prev_text = None;
                // Recurse into element/fragment children.
                if dom.node_kind(child) == Some(NodeKind::Element)
                    || dom.node_kind(child) == Some(NodeKind::DocumentFragment)
                {
                    Self::normalize_entity(child, dom);
                }
            }

            current = next;
        }
    }
}

impl DomApiHandler for Normalize {
    fn method_name(&self) -> &str {
        "normalize"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Self::normalize_entity(this, dom);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// 5. IsConnected
// ---------------------------------------------------------------------------

/// `node.isConnected` getter — true if root is a Document node.
pub struct IsConnected;

impl DomApiHandler for IsConnected {
    fn method_name(&self) -> &str {
        "isConnected.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let root = find_root(this, dom);
        let is_doc = dom.node_kind(root) == Some(NodeKind::Document);
        Ok(JsValue::Bool(is_doc))
    }
}

// ---------------------------------------------------------------------------
// 6. GetRootNode
// ---------------------------------------------------------------------------

/// `node.getRootNode(options?)` — return the root node.
pub struct GetRootNode;

impl DomApiHandler for GetRootNode {
    fn method_name(&self) -> &str {
        "getRootNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // Check for composed option: getRootNode({ composed: true }).
        // In our handler system, the JS bridge extracts the composed property
        // and passes it as a boolean arg.
        let composed = matches!(args.first(), Some(JsValue::Bool(true)));

        let root = if composed {
            find_root(this, dom)
        } else {
            find_root_non_composed(this, dom)
        };

        let kind = match dom.node_kind(root) {
            Some(nk) => ComponentKind::from_node_kind(nk),
            None => ComponentKind::Element,
        };
        let obj_ref = session.get_or_create_wrapper(root, kind);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// 7. GetTextContentNodeKind
// ---------------------------------------------------------------------------

/// `node.textContent` getter — NodeKind-aware behavior.
pub struct GetTextContentNodeKind;

impl DomApiHandler for GetTextContentNodeKind {
    fn method_name(&self) -> &str {
        "textContent.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        match dom.node_kind(this) {
            Some(NodeKind::Document | NodeKind::DocumentType) => Ok(JsValue::Null),
            Some(NodeKind::Text | NodeKind::CdataSection) => {
                let text = dom
                    .world()
                    .get::<&TextContent>(this)
                    .map(|t| t.0.clone())
                    .unwrap_or_default();
                Ok(JsValue::String(text))
            }
            Some(NodeKind::Comment) => {
                let data = dom
                    .world()
                    .get::<&CommentData>(this)
                    .map(|c| c.0.clone())
                    .unwrap_or_default();
                Ok(JsValue::String(data))
            }
            _ => {
                // Element, DocumentFragment, or any other kind.
                let text = descendant_text_content(this, dom);
                Ok(JsValue::String(text))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. SetTextContentNodeKind
// ---------------------------------------------------------------------------

/// `node.textContent` setter — NodeKind-aware behavior.
pub struct SetTextContentNodeKind;

impl DomApiHandler for SetTextContentNodeKind {
    fn method_name(&self) -> &str {
        "textContent.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = crate::util::require_string_arg(args, 0)?;

        match dom.node_kind(this) {
            Some(NodeKind::Document | NodeKind::DocumentType) => {
                // No-op per spec.
                Ok(JsValue::Undefined)
            }
            Some(NodeKind::Text | NodeKind::CdataSection) => {
                if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(this) {
                    text.clone_into(&mut tc.0);
                }
                dom.rev_version(this);
                Ok(JsValue::Undefined)
            }
            Some(NodeKind::Comment) => {
                if let Ok(mut cd) = dom.world_mut().get::<&mut CommentData>(this) {
                    text.clone_into(&mut cd.0);
                }
                dom.rev_version(this);
                Ok(JsValue::Undefined)
            }
            _ => {
                // Element / DocumentFragment: remove all children, add text node.
                let children = dom.children(this);
                for child in children {
                    session.release(child);
                    let _ = dom.remove_child(this, child);
                }
                if !text.is_empty() {
                    let text_node = dom.create_text(text);
                    let _ = dom.append_child(this, text_node);
                }
                Ok(JsValue::Undefined)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 9. SetNodeValue
// ---------------------------------------------------------------------------

/// `node.nodeValue` setter.
pub struct SetNodeValue;

impl DomApiHandler for SetNodeValue {
    fn method_name(&self) -> &str {
        "nodeValue.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = crate::util::require_string_arg(args, 0)?;

        match dom.node_kind(this) {
            Some(NodeKind::Text | NodeKind::CdataSection) => {
                if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(this) {
                    value.clone_into(&mut tc.0);
                }
                dom.rev_version(this);
            }
            Some(NodeKind::Comment) => {
                if let Ok(mut cd) = dom.world_mut().get::<&mut CommentData>(this) {
                    value.clone_into(&mut cd.0);
                }
                dom.rev_version(this);
            }
            _ => {
                // Element, Document, DocumentType, DocumentFragment — no-op.
            }
        }

        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// 10. OwnerDocument
// ---------------------------------------------------------------------------

/// `node.ownerDocument` getter.
pub struct OwnerDocument;

impl DomApiHandler for OwnerDocument {
    fn method_name(&self) -> &str {
        "ownerDocument.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // Document node itself returns null.
        if dom.node_kind(this) == Some(NodeKind::Document) {
            return Ok(JsValue::Null);
        }

        // Per spec, ownerDocument always returns the associated document,
        // even for disconnected nodes.
        if let Some(doc_root) = dom.document_root() {
            let obj_ref = session.get_or_create_wrapper(doc_root, ComponentKind::Document);
            Ok(JsValue::ObjectRef(obj_ref.to_raw()))
        } else {
            Ok(JsValue::Null)
        }
    }
}

// ---------------------------------------------------------------------------
// 11. IsSameNode
// ---------------------------------------------------------------------------

/// `node.isSameNode(other)` — identity comparison.
pub struct IsSameNode;

impl DomApiHandler for IsSameNode {
    fn method_name(&self) -> &str {
        "isSameNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let other = resolve_optional_entity(args, 0, session)?;
        let _ = dom; // Needed for trait signature.
        match other {
            None => Ok(JsValue::Bool(false)),
            Some(other_entity) => Ok(JsValue::Bool(this == other_entity)),
        }
    }
}

// ---------------------------------------------------------------------------
// 12. IsEqualNode
// ---------------------------------------------------------------------------

/// `node.isEqualNode(other)` — deep structural equality.
pub struct IsEqualNode;

impl DomApiHandler for IsEqualNode {
    fn method_name(&self) -> &str {
        "isEqualNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let other = resolve_optional_entity(args, 0, session)?;
        match other {
            None => Ok(JsValue::Bool(false)),
            Some(other_entity) => Ok(JsValue::Bool(nodes_equal(this, other_entity, dom))),
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, InlineStyle};

    fn setup() -> (EcsDom, SessionCore) {
        (EcsDom::new(), SessionCore::new())
    }

    fn wrap(entity: Entity, session: &mut SessionCore) -> u64 {
        session
            .get_or_create_wrapper(entity, ComponentKind::Element)
            .to_raw()
    }

    fn obj_ref_arg(entity: Entity, session: &mut SessionCore) -> JsValue {
        JsValue::ObjectRef(wrap(entity, session))
    }

    // -----------------------------------------------------------------------
    // Contains
    // -----------------------------------------------------------------------

    #[test]
    fn contains_self() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        wrap(div, &mut session);
        let r = Contains
            .invoke(
                div,
                &[obj_ref_arg(div, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Bool(true));
    }

    #[test]
    fn contains_descendant() {
        let (mut dom, mut session) = setup();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);
        wrap(parent, &mut session);
        wrap(child, &mut session);
        let r = Contains
            .invoke(
                parent,
                &[obj_ref_arg(child, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Bool(true));
    }

    #[test]
    fn contains_not_ancestor() {
        let (mut dom, mut session) = setup();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);
        wrap(parent, &mut session);
        wrap(child, &mut session);
        // child does NOT contain parent.
        let r = Contains
            .invoke(
                child,
                &[obj_ref_arg(parent, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    #[test]
    fn contains_disconnected() {
        let (mut dom, mut session) = setup();
        let a = dom.create_element("div", Attributes::default());
        let b = dom.create_element("span", Attributes::default());
        wrap(a, &mut session);
        wrap(b, &mut session);
        let r = Contains
            .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    #[test]
    fn contains_null() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let r = Contains
            .invoke(div, &[JsValue::Null], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // CompareDocumentPosition
    // -----------------------------------------------------------------------

    #[test]
    fn compare_position_same() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        wrap(div, &mut session);
        let r = CompareDocumentPosition
            .invoke(
                div,
                &[obj_ref_arg(div, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Number(0.0));
    }

    #[test]
    fn compare_position_following() {
        let (mut dom, mut session) = setup();
        let root = dom.create_document_root();
        let a = dom.create_element("a", Attributes::default());
        let b = dom.create_element("b", Attributes::default());
        dom.append_child(root, a);
        dom.append_child(root, b);
        wrap(a, &mut session);
        wrap(b, &mut session);
        let r = CompareDocumentPosition
            .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Number(f64::from(DOCUMENT_POSITION_FOLLOWING)));
    }

    #[test]
    fn compare_position_preceding() {
        let (mut dom, mut session) = setup();
        let root = dom.create_document_root();
        let a = dom.create_element("a", Attributes::default());
        let b = dom.create_element("b", Attributes::default());
        dom.append_child(root, a);
        dom.append_child(root, b);
        wrap(a, &mut session);
        wrap(b, &mut session);
        let r = CompareDocumentPosition
            .invoke(b, &[obj_ref_arg(a, &mut session)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Number(f64::from(DOCUMENT_POSITION_PRECEDING)));
    }

    #[test]
    fn compare_position_contains() {
        let (mut dom, mut session) = setup();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);
        wrap(parent, &mut session);
        wrap(child, &mut session);
        // child.compareDocumentPosition(parent) → parent CONTAINS child → CONTAINS | PRECEDING
        let r = CompareDocumentPosition
            .invoke(
                child,
                &[obj_ref_arg(parent, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(
            r,
            JsValue::Number(f64::from(
                DOCUMENT_POSITION_CONTAINS | DOCUMENT_POSITION_PRECEDING
            ))
        );
    }

    #[test]
    fn compare_position_contained_by() {
        let (mut dom, mut session) = setup();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(parent, child);
        wrap(parent, &mut session);
        wrap(child, &mut session);
        // parent.compareDocumentPosition(child) → this CONTAINED_BY child → CONTAINED_BY | FOLLOWING
        let r = CompareDocumentPosition
            .invoke(
                parent,
                &[obj_ref_arg(child, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(
            r,
            JsValue::Number(f64::from(
                DOCUMENT_POSITION_CONTAINED_BY | DOCUMENT_POSITION_FOLLOWING
            ))
        );
    }

    #[test]
    fn compare_position_disconnected() {
        let (mut dom, mut session) = setup();
        let a = dom.create_element("a", Attributes::default());
        let b = dom.create_element("b", Attributes::default());
        wrap(a, &mut session);
        wrap(b, &mut session);
        let r = CompareDocumentPosition
            .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
            .unwrap();
        if let JsValue::Number(v) = r {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let v = v as u32;
            assert!(v & DOCUMENT_POSITION_DISCONNECTED != 0);
            assert!(v & DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC != 0);
        } else {
            panic!("expected Number");
        }
    }

    // -----------------------------------------------------------------------
    // CloneNode
    // -----------------------------------------------------------------------

    #[test]
    fn clone_node_shallow() {
        let (mut dom, mut session) = setup();
        let mut attrs = Attributes::default();
        attrs.set("class", "test");
        let div = dom.create_element("div", attrs);
        let child = dom.create_text("hello");
        dom.append_child(div, child);
        wrap(div, &mut session);

        let r = CloneNode
            .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (cloned, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            // Tag preserved.
            assert_eq!(dom.world().get::<&TagType>(cloned).unwrap().0, "div");
            // Attributes preserved.
            assert_eq!(
                dom.world().get::<&Attributes>(cloned).unwrap().get("class"),
                Some("test")
            );
            // No children (shallow).
            assert!(dom.children(cloned).is_empty());
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn clone_node_deep() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("hello");
        dom.append_child(div, text);
        wrap(div, &mut session);

        let r = CloneNode
            .invoke(div, &[JsValue::Bool(true)], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (cloned, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            let children = dom.children(cloned);
            assert_eq!(children.len(), 1);
            let child_text = dom
                .world()
                .get::<&TextContent>(children[0])
                .unwrap()
                .0
                .clone();
            assert_eq!(child_text, "hello");
            // Cloned child is a different entity.
            assert_ne!(children[0], text);
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn clone_node_no_identity() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        wrap(div, &mut session);

        let r = CloneNode.invoke(div, &[], &mut session, &mut dom).unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (cloned, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            // Cloned entity is different from original.
            assert_ne!(cloned, div);
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn clone_node_no_inline_style() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        dom.world_mut()
            .insert_one(div, InlineStyle::default())
            .unwrap();
        wrap(div, &mut session);

        let r = CloneNode.invoke(div, &[], &mut session, &mut dom).unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (cloned, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            // InlineStyle should NOT be copied.
            assert!(dom.world().get::<&InlineStyle>(cloned).is_err());
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn clone_node_shadow_root_error() {
        let (mut dom, mut session) = setup();
        let host = dom.create_element("div", Attributes::default());
        let sr = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .unwrap();
        wrap(sr, &mut session);

        let r = CloneNode.invoke(sr, &[], &mut session, &mut dom);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().kind, DomApiErrorKind::NotSupportedError);
    }

    #[test]
    fn clone_node_document_type() {
        let (mut dom, mut session) = setup();
        let dt = dom.create_document_type("html", "-//W3C", "http://example.com");
        wrap(dt, &mut session);

        let r = CloneNode.invoke(dt, &[], &mut session, &mut dom).unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (cloned, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            let data = dom.world().get::<&DocTypeData>(cloned).unwrap();
            assert_eq!(data.name, "html");
            assert_eq!(data.public_id, "-//W3C");
            assert_eq!(data.system_id, "http://example.com");
            assert_ne!(cloned, dt);
        } else {
            panic!("expected ObjectRef");
        }
    }

    // -----------------------------------------------------------------------
    // Normalize
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_merge() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("hello ");
        let t2 = dom.create_text("world");
        dom.append_child(div, t1);
        dom.append_child(div, t2);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(div);
        assert_eq!(children.len(), 1);
        let text = dom
            .world()
            .get::<&TextContent>(children[0])
            .unwrap()
            .0
            .clone();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn normalize_remove_empty() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t = dom.create_text("");
        dom.append_child(div, t);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        assert!(dom.children(div).is_empty());
    }

    #[test]
    fn normalize_no_change() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let t = dom.create_text("hello");
        dom.append_child(div, span);
        dom.append_child(div, t);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        assert_eq!(dom.children(div).len(), 2);
    }

    #[test]
    fn normalize_recursive() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        let t1 = dom.create_text("a");
        let t2 = dom.create_text("b");
        dom.append_child(div, span);
        dom.append_child(span, t1);
        dom.append_child(span, t2);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        let span_children = dom.children(span);
        assert_eq!(span_children.len(), 1);
        let text = dom
            .world()
            .get::<&TextContent>(span_children[0])
            .unwrap()
            .0
            .clone();
        assert_eq!(text, "ab");
    }

    // -----------------------------------------------------------------------
    // IsConnected
    // -----------------------------------------------------------------------

    #[test]
    fn is_connected_true() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(doc, div);

        let r = IsConnected
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(true));
    }

    #[test]
    fn is_connected_false() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());

        let r = IsConnected
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    #[test]
    fn is_connected_detached() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(doc, div);
        dom.remove_child(doc, div);

        let r = IsConnected
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // GetRootNode
    // -----------------------------------------------------------------------

    #[test]
    fn get_root_node_connected() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(doc, div);
        wrap(doc, &mut session);

        let r = GetRootNode
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (root, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            assert_eq!(root, doc);
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn get_root_node_detached() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let child = dom.create_element("span", Attributes::default());
        dom.append_child(div, child);

        let r = GetRootNode
            .invoke(child, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (root, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            assert_eq!(root, div);
        } else {
            panic!("expected ObjectRef");
        }
    }

    // -----------------------------------------------------------------------
    // GetTextContentNodeKind
    // -----------------------------------------------------------------------

    #[test]
    fn text_content_get_element() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t = dom.create_text("hello");
        dom.append_child(div, t);

        let r = GetTextContentNodeKind
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::String("hello".into()));
    }

    #[test]
    fn text_content_get_document_null() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();

        let r = GetTextContentNodeKind
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Null);
    }

    #[test]
    fn text_content_get_doctype_null() {
        let (mut dom, mut session) = setup();
        let dt = dom.create_document_type("html", "", "");

        let r = GetTextContentNodeKind
            .invoke(dt, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Null);
    }

    #[test]
    fn text_content_get_comment() {
        let (mut dom, mut session) = setup();
        let comment = dom.create_comment("test comment");

        let r = GetTextContentNodeKind
            .invoke(comment, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::String("test comment".into()));
    }

    #[test]
    fn text_content_get_text_node() {
        let (mut dom, mut session) = setup();
        let t = dom.create_text("direct text");

        let r = GetTextContentNodeKind
            .invoke(t, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::String("direct text".into()));
    }

    #[test]
    fn text_content_get_element_empty() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());

        let r = GetTextContentNodeKind
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::String(String::new()));
    }

    // -----------------------------------------------------------------------
    // SetTextContentNodeKind
    // -----------------------------------------------------------------------

    #[test]
    fn text_content_set_element() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let old = dom.create_text("old");
        dom.append_child(div, old);

        SetTextContentNodeKind
            .invoke(
                div,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let children = dom.children(div);
        assert_eq!(children.len(), 1);
        let text = dom
            .world()
            .get::<&TextContent>(children[0])
            .unwrap()
            .0
            .clone();
        assert_eq!(text, "new");
        // Old child removed (different entity).
        assert_ne!(children[0], old);
    }

    #[test]
    fn text_content_set_document_noop() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let child = dom.create_element("html", Attributes::default());
        dom.append_child(doc, child);

        SetTextContentNodeKind
            .invoke(
                doc,
                &[JsValue::String("ignored".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // Children unchanged.
        assert_eq!(dom.children(doc).len(), 1);
    }

    #[test]
    fn text_content_set_comment() {
        let (mut dom, mut session) = setup();
        let comment = dom.create_comment("old");

        SetTextContentNodeKind
            .invoke(
                comment,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let data = dom.world().get::<&CommentData>(comment).unwrap().0.clone();
        assert_eq!(data, "new");
    }

    // -----------------------------------------------------------------------
    // SetNodeValue
    // -----------------------------------------------------------------------

    #[test]
    fn node_value_set_text() {
        let (mut dom, mut session) = setup();
        let t = dom.create_text("old");

        SetNodeValue
            .invoke(t, &[JsValue::String("new".into())], &mut session, &mut dom)
            .unwrap();

        let text = dom.world().get::<&TextContent>(t).unwrap().0.clone();
        assert_eq!(text, "new");
    }

    #[test]
    fn node_value_set_comment() {
        let (mut dom, mut session) = setup();
        let c = dom.create_comment("old");

        SetNodeValue
            .invoke(c, &[JsValue::String("new".into())], &mut session, &mut dom)
            .unwrap();

        let data = dom.world().get::<&CommentData>(c).unwrap().0.clone();
        assert_eq!(data, "new");
    }

    #[test]
    fn node_value_set_element_noop() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());

        let r = SetNodeValue
            .invoke(
                div,
                &[JsValue::String("ignored".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Undefined);
    }

    #[test]
    fn node_value_set_document_noop() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();

        let r = SetNodeValue
            .invoke(
                doc,
                &[JsValue::String("ignored".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Undefined);
    }

    // -----------------------------------------------------------------------
    // OwnerDocument
    // -----------------------------------------------------------------------

    #[test]
    fn owner_document_element() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(doc, div);
        wrap(doc, &mut session);

        let r = OwnerDocument
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(ref_id) = r {
            let (owner, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            assert_eq!(owner, doc);
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn owner_document_doc_null() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();

        let r = OwnerDocument
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Null);
    }

    // -----------------------------------------------------------------------
    // IsSameNode
    // -----------------------------------------------------------------------

    #[test]
    fn is_same_node_true() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        wrap(div, &mut session);

        let r = IsSameNode
            .invoke(
                div,
                &[obj_ref_arg(div, &mut session)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(r, JsValue::Bool(true));
    }

    #[test]
    fn is_same_node_false() {
        let (mut dom, mut session) = setup();
        let a = dom.create_element("div", Attributes::default());
        let b = dom.create_element("div", Attributes::default());
        wrap(a, &mut session);
        wrap(b, &mut session);

        let r = IsSameNode
            .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    #[test]
    fn is_same_node_null() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());

        let r = IsSameNode
            .invoke(div, &[JsValue::Null], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // IsEqualNode
    // -----------------------------------------------------------------------

    #[test]
    fn is_equal_node_true() {
        let (mut dom, mut session) = setup();
        let mut attrs = Attributes::default();
        attrs.set("id", "x");
        let a = dom.create_element("div", attrs.clone());
        let t1 = dom.create_text("hello");
        dom.append_child(a, t1);

        let b = dom.create_element("div", attrs);
        let t2 = dom.create_text("hello");
        dom.append_child(b, t2);

        wrap(a, &mut session);
        wrap(b, &mut session);

        let r = IsEqualNode
            .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(true));
    }

    #[test]
    fn is_equal_node_false() {
        let (mut dom, mut session) = setup();
        let a = dom.create_element("div", Attributes::default());
        let b = dom.create_element("span", Attributes::default());
        wrap(a, &mut session);
        wrap(b, &mut session);

        let r = IsEqualNode
            .invoke(a, &[obj_ref_arg(b, &mut session)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(r, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // Normalize — sibling-walk fix (H4)
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_adjacent_merge() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("hello ");
        let t2 = dom.create_text("world");
        dom.append_child(div, t1);
        dom.append_child(div, t2);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(div);
        assert_eq!(children.len(), 1);
        let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(tc.0, "hello world");
    }

    #[test]
    fn normalize_removes_empty() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("");
        let t2 = dom.create_text("hello");
        dom.append_child(div, t1);
        dom.append_child(div, t2);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(div);
        assert_eq!(children.len(), 1);
        let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(tc.0, "hello");
    }

    #[test]
    fn normalize_three_adjacent() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("a");
        let t2 = dom.create_text("b");
        let t3 = dom.create_text("c");
        dom.append_child(div, t1);
        dom.append_child(div, t2);
        dom.append_child(div, t3);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(div);
        assert_eq!(children.len(), 1);
        let tc = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(tc.0, "abc");
    }

    #[test]
    fn normalize_comment_boundary() {
        // Text nodes separated by a comment should NOT be merged.
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("before");
        let comment = dom.create_comment("separator");
        let t2 = dom.create_text("after");
        dom.append_child(div, t1);
        dom.append_child(div, comment);
        dom.append_child(div, t2);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        let children = dom.children(div);
        assert_eq!(children.len(), 3, "comment should prevent text merge");
        let tc1 = dom.world().get::<&TextContent>(children[0]).unwrap();
        assert_eq!(tc1.0, "before");
        let tc2 = dom.world().get::<&TextContent>(children[2]).unwrap();
        assert_eq!(tc2.0, "after");
    }

    #[test]
    fn normalize_all_empty_text() {
        // All empty text nodes should be removed.
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        let t1 = dom.create_text("");
        let t2 = dom.create_text("");
        dom.append_child(div, t1);
        dom.append_child(div, t2);

        Normalize.invoke(div, &[], &mut session, &mut dom).unwrap();

        assert!(dom.children(div).is_empty());
    }

    // -----------------------------------------------------------------------
    // OwnerDocument — disconnected nodes (H5)
    // -----------------------------------------------------------------------

    #[test]
    fn owner_document_orphan() {
        let (mut dom, mut session) = setup();
        let _doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        // div is orphaned (not appended to doc).
        wrap(div, &mut session);

        let result = OwnerDocument
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        // Should still return the document, not null.
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn owner_document_null_for_document() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        wrap(doc, &mut session);

        let result = OwnerDocument
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // -----------------------------------------------------------------------
    // GetRootNode — composed support (M5)
    // -----------------------------------------------------------------------

    #[test]
    fn get_root_node_non_composed_default() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(doc, div);
        wrap(div, &mut session);

        let result = GetRootNode
            .invoke(div, &[], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn get_root_node_composed_true() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let host = dom.create_element("div", Attributes::default());
        dom.append_child(doc, host);
        let sr = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .unwrap();
        let inner = dom.create_element("span", Attributes::default());
        dom.append_child(sr, inner);

        let result = GetRootNode
            .invoke(inner, &[JsValue::Bool(true)], &mut session, &mut dom)
            .unwrap();
        // Composed root should cross shadow boundary and reach the document.
        if let JsValue::ObjectRef(ref_id) = result {
            let (root, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            assert_eq!(root, doc);
        } else {
            panic!("expected ObjectRef");
        }
    }

    #[test]
    fn get_root_node_non_composed_stops_at_shadow() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        let host = dom.create_element("div", Attributes::default());
        dom.append_child(doc, host);
        let sr = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .unwrap();
        let inner = dom.create_element("span", Attributes::default());
        dom.append_child(sr, inner);

        let result = GetRootNode
            .invoke(inner, &[JsValue::Bool(false)], &mut session, &mut dom)
            .unwrap();
        // Non-composed root should stop at the shadow root.
        if let JsValue::ObjectRef(ref_id) = result {
            let (root, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            assert_eq!(root, sr);
        } else {
            panic!("expected ObjectRef");
        }
    }

    // -----------------------------------------------------------------------
    // textContent / nodeValue — CdataSection (M7) + existing behavior
    // -----------------------------------------------------------------------

    #[test]
    fn text_content_text_node_direct() {
        let (mut dom, mut session) = setup();
        let text = dom.create_text("hello");

        let result = GetTextContentNodeKind
            .invoke(text, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("hello".into()));
    }

    #[test]
    fn set_text_content_text_node_direct() {
        let (mut dom, mut session) = setup();
        let text = dom.create_text("old");

        SetTextContentNodeKind
            .invoke(
                text,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "new");
    }

    #[test]
    fn set_node_value_text_node_direct() {
        let (mut dom, mut session) = setup();
        let text = dom.create_text("old");

        SetNodeValue
            .invoke(
                text,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "new");
    }

    // -----------------------------------------------------------------------
    // CloneNode — ComponentKind (M6)
    // -----------------------------------------------------------------------

    #[test]
    fn clone_node_component_kind_element() {
        let (mut dom, mut session) = setup();
        let div = dom.create_element("div", Attributes::default());
        wrap(div, &mut session);

        let result = CloneNode
            .invoke(div, &[JsValue::Bool(false)], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn clone_node_component_kind_document() {
        let (mut dom, mut session) = setup();
        let doc = dom.create_document_root();
        wrap(doc, &mut session);

        let result = CloneNode
            .invoke(doc, &[JsValue::Bool(false)], &mut session, &mut dom)
            .unwrap();
        if let JsValue::ObjectRef(ref_id) = result {
            let (cloned, kind) = session
                .identity_map()
                .get(JsObjectRef::from_raw(ref_id))
                .unwrap();
            assert_ne!(cloned, doc);
            assert_eq!(kind, ComponentKind::Document);
        } else {
            panic!("expected ObjectRef");
        }
    }
}
