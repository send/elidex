//! ParentNode mixin (WHATWG DOM §4.2.6) — mutation methods
//! (`prepend` / `append` / `replaceChildren`) via
//! [`VmInner::install_parent_node_mixin`] and read surface
//! (`children` / `firstElementChild` / `lastElementChild` /
//! `childElementCount` / `querySelector` / `querySelectorAll`) via
//! [`VmInner::install_parent_node_readers`].
//!
//! Spec receivers are Element / Document / DocumentFragment.  Both
//! installs run on `Element.prototype` and `DocumentFragment.prototype`
//! (ShadowRoot inherits via the latter); Document has no shared
//! prototype so its wrapper picks up the four RO accessors via
//! [`super::document::DOCUMENT_RO_ACCESSORS`] and keeps its existing
//! `native_document_query_selector*` own-properties.
//!
//! Argument normalisation for the mutation methods reuses
//! [`super::childnode::convert_nodes_to_single_node_or_fragment`] so
//! the Phase 2 (spec §4.2.5) rules are identical.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::{NativeFn, VmInner};
use super::childnode::{convert_nodes_to_single_node_or_fragment, finalize_pair};
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api, nodes_to_insert,
    query_selector_all_snapshot, tree_nav_getter,
};
use super::event_target::entity_from_this;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Install `prepend` / `append` / `replaceChildren` on
    /// `proto_id` (Element.prototype or a document wrapper).
    pub(in crate::vm) fn install_parent_node_mixin(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (
                self.well_known.prepend,
                native_parent_node_prepend as NativeFn,
            ),
            (self.well_known.append, native_parent_node_append),
            (
                self.well_known.replace_children,
                native_parent_node_replace_children,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

/// Thin wrapper over [`super::dom_exception::hierarchy_request_error`]
/// that fills in the `'ParentNode'` interface label.  Mixin is
/// installed on `Element.prototype` and the document wrapper, so
/// `document.append(...)` and `element.prepend(...)` both surface
/// through this factory.
fn hierarchy_request_error(ctx: &NativeContext<'_>, method: &str) -> VmError {
    super::dom_exception::hierarchy_request_error(
        ctx.vm.well_known.dom_exc_hierarchy_request_error,
        "ParentNode",
        method,
        "the new child node cannot be inserted.",
    )
}

/// ParentNode mixin receivers per WHATWG §4.2.6 — Element,
/// Document, DocumentFragment.  ShadowRoot is `NodeKind::DocumentFragment`
/// in our model (it carries the `elidex_ecs::ShadowRoot` brand
/// component but shares the fragment node-kind), so it accepts
/// uniformly without a separate kind enum.
fn is_parent_node_kind(k: NodeKind) -> bool {
    matches!(
        k,
        NodeKind::Element | NodeKind::Document | NodeKind::DocumentFragment
    )
}

fn native_parent_node_prepend(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "prepend",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Pre-insertion validity (WHATWG §4.2.3): reject ancestor
    // cycles and self-insert BEFORE mutating the tree so a throw
    // leaves the parent unchanged.  Same pattern as
    // `replaceChildren` / `replaceWith`.
    for &child in &children {
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, pair, false);
            return Err(hierarchy_request_error(ctx, "prepend"));
        }
    }
    // Track the "reference child" we insert before.  Starts as the
    // parent's current first child; if we'd insert a node as its own
    // reference, advance to that node's next sibling (WHATWG
    // pre-insert no-op).  Snapshotting is correct because
    // `insert_before` leaves the reference child's position intact
    // relative to nodes inserted in front of it.
    let mut ref_child = ctx.host().dom().children_iter(parent).next();
    let mut err = None;
    for child in children {
        if ref_child == Some(child) {
            ref_child = ctx.host().dom().get_next_sibling(child);
            continue;
        }
        let ok = match ref_child {
            Some(r) => ctx.host().dom().insert_before(parent, child, r),
            None => ctx.host().dom().append_child(parent, child),
        };
        if !ok {
            err = Some(hierarchy_request_error(ctx, "prepend"));
            break;
        }
    }
    finalize_pair(ctx, pair, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_parent_node_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "append",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    let Some(pair) = convert_nodes_to_single_node_or_fragment(ctx, args)? else {
        return Ok(JsValue::Undefined);
    };
    let children = nodes_to_insert(ctx, pair.0);
    // Pre-insertion validity (WHATWG §4.2.3): cycle / self-insert
    // rejection BEFORE any mutation, matching the rest of the
    // ParentNode / ChildNode mixin.
    for &child in &children {
        if ctx
            .host()
            .dom()
            .is_light_tree_ancestor_or_self(child, parent)
        {
            finalize_pair(ctx, pair, false);
            return Err(hierarchy_request_error(ctx, "append"));
        }
    }
    let mut err = None;
    for child in children {
        if !ctx.host().dom().append_child(parent, child) {
            err = Some(hierarchy_request_error(ctx, "append"));
            break;
        }
    }
    finalize_pair(ctx, pair, err.is_none());
    err.map_or(Ok(JsValue::Undefined), Err)
}

fn native_parent_node_replace_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "replaceChildren",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Undefined);
    };
    // WHATWG §4.2.3 order: "convert nodes into a node" → "ensure
    // pre-insertion validity" → "replace all".  If validity fails,
    // the replace-all step never runs, so the tree is unchanged.
    // Our earlier implementation inverted the validity check (tried
    // the insertion and rolled back on failure), which broke when an
    // argument was one of `parent`'s own children: normalisation
    // already reparented it into the wrapper fragment, so the
    // rollback snapshot was incomplete and the original tree could
    // not be restored.  We now validate up-front and only mutate
    // after we know every child is insertable.
    let pair = convert_nodes_to_single_node_or_fragment(ctx, args)?;
    if let Some(p) = pair {
        let to_insert = nodes_to_insert(ctx, p.0);
        // Pre-validate every leaf — `EcsDom::append_child` rejects
        // self-insert (`child == parent`) and ancestor cycles
        // (`child` is an ancestor of `parent`).  Checking with
        // `is_light_tree_ancestor_or_self` covers both in one call.
        for &child in &to_insert {
            if ctx
                .host()
                .dom()
                .is_light_tree_ancestor_or_self(child, parent)
            {
                finalize_pair(ctx, p, false);
                return Err(hierarchy_request_error(ctx, "replaceChildren"));
            }
        }
        let existing: Vec<Entity> = ctx.host().dom().children_iter(parent).collect();
        for child in existing {
            let _ = ctx.host().dom().remove_child(parent, child);
        }
        let mut err = None;
        for child in to_insert {
            // Pre-validation covers `EcsDom::append_child`'s documented
            // rejection modes (self-insert, ancestor cycle), so this
            // branch is unreachable under the current invariants.
            // Surface an error anyway as defence-in-depth: silently
            // dropping a child would hide any future ECS invariant
            // regression and still leave the parent half-populated,
            // which is worse than an explicit throw.
            if !ctx.host().dom().append_child(parent, child) {
                err = Some(hierarchy_request_error(ctx, "replaceChildren"));
                break;
            }
        }
        finalize_pair(ctx, p, err.is_none());
        if let Some(e) = err {
            return Err(e);
        }
    } else {
        // No args — clear the parent.
        let existing: Vec<Entity> = ctx.host().dom().children_iter(parent).collect();
        for child in existing {
            let _ = ctx.host().dom().remove_child(parent, child);
        }
    }
    Ok(JsValue::Undefined)
}

