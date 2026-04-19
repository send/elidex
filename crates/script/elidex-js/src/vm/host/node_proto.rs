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
//!   `isSameNode`, `getRootNode`.
//!
//! Element-only members (`getAttribute`, `children`, `matches`, …)
//! live on `Element.prototype` which chains here.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::tree_nav_getter;
use super::event_target::entity_from_this;

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
            (
                self.well_known.owner_document,
                native_node_get_owner_document,
            ),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_node_rw_accessors(&mut self, proto_id: ObjectId) {
        // `WEBIDL_RO_ACCESSOR`'s `writable` bit is meaningless for
        // accessors — the setter slot is what makes these RW.
        let rw_attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
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
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let sid = self.create_native_function(&format!("set {name}"), setter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: Some(sid),
                },
                rw_attrs,
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
        ] {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                shape::PropertyAttrs::METHOD,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract an entity from a `JsValue` expected to be a Node
/// HostObject.  Used by every Node method that accepts a `Node`
/// argument — `contains`, `appendChild`, `removeChild`,
/// `insertBefore`, `replaceChild` — so WebIDL-style conversion and
/// error messages stay aligned across callers.
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
fn require_node_arg(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let not_a_node = || -> VmError {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Node': parameter is not of type 'Node'."
        ))
    };
    let id = match value {
        JsValue::Object(id) => id,
        _ => return Err(not_a_node()),
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(not_a_node());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Node': the node is detached (invalid entity)."
        ))
    })?;
    match ctx.host().dom().node_kind(entity) {
        None | Some(NodeKind::Window) => Err(not_a_node()),
        Some(_) => Ok(entity),
    }
}

// ---------------------------------------------------------------------------
// Tree-navigation accessors
// ---------------------------------------------------------------------------

fn native_node_get_parent_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| dom.get_parent(e))
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
    tree_nav_getter(ctx, this, |dom, e| match dom.get_parent(e) {
        Some(p) if dom.world().get::<&TagType>(p).is_ok() => Some(p),
        _ => None,
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
    tree_nav_getter(ctx, this, |dom, e| dom.next_exposed_sibling(e))
}

fn native_node_get_previous_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| dom.prev_exposed_sibling(e))
}

