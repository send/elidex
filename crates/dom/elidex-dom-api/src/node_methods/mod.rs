//! Node interface method implementations as `DomApiHandler` trait impls.
//!
//! Covers: `contains`, `compareDocumentPosition`, `cloneNode`, `normalize`,
//! `isConnected`, `getRootNode`, `textContent` (NodeKind-aware), `nodeValue`,
//! `ownerDocument`, `isSameNode`, `isEqualNode`.
//!
//! `cloneNode` / `compareDocumentPosition` / `isEqualNode` are
//! marshalling-only thin wrappers over `elidex-ecs` primitives —
//! tree-walking algorithms (clone subtree, position bitmask, deep
//! equality) live engine-independently in [`elidex_ecs::EcsDom`]
//! per the CLAUDE.md Layering mandate.

mod clone;
mod core;
mod text_content;

pub use clone::CloneNode;
pub use core::{
    CompareDocumentPosition, Contains, GetRootNode, IsConnected, IsEqualNode, IsSameNode,
    Normalize, OwnerDocument,
};
#[cfg(test)]
pub(crate) use elidex_ecs::{
    DOCUMENT_POSITION_CONTAINED_BY, DOCUMENT_POSITION_CONTAINS, DOCUMENT_POSITION_DISCONNECTED,
    DOCUMENT_POSITION_FOLLOWING, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC,
    DOCUMENT_POSITION_PRECEDING,
};
pub use text_content::{GetTextContentNodeKind, SetNodeValue, SetTextContentNodeKind};

use elidex_ecs::{EcsDom, Entity};
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

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