// === Read surface (WHATWG §4.2.6) ===

impl VmInner {
    /// Install the four ParentNode read accessors + the two selector
    /// methods on `proto_id` (Element.prototype, DocumentFragment.prototype).
    pub(in crate::vm) fn install_parent_node_readers(&mut self, proto_id: ObjectId) {
        for (name_sid, getter) in [
            (
                self.well_known.first_element_child,
                native_pn_first_element_child as NativeFn,
            ),
            (
                self.well_known.last_element_child,
                native_pn_last_element_child,
            ),
            (self.well_known.children, native_pn_children),
            (
                self.well_known.child_element_count,
                native_pn_child_element_count,
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
        for (name_sid, func) in [
            (
                self.well_known.query_selector,
                native_pn_query_selector as NativeFn,
            ),
            (
                self.well_known.query_selector_all,
                native_pn_query_selector_all,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

/// `firstElementChild` / `lastElementChild` use [`tree_nav_getter`],
/// which returns `null` on unbound receivers.  No brand check: a
/// non-parent receiver has no `TagType` children, so the lookup
/// naturally returns `None`.
pub(super) fn native_pn_first_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::first_element_child)
}

pub(super) fn native_pn_last_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::last_element_child)
}

/// `children` returns a fresh live `HTMLCollection` per access.  This
/// does **not** satisfy the `[SameObject] children` annotation in
/// WHATWG §4.2.6 IDL (which requires
/// `node.children === node.children`).  Caching the wrapper requires
/// a per-entity side-table mirroring
/// `html_form_proto.rs::form_elements_wrappers` and is tracked under
/// defer slot `#11-parentnode-children-sameobject-cache` (trigger:
/// framework site or WPT failure on identity preservation).
pub(super) fn native_pn_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let id = ctx.vm.alloc_collection(elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::ElementChildren,
        elidex_dom_api::CollectionKind::HtmlCollection,
    ));
    Ok(JsValue::Object(id))
}

pub(super) fn native_pn_child_element_count(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(0.0));
    };
    let dom = ctx.host().dom();
    let count = dom
        .children_iter(entity)
        .filter(|c| dom.is_element(*c))
        .count();
    #[allow(clippy::cast_precision_loss)]
    let count_f = count as f64;
    Ok(JsValue::Number(count_f))
}

/// `querySelector(selector)` — subtree-scoped per WHATWG §4.2.6.
/// `this` itself is never a match candidate, only its descendants.
/// The brand check is relaxed from Element-only to
/// `is_parent_node_kind` so DocumentFragment / ShadowRoot receivers
/// route through the same engine-indep `QuerySelector` handler.
fn native_pn_query_selector(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "querySelector",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Null);
    };
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "querySelector", entity, &[JsValue::String(target_sid)])
}

/// `querySelectorAll(selector)` — subtree-scoped, returns a static
/// `NodeList` snapshot per WHATWG §4.2.6.
fn native_pn_query_selector_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = super::event_target::require_receiver(
        ctx,
        this,
        "ParentNode",
        "querySelectorAll",
        is_parent_node_kind,
    )?
    else {
        return Ok(JsValue::Null);
    };
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    query_selector_all_snapshot(ctx, entity, &selector_str)
}