fn native_node_get_child_nodes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Phase 2: return a plain JS array (static snapshot) rather than
    // a live NodeList.  Full NodeList semantics land with Observers /
    // CE lifecycle in a later PR.
    let children: Vec<Entity> = ctx.host().dom().children_iter(entity).collect();
    let elements: Vec<JsValue> = children
        .into_iter()
        .map(|e| JsValue::Object(ctx.vm.create_element_wrapper(e)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elements)))
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
        .and_then(|hd| hd.document_entity_opt())
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
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    enum NodeNameKind {
        Tag(String),
        Hash(StringId),
        Empty,
    }
    let kind = {
        let dom = ctx.host().dom();
        if let Some(tag) = dom.get_tag_name(entity) {
            NodeNameKind::Tag(tag)
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
        NodeNameKind::Tag(tag) => {
            let upper = tag.to_ascii_uppercase();
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
    let data: Option<String> = {
        let dom = ctx.host().dom();
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(entity) {
            Some(text.0.clone())
        } else if let Ok(c) = dom.world().get::<&elidex_ecs::CommentData>(entity) {
            Some(c.0.clone())
        } else {
            None
        }
    };
    match data {
        Some(s) => {
            let sid = ctx.vm.strings.intern(&s);
            Ok(JsValue::String(sid))
        }
        None => Ok(JsValue::Null),
    }
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
    // WHATWG §4.4: nodeValue setter treats null as empty string; every
    // other value is coerced via ToString.
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data: String = match arg {
        JsValue::Null => String::new(),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    {
        let dom = ctx.host().dom();
        let is_text = dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok();
        if is_text {
            if let Ok(mut text) = dom.world_mut().get::<&mut elidex_ecs::TextContent>(entity) {
                text.0 = data;
            }
        } else if dom.world().get::<&elidex_ecs::CommentData>(entity).is_ok() {
            if let Ok(mut c) = dom.world_mut().get::<&mut elidex_ecs::CommentData>(entity) {
                c.0 = data;
            }
        }
    }
    Ok(JsValue::Undefined)
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
    let result: Option<String> = {
        let dom = ctx.host().dom();
        // Character-data nodes return their own data directly.
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(entity) {
            Some(text.0.clone())
        } else if let Ok(c) = dom.world().get::<&elidex_ecs::CommentData>(entity) {
            Some(c.0.clone())
        } else {
            // WHATWG §4.4: only Element and DocumentFragment
            // concatenate descendant Text data.  Document,
            // DocumentType, and everything else return `null` — in
            // particular `document.textContent === null` in every
            // major browser.
            let kind = dom.node_kind(entity);
            match kind {
                Some(NodeKind::Element | NodeKind::DocumentFragment) => {
                    let mut buf = String::new();
                    dom.traverse_descendants(entity, |e| {
                        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
                            buf.push_str(&text.0);
                        }
                        true
                    });
                    Some(buf)
                }
                _ => None,
            }
        }
    };
    match result {
        Some(s) => {
            if s.is_empty() {
                Ok(JsValue::String(ctx.vm.well_known.empty))
            } else {
                let sid = ctx.vm.strings.intern(&s);
                Ok(JsValue::String(sid))
            }
        }
        None => Ok(JsValue::Null),
    }
}

fn native_node_set_text_content(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data: String = match arg {
        JsValue::Null => String::new(),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    {
        let dom = ctx.host().dom();
        // Character-data fast path.  Perform the type check with a
        // shared borrow, then upgrade to a mutable one for the
        // actual mutation — hecs's `RefMut` destructor re-enters the
        // world, so two back-to-back `get::<&mut _>` calls in the
        // same scope would clash even with ok/err short-circuits.
        let is_text = dom.world().get::<&elidex_ecs::TextContent>(entity).is_ok();
        if is_text {
            if let Ok(mut text) = dom.world_mut().get::<&mut elidex_ecs::TextContent>(entity) {
                text.0 = data;
            }
            return Ok(JsValue::Undefined);
        }
        let is_comment = dom.world().get::<&elidex_ecs::CommentData>(entity).is_ok();
        if is_comment {
            if let Ok(mut c) = dom.world_mut().get::<&mut elidex_ecs::CommentData>(entity) {
                c.0 = data;
            }
            return Ok(JsValue::Undefined);
        }
        // WHATWG §4.4: only Element and DocumentFragment replace
        // children.  Document.textContent = …  is a no-op — every
        // other node kind (including Document) falls through here.
        let kind = dom.node_kind(entity);
        if !matches!(kind, Some(NodeKind::Element | NodeKind::DocumentFragment)) {
            return Ok(JsValue::Undefined);
        }
        // Remove every existing child.  Collect first to avoid
        // mutating the sibling chain mid-iteration.
        let existing: Vec<Entity> = dom.children_iter(entity).collect();
        for child in existing {
            let _ = dom.remove_child(entity, child);
        }
        if !data.is_empty() {
            let text_entity = dom.create_text(data);
            let _ = dom.append_child(entity, text_entity);
        }
    }
    Ok(JsValue::Undefined)
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

fn native_node_append_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let child_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let child = require_node_arg(ctx, child_arg, "appendChild")?;
    let ok = ctx.host().dom().append_child(parent, child);
    if !ok {
        // WHATWG §4.5 pre-insertion validity — the lifecycle
        // violations EcsDom rejects (self-append, cycle, destroyed
        // entity) are spec'd as HierarchyRequestError.  Phase 2
        // surfaces them as TypeError with a descriptive message;
        // DOMException integration lands with the shell in a later PR.
        return Err(VmError::type_error(
            "Failed to execute 'appendChild' on 'Node': the new child node cannot be inserted.",
        ));
    }
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(child)))
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
    let child = require_node_arg(ctx, child_arg, "removeChild")?;
    let ok = ctx.host().dom().remove_child(parent, child);
    if !ok {
        // `NotFoundError` in the spec — surfaced as TypeError per the
        // DOMException deferral documented on `appendChild`.
        return Err(VmError::type_error(
            "Failed to execute 'removeChild' on 'Node': \
             The node to be removed is not a child of this node.",
        ));
    }
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(child)))
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
    let new_node = require_node_arg(ctx, new_arg, "insertBefore")?;
    // `ref_node` may be `null` → append at end.
    let ref_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    match ref_arg {
        JsValue::Null | JsValue::Undefined => {
            if !ctx.host().dom().append_child(parent, new_node) {
                return Err(VmError::type_error(
                    "Failed to execute 'insertBefore' on 'Node': \
                     the new child node cannot be inserted.",
                ));
            }
        }
        _ => {
            let ref_node = require_node_arg(ctx, ref_arg, "insertBefore")?;
            if !ctx.host().dom().insert_before(parent, new_node, ref_node) {
                return Err(VmError::type_error(
                    "Failed to execute 'insertBefore' on 'Node': \
                     the reference node is not a child of this node.",
                ));
            }
        }
    }
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_node)))
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
    let new_node = require_node_arg(ctx, new_arg, "replaceChild")?;
    let old_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let old_node = require_node_arg(ctx, old_arg, "replaceChild")?;
    if !ctx.host().dom().replace_child(parent, new_node, old_node) {
        return Err(VmError::type_error(
            "Failed to execute 'replaceChild' on 'Node': \
             the node to be replaced is not a child of this node.",
        ));
    }
    // Spec: returns the *replaced* (old) node.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(old_node)))
}

