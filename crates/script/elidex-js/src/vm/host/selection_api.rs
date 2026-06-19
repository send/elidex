//! Shared HTML Selection API (HTML §4.10.20) bodies for
//! `<input>` and `<textarea>`.
//!
//! ## Offset units (HTML §4.10.20)
//!
//! Every offset crossing this IDL surface is measured in **UTF-16
//! code units of the relevant value**.  `FormControlState` stores
//! selection offsets internally as **byte** offsets (natural for
//! Rust `String` slicing), so this module is the single boundary
//! where the two representations are reconciled: getters convert
//! byte→UTF-16 on read, setters convert UTF-16→byte before write,
//! via [`byte_offset_to_utf16`] / [`utf16_to_byte_offset`].  No
//! other reader of the selection offsets escapes to JS (the shell /
//! render consumers stay byte-internal), so the editing logic in
//! `elidex_form` remains byte-internal and unchanged.
//!
//! ### Known limitations of the byte-internal model (deferred)
//!
//! Two §4.10.20 fidelity gaps are intrinsic to the byte-internal
//! storage this slot deliberately keeps, and each needs a separate
//! representation change rather than a boundary tweak:
//!
//! - **Offsets that split a surrogate pair** are not representable as
//!   a UTF-8 byte offset, so [`utf16_to_byte_offset`] snaps them down
//!   to the character start (deterministic, documented).  Exact
//!   preservation of a mid-surrogate code-unit offset would require
//!   UTF-16-internal selection storage → slot
//!   `#11-selection-mid-surrogate-fidelity`.
//! - For `<textarea>`, §4.10.20's "relevant value" is the **API
//!   value** (CR / CRLF normalized to LF), but elidex stores and
//!   reports the raw value uniformly (the `value` getter, `textLength`
//!   and these offsets all operate on the raw value).  Conversion here
//!   uses `state.value()` so it stays consistent with the stored byte
//!   offsets; implementing textarea API-value normalization (which
//!   would make this conversion spec-correct without code change here)
//!   → slot `#11-textarea-api-value-newline-normalization`.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", every body in this module is
//! pure marshalling glue: brand-checked entity → `FormControlState`
//! read/write through public `elidex_form` accessors.  No selection
//! algorithm lives here — clamping / range-replacement / select-all
//! all delegate to `elidex_form::FormControlState` (which itself
//! reaches `elidex_form::selection`).
//!
//! ## Sharing pattern
//!
//! Both `html_input_proto.rs` and `html_textarea_proto.rs` install
//! the same six accessor pairs (selectionStart / selectionEnd /
//! selectionDirection) and three methods (select / setSelectionRange
//! / setRangeText).  Each proto's per-tag native wrapper performs:
//!
//! 1. its own brand check (`require_input_receiver` /
//!    `require_textarea_receiver`) — interface-specific so error
//!    messages report the correct `HTMLInputElement` /
//!    `HTMLTextAreaElement`,
//! 2. [`require_text_control`] to reject types whose
//!    `FormControlKind::supports_selection` returns false, and
//! 3. forwards to a shared body in this module that performs the
//!    `FormControlState` read / write.
//!
//! The error-message templates for `require_text_control` differ
//! between input ("input element") and textarea ("element"), so the
//! interface name + element-label parameters are passed in.

#![cfg(feature = "engine")]
// Selection setters clamp negatives via `to_uint32` (WebIDL `unsigned
// long`), so the `as usize` casts are value-preserving.  Module-wide
// allow matches `html_input_proto.rs` / `html_textarea_proto.rs`.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
// `map(...).unwrap_or(default)` on `Result<&FormControlState>` reads
// the entity component straightforwardly; `is_ok_and` / `map_or`
// rewrites require closure arguments by value, which doesn't compose
// with the borrow checker for these shared-borrow patterns.
#![allow(clippy::map_unwrap_or)]

use elidex_ecs::Entity;
use elidex_form::util::{byte_offset_to_utf16, utf16_to_byte_offset};
use elidex_form::{FormControlState, SelectionDirection};

use super::super::value::{JsValue, NativeContext, VmError};

