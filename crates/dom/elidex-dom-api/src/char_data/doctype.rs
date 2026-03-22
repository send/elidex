//! `DocumentType` handlers.

use elidex_ecs::{DocTypeData, EcsDom, Entity, NodeKind};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiError, DomApiHandler, SessionCore};

use crate::util::not_found_error;

// ===========================================================================
// DocumentType handlers
// ===========================================================================

/// Walk document children to find the first entity with `NodeKind::DocumentType`.
fn find_doctype(dom: &EcsDom, doc: Entity) -> Option<Entity> {
    for child in dom.children_iter(doc) {
        if let Ok(nk) = dom.world().get::<&NodeKind>(child) {
            if *nk == NodeKind::DocumentType {
                return Some(child);
            }
        }
    }
    None
}

/// `document.doctype` getter.
pub struct GetDoctype;

impl DomApiHandler for GetDoctype {
    fn method_name(&self) -> &str {
        "doctype.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        match find_doctype(dom, this) {
            Some(entity) => {
                let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
        }
    }
}

/// `documentType.name` getter.
pub struct GetDoctypeName;

impl DomApiHandler for GetDoctypeName {
    fn method_name(&self) -> &str {
        "doctype.name.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let dt = dom
            .world()
            .get::<&DocTypeData>(this)
            .map_err(|_| not_found_error("not a DocumentType node"))?;
        Ok(JsValue::String(dt.name.clone()))
    }
}

/// `documentType.publicId` getter.
pub struct GetDoctypePublicId;

impl DomApiHandler for GetDoctypePublicId {
    fn method_name(&self) -> &str {
        "doctype.publicId.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let dt = dom
            .world()
            .get::<&DocTypeData>(this)
            .map_err(|_| not_found_error("not a DocumentType node"))?;
        Ok(JsValue::String(dt.public_id.clone()))
    }
}

/// `documentType.systemId` getter.
pub struct GetDoctypeSystemId;

impl DomApiHandler for GetDoctypeSystemId {
    fn method_name(&self) -> &str {
        "doctype.systemId.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let dt = dom
            .world()
            .get::<&DocTypeData>(this)
            .map_err(|_| not_found_error("not a DocumentType node"))?;
        Ok(JsValue::String(dt.system_id.clone()))
    }
}
