//! D-9 modern input event family (slot
//! `#11-events-modern-input`).
//!
//! Five new constructable Event-family interfaces plus the
//! transferable data container (DataTransfer + items + ItemList) and
//! the Touch object family (Touch + TouchList + TouchEvent), grouped
//! into a directory module because the combined production LoC
//! (~1,700 LoC) exceeds the 1000-line file convention.
//!
//! WebIDL inheritance:
//!
//! ```text
//! PointerEvent      : MouseEvent
//! DragEvent         : MouseEvent
//! TouchEvent        : UIEvent
//! Touch             : (Object)
//! TouchList         : (Object — indexed exotic, no constructor)
//! DataTransfer      : (Object)
//! DataTransferItem  : (Object)
//! DataTransferItemList : (Object — indexed exotic)
//! ```
//!
//! ## Layering
//!
//! Engine-bound only — VM-side init-dict coercion + state-machine
//! transitions + identity-cache plumbing.  No DOM mutation /
//! selector walking happens here.  Drag-image element references
//! (`setDragImage(element, x, y)`) are stored as `entity_bits` only;
//! integration with the paint pipeline is deferred to slot
//! `#11-data-transfer-drag-image-paint`.
//!
//! ## Deferred: indexed-exotic dispatch
//!
//! `TouchList` / `DataTransferItemList` are WebIDL "indexed-exotic"
//! interfaces.  This module provides the state, identity caches, and
//! the `length` / `item(i)` operations, but **does not** wire indexed
//! `[[Get]]` (`list[0]`) into the property-access path.  Bracket-index
//! access currently resolves to `undefined`; scripts must call `.item(i)`
//! until slot `#11-events-modern-indexed-exotic` lands the dispatch
//! (mirrors `DOMTokenList` / `CSSRuleList`).

#![cfg(feature = "engine")]

use super::super::value::{ObjectId, StringId};

pub(in crate::vm) mod data_transfer;
pub(in crate::vm) mod drag;
pub(in crate::vm) mod pointer;
pub(in crate::vm) mod touch;

// `ua_shape_fold` is intentionally elided as a separate module:
// the UA-shape gap fold (plan v4 §F.2) lives inline in
// `vm/host/event_shapes.rs::dispatch_payload`, since the fold is
// purely an extension of an existing match — splitting would
// require a public extension trait for marginal LoC savings.

// Re-export the per-cluster register fns so `globals.rs` can call
// them in a single `use` block.
pub(in crate::vm) use data_transfer::{
    register_data_transfer_global, register_data_transfer_item_global,
    register_data_transfer_item_list_global,
};
pub(in crate::vm) use drag::register_drag_event_global;
pub(in crate::vm) use pointer::register_pointer_event_global;
pub(in crate::vm) use touch::{
    register_touch_event_global, register_touch_global, register_touch_list_global,
};

// ---------------------------------------------------------------------------
// Mutable side-table state structs
// ---------------------------------------------------------------------------

