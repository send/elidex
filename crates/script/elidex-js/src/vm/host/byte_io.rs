//! Byte-level read/write primitives over [`super::super::VmInner::body_data`].
//!
//! Shared by [`super::data_view`] (per-method getters / setters) and
//! [`super::typed_array`] (per-element raw read / write).  Both
//! subsystems treat the backing `Arc<[u8]>` as immutable: reads
//! snapshot the requested span into a fixed-size array, writes
//! clone the existing bytes into a fresh `Vec<u8>`, apply the
//! mutation, and install a new `Arc::from(...)`.  This keeps the
//! spec-mandated "all views share bytes" invariant — every view
//! reads through `body_data[buffer_id]`, which always holds the
//! latest snapshot.
//!
//! The O(N)-per-write cost is acceptable for C2; a byte-level
//! interior-mutability refactor (`Rc<RefCell<Vec<u8>>>`) is the
//! larger half of PR-spec-polish SP9 and lands separately.  The
//! present module is the *placement* half: every read/write that
//! pre-existed inline in `data_view` / `typed_array` is now
//! threaded through the same two primitives so the future
//! migration only needs to swap the storage type, not chase
//! call sites.

#![cfg(feature = "engine")]

use std::collections::HashMap;
use std::sync::Arc;

use super::super::value::ObjectId;

/// Read up to `N` bytes from `body_data[buffer_id]` starting at
/// absolute byte offset `abs`.  Returns a fixed-size array
/// initialised to zero; bytes that fall past the underlying
/// buffer's length are left at zero — partial reads are part of
/// the contract, not an error condition.  Callers retain full
/// responsibility for any view-relative bounds check.
///
/// Why partial copies (rather than all-or-nothing): the two
/// callers have different size profiles.  `DataView` getters
/// pre-validate the full `N` span upstream via `ensure_in_range`,
/// so the partial-copy branch is unreachable for them.
/// `TypedArray::read_element_raw` always requests `N = 8` (the
/// widest per-element type) but each `ElementKind` decodes only
/// `bytes_per_element()` of the prefix; a `Uint8Array` reading
/// its last element near the end of the backing buffer may
/// observe `bytes.len() - abs < 8`, and the element byte(s) must
/// still land in the first few bytes of the returned array with
/// the rest zero-padded.
pub(super) fn read_into<const N: usize>(
    body_data: &HashMap<ObjectId, Arc<[u8]>>,
    buffer_id: ObjectId,
    abs: usize,
) -> [u8; N] {
    let mut out = [0_u8; N];
    if let Some(bytes) = body_data.get(&buffer_id) {
        if abs < bytes.len() {
            let avail = (bytes.len() - abs).min(N);
            out[..avail].copy_from_slice(&bytes[abs..abs + avail]);
        }
    }
    out
}

/// Write `bytes` into `body_data[buffer_id]` starting at absolute
/// byte offset `abs`, growing the backing buffer with zero-fill as
/// needed and installing a fresh `Arc<[u8]>` so other views over
/// the same `buffer_id` observe the mutation through their next
/// `body_data.get(&buffer_id)`.
///
/// Callers retain full responsibility for bounds-checking against
/// the *view's* own `[[ByteLength]]` — this helper only ensures
/// the underlying buffer is large enough to hold the write itself.
pub(super) fn write_at(
    body_data: &mut HashMap<ObjectId, Arc<[u8]>>,
    buffer_id: ObjectId,
    abs: usize,
    bytes: &[u8],
) {
    let needed = abs + bytes.len();
    let current: &[u8] = body_data.get(&buffer_id).map(AsRef::as_ref).unwrap_or(&[]);
    let mut new_bytes: Vec<u8> = current.to_vec();
    if new_bytes.len() < needed {
        new_bytes.resize(needed, 0);
    }
    new_bytes[abs..abs + bytes.len()].copy_from_slice(bytes);
    body_data.insert(buffer_id, Arc::from(new_bytes));
}
