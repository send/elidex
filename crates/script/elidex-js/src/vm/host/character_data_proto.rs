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
//! Each native is a thin binding that runs WebIDL coercion + a
//! `NodeKind ∈ {Text, Comment}` brand check at the boundary, then
//! dispatches through [`super::dom_bridge::invoke_dom_api`] to the
//! engine-independent handler in `elidex-dom-api`. The UTF-16 splice
//! algorithm (and its surrogate-pair lossy decode contract — see
//! `splice_utf16` in `crates/dom/elidex-dom-api/src/char_data/`) lives
//! exclusively on the handler side per the CLAUDE.md Layering mandate.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, VmInner};
use super::event_target::entity_from_this;

use elidex_ecs::{Entity, NodeKind};

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
        self.install_accessor_pair(
            proto_id,
            self.well_known.data,
            native_char_data_get_data,
            Some(native_char_data_set_data),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `length` (RO) — UTF-16 code unit count.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_char_data_get_length,
            None,
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
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

// ---------------------------------------------------------------------------
// VM-boundary helpers
// ---------------------------------------------------------------------------

/// TypeError for CharacterData methods invoked on a non-Text /
/// non-Comment receiver.  Matches the WebIDL behaviour (the method
/// is exposed on `CharacterData.prototype`, so a `Function.call`-style
/// reroute to another receiver is the only way to trip this).
fn wrong_receiver_error(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'CharacterData': \
         this is not a Text or Comment node."
    ))
}

/// Brand check: the receiver entity must be a Text or Comment node.
///
/// Performed at the VM boundary so a wrong-receiver call (e.g.
/// `text.appendData.call(elementWrapper, …)`) raises **TypeError**
/// — the WebIDL-mandated shape — before the handler dispatches and
/// the algorithm-side `InvalidStateError` would bubble up as a
/// DOMException instead.  Uses `node_kind_inferred` so legacy
/// payload-only entities (no explicit `NodeKind` component) are still
/// accepted, mirroring the prior `char_data_get` / `char_data_set`
/// component-presence check.
fn ensure_character_data(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    method: &str,
) -> Result<(), VmError> {
    match ctx.host().dom().node_kind_inferred(entity) {
        Some(NodeKind::Text | NodeKind::Comment) => Ok(()),
        _ => Err(wrong_receiver_error(method)),
    }
}

/// Coerce the arg at `idx` via WebIDL `unsigned long` (ToUint32,
/// ES2020 §7.1.7) — the spec-mandated conversion for CharacterData
/// offsets.  Unlike a naive `to_number + floor`, this wraps
/// out-of-range / negative inputs mod 2^32 before range-checking
/// against the data length, matching browser behaviour.  Returns the
/// uint32 packaged as a `JsValue::Number` so the caller can hand it
/// straight to the handler dispatch (the dom-api `require_usize_arg`
/// extracts via `JsValue::Number(n) as usize`).
fn coerce_offset_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    idx: usize,
) -> Result<JsValue, VmError> {
    let arg = args.get(idx).copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, arg)?;
    Ok(JsValue::Number(f64::from(n)))
}

/// Coerce the arg at `idx` to a string (ToString) and rewrap as
/// `JsValue::String(StringId)` so the bridge's `prepare_arg` can
/// materialize it as a primitive `String` for the handler.
fn coerce_string_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    idx: usize,
) -> Result<JsValue, VmError> {
    let arg = args.get(idx).copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    Ok(JsValue::String(sid))
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
    ensure_character_data(ctx, entity, "data")?;
    super::dom_bridge::invoke_dom_api(ctx, "data.get", entity, &[])
}

fn native_char_data_set_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    ensure_character_data(ctx, entity, "data")?;
    // WebIDL `CharacterData.data` is a non-nullable `DOMString`: every
    // value (including `null`) goes through `ToString`, so `null`
    // becomes the literal string `"null"`.  This differs from
    // `Node.nodeValue` / `textContent`, whose nullable setters treat
    // `null` as the empty string.
    let coerced = coerce_string_arg(ctx, args, 0)?;
    super::dom_bridge::invoke_dom_api(ctx, "data.set", entity, &[coerced])
}

fn native_char_data_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(0.0));
    };
    ensure_character_data(ctx, entity, "length")?;
    super::dom_bridge::invoke_dom_api(ctx, "length.get", entity, &[])
}

// ---------------------------------------------------------------------------
// Natives: methods
// ---------------------------------------------------------------------------

fn native_char_data_append_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    ensure_character_data(ctx, entity, "appendData")?;
    let data_arg = coerce_string_arg(ctx, args, 0)?;
    super::dom_bridge::invoke_dom_api(ctx, "appendData", entity, &[data_arg])
}

fn native_char_data_insert_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    ensure_character_data(ctx, entity, "insertData")?;
    let offset_arg = coerce_offset_arg(ctx, args, 0)?;
    let data_arg = coerce_string_arg(ctx, args, 1)?;
    super::dom_bridge::invoke_dom_api(ctx, "insertData", entity, &[offset_arg, data_arg])
}

fn native_char_data_delete_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    ensure_character_data(ctx, entity, "deleteData")?;
    let offset_arg = coerce_offset_arg(ctx, args, 0)?;
    let count_arg = coerce_offset_arg(ctx, args, 1)?;
    super::dom_bridge::invoke_dom_api(ctx, "deleteData", entity, &[offset_arg, count_arg])
}

fn native_char_data_replace_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    ensure_character_data(ctx, entity, "replaceData")?;
    let offset_arg = coerce_offset_arg(ctx, args, 0)?;
    let count_arg = coerce_offset_arg(ctx, args, 1)?;
    let data_arg = coerce_string_arg(ctx, args, 2)?;
    super::dom_bridge::invoke_dom_api(
        ctx,
        "replaceData",
        entity,
        &[offset_arg, count_arg, data_arg],
    )
}

fn native_char_data_substring_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    ensure_character_data(ctx, entity, "substringData")?;
    let offset_arg = coerce_offset_arg(ctx, args, 0)?;
    let count_arg = coerce_offset_arg(ctx, args, 1)?;
    super::dom_bridge::invoke_dom_api(ctx, "substringData", entity, &[offset_arg, count_arg])
}