// ---------------------------------------------------------------------------
// ownerDocument / isSameNode / getRootNode (WHATWG DOM §4.4)
// ---------------------------------------------------------------------------

/// `Node.prototype.ownerDocument` — WHATWG §4.4.  Returns `null` for
/// the document itself (including detached/unbound wrappers); for any
/// other Node we return the bound `document` entity, consistent with
/// elidex's single-document model.  Multi-document (iframe, fragments
/// created via `DOMImplementation`) support lands with Workers.
fn native_node_get_owner_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Document itself: ownerDocument is null.
    if matches!(ctx.host().dom().node_kind(entity), Some(NodeKind::Document)) {
        return Ok(JsValue::Null);
    }
    let doc = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.document_entity_opt());
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, doc))
}

/// `Node.prototype.isSameNode(other)` — WHATWG §4.4.  Legacy alias of
/// `===`: returns true iff `this` and `other` are the same wrapper.
fn native_node_is_same_node(
    _ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let other = args.first().copied().unwrap_or(JsValue::Undefined);
    // WebIDL `Node? other`: null / undefined short-circuit to false.
    if matches!(other, JsValue::Null | JsValue::Undefined) {
        return Ok(JsValue::Boolean(false));
    }
    // WHATWG legacy alias — strict equality on the `Object` wrapper.
    let same = matches!(
        (this, other),
        (JsValue::Object(a), JsValue::Object(b)) if a == b
    );
    Ok(JsValue::Boolean(same))
}

/// `Node.prototype.getRootNode(options?)` — WHATWG §4.4.  Returns the
/// root of the composed tree if `{composed: true}`, otherwise the
/// light-tree root (shadow boundary respected).
fn native_node_get_root_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Read `composed` from the options argument.  WebIDL treats a
    // non-object argument as a zero-filled dictionary — per spec, a
    // primitive / null / undefined all yield the defaults.
    let composed = match args.first().copied() {
        Some(JsValue::Object(opts_id)) => {
            let v = ctx.vm.get_property_value(
                opts_id,
                super::super::value::PropertyKey::String(ctx.vm.well_known.composed),
            )?;
            super::super::coerce::to_boolean(ctx.vm, v)
        }
        _ => false,
    };
    let root = {
        let dom = ctx.host().dom();
        if composed {
            dom.find_tree_root_composed(entity)
        } else {
            dom.find_tree_root(entity)
        }
    };
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(root)))
}

// ---------------------------------------------------------------------------
// compareDocumentPosition (WHATWG DOM §4.4)
// ---------------------------------------------------------------------------

