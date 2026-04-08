//! Inline caches for property access and call-site optimization.
//!
//! Each [`PropertyIC`] caches the result of a property lookup keyed on the
//! receiver object's [`ShapeId`].  When the shape matches (IC hit), the VM
//! reads/writes `slots[slot]` directly — O(1) with no hash lookup.
//!
//! [`CallIC`] caches the resolved function metadata for a call site so that
//! repeated calls to the same callee skip the callee-resolution step.
//!
//! All ICs are **self-invalidating**: a shape guard mismatch falls through to
//! the slow path which re-resolves and overwrites the IC slot.

use std::sync::Arc;

use super::shape::ShapeId;
use super::value::{FuncId, JsValue, ObjectId, ThisMode, UpvalueId};

/// Inline cache for a property access site (GetProp / SetProp).
#[derive(Clone, Debug)]
pub struct PropertyIC {
    /// Shape of the receiver at the time this IC was populated.
    pub receiver_shape: ShapeId,
    /// Slot index where the property value lives.
    pub slot: u16,
    /// Whether the property was found on the receiver itself or on a prototype.
    pub holder: ICHolder,
}

/// Where the cached property was found.
#[derive(Clone, Copy, Debug)]
pub enum ICHolder {
    /// Own property: `receiver.slots[slot]`.
    Own,
    /// Property found on the immediate prototype.
    ///
    /// Guards: (1) receiver shape hasn't added a shadowing property,
    /// (2) `receiver.prototype == Some(proto_id)` (prototype pointer unchanged),
    /// (3) prototype's shape matches `proto_shape`.
    ///
    /// Covers the 95%+ case of `obj.method()` calls where the method lives on
    /// the immediate prototype.  Deeper chains fall through to the slow path.
    Proto {
        proto_shape: ShapeId,
        proto_slot: u16,
        proto_id: ObjectId,
    },
}

/// Inline cache for a call site (Call / CallMethod).
///
/// Caches all resolved function metadata so that IC-hit calls skip the
/// object-table lookup entirely.
#[derive(Clone, Debug)]
pub struct CallIC {
    pub callee: ObjectId,
    pub func_id: FuncId,
    pub this_mode: ThisMode,
    pub upvalue_ids: Arc<[UpvalueId]>,
    pub captured_this: Option<JsValue>,
}
