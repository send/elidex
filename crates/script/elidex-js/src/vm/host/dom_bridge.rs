//! Helpers shared across host-side DOM natives — wrapper lifting
//! and selector parsing.
//!
//! These existed as file-local `fn`s in `document.rs` and
//! `element_proto.rs` before they grew a second consumer.  Keeping
//! them in one place avoids the near-identical copies drifting over
//! time (each had seven call sites between the two files).

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::super::VmInner;
use super::event_target::entity_from_this;

use elidex_css::{parse_selector_from_str, Selector};
use elidex_ecs::{EcsDom, Entity, NodeKind};
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, JsObjectRef, SessionCore,
};

/// Return `Option<Entity>` as a JS wrapper or `null` — no intermediate
/// `ObjectId`, so callers can chain it straight into a `Result::Ok`.
pub(super) fn wrap_entity_or_null(vm: &mut VmInner, entity: Option<Entity>) -> JsValue {
    match entity {
        Some(e) => JsValue::Object(vm.create_element_wrapper(e)),
        None => JsValue::Null,
    }
}

/// Wrap a list of entities as a JS Array of element wrappers.  One
/// allocation for the intermediate `Vec<JsValue>`, one for the
/// Array object.
pub(super) fn wrap_entities_as_array(vm: &mut VmInner, entities: &[Entity]) -> JsValue {
    let elements: Vec<JsValue> = entities
        .iter()
        .map(|&e| JsValue::Object(vm.create_element_wrapper(e)))
        .collect();
    JsValue::Object(vm.create_array_object(elements))
}

/// Parse a selector string and reject shadow-scoped pseudos.  Shared
/// by `document.querySelector*` and `Element.prototype.matches` /
/// `closest` — all four throw a `DOMException` named `"SyntaxError"`
/// on invalid input and on `:host` / `::slotted()`, which are only
/// valid inside shadow-tree context (WHATWG DOM §4.7 / WHATWG
/// Selectors API §1.2 — the spec-mandated exception class is
/// `DOMException`, *not* the ECMA `SyntaxError` constructor).
///
/// `syntax_error_name` is the pre-interned `StringId` for
/// `"SyntaxError"` (`WellKnownStrings::dom_exc_syntax_error`); the
/// caller threads it in to keep this helper independent of
/// `&VmInner` and avoid an extra borrow in the hot path.  The
/// `method` name appears in the shadow-pseudo error message so
/// callers get a call-site-accurate complaint (`… are not valid in
/// querySelector` vs `… in matches/closest`).
pub(super) fn parse_dom_selector(
    selector_str: &str,
    shadow_method_label: &str,
    syntax_error_name: super::super::value::StringId,
) -> Result<Vec<Selector>, VmError> {
    let selectors = parse_selector_from_str(selector_str).map_err(|()| {
        VmError::dom_exception(
            syntax_error_name,
            format!("Invalid selector: {selector_str}"),
        )
    })?;
    if selectors.iter().any(|s| s.has_shadow_pseudo()) {
        return Err(VmError::dom_exception(
            syntax_error_name,
            format!(":host and ::slotted() are not valid in {shadow_method_label}"),
        ));
    }
    Ok(selectors)
}

/// Coerce the first argument to a string and hand back its UTF-8
/// materialisation — the shape every selector-accepting native
/// (querySelector, matches, closest, …) starts with.
pub(super) fn coerce_first_arg_to_string(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<String, VmError> {
    let sid = coerce_first_arg_to_string_id(ctx, args)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

/// Coerce the first argument to a string and return the interned
/// `StringId` directly — skips the `get_utf8 → intern` round trip
/// for callers that need the id (e.g. building `JsValue::String` for
/// handler dispatch).
pub(super) fn coerce_first_arg_to_string_id(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<super::super::value::StringId, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    super::super::coerce::to_string(ctx.vm, arg)
}

/// Shared body for every "map `this` through one `EcsDom` tree-nav
/// accessor and wrap-or-null" native — extracts the receiver entity,
/// runs `lookup` against the bound DOM, and lifts the result to a
/// wrapper (or `null`).  The unbound-receiver path returns `null`.
///
/// Used by both `Element.prototype` (ParentNode / sibling accessors)
/// and `Node.prototype` (parentNode / firstChild / nextSibling / …).
pub(super) fn tree_nav_getter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    lookup: impl FnOnce(&EcsDom, Entity) -> Option<Entity>,
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let target = lookup(ctx.host().dom(), entity);
    Ok(wrap_entity_or_null(ctx.vm, target))
}

/// Pre-order DFS over descendants of `root` looking for the first
/// element that matches any selector in `selectors`.  `root` itself is
/// **not** a match candidate — WHATWG §4.2.6 step 3.  Returns the
/// matched entity, or `None` if none found.
///
/// Shared by both `document.querySelector` and
/// `Element.prototype.querySelector`.
pub(super) fn query_selector_in_subtree_first(
    dom: &EcsDom,
    root: Entity,
    selectors: &[elidex_css::Selector],
) -> Option<Entity> {
    use elidex_ecs::TagType;
    let mut result = None;
    dom.traverse_descendants(root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|s| s.matches(entity, dom))
        {
            result = Some(entity);
            false
        } else {
            true
        }
    });
    result
}