/// Verify that `entity`'s [`FormControlState::kind`] supports
/// selection per [`FormControlKind::supports_selection`].  Returns an
/// `InvalidStateError` DOMException with an interface-specific
/// message otherwise (matches HTML §4.10.20's "throw an
/// InvalidStateError" branches).
///
/// Used by both [`html_input_selection`](super::html_input_selection)
/// and [`html_textarea_proto`](super::html_textarea_proto) wrappers.
/// `interface` is the WebIDL interface name in the error message
/// (`"HTMLInputElement"` / `"HTMLTextAreaElement"`); `elem_label` is
/// the descriptor used in the message body (`"input element"` for
/// `<input>`, `"element"` for `<textarea>`).
pub(super) fn require_text_control(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    method: &str,
    interface: &str,
    elem_label: &str,
) -> Result<(), VmError> {
    let dom = ctx.host().dom();
    let supports = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.kind.supports_selection())
        .unwrap_or(false);
    if !supports {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_state_error,
            format!(
                "Failed to execute '{method}' on '{interface}': \
                 The {elem_label}'s type does not support selection"
            ),
        ));
    }
    Ok(())
}

/// Whether `entity`'s control has selectable text for `select()` to act
/// on (HTML "select() method", step 1 — `select()` is a no-op for a
/// control with no selectable text).  Unlike [`require_text_control`] this
/// is **not** an error for other kinds — `select()` simply does nothing —
/// so it returns a `bool` for the caller to gate the selection on rather
/// than raising an `InvalidStateError`.
pub(super) fn has_selectable_text(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    ctx.host()
        .dom()
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.kind.has_selectable_text())
        .unwrap_or(false)
}

/// Whether `entity`'s control supports the text-selection IDL attributes
/// ([`FormControlKind::supports_selection`]).  The non-throwing predicate
/// for the selection *getters*, which return null (rather than the
/// [`require_text_control`] `InvalidStateError`) when the attribute does not
/// apply — HTML §4.10.20 throws only from the setters / `setSelectionRange()`
/// / `setRangeText()`, never the `selectionStart`/`End`/`Direction` getters.
pub(super) fn supports_selection(ctx: &mut NativeContext<'_>, entity: Entity) -> bool {
    ctx.host()
        .dom()
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.kind.supports_selection())
        .unwrap_or(false)
}

// -------------------------------------------------------------------------
// selectionStart / selectionEnd
// -------------------------------------------------------------------------

pub(super) fn get_selection_start(ctx: &mut NativeContext<'_>, entity: Entity) -> JsValue {
    let dom = ctx.host().dom();
    // HTML §4.10.20: return the offset in UTF-16 code units of the
    // relevant value (internal offset is a byte offset → convert).
    let pos = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| byte_offset_to_utf16(s.value(), s.selection_start()))
        .unwrap_or(0);
    JsValue::Number(f64::from(u32::try_from(pos).unwrap_or(u32::MAX)))
}

pub(super) fn set_selection_start(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    args: &[JsValue],
) -> Result<(), VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, val)? as usize;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        // `n` is a UTF-16 code-unit offset (HTML §4.10.20); convert to
        // the internal byte offset.  `utf16_to_byte_offset` self-clamps
        // to `value.len()` for over-length input (= spec "set it to the
        // length"), so no separate UTF-16-length clamp is needed.
        let byte = utf16_to_byte_offset(state.value(), n);
        state.set_selection_start(byte);
    }
    Ok(())
}

pub(super) fn get_selection_end(ctx: &mut NativeContext<'_>, entity: Entity) -> JsValue {
    let dom = ctx.host().dom();
    // HTML §4.10.20: return the offset in UTF-16 code units of the
    // relevant value (internal offset is a byte offset → convert).
    let pos = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| byte_offset_to_utf16(s.value(), s.selection_end()))
        .unwrap_or(0);
    JsValue::Number(f64::from(u32::try_from(pos).unwrap_or(u32::MAX)))
}

pub(super) fn set_selection_end(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    args: &[JsValue],
) -> Result<(), VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, val)? as usize;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        // `n` is a UTF-16 code-unit offset (HTML §4.10.20) → byte.
        let byte = utf16_to_byte_offset(state.value(), n);
        state.set_selection_end(byte);
    }
    Ok(())
}

// -------------------------------------------------------------------------
// selectionDirection
// -------------------------------------------------------------------------

pub(super) fn get_selection_direction(ctx: &mut NativeContext<'_>, entity: Entity) -> JsValue {
    let dom = ctx.host().dom();
    let dir = dom
        .world()
        .get::<&FormControlState>(entity)
        .map(|s| s.selection_direction)
        .unwrap_or(SelectionDirection::None);
    let s = match dir {
        SelectionDirection::Forward => "forward",
        SelectionDirection::Backward => "backward",
        SelectionDirection::None => "none",
    };
    let sid = ctx.vm.strings.intern(s);
    JsValue::String(sid)
}

pub(super) fn set_selection_direction(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    args: &[JsValue],
) -> Result<(), VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    let dir = parse_selection_direction(s.as_str());
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        state.selection_direction = dir;
    }
    Ok(())
}

