//! `CharacterData.prototype` intrinsic (WHATWG DOM §4.10).
//!
//! Sits between `Node.prototype` and Text / Comment wrappers:
//!
//! ```text
//! comment wrapper
//!   → CharacterData.prototype   (this intrinsic)
//!     → Node.prototype
//!       → EventTarget.prototype
//!         → Object.prototype
//!
//! text wrapper
//!   → Text.prototype            (`vm/host/text_proto.rs`)
//!     → CharacterData.prototype (this intrinsic)
//!       → Node.prototype
//!         → EventTarget.prototype
//!           → Object.prototype
//! ```
//!
//! Implemented members:
//!
//! - Accessors: `data` (read/write), `length` (read-only, UTF-16
//!   code unit count).
//! - Methods:   `appendData`, `insertData`, `deleteData`,
//!   `replaceData`, `substringData`.
//!
//! ## UTF-16 / WTF-16 caveat
//!
//! WHATWG `data` / offsets are defined in **UTF-16 code units**.  JS
//! strings inside the VM are Rust `String`s (UTF-8) — the methods
//! below round-trip via `encode_utf16().collect::<Vec<u16>>()` so
//! surrogate pairs are honoured.  The current round-trip uses
//! [`String::from_utf16_lossy`] on write-back, which converts *lone*
//! surrogates to `U+FFFD` — a spec deviation that needs a WTF-16
//! text buffer in ECS to fix properly.  `debug_assert!` panics in
//! debug builds when a lone surrogate is produced so regressions do
//! not slip in silently.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::event_target::entity_from_this;

use elidex_ecs::{CommentData, Entity, NodeKind, TextContent};

impl VmInner {
    /// Allocate `CharacterData.prototype` whose parent is
    /// `Node.prototype`.  Must run after `register_node_prototype`.
    pub(in crate::vm) fn register_character_data_prototype(&mut self) {
        let node_proto = self
            .node_prototype
            .expect("register_character_data_prototype called before register_node_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(node_proto),
            extensible: true,
        });
        self.character_data_prototype = Some(proto_id);
        self.install_character_data_accessors(proto_id);
        self.install_character_data_methods(proto_id);
        // ChildNode mixin (WHATWG §5.2.2) — `before` / `after` /
        // `replaceWith` / `remove` are installed identically on
        // `Element.prototype`.
        self.install_child_node_mixin(proto_id);
    }

    fn install_character_data_accessors(&mut self, proto_id: ObjectId) {
        // `data` (RW).
        let data_sid = self.well_known.data;
        let getter = self.create_native_function("get data", native_char_data_get_data);
        let setter = self.create_native_function("set data", native_char_data_set_data);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(data_sid),
            PropertyValue::Accessor {
                getter: Some(getter),
                setter: Some(setter),
            },
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `length` (RO) — UTF-16 code unit count.
        let length_sid = self.well_known.length;
        let length_getter = self.create_native_function("get length", native_char_data_get_length);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(length_sid),
            PropertyValue::Accessor {
                getter: Some(length_getter),
                setter: None,
            },
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_character_data_methods(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (
                self.well_known.append_data,
                native_char_data_append_data as NativeFn,
            ),
            (self.well_known.insert_data, native_char_data_insert_data),
            (self.well_known.delete_data, native_char_data_delete_data),
            (self.well_known.replace_data, native_char_data_replace_data),
            (
                self.well_known.substring_data,
                native_char_data_substring_data,
            ),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                shape::PropertyAttrs::METHOD,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the string data on `entity` — `TextContent` for Text nodes,
/// `CommentData` for Comment nodes, `None` otherwise.
pub(super) fn char_data_get(ctx: &mut NativeContext<'_>, entity: Entity) -> Option<String> {
    let dom = ctx.host().dom();
    if let Ok(text) = dom.world().get::<&TextContent>(entity) {
        return Some(text.0.clone());
    }
    if let Ok(c) = dom.world().get::<&CommentData>(entity) {
        return Some(c.0.clone());
    }
    None
}

/// Overwrite the string data on `entity` based on its `NodeKind`.
/// Returns `false` if the entity is neither a Text nor a Comment (the
/// CharacterData methods are non-meaningful on other kinds).
pub(super) fn char_data_set(ctx: &mut NativeContext<'_>, entity: Entity, data: String) -> bool {
    let dom = ctx.host().dom();
    match dom.node_kind(entity) {
        Some(NodeKind::Text) => {
            if let Ok(mut text) = dom.world_mut().get::<&mut TextContent>(entity) {
                text.0 = data;
                return true;
            }
            false
        }
        Some(NodeKind::Comment) => {
            if let Ok(mut c) = dom.world_mut().get::<&mut CommentData>(entity) {
                c.0 = data;
                return true;
            }
            false
        }
        _ => false,
    }
}

/// Edit a UTF-8 string via a UTF-16 offset/count window, producing the
/// new contents.
///
/// `offset` and `count` are UTF-16 code unit positions (WHATWG spec).
/// Returns `Err(RangeError)` when `offset > len`.  `count` is clamped
/// to the distance from `offset` to the end, matching WHATWG §4.10.1
/// steps 1-2 for every `*Data` method.
///
/// `replacement` — `None` for deleteData (remove only); `Some(s)` for
/// replaceData / insertData (insert); appendData can use offset=len,
/// count=0, replacement=Some(data).
fn edit_data_utf16(
    original: &str,
    offset: usize,
    count: usize,
    replacement: Option<&str>,
    method: &str,
) -> Result<String, VmError> {
    let units: Vec<u16> = original.encode_utf16().collect();
    let len = units.len();
    if offset > len {
        return Err(VmError::range_error(format!(
            "Failed to execute '{method}' on 'CharacterData': \
             offset {offset} exceeds data length {len}."
        )));
    }
    let end = offset.saturating_add(count).min(len);
    let mut out: Vec<u16> = Vec::with_capacity(len + replacement.map_or(0, str::len));
    out.extend_from_slice(&units[..offset]);
    if let Some(rep) = replacement {
        out.extend(rep.encode_utf16());
    }
    out.extend_from_slice(&units[end..]);
    // WTF-16 → UTF-8 round trip: lone surrogates (unpaired high/low
    // units) corrupt to U+FFFD under `from_utf16_lossy`.  A
    // spec-correct fix needs a WTF-16 ECS text buffer; panic in
    // debug so regressions surface early, lossy in release so we do
    // not crash a page on pathological input.
    let has_lone_surrogate =
        out.iter().any(|&u| (0xD800..=0xDFFF).contains(&u)) && String::from_utf16(&out).is_err();
    debug_assert!(
        !has_lone_surrogate,
        "CharacterData edit produced a lone UTF-16 surrogate — spec-correct handling \
         requires a WTF-16 text buffer."
    );
    Ok(String::from_utf16_lossy(&out))
}

// ---------------------------------------------------------------------------
// Natives: accessors
// ---------------------------------------------------------------------------

fn native_char_data_get_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let s = char_data_get(ctx, entity).unwrap_or_default();
    if s.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

fn native_char_data_set_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let data: String = match arg {
        JsValue::Null => String::new(),
        other => {
            let sid = super::super::coerce::to_string(ctx.vm, other)?;
            ctx.vm.strings.get_utf8(sid)
        }
    };
    char_data_set(ctx, entity, data);
    Ok(JsValue::Undefined)
}

fn native_char_data_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(0.0));
    };
    let s = char_data_get(ctx, entity).unwrap_or_default();
    // WHATWG §4.10: length is UTF-16 code unit count.  `chars().count()`
    // would undercount surrogate pairs — must use `encode_utf16`.
    #[allow(clippy::cast_precision_loss)]
    let len = s.encode_utf16().count() as f64;
    Ok(JsValue::Number(len))
}