/// Recursively flatten `node` into the list of real nodes to insert.
/// `DocumentFragment` at any depth expands to its light-tree
/// descendants; every other `NodeKind` becomes a single leaf entry.
///
/// **Side-effect free**: the walk reads children without mutating
/// the source tree.  Fragment emptying happens separately in
/// [`finalize_pair`] AFTER the insertion loop succeeds — draining
/// during the walk would orphan leaves whenever a pre-insertion
/// validity check (`replaceChildren` / `replaceWith`) later throws,
/// because the detach would already have happened.
pub(super) fn nodes_to_insert(ctx: &mut NativeContext<'_>, node: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    flatten_into(ctx, node, &mut out);
    out
}

fn flatten_into(ctx: &mut NativeContext<'_>, node: Entity, out: &mut Vec<Entity>) {
    if !matches!(
        ctx.host().dom().node_kind(node),
        Some(NodeKind::DocumentFragment)
    ) {
        out.push(node);
        return;
    }
    let children: Vec<Entity> = ctx.host().dom().children_iter(node).collect();
    for child in children {
        flatten_into(ctx, child, out);
    }
}

/// Recursively detach every `DocumentFragment` descendant of `root`
/// from its fragment parent.  Called on the **success path** of an
/// insertion to finalise WHATWG §4.2.3's "fragment becomes empty
/// after pre-insert" contract — leaves already moved during the
/// insert loop, this pass empties the intermediate fragment
/// scaffolding that the leaves were originally parented to.
///
/// Must NOT be called on an error path: some leaves may still be
/// linked to the fragment tree, and detaching their fragment
/// parents would leave them stranded in orphan fragments.
pub(super) fn drain_fragment_descendants(ctx: &mut NativeContext<'_>, root: Entity) {
    if !matches!(
        ctx.host().dom().node_kind(root),
        Some(NodeKind::DocumentFragment)
    ) {
        return;
    }
    let children: Vec<Entity> = ctx.host().dom().children_iter(root).collect();
    for child in children {
        if matches!(
            ctx.host().dom().node_kind(child),
            Some(NodeKind::DocumentFragment)
        ) {
            drain_fragment_descendants(ctx, child);
            let _ = ctx.host().dom().remove_child(root, child);
        }
        // Non-fragment (leaf) children shouldn't linger on the
        // success path — leaves already moved during the insert
        // loop.  If one does stay (e.g. the caller skipped a
        // leaf), leaving it attached is safer than an aggressive
        // detach.
    }
}

/// Pre-order DFS collecting every descendant of `root` matching any
/// selector in `selectors`.  `root` itself is not a match candidate.
pub(super) fn query_selector_in_subtree_all(
    dom: &EcsDom,
    root: Entity,
    selectors: &[elidex_css::Selector],
) -> Vec<Entity> {
    use elidex_ecs::TagType;
    let mut out = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|s| s.matches(entity, dom))
        {
            out.push(entity);
        }
        true
    });
    out
}

// `collect_descendants_by_tag_name` / `collect_descendants_by_class_name`
// lived here until PR5b §C3 migrated every caller (`document.getElementsBy*`
// / `element.getElementsBy*`) onto the shared live-collection
// infrastructure in `dom_collection.rs`.  The traversal now runs
// inside `LiveCollectionKind::{ByTag, ByClass}` resolution on each
// read, so this file no longer needs a static snapshot helper.

