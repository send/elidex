//! `Node.prototype` intrinsic (WHATWG DOM §4.4).
//!
//! Sits between `EventTarget.prototype` and `Element.prototype` in
//! the DOM wrapper chain:
//!
//! ```text
//! element wrapper (HostObject)
//!   → Element.prototype        (Element-only members)
//!     → Node.prototype         (this intrinsic)
//!       → EventTarget.prototype
//!         → Object.prototype
//!
//! text / comment / document wrapper (HostObject)
//!   → Node.prototype           (skip Element.prototype)
//!     → EventTarget.prototype
//!       → Object.prototype
//!
//! window (HostObject)
//!   → Window.prototype         (Window-only members)
//!     → EventTarget.prototype  (no Node members — Window is not a Node)
//!       → Object.prototype
//! ```
//!
//! Every DOM **Node** (Element, Text, Comment, Document,
//! DocumentFragment, …) reaches this prototype; Window and any
//! future `EventTarget`-but-not-`Node` host object (e.g. XHR,
//! AbortSignal) do not — so `typeof window.nodeType` remains
//! `"undefined"` the way the Web platform demands.
//!
//! Members installed here:
//!
//! - Accessors: `parentNode`, `parentElement`, `firstChild`,
//!   `lastChild`, `nextSibling`, `previousSibling`, `childNodes`,
//!   `nodeType`, `nodeName`, `nodeValue`, `textContent`,
//!   `isConnected`, `ownerDocument`.
//! - Methods:   `hasChildNodes`, `contains`, `appendChild`,
//!   `removeChild`, `insertBefore`, `replaceChild`, `cloneNode`,
//!   `isSameNode`, `getRootNode`, `isEqualNode`,
//!   `compareDocumentPosition`, `normalize`.
//!
//! Element-only members (`getAttribute`, `children`, `matches`, …)
//! live on `Element.prototype` which chains here.
//!
//! Every mutation / nodeValue / textContent / identity native here is
//! a thin binding that runs WebIDL coercion + brand check at the VM
//! boundary, then dispatches through
//! [`super::dom_bridge::invoke_dom_api`] to the engine-independent
//! handler in `elidex-dom-api`. The DOM mutation algorithm proper —
//! pre-insertion validity, ECS structural mutation, mutation-record
//! emission — lives there per the CLAUDE.md Layering mandate.
//! `cloneNode` / `isEqualNode` / `compareDocumentPosition` retain
//! their VM-side bodies in `node_methods_extras.rs`.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::tree_nav_getter;
use super::event_target::entity_from_this;
use super::node_methods_extras::{
    native_node_clone_node, native_node_compare_document_position, native_node_get_owner_document,
    native_node_get_root_node, native_node_is_equal_node, native_node_is_same_node,
    native_node_normalize,
};

use elidex_ecs::{Entity, NodeKind, TagType};