/// `Node.prototype.compareDocumentPosition(other)` — returns a bit
/// flag describing the relative position of `other` to `this`.
///
/// Bit values (WHATWG §4.4):
/// - `0x01 DOCUMENT_POSITION_DISCONNECTED`
/// - `0x02 DOCUMENT_POSITION_PRECEDING`
/// - `0x04 DOCUMENT_POSITION_FOLLOWING`
/// - `0x08 DOCUMENT_POSITION_CONTAINS`
/// - `0x10 DOCUMENT_POSITION_CONTAINED_BY`
/// - `0x20 DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC` — always 0
///   (implementation-defined per spec, elidex returns 0 by choice).
///
/// `this === other` → `0`.  Non-Node argument throws TypeError.
///
/// Shadow-tree awareness is light-tree only in Phase 2; full
/// shadow-including semantics land with Custom Elements (PR5b).
fn native_node_compare_document_position(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    const DISCONNECTED: u32 = 0x01;
    const PRECEDING: u32 = 0x02;
    const FOLLOWING: u32 = 0x04;
    const CONTAINS: u32 = 0x08;
    const CONTAINED_BY: u32 = 0x10;

    let Some(self_entity) = entity_from_this(ctx, this) else {
        // Unbound receiver — nothing to compare against; return
        // DISCONNECTED.  (Browsers throw TypeError here, but our
        // policy for unbound receivers is the softer silent path.)
        return Ok(JsValue::Number(f64::from(DISCONNECTED)));
    };
    let other_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let other_entity = require_node_arg(ctx, other_arg, "compareDocumentPosition")?;
    if self_entity == other_entity {
        return Ok(JsValue::Number(0.0));
    }
    let dom = ctx.host().dom();
    // Containment.
    if dom.is_light_tree_ancestor_or_self(other_entity, self_entity) {
        // `other` contains `this`.  Spec: CONTAINS | PRECEDING.
        return Ok(JsValue::Number(f64::from(CONTAINS | PRECEDING)));
    }
    if dom.is_light_tree_ancestor_or_self(self_entity, other_entity) {
        return Ok(JsValue::Number(f64::from(CONTAINED_BY | FOLLOWING)));
    }
    // Either siblings (with common ancestor) or disconnected subtrees.
    // Use `tree_order_cmp` — returns Ordering that already handles
    // both disjoint-tree fallback and sibling-order comparison.
    if dom.find_tree_root(self_entity) != dom.find_tree_root(other_entity) {
        // Disjoint trees: spec requires DISCONNECTED |
        // IMPLEMENTATION_SPECIFIC | PRECEDING/FOLLOWING based on
        // implementation choice.  We always return
        // DISCONNECTED | PRECEDING for a stable comparator.
        return Ok(JsValue::Number(f64::from(DISCONNECTED | PRECEDING)));
    }
    match dom.tree_order_cmp(self_entity, other_entity) {
        std::cmp::Ordering::Less => Ok(JsValue::Number(f64::from(FOLLOWING))),
        std::cmp::Ordering::Greater => Ok(JsValue::Number(f64::from(PRECEDING))),
        std::cmp::Ordering::Equal => Ok(JsValue::Number(0.0)),
    }
}

// ---------------------------------------------------------------------------
// isEqualNode (WHATWG DOM §4.4 "equals" algorithm)
// ---------------------------------------------------------------------------

/// `Node.prototype.isEqualNode(other)` — structural deep equality.
///
/// Returns `true` iff both nodes have:
/// - the same `NodeKind`,
/// - the same node name (tag for Elements, fixed `#text` / `#comment` /
///   `#document` / `#document-fragment` for others),
/// - identical attribute sets (names and values, order-independent),
///   for Elements,
/// - identical character-data (for Text / Comment),
/// - the same number of children, each pair of which is isEqualNode.
///
/// Event listeners are ignored.  WebIDL `Node? other`: null / undefined
/// → `false`.
fn native_node_is_equal_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(self_entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let other_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(other_arg, JsValue::Null | JsValue::Undefined) {
        return Ok(JsValue::Boolean(false));
    }
    // Non-Node argument (not a wrapper, Window, etc.) is treated as
    // not-equal rather than a TypeError — WHATWG §4.4 says the spec
    // does not require type coercion for isEqualNode (the step 1
    // early-return leaks out a `false` for unreachable branches).
    let other_entity = match require_node_arg(ctx, other_arg, "isEqualNode") {
        Ok(e) => e,
        Err(_) => return Ok(JsValue::Boolean(false)),
    };
    let equal = {
        let dom = ctx.host().dom();
        nodes_equal(dom, self_entity, other_entity)
    };
    Ok(JsValue::Boolean(equal))
}

