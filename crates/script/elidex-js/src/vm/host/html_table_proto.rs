//! `HTMLTableElement.prototype` intrinsic — per-tag prototype layer
//! for `<table>` wrappers (HTML §4.9.1, slot `#11-tags-T2c-table`).
//!
//! IDL surface:
//! - `caption` / `tHead` / `tFoot` — getter+setter pairs.  Setters
//!   throw `HierarchyRequestError` for non-matching tag arguments;
//!   `null` removes any existing matching child.  Algorithm in
//!   `elidex_dom_api::element::table_mutation::{set_caption, set_thead, set_tfoot}`.
//! - `tBodies` / `rows` — `[SameObject]` HTMLCollections of direct
//!   `<tbody>` children and the spec-ordered table rows
//!   (thead/tbodies/tfoot per HTML §4.9.1) respectively.
//! - `createTHead` / `createTFoot` / `createCaption` — idempotent
//!   create-or-return.
//! - `createTBody` — **NOT idempotent** (always creates a new
//!   `<tbody>`).
//! - `delete{THead,TFoot,Caption}` — remove if exists, no-op
//!   otherwise.  No `deleteTBody` per spec.
//! - `insertRow(index?)` / `deleteRow(index)` — mutation methods.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", marshalling-only.  All mutation
//! and walker logic lives engine-indep in
//! `elidex_dom_api::element::table_mutation` and
//! `elidex_dom_api::live_collection`.

#![cfg(feature = "engine")]

use elidex_dom_api::element::table_mutation;
use elidex_dom_api::{CollectionFilter, CollectionKind, LiveCollection};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::VmInner;
use super::dom_bridge::invoke_dom_api;
use super::idl_coerce::{coerce_long_idl_arg, coerce_optional_long_idl_arg};

impl VmInner {
    pub(in crate::vm) fn register_html_table_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_table_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_html_subclass_prototype(parent);
        self.html_table_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
        // Section accessors (getter + setter pairs).
        for (name, getter, setter) in [
            (
                "caption",
                table_get_caption as super::super::NativeFn,
                Some(table_set_caption as super::super::NativeFn),
            ),
            ("tHead", table_get_thead, Some(table_set_thead)),
            ("tFoot", table_get_tfoot, Some(table_set_tfoot)),
        ] {
            let sid = self.strings.intern(name);
            self.install_accessor_pair(proto_id, sid, getter, setter, attrs);
        }

        // SameObject collection accessors (read-only).
        for (name, getter) in [
            ("tBodies", table_get_tbodies as super::super::NativeFn),
            ("rows", table_get_rows),
        ] {
            let sid = self.strings.intern(name);
            self.install_accessor_pair(proto_id, sid, getter, None, attrs);
        }

        // Methods.
        let method_attrs = shape::PropertyAttrs::METHOD;
        for (name, func) in [
            ("createTHead", table_create_thead as super::super::NativeFn),
            ("createTFoot", table_create_tfoot),
            ("createCaption", table_create_caption),
            ("createTBody", table_create_tbody),
            ("deleteTHead", table_delete_thead),
            ("deleteTFoot", table_delete_tfoot),
            ("deleteCaption", table_delete_caption),
            ("insertRow", table_insert_row),
            ("deleteRow", table_delete_row),
        ] {
            let sid = self.strings.intern(name);
            self.install_native_method(proto_id, sid, func, method_attrs);
        }
    }

    /// Allocate (or return cached) `<table>.rows` HTMLCollection
    /// wrapper.  HTML §4.9.1 mandates `[SameObject]`.
    pub(crate) fn alloc_or_cached_table_rows(&mut self, owner: Entity) -> ObjectId {
        if let Some(&id) = self.table_rows_wrappers.get(&owner) {
            return id;
        }
        let coll = LiveCollection::new(
            owner,
            CollectionFilter::TableRows,
            CollectionKind::HtmlCollection,
        );
        let id = self.alloc_collection(coll);
        self.table_rows_wrappers.insert(owner, id);
        id
    }

    /// Allocate (or return cached) `<table>.tBodies` HTMLCollection
    /// wrapper.  HTML §4.9.1 mandates `[SameObject]`.
    pub(crate) fn alloc_or_cached_table_bodies(&mut self, owner: Entity) -> ObjectId {
        if let Some(&id) = self.table_bodies_wrappers.get(&owner) {
            return id;
        }
        let coll = LiveCollection::new(
            owner,
            CollectionFilter::DirectChildrenByTagName(vec!["tbody".into()]),
            CollectionKind::HtmlCollection,
        );
        let id = self.alloc_collection(coll);
        self.table_bodies_wrappers.insert(owner, id);
        id
    }
}

