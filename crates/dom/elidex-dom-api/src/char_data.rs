//! `CharacterData` interface methods, `Attr` node handlers, `DocumentType` handlers,
//! and additional `Document` property handlers.

use elidex_ecs::{
    AttrData, Attributes, CommentData, DocTypeData, EcsDom, Entity, NodeKind, TagType, TextContent,
};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

use crate::util::{not_found_error, require_object_ref_arg, require_string_arg};

// ===========================================================================
// CharacterData helpers
// ===========================================================================

/// Read the character data (text or comment) of an entity.
fn get_char_data(entity: Entity, dom: &EcsDom) -> Result<String, DomApiError> {
    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
        return Ok(tc.0.clone());
    }
    if let Ok(cd) = dom.world().get::<&CommentData>(entity) {
        return Ok(cd.0.clone());
    }
    Err(DomApiError {
        kind: DomApiErrorKind::InvalidStateError,
        message: "entity is not a CharacterData node".into(),
    })
}

/// Write character data (text or comment) to an entity.
fn set_char_data(entity: Entity, dom: &mut EcsDom, data: &str) -> Result<(), DomApiError> {
    if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(entity) {
        data.clone_into(&mut tc.0);
        return Ok(());
    }
    if let Ok(mut cd) = dom.world_mut().get::<&mut CommentData>(entity) {
        data.clone_into(&mut cd.0);
        return Ok(());
    }
    Err(DomApiError {
        kind: DomApiErrorKind::InvalidStateError,
        message: "entity is not a CharacterData node".into(),
    })
}

/// Extract a required numeric argument as `usize`.
fn require_usize_arg(args: &[JsValue], index: usize) -> Result<usize, DomApiError> {
    match args.get(index) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(JsValue::Number(n)) => Ok(*n as usize),
        _ => Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: format!("argument {index} must be a number"),
        }),
    }
}

/// Return an `IndexSizeError` for out-of-bounds offsets.
fn index_size_error(message: &str) -> DomApiError {
    DomApiError {
        kind: DomApiErrorKind::IndexSizeError,
        message: format!("IndexSizeError: {message}"),
    }
}

/// Return the number of UTF-16 code units in a Rust string.
///
/// WHATWG DOM uses UTF-16 code unit counts for `CharacterData.length` and all
/// offset/count parameters (§11.1). Surrogate pairs (characters outside BMP)
/// count as 2 code units.
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Convert a UTF-16 code unit offset to a byte offset in a Rust (UTF-8) string.
///
/// Returns `None` if `utf16_offset` exceeds the string's UTF-16 length or
/// lands in the middle of a surrogate pair.
fn utf16_to_byte_offset(s: &str, utf16_offset: usize) -> Option<usize> {
    let mut utf16_pos = 0;
    for (byte_pos, ch) in s.char_indices() {
        if utf16_pos == utf16_offset {
            return Some(byte_pos);
        }
        utf16_pos += ch.len_utf16();
    }
    // offset == total length means "end of string"
    if utf16_pos == utf16_offset {
        Some(s.len())
    } else {
        None
    }
}

// ===========================================================================
// CharacterData handlers
// ===========================================================================

/// `characterData.data` getter.
pub struct GetData;

impl DomApiHandler for GetData {
    fn method_name(&self) -> &str {
        "data.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let data = get_char_data(this, dom)?;
        Ok(JsValue::String(data))
    }
}

/// `characterData.data` setter.
pub struct SetData;