impl VmInner {
    /// Allocate `Node.prototype` and populate it with the Node-level
    /// accessors and methods.  Its parent is `EventTarget.prototype`.
    ///
    /// Called from `register_globals()` after
    /// `register_event_target_prototype` and **before**
    /// `register_element_prototype` (Element.prototype chains here).
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` has not been populated.
    pub(in crate::vm) fn register_node_prototype(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_node_prototype called before register_event_target_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.node_prototype = Some(proto_id);
        self.install_node_ro_accessors(proto_id);
        self.install_node_rw_accessors(proto_id);
        self.install_node_methods(proto_id);
    }

    fn install_node_ro_accessors(&mut self, proto_id: ObjectId) {
        for (name_sid, getter) in [
            (
                self.well_known.parent_node,
                native_node_get_parent_node as NativeFn,
            ),
            (
                self.well_known.parent_element,
                native_node_get_parent_element,
            ),
            (self.well_known.first_child, native_node_get_first_child),
            (self.well_known.last_child, native_node_get_last_child),
            (self.well_known.next_sibling, native_node_get_next_sibling),
            (
                self.well_known.previous_sibling,
                native_node_get_previous_sibling,
            ),
            (self.well_known.child_nodes, native_node_get_child_nodes),
            (self.well_known.node_type, native_node_get_node_type),
            (self.well_known.node_name, native_node_get_node_name),
            (self.well_known.is_connected, native_node_get_is_connected),
            (self.well_known.base_uri, native_node_get_base_uri),
            (
                self.well_known.owner_document,
                native_node_get_owner_document,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_node_rw_accessors(&mut self, proto_id: ObjectId) {
        // `WEBIDL_RO_ACCESSOR`'s `writable` bit is meaningless for
        // accessors — the setter slot is what makes these RW.
        for (name_sid, getter, setter) in [
            (
                self.well_known.node_value,
                native_node_get_node_value as NativeFn,
                native_node_set_node_value as NativeFn,
            ),
            (
                self.well_known.text_content,
                native_node_get_text_content,
                native_node_set_text_content,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_node_methods(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (
                self.well_known.has_child_nodes,
                native_node_has_child_nodes as NativeFn,
            ),
            (self.well_known.contains, native_node_contains),
            (self.well_known.append_child, native_node_append_child),
            (self.well_known.remove_child, native_node_remove_child),
            (self.well_known.insert_before, native_node_insert_before),
            (self.well_known.replace_child, native_node_replace_child),
            (self.well_known.clone_node, native_node_clone_node),
            (self.well_known.is_same_node, native_node_is_same_node),
            (self.well_known.get_root_node, native_node_get_root_node),
            (self.well_known.is_equal_node, native_node_is_equal_node),
            (
                self.well_known.compare_document_position,
                native_node_compare_document_position,
            ),
            (self.well_known.normalize, native_node_normalize),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Typed failure mode for the shared Node-argument extraction logic
/// (see [`extract_node_entity`]).  Each shaper —
/// [`require_node_arg`] and [`require_node_arg_required`] — maps these
/// variants to its own interface-scoped wording, so the wrong-type
/// and detached-entity distinctions survive re-scoping by construction
/// (no string-pattern coupling).
enum NodeArgFail {
    /// Value is not a `Node`-typed `HostObject` (primitive / wrong
    /// `ObjectKind` / canvas-2D context).
    NotANode,
    /// `HostObject.entity_bits` does not reconstruct a valid `Entity`
    /// (truly corrupt / recycled — generation zero, etc.).
    Detached,
}

/// Pure extraction of an `Entity` from a `JsValue` expected to be a
/// Node `HostObject`.  Returns a typed [`NodeArgFail`] so callers can
/// re-scope the resulting message to their own interface without
/// parsing strings.  See [`require_node_arg`] /
/// [`require_node_arg_required`] for the public shapers.
///
/// Rejects:
/// - values that are not `HostObject` wrappers,
/// - `HostObject`s whose `entity_bits` do not reconstruct a valid
///   `Entity` (truly corrupt / recycled),
/// - `HostObject`s whose entity is `NodeKind::Window` or has no
///   `NodeKind` component at all (e.g. a raw `HostObject`
///   placeholder).  Window is an `EventTarget` but not a Node in
///   WHATWG, so accepting it would let `document.appendChild(window)`
///   graft a non-Node into the DOM tree.
///
/// `host_if_bound` so a JS caller racing against `Vm::unbind()` gets
/// a fail-soft (returns `NotANode`) rather than the `host().dom()`
/// panic.
fn extract_node_entity(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<Entity, NodeArgFail> {
    let JsValue::Object(id) = value else {
        return Err(NodeArgFail::NotANode);
    };
    // Both Element and ShadowRoot wrappers are `HostObject` now —
    // ShadowRoot is identified by its ECS component, not by an
    // ObjectKind variant.  ShadowRoot remains accepted here per
    // WHATWG DOM §4.8 (ShadowRoot → DocumentFragment → Node);
    // insertion paths that must reject it call
    // [`reject_shadow_root_insertion`] separately.
    let entity = match ctx.vm.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => {
            Entity::from_bits(entity_bits).ok_or(NodeArgFail::Detached)?
        }
        _ => return Err(NodeArgFail::NotANode),
    };
    // Reverse half of the canvas-2D-context bidirectional brand: a
    // `CanvasRenderingContext2D` wrapper shares its `<canvas>` entity
    // (which IS a node), so `is_node()` alone would wrongly accept it
    // as a Node argument (e.g. `document.appendChild(ctx)`). Reject it
    // here — it brands as a context, not a node.
    if super::canvas::is_canvas_2d_context_wrapper(ctx.vm, id, entity) {
        return Err(NodeArgFail::NotANode);
    }
    // Use the inferred kind so legacy DOM entities (payload present
    // but no explicit `NodeKind` component) are accepted as Nodes —
    // matches `normalize_mixin_arg`, `nodes_equal`, and
    // `HostData::prototype_kind_for`.  `Window` and `Worker` are
    // `EventTarget`s but not Nodes, so they're rejected via
    // `NodeKind::is_node()`.  Destroyed entities return `None` here
    // and surface as the brand-check failure.
    let kind = ctx
        .host_if_bound()
        .and_then(|h| h.dom().node_kind_inferred(entity));
    match kind {
        Some(k) if k.is_node() => Ok(entity),
        _ => Err(NodeArgFail::NotANode),
    }
}

/// Extract an entity from a `JsValue` expected to be a Node
/// HostObject.  Used by every Node method that accepts a `Node`
/// argument — `contains`, `appendChild`, `removeChild`,
/// `insertBefore`, `replaceChild` — so WebIDL-style conversion and
/// error messages stay aligned across callers.  Errors are scoped to
/// the WebIDL `Node` interface; callers in a different interface
/// (observer family) use [`require_node_arg_required`] instead.
pub(super) fn require_node_arg(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    extract_node_entity(ctx, value).map_err(|f| match f {
        NodeArgFail::NotANode => VmError::type_error(format!(
            "Failed to execute '{method}' on 'Node': parameter is not of type 'Node'."
        )),
        NodeArgFail::Detached => VmError::type_error(format!(
            "Failed to execute '{method}' on 'Node': the node is detached (invalid entity)."
        )),
    })
}

/// Missing-arg wrapper that emits interface-scoped errors directly
/// (no post-hoc string rewriting): the typed [`NodeArgFail`] from
/// [`extract_node_entity`] is mapped per-variant into an error
/// scoped to the caller interface (`ResizeObserver` /
/// `IntersectionObserver` / `MutationObserver`).  Matches Chrome's
/// wording for e.g. `iObs.observe({})` →
/// `"Failed to execute 'observe' on 'IntersectionObserver': …"`.
pub(super) fn require_node_arg_required(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    interface: &str,
    method: &str,
) -> Result<Entity, VmError> {
    let value = arg.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': 1 argument required"
        ))
    })?;
    extract_node_entity(ctx, value).map_err(|f| match f {
        NodeArgFail::NotANode => VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': parameter 1 is not of type 'Node'."
        )),
        NodeArgFail::Detached => VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': the node is detached (invalid entity)."
        )),
    })
}

// ---------------------------------------------------------------------------
// Tree-navigation accessors
// ---------------------------------------------------------------------------

fn native_node_get_parent_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `shadowRoot.parentNode === null` per WHATWG DOM §4.8 — a
    // shadow root is not a tree child of its host even though the
    // ECS edge points back.  Filter the ECS parent for shadow root
    // receivers before lifting it to a wrapper.
    tree_nav_getter(ctx, this, |dom, e| {
        if dom.world().get::<&elidex_ecs::ShadowRoot>(e).is_ok() {
            return None;
        }
        dom.get_parent(e)
    })
}

/// `Node.prototype.parentElement` — returns the parent only if it is
/// itself an Element (WHATWG §4.4).  Defined on Node (not Element)
/// so `textNode.parentElement` works.  The document root has no
/// `TagType`, so `documentElement.parentElement === null`.
fn native_node_get_parent_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| {
        if dom.world().get::<&elidex_ecs::ShadowRoot>(e).is_ok() {
            return None;
        }
        match dom.get_parent(e) {
            Some(p) if dom.world().get::<&TagType>(p).is_ok() => Some(p),
            _ => None,
        }
    })
}

fn native_node_get_first_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `children_iter` skips `ShadowRoot` entities so `firstChild`
    // never leaks a shadow root — matches `childNodes` and the Web
    // platform.
    tree_nav_getter(ctx, this, |dom, e| dom.children_iter(e).next())
}

fn native_node_get_last_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Same shadow-root-skipping rationale as `firstChild`.
    tree_nav_getter(ctx, this, |dom, e| dom.children_iter_rev(e).next())
}

fn native_node_get_next_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Shadow roots have no siblings per WHATWG §4.8 (the shadow
    // root isn't a tree child of its host even though the ECS edge
    // points back).  Same rationale as `parentNode`.
    tree_nav_getter(ctx, this, |dom, e| {
        if dom.world().get::<&elidex_ecs::ShadowRoot>(e).is_ok() {
            return None;
        }
        dom.next_exposed_sibling(e)
    })
}

fn native_node_get_previous_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| {
        if dom.world().get::<&elidex_ecs::ShadowRoot>(e).is_ok() {
            return None;
        }
        dom.prev_exposed_sibling(e)
    })
}

fn native_node_get_child_nodes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Live `NodeList` per WHATWG §4.2.5 — subsequent reads of
    // `.length` / indexed access / iteration reflect child mutations
    // made after the collection was obtained.  Shares the
    // `live_collection_states` infrastructure with HTMLCollection
    // (see `dom_collection.rs`).
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::ChildNodes,
        elidex_dom_api::CollectionKind::NodeList,
    ));
    Ok(JsValue::Object(id))
}

/// `Node.prototype.baseURI` getter (D-31; WHATWG DOM §4.4 Interface
/// Node `baseURI` getter, anchor `#dom-node-baseuri`).  Routes
/// through the `node.baseURI.get` handler which resolves the owner
/// document (Phase B: single-doc EcsDom == document_root) and reads
/// the cached `DocumentBaseUrl` component.
fn native_node_get_base_uri(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    super::dom_bridge::invoke_dom_api(ctx, "node.baseURI.get", entity, &[])
}

fn native_node_get_is_connected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    // WHATWG §4.4: connected iff the shadow-including root is the
    // document.  We approximate that by walking the composed tree
    // via `find_tree_root_composed` — if the resulting root matches
    // the bound `document_entity`, the node is considered connected.
    // Full shadow-aware semantics follow alongside Custom Elements.
    let dom = ctx.host().dom();
    let root = dom.find_tree_root_composed(entity);
    let connected = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(super::super::host_data::HostData::document_entity_opt)
        .is_some_and(|doc| root == doc);
    Ok(JsValue::Boolean(connected))
}

