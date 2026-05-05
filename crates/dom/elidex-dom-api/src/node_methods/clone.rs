//! `node.cloneNode(deep?)` — marshalling-only thin wrapper over
//! [`elidex_ecs::EcsDom::clone_subtree`] (deep) and
//! [`elidex_ecs::EcsDom::clone_node_shallow`] (shallow).
//!
//! Algorithmic concerns (per-NodeKind payload copy, descendant
//! traversal, AssociatedDocument propagation, ShadowRoot exclusion)
//! live in `elidex-ecs` so layout / parser / WPT runner consumers
//! see the same WHATWG §4.5 semantics through one entry point.

use elidex_ecs::{EcsDom, Entity, NodeKind, ShadowRoot};
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
        // Reject up front any node whose payload the ECS cloners do
        // not handle.  `clone_node_shallow_unchecked` snapshots only
        // TagType / TextContent / CommentData / DocTypeData /
        // Attributes — so dispatching an Attribute (`AttrData`),
        // ProcessingInstruction (no payload component yet), or
        // Window (not a Node) entity through it would yield a
        // structurally invalid clone (NodeKind set, payload
        // missing).  ShadowRoot is rejected by spec (WHATWG §4.5
        // explicitly excludes shadow trees) and would otherwise
        // produce a fragment-shaped clone because the cloner does
        // NOT copy the `ShadowRoot` component — neither the
        // structural-invalid path nor the spec-error path is
        // acceptable, so refuse early with the spec-named
        // DOMException.
        if dom.world().get::<&ShadowRoot>(this).is_ok() {
            return Err(DomApiError {
                kind: DomApiErrorKind::NotSupportedError,
                message: "cloneNode: ShadowRoot cannot be cloned".into(),
            });
        }
        match dom.node_kind(this) {
            Some(NodeKind::Attribute) => {
                return Err(DomApiError {
                    kind: DomApiErrorKind::NotSupportedError,
                    message: "cloneNode: Attribute cloning is not yet supported".into(),
                });
            }
            Some(NodeKind::ProcessingInstruction) => {
                return Err(DomApiError {
                    kind: DomApiErrorKind::NotSupportedError,
                    message: "cloneNode: ProcessingInstruction cloning is not yet supported".into(),
                });
            }
            Some(NodeKind::Window) => {
                // Window is NOT a Node per WHATWG DOM (no nodeType,
                // EventTarget mixin only), so an explicit
                // `Node.prototype.cloneNode.call(window)` from JS is
                // a receiver-type mismatch.  Per WebIDL §3.6.5
                // "illegal invocation" that must surface as a plain
                // TypeError, not a DOMException — DOMException is
                // reserved for Node receivers whose operation can't
                // be performed (e.g. Attribute / ProcessingInstruction
                // above) rather than for "this isn't a Node at all".
                return Err(DomApiError {
                    kind: DomApiErrorKind::TypeError,
                    message: "cloneNode: Window is not a Node".into(),
                });
            }
            _ => {}
        }

        let deep = matches!(args.first(), Some(JsValue::Bool(true)));
        let cloned = if deep {
            dom.clone_subtree(this)
        } else {
            dom.clone_node_shallow(this)
        };
        // ECS cloners return `None` only when `this` no longer
        // exists in the world (per their `#[must_use]` contract), so
        // map that to NotFoundError rather than NotSupportedError.
        let Some(cloned) = cloned else {
            return Err(DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "cloneNode: source entity does not exist".into(),
            });
        };

        let kind = dom
            .node_kind(cloned)
            .map_or(ComponentKind::Element, ComponentKind::from_node_kind);
        let obj_ref = session.get_or_create_wrapper(cloned, kind);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}