/// Per-`DataTransfer` mutable state (HTML DnD §6.2).  Keyed in
/// `crate::vm::VmInner::data_transfer_states` by the wrapper's
/// `ObjectId`.
///
/// Identity-cached `[SameObject]` wrappers for `items` / `files`
/// hang off the state entry so `dt.items === dt.items` holds.  The
/// drag-image element reference is stored as raw `entity_bits` so
/// the state can be Copy-cheap; integration with the paint pipeline
/// is deferred to slot `#11-data-transfer-drag-image-paint`.
pub(crate) struct DataTransferState {
    /// `dropEffect` enum (HTML §6.2): `0=none / 1=copy / 2=link / 3=move`.
    pub(in crate::vm) drop_effect: DropEffect,
    /// `effectAllowed` enum (HTML §6.2): `0=none / 1=copy / 2=copyLink /
    /// 3=copyMove / 4=link / 5=linkMove / 6=move / 7=all /
    /// 8=uninitialized`.
    pub(in crate::vm) effect_allowed: EffectAllowed,
    /// Ordered entry list (string + file kinds).  Mutation order
    /// matches WebIDL `add` / `setData` / `clearData` / `remove` /
    /// `clear` semantics; UA-fired drag/clipboard events populate
    /// this in spec order (deferred via `#11-event-dispatch-extra`).
    pub(in crate::vm) items: Vec<DataTransferEntry>,
    /// `[SameObject]` cache for the `items` accessor (HTML §6.2).
    /// `None` until the first `dt.items` read; subsequent reads
    /// return the cached wrapper.
    pub(in crate::vm) items_wrapper: Option<ObjectId>,
    /// `[SameObject]` cache for the `files` accessor (HTML §6.2).
    /// D-9 ships an empty FileList stub — the wrapper itself is
    /// always identity-stable, but the FileList interface is
    /// payload-empty pending D-14 (slot
    /// `#11-data-transfer-file-paired`).
    pub(in crate::vm) files_wrapper: Option<ObjectId>,
    /// `setDragImage` element reference as `entity_bits` (NonZero
    /// when present).  Pre-populated to `None`; `setDragImage(el,
    /// x, y)` writes both this and the offsets atomically.  Real
    /// paint integration is deferred (`#11-data-transfer-drag-
    /// image-paint`).
    pub(in crate::vm) drag_image_entity: Option<u64>,
    /// `setDragImage` x offset (WebIDL `long`).
    pub(in crate::vm) drag_image_x: i32,
    /// `setDragImage` y offset (WebIDL `long`).
    pub(in crate::vm) drag_image_y: i32,
}

impl DataTransferState {
    /// Fresh empty state — `new DataTransfer()` default.  Matches
    /// HTML §6.2 + Chrome / Firefox shipping ctor behaviour.
    pub(in crate::vm) fn empty() -> Self {
        Self {
            drop_effect: DropEffect::None,
            effect_allowed: EffectAllowed::None,
            items: Vec::new(),
            items_wrapper: None,
            files_wrapper: None,
            drag_image_entity: None,
            drag_image_x: 0,
            drag_image_y: 0,
        }
    }
}

/// `DataTransfer.dropEffect` enum (HTML §6.2).  ASCII-CI input is
/// canonicalised to the lowercase enum form on set; invalid values
/// silently leave the prior value (per spec).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::vm) enum DropEffect {
    None = 0,
    Copy = 1,
    Link = 2,
    Move = 3,
}

impl DropEffect {
    /// Try to parse an ASCII-CI string into a `DropEffect` enum
    /// variant.  `None` (the Option, not the enum) signals "not a
    /// recognised value" — callers preserve the prior state per
    /// spec.
    pub(in crate::vm) fn from_str_ascii_ci(s: &str) -> Option<Self> {
        if s.eq_ignore_ascii_case("none") {
            Some(Self::None)
        } else if s.eq_ignore_ascii_case("copy") {
            Some(Self::Copy)
        } else if s.eq_ignore_ascii_case("link") {
            Some(Self::Link)
        } else if s.eq_ignore_ascii_case("move") {
            Some(Self::Move)
        } else {
            None
        }
    }

    /// Canonical lowercase representation per HTML §6.2.
    pub(in crate::vm) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Copy => "copy",
            Self::Link => "link",
            Self::Move => "move",
        }
    }
}

/// `DataTransfer.effectAllowed` enum (HTML §6.2).  9 values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::vm) enum EffectAllowed {
    None = 0,
    Copy = 1,
    CopyLink = 2,
    CopyMove = 3,
    Link = 4,
    LinkMove = 5,
    Move = 6,
    All = 7,
    Uninitialized = 8,
}

