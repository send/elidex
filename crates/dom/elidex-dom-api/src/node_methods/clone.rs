//! `node.cloneNode(deep?)` — marshalling-only thin wrapper over
//! [`clone_node_with_shadow_honor`], the engine-indep WHATWG DOM §4.4
//! "clone a node" algorithm built on [`elidex_ecs::EcsDom::clone_subtree`]
//! (deep) and [`elidex_ecs::EcsDom::clone_node_shallow`] (shallow).
//!
//! Algorithmic concerns (per-NodeKind payload copy, descendant
//! traversal, AssociatedDocument propagation, ShadowRoot exclusion)
//! live in `elidex-ecs` so layout / parser / WPT runner consumers
//! see the same WHATWG §4.4 semantics through one entry point.  This
//! module owns the two per-pair post-passes the ECS cloner cannot
//! perform itself (see the clone-policy table in
//! `elidex_ecs`'s `tree_clone` module): `CustomElementState` identity
//! propagation and clonable-shadow-root replication.

use elidex_custom_elements::CustomElementState;
use elidex_ecs::{EcsDom, Entity, NodeKind, ShadowHost, ShadowInit, ShadowRoot};
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
        // the components in the clone-policy copy-set (TagType /
        // TextContent / CommentData / DocTypeData / Attributes /
        // Namespace / InlineStyle / IframeData) — so dispatching an
        // Attribute (`AttrData`), ProcessingInstruction (no payload
        // component yet), or Window (not a Node) entity through it
        // would yield a structurally invalid clone (NodeKind set,
        // payload missing).  ShadowRoot is rejected by spec (WHATWG
        // DOM §4.4 explicitly excludes shadow trees) and would
        // otherwise produce a fragment-shaped clone because the
        // cloner does NOT copy the `ShadowRoot` component — neither
        // the structural-invalid path nor the spec-error path is
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
        // to NotFoundError rather than NotSupportedError.
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