// ---------------------------------------------------------------------------
// Node info — nodeType / nodeName / nodeValue
// ---------------------------------------------------------------------------

fn native_node_get_node_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(0.0));
    };
    let kind = ctx.host().dom().node_kind(entity);
    let n = kind.map_or(0, NodeKind::node_type);
    Ok(JsValue::Number(f64::from(n)))
}

fn native_node_get_node_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enum NodeNameKind {
        Upper(String),
        Hash(StringId),
        Empty,
    }
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let kind = {
        let dom = ctx.host().dom();
        let upper = dom.with_tag_name(entity, |t| t.map(str::to_ascii_uppercase));
        if let Some(s) = upper {
            NodeNameKind::Upper(s)
        } else {
            match dom.node_kind(entity) {
                Some(NodeKind::Text) => NodeNameKind::Hash(ctx.vm.well_known.hash_text),
                Some(NodeKind::Comment) => NodeNameKind::Hash(ctx.vm.well_known.hash_comment),
                Some(NodeKind::Document) => NodeNameKind::Hash(ctx.vm.well_known.hash_document),
                Some(NodeKind::DocumentFragment) => {
                    NodeNameKind::Hash(ctx.vm.well_known.hash_document_fragment)
                }
                _ => NodeNameKind::Empty,
            }
        }
    };
    match kind {
        NodeNameKind::Hash(sid) => Ok(JsValue::String(sid)),
        NodeNameKind::Empty => Ok(JsValue::String(ctx.vm.well_known.empty)),
        NodeNameKind::Upper(upper) => {
            let sid = ctx.vm.strings.intern(&upper);
            Ok(JsValue::String(sid))
        }
    }
}