// ---------------------------------------------------------------------------
// `DomApiHandler` dispatch — bridge from VM-internal `JsValue` →
// engine-independent `elidex_plugin::JsValue` and back, invoking
// `DomApiHandler::invoke()` through the registry stored on
// `VmInner.dom_registry`.  Keeps DOM mutation algorithms / selector
// matching / form validation / live-collection walking on the
// engine-independent side per the CLAUDE.md Layering mandate.
// ---------------------------------------------------------------------------

/// VM-side pre-validated argument representation — primitives are
/// already converted to `elidex_plugin::JsValue`, but `Object` args
/// must defer to `materialize` because session-side
/// `IdentityMap::get_or_create` requires `&mut SessionCore` (only
/// available inside `with_session_and_dom`).
enum PreVal {
    Primitive(elidex_plugin::JsValue),
    Entity(Entity),
}

impl PreVal {
    /// Convert to a final `elidex_plugin::JsValue`.  Consumes `self`
    /// so primitive `Pv::String` payloads can move directly into the
    /// args vec without re-allocating.  For `Entity`, classifies the
    /// kind via [`arg_component_kind`] (which rejects entities that
    /// have no inferable kind AND entities classified as Window —
    /// Window is an `EventTarget`, not a Node, matching
    /// `node_proto::require_node_arg`'s policy).
    ///
    /// In practice every current call site routes Node-typed
    /// arguments through `require_node_arg` (`node_proto.rs`), which
    /// already runs `node_kind_inferred` and rejects both bare
    /// entities and Window before dispatch.  The rejection here is
    /// therefore **defense-in-depth**: a future native that hands
    /// the bridge a HostObject without a brand check still fails
    /// closed instead of fabricating an Element / Window wrapper
    /// through the session's identity map.
    fn materialize(
        self,
        session: &mut SessionCore,
        dom: &EcsDom,
    ) -> Result<elidex_plugin::JsValue, DomApiError> {
        match self {
            PreVal::Primitive(v) => Ok(v),
            PreVal::Entity(entity) => {
                let kind = arg_component_kind(dom.node_kind_inferred(entity))?;
                let obj_ref = session.get_or_create_wrapper(entity, kind);
                Ok(elidex_plugin::JsValue::ObjectRef(obj_ref.to_raw()))
            }
        }
    }
}

/// Classify an entity's inferred [`NodeKind`] into the
/// [`ComponentKind`] that the bridge passes to handlers.
///
/// Two failure modes both surface as a `TypeError`:
/// - `None` — the entity has no `NodeKind` component and no
///   payload (`TagType` / `TextContent` / `CommentData` /
///   `DocTypeData`) for [`EcsDom::node_kind_inferred`] to recover
///   from.  Reaching this arm means a HostObject wrapper points at
///   a bare entity, which should never escape a brand-checked call
///   path.
/// - `Some(NodeKind::Window)` — Window is an `EventTarget` but
///   *not* a Node (WHATWG HTML §7.2 / DOM §4.4), and
///   `node_proto::require_node_arg` rejects it explicitly.  Match
///   that policy here so the marshalling layer never produces a
///   Window-shaped wrapper for a "Node-expecting" handler.
///
/// Pure function over `Option<NodeKind>` — unit-testable without
/// constructing an `EcsDom`.
fn arg_component_kind(node_kind: Option<NodeKind>) -> Result<ComponentKind, DomApiError> {
    let nk = node_kind.ok_or_else(|| DomApiError {
        kind: DomApiErrorKind::TypeError,
        message: "DOM API: argument is not a valid Node".into(),
    })?;
    if matches!(nk, NodeKind::Window) {
        return Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: "DOM API: Window is not a Node".into(),
        });
    }
    Ok(ComponentKind::from_node_kind(nk))
}

