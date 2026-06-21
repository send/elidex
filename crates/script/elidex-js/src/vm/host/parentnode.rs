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
//! The mutation methods are **thin dispatchers** (B1.2b convergence): they
//! brand-check the receiver and route to the engine-independent dom-api handler
//! via [`super::childnode::dispatch_child_parent_mixin`], which owns "convert
//! nodes into a node" (§4.2.6), validity, the insert/replace-all, and
//! `MutationRecord` production. Same path boa uses (One-issue-one-way).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::{NativeFn, VmInner};
use super::childnode::dispatch_child_parent_mixin;
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api,
    query_selector_all_snapshot, tree_nav_getter,
};
use super::event_target::entity_from_this;

use elidex_ecs::NodeKind;

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
    dispatch_child_parent_mixin(
        ctx,
        this,
        args,
        "ParentNode",
        "prepend",
        is_parent_node_kind,
    )
}

fn native_parent_node_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_child_parent_mixin(ctx, this, args, "ParentNode", "append", is_parent_node_kind)
}

fn native_parent_node_replace_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_child_parent_mixin(
        ctx,
        this,
        args,
        "ParentNode",
        "replaceChildren",
        is_parent_node_kind,
    )
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
/// which returns `null` on unbound receivers.  Intentionally no
/// WebIDL brand check: `entity_from_this` filters non-host receivers
/// and the engine-side walker (which uses `EcsDom::is_element`) just
/// returns `None` for receivers with no element children.
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
