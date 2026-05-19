//! `CharacterData` interface method handlers.

use elidex_ecs::{CommentData, EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
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

/// Write character data (text or comment) to an entity.
///
/// Text / CData writes route through [`EcsDom::set_text_data`] so the
/// installed [`elidex_ecs::MutationDispatcher`] (typically
/// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`) receives the
/// [`elidex_ecs::MutationEvent::TextChange`] event and consumers
/// (e.g. [`crate::LiveRangeBridge`]) apply WHATWG DOM §5.5
/// "set/replace data steps" Range live-tracking.  Comment writes
/// update `CommentData` in place; per WHATWG §5.5 Range live-tracking
/// does not cover Comment nodes, so no event fires for that branch.
///
/// Both branches are self-contained for cache invalidation: the Text
/// path inherits the `rev_version(entity)` call inside `set_text_data`,
/// and the Comment path bumps `rev_version(entity)` explicitly. Callers
/// MUST NOT re-bump `rev_version` after this call.
pub(crate) fn set_char_data(
    entity: Entity,
    dom: &mut EcsDom,
    data: &str,
) -> Result<(), DomApiError> {
    // Try the Text/CData branch first: `set_text_data` returns `Some`
    // iff the entity has a `TextContent` component, so its `Option`
    // result doubles as the branch discriminator and saves a duplicate
    // lookup. `set_text_data` takes `&str` and reuses the existing
    // `TextContent` buffer capacity, so the Text path stays
    // single-lookup with no extra allocation.
    if dom.set_text_data(entity, data).is_some() {
        return Ok(());
    }
    let comment_present = {
        if let Ok(mut cd) = dom.world_mut().get::<&mut CommentData>(entity) {
            data.clone_into(&mut cd.0);
            true
        } else {
            false
        }
    };
    if comment_present {
        // Comment writes don't go through `set_text_data` (it's
        // Text/CData-only per the docstring), so bump the version
        // here to match the Text path's invariant.
        dom.rev_version(entity);
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
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Convert a UTF-16 code unit offset to a byte offset in a Rust (UTF-8)
/// string. Returns `None` if `utf16_offset` exceeds the string's UTF-16
/// length or lands in the middle of a surrogate pair.
///
/// Currently used by [`Range`](crate::Range) for boundary-point math
/// where a `None` result is tolerated via `.unwrap_or(s.len())`. The
/// CharacterData splice methods use `splice_utf16` (crate-private)
/// instead because they must accept mid-surrogate offsets per WHATWG §11.2.
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

/// Splice `entity`'s CharacterData per WHATWG DOM §4.10 "replace
/// data". Routes Text / CDATASection writes through the
/// [`elidex_ecs::EcsDom::replace_text_data`] chokepoint so the
/// installed [`elidex_ecs::MutationDispatcher`] (typically
/// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`) sees the middle-splice as
/// [`elidex_ecs::MutationEvent::ReplaceData`] rather than a whole-
/// string [`elidex_ecs::MutationEvent::TextChange`] — boundary-
/// adjustment math differs (§4.10 steps 8-11 vs §5.5 clamp).
///
/// Comment writes do not fire Range live-tracking per WHATWG §5.5,
/// so the Comment branch keeps the legacy "compute spliced string +
/// set_char_data" path. Caller is responsible for validating
/// `offset > utf16_len` and raising `IndexSizeError` BEFORE invoking
/// this helper (the engine-side `replace_text_data` only
/// `debug_assert!`s the bound).
pub(crate) fn splice_char_data(
    entity: Entity,
    dom: &mut EcsDom,
    offset: usize,
    count: usize,
    replacement: &str,
) -> Result<(), DomApiError> {
    if dom
        .replace_text_data(entity, offset, count, replacement)
        .is_some()
    {
        return Ok(());
    }
    // Comment fallback: read original, splice, write back via the
    // CommentData path (no Range hook fires for Comment per §5.5).
    let original = get_char_data(entity, dom)?;
    let new = splice_utf16(&original, offset, count, Some(replacement));
    set_char_data(entity, dom, &new)
}

/// Splice a UTF-16 view of `original` and return the result as a Rust
/// `String`.
///
/// **Caller contract**: `offset` MUST be ≤ the UTF-16 length of
/// `original`. This helper is **not** a spec-validating primitive —
/// the CharacterData spec (§11.2) requires `offset > length` to raise
/// `IndexSizeError`, and that check lives in every caller
/// (`InsertData` / `DeleteData` / `ReplaceData` / `SubstringData`).
/// Adding a new caller? Validate `offset` first. Debug builds enforce
/// the contract via `debug_assert!`; release builds rely on the slice
/// indexing below to panic on violation rather than silently clamp.
///
/// `count` IS clamped to `len - offset` to match the spec's silent
/// clamp ("if offset+count is greater than length, end at length").
/// `replacement` is `None` for delete, `Some` for replace / insert /
/// append.
///
/// Splitting through a surrogate pair (offset / end mid-pair) is
/// **spec-valid** — UTF-16 offsets ignore character boundaries — and
/// produces lone surrogates in the intermediate `Vec<u16>`. Rust's
/// `String` storage cannot represent lone surrogates, so the result is
/// rendered through `from_utf16_lossy` which substitutes `U+FFFD` for
/// each unpaired half. This intentionally degrades into a known-lossy
/// shape rather than panicking; matches the pre-arch-hoist VM-side
/// behaviour and the lossy-not-panic contract pinned by
/// `tests_character_data::*surrogate_pair*`.
pub(crate) fn splice_utf16(
    original: &str,
    offset: usize,
    count: usize,
    replacement: Option<&str>,
) -> String {
    let units: Vec<u16> = original.encode_utf16().collect();
    let len = units.len();
    debug_assert!(
        offset <= len,
        "splice_utf16: offset {offset} exceeds UTF-16 length {len}; caller must \
         validate via `if offset > utf16_len(&data)` before invocation"
    );
    let end = offset.saturating_add(count).min(len);
    let replacement_units = replacement.map_or(0, |r| r.encode_utf16().count());
    let mut out: Vec<u16> = Vec::with_capacity(len - (end - offset) + replacement_units);
    out.extend_from_slice(&units[..offset]);
    if let Some(rep) = replacement {
        out.extend(rep.encode_utf16());
    }
    out.extend_from_slice(&units[end..]);
    String::from_utf16_lossy(&out)
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
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let append_str = require_string_arg(args, 0)?;
        let len = utf16_len(&get_char_data(this, dom)?);
        // `appendData(s)` is the WHATWG-defined shorthand for
        // `replaceData(length, 0, s)` (§11.2 step 6 default). Route
        // through the splice chokepoint so `after_replace_data`
        // fires with the canonical `(offset=length, count=0,
        // new_data_len)` shape — Range live-tracking treats this as
        // a pure tail insertion (no boundary collapse, only
        // `off > length` boundaries shift; in practice none on a
        // CharacterData node).
        splice_char_data(this, dom, len, 0, &append_str)?;
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
        // `insertData(offset, s)` == `replaceData(offset, 0, s)`.
        splice_char_data(this, dom, offset, 0, &insert_str)?;
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
        if offset > utf16_len(&data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        // `deleteData(offset, count)` == `replaceData(offset, count, "")`.
        splice_char_data(this, dom, offset, count, "")?;
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
        if offset > utf16_len(&data) {
            return Err(index_size_error("offset exceeds data length"));
        }
        splice_char_data(this, dom, offset, count, &replace_str)?;
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
