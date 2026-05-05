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
//!   `isEqualNode`, `cloneNode`, `normalize`.
//!
//! All members are thin bindings around the corresponding
//! `elidex-dom-api` / `elidex-ecs` handlers via
//! [`super::dom_bridge::invoke_dom_api`] — the algorithm proper lives
//! engine-independently per the CLAUDE.md Layering mandate.
//! `cloneNode` carries one extra step VM-side: a Document clone gets
//! the document-specific own-property suite installed onto its
//! wrapper post-dispatch ([`install_document_methods_if_cloned_doc`]).

#![cfg(feature = "engine")]

use super::super::object_kind::ObjectKind;
use super::super::value::{JsValue, NativeContext, PropertyKey, VmError};
use super::event_target::entity_from_this;
use super::node_proto::require_node_arg;

// ---------------------------------------------------------------------------
// ownerDocument / isSameNode / getRootNode (WHATWG DOM §4.4)
// ---------------------------------------------------------------------------

/// `Node.prototype.ownerDocument` — WHATWG §4.4.
///
/// Dispatches to the `ownerDocument.get` handler in `elidex-dom-api`,
/// which honours the per-entity `AssociatedDocument` component (so
/// `clonedDoc.createElement(...)` reports the clone, not the bound
/// global) and falls back to `EcsDom::document_root` for orphans whose
/// tree-root walk does not land on a Document.
pub(super) fn native_node_get_owner_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    super::dom_bridge::invoke_dom_api(ctx, "ownerDocument.get", entity, &[])
}

/// `Node.prototype.isSameNode(other)` — WHATWG §4.4.  Legacy alias of
/// `===`: returns true iff `this` and `other` are the same wrapper.
///
/// WebIDL signature is `boolean isSameNode(Node? otherNode)`:
/// `null` / `undefined` ⇒ `false`, non-Node object ⇒ `TypeError`
/// (the same brand-check seam every Node-arg method uses).
pub(super) fn native_node_is_same_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(self_entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let other = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(other, JsValue::Null | JsValue::Undefined) {
        return Ok(JsValue::Boolean(false));
    }
    // Brand check before dispatch so `node.isSameNode({})` raises
    // TypeError rather than returning a stable handler result.
    require_node_arg(ctx, other, "isSameNode")?;
    super::dom_bridge::invoke_dom_api(ctx, "isSameNode", self_entity, &[other])
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
    // Extract `options.composed` at the boundary so the handler sees a
    // primitive `JsValue::Bool`.
    let composed = match args.first().copied() {
        Some(JsValue::Object(opts_id)) => {
            let v = ctx
                .vm
                .get_property_value(opts_id, PropertyKey::String(ctx.vm.well_known.composed))?;
            super::super::coerce::to_boolean(ctx.vm, v)
        }
        _ => false,
    };
    super::dom_bridge::invoke_dom_api(ctx, "getRootNode", entity, &[JsValue::Boolean(composed)])
}

// ---------------------------------------------------------------------------
// compareDocumentPosition (WHATWG DOM §4.4)
// ---------------------------------------------------------------------------

/// `Node.prototype.compareDocumentPosition(other)` — returns the
/// WHATWG §4.4 bitmask for `other`'s position relative to `this`.
///
/// Thin binding over the `compareDocumentPosition` handler in
/// `elidex-dom-api` (which delegates to
/// [`elidex_ecs::EcsDom::compare_document_position`]).  Unbound
/// receiver (no `HostObject` brand) silently returns the
/// "disconnected, preceding" bitmask matching elidex's softer
/// unbound-receiver policy; browsers throw TypeError here.  Non-Node
/// `other` argument is rejected at the bridge boundary
/// (`prepare_arg → require_node_wrapper_kind`) with a generic
/// TypeError, no method-specific brand check needed at the VM seam.
pub(super) fn native_node_compare_document_position(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    use elidex_ecs::{
        DOCUMENT_POSITION_DISCONNECTED, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC,
        DOCUMENT_POSITION_PRECEDING,
    };
    let Some(self_entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(f64::from(
            DOCUMENT_POSITION_DISCONNECTED
                | DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC
                | DOCUMENT_POSITION_PRECEDING,
        )));
    };
    let other_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    super::dom_bridge::invoke_dom_api(ctx, "compareDocumentPosition", self_entity, &[other_arg])
}

// ---------------------------------------------------------------------------
// isEqualNode (WHATWG DOM §4.4 "equals" algorithm)
// ---------------------------------------------------------------------------

/// `Node.prototype.isEqualNode(other)` — structural deep equality.
///
/// Thin binding over the `isEqualNode` handler in `elidex-dom-api`
/// (which delegates to [`elidex_ecs::EcsDom::nodes_equal`]).  WebIDL
/// `Node? other`: null / undefined return `false` without dispatch.
/// Unbound receiver returns `false`.  Non-Node `other` is rejected
/// at the bridge boundary with a generic TypeError.
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
    super::dom_bridge::invoke_dom_api(ctx, "isEqualNode", self_entity, &[other_arg])
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
    let result =
        super::dom_bridge::invoke_dom_api(ctx, "cloneNode", src, &[JsValue::Boolean(deep)])?;
    install_document_methods_if_cloned_doc(ctx, result);
    Ok(result)
}

/// Post-dispatch hook for `cloneNode`: when the cloned wrapper wraps
/// a Document entity, install the per-Document own-property suite
/// onto it.  The bridge's `create_element_wrapper` set up the
/// prototype chain; this adds the document-specific method bag
/// (`createElement`, `body`/`head` accessors, etc.) so
/// `document.cloneNode(true).createElement('p')` works.
///
/// File-local single-use helper — only `cloneNode` produces
/// Documents through the clone path.  Centralising this hook at the
/// bridge layer is tracked under the deferred slot
/// `#11-bridge-document-post-install`.
fn install_document_methods_if_cloned_doc(ctx: &mut NativeContext<'_>, result: JsValue) {
    let JsValue::Object(wrapper) = result else {
        return;
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(wrapper).kind else {
        return;
    };
    let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) else {
        return;
    };
    if matches!(
        ctx.host().dom().node_kind(entity),
        Some(elidex_ecs::NodeKind::Document)
    ) {
        ctx.vm.install_document_methods_for_entity(entity, wrapper);
    }
}

// ---------------------------------------------------------------------------
// Node.prototype.normalize — WHATWG DOM §4.4
// ---------------------------------------------------------------------------

/// `Node.prototype.normalize()` — WHATWG DOM §4.4.
///
/// Dispatches to the `normalize` handler in `elidex-dom-api`. The handler
/// recurses through `this`'s descendants, removing empty Text nodes and
/// merging adjacent Text siblings — see `Normalize::normalize_entity`
/// (`crates/dom/elidex-dom-api/src/node_methods/core.rs`). Unbound
/// receivers (no `HostObject` brand) silently no-op, matching every
/// other Node method here; Window wrappers never reach this native
/// because `Window.prototype` does not chain through `Node.prototype`.
///
/// Live [`Range`] offset adjustment (spec steps 6.1-6.4) is still
/// out of scope — `Range` is not yet implemented; `PR-Range` will
/// re-visit when it lands.
pub(super) fn native_node_normalize(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(root) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    super::dom_bridge::invoke_dom_api(ctx, "normalize", root, &[])
}