/// Handler return-value classification — primitives can be
/// converted in the VM-only phase, but `Entity` returns must be
/// resolved through the session's identity map (read-only access)
/// before the dual-borrow scope ends.  The `ComponentKind` carried
/// alongside the `Entity` lets the VM-side wrapper-allocation phase
/// reject non-Node return kinds (Attribute / Window / sub-objects)
/// that `create_element_wrapper` cannot dispatch correctly.
enum HandlerOut {
    Primitive(elidex_plugin::JsValue),
    Entity(Entity, ComponentKind),
}

/// Validate that a handler's `ObjectRef` return resolves to a
/// `ComponentKind` the bridge knows how to wrap.  Today
/// [`VmInner::create_element_wrapper`] handles all real DOM Node
/// kinds (Element, character-data variants, Document family,
/// DocumentFragment) but routes Attribute / Window / sub-object
/// kinds (Style / ClassList / ChildNodes / Dataset) into a generic
/// `Node.prototype` chain that does NOT match their actual
/// IDL-defined prototype.  Failing fast here means an unsupported
/// return surfaces a clear `VmError::internal` instead of a wrong
/// JS wrapper that misbehaves on first method call.  Subsequent
/// arch-hoist slots widen this set as dedicated wrapper paths
/// (e.g. Attr / NodeList) come online.
fn require_node_wrapper_kind(kind: ComponentKind) -> Result<(), VmError> {
    use ComponentKind as Ck;
    match kind {
        Ck::Element
        | Ck::TextNode
        | Ck::Comment
        | Ck::CdataSection
        | Ck::ProcessingInstruction
        | Ck::Document
        | Ck::DocumentType
        | Ck::DocumentFragment => Ok(()),
        _ => Err(VmError::internal(format!(
            "DomApiHandler returned ObjectRef of kind {kind:?}; bridge currently \
             wraps only Node-derived kinds via create_element_wrapper. Attribute / \
             Window / sub-object returns require dedicated wrapper paths to be \
             plumbed in subsequent arch-hoist slots."
        ))),
    }
}

/// Pre-validate a single VM `JsValue` argument: primitives convert
/// directly, `Object` extracts its bound entity bits (deferring
/// session-side ID allocation to the materialize phase).
///
/// `Symbol` raises `TypeError` per WebIDL §3.10.14 / ECMA §7.1.17
/// (Symbol coercion is total-throw across all non-Symbol types).
/// `BigInt` rejection here is a **defensive bridge-level rule**, not
/// a WebIDL mandate — ECMA §7.1.17 lets `BigInt → String` coerce
/// successfully (`1n` ⇒ `"1"`), and call-site coercion (e.g.
/// [`coerce_first_arg_to_string`]) already converts BigInt before it
/// reaches the bridge.  This arm only fires when a future call site
/// hands `prepare_arg` a raw `BigInt`, in which case rejecting is
/// safer than guessing whether a string or number coercion was
/// intended.
fn prepare_arg(ctx: &mut NativeContext<'_>, v: JsValue) -> Result<PreVal, VmError> {
    use elidex_plugin::JsValue as Pv;
    Ok(match v {
        JsValue::Empty | JsValue::Undefined => PreVal::Primitive(Pv::Undefined),
        JsValue::Null => PreVal::Primitive(Pv::Null),
        JsValue::Boolean(b) => PreVal::Primitive(Pv::Bool(b)),
        JsValue::Number(n) => PreVal::Primitive(Pv::Number(n)),
        JsValue::String(sid) => PreVal::Primitive(Pv::String(ctx.get_utf8(sid))),
        JsValue::Object(obj_id) => match &ctx.vm.get_object(obj_id).kind {
            ObjectKind::HostObject { entity_bits } => {
                let entity = Entity::from_bits(*entity_bits).ok_or_else(|| {
                    VmError::type_error("DOM API: argument wraps an invalid entity")
                })?;
                PreVal::Entity(entity)
            }
            _ => {
                return Err(VmError::type_error("DOM API expected a Node argument"));
            }
        },
        JsValue::Symbol(_) => {
            return Err(VmError::type_error(
                "Cannot convert a Symbol value to a DOM API argument",
            ));
        }
        JsValue::BigInt(_) => {
            return Err(VmError::type_error(
                "Cannot convert a BigInt value to a DOM API argument",
            ));
        }
    })
}

