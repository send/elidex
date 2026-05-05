//! Core Node interface methods: contains, compareDocumentPosition, normalize,
//! isConnected, getRootNode, ownerDocument, isSameNode, isEqualNode.
//!
//! `compareDocumentPosition` and `isEqualNode` are marshalling-only
//! delegates over [`elidex_ecs::EcsDom::compare_document_position`] /
//! [`elidex_ecs::EcsDom::nodes_equal`].  Algorithmic concerns
//! (bitmask construction, Attr-vs-Attr handling, light-tree
//! traversal, deep equality) live in `elidex-ecs` per the CLAUDE.md
//! Layering mandate.

use elidex_ecs::{EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiError, DomApiHandler, JsObjectRef, SessionCore};

use super::{find_root, find_root_non_composed, resolve_optional_entity};
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

/// `node.compareDocumentPosition(other)` — returns a WHATWG DOM §4.4
/// bitmask describing `other`'s position relative to `this`.
///
/// Marshalling-only: delegates to
/// [`elidex_ecs::EcsDom::compare_document_position`] for the actual
/// algorithm (bitmask, Attr-substitute-ownerElement, light-tree
/// traversal, disconnected entity-bits ordering).
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
        Ok(JsValue::Number(f64::from(
            dom.compare_document_position(this, other),
        )))
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

/// `node.ownerDocument` getter (WHATWG DOM §4.4).
///
/// Resolution order (each step short-circuits on hit):
/// 1. Document receiver → `null` (per spec).
/// 2. Per-entity [`elidex_ecs::AssociatedDocument`] component → that
///    Document (so `clonedDoc.createElement(…)` reports the clone, not
///    the singleton).
/// 3. Tree-root walk: if root is a Document, return it.
/// 4. Singleton fallback: [`EcsDom::document_root`].
/// 5. `null` only when no Document has been created in this DOM.
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
        // WHATWG §4.4: Document.ownerDocument === null. `owner_document`
        // already returns None for that branch (it short-circuits before
        // touching the AssociatedDocument or tree-root walk).
        if dom.node_kind(this) == Some(NodeKind::Document) {
            return Ok(JsValue::Null);
        }
        // 1. Per-entity AssociatedDocument lookup (preserves
        //    `clonedDoc.createElement(...)` reporting the clone).
        // 2. Fall back to the singleton document_root so orphans created
        //    via `EcsDom::create_element` without an explicit owner still
        //    report the bound document — matches the pre-arch-hoist
        //    VM-side `host_data.document_entity_opt()` fallback.
        let doc = dom.owner_document(this).or_else(|| dom.document_root());
        match doc {
            Some(d) => {
                let obj_ref = session.get_or_create_wrapper(d, ComponentKind::Document);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
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
///
/// Marshalling-only: delegates to [`elidex_ecs::EcsDom::nodes_equal`]
/// for the iterative-stack equality walk + per-NodeKind payload
/// comparison.
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
        Ok(JsValue::Bool(
            other.is_some_and(|b| dom.nodes_equal(this, b)),
        ))
    }
}