/// Structural deep-equality for two Node entities.  Walks children
/// via `children_iter` (shadow-root entities are skipped in both
/// subtrees, matching WHATWG light-tree semantics).
fn nodes_equal(dom: &elidex_ecs::EcsDom, a: elidex_ecs::Entity, b: elidex_ecs::Entity) -> bool {
    if dom.node_kind(a) != dom.node_kind(b) {
        return false;
    }
    // Tag comparison (Element only).
    let tag_a = dom.get_tag_name(a);
    let tag_b = dom.get_tag_name(b);
    if tag_a != tag_b {
        return false;
    }
    // Attribute sets — order-independent (WHATWG §4.4 "equals").
    if !attributes_equal(dom, a, b) {
        return false;
    }
    // Character data.
    let text_a = dom.world().get::<&elidex_ecs::TextContent>(a).ok();
    let text_b = dom.world().get::<&elidex_ecs::TextContent>(b).ok();
    match (text_a.as_deref(), text_b.as_deref()) {
        (Some(x), Some(y)) if x.0 != y.0 => return false,
        (Some(_), None) | (None, Some(_)) => return false,
        _ => {}
    }
    drop((text_a, text_b));
    let comment_a = dom.world().get::<&elidex_ecs::CommentData>(a).ok();
    let comment_b = dom.world().get::<&elidex_ecs::CommentData>(b).ok();
    match (comment_a.as_deref(), comment_b.as_deref()) {
        (Some(x), Some(y)) if x.0 != y.0 => return false,
        (Some(_), None) | (None, Some(_)) => return false,
        _ => {}
    }
    drop((comment_a, comment_b));
    // Children pairwise.
    let kids_a: Vec<elidex_ecs::Entity> = dom.children_iter(a).collect();
    let kids_b: Vec<elidex_ecs::Entity> = dom.children_iter(b).collect();
    if kids_a.len() != kids_b.len() {
        return false;
    }
    for (ca, cb) in kids_a.iter().zip(kids_b.iter()) {
        if !nodes_equal(dom, *ca, *cb) {
            return false;
        }
    }
    true
}

/// Attribute-set equality — same keys, same values, order-independent.
fn attributes_equal(
    dom: &elidex_ecs::EcsDom,
    a: elidex_ecs::Entity,
    b: elidex_ecs::Entity,
) -> bool {
    let attrs_a = dom.world().get::<&elidex_ecs::Attributes>(a).ok();
    let attrs_b = dom.world().get::<&elidex_ecs::Attributes>(b).ok();
    match (attrs_a, attrs_b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().all(|(k, v)| b.get(k) == Some(v))
        }
        // One side has an `Attributes` component, the other does not —
        // treat an absent component as an empty attribute set.  Elements
        // without attributes sometimes carry the component (default),
        // sometimes not, depending on construction path; normalise here.
        (Some(a), None) => a.is_empty(),
        (None, Some(b)) => b.is_empty(),
    }
}

// ---------------------------------------------------------------------------
// cloneNode (WHATWG DOM §4.5)
// ---------------------------------------------------------------------------

/// `Node.prototype.cloneNode(deep?)` — allocate a new entity carrying
/// the same `NodeKind` and payload as `this`.
///
/// Behaviour:
/// - `deep` is coerced via `ToBoolean`; default is `false` (shallow).
/// - Shallow clone copies attributes (Element) or character data
///   (Text / Comment) only.
/// - Deep clone additionally recurses through all light-tree
///   descendants via [`elidex_ecs::EcsDom::clone_subtree`].
/// - The returned wrapper's entity has no parent or siblings
///   (WHATWG §4.5 "cloning steps" — the clone is an orphan).
/// - Event listeners and shadow roots are **not** cloned.  WHATWG
///   §4.5 explicitly excludes both; [`elidex_ecs::EcsDom::clone_subtree`]
///   enforces it.
fn native_node_clone_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(src) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let deep_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let deep = super::super::coerce::to_boolean(ctx.vm, deep_arg);

    let new_entity = {
        let dom = ctx.host().dom();
        let new_root = dom.clone_subtree(src);
        if !deep {
            // Shallow: drop the cloned children so only the root
            // survives.  `clone_subtree` is the only ECS entry
            // point for cloning today — destroying the freshly
            // linked children here keeps the public ECS surface
            // narrow (no dedicated `clone_node_shallow`), at the
            // cost of one extra allocation + despawn for shallow
            // deep-tree cases.  Profile-driven optimisation (e.g.
            // a native `EcsDom::clone_node_shallow`) is a straight
            // swap if profiles demand it.
            let kids: Vec<elidex_ecs::Entity> = dom.children_iter(new_root).collect();
            for child in kids {
                despawn_subtree(dom, child);
            }
        }
        new_root
    };
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

/// Recursively despawn `entity` and everything underneath it, used to
/// pare a clone subtree back to its root for the shallow path.
/// [`elidex_ecs::EcsDom::destroy_entity`] only removes one node, so
/// children would otherwise leak.
fn despawn_subtree(dom: &mut elidex_ecs::EcsDom, entity: elidex_ecs::Entity) {
    let kids: Vec<elidex_ecs::Entity> = dom.children_iter(entity).collect();
    for c in kids {
        despawn_subtree(dom, c);
    }
    let _ = dom.destroy_entity(entity);
}