/// Lift a primitive `elidex_plugin::JsValue` to a VM `JsValue`.
/// `ObjectRef` is **not** handled here — those go through the
/// `HandlerOut::Entity` path which extracts entity+kind from the
/// session's identity map inside the dual-borrow scope.
fn plugin_primitive_to_vm_value(
    ctx: &mut NativeContext<'_>,
    v: elidex_plugin::JsValue,
) -> Result<JsValue, VmError> {
    use elidex_plugin::JsValue as Pv;
    Ok(match v {
        Pv::Undefined => JsValue::Undefined,
        Pv::Null => JsValue::Null,
        Pv::Bool(b) => JsValue::Boolean(b),
        Pv::Number(n) => JsValue::Number(n),
        Pv::String(s) => JsValue::String(ctx.intern(&s)),
        Pv::ObjectRef(_) => {
            // Should never reach here — ObjectRef returns are
            // intercepted in `invoke_dom_api` and routed through
            // `HandlerOut::Entity` so the session lookup happens
            // while we still hold the dual borrow.
            return Err(VmError::internal(
                "plugin_primitive_to_vm_value received an ObjectRef",
            ));
        }
        // `elidex_plugin::JsValue` is `#[non_exhaustive]`; future
        // variants land as a hard error so the bridge never silently
        // mis-marshals a new value type.
        _ => {
            return Err(VmError::internal(
                "plugin_primitive_to_vm_value: unhandled JsValue variant",
            ));
        }
    })
}

/// Convert a `DomApiError` returned by a handler into a VM-flavoured
/// `VmError`.  `TypeError` becomes the ECMA-flavoured plain error
/// (WebIDL §3.10.14 — Symbol coercion failures and similar are
/// genuinely ECMA `TypeError`s); every other named variant resolves
/// to a `DOMException` whose `name` comes from the pre-interned
/// `WellKnownStrings` (alloc-free on the throw path).
/// **`SyntaxError` is `DOMException("SyntaxError")`, not the ECMA
/// `SyntaxError` constructor** — DOM-side selector / URL / CSS-OM
/// parse failures are spec-mandated as `DOMException` (WHATWG DOM
/// §4.7 legacy code 12); ECMA `SyntaxError` is reserved for
/// JavaScript parser errors (`eval`, `new Function`, etc.) which
/// don't reach this bridge.  `Other` and any future-added
/// `DomApiErrorKind` variants surface as `VmError::internal` —
/// that path means "bridge encountered an unmapped error kind" and
/// is intentionally distinct from a spec-named DOMException so a
/// missed mapping shows up as an internal error rather than
/// masquerading as a generic `DOMException("Error", …)`.
fn dom_api_error_to_vm_error(vm: &VmInner, err: DomApiError) -> VmError {
    let DomApiError { kind, message } = err;
    let wk = &vm.well_known;
    match kind {
        // ECMA exception (call-site coercion failures, etc.)
        DomApiErrorKind::TypeError => VmError::type_error(message),
        // DOMException variants — `SyntaxError` is a DOMException
        // name (WHATWG DOM §4.7), NOT the ECMA SyntaxError class.
        DomApiErrorKind::SyntaxError => VmError::dom_exception(wk.dom_exc_syntax_error, message),
        DomApiErrorKind::NotFoundError => {
            VmError::dom_exception(wk.dom_exc_not_found_error, message)
        }
        DomApiErrorKind::HierarchyRequestError => {
            VmError::dom_exception(wk.dom_exc_hierarchy_request_error, message)
        }
        DomApiErrorKind::InvalidStateError => {
            VmError::dom_exception(wk.dom_exc_invalid_state_error, message)
        }
        DomApiErrorKind::IndexSizeError => {
            VmError::dom_exception(wk.dom_exc_index_size_error, message)
        }
        DomApiErrorKind::InvalidCharacterError => {
            VmError::dom_exception(wk.dom_exc_invalid_character_error, message)
        }
        DomApiErrorKind::InUseAttributeError => {
            VmError::dom_exception(wk.dom_exc_in_use_attribute_error, message)
        }
        DomApiErrorKind::NotSupportedError => {
            VmError::dom_exception(wk.dom_exc_not_supported_error, message)
        }
        // Generic / unclassified.  `DomApiErrorKind` is
        // `#[non_exhaustive]`; new variants land here until they get
        // an explicit arm above, so a missed mapping surfaces as an
        // internal error rather than a generic DOMException.
        DomApiErrorKind::Other => VmError::internal(message),
        _ => VmError::internal(message),
    }
}

