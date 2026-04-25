//! `TypedArrayParts` snapshot + WebIDL brand-check helper.
//!
//! Hosts the small struct that bundles the four immutable
//! [`super::super::value::ObjectKind::TypedArray`] spec slots with
//! the receiver's `ObjectId`, plus the
//! [`require_typed_array_parts`] brand-check used by every
//! `%TypedArray%.prototype` method.  Sibling-extracted from
//! [`super::typed_array_methods`] so that file stays under the
//! 1000-line convention (cleanup tranche 2 lesson).

#![cfg(feature = "engine")]

use super::super::value::{ElementKind, JsValue, ObjectId, ObjectKind, VmError};
use super::super::NativeContext;

/// Snapshot of the four immutable [`ObjectKind::TypedArray`] spec
/// slots plus the receiver's `ObjectId`, returned by
/// [`require_typed_array_parts`].  Held as a struct so callers can
/// pick the destructure shape that matches what they consume:
///
/// ```ignore
/// // Methods that mutate or return the receiver:
/// let TypedArrayParts { id, buffer_id, byte_offset, element_kind: ek, .. } = parts;
/// // Methods that only need to read/write into the backing buffer:
/// let TypedArrayParts { buffer_id, byte_offset, element_kind: ek, .. } = parts;
/// // Single-field readers (e.g. iterator construction) skip the destructure:
/// let iter = ArrayIterState { array_id: parts.id, .. };
/// ```
///
/// The `..` syntax drops fields the caller doesn't reference,
/// replacing the original 5-tuple form's `_id` / `_buffer_id` /
/// `_byte_offset` / `_byte_length` placeholders.
/// [`Self::len_elem`] and [`Self::bpe`] centralise the
/// `byte_length / bytes_per_element` derivations so each prototype
/// method stops open-coding the same two-line preamble.
#[derive(Clone, Copy)]
pub(super) struct TypedArrayParts {
    /// Receiver of the prototype call (the TypedArray instance
    /// itself), used by methods that need to allocate iterators
    /// (`values` / `keys` / `entries`) or self-mutate (`copyWithin`
    /// / `reverse`).
    pub(super) id: ObjectId,
    /// Backing `ArrayBuffer` whose `body_data` holds the bytes that
    /// `read_element_raw` / `write_element_raw` index into.
    pub(super) buffer_id: ObjectId,
    /// First byte the view covers within `body_data[buffer_id]`.
    pub(super) byte_offset: u32,
    /// Total number of bytes the view covers (always a multiple of
    /// `element_kind.bytes_per_element()`).
    pub(super) byte_length: u32,
    /// Element type — fixes the per-index width and the
    /// (un)signedness / float / BigInt coercion rules used by the
    /// indexed access path.
    pub(super) element_kind: ElementKind,
}

impl TypedArrayParts {
    /// Length in elements: `byte_length / bpe`.  The constructors
    /// guarantee `byte_length % bpe == 0` so the division is exact.
    /// Most prototype methods immediately derive `bpe` and
    /// `len_elem` from `byte_length` + `ek`; this helper centralises
    /// both.
    #[inline]
    pub(super) fn len_elem(&self) -> u32 {
        self.byte_length / self.bpe()
    }

    /// Bytes per element widened to `u32`.  Matches the width of
    /// `byte_length` / `byte_offset` / index arithmetic everywhere
    /// in this module so multiplications such as `len_elem * bpe`
    /// (in `subarray` / `slice`) stay in `u32` without an
    /// intermediate `u8` overflow.
    #[inline]
    pub(super) fn bpe(&self) -> u32 {
        u32::from(self.element_kind.bytes_per_element())
    }
}

/// WebIDL brand-check for `%TypedArray%.prototype` methods.
/// Extracts the four immutable [`ObjectKind::TypedArray`] spec
/// slots inline in one pattern-match and returns them alongside
/// the receiver's id as a [`TypedArrayParts`] snapshot.
pub(super) fn require_typed_array_parts(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<TypedArrayParts, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::TypedArray {
            buffer_id,
            byte_offset,
            byte_length,
            element_kind,
        } => Ok(TypedArrayParts {
            id,
            buffer_id,
            byte_offset,
            byte_length,
            element_kind,
        }),
        _ => Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        ))),
    }
}
