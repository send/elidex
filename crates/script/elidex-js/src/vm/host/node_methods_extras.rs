//! PR4e additions to `Node.prototype` — split out of `node_proto.rs`
//! so that file stays under the project's 1000-line convention.
//!
//! Installed by `install_node_methods` / `install_node_ro_accessors`
//! via the `pub(super)` native-fn pointers below.  Implementation
//! bodies only; the shape of `Node.prototype` itself is defined in
//! `node_proto.rs`.
//!
//! Members:
//!
//! - Accessors: `ownerDocument`.
//! - Methods:   `isSameNode`, `getRootNode`, `compareDocumentPosition`,
//!   `isEqualNode`, `cloneNode`.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, PropertyKey, VmError};
use super::dom_bridge::wrap_entity_or_null;
use super::event_target::entity_from_this;
use super::node_proto::require_node_arg;

use elidex_ecs::{Entity, NodeKind};

// ---------------------------------------------------------------------------
// ownerDocument / isSameNode / getRootNode (WHATWG DOM §4.4)
// ---------------------------------------------------------------------------

/// `Node.prototype.ownerDocument` — WHATWG §4.4.  Returns `null` for
/// the document itself (including detached/unbound wrappers); for any
/// other Node we return the bound `document` entity, consistent with
/// elidex's single-document model.  Multi-document (iframe, fragments
/// created via `DOMImplementation`) support lands with Workers.
pub(super) fn native_node_get_owner_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    if matches!(ctx.host().dom().node_kind(entity), Some(NodeKind::Document)) {
        return Ok(JsValue::Null);
    }
    let doc = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.document_entity_opt());
    Ok(wrap_entity_or_null(ctx.vm, doc))
}

/// `Node.prototype.isSameNode(other)` — WHATWG §4.4.  Legacy alias of
/// `===`: returns true iff `this` and `other` are the same wrapper.
pub(super) fn native_node_is_same_node(
    _ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let other = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(other, JsValue::Null | JsValue::Undefined) {
        return Ok(JsValue::Boolean(false));
    }
    let same = matches!(
        (this, other),
        (JsValue::Object(a), JsValue::Object(b)) if a == b
    );
    Ok(JsValue::Boolean(same))
}