// ---------------------------------------------------------------------------
// Natives: methods
// ---------------------------------------------------------------------------

/// Coerce the arg at `idx` to a u32 offset, rejecting negatives with
/// RangeError.  WebIDL `unsigned long` conversion.
fn coerce_offset(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    idx: usize,
    label: &str,
    method: &str,
) -> Result<usize, VmError> {
    let arg = args.get(idx).copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_number(ctx.vm, arg)?;
    if !n.is_finite() || n < 0.0 {
        return Err(VmError::range_error(format!(
            "Failed to execute '{method}' on 'CharacterData': \
             {label} must be a non-negative integer."
        )));
    }
    // Floor matches ToUint32 semantics for non-integer inputs.
    Ok(n.floor() as usize)
}

fn coerce_data_arg(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<String, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    Ok(ctx.vm.strings.get_utf8(sid))
}

fn native_char_data_append_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let append = coerce_data_arg(ctx, args)?;
    let mut current = char_data_get(ctx, entity).unwrap_or_default();
    current.push_str(&append);
    char_data_set(ctx, entity, current);
    Ok(JsValue::Undefined)
}

fn native_char_data_insert_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let offset = coerce_offset(ctx, args, 0, "offset", "insertData")?;
    let data = {
        let arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
        let sid = super::super::coerce::to_string(ctx.vm, arg)?;
        ctx.vm.strings.get_utf8(sid)
    };
    let current = char_data_get(ctx, entity).unwrap_or_default();
    let new = edit_data_utf16(&current, offset, 0, Some(&data), "insertData")?;
    char_data_set(ctx, entity, new);
    Ok(JsValue::Undefined)
}

fn native_char_data_delete_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let offset = coerce_offset(ctx, args, 0, "offset", "deleteData")?;
    let count = coerce_offset(ctx, args, 1, "count", "deleteData")?;
    let current = char_data_get(ctx, entity).unwrap_or_default();
    let new = edit_data_utf16(&current, offset, count, None, "deleteData")?;
    char_data_set(ctx, entity, new);
    Ok(JsValue::Undefined)
}

fn native_char_data_replace_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let offset = coerce_offset(ctx, args, 0, "offset", "replaceData")?;
    let count = coerce_offset(ctx, args, 1, "count", "replaceData")?;
    let data = {
        let arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
        let sid = super::super::coerce::to_string(ctx.vm, arg)?;
        ctx.vm.strings.get_utf8(sid)
    };
    let current = char_data_get(ctx, entity).unwrap_or_default();
    let new = edit_data_utf16(&current, offset, count, Some(&data), "replaceData")?;
    char_data_set(ctx, entity, new);
    Ok(JsValue::Undefined)
}

fn native_char_data_substring_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let offset = coerce_offset(ctx, args, 0, "offset", "substringData")?;
    let count = coerce_offset(ctx, args, 1, "count", "substringData")?;
    let current = char_data_get(ctx, entity).unwrap_or_default();
    let units: Vec<u16> = current.encode_utf16().collect();
    let len = units.len();
    if offset > len {
        return Err(VmError::range_error(format!(
            "Failed to execute 'substringData' on 'CharacterData': \
             offset {offset} exceeds data length {len}."
        )));
    }
    let end = offset.saturating_add(count).min(len);
    let slice = &units[offset..end];
    let has_lone_surrogate =
        slice.iter().any(|&u| (0xD800..=0xDFFF).contains(&u)) && String::from_utf16(slice).is_err();
    debug_assert!(
        !has_lone_surrogate,
        "CharacterData.substringData produced a lone UTF-16 surrogate — \
         spec-correct handling requires a WTF-16 text buffer."
    );
    let s = String::from_utf16_lossy(slice);
    if s.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}
