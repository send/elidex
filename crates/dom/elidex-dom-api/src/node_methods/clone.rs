//! `node.cloneNode(deep?)` — marshalling-only thin wrapper over
//! [`elidex_ecs::EcsDom::clone_subtree`] (deep) and
//! [`elidex_ecs::EcsDom::clone_node_shallow`] (shallow).
//!
//! Algorithmic concerns (per-NodeKind payload copy, descendant
//! traversal, AssociatedDocument propagation, ShadowRoot exclusion)
//! live in `elidex-ecs` so layout / parser / WPT runner consumers
//! see the same WHATWG §4.5 semantics through one entry point.

use elidex_custom_elements::{is_valid_custom_element_name, CustomElementState};
use elidex_ecs::{EcsDom, Entity, NodeKind, ShadowHost, ShadowInit, ShadowRoot, TagType};
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
        // ECS cloners return `None` only when `this` no longer exists
        // in the world (per their `#[must_use]` contract), so map that
        // to NotFoundError rather than NotSupportedError. Shadow root
        // honouring (HTML §4.7.10 step 5 "if node is a shadow host
        // with a clonable shadow root, clone declarative shadow root")
        // lives in [`clone_node_with_shadow_honor`] so the algorithm
        // is reusable from outside the DomApiHandler dispatcher.
        let Some(cloned) = clone_node_with_shadow_honor(this, dom, deep) else {
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

/// Engine-indep `Node.cloneNode(deep?)` algorithm with HTML §4.7.10
/// step 5 declarative-shadow-root honouring.
///
/// Wraps `EcsDom::clone_subtree` / `clone_node_shallow` (which by
/// invariant never copy `ShadowRoot` / `ShadowHost` components) with a
/// shadow-root replication pass: when `deep == true` and `src` itself
/// is a shadow host whose shadow root has `clonable = true`, attach a
/// fresh shadow root on the clone with the same `ShadowInit` and
/// deep-clone each shadow-tree child into it.
///
/// Returns `None` only when `src` does not exist in the ECS — matching
/// the `clone_subtree` / `clone_node_shallow` contract so the caller
/// can map that to `NotFoundError` once at the boundary.
///
/// Shadow honouring applies to `src` only; per-descendant shadow
/// hosts retain ECS clone semantics (no shadow copied). Lifting that
/// limitation requires a parallel src↔dst entity mapping (deferred to
/// `#11-clone-shadow-descendant-hosts` if/when surfaced by tests).
#[must_use = "returns None when src does not exist"]
pub fn clone_node_with_shadow_honor(src: Entity, dom: &mut EcsDom, deep: bool) -> Option<Entity> {
    let cloned = if deep {
        dom.clone_subtree(src)?
    } else {
        dom.clone_node_shallow(src)?
    };
    // Shallow clone: only the root needs CE state re-attach.
    if !deep {
        attach_ce_state_on_clone(cloned, dom);
        return Some(cloned);
    }
    // Deep clone: handle the shadow tree first so the subsequent
    // shadow-including descendant walk covers BOTH the light tree AND
    // the cloned shadow children in one pass — fixes the gap where the
    // shadow-cloned subtree previously bypassed CE state attach.
    if let Some((init, src_shadow_root)) = read_clonable_shadow_init(src, dom) {
        // `attach_shadow_with_init` can refuse (e.g. clone is a tag
        // outside the shadow-host allowlist) — fall through with the
        // base clone in that case, matching the silent-fallback shape
        // used by the declarative-shadow parser hook.
        if let Ok(cloned_shadow) = dom.attach_shadow_with_init(cloned, init) {
            let source_shadow_children: Vec<Entity> = dom.children(src_shadow_root);
            for child in source_shadow_children {
                if let Some(child_clone) = dom.clone_subtree(child) {
                    let _ = dom.append_child(cloned_shadow, child_clone);
                }
            }
        }
    }
    // HTML §4.5 "clone a node" step 6: if the source element is a
    // custom element (any state other than Uncustomized), the clone
    // must also be queued for upgrade. The engine-indep `clone_subtree`
    // / `clone_node_shallow` cloners (in elidex-ecs) cannot reach
    // `elidex-custom-elements` types due to the inverse cargo edge, so
    // the CE state component is re-attached here at the engine-indep
    // DOM-api boundary. Always in `Undefined` — the clone goes through
    // the upgrade pipeline fresh; the source's Custom/Failed state does
    // NOT carry over.
    //
    // Walks shadow-including descendants so the cloned shadow subtree
    // (attached above) is covered in the same pass — without that, CE
    // elements nested inside a cloned shadow root would remain CE-
    // state-less and silently skip upgrade.
    let mut to_attach: Vec<Entity> = Vec::new();
    dom.for_each_shadow_inclusive_descendant(cloned, &mut |entity| to_attach.push(entity));
    for entity in to_attach {
        attach_ce_state_on_clone(entity, dom);
    }
    Some(cloned)
}

/// Re-attach `CustomElementState::undefined(tag)` to a cloned element
/// whose tag is a valid custom element name (HTML §4.13.3 "valid
/// custom element name" + WHATWG DOM §4.5 "clone a node" step 6).
/// No-op for non-Element nodes, non-hyphenated tags, and entities
/// that already carry the component.
fn attach_ce_state_on_clone(entity: Entity, dom: &mut EcsDom) {
    let tag = match dom.world().get::<&TagType>(entity) {
        Ok(t) => t.0.clone(),
        Err(_) => return,
    };
    if !is_valid_custom_element_name(&tag) {
        return;
    }
    if dom.world().get::<&CustomElementState>(entity).is_ok() {
        return;
    }
    let _ = dom
        .world_mut()
        .insert_one(entity, CustomElementState::undefined(&tag));
}

/// Extract the `ShadowInit` for `entity`'s shadow root if it is a
/// shadow host whose root opts into cloning. Returns the init and the
/// source shadow root entity so the caller can also enumerate its
/// children.
fn read_clonable_shadow_init(entity: Entity, dom: &EcsDom) -> Option<(ShadowInit, Entity)> {
    let shadow_root = dom.world().get::<&ShadowHost>(entity).ok()?.shadow_root;
    let sr = dom.world().get::<&ShadowRoot>(shadow_root).ok()?;
    if !sr.clonable {
        return None;
    }
    Some((
        ShadowInit {
            mode: sr.mode,
            delegates_focus: sr.delegates_focus,
            slot_assignment: sr.slot_assignment,
            clonable: sr.clonable,
            serializable: sr.serializable,
        },
        shadow_root,
    ))
}