/// `Node.prototype.getRootNode(options?)` — WHATWG §4.4.  Returns the
/// root of the composed tree if `{composed: true}`, otherwise the
/// light-tree root (shadow boundary respected).
pub(super) fn native_node_get_root_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // WebIDL treats a non-object argument as a zero-filled
    // dictionary — primitive / null / undefined all yield defaults.
    let composed = match args.first().copied() {
        Some(JsValue::Object(opts_id)) => {
            let v = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(ctx.vm.well_known.composed))?;
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
pub(super) fn native_node_compare_document_position(
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
        // Unbound receiver: fall through to DISCONNECTED.  Browsers
        // throw TypeError here, but elidex's unbound-receiver policy
        // is the softer silent path.
        return Ok(JsValue::Number(f64::from(DISCONNECTED)));
    };
    let other_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let other_entity = require_node_arg(ctx, other_arg, "compareDocumentPosition")?;
    if self_entity == other_entity {
        return Ok(JsValue::Number(0.0));
    }
    let dom = ctx.host().dom();
    if dom.is_light_tree_ancestor_or_self(other_entity, self_entity) {
        return Ok(JsValue::Number(f64::from(CONTAINS | PRECEDING)));
    }
    if dom.is_light_tree_ancestor_or_self(self_entity, other_entity) {
        return Ok(JsValue::Number(f64::from(CONTAINED_BY | FOLLOWING)));
    }
    if dom.find_tree_root(self_entity) != dom.find_tree_root(other_entity) {
        // Disjoint trees: spec permits any stable comparator; we
        // always return DISCONNECTED | PRECEDING.
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
/// - the same DocTypeData (for DocumentType),
/// - the same number of children, each pair of which is isEqualNode.
///
/// Event listeners are ignored.  WebIDL `Node? other`: null / undefined
/// → `false`.
pub(super) fn native_node_is_equal_node(
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
    // Non-Node argument resolves to `false` rather than TypeError —
    // WHATWG §4.4 step 1 leaks `false` for unreachable branches.
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
fn nodes_equal(dom: &elidex_ecs::EcsDom, a: Entity, b: Entity) -> bool {
    let kind = dom.node_kind(a);
    if kind != dom.node_kind(b) {
        return false;
    }
    if dom.get_tag_name(a) != dom.get_tag_name(b) {
        return false;
    }
    // Character-data equality is dispatched by kind — Text compares
    // TextContent, Comment compares CommentData, everything else has
    // neither component and skips the lookup entirely.
    match kind {
        Some(NodeKind::Text) => {
            let ta = dom.world().get::<&elidex_ecs::TextContent>(a).ok();
            let tb = dom.world().get::<&elidex_ecs::TextContent>(b).ok();
            if ta.as_deref().map(|t| &t.0) != tb.as_deref().map(|t| &t.0) {
                return false;
            }
        }
        Some(NodeKind::Comment) => {
            let ca = dom.world().get::<&elidex_ecs::CommentData>(a).ok();
            let cb = dom.world().get::<&elidex_ecs::CommentData>(b).ok();
            if ca.as_deref().map(|c| &c.0) != cb.as_deref().map(|c| &c.0) {
                return false;
            }
        }
        Some(NodeKind::DocumentType) => {
            let da = dom.world().get::<&elidex_ecs::DocTypeData>(a).ok();
            let db = dom.world().get::<&elidex_ecs::DocTypeData>(b).ok();
            match (da.as_deref(), db.as_deref()) {
                (Some(x), Some(y)) => {
                    if x.name != y.name || x.public_id != y.public_id || x.system_id != y.system_id
                    {
                        return false;
                    }
                }
                (Some(_), None) | (None, Some(_)) => return false,
                (None, None) => {}
            }
        }
        _ => {}
    }
    if !attributes_equal(dom, a, b) {
        return false;
    }
    let kids_a: Vec<Entity> = dom.children_iter(a).collect();
    let kids_b: Vec<Entity> = dom.children_iter(b).collect();
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
fn attributes_equal(dom: &elidex_ecs::EcsDom, a: Entity, b: Entity) -> bool {
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
        // treat an absent component as an empty attribute set.
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
/// - Cloned Document entities receive the full document-specific
///   own-property suite via
///   [`super::super::VmInner::install_document_methods_for_entity`]
///   so `document.cloneNode(true).createElement(...)` works.
pub(super) fn native_node_clone_node(
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
        let Some(new_root) = dom.clone_subtree(src) else {
            return Ok(JsValue::Null);
        };
        if !deep {
            // Shallow: drop the cloned children so only the root
            // survives.  `clone_subtree` is the only ECS entry
            // point for cloning today; a dedicated
            // `EcsDom::clone_node_shallow` is a straight profile-
            // driven swap if it ever shows up in profiles.
            let kids: Vec<Entity> = dom.children_iter(new_root).collect();
            for child in kids {
                despawn_subtree(dom, child);
            }
        }
        new_root
    };
    // Stash the cloned NodeKind before `create_element_wrapper` so
    // that we can patch the document-specific method suite onto the
    // clone's wrapper.
    let is_document = matches!(
        ctx.host().dom().node_kind(new_entity),
        Some(NodeKind::Document)
    );
    let wrapper = ctx.vm.create_element_wrapper(new_entity);
    if is_document {
        ctx.vm
            .install_document_methods_for_entity(new_entity, wrapper);
    }
    Ok(JsValue::Object(wrapper))
}

/// Recursively despawn `entity` and everything underneath it, used
/// to pare a clone subtree back to its root for the shallow path.
/// [`elidex_ecs::EcsDom::destroy_entity`] only removes one node, so
/// children would otherwise leak.
fn despawn_subtree(dom: &mut elidex_ecs::EcsDom, entity: Entity) {
    let kids: Vec<Entity> = dom.children_iter(entity).collect();
    for c in kids {
        despawn_subtree(dom, c);
    }
    let _ = dom.destroy_entity(entity);
}