fn native_node_get_node_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    super::dom_bridge::invoke_dom_api(ctx, "nodeValue.get", entity, &[])
}

/// `nodeValue` setter — spec-defined only for character-data (Text
/// / Comment) nodes; no-op otherwise.
fn native_node_set_node_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // WHATWG §4.4 nodeValue setter step 1: WebIDL `DOMString?`, with
    // `[LegacyNullToEmptyString]` semantics — `null` is the literal
    // empty string, every other value goes through ToString.  The
    // coercion happens at the boundary so the handler sees only a
    // primitive `JsValue::String`.
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let coerced = coerce_legacy_null_to_empty_string(ctx, arg)?;
    super::dom_bridge::invoke_dom_api(ctx, "nodeValue.set", entity, &[coerced])
}

// ---------------------------------------------------------------------------
// textContent getter / setter
// ---------------------------------------------------------------------------

fn native_node_get_text_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    super::dom_bridge::invoke_dom_api(ctx, "textContent.get", entity, &[])
}

fn native_node_set_text_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // WHATWG §4.4 textContent setter: same `[LegacyNullToEmptyString]`
    // contract as nodeValue — null → empty string, else ToString.
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let coerced = coerce_legacy_null_to_empty_string(ctx, arg)?;
    super::dom_bridge::invoke_dom_api(ctx, "textContent.set", entity, &[coerced])
}

