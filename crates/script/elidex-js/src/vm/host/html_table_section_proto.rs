//! `HTMLTableSectionElement.prototype` intrinsic — shared per-tag
//! prototype layer for `<thead>` / `<tbody>` / `<tfoot>` wrappers
//! (HTML §4.9.5-7, slot `#11-tags-T2c-table`).
//!
//! IDL surface:
//! - `rows` — `[SameObject]` HTMLCollection of direct `<tr>`
//!   children, identity-cached per
//!   [`super::super::VmInner::table_section_rows_wrappers`].
//! - `insertRow(index?)` / `deleteRow(index)` — mutation methods,
//!   dispatched through the `section.insertRow` /
//!   `section.deleteRow` registry handlers.
//!
//! The brand check accepts any of `<thead>` / `<tbody>` / `<tfoot>`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.

#![cfg(feature = "engine")]

use elidex_dom_api::{CollectionFilter, CollectionKind, LiveCollection};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::{coerce_long_idl_arg, coerce_optional_long_idl_arg};

impl VmInner {
    pub(in crate::vm) fn register_html_table_section_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_table_section_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_table_section_prototype = Some(proto_id);

        let rows_sid = self.strings.intern("rows");
        self.install_accessor_pair(
            proto_id,
            rows_sid,
            section_get_rows,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let method_attrs = shape::PropertyAttrs::METHOD;
        let insert_row_sid = self.strings.intern("insertRow");
        self.install_native_method(proto_id, insert_row_sid, section_insert_row, method_attrs);
        let delete_row_sid = self.strings.intern("deleteRow");
        self.install_native_method(proto_id, delete_row_sid, section_delete_row, method_attrs);
    }

    /// Allocate (or return cached) `<thead>`/`<tbody>`/`<tfoot>`.rows
    /// HTMLCollection wrapper.  HTML §4.9.5-7 mandates `[SameObject]`.
    pub(crate) fn alloc_or_cached_table_section_rows(&mut self, owner: Entity) -> ObjectId {
        if let Some(&id) = self.table_section_rows_wrappers.get(&owner) {
            return id;
        }
        let coll = LiveCollection::new(
            owner,
            CollectionFilter::DirectChildrenByTagName(vec!["tr".into()]),
            CollectionKind::HtmlCollection,
        );
        let id = self.alloc_collection(coll);
        self.table_section_rows_wrappers.insert(owner, id);
        id
    }
}

fn require_section_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTableSectionElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    let host = ctx.host();
    if !host.tag_matches_ascii_case(entity, "thead")
        && !host.tag_matches_ascii_case(entity, "tbody")
        && !host.tag_matches_ascii_case(entity, "tfoot")
    {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTableSectionElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn section_get_rows(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_section_receiver(ctx, this, "rows")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_table_section_rows(entity);
    Ok(JsValue::Object(id))
}

fn section_insert_row(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_section_receiver(ctx, this, "insertRow")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let index = coerce_optional_long_idl_arg(ctx, args, -1)?;
    invoke_dom_api(
        ctx,
        "section.insertRow",
        entity,
        &[JsValue::Number(f64::from(index))],
    )
}

fn section_delete_row(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_section_receiver(ctx, this, "deleteRow")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let index = coerce_long_idl_arg(ctx, args)?;
    invoke_dom_api(
        ctx,
        "section.deleteRow",
        entity,
        &[JsValue::Number(f64::from(index))],
    )
}