// -------------------------------------------------------------------------
// select() / setSelectionRange / setRangeText
// -------------------------------------------------------------------------

pub(super) fn select_all(ctx: &mut NativeContext<'_>, entity: Entity) {
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        elidex_form::select_all(&mut state);
    }
}

pub(super) fn set_selection_range(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    args: &[JsValue],
) -> Result<(), VmError> {
    let start_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let end_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let dir_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    // setSelectionRange start / end are WebIDL `unsigned long`
    // (HTML §4.10.20) — coerce via ToUint32 so negative inputs wrap to
    // 2³² + n rather than clamping to 0.  The coerced values are
    // UTF-16 code-unit offsets; `utf16_to_byte_offset` converts to the
    // internal byte offset and self-clamps over-length offsets to
    // `value.len()` (= spec "treated as pointing at the end").
    let start = super::super::coerce::to_uint32(ctx.vm, start_arg)? as usize;
    let end = super::super::coerce::to_uint32(ctx.vm, end_arg)? as usize;
    let dir = if matches!(dir_arg, JsValue::Undefined) {
        SelectionDirection::None
    } else {
        let sid = super::super::coerce::to_string(ctx.vm, dir_arg)?;
        let s = ctx.vm.strings.get_utf8(sid);
        parse_selection_direction(s.as_str())
    };
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        let byte_start = utf16_to_byte_offset(state.value(), start);
        let byte_end = utf16_to_byte_offset(state.value(), end);
        state.set_selection(byte_start, byte_end);
        state.selection_direction = dir;
    }
    Ok(())
}

pub(super) fn set_range_text(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    args: &[JsValue],
) -> Result<(), VmError> {
    let replacement_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, replacement_arg)?;
    let replacement = ctx.vm.strings.get_utf8(sid);
    // Optional start / end via WebIDL `unsigned long` coercion
    // (HTML §4.10.20) — `to_uint32` runs ToNumber first so strings
    // ("2") and booleans (true → 1 / false → 0) coerce.  BigInt inputs
    // throw TypeError (ES `ToNumber` on BigInt is a hard error — see
    // `coerce::to_number`).  `Undefined` / missing → use the current
    // selection bounds.  The coerced values are UTF-16 code-unit
    // offsets against the PRE-replacement value (converted below).
    let coerced_start = coerce_optional_clamp(ctx, args.get(1).copied())?;
    let coerced_end = coerce_optional_clamp(ctx, args.get(2).copied())?;
    let dom = ctx.host().dom();
    if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(entity) {
        let (cur_s, cur_e) = state.safe_selection_range();
        // Provided start / end are UTF-16 code units against the
        // pre-replacement value → convert to byte offsets here (where
        // `state.value()` is still the old value).  Omitted args fall
        // back to the current selection bounds, which are ALREADY byte
        // offsets — these must NOT be converted (mixing a converted
        // arg with an unconverted byte fallback is the hazard the
        // per-arg `match` prevents).
        let start = match coerced_start {
            Some(u) => utf16_to_byte_offset(state.value(), u),
            None => cur_s,
        };
        let end = match coerced_end {
            Some(u) => utf16_to_byte_offset(state.value(), u),
            None => cur_e,
        };
        state.set_selection(start, end);
        // FormControlState exposes `replace_selection` directly,
        // matching `elidex_form::selection::replace_selection` but
        // accessible without the private-module re-export.
        state.replace_selection(replacement.as_str());
    }
    Ok(())
}

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

fn parse_selection_direction(s: &str) -> SelectionDirection {
    match s {
        "forward" => SelectionDirection::Forward,
        "backward" => SelectionDirection::Backward,
        _ => SelectionDirection::None,
    }
}

/// Coerce an optional `start` / `end` argument from `setRangeText`
/// into a `usize` **UTF-16 code-unit offset**.  `Undefined` / missing
/// yields `None` (caller substitutes the current selection bound,
/// which is already a byte offset); other values flow through
/// `to_uint32` (full WebIDL `unsigned long` coercion: ToNumber →
/// trunc → mod-2³² as an unsigned integer).  The returned value is
/// still in UTF-16 units — the caller converts it to a byte offset
/// against the pre-replacement value (HTML §4.10.20), at which point
/// over-length offsets self-clamp to `value.len()`.  This helper does
/// not convert because it has no access to the value string.
fn coerce_optional_clamp(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<Option<usize>, VmError> {
    match arg {
        None | Some(JsValue::Undefined) => Ok(None),
        Some(v) => {
            let n = super::super::coerce::to_uint32(ctx.vm, v)?;
            Ok(Some(n as usize))
        }
    }
}