/// Apply WebIDL `[LegacyNullToEmptyString]`-style coercion at the bridge
/// boundary: `null` becomes `""`; anything else routes through ToString
/// before reaching the handler.  Used by `nodeValue` / `textContent`
/// setters which are spec-typed `DOMString?` with the legacy null mapping.
fn coerce_legacy_null_to_empty_string(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<JsValue, VmError> {
    match value {
        JsValue::Null => Ok(JsValue::String(ctx.vm.well_known.empty)),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            Ok(JsValue::String(sid))
        }
    }
}

// ---------------------------------------------------------------------------
// hasChildNodes / contains
// ---------------------------------------------------------------------------

fn native_node_has_child_nodes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    // Use `children_iter` (skips shadow roots) so a host whose only
    // child is a shadow root reports `false`, consistent with
    // `childNodes.length === 0`.
    Ok(JsValue::Boolean(
        ctx.host().dom().children_iter(entity).next().is_some(),
    ))
}

fn native_node_contains(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(self_entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    // WebIDL: `boolean contains(Node? other)` — `null` / `undefined`
    // short-circuit to `false` without throwing; any other non-Node
    // value (arbitrary object, Window, …) is a WebIDL conversion
    // failure and throws `TypeError`.  Delegate to `require_node_arg`
    // once the nullable case is handled.
    let other_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(other_arg, JsValue::Null | JsValue::Undefined) {
        return Ok(JsValue::Boolean(false));
    }
    let other_entity = require_node_arg(ctx, other_arg, "contains")?;
    if self_entity == other_entity {
        return Ok(JsValue::Boolean(true));
    }
    // Use the shadow-boundary-aware ancestor check so
    // `host.contains(nodeInsideShadow)` returns `false` (shadow
    // roots are not light-tree descendants of their host).
    Ok(JsValue::Boolean(
        ctx.host()
            .dom()
            .is_light_tree_ancestor_or_self(self_entity, other_entity),
    ))
}

// ---------------------------------------------------------------------------
// DOM mutation — appendChild / removeChild / insertBefore / replaceChild
// ---------------------------------------------------------------------------

/// Reject `value` if it wraps a `ShadowRoot` — WHATWG DOM §4.2.3
/// "ensure pre-insert validity" treats shadow roots as
/// non-insertable: they belong to exactly one host and moving them
/// into a different parent would break the shadow encapsulation
/// invariant (host → shadow_root unique edge).  All mutation
/// natives that take a "node to insert" arg (`appendChild`,
/// `insertBefore`'s newChild, `replaceChild`'s newChild) call this
/// AFTER [`require_node_arg`] so non-Node args still surface as
/// TypeError before the shadow-specific HierarchyRequestError fires.
fn reject_shadow_root_insertion(
    ctx: &NativeContext<'_>,
    value: JsValue,
    interface: &str,
    method: &str,
) -> Result<(), VmError> {
    let JsValue::Object(id) = value else {
        return Ok(());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Ok(());
    };
    let Some(entity) = Entity::from_bits(entity_bits) else {
        return Ok(());
    };
    if !super::event_target::is_shadow_root_entity(ctx.vm, entity) {
        return Ok(());
    }
    let hierarchy_request = ctx.vm.well_known.dom_exc_hierarchy_request_error;
    Err(VmError::dom_exception(
        hierarchy_request,
        format!(
            "Failed to execute '{method}' on '{interface}': \
             a ShadowRoot cannot be moved into the light DOM"
        ),
    ))
}

fn native_node_append_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let child_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // Brand-check before dispatch so non-Node args raise TypeError
    // rather than the generic HierarchyRequestError the handler would
    // emit if `prepare_arg` somehow let a non-HostObject through. The
    // extracted Entity is intentionally discarded — `prepare_arg`
    // decodes it again from `child_arg` (duplicate work the bridge
    // accepts in exchange for a clean brand-check seam).
    require_node_arg(ctx, child_arg, "appendChild")?;
    reject_shadow_root_insertion(ctx, child_arg, "Node", "appendChild")?;
    super::dom_bridge::invoke_dom_api(ctx, "appendChild", parent, &[child_arg])
}

