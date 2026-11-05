//! `DomApiHandler` bindings for the table-family algorithms in
//! [`super`] (slot `#11-tags-T2c-table`).
//!
//! Thin argument-coercion + dispatch shims: each handler marshals
//! `JsValue` arguments and calls the engine-independent algorithm in
//! `mod.rs`.  Registered in `crate::registry`; the handler structs are
//! re-exported from the parent module so their public paths are unchanged.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use super::{
    create_caption, create_tbody, create_tfoot, create_thead, delete_caption, delete_cell_from_row,
    delete_row_from_section, delete_row_from_table, delete_tfoot, delete_thead,
    insert_cell_into_row, insert_row_into_section, insert_row_into_table, set_caption, set_tfoot,
    set_thead,
};
use crate::util::not_found_error;

// ---------------------------------------------------------------------------
// DomApiHandler bindings
//
// VM-side per-tag prototype files invoke these via
// `invoke_dom_api(ctx, "<name>", entity, &args)` in the standard
// marshalling-only shape.  Algorithms in `super` (`mod.rs`) are the engine-independent
// truth; handlers are thin coercion + dispatch shims.
// ---------------------------------------------------------------------------

// See `#11-webidl-integer-conversion-canonical` (not WebIDL ConvertToInt).
fn require_long_arg(args: &[JsValue], index: usize) -> Result<i32, DomApiError> {
    match args.get(index) {
        Some(JsValue::Number(n)) => {
            if n.is_nan() || n.is_infinite() {
                return Ok(0);
            }
            #[allow(clippy::cast_possible_truncation)]
            let truncated = n.trunc() as i64;
            if truncated > i64::from(i32::MAX) {
                Ok(i32::MAX)
            } else if truncated < i64::from(i32::MIN) {
                Ok(i32::MIN)
            } else {
                #[allow(clippy::cast_possible_truncation)]
                Ok(truncated as i32)
            }
        }
        Some(JsValue::Bool(b)) => Ok(i32::from(*b)),
        Some(JsValue::Null) => Ok(0),
        None | Some(JsValue::Undefined) => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} is required"),
        }),
        Some(_) => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a number"),
        }),
    }
}

fn optional_long_arg(args: &[JsValue], index: usize, default: i32) -> Result<i32, DomApiError> {
    match args.get(index) {
        None | Some(JsValue::Undefined) => Ok(default),
        _ => require_long_arg(args, index),
    }
}

// See `#11-table-handlers-dedup` (duplicated helper, copy-pasted handler bodies).
fn resolve_optional_entity_arg(
    args: &[JsValue],
    index: usize,
    session: &SessionCore,
) -> Result<Option<Entity>, DomApiError> {
    match args.get(index) {
        None | Some(JsValue::Undefined | JsValue::Null) => Ok(None),
        Some(JsValue::ObjectRef(raw)) => {
            let (entity, _kind) = session
                .identity_map()
                .get(JsObjectRef::from_raw(*raw))
                .ok_or_else(|| not_found_error("argument is not a known node"))?;
            Ok(Some(entity))
        }
        _ => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a Node or null"),
        }),
    }
}

fn entity_to_objectref(entity: Entity, session: &mut SessionCore, kind: ComponentKind) -> JsValue {
    let obj_ref = session.get_or_create_wrapper(entity, kind);
    JsValue::ObjectRef(obj_ref.to_raw())
}

/// `<table>.insertRow(index?)`.
pub struct TableInsertRow;
impl DomApiHandler for TableInsertRow {
    fn method_name(&self) -> &str {
        "table.insertRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = optional_long_arg(args, 0, -1)?;
        let new = insert_row_into_table(this, dom, index)?;
        Ok(entity_to_objectref(new, session, ComponentKind::Element))
    }
}

/// `<table>.deleteRow(index)`.
pub struct TableDeleteRow;
impl DomApiHandler for TableDeleteRow {
    fn method_name(&self) -> &str {
        "table.deleteRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = require_long_arg(args, 0)?;
        delete_row_from_table(this, dom, index)?;
        Ok(JsValue::Undefined)
    }
}