impl EffectAllowed {
    pub(in crate::vm) fn from_str_ascii_ci(s: &str) -> Option<Self> {
        if s.eq_ignore_ascii_case("none") {
            Some(Self::None)
        } else if s.eq_ignore_ascii_case("copy") {
            Some(Self::Copy)
        } else if s.eq_ignore_ascii_case("copyLink") {
            Some(Self::CopyLink)
        } else if s.eq_ignore_ascii_case("copyMove") {
            Some(Self::CopyMove)
        } else if s.eq_ignore_ascii_case("link") {
            Some(Self::Link)
        } else if s.eq_ignore_ascii_case("linkMove") {
            Some(Self::LinkMove)
        } else if s.eq_ignore_ascii_case("move") {
            Some(Self::Move)
        } else if s.eq_ignore_ascii_case("all") {
            Some(Self::All)
        } else if s.eq_ignore_ascii_case("uninitialized") {
            Some(Self::Uninitialized)
        } else {
            None
        }
    }

    pub(in crate::vm) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Copy => "copy",
            Self::CopyLink => "copyLink",
            Self::CopyMove => "copyMove",
            Self::Link => "link",
            Self::LinkMove => "linkMove",
            Self::Move => "move",
            Self::All => "all",
            Self::Uninitialized => "uninitialized",
        }
    }
}

/// A single entry in the `DataTransfer.items` list (HTML §6.3).
/// D-9 ships only String entries — File entries are TypeError
/// stubs until D-14 (slot `#11-data-transfer-file-paired`).
pub(in crate::vm) enum DataTransferEntry {
    /// `add(string, type)` / `setData(format, data)`: a `(format,
    /// data)` pair both held as interned `StringId`s.  HTML §6.3
    /// step: kind="string", type=format, data=data.
    String { format: StringId, data: StringId },
    /// `add(File)` overload — D-9 reserves the variant but does
    /// not allow construction (the `add(File)` path throws
    /// TypeError).  D-14 will activate File entries by populating
    /// `blob_id` (the wrapper) + `type_sid` from the File object.
    ///
    /// Variant kept in the enum so GC trace + serialisation code
    /// already handle the File-entry shape when D-14 lands; until
    /// then, the variant is unreachable from JS so the
    /// `#[allow(dead_code)]` suppression is intentional.
    #[allow(dead_code)]
    File {
        blob_id: ObjectId,
        type_sid: StringId,
    },
}

/// Per-`Touch` mutable state (Touch Events §5.6).  Keyed in
/// `crate::vm::VmInner::touch_states` by the wrapper's
/// `ObjectId`.  All 12 IDL members live here as `f64` (Number
/// idiom) + `Option<ObjectId>` (the EventTarget `target`).
pub(crate) struct TouchState {
    /// `Touch.identifier` (WebIDL `long`).
    pub(in crate::vm) identifier: i32,
    /// `Touch.target` (`EventTarget?`).  Accepts any EventTarget
    /// brand (HostObject / AbortSignal / etc.).  `None` if the
    /// init dict supplied `null` — actually `Touch.target` is
    /// `required` per IDL, so a missing value throws TypeError at
    /// ctor time; this Option captures the post-validation state.
    pub(in crate::vm) target: Option<ObjectId>,
    pub(in crate::vm) client_x: f64,
    pub(in crate::vm) client_y: f64,
    pub(in crate::vm) screen_x: f64,
    pub(in crate::vm) screen_y: f64,
    pub(in crate::vm) page_x: f64,
    pub(in crate::vm) page_y: f64,
    /// `Touch.radiusX` (WebIDL `float`).  VM stores as f64 because
    /// the Number primitive is always f64; no precision clamping
    /// per the no-clamp convention used by PointerEvent.pressure.
    pub(in crate::vm) radius_x: f64,
    pub(in crate::vm) radius_y: f64,
    pub(in crate::vm) rotation_angle: f64,
    pub(in crate::vm) force: f64,
}

/// Per-`TouchList` state (Touch Events §5.6).  Keyed in
/// `crate::vm::VmInner::touch_list_states` by the wrapper's
/// `ObjectId`.
pub(crate) struct TouchListState {
    /// Member [`super::super::value::ObjectKind::Touch`] wrapper
    /// IDs in spec order.  TouchList is indexed exotic — `list[i]`
    /// reads this Vec directly; out-of-range returns null per IDL.
    pub(in crate::vm) items: Vec<ObjectId>,
}