/// Engine-indep `Node.cloneNode(deep?)` algorithm — WHATWG DOM §4.4
/// "clone a node" with the two per-pair post-passes layered over the
/// ECS cloners:
///
/// 1. **`CustomElementState` identity propagation** ("clone a single
///    node" step 2.4 passes the source's *is value* to *create an
///    element*): for every `(src, dst)` pair whose source carries a
///    `CustomElementState`, the clone receives a fresh
///    `Undefined(definition_name)` — identity (which custom-element
///    name) propagates, lifecycle state does not (`Custom` / `Failed`
///    never carry over; NB no upgrade reaction is enqueued at clone
///    time yet — the clone upgrades via the insert-time consumer or
///    the next `define()` walk; the spec's clone-time enqueue is part
///    of the per-pair clone-reaction seam, slot
///    `#11-clone-cloning-steps-event`).  No tag inspection, `is=` attribute parsing,
///    or namespace guard happens here — those are creation-path
///    concerns (`CustomElementState::for_created_element`), and the
///    component's existence on the source already proves them.  An
///    element whose `is` attribute was set *after* creation has a
///    null is value per spec — and, matching that, no source
///    component — so its clone is correctly left unmarked.
///
/// 2. **Clonable-shadow-root replication** (step 6, which step 5's
///    per-child re-entry applies to *every* cloned node, not just the
///    root): for each pair whose source is a shadow host with
///    `clonable = true`, attach a fresh shadow root on the clone with
///    the same `ShadowInit` and clone the shadow children into it.
///    Per step 6.7 the shadow children are cloned with the *same*
///    `subtree` flag as the overall operation — so a shallow clone of
///    a shadow host still replicates its (shallowly-cloned) shadow
///    tree, while light-tree children are skipped entirely.
///
/// Both passes run off one worklist of `(src, dst)` pairs seeded by
/// the ECS cloner; shadow children cloned by pass 2 push their own
/// pairs onto the same worklist, so arbitrarily nested shadow trees
/// converge without recursion.
///
/// Returns `None` only when `src` does not exist in the ECS — matching
/// the `clone_subtree` / `clone_node_shallow` contract so the caller
/// can map that to `NotFoundError` once at the boundary.
#[must_use = "returns None when src does not exist"]
pub fn clone_node_with_shadow_honor(src: Entity, dom: &mut EcsDom, deep: bool) -> Option<Entity> {
    let mut pairs: Vec<(Entity, Entity)> = Vec::new();
    let cloned = clone_recording(dom, src, deep, None, &mut pairs)?;
    // The *node document* threaded through the spec recursion: a
    // Document clone becomes its own node document; everything else
    // keeps the source's.  The ECS cloner applies this to light-tree
    // descendants — the shadow-replication pass below threads the
    // SAME document into shadow subtrees via the cloner's
    // `doc_override` (deriving from the *source* child would stamp
    // the original document onto a cloned Document's shadow contents;
    // the shallow cloner stamps nothing, so the shallow branch stamps
    // the single replicated child explicitly).
    let shadow_doc: Option<Entity> = if matches!(dom.node_kind(src), Some(NodeKind::Document)) {
        Some(cloned)
    } else {
        dom.owner_document(src)
    };
    // Index-based worklist: pass 2 appends pairs while we iterate.
    let mut idx = 0;
    while idx < pairs.len() {
        let (s, d) = pairs[idx];
        idx += 1;
        propagate_ce_identity(s, d, dom);
        // Pass 3 (HTML §4.12.3 cloning steps): a cloned `<template>` gets its
        // own fresh content fragment, deep-cloned from the source's only when
        // the clone-children flag (`deep`) is set. Pushes the deep-clone's pairs
        // onto this same worklist, so templates nested inside template content
        // converge here too.
        replicate_template_contents(s, d, dom, deep, &mut pairs);
        let Some((init, src_shadow_root)) = read_clonable_shadow_init(s, dom) else {
            continue;
        };
        // `attach_shadow_with_init` can refuse (e.g. clone is a tag
        // outside the shadow-host allowlist) — fall through with the
        // base clone in that case, matching the silent-fallback shape
        // used by the declarative-shadow parser hook.
        let Ok(cloned_shadow) = dom.attach_shadow_with_init(d, init) else {
            continue;
        };
        // The spec threads ONE document through the whole clone
        // recursion, shadow trees included — `attach_shadow_with_init`
        // spawns the replicated shadow root without a node document,
        // and the shallow cloner stamps neither it nor the cloned
        // host, so `clonedHost.shadowRoot.ownerDocument` would
        // otherwise diverge from its own children.
        if let Some(doc) = shadow_doc {
            dom.set_associated_document(cloned_shadow, doc);
            dom.set_associated_document(d, doc);
        }
        let source_shadow_children: Vec<Entity> = dom.children(src_shadow_root);
        for child in source_shadow_children {
            // Step 6.7: shadow children clone with the operation's
            // own `subtree` flag, threading the operation's document.
            let Some(child_clone) = clone_recording(dom, child, deep, shadow_doc, &mut pairs)
            else {
                continue;
            };
            if !deep {
                // The shallow cloner never stamps AssociatedDocument;
                // mirror what the deep path's doc threading does for
                // the one node it produced.
                if let Some(doc) = shadow_doc {
                    dom.set_associated_document(child_clone, doc);
                }
            }
            let _ = dom.append_child(cloned_shadow, child_clone);
        }
    }
    Some(cloned)
}

