//! `node.cloneNode(deep?)` — marshalling-only thin wrapper over
//! [`elidex_ecs::EcsDom::clone_subtree`] (deep) and
//! [`elidex_ecs::EcsDom::clone_node_shallow`] (shallow).
//!
//! Algorithmic concerns (per-NodeKind payload copy, descendant
//! traversal, AssociatedDocument propagation, ShadowRoot exclusion)
//! live in `elidex-ecs` so layout / parser / WPT runner consumers
//! see the same WHATWG §4.5 semantics through one entry point.

use elidex_ecs::{EcsDom, Entity, ShadowRoot};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

/// `node.cloneNode(deep?)` — clone a node (optionally deep).
pub struct CloneNode;

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
        // ShadowRoot cannot be cloned (WHATWG §4.5 explicitly
        // excludes shadow trees).  Reject before dispatching to ECS
        // so the error surfaces with the spec-mandated DOMException
        // name rather than as a silent `None` from the cloner.
        if dom.world().get::<&ShadowRoot>(this).is_ok() {
            return Err(DomApiError {
                kind: DomApiErrorKind::NotSupportedError,
                message: "cloneNode: ShadowRoot cannot be cloned".into(),
            });
        }

        let deep = matches!(args.first(), Some(JsValue::Bool(true)));
        let cloned = if deep {
            dom.clone_subtree(this)
        } else {
            dom.clone_node_shallow(this)
        };
        let Some(cloned) = cloned else {
            return Err(DomApiError {
                kind: DomApiErrorKind::NotSupportedError,
                message: "cloneNode: source not found".into(),
            });
        };

        let kind = dom
            .node_kind(cloned)
            .map_or(ComponentKind::Element, ComponentKind::from_node_kind);
        let obj_ref = session.get_or_create_wrapper(cloned, kind);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}