fn require_table_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLTableElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "table") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLTableElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// -- caption / tHead / tFoot getters ---------------------------------------

fn first_section_child_wrapper(ctx: &mut NativeContext<'_>, table: Entity, tag: &str) -> JsValue {
    // Reuses `table_mutation::first_section_child` so the
    // VM-side getter walk stays in lockstep with the
    // engine-indep `create_*` / `delete_*` algorithms.
    let dom = ctx.host().dom_shared();
    match table_mutation::first_section_child(table, dom, tag) {
        Some(e) => {
            let id = ctx.vm.create_element_wrapper(e);
            JsValue::Object(id)
        }
        None => JsValue::Null,
    }
}

fn table_get_caption(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "caption")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    Ok(first_section_child_wrapper(ctx, entity, "caption"))
}

fn table_get_thead(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "tHead")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    Ok(first_section_child_wrapper(ctx, entity, "thead"))
}

fn table_get_tfoot(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "tFoot")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    Ok(first_section_child_wrapper(ctx, entity, "tfoot"))
}

// -- caption / tHead / tFoot setters --------------------------------------

fn table_set_caption(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    table_section_setter(ctx, this, args, "caption", "table.caption.set")
}

fn table_set_thead(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    table_section_setter(ctx, this, args, "tHead", "table.tHead.set")
}

fn table_set_tfoot(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    table_section_setter(ctx, this, args, "tFoot", "table.tFoot.set")
}

fn table_section_setter(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    handler: &'static str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    invoke_dom_api(ctx, handler, entity, args)
}

// -- tBodies / rows (SameObject) ------------------------------------------

fn table_get_tbodies(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "tBodies")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_table_bodies(entity);
    Ok(JsValue::Object(id))
}

fn table_get_rows(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "rows")? else {
        return Ok(JsValue::Null);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let id = ctx.vm.alloc_or_cached_table_rows(entity);
    Ok(JsValue::Object(id))
}

// -- mutation methods (each forwards to a registry handler) ---------------

fn dispatch_table_method(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    method: &str,
    handler: &'static str,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, method)? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    invoke_dom_api(ctx, handler, entity, args)
}

fn table_create_thead(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "createTHead", "table.createTHead")
}

fn table_create_tfoot(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "createTFoot", "table.createTFoot")
}

fn table_create_caption(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "createCaption", "table.createCaption")
}

fn table_create_tbody(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "createTBody", "table.createTBody")
}

fn table_delete_thead(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "deleteTHead", "table.deleteTHead")
}

fn table_delete_tfoot(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "deleteTFoot", "table.deleteTFoot")
}

fn table_delete_caption(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    dispatch_table_method(ctx, this, args, "deleteCaption", "table.deleteCaption")
}

fn table_insert_row(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "insertRow")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // WebIDL ToNumber + i32 saturate VM-side so handler receives a
    // pre-coerced number (lesson #201: `<table>.insertRow("0")` and
    // `<table>.insertRow({valueOf: () => 0})` must reach the
    // algorithm with index = 0, not throw TypeError).
    let index = coerce_optional_long_idl_arg(ctx, args, -1)?;
    invoke_dom_api(
        ctx,
        "table.insertRow",
        entity,
        &[JsValue::Number(f64::from(index))],
    )
}

fn table_delete_row(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_table_receiver(ctx, this, "deleteRow")? else {
        return Ok(JsValue::Undefined);
    };
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let index = coerce_long_idl_arg(ctx, args)?;
    invoke_dom_api(
        ctx,
        "table.deleteRow",
        entity,
        &[JsValue::Number(f64::from(index))],
    )
}