/// Clone `node` (deep or shallow per `deep`) recording every `(src,
/// dst)` pair into `pairs` — the one place the "clone + record" shape
/// is spelled, so the worklist can never silently lose a subtree's
/// pairs (which would drop CE propagation and nested shadow honor for
/// it with no error).
fn clone_recording(
    dom: &mut EcsDom,
    node: Entity,
    deep: bool,
    doc_override: Option<Entity>,
    pairs: &mut Vec<(Entity, Entity)>,
) -> Option<Entity> {
    if deep {
        dom.clone_subtree(node, pairs, doc_override)
    } else {
        let cloned = dom.clone_node_shallow(node)?;
        pairs.push((node, cloned));
        Some(cloned)
    }
}

/// Pass 1 of [`clone_node_with_shadow_honor`]: propagate the custom
/// element *identity* (the is-value slot materialized as
/// `CustomElementState.definition_name`) from `src` to `dst`, resetting
/// the lifecycle state to `Undefined`.
///
/// This is the registry-INDEPENDENT half. The engine bindings run the
/// registry-dependent half over the finished clone subtree —
/// `elidex_custom_elements::apply_clone_creation_ce_semantics` — which
/// applies DOM §4.9 step 5.2's async-autonomous null-is rule (a clone
/// made while the autonomous definition is registered drops the
/// retained is value) and enqueues the §4.4 clone-time upgrade
/// reactions (Codex PR331 R13+R14).  No-op when the source carries
/// no `CustomElementState` — non-elements, ordinary built-ins, and
/// elements whose `is` attribute appeared only after creation (null is
/// value per spec).
fn propagate_ce_identity(src: Entity, dst: Entity, dom: &mut EcsDom) {
    let (name, is_value, registry) = match dom.world().get::<&CustomElementState>(src) {
        Ok(state) => (
            state.definition_name.clone(),
            state.is_value.clone(),
            state.registry,
        ),
        Err(_) => return,
    };
    let _ = dom.world_mut().insert_one(
        dst,
        CustomElementState {
            state: elidex_custom_elements::CEState::Undefined,
            definition_name: name,
            is_value,
            // DOM §4.4 "clone a single node" passes the source's
            // custom element registry through *create an element* —
            // a null-registry source yields a null-registry clone
            // (still never upgraded).
            registry,
        },
    );
}

/// Pass 3 of [`clone_node_with_shadow_honor`]: HTML §4.12.3 "cloning steps"
/// for a `<template>` element.
///
/// Per the spec every template clone has its own associated content
/// `DocumentFragment` (never the source's — the `TemplateContents` component
/// is in `tree_clone`'s deliberate-non-copy row precisely so a shallow clone
/// cannot alias it). The contents are cloned **only when the clone-children
/// flag is set** (`deep`): a shallow `cloneNode(false)` of a template still
/// gets a *fresh empty* fragment, while a deep clone deep-copies the source
/// fragment's children into it (threading the clone's node document, the C2
/// owner-doc approximation).
///
/// No-op when `src` is not a template (no `TemplateContents` link). Cloned
/// fragment children are recorded onto `pairs` via [`clone_recording`], so a
/// template nested inside template content is itself replicated when its pair
/// is reached.
fn replicate_template_contents(
    src: Entity,
    dst: Entity,
    dom: &mut EcsDom,
    deep: bool,
    pairs: &mut Vec<(Entity, Entity)>,
) {
    let Some(src_fragment) = dom.template_contents_fragment(src) else {
        return;
    };
    // The clone's content fragment's node document is the clone's own node
    // document (set by the ECS cloner's doc threading on `dst`).
    let owner = dom.owner_document(dst);
    let new_fragment = dom.attach_template_contents(dst, owner);
    if !deep {
        return;
    }
    for child in dom.children(src_fragment) {
        if let Some(child_clone) = clone_recording(dom, child, deep, owner, pairs) {
            let _ = dom.append_child(new_fragment, child_clone);
        }
    }
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
            // DOM §4.4 clone-a-node step 6.5 attaches the clone's
            // shadow with the source root's registry — a
            // null-registry shadow tree clones to a null-registry
            // shadow tree.
            null_registry: sr.null_registry,
        },
        shadow_root,
    ))
}
