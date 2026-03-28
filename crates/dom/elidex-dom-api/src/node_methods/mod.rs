//! Node interface method implementations as `DomApiHandler` trait impls.
//!
//! Covers: `contains`, `compareDocumentPosition`, `cloneNode`, `normalize`,
//! `isConnected`, `getRootNode`, `textContent` (NodeKind-aware), `nodeValue`,
//! `ownerDocument`, `isSameNode`, `isEqualNode`.

mod clone;
mod core;
mod text_content;

pub use clone::CloneNode;
pub use core::{
    CompareDocumentPosition, Contains, GetRootNode, IsConnected, IsEqualNode, IsSameNode,
    Normalize, OwnerDocument,
};
#[cfg(test)]
pub(crate) use core::{
    DOCUMENT_POSITION_CONTAINED_BY, DOCUMENT_POSITION_CONTAINS, DOCUMENT_POSITION_DISCONNECTED,
    DOCUMENT_POSITION_FOLLOWING, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC,
    DOCUMENT_POSITION_PRECEDING,
};
pub use text_content::{GetTextContentNodeKind, SetNodeValue, SetTextContentNodeKind};

use elidex_ecs::{
    Attributes, CommentData, DocTypeData, EcsDom, Entity, NodeKind, TagType, TextContent,
};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind, JsObjectRef, SessionCore};

use crate::util::not_found_error;

// ---------------------------------------------------------------------------
// Shared helpers (used by sub-modules)
// ---------------------------------------------------------------------------

/// Resolve an optional `ObjectRef` arg to an Entity. Returns `None` if the arg
/// is `Null`, `Undefined`, or missing.
pub(crate) fn resolve_optional_entity(
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

/// Walk ancestors to find the composed tree root (crosses shadow boundaries).
pub(crate) fn find_root(entity: Entity, dom: &EcsDom) -> Entity {
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

/// Walk ancestors to find the tree root, stopping at `ShadowRoot` boundaries
/// (non-composed root). Delegates to `EcsDom::find_tree_root`.
pub(crate) fn find_root_non_composed(entity: Entity, dom: &EcsDom) -> Entity {
    dom.find_tree_root(entity)
}

/// Check deep structural equality of two nodes.
pub(crate) fn nodes_equal(a: Entity, b: Entity, dom: &EcsDom) -> bool {
    let kind_a = dom.node_kind(a);
    let kind_b = dom.node_kind(b);
    if kind_a != kind_b {
        return false;
    }

    match kind_a {
        Some(NodeKind::Element) => {
            let tag_a = dom.world().get::<&TagType>(a).ok().map(|t| t.0.clone());
            let tag_b = dom.world().get::<&TagType>(b).ok().map(|t| t.0.clone());
            if tag_a != tag_b {
                return false;
            }
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
        Some(NodeKind::Document | NodeKind::DocumentFragment) => {}
        _ => {
            if kind_a.is_none() {
                return false;
            }
        }
    }

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

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