fn native_node_remove_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let child_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    require_node_arg(ctx, child_arg, "removeChild")?;
    // `removeChild` accepts ShadowRoot in principle (it's a Node)
    // but a ShadowRoot is never a `parent.children[i]` in the light
    // tree, so the existing engine "not a child" check rejects it
    // naturally without a special-case here.
    super::dom_bridge::invoke_dom_api(ctx, "removeChild", parent, &[child_arg])
}

fn native_node_insert_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let new_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    require_node_arg(ctx, new_arg, "insertBefore")?;
    reject_shadow_root_insertion(ctx, new_arg, "Node", "insertBefore")?;
    // `refChild` is `Node?` per WHATWG: `null` / `undefined` mean
    // "append at end"; the InsertBefore handler interprets the second
    // arg the same way (matches `args.get(1) == None | Some(Null)`).
    let ref_arg = args.get(1).copied().unwrap_or(JsValue::Null);
    if !matches!(ref_arg, JsValue::Null | JsValue::Undefined) {
        require_node_arg(ctx, ref_arg, "insertBefore")?;
    }
    super::dom_bridge::invoke_dom_api(ctx, "insertBefore", parent, &[new_arg, ref_arg])
}

fn native_node_replace_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let new_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    require_node_arg(ctx, new_arg, "replaceChild")?;
    reject_shadow_root_insertion(ctx, new_arg, "Node", "replaceChild")?;
    let old_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    require_node_arg(ctx, old_arg, "replaceChild")?;
    super::dom_bridge::invoke_dom_api(ctx, "replaceChild", parent, &[new_arg, old_arg])
}

// The `ownerDocument` / `isSameNode` / `getRootNode` /
// `compareDocumentPosition` / `isEqualNode` / `cloneNode` bodies live
// in `node_methods_extras.rs` so this file stays under the 1000-line
// convention.  Install-time references come through the `use` at the
// top of this file.