/// Dispatch a DOM API method by handler name.
///
/// Three phases:
///
/// 1. **VM pre-validate** — each input `JsValue` becomes a
///    [`PreVal`].  String args allocate (`String::from`) at the
///    boundary — a known cost we accept here; subsequent slots
///    consider a fast-path variant if benchmarks demand.  Symbol
///    args raise `TypeError` (WebIDL §3.10.14 / ECMA §7.1.17 —
///    Symbol ToString is total-throw); raw BigInt args also reject
///    here as a defensive rule (call sites that ToString-coerce
///    first feed `prepare_arg` a `JsValue::String` and never trip
///    this arm).
/// 2. **Dual-borrow** — inside [`HostData::with_session_and_dom`],
///    materialize each `PreVal` (allocating a [`JsObjectRef`] for
///    `Entity` args via `IdentityMap::get_or_create`), invoke the
///    handler, and (for object returns) resolve the returned
///    `ObjectRef` back to `(Entity, ComponentKind)` while the
///    session is still in scope.
/// 3. **Post** — outside the dual borrow, map [`DomApiError`] →
///    [`VmError`] using the pre-interned DOMException names and
///    materialize the final `JsValue` (primitive convert or wrapper
///    allocation through `VmInner::create_element_wrapper`).
///
/// Handler-not-registered is a hard runtime error
/// (`VmError::type_error("Unknown DOM method: ...")`): no
/// `EcsDom::*` direct-call fallback is provided, so a missing
/// handler surfaces immediately at the first call site rather
/// than silently regressing to the layer this bridge exists to
/// keep separated.
pub(super) fn invoke_dom_api(
    ctx: &mut NativeContext<'_>,
    handler_name: &'static str,
    this: Entity,
    args_in: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 1: VM-side pre-validation
    let pre: Vec<PreVal> = args_in
        .iter()
        .copied()
        .map(|v| prepare_arg(ctx, v))
        .collect::<Result<_, _>>()?;

    let registry = ctx.vm.dom_registry.clone();
    let handler = registry
        .resolve(handler_name)
        .ok_or_else(|| VmError::type_error(format!("Unknown DOM method: {handler_name}")))?;

    // Phase 2: Dual borrow — materialize args + invoke + resolve
    // ObjectRef return.
    let host_data = ctx
        .vm
        .host_data
        .as_deref_mut()
        .expect("invoke_dom_api requires HostData to be bound (Vm::bind not called)");
    let result: Result<HandlerOut, DomApiError> = host_data.with_session_and_dom(|session, dom| {
        let args_plugin: Vec<elidex_plugin::JsValue> = pre
            .into_iter()
            .map(|p| p.materialize(session, dom))
            .collect::<Result<_, _>>()?;
        let raw = handler.invoke(this, &args_plugin, session, dom)?;
        Ok(match raw {
            elidex_plugin::JsValue::ObjectRef(r) => {
                let (entity, kind) = session
                    .identity_map()
                    .get(JsObjectRef::from_raw(r))
                    .ok_or_else(|| DomApiError {
                        kind: DomApiErrorKind::Other,
                        message: "DomApiHandler returned an unmapped ObjectRef".into(),
                    })?;
                HandlerOut::Entity(entity, kind)
            }
            other => HandlerOut::Primitive(other),
        })
    });

    // Phase 3: Error map + final wrapper allocation
    match result {
        Ok(HandlerOut::Primitive(v)) => plugin_primitive_to_vm_value(ctx, v),
        Ok(HandlerOut::Entity(e, kind)) => {
            require_node_wrapper_kind(kind)?;
            Ok(JsValue::Object(ctx.vm.create_element_wrapper(e)))
        }
        Err(e) => Err(dom_api_error_to_vm_error(ctx.vm, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::value::VmErrorKind;
    use super::*;

    #[test]
    fn require_node_wrapper_kind_accepts_node_kinds() {
        for k in [
            ComponentKind::Element,
            ComponentKind::TextNode,
            ComponentKind::Comment,
            ComponentKind::CdataSection,
            ComponentKind::ProcessingInstruction,
            ComponentKind::Document,
            ComponentKind::DocumentType,
            ComponentKind::DocumentFragment,
        ] {
            assert!(
                require_node_wrapper_kind(k).is_ok(),
                "{k:?} must be accepted"
            );
        }
    }

    #[test]
    fn require_node_wrapper_kind_rejects_non_node_kinds() {
        // Attribute / Window / sub-object kinds — wrapping them via
        // `create_element_wrapper` would produce a Node.prototype-
        // chained wrapper that does not match their IDL prototype,
        // so the bridge must fail closed.
        for k in [
            ComponentKind::Attribute,
            ComponentKind::Window,
            ComponentKind::Style,
            ComponentKind::ClassList,
            ComponentKind::ChildNodes,
            ComponentKind::Dataset,
        ] {
            let err = require_node_wrapper_kind(k).expect_err(&format!("{k:?} must be rejected"));
            assert!(
                matches!(err.kind, VmErrorKind::InternalError),
                "{k:?} must produce VmError::internal, got {:?}",
                err.kind,
            );
        }
    }

    #[test]
    fn arg_component_kind_accepts_node_kinds() {
        for nk in [
            NodeKind::Element,
            NodeKind::Attribute,
            NodeKind::Text,
            NodeKind::CdataSection,
            NodeKind::ProcessingInstruction,
            NodeKind::Comment,
            NodeKind::Document,
            NodeKind::DocumentType,
            NodeKind::DocumentFragment,
        ] {
            let ck = arg_component_kind(Some(nk))
                .unwrap_or_else(|e| panic!("{nk:?} must classify, got {:?}", e.kind));
            assert_eq!(ck, ComponentKind::from_node_kind(nk));
        }
    }

    #[test]
    fn arg_component_kind_rejects_none() {
        // Bare entity (no NodeKind / TagType / TextContent / …) —
        // never reachable from a brand-checked call site, so this
        // arm is the defense-in-depth fallback.
        let err = arg_component_kind(None).expect_err("None must reject");
        assert!(matches!(err.kind, DomApiErrorKind::TypeError));
        assert!(err.message.contains("not a valid Node"));
    }

    #[test]
    fn arg_component_kind_rejects_window() {
        // Window is an EventTarget but explicitly NOT a Node; match
        // `node_proto::require_node_arg`'s policy at the marshalling
        // layer too so a future native that skips brand check still
        // fails closed.
        let err = arg_component_kind(Some(NodeKind::Window)).expect_err("Window must reject");
        assert!(matches!(err.kind, DomApiErrorKind::TypeError));
        assert!(err.message.contains("Window"));
    }

    #[test]
    fn dom_api_error_syntax_kind_maps_to_dom_exception() {
        // WHATWG DOM §4.7: `SyntaxError` is a DOMException name
        // (legacy code 12), NOT the ECMA SyntaxError class.  Selector
        // / URL / CSS-OM parse failures dispatched through the bridge
        // must surface as `DOMException("SyntaxError")` so JS code
        // that does `e instanceof DOMException` keeps working.
        let vm = crate::vm::Vm::new();
        let err = dom_api_error_to_vm_error(
            &vm.inner,
            DomApiError {
                kind: DomApiErrorKind::SyntaxError,
                message: "test syntax error".into(),
            },
        );
        let VmErrorKind::DomException { name } = err.kind else {
            panic!("expected DomException, got {:?}", err.kind);
        };
        assert_eq!(vm.inner.strings.get_utf8(name), "SyntaxError");
        assert_eq!(err.message, "test syntax error");
    }

    #[test]
    fn dom_api_error_type_kind_stays_ecma_type_error() {
        // Counterpart sanity check: TypeError must remain an ECMA
        // TypeError (WebIDL §3.10.14 — Symbol coercion failures and
        // other "wrong type" diagnostics are genuinely ECMA, not
        // DOMException-typed).
        let vm = crate::vm::Vm::new();
        let err = dom_api_error_to_vm_error(
            &vm.inner,
            DomApiError {
                kind: DomApiErrorKind::TypeError,
                message: "test type error".into(),
            },
        );
        assert!(matches!(err.kind, VmErrorKind::TypeError));
        assert_eq!(err.message, "test type error");
    }
}
