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

/// `Node.prototype.ownerDocument` — WHATWG §4.4.
///
/// Delegates to [`EcsDom::owner_document`], which honours the
/// per-node [`AssociatedDocument`] component set at creation time and
/// falls back to the tree-root walk for legacy entities.  Result
/// mapping:
///
/// - Document receiver → `null` (per spec).
/// - [`AssociatedDocument`] present and points at a live Document →
///   that Document's wrapper (fixes `cloneDoc.createElement(...)` →
///   reports the clone, not the bound global).
/// - No component and tree-root is a Document → that Document
///   (preserves html5ever-produced fixtures and anything already
///   rooted in the main tree).
/// - Otherwise (true orphan whose root is not a Document) → fall
///   back to the bound global document so that pre-PR4f callers
///   relying on the implicit single-document fallback keep working.
///
/// [`AssociatedDocument`]: elidex_ecs::AssociatedDocument
/// [`EcsDom::owner_document`]: elidex_ecs::EcsDom::owner_document
pub(super) fn native_node_get_owner_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let dom = ctx.host().dom();
    if matches!(dom.node_kind(entity), Some(NodeKind::Document)) {
        return Ok(JsValue::Null);
    }
    if let Some(doc) = dom.owner_document(entity) {
        return Ok(JsValue::Object(ctx.vm.create_element_wrapper(doc)));
    }
    // Orphan / fragment root — fall back to the bound document so
    // that nodes created outside the VM (parser fixtures, bare
    // `EcsDom::create_*` calls in tests) still report a sensible
    // ownerDocument.  VM-created nodes never reach this branch once
    // `createElement` sets `AssociatedDocument` at birth.
    let doc = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.document_entity_opt());
    Ok(wrap_entity_or_null(ctx.vm, doc))
}

/// `Node.prototype.isSameNode(other)` — WHATWG §4.4.  Legacy alias of
/// `===`: returns true iff `this` and `other` are the same wrapper.
///
/// WebIDL signature is `boolean isSameNode(Node? otherNode)`:
/// `null` / `undefined` ⇒ `false`, non-Node object ⇒ `TypeError`
/// (matches `contains` / `isEqualNode` / `compareDocumentPosition`
/// which all delegate to [`require_node_arg`]).
pub(super) fn native_node_is_same_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let other = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(other, JsValue::Null | JsValue::Undefined) {
        return Ok(JsValue::Boolean(false));
    }
    // Brand check — throw TypeError for non-Node arguments before
    // the identity comparison, so `node.isSameNode({})` matches
    // browser behaviour instead of silently returning `false`.
    let _other_entity = require_node_arg(ctx, other, "isSameNode")?;
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
/// - `0x20 DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC` — set in the
///   disconnected-trees branch per WHATWG §4.4 ("the result must
///   also include IMPLEMENTATION_SPECIFIC"), zero elsewhere.
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
    const IMPLEMENTATION_SPECIFIC: u32 = 0x20;

    let Some(self_entity) = entity_from_this(ctx, this) else {
        // Unbound receiver: fall through to DISCONNECTED.  Browsers
        // throw TypeError here, but elidex's unbound-receiver policy
        // is the softer silent path.
        return Ok(JsValue::Number(f64::from(
            DISCONNECTED | IMPLEMENTATION_SPECIFIC | PRECEDING,
        )));
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
        // WHATWG §4.4: when DISCONNECTED is set the result must also
        // include IMPLEMENTATION_SPECIFIC and one of PRECEDING /
        // FOLLOWING, with a *consistent* relative ordering.  The
        // ordering must be antisymmetric: swapping the operands must
        // flip PRECEDING ↔ FOLLOWING.  Compare by entity bits (a
        // stable, total order independent of tree structure) so
        // `a.compareDocumentPosition(b) ^ b.compareDocumentPosition(a)`
        // is always `(PRECEDING | FOLLOWING)` for disconnected nodes.
        let order = if self_entity.to_bits().get() < other_entity.to_bits().get() {
            FOLLOWING
        } else {
            PRECEDING
        };
        return Ok(JsValue::Number(f64::from(
            DISCONNECTED | IMPLEMENTATION_SPECIFIC | order,
        )));
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
    // Non-Node arguments go through WebIDL's `Node?` conversion and
    // throw TypeError, matching `compareDocumentPosition` and every
    // shipping browser (null / undefined are handled above).
    let other_entity = require_node_arg(ctx, other_arg, "isEqualNode")?;
    let equal = {
        let dom = ctx.host().dom();
        nodes_equal(dom, self_entity, other_entity)
    };
    Ok(JsValue::Boolean(equal))
}

