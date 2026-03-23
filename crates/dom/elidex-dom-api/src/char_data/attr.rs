//! `Attr` node handlers.

use elidex_ecs::{AttrData, AttrEntityCache, Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::{not_found_error, require_object_ref_arg, require_string_arg};

// ===========================================================================
// Attr node handlers
// ===========================================================================

/// `document.createAttribute(name)` — creates an Attr node.
pub struct CreateAttribute;

impl DomApiHandler for CreateAttribute {
    fn method_name(&self) -> &str {
        "createAttribute"
    }

    fn invoke(
        &self,
        _this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let raw_name = require_string_arg(args, 0)?;
        crate::element::validate_attribute_name(&raw_name)?;
        let name = raw_name.to_ascii_lowercase();
        let entity = dom.create_attribute(&name);
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Attribute);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// `element.getAttributeNode(name)` — returns the Attr node for a named attribute.
pub struct GetAttributeNode;

impl DomApiHandler for GetAttributeNode {
    fn method_name(&self) -> &str {
        "getAttributeNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let name = require_string_arg(args, 0)?.to_ascii_lowercase();
        let attrs = dom
            .world()
            .get::<&Attributes>(this)
            .map_err(|_| not_found_error("element not found"))?;
        let value = match attrs.get(&name) {
            Some(v) => v.to_string(),
            None => return Ok(JsValue::Null),
        };
        drop(attrs);

        // Check the identity cache: return the same Attr entity on repeated calls.
        let cached_hit = dom
            .world()
            .get::<&AttrEntityCache>(this)
            .ok()
            .and_then(|cache| cache.entries.get(&name).copied());
        if let Some(cached_entity) = cached_hit {
            // Update the cached Attr's value to reflect the current attribute.
            if let Ok(mut ad) = dom.world_mut().get::<&mut AttrData>(cached_entity) {
                ad.value.clone_from(&value);
            }
            let obj_ref = session.get_or_create_wrapper(cached_entity, ComponentKind::Attribute);
            return Ok(JsValue::ObjectRef(obj_ref.to_raw()));
        }

        // Create a standalone Attr entity representing this attribute.
        let attr_entity = dom.create_attribute(&name);
        {
            let mut ad = dom
                .world_mut()
                .get::<&mut AttrData>(attr_entity)
                .map_err(|_| not_found_error("failed to access newly created Attr"))?;
            ad.value = value;
            ad.owner_element = Some(this);
        }

        // Cache the Attr entity for identity preservation.
        let has_cache = dom.world().get::<&AttrEntityCache>(this).is_ok();
        if has_cache {
            if let Ok(mut cache) = dom.world_mut().get::<&mut AttrEntityCache>(this) {
                cache.entries.insert(name, attr_entity);
            }
        } else {
            let mut cache = AttrEntityCache::default();
            cache.entries.insert(name, attr_entity);
            let _ = dom.world_mut().insert_one(this, cache);
        }

        let obj_ref = session.get_or_create_wrapper(attr_entity, ComponentKind::Attribute);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// `element.setAttributeNode(attr)` — sets an attribute from an Attr node.
pub struct SetAttributeNode;

impl DomApiHandler for SetAttributeNode {
    fn method_name(&self) -> &str {
        "setAttributeNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let attr_ref = require_object_ref_arg(args, 0)?;
        let (attr_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(attr_ref))
            .ok_or_else(|| not_found_error("attr not found"))?;

        // Read AttrData.
        let ad = dom
            .world()
            .get::<&AttrData>(attr_entity)
            .map_err(|_| not_found_error("not an Attr node"))?;

        // InUseAttributeError if owned by a different element.
        if let Some(owner) = ad.owner_element {
            if owner != this {
                return Err(DomApiError {
                    kind: DomApiErrorKind::InUseAttributeError,
                    message: "attribute is already in use by another element".into(),
                });
            }
        }

        let name = ad.local_name.clone();
        let value = ad.value.clone();
        drop(ad);

        // Set attribute on element.
        {
            let mut attrs = dom
                .world_mut()
                .get::<&mut Attributes>(this)
                .map_err(|_| not_found_error("element not found"))?;
            attrs.set(&name, &value);
        }

        // Update owner.
        {
            let mut ad = dom
                .world_mut()
                .get::<&mut AttrData>(attr_entity)
                .map_err(|_| not_found_error("Attr entity missing AttrData"))?;
            ad.owner_element = Some(this);
        }

        dom.rev_version(this);
        Ok(JsValue::Null)
    }
}

/// `element.removeAttributeNode(attr)` — removes an attribute via Attr node.
pub struct RemoveAttributeNode;

impl DomApiHandler for RemoveAttributeNode {
    fn method_name(&self) -> &str {
        "removeAttributeNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let attr_ref = require_object_ref_arg(args, 0)?;
        let (attr_entity, _) = session
            .identity_map()
            .get(JsObjectRef::from_raw(attr_ref))
            .ok_or_else(|| not_found_error("attr not found"))?;

        let ad = dom
            .world()
            .get::<&AttrData>(attr_entity)
            .map_err(|_| not_found_error("not an Attr node"))?;

        // Verify this attr belongs to this element.
        if ad.owner_element != Some(this) {
            return Err(not_found_error("attribute is not owned by this element"));
        }

        let name = ad.local_name.clone();
        drop(ad);

        // Remove attribute from element.
        {
            let mut attrs = dom
                .world_mut()
                .get::<&mut Attributes>(this)
                .map_err(|_| not_found_error("element not found"))?;
            attrs.remove(&name);
        }

        // Clear owner.
        {
            let mut ad = dom
                .world_mut()
                .get::<&mut AttrData>(attr_entity)
                .map_err(|_| not_found_error("Attr entity missing AttrData"))?;
            ad.owner_element = None;
        }

        dom.rev_version(this);
        Ok(JsValue::ObjectRef(attr_ref))
    }
}

/// `attr.name` getter.
pub struct GetAttrName;

impl DomApiHandler for GetAttrName {
    fn method_name(&self) -> &str {
        "attr.name.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let ad = dom
            .world()
            .get::<&AttrData>(this)
            .map_err(|_| not_found_error("not an Attr node"))?;
        Ok(JsValue::String(ad.local_name.clone()))
    }
}

/// `attr.value` getter.
pub struct GetAttrValue;

impl DomApiHandler for GetAttrValue {
    fn method_name(&self) -> &str {
        "attr.value.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let ad = dom
            .world()
            .get::<&AttrData>(this)
            .map_err(|_| not_found_error("not an Attr node"))?;
        Ok(JsValue::String(ad.value.clone()))
    }
}

/// `attr.value` setter.
pub struct SetAttrValue;

impl DomApiHandler for SetAttrValue {
    fn method_name(&self) -> &str {
        "attr.value.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;

        let owner = {
            let mut ad = dom
                .world_mut()
                .get::<&mut AttrData>(this)
                .map_err(|_| not_found_error("not an Attr node"))?;
            ad.value.clone_from(&value);
            ad.owner_element
        };

        // Sync to owner element's Attributes.
        if let Some(owner) = owner {
            let name = dom
                .world()
                .get::<&AttrData>(this)
                .map(|ad| ad.local_name.clone())
                .unwrap_or_default();
            if let Ok(mut attrs) = dom.world_mut().get::<&mut Attributes>(owner) {
                attrs.set(&name, &value);
            }
            dom.rev_version(owner);
        }

        Ok(JsValue::Undefined)
    }
}

/// `attr.ownerElement` getter.
pub struct GetOwnerElement;

impl DomApiHandler for GetOwnerElement {
    fn method_name(&self) -> &str {
        "attr.ownerElement.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let ad = dom
            .world()
            .get::<&AttrData>(this)
            .map_err(|_| not_found_error("not an Attr node"))?;
        match ad.owner_element {
            Some(owner) => {
                let obj_ref = session.get_or_create_wrapper(owner, ComponentKind::Element);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
        }
    }
}

/// `attr.specified` getter — always returns `true` per spec.
pub struct GetAttrSpecified;

impl DomApiHandler for GetAttrSpecified {
    fn method_name(&self) -> &str {
        "attr.specified.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::Bool(true))
    }
}
