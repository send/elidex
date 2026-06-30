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
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;
use super::event_target::entity_from_this;

use elidex_dom_api::char_data::split_text::{split_text_at_offset, SplitTextError};
use elidex_ecs::NodeKind;

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

        self.install_native_method(
            proto_id,
            self.well_known.split_text,
            native_text_split_text,
            shape::PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

/// `Text.prototype.splitText(offset)` — WHATWG §4.11.
///
/// Marshalling-only wrapper around the engine-independent
/// [`elidex_dom_api::char_data::split_text::split_text_at_offset`]
/// algorithm, per the CLAUDE.md layering mandate ("VM host/ は
/// marshalling 用途 (entity 取得 / 単純 attribute read / wrapper
/// 生成) に限定"). Brand check + WebIDL `unsigned long` coercion
/// happen here; the actual splice + Range live-tracking ordering
/// (insert → fire_split_text → set_text_data) is in
/// elidex-dom-api.
///
/// ## Range live-tracking ordering (informational)
///
/// `split_text_at_offset` orchestrates a three-step Range boundary
/// dance: `insert_before(new_node)` fires
/// [`MutationEvent::Insert`](elidex_ecs::MutationEvent::Insert)
/// (parent-side `off > node_idx + 1 → +1`), then `fire_split_text`
/// carrying the pre-split `parent` + `node_index` fires
/// [`MutationEvent::SplitText`](elidex_ecs::MutationEvent::SplitText)
/// (node-side `off > offset → (new_node, off - offset)` + parent-side
/// `off == node_idx + 1 → +1` delta top-up), then
/// `set_text_data(head)` truncates the original node. The combined
/// dispatch sequence implements WHATWG §4.10 step 7 in full when the
/// standard [`elidex_dom_api::LiveRangeBridge`] consumer (composed by
/// [`crate::vm::consumer_dispatcher::ConsumerDispatcher`]) is the installed
/// dispatcher.  Engines installing a custom dispatcher that ignores
/// the parent / node_index args inherit only the `Insert` shift (lag
/// at `node_idx + 1`); such dispatchers should document the gap
/// explicitly.
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
    // Receiver must be a Text node.  Use the inferred NodeKind so
    // legacy entities (missing `NodeKind` but carrying
    // `TextContent`) are accepted — matching the
    // `HostData::prototype_kind_for` routing that placed this
    // entity on `Text.prototype` in the first place.
    if ctx.host().dom().node_kind_inferred(entity) != Some(NodeKind::Text) {
        return Err(VmError::type_error(
            "Failed to execute 'splitText' on 'Text': this is not a Text node.",
        ));
    }
    // WebIDL `unsigned long` conversion (ToUint32, ECMA-262 §7.1.8).
    // The DOM range check against `length` runs below.
    let offset_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let offset = super::super::coerce::to_uint32(ctx.vm, offset_arg)? as usize;

    let (new_entity, records) =
        split_text_at_offset(entity, offset, ctx.host().dom()).map_err(|e| match e {
            SplitTextError::NotTextNode => VmError::type_error(
                "Failed to execute 'splitText' on 'Text': this is not a Text node.",
            ),
            SplitTextError::MissingTextContent => VmError::type_error(
                "Failed to execute 'splitText' on 'Text': \
                 receiver is missing a TextContent payload.",
            ),
            SplitTextError::OffsetOutOfBounds { offset, len } => VmError::range_error(format!(
                "Failed to execute 'splitText' on 'Text': \
                 offset {offset} exceeds data length {len}."
            )),
            SplitTextError::InsertFailed => VmError::type_error(
                "Failed to execute 'splitText' on 'Text': \
                 could not insert the trailing Text node.",
            ),
            SplitTextError::InternalInvariant => VmError::type_error(
                "Failed to execute 'splitText' on 'Text': \
                 internal invariant violation (TextContent disappeared mid-operation).",
            ),
        })?;
    // §4.11 split-a-Text-node records ([childList insert?, characterData
    // truncate]). The `host().dom()` borrow above is released (the Result
    // is owned), so deliver them through the same chokepoint as the Range
    // natives.
    ctx.vm.commit_notify_records(records);
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}
