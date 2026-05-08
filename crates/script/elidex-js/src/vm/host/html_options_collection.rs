//! `HTMLOptionsCollection.prototype` intrinsic — the `select.options`
//! mutable surface (HTML §4.10.10.2).  Subclass of
//! `HTMLCollection.prototype`; everything inherited (`length` /
//! `item` / `namedItem` / `[Symbol.iterator]`) flows through the
//! parent prototype, while this module installs the four
//! Options-only members:
//!
//! - `add(option, before?)` — same algorithm as
//!   `HTMLSelectElement.prototype.add` (HTML §4.10.7.5), reached
//!   here via the collection's root entity.
//! - `remove(idx)` — option-at-index detach.  Mirrors
//!   `HTMLSelectElement.prototype.remove(idx)` numeric overload
//!   (HTML §4.10.7.6 / §4.10.10.2).
//! - `length` setter — extends with bare `<option>` elements or
//!   truncates from the end (HTML §4.10.10.2).  The getter is
//!   inherited from `HTMLCollection.prototype.length` accessor.
//! - `selectedIndex` (R/W) — alias for `select.selectedIndex`,
//!   exposed on the collection per HTML §4.10.10.2.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", every algorithm body (option
//! insertion, removal, selectedness fallback) lives in
//! `html_select_proto::*_impl` helpers (which themselves delegate to
//! `elidex_form` for the spec algorithms — D-3 / D-4 of the drift-
//! hoist PR).  This module is purely brand-check + entity resolution
//! glue between the OptionsCollection wrapper and the shared
//! algorithm helpers.
//!
//! ## Brand check
//!
//! The receiver must be an [`ObjectKind::HtmlCollection`] whose
//! backing [`elidex_dom_api::LiveCollection`] has
//! `CollectionFilter::Options`.  A non-Options HTMLCollection (e.g.
//! `getElementsByTagName` result) reaches a separate branch via the
//! `OPTIONS_INTERFACE` brand string and throws "Illegal invocation"
//! — preserving the WebIDL invariant that `.call(other_kind)` is
//! rejected.

#![cfg(feature = "engine")]

use elidex_dom_api::CollectionFilter;
use elidex_ecs::Entity;

use super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::html_select_proto::{
    select_add_impl, select_remove_option_at_impl, select_set_options_length_impl,
    select_set_selected_index_impl,
};

const OPTIONS_INTERFACE: &str = "HTMLOptionsCollection";

/// Brand-check: the receiver must be an HTMLCollection whose backing
/// `LiveCollection` carries `CollectionFilter::Options`.  Returns
/// the underlying `<select>` entity (the collection root) for the
/// caller to forward to a select-side algorithm.
fn require_options_collection_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<Option<Entity>, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{OPTIONS_INTERFACE}': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::HtmlCollection) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{OPTIONS_INTERFACE}': Illegal invocation"
        )));
    }
    let coll = ctx.vm.live_collection_states.get(&id).ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on '{OPTIONS_INTERFACE}': Illegal invocation"
        ))
    })?;
    if !matches!(coll.filter(), CollectionFilter::Options) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{OPTIONS_INTERFACE}': Illegal invocation"
        )));
    }
    Ok(coll.root())
}

pub(super) fn native_options_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select_entity) = require_options_collection_receiver(ctx, this, "add")? else {
        return Ok(JsValue::Undefined);
    };
    select_add_impl(ctx, select_entity, args)
}

pub(super) fn native_options_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select_entity) = require_options_collection_receiver(ctx, this, "remove")? else {
        return Ok(JsValue::Undefined);
    };
    let idx_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    select_remove_option_at_impl(ctx, select_entity, &idx_arg)
}

pub(super) fn native_options_set_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select_entity) = require_options_collection_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Undefined);
    };
    select_set_options_length_impl(ctx, select_entity, args)
}

pub(super) fn native_options_get_selected_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select_entity) = require_options_collection_receiver(ctx, this, "selectedIndex")?
    else {
        return Ok(JsValue::Number(-1.0));
    };
    let value = elidex_form::select_selected_index(ctx.host().dom(), select_entity);
    Ok(JsValue::Number(value))
}

pub(super) fn native_options_set_selected_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(select_entity) = require_options_collection_receiver(ctx, this, "selectedIndex")?
    else {
        return Ok(JsValue::Undefined);
    };
    select_set_selected_index_impl(ctx, select_entity, args)
}
