//! Core Node interface methods: contains, compareDocumentPosition, normalize,
//! isConnected, getRootNode, ownerDocument, isSameNode, isEqualNode.

use elidex_ecs::{EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiError, DomApiHandler, JsObjectRef, SessionCore};

use super::{find_root, find_root_non_composed, nodes_equal, resolve_optional_entity};
use crate::util::{not_found_error, require_object_ref_arg};

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
pub(crate) const DOCUMENT_POSITION_DISCONNECTED: u32 = 1;
pub(crate) const DOCUMENT_POSITION_PRECEDING: u32 = 2;
pub(crate) const DOCUMENT_POSITION_FOLLOWING: u32 = 4;
pub(crate) const DOCUMENT_POSITION_CONTAINS: u32 = 8;
pub(crate) const DOCUMENT_POSITION_CONTAINED_BY: u32 = 16;
pub(crate) const DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: u32 = 32;

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
            return Ok(JsValue::Number(f64::from(
                DOCUMENT_POSITION_CONTAINS | DOCUMENT_POSITION_PRECEDING,
            )));
        }
        if dom.is_ancestor_or_self(this, other) && this != other {
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
// 4. Normalize
// ---------------------------------------------------------------------------

/// `node.normalize()` — merge adjacent text nodes, remove empty text nodes.
pub struct Normalize;

impl Normalize {
    pub(crate) fn normalize_entity(entity: Entity, dom: &mut EcsDom) {
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
                    let ok = dom.remove_child(entity, child);
                    debug_assert!(ok, "remove_child: child from get_first_child walk");
                    dom.rev_version(entity);
                    current = next;
                    continue;
                }

                if let Some(prev) = prev_text {
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
        if dom.node_kind(this) == Some(NodeKind::Document) {
            return Ok(JsValue::Null);
        }

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
        let _ = dom;
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
