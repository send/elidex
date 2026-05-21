//! `HTMLTableRowElement.prototype` intrinsic — per-tag prototype
//! layer for `<tr>` wrappers (HTML §4.9.8, slot
//! `#11-tags-T2c-table`).
//!
//! IDL surface:
//! - `rowIndex` — long, position in the **table**'s rows list
//!   (across thead, tbodies in order, tfoot).  Returns `-1` if not
//!   in a table.
//! - `sectionRowIndex` — long, position in the **parent section**'s
//!   rows list.  Returns `-1` if the parent is not
//!   `<thead>`/`<tbody>`/`<tfoot>`.
//! - `cells` — `[SameObject]` HTMLCollection of direct `<td>`/`<th>`
//!   children, interned under `WrapperKind::TableRowCells`.
//! - `insertCell(index?)` / `deleteCell(index)` — mutation methods
//!   per HTML §4.9.8, dispatched through the `row.insertCell` /
//!   `row.deleteCell` registry handlers.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.

#![cfg(feature = "engine")]

use elidex_dom_api::element::table_mutation;
use elidex_dom_api::{CollectionFilter, CollectionKind, LiveCollection};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::{coerce_long_idl_arg, coerce_optional_long_idl_arg};

impl VmInner {
    pub(in crate::vm) fn register_html_table_row_prototype(&mut self) {
        let parent = self.html_element_prototype.expect(
            "register_html_table_row_prototype called before register_html_element_prototype",
        );
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_table_row_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        for (name, getter) in [
            ("rowIndex", row_get_row_index as super::super::NativeFn),
            ("sectionRowIndex", row_get_section_row_index),
            ("cells", row_get_cells),
        ] {
            let sid = self.strings.intern(name);
            self.install_accessor_pair(proto_id, sid, getter, None, attrs);
        }

        let method_attrs = shape::PropertyAttrs::METHOD;
        let insert_cell_sid = self.strings.intern("insertCell");
        self.install_native_method(proto_id, insert_cell_sid, row_insert_cell, method_attrs);
        let delete_cell_sid = self.strings.intern("deleteCell");
        self.install_native_method(proto_id, delete_cell_sid, row_delete_cell, method_attrs);
    }

    /// Allocate (or return cached) `<tr>.cells` HTMLCollection wrapper.
    /// HTML §4.9.8 mandates `[SameObject]`.
    pub(crate) fn alloc_or_cached_table_row_cells(&mut self, owner: Entity) -> ObjectId {
        self.intern_wrapper(
            WrapperKey::entity(owner, WrapperKind::TableRowCells),
            |vm| {
                let coll = LiveCollection::new(
                    owner,
                    CollectionFilter::DirectChildrenByTagName(vec!["td".into(), "th".into()]),
                    CollectionKind::HtmlCollection,
                );
                vm.alloc_collection(coll)
            },
        )
    }
}

fn require_row_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTableRowElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "tr") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTableRowElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

fn row_get_row_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_row_receiver(ctx, this, "rowIndex")? else {
        return Ok(JsValue::Number(-1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(-1.0));
    }
    let dom = ctx.host().dom_shared();
    Ok(JsValue::Number(f64::from(table_mutation::row_index(
        entity, dom,
    ))))
}

fn row_get_section_row_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_row_receiver(ctx, this, "sectionRowIndex")? else {
        return Ok(JsValue::Number(-1.0));
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Number(-1.0));
    }
    let dom = ctx.host().dom_shared();
    Ok(JsValue::Number(f64::from(
        table_mutation::section_row_index(entity, dom),
    )))
}

fn row_get_cells(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_row_receiver(ctx, this, "cells")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_table_row_cells(entity);
    Ok(JsValue::Object(id))
}

fn row_insert_cell(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_row_receiver(ctx, this, "insertCell")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let index = coerce_optional_long_idl_arg(ctx, args, -1)?;
    invoke_dom_api(
        ctx,
        "row.insertCell",
        entity,
        &[JsValue::Number(f64::from(index))],
    )
}

fn row_delete_cell(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_row_receiver(ctx, this, "deleteCell")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let index = coerce_long_idl_arg(ctx, args)?;
    invoke_dom_api(
        ctx,
        "row.deleteCell",
        entity,
        &[JsValue::Number(f64::from(index))],
    )
}