impl DomApiHandler for SetData {
    fn method_name(&self) -> &str {
        "data.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let data = require_string_arg(args, 0)?;
        set_char_data(this, dom, &data)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `characterData.length` getter.
///
/// Returns the number of UTF-16 code units per WHATWG DOM §11.1.
pub struct GetLength;

impl DomApiHandler for GetLength {
    fn method_name(&self) -> &str {
        "length.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let data = get_char_data(this, dom)?;
        // WHATWG DOM §11.1: CharacterData.length returns the number of
        // UTF-16 code units, not Unicode code points.
        #[allow(clippy::cast_precision_loss)] // DOM IDL uses f64 for all numeric values
        Ok(JsValue::Number(data.encode_utf16().count() as f64))
    }
}

/// `characterData.substringData(offset, count)`.
pub struct SubstringData;

impl DomApiHandler for SubstringData {
    fn method_name(&self) -> &str {
        "substringData"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let count = require_usize_arg(args, 1)?;
        let data = get_char_data(this, dom)?;
        let len = utf16_len(&data);
        if offset > len {
            return Err(index_size_error("offset exceeds data length"));
        }
        let byte_start = utf16_to_byte_offset(&data, offset)
            .ok_or_else(|| index_size_error("offset not on character boundary"))?;
        let end = (offset + count).min(len);
        let byte_end = utf16_to_byte_offset(&data, end)
            .ok_or_else(|| index_size_error("end not on character boundary"))?;
        Ok(JsValue::String(data[byte_start..byte_end].to_string()))
    }
}

/// `characterData.appendData(data)`.
pub struct AppendData;

impl DomApiHandler for AppendData {
    fn method_name(&self) -> &str {
        "appendData"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let append_str = require_string_arg(args, 0)?;
        let mut existing = get_char_data(this, dom)?;
        existing.push_str(&append_str);
        set_char_data(this, dom, &existing)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `characterData.insertData(offset, data)`.
pub struct InsertData;

impl DomApiHandler for InsertData {
    fn method_name(&self) -> &str {
        "insertData"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let insert_str = require_string_arg(args, 1)?;
        let data = get_char_data(this, dom)?;
        if offset > utf16_len(&data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        let byte_off = utf16_to_byte_offset(&data, offset)
            .ok_or_else(|| index_size_error("offset not on character boundary"))?;
        let mut result = String::with_capacity(data.len() + insert_str.len());
        result.push_str(&data[..byte_off]);
        result.push_str(&insert_str);
        result.push_str(&data[byte_off..]);
        set_char_data(this, dom, &result)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `characterData.deleteData(offset, count)`.
pub struct DeleteData;

impl DomApiHandler for DeleteData {
    fn method_name(&self) -> &str {
        "deleteData"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let count = require_usize_arg(args, 1)?;
        let data = get_char_data(this, dom)?;
        let len = utf16_len(&data);
        if offset > len {
            return Err(index_size_error("offset exceeds data length"));
        }
        let byte_start = utf16_to_byte_offset(&data, offset)
            .ok_or_else(|| index_size_error("offset not on character boundary"))?;
        let end = (offset + count).min(len);
        let byte_end = utf16_to_byte_offset(&data, end)
            .ok_or_else(|| index_size_error("end not on character boundary"))?;
        let mut result = String::with_capacity(data.len() - (byte_end - byte_start));
        result.push_str(&data[..byte_start]);
        result.push_str(&data[byte_end..]);
        set_char_data(this, dom, &result)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `characterData.replaceData(offset, count, data)`.
pub struct ReplaceData;

impl DomApiHandler for ReplaceData {
    fn method_name(&self) -> &str {
        "replaceData"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let count = require_usize_arg(args, 1)?;
        let replace_str = require_string_arg(args, 2)?;
        let data = get_char_data(this, dom)?;
        let len = utf16_len(&data);
        if offset > len {
            return Err(index_size_error("offset exceeds data length"));
        }
        let byte_start = utf16_to_byte_offset(&data, offset)
            .ok_or_else(|| index_size_error("offset not on character boundary"))?;
        let end = (offset + count).min(len);
        let byte_end = utf16_to_byte_offset(&data, end)
            .ok_or_else(|| index_size_error("end not on character boundary"))?;
        let mut result =
            String::with_capacity(data.len() - (byte_end - byte_start) + replace_str.len());
        result.push_str(&data[..byte_start]);
        result.push_str(&replace_str);
        result.push_str(&data[byte_end..]);
        set_char_data(this, dom, &result)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `text.splitText(offset)` — splits a Text node at the given offset.
///
/// Creates a new text node containing the data from `offset` onward, truncates
/// this node's data to `[0, offset)`, and inserts the new node after this one.
/// Returns the new node as an `ObjectRef`.
pub struct SplitText;

impl DomApiHandler for SplitText {
    fn method_name(&self) -> &str {
        "splitText"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // Verify this is a text node.
        let nk = dom
            .world()
            .get::<&NodeKind>(this)
            .map_err(|_| not_found_error("entity not found"))?;
        if !matches!(*nk, NodeKind::Text | NodeKind::CdataSection) {
            return Err(DomApiError {
                kind: DomApiErrorKind::InvalidStateError,
                message: "splitText: not a Text node".into(),
            });
        }
        drop(nk);

        let offset = require_usize_arg(args, 0)?;
        let data = get_char_data(this, dom)?;
        if offset > utf16_len(&data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        let byte_off = utf16_to_byte_offset(&data, offset)
            .ok_or_else(|| index_size_error("offset not on character boundary"))?;

        let head = data[..byte_off].to_string();
        let tail = data[byte_off..].to_string();

        // Update this node's data.
        set_char_data(this, dom, &head)?;

        // Create new text node with the tail.
        let new_node = dom.create_text(&tail);

        // Insert new node after this node in the parent's children.
        if let Some(parent) = dom.get_parent(this) {
            if let Some(next) = dom.get_next_sibling(this) {
                let ok = dom.insert_before(parent, new_node, next);
                debug_assert!(ok, "insert_before: parent/sibling verified");
            } else {
                let ok = dom.append_child(parent, new_node);
                debug_assert!(ok, "append_child: parent verified via get_parent");
            }
        }

        dom.rev_version(this);
        let obj_ref = session.get_or_create_wrapper(new_node, ComponentKind::TextNode);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

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

// ===========================================================================
// Document property handlers
// ===========================================================================

/// `document.URL` getter — returns `"about:blank"` for now.
pub struct GetDocumentUrl;

impl DomApiHandler for GetDocumentUrl {
    fn method_name(&self) -> &str {
        "document.URL.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String("about:blank".into()))
    }
}

/// `document.readyState` getter.
pub struct GetReadyState;

impl DomApiHandler for GetReadyState {
    fn method_name(&self) -> &str {
        "document.readyState.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String(
            session.document_ready_state.as_str().into(),
        ))
    }
}

/// `document.compatMode` getter — returns `"CSS1Compat"` (standards mode).
pub struct GetCompatMode;

impl DomApiHandler for GetCompatMode {
    fn method_name(&self) -> &str {
        "document.compatMode.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String("CSS1Compat".into()))
    }
}

/// `document.characterSet` getter — returns `"UTF-8"`.
pub struct GetCharacterSet;

impl DomApiHandler for GetCharacterSet {
    fn method_name(&self) -> &str {
        "document.characterSet.get"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Ok(JsValue::String("UTF-8".into()))
    }
}

/// Find the first child element of `parent` with tag matching `tag_name`.
fn find_child_element(dom: &EcsDom, parent: Entity, tag_name: &str) -> Option<Entity> {
    for child in dom.children_iter(parent) {
        if let Ok(tag) = dom.world().get::<&TagType>(child) {
            if tag.0 == tag_name {
                return Some(child);
            }
        }
    }
    None
}

/// `document.documentElement` getter — first Element child of the document.
pub struct GetDocumentElement;

impl DomApiHandler for GetDocumentElement {
    fn method_name(&self) -> &str {
        "document.documentElement.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        for child in dom.children_iter(this) {
            if dom.world().get::<&TagType>(child).is_ok() {
                let obj_ref = session.get_or_create_wrapper(child, ComponentKind::Element);
                return Ok(JsValue::ObjectRef(obj_ref.to_raw()));
            }
        }
        Ok(JsValue::Null)
    }
}

/// `document.head` getter — finds `<html>` child, then `<head>` child.
pub struct GetHead;

impl DomApiHandler for GetHead {
    fn method_name(&self) -> &str {
        "document.head.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::Null);
        };
        let Some(head) = find_child_element(dom, html, "head") else {
            return Ok(JsValue::Null);
        };
        let obj_ref = session.get_or_create_wrapper(head, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// `document.body` getter — finds `<html>` child, then first `<body>` or `<frameset>` child.
pub struct GetBody;

impl DomApiHandler for GetBody {
    fn method_name(&self) -> &str {
        "document.body.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::Null);
        };
        // Per spec, body is the first child of <html> that is <body> or <frameset>.
        let body = dom
            .children_iter(html)
            .find(|child| dom.has_tag(*child, "body") || dom.has_tag(*child, "frameset"));
        let Some(body) = body else {
            return Ok(JsValue::Null);
        };
        let obj_ref = session.get_or_create_wrapper(body, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// Collect text content from direct Text node children only (not descendants).
fn child_text_content(entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    for child in dom.children_iter(entity) {
        if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            result.push_str(&tc.0);
        }
    }
    result
}

/// `document.title` getter — finds `<title>` in `<head>`, strips and collapses whitespace.
pub struct GetTitle;

impl DomApiHandler for GetTitle {
    fn method_name(&self) -> &str {
        "document.title.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::String(String::new()));
        };
        let Some(head) = find_child_element(dom, html, "head") else {
            return Ok(JsValue::String(String::new()));
        };
        let Some(title_elem) = find_child_element(dom, head, "title") else {
            return Ok(JsValue::String(String::new()));
        };

        let raw = child_text_content(title_elem, dom);
        // Strip and collapse whitespace per WHATWG HTML spec.
        let collapsed: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(JsValue::String(collapsed))
    }
}

/// `document.title` setter — finds or creates `<title>` in `<head>`, sets text content.
pub struct SetTitle;

impl DomApiHandler for SetTitle {
    fn method_name(&self) -> &str {
        "document.title.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let title_text = require_string_arg(args, 0)?;

        let Some(html) = find_child_element(dom, this, "html") else {
            return Ok(JsValue::Undefined);
        };
        let Some(head) = find_child_element(dom, html, "head") else {
            return Ok(JsValue::Undefined);
        };

        let title_elem = if let Some(e) = find_child_element(dom, head, "title") {
            e
        } else {
            // Create <title> element and append to <head>.
            let t = dom.create_element("title", Attributes::default());
            let ok = dom.append_child(head, t);
            debug_assert!(ok, "append_child: head verified");
            t
        };

        // Remove existing children of <title>.
        let children: Vec<Entity> = dom.children_iter(title_elem).collect();
        for child in children {
            let ok = dom.remove_child(title_elem, child);
            debug_assert!(ok, "remove_child: child from children_iter");
        }

        // Add text node.
        if !title_text.is_empty() {
            let text_node = dom.create_text(&title_text);
            let ok = dom.append_child(title_elem, text_node);
            debug_assert!(ok, "append_child: title_elem verified");
        }

        Ok(JsValue::Undefined)
    }
}

/// `document.createDocumentFragment()`.
pub struct CreateDocumentFragment;

impl DomApiHandler for CreateDocumentFragment {
    fn method_name(&self) -> &str {
        "createDocumentFragment"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let entity = dom.create_document_fragment();
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

/// `document.createComment(data)`.
pub struct CreateComment;

impl DomApiHandler for CreateComment {
    fn method_name(&self) -> &str {
        "createComment"
    }

    fn invoke(
        &self,
        _this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let data = require_string_arg(args, 0)?;
        let entity = dom.create_comment(&data);
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Setup helpers
    // -----------------------------------------------------------------------

    fn setup_text() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let text = dom.create_text("Hello, world!");
        let session = SessionCore::new();
        (dom, text, session)
    }

    fn setup_comment() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let comment = dom.create_comment("a comment");
        let session = SessionCore::new();
        (dom, comment, session)
    }

    fn setup_document() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let doctype = dom.create_document_type(
            "html",
            "-//W3C//DTD HTML 4.01//EN",
            "http://www.w3.org/TR/html4/strict.dtd",
        );
        let html = dom.create_element("html", Attributes::default());
        let head = dom.create_element("head", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, doctype);
        dom.append_child(doc, html);
        dom.append_child(html, head);
        dom.append_child(html, body);
        let session = SessionCore::new();
        (dom, doc, session)
    }

    // -----------------------------------------------------------------------
    // CharacterData tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_data_text() {
        let (mut dom, text, mut session) = setup_text();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, world!".into()));
    }

    #[test]
    fn get_data_comment() {
        let (mut dom, comment, mut session) = setup_comment();
        let result = GetData
            .invoke(comment, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("a comment".into()));
    }

    #[test]
    fn get_data_element_error() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = GetData.invoke(div, &[], &mut session, &mut dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::InvalidStateError);
    }

    #[test]
    fn set_data_text() {
        let (mut dom, text, mut session) = setup_text();
        SetData
            .invoke(
                text,
                &[JsValue::String("new data".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("new data".into()));
    }

    #[test]
    fn set_data_comment() {
        let (mut dom, comment, mut session) = setup_comment();
        SetData
            .invoke(
                comment,
                &[JsValue::String("updated".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData
            .invoke(comment, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("updated".into()));
    }

    #[test]
    fn get_length() {
        let (mut dom, text, mut session) = setup_text();
        let result = GetLength.invoke(text, &[], &mut session, &mut dom).unwrap();
        // "Hello, world!" = 13 UTF-16 code units (all BMP)
        assert_eq!(result, JsValue::Number(13.0));
    }

    #[test]
    fn get_length_utf16_surrogate() {
        let mut dom = EcsDom::new();
        // U+1F44D (👍) is 1 Unicode code point but 2 UTF-16 code units
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        let result = GetLength.invoke(text, &[], &mut session, &mut dom).unwrap();
        // 'A' = 1, '👍' = 2, 'B' = 1 → 4 UTF-16 code units
        assert_eq!(result, JsValue::Number(4.0));
    }

    #[test]
    fn substring_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        // substringData(1, 2) should extract the emoji (2 UTF-16 code units)
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(1.0), JsValue::Number(2.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("\u{1F44D}".into()));
    }

    #[test]
    fn split_text_utf16_surrogate() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        // splitText(3) — after 'A' (1) + '👍' (2) = offset 3
        SplitText
            .invoke(text, &[JsValue::Number(3.0)], &mut session, &mut dom)
            .unwrap();
        let head = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(head, JsValue::String("A\u{1F44D}".into()));
    }

    #[test]
    fn split_text_offset_zero() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");
        let mut session = SessionCore::new();
        SplitText
            .invoke(text, &[JsValue::Number(0.0)], &mut session, &mut dom)
            .unwrap();
        let head = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(head, JsValue::String(String::new()));
    }

    #[test]
    fn split_text_offset_at_length() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");
        let mut session = SessionCore::new();
        SplitText
            .invoke(text, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        let head = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(head, JsValue::String("hello".into()));
    }

    #[test]
    fn insert_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        // "A👍B" → insert "X" at offset 3 (after emoji's 2 UTF-16 code units + 'A')
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        InsertData
            .invoke(
                text,
                &[JsValue::Number(3.0), JsValue::String("X".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("A\u{1F44D}XB".into()));
    }

    #[test]
    fn delete_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        // "A👍B" → delete 2 code units at offset 1 (the emoji)
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        DeleteData
            .invoke(
                text,
                &[JsValue::Number(1.0), JsValue::Number(2.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("AB".into()));
    }

    #[test]
    fn replace_data_utf16_surrogate() {
        let mut dom = EcsDom::new();
        // "A👍B" → replace emoji (offset 1, count 2) with "XY"
        let text = dom.create_text("A\u{1F44D}B");
        let mut session = SessionCore::new();
        ReplaceData
            .invoke(
                text,
                &[
                    JsValue::Number(1.0),
                    JsValue::Number(2.0),
                    JsValue::String("XY".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("AXYB".into()));
    }

    #[test]
    fn insert_data_at_length() {
        let (mut dom, text, mut session) = setup_text();
        // "Hello, world!" length = 13, insert at offset 13 (append)
        InsertData
            .invoke(
                text,
                &[JsValue::Number(13.0), JsValue::String("!".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let data = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(data, JsValue::String("Hello, world!!".into()));
    }

    #[test]
    fn substring_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(0.0), JsValue::Number(5.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("Hello".into()));
    }

    #[test]
    fn substring_data_middle() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(7.0), JsValue::Number(5.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("world".into()));
    }

    #[test]
    fn substring_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = SubstringData.invoke(
            text,
            &[JsValue::Number(100.0), JsValue::Number(5.0)],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn substring_data_count_exceeds() {
        let (mut dom, text, mut session) = setup_text();
        // count exceeds remaining length — should clamp, not error
        let result = SubstringData
            .invoke(
                text,
                &[JsValue::Number(10.0), JsValue::Number(100.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("ld!".into()));
    }

    #[test]
    fn append_data() {
        let (mut dom, text, mut session) = setup_text();
        AppendData
            .invoke(
                text,
                &[JsValue::String(" Goodbye!".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, world! Goodbye!".into()));
    }

    #[test]
    fn insert_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        // "Hello, world!" -> insert "beautiful " at offset 7
        InsertData
            .invoke(
                text,
                &[JsValue::Number(7.0), JsValue::String("beautiful ".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, beautiful world!".into()));
    }

    #[test]
    fn insert_data_at_start() {
        let (mut dom, text, mut session) = setup_text();
        InsertData
            .invoke(
                text,
                &[JsValue::Number(0.0), JsValue::String(">> ".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String(">> Hello, world!".into()));
    }

    #[test]
    fn insert_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = InsertData.invoke(
            text,
            &[JsValue::Number(100.0), JsValue::String("x".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn delete_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        // "Hello, world!" -> delete 7 chars from offset 5 -> "Hello!"
        DeleteData
            .invoke(
                text,
                &[JsValue::Number(5.0), JsValue::Number(7.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello!".into()));
    }

    #[test]
    fn delete_data_count_exceeds() {
        let (mut dom, text, mut session) = setup_text();
        // delete from offset 10 with count 100 — should clamp
        DeleteData
            .invoke(
                text,
                &[JsValue::Number(10.0), JsValue::Number(100.0)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, wor".into()));
    }

    #[test]
    fn delete_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = DeleteData.invoke(
            text,
            &[JsValue::Number(100.0), JsValue::Number(1.0)],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn replace_data_valid() {
        let (mut dom, text, mut session) = setup_text();
        // "Hello, world!" -> replace 5 chars at offset 7 with "Rust"
        ReplaceData
            .invoke(
                text,
                &[
                    JsValue::Number(7.0),
                    JsValue::Number(5.0),
                    JsValue::String("Rust".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello, Rust!".into()));
    }

    #[test]
    fn replace_data_out_of_bounds() {
        let (mut dom, text, mut session) = setup_text();
        let result = ReplaceData.invoke(
            text,
            &[
                JsValue::Number(100.0),
                JsValue::Number(1.0),
                JsValue::String("x".into()),
            ],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn split_text_valid() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let text = dom.create_text("HelloWorld");
        dom.append_child(parent, text);
        let mut session = SessionCore::new();
        session.get_or_create_wrapper(text, ComponentKind::Element);

        let result = SplitText
            .invoke(text, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));

        // Original node should have "Hello"
        let orig = GetData.invoke(text, &[], &mut session, &mut dom).unwrap();
        assert_eq!(orig, JsValue::String("Hello".into()));

        // Parent should now have 2 children
        let children: Vec<Entity> = dom.children_iter(parent).collect();
        assert_eq!(children.len(), 2);

        // Second child should have "World"
        let second_data = GetData
            .invoke(children[1], &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(second_data, JsValue::String("World".into()));
    }

    #[test]
    fn split_text_out_of_bounds() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("Hello");
        let mut session = SessionCore::new();
        let result = SplitText.invoke(text, &[JsValue::Number(100.0)], &mut session, &mut dom);
        assert!(result.is_err());
    }

    #[test]
    fn split_text_inserts_after() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let text1 = dom.create_text("AB");
        let text2 = dom.create_text("CD");
        dom.append_child(parent, text1);
        dom.append_child(parent, text2);
        let mut session = SessionCore::new();

        // Split text1 at offset 1 — new node "B" should appear between text1 and text2.
        SplitText
            .invoke(text1, &[JsValue::Number(1.0)], &mut session, &mut dom)
            .unwrap();

        let children: Vec<Entity> = dom.children_iter(parent).collect();
        assert_eq!(children.len(), 3);
        // children[0] = "A", children[1] = "B", children[2] = "CD"
        let d0 = GetData
            .invoke(children[0], &[], &mut session, &mut dom)
            .unwrap();
        let d1 = GetData
            .invoke(children[1], &[], &mut session, &mut dom)
            .unwrap();
        let d2 = GetData
            .invoke(children[2], &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(d0, JsValue::String("A".into()));
        assert_eq!(d1, JsValue::String("B".into()));
        assert_eq!(d2, JsValue::String("CD".into()));
    }

    #[test]
    fn split_text_on_element_error() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = SplitText.invoke(div, &[JsValue::Number(0.0)], &mut session, &mut dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::InvalidStateError);
    }

    // -----------------------------------------------------------------------
    // Attr node tests
    // -----------------------------------------------------------------------

    #[test]
    fn create_attribute() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = CreateAttribute
            .invoke(
                doc,
                &[JsValue::String("Data-X".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));

        // Name should be lowercased.
        if let JsValue::ObjectRef(id) = result {
            let (entity, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(id))
                .unwrap();
            let ad = dom.world().get::<&AttrData>(entity).unwrap();
            assert_eq!(ad.local_name, "data-x");
        }
    }

    #[test]
    fn get_attribute_node_exists() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "active");
        let div = dom.create_element("div", attrs);
        let mut session = SessionCore::new();

        let result = GetAttributeNode
            .invoke(
                div,
                &[JsValue::String("class".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));

        if let JsValue::ObjectRef(id) = result {
            let (entity, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(id))
                .unwrap();
            let ad = dom.world().get::<&AttrData>(entity).unwrap();
            assert_eq!(ad.local_name, "class");
            assert_eq!(ad.value, "active");
            assert_eq!(ad.owner_element, Some(div));
        }
    }

    #[test]
    fn get_attribute_node_missing() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();

        let result = GetAttributeNode
            .invoke(
                div,
                &[JsValue::String("nonexistent".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn set_attribute_node() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let attr = dom.create_attribute("data-x");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "42".into();
        }
        let mut session = SessionCore::new();
        let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

        SetAttributeNode
            .invoke(
                div,
                &[JsValue::ObjectRef(attr_ref.to_raw())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // Verify attribute was set.
        let attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(attrs.get("data-x"), Some("42"));

        // Verify owner was updated.
        let ad = dom.world().get::<&AttrData>(attr).unwrap();
        assert_eq!(ad.owner_element, Some(div));
    }

    #[test]
    fn set_attribute_node_in_use_error() {
        let mut dom = EcsDom::new();
        let div1 = dom.create_element("div", Attributes::default());
        let div2 = dom.create_element("div", Attributes::default());
        let attr = dom.create_attribute("data-x");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "42".into();
            ad.owner_element = Some(div1);
        }
        let mut session = SessionCore::new();
        let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

        // Try to set on div2 — should fail because owned by div1.
        let result = SetAttributeNode.invoke(
            div2,
            &[JsValue::ObjectRef(attr_ref.to_raw())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::InUseAttributeError
        );
    }

    #[test]
    fn remove_attribute_node() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("data-x", "42");
        let div = dom.create_element("div", attrs);
        let attr = dom.create_attribute("data-x");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "42".into();
            ad.owner_element = Some(div);
        }
        let mut session = SessionCore::new();
        let attr_ref = session.get_or_create_wrapper(attr, ComponentKind::Element);

        RemoveAttributeNode
            .invoke(
                div,
                &[JsValue::ObjectRef(attr_ref.to_raw())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // Verify attribute was removed.
        let el_attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert!(!el_attrs.contains("data-x"));

        // Verify owner was cleared.
        let ad = dom.world().get::<&AttrData>(attr).unwrap();
        assert_eq!(ad.owner_element, None);
    }

    #[test]
    fn attr_name() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("class");
        let mut session = SessionCore::new();
        let result = GetAttrName
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("class".into()));
    }

    #[test]
    fn attr_value_get_set() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("class");
        let mut session = SessionCore::new();

        // Initially empty.
        let result = GetAttrValue
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));

        // Set value.
        SetAttrValue
            .invoke(
                attr,
                &[JsValue::String("active".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = GetAttrValue
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("active".into()));
    }

    #[test]
    fn attr_value_syncs_to_owner() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "old");
        let div = dom.create_element("div", attrs);
        let attr = dom.create_attribute("class");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "old".into();
            ad.owner_element = Some(div);
        }
        let mut session = SessionCore::new();

        // Set attr value — should sync to element's Attributes.
        SetAttrValue
            .invoke(
                attr,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let el_attrs = dom.world().get::<&Attributes>(div).unwrap();
        assert_eq!(el_attrs.get("class"), Some("new"));
    }

    #[test]
    fn attr_owner_element() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let attr = dom.create_attribute("id");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.owner_element = Some(div);
        }
        let mut session = SessionCore::new();

        let result = GetOwnerElement
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn attr_owner_element_null() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("id");
        let mut session = SessionCore::new();
        let result = GetOwnerElement
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn attr_specified() {
        let mut dom = EcsDom::new();
        let attr = dom.create_attribute("id");
        let mut session = SessionCore::new();
        let result = GetAttrSpecified
            .invoke(attr, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    // -----------------------------------------------------------------------
    // DocumentType tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_doctype() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn get_doctype_name() {
        let (mut dom, doc, mut session) = setup_document();
        let dt_entity = find_doctype(&dom, doc).unwrap();
        let result = GetDoctypeName
            .invoke(dt_entity, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("html".into()));
    }

    #[test]
    fn get_doctype_public_id() {
        let (mut dom, doc, mut session) = setup_document();
        let dt_entity = find_doctype(&dom, doc).unwrap();
        let result = GetDoctypePublicId
            .invoke(dt_entity, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("-//W3C//DTD HTML 4.01//EN".into()));
    }

    #[test]
    fn get_doctype_system_id() {
        let (mut dom, doc, mut session) = setup_document();
        let dt_entity = find_doctype(&dom, doc).unwrap();
        let result = GetDoctypeSystemId
            .invoke(dt_entity, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(
            result,
            JsValue::String("http://www.w3.org/TR/html4/strict.dtd".into())
        );
    }

    #[test]
    fn get_doctype_none() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        dom.append_child(doc, html);
        let mut session = SessionCore::new();
        let result = GetDoctype.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // -----------------------------------------------------------------------
    // Document property tests
    // -----------------------------------------------------------------------

    #[test]
    fn document_url() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetDocumentUrl
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("about:blank".into()));
    }

    #[test]
    fn ready_state() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetReadyState
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("loading".into()));
    }

    #[test]
    fn compat_mode() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetCompatMode
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("CSS1Compat".into()));
    }

    #[test]
    fn character_set() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetCharacterSet
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("UTF-8".into()));
    }

    #[test]
    fn document_element() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetDocumentElement
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn document_element_empty() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = GetDocumentElement
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn document_head() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetHead.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn document_head_missing() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = GetHead.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn document_body() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn document_body_missing() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        dom.append_child(doc, html);
        let mut session = SessionCore::new();
        let result = GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn title_get() {
        let (mut dom, doc, mut session) = setup_document();
        let html = find_child_element(&dom, doc, "html").unwrap();
        let head = find_child_element(&dom, html, "head").unwrap();
        let title = dom.create_element("title", Attributes::default());
        let text = dom.create_text("  Hello  World  ");
        dom.append_child(head, title);
        dom.append_child(title, text);

        let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("Hello World".into()));
    }

    #[test]
    fn title_get_empty() {
        let (mut dom, doc, mut session) = setup_document();
        let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    #[test]
    fn title_set() {
        let (mut dom, doc, mut session) = setup_document();

        SetTitle
            .invoke(
                doc,
                &[JsValue::String("New Title".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("New Title".into()));
    }

    #[test]
    fn title_set_creates_element() {
        let (mut dom, doc, mut session) = setup_document();
        // No <title> exists yet.
        SetTitle
            .invoke(
                doc,
                &[JsValue::String("Created".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // Verify <title> was created.
        let html = find_child_element(&dom, doc, "html").unwrap();
        let head = find_child_element(&dom, html, "head").unwrap();
        assert!(find_child_element(&dom, head, "title").is_some());
    }

    #[test]
    fn title_set_replaces_existing() {
        let (mut dom, doc, mut session) = setup_document();
        let html = find_child_element(&dom, doc, "html").unwrap();
        let head = find_child_element(&dom, html, "head").unwrap();
        let title = dom.create_element("title", Attributes::default());
        let text = dom.create_text("Old Title");
        dom.append_child(head, title);
        dom.append_child(title, text);

        SetTitle
            .invoke(
                doc,
                &[JsValue::String("New Title".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert_eq!(result, JsValue::String("New Title".into()));
    }

    #[test]
    fn create_document_fragment() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = CreateDocumentFragment
            .invoke(doc, &[], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));

        if let JsValue::ObjectRef(id) = result {
            let (entity, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(id))
                .unwrap();
            let nk = dom.world().get::<&NodeKind>(entity).unwrap();
            assert_eq!(*nk, NodeKind::DocumentFragment);
        }
    }

    #[test]
    fn create_comment() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        let result = CreateComment
            .invoke(
                doc,
                &[JsValue::String("test comment".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));

        if let JsValue::ObjectRef(id) = result {
            let (entity, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(id))
                .unwrap();
            let cd = dom.world().get::<&CommentData>(entity).unwrap();
            assert_eq!(cd.0, "test comment");
        }
    }

    // -----------------------------------------------------------------------
    // Step 4 tests: rev_version, IndexSizeError, validation, spec fixes
    // -----------------------------------------------------------------------

    #[test]
    fn set_data_rev_version() {
        let (mut dom, text, mut session) = setup_text();
        let parent = dom.create_element("div", Attributes::default());
        dom.append_child(parent, text);
        let v1 = dom.inclusive_descendants_version(text);
        SetData
            .invoke(
                text,
                &[JsValue::String("new".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(text);
        assert_ne!(v1, v2);
    }

    #[test]
    fn append_data_rev_version() {
        let (mut dom, text, mut session) = setup_text();
        let parent = dom.create_element("div", Attributes::default());
        dom.append_child(parent, text);
        let v1 = dom.inclusive_descendants_version(text);
        AppendData
            .invoke(
                text,
                &[JsValue::String(" extra".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let v2 = dom.inclusive_descendants_version(text);
        assert_ne!(v1, v2);
    }

    #[test]
    fn index_size_error_kind() {
        let (mut dom, text, mut session) = setup_text();
        let err = SubstringData
            .invoke(
                text,
                &[JsValue::Number(999.0), JsValue::Number(1.0)],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::IndexSizeError);
    }

    #[test]
    fn create_attribute_validates_name() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut session = SessionCore::new();
        // Invalid attribute name (contains space).
        let result = CreateAttribute.invoke(
            doc,
            &[JsValue::String("invalid name".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn split_text_still_works() {
        let (mut dom, text, mut session) = setup_text();
        let parent = dom.create_element("div", Attributes::default());
        dom.append_child(parent, text);
        session.get_or_create_wrapper(text, ComponentKind::Element);

        let result = SplitText
            .invoke(text, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "Hello");
    }

    #[test]
    fn remove_attribute_node_wrong_owner() {
        let mut dom = EcsDom::new();
        let elem1 = dom.create_element("div", Attributes::default());
        let elem2 = dom.create_element("span", Attributes::default());
        {
            let mut a1 = dom.world_mut().get::<&mut Attributes>(elem1).unwrap();
            a1.set("foo", "bar");
        }
        let mut session = SessionCore::new();
        session.get_or_create_wrapper(elem1, ComponentKind::Element);
        session.get_or_create_wrapper(elem2, ComponentKind::Element);

        // Create attr node owned by elem1.
        let attr_result = GetAttributeNode
            .invoke(
                elem1,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // Try to remove it from elem2 -- should fail.
        let result = RemoveAttributeNode.invoke(elem2, &[attr_result], &mut session, &mut dom);
        assert!(result.is_err());
    }

    #[test]
    fn set_attribute_node_returns_null() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        session.get_or_create_wrapper(elem, ComponentKind::Element);

        let attr = dom.create_attribute("foo");
        {
            let mut ad = dom.world_mut().get::<&mut AttrData>(attr).unwrap();
            ad.value = "bar".into();
        }
        let attr_ref = session
            .get_or_create_wrapper(attr, ComponentKind::Element)
            .to_raw();

        let result = SetAttributeNode
            .invoke(
                elem,
                &[JsValue::ObjectRef(attr_ref)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn title_child_text_only() {
        let (mut dom, doc, mut session) = setup_document();
        let html_entity = dom
            .children_iter(doc)
            .find(|e| dom.has_tag(*e, "html"))
            .unwrap();
        let head = dom
            .children_iter(html_entity)
            .find(|e| dom.has_tag(*e, "head"))
            .unwrap();
        let title = dom.create_element("title", Attributes::default());
        dom.append_child(head, title);
        let text = dom.create_text("Hello ");
        dom.append_child(title, text);
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(title, span);
        let inner_text = dom.create_text("World");
        dom.append_child(span, inner_text);

        let result = GetTitle.invoke(doc, &[], &mut session, &mut dom).unwrap();
        // Per spec: should only get direct child text, not descendant.
        assert_eq!(result, JsValue::String("Hello".into()));
    }

    #[test]
    fn body_frameset() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        dom.append_child(doc, html);
        let frameset = dom.create_element("frameset", Attributes::default());
        dom.append_child(html, frameset);
        let mut session = SessionCore::new();

        let result = GetBody.invoke(doc, &[], &mut session, &mut dom).unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }
}
