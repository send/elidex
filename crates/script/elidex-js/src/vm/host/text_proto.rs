//! `Text.prototype` intrinsic (WHATWG DOM §4.11).
//!
//! Intermediate prototype layer for Text wrappers:
//!
//! ```text
//! text wrapper
//!   → Text.prototype          (this intrinsic)
//!     → CharacterData.prototype
//!       → Node.prototype
//!         → EventTarget.prototype
//!           → Object.prototype
//! ```
//!
//! Holds Text-only members that WHATWG does not define on
//! `CharacterData` (which Text shares with Comment / ProcessingInstruction):
//!
//! - `splitText(offset)` — splits this Text at `offset` and returns
//!   the new Text node covering the trailing portion.
//!
//! `wholeText` (contiguous sibling Text merge) and `assignedSlot`
//! (slot-distribution tracking — arrives with Custom Elements) are
//! not yet implemented.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::VmInner;
use super::event_target::entity_from_this;

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Allocate `Text.prototype` with `CharacterData.prototype` as
    /// its parent.  Must run after `register_character_data_prototype`.
    pub(in crate::vm) fn register_text_prototype(&mut self) {
        let parent = self
            .character_data_prototype
            .expect("register_text_prototype called before register_character_data_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.text_prototype = Some(proto_id);

        let name_sid = self.well_known.split_text;
        let name = self.strings.get_utf8(name_sid);
        let fn_id = self.create_native_function(&name, native_text_split_text);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(name_sid),
            PropertyValue::Data(JsValue::Object(fn_id)),
            shape::PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

/// `Text.prototype.splitText(offset)` — WHATWG §4.11.
///
/// Splits the Text node at the given UTF-16 code-unit offset.  The
/// original node retains the substring `[0..offset]`; a new Text is
/// allocated for `[offset..]` and inserted as the next sibling of
/// `this`.  Returns the new Text wrapper.
///
/// Errors:
/// - `RangeError` when `offset > length`.
/// - `TypeError` when the receiver is not a Text node.
fn native_text_split_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Receiver must be a Text node.
    if ctx.host().dom().node_kind(entity) != Some(NodeKind::Text) {
        return Err(VmError::type_error(
            "Failed to execute 'splitText' on 'Text': this is not a Text node.",
        ));
    }
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let offset_num = super::super::coerce::to_number(ctx.vm, offset_arg)?;
    if !offset_num.is_finite() || offset_num < 0.0 {
        return Err(VmError::range_error(
            "Failed to execute 'splitText' on 'Text': offset must be a non-negative integer.",
        ));
    }
    let offset = offset_num.floor() as usize;

    let dom = ctx.host().dom();
    let current = dom
        .world()
        .get::<&elidex_ecs::TextContent>(entity)
        .map(|t| t.0.clone())
        .unwrap_or_default();
    let units: Vec<u16> = current.encode_utf16().collect();
    let len = units.len();
    if offset > len {
        return Err(VmError::range_error(format!(
            "Failed to execute 'splitText' on 'Text': \
             offset {offset} exceeds data length {len}."
        )));
    }
    let (left_units, right_units) = units.split_at(offset);
    // WHATWG §4.11 splitText offsets are UTF-16 code units, so the
    // split can land between a surrogate pair per spec.  Our Rust
    // `String` storage cannot represent lone surrogates; the lossy
    // coercion here maps them to U+FFFD — a known Phase 2 limitation
    // tied to the CharacterData WTF-16 buffer work (see
    // `character_data_proto` module doc).
    let left = String::from_utf16_lossy(left_units);
    let right = String::from_utf16_lossy(right_units);

    // Allocate the trailing Text node and thread it into place
    // BEFORE mutating the original, so a rejected insertion leaves
    // the tree unchanged and the original node's data intact.
    let new_entity = dom.create_text(right);
    if let Some(parent) = dom.get_parent(entity) {
        let inserted = if let Some(next) = dom.get_next_sibling(entity) {
            dom.insert_before(parent, new_entity, next)
        } else {
            dom.append_child(parent, new_entity)
        };
        if !inserted {
            let _ = dom.destroy_entity(new_entity);
            return Err(VmError::type_error(
                "Failed to execute 'splitText' on 'Text': \
                 could not insert the trailing Text node.",
            ));
        }
    }
    if let Ok(mut text) = dom.world_mut().get::<&mut elidex_ecs::TextContent>(entity) {
        text.0 = left;
    }
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}
