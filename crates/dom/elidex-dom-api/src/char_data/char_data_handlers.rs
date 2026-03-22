//! `CharacterData` interface method handlers.

use elidex_ecs::{CommentData, EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

use crate::util::{not_found_error, require_string_arg};

// ===========================================================================
// CharacterData helpers
// ===========================================================================

/// Read the character data (text or comment) of an entity.
pub(crate) fn get_char_data(entity: Entity, dom: &EcsDom) -> Result<String, DomApiError> {
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
pub(crate) fn set_char_data(
    entity: Entity,
    dom: &mut EcsDom,
    data: &str,
) -> Result<(), DomApiError> {
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
pub(crate) fn require_usize_arg(args: &[JsValue], index: usize) -> Result<usize, DomApiError> {
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
pub(crate) fn index_size_error(message: &str) -> DomApiError {
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
pub(crate) fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Convert a UTF-16 code unit offset to a byte offset in a Rust (UTF-8) string.
///
/// Returns `None` if `utf16_offset` exceeds the string's UTF-16 length or
/// lands in the middle of a surrogate pair.
pub(crate) fn utf16_to_byte_offset(s: &str, utf16_offset: usize) -> Option<usize> {
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
