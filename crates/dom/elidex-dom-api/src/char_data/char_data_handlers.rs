//! `CharacterData` interface method handlers.

use elidex_ecs::{CommentData, EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_replace_data, ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

use crate::char_data::split_text::{split_text_at_offset, SplitTextError};
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
/// offset/count parameters (§4.10 Interface CharacterData). Surrogate pairs
/// (characters outside BMP) count as 2 code units.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Convert a UTF-16 code unit offset to a byte offset in a Rust (UTF-8)
/// string. Returns `None` if `utf16_offset` exceeds the string's UTF-16
/// length or lands in the middle of a surrogate pair.
///
/// Currently used by [`Range`](crate::Range) for boundary-point math
/// where a `None` result is tolerated via `.unwrap_or(s.len())`. The
/// CharacterData splice methods route through `apply_replace_data`
/// (→ `EcsDom::replace_text_data` / `replace_comment_data`) instead,
/// because they must accept mid-surrogate offsets per the WHATWG DOM
/// "replace data" algorithm (§4.10), which operates on UTF-16 code units.
pub fn utf16_to_byte_offset(s: &str, utf16_offset: usize) -> Option<usize> {
    let mut utf16_pos = 0;
    for (byte_pos, ch) in s.char_indices() {
        if utf16_pos == utf16_offset {
            return Some(byte_pos);
        }
        utf16_pos += ch.len_utf16();
    }
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let new = require_string_arg(args, 0)?;
        // setData(new) == replaceData(0, length, new) (WHATWG DOM §4.10).
        let old_data = get_char_data(this, dom)?;
        if let Some(record) = apply_replace_data(dom, this, 0, utf16_len(&old_data), &new, old_data)
        {
            session.push_notify_record(record);
        }
        Ok(JsValue::Undefined)
    }
}

/// `characterData.length` getter.
///
/// Returns the number of UTF-16 code units per WHATWG DOM §4.10
/// (`#dom-characterdata-length`).
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
        // WHATWG DOM §4.10: CharacterData.length returns the number of
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
        let end = offset.saturating_add(count).min(len);
        // UTF-16 slicing through a surrogate pair degrades to U+FFFD per
        // `splice_utf16` doc — spec-valid and matches the lossy-not-panic
        // test contract.
        let units: Vec<u16> = data.encode_utf16().collect();
        let s = String::from_utf16_lossy(&units[offset..end]);
        Ok(JsValue::String(s))
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let append_str = require_string_arg(args, 0)?;
        // `appendData(s)` == `replaceData(length, 0, s)` (WHATWG DOM §4.10).
        let old_data = get_char_data(this, dom)?;
        let len = utf16_len(&old_data);
        if let Some(record) = apply_replace_data(dom, this, len, 0, &append_str, old_data) {
            session.push_notify_record(record);
        }
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let insert_str = require_string_arg(args, 1)?;
        let old_data = get_char_data(this, dom)?;
        if offset > utf16_len(&old_data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        // `insertData(offset, s)` == `replaceData(offset, 0, s)`.
        if let Some(record) = apply_replace_data(dom, this, offset, 0, &insert_str, old_data) {
            session.push_notify_record(record);
        }
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let count = require_usize_arg(args, 1)?;
        let old_data = get_char_data(this, dom)?;
        if offset > utf16_len(&old_data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        // `deleteData(offset, count)` == `replaceData(offset, count, "")`.
        if let Some(record) = apply_replace_data(dom, this, offset, count, "", old_data) {
            session.push_notify_record(record);
        }
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let offset = require_usize_arg(args, 0)?;
        let count = require_usize_arg(args, 1)?;
        let replace_str = require_string_arg(args, 2)?;
        let old_data = get_char_data(this, dom)?;
        if offset > utf16_len(&old_data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        if let Some(record) = apply_replace_data(dom, this, offset, count, &replace_str, old_data) {
            session.push_notify_record(record);
        }
        Ok(JsValue::Undefined)
    }
}

/// `text.splitText(offset)` — splits a Text node at the given offset.
///
/// Delegates to the engine-independent [`split_text_at_offset`] which
/// owns the canonical WHATWG DOM §4.10 "split a Text node" algorithm:
/// `insert new_node → fire_split_text → set_text_data(head)`.
/// Range live-tracking via the [`elidex_ecs::MutationEvent::SplitText`]
/// event (consumed by [`crate::LiveRangeBridge`] through
/// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`) falls out of the ordering —
/// boundaries on the original node at off > split_offset migrate to
/// (new_node, off - offset) BEFORE the head-truncate would clamp them
/// down.
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
        // Verify this is a text node (mirrors the legacy NodeKind brand
        // check; engine-indep `split_text_at_offset` does its own check
        // via `node_kind_inferred`, but matching this error variant
        // keeps the DomApiHandler error shape stable for boa).
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
        let new_node = split_text_at_offset(this, offset, dom).map_err(|e| match e {
            SplitTextError::NotTextNode => DomApiError {
                kind: DomApiErrorKind::InvalidStateError,
                message: "splitText: not a Text node".into(),
            },
            SplitTextError::MissingTextContent => DomApiError {
                kind: DomApiErrorKind::InvalidStateError,
                message: "splitText: missing TextContent payload".into(),
            },
            SplitTextError::OffsetOutOfBounds { offset, len } => {
                index_size_error(&format!("offset {offset} exceeds data length {len}"))
            }
            SplitTextError::InsertFailed | SplitTextError::InternalInvariant => DomApiError {
                kind: DomApiErrorKind::InvalidStateError,
                message: "splitText: internal invariant violation".into(),
            },
        })?;
        let obj_ref = session.get_or_create_wrapper(new_node, ComponentKind::TextNode);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}
