//! `node.cloneNode(deep?)` implementation.

use elidex_ecs::{
    Attributes, CommentData, DocTypeData, EcsDom, Entity, NodeKind, ShadowRoot, TagType,
    TextContent,
};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

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