/// `<thead>`/`<tbody>`/`<tfoot>`.insertRow(index?).
pub struct SectionInsertRow;
impl DomApiHandler for SectionInsertRow {
    fn method_name(&self) -> &str {
        "section.insertRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = optional_long_arg(args, 0, -1)?;
        let new = insert_row_into_section(this, dom, index)?;
        Ok(entity_to_objectref(new, session, ComponentKind::Element))
    }
}

/// `<thead>`/`<tbody>`/`<tfoot>`.deleteRow(index).
pub struct SectionDeleteRow;
impl DomApiHandler for SectionDeleteRow {
    fn method_name(&self) -> &str {
        "section.deleteRow"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = require_long_arg(args, 0)?;
        delete_row_from_section(this, dom, index)?;
        Ok(JsValue::Undefined)
    }
}

/// `<tr>.insertCell(index?)`.
pub struct RowInsertCell;
impl DomApiHandler for RowInsertCell {
    fn method_name(&self) -> &str {
        "row.insertCell"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = optional_long_arg(args, 0, -1)?;
        let new = insert_cell_into_row(this, dom, index)?;
        Ok(entity_to_objectref(new, session, ComponentKind::Element))
    }
}

/// `<tr>.deleteCell(index)`.
pub struct RowDeleteCell;
impl DomApiHandler for RowDeleteCell {
    fn method_name(&self) -> &str {
        "row.deleteCell"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = require_long_arg(args, 0)?;
        delete_cell_from_row(this, dom, index)?;
        Ok(JsValue::Undefined)
    }
}

/// `<table>.createTHead()` (idempotent).
pub struct TableCreateTHead;
impl DomApiHandler for TableCreateTHead {
    fn method_name(&self) -> &str {
        "table.createTHead"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_thead(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.createTFoot()` (idempotent).
pub struct TableCreateTFoot;
impl DomApiHandler for TableCreateTFoot {
    fn method_name(&self) -> &str {
        "table.createTFoot"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_tfoot(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.createCaption()` (idempotent).
pub struct TableCreateCaption;
impl DomApiHandler for TableCreateCaption {
    fn method_name(&self) -> &str {
        "table.createCaption"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_caption(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.createTBody()` (NOT idempotent — always creates).
pub struct TableCreateTBody;
impl DomApiHandler for TableCreateTBody {
    fn method_name(&self) -> &str {
        "table.createTBody"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let e = create_tbody(this, dom);
        Ok(entity_to_objectref(e, session, ComponentKind::Element))
    }
}

/// `<table>.deleteTHead()` (no-op when absent).
pub struct TableDeleteTHead;
impl DomApiHandler for TableDeleteTHead {
    fn method_name(&self) -> &str {
        "table.deleteTHead"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        delete_thead(this, dom);
        Ok(JsValue::Undefined)
    }
}

/// `<table>.deleteTFoot()`.
pub struct TableDeleteTFoot;
impl DomApiHandler for TableDeleteTFoot {
    fn method_name(&self) -> &str {
        "table.deleteTFoot"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        delete_tfoot(this, dom);
        Ok(JsValue::Undefined)
    }
}

/// `<table>.deleteCaption()`.
pub struct TableDeleteCaption;
impl DomApiHandler for TableDeleteCaption {
    fn method_name(&self) -> &str {
        "table.deleteCaption"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        delete_caption(this, dom);
        Ok(JsValue::Undefined)
    }
}

/// `<table>.tHead = <thead> | null` setter.
pub struct TableSetTHead;
impl DomApiHandler for TableSetTHead {
    fn method_name(&self) -> &str {
        "table.tHead.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = resolve_optional_entity_arg(args, 0, session)?;
        set_thead(this, dom, new)?;
        Ok(JsValue::Undefined)
    }
}

/// `<table>.tFoot = <tfoot> | null` setter.
pub struct TableSetTFoot;
impl DomApiHandler for TableSetTFoot {
    fn method_name(&self) -> &str {
        "table.tFoot.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = resolve_optional_entity_arg(args, 0, session)?;
        set_tfoot(this, dom, new)?;
        Ok(JsValue::Undefined)
    }
}

/// `<table>.caption = <caption> | null` setter.
pub struct TableSetCaption;
impl DomApiHandler for TableSetCaption {
    fn method_name(&self) -> &str {
        "table.caption.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = resolve_optional_entity_arg(args, 0, session)?;
        set_caption(this, dom, new)?;
        Ok(JsValue::Undefined)
    }
}