/// Structural deep-equality for two Node entities.  Walks children
/// via `children_iter` (shadow-root entities are skipped in both
/// subtrees, matching WHATWG light-tree semantics).
///
/// Iterative DFS over `(a, b)` pairs — deep DOM trees must not
/// overflow the Rust call stack (matches the explicit-stack pattern
/// used by `despawn_subtree` and `clone_children_recursive`).
fn nodes_equal(dom: &elidex_ecs::EcsDom, a: Entity, b: Entity) -> bool {
    let mut stack: Vec<(Entity, Entity)> = vec![(a, b)];
    while let Some((a, b)) = stack.pop() {
        // `node_kind_inferred` falls back to payload-based inference
        // for legacy entities missing the `NodeKind` component.
        // Comparing raw `node_kind` would treat two legacy entities
        // of different payload kinds (e.g. a legacy Text and a
        // legacy Comment, both reporting `kind == None`) as equal
        // because the character-data match arms below would both
        // be skipped.
        let kind = dom.node_kind_inferred(a);
        if kind != dom.node_kind_inferred(b) {
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
                        if x.name != y.name
                            || x.public_id != y.public_id
                            || x.system_id != y.system_id
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
        // Push in reverse so pre-order pops match recursive walk order
        // (not functionally required for equality, but keeps early-exit
        // behaviour predictable and easier to reason about in logs).
        for (ca, cb) in kids_a.iter().zip(kids_b.iter()).rev() {
            stack.push((*ca, *cb));
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
/// - Shallow clone (`deep == false`) dispatches to
///   [`elidex_ecs::EcsDom::clone_node_shallow`] — copies attributes
///   (Element) or character data (Text / Comment) only, in
///   O(attrs + character-data) work.  The descendant walk never
///   runs.
/// - Deep clone (`deep == true`) dispatches to
///   [`elidex_ecs::EcsDom::clone_subtree`], which additionally
///   recurses through all light-tree descendants.
/// - The returned wrapper's entity has no parent or siblings
///   (WHATWG §4.5 "cloning steps" — the clone is an orphan).
/// - Event listeners and shadow roots are **not** cloned.  WHATWG
///   §4.5 explicitly excludes both; both ECS helpers enforce it.
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
        let cloned = if deep {
            dom.clone_subtree(src)
        } else {
            // `cloneNode(false)` — skip the subtree walk entirely
            // so shallow clone stays O(attrs + character-data)
            // rather than O(|descendants|).
            dom.clone_node_shallow(src)
        };
        match cloned {
            Some(new_root) => new_root,
            None => return Ok(JsValue::Null),
        }
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

// ---------------------------------------------------------------------------
// Node.prototype.normalize — WHATWG DOM §4.4
// ---------------------------------------------------------------------------

/// `Node.prototype.normalize()` — WHATWG DOM §4.4.
///
/// For each exclusive Text descendant of `this`:
/// - remove it if its data is empty, otherwise
/// - absorb every following contiguous Text sibling's data and
///   remove those siblings.
///
/// elidex details:
/// - Iteration is snapshot-based (pre-order DFS via
///   [`EcsDom::traverse_descendants`] which skips `ShadowRoot`) so
///   the walk is safe against the in-loop mutations `remove_child`
///   performs.
/// - Text entities that a prior merge has detached from their parent
///   are silently skipped — the outer pass can otherwise revisit a
///   swallowed sibling.
/// - Unbound / non-HostObject receivers are silent no-ops, matching
///   every other Node method here.  Window wrappers never reach this
///   native because `Window.prototype` does not chain through
///   `Node.prototype`.
/// - Live [`Range`] offset adjustment (spec steps 6.1-6.4) is skipped:
///   elidex does not yet implement [`Range`], so there are no ranges
///   to fix up.  `PR-Range` will re-visit this when `Range` lands.
pub(super) fn native_node_normalize(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(root) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };

    // --- Pass 1: snapshot every descendant Text entity in pre-order.
    let dom = ctx.host().dom();
    if !dom.contains(root) {
        return Ok(JsValue::Undefined);
    }
    let mut text_nodes: Vec<Entity> = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if matches!(dom.node_kind_inferred(entity), Some(NodeKind::Text)) {
            text_nodes.push(entity);
        }
        true
    });

    // --- Pass 2: empty-data removal + contiguous-sibling merge.
    for text in text_nodes {
        let dom = ctx.host().dom();
        if !dom.contains(text) {
            // Already destroyed (defence — `remove_child` only
            // detaches in this codebase, but stay resilient).
            continue;
        }
        // A prior iteration may have merged this entry into an
        // earlier Text and then detached it; detached nodes have no
        // parent and are no longer descendants of `root`.
        let Some(parent) = dom.get_parent(text) else {
            continue;
        };

        let original = match dom.world().get::<&elidex_ecs::TextContent>(text) {
            Ok(t) => t.0.clone(),
            Err(_) => continue,
        };
        if original.is_empty() {
            let _ = dom.remove_child(parent, text);
            continue;
        }

        // Walk forward through contiguous Text siblings, collecting
        // their data.  `get_next_sibling` is re-read each iteration
        // because removing a sibling rewires the sibling chain.
        let mut merged = original.clone();
        loop {
            let Some(next) = dom.get_next_sibling(text) else {
                break;
            };
            if !matches!(dom.node_kind_inferred(next), Some(NodeKind::Text)) {
                break;
            }
            let next_data = match dom.world().get::<&elidex_ecs::TextContent>(next) {
                Ok(t) => t.0.clone(),
                Err(_) => break,
            };
            merged.push_str(&next_data);
            // `next` is a child of the same parent — by construction
            // `get_next_sibling(text)` returns a sibling under
            // `parent`.  `remove_child` detaches without destroying.
            let _ = dom.remove_child(parent, next);
        }

        if merged != original {
            if let Ok(mut t) = dom.world_mut().get::<&mut elidex_ecs::TextContent>(text) {
                t.0 = merged;
            }
            dom.rev_version(text);
        }
    }

    Ok(JsValue::Undefined)
}
