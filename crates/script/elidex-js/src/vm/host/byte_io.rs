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
    // `abs + bytes.len()` can overflow on 32-bit targets when
    // callers pass an `abs` near `usize::MAX`.  Treat overflow as
    // a no-op write — the call sites pre-validate against their
    // own view's `[[ByteLength]]`, so reaching this branch
    // indicates a malformed receiver that must not corrupt the
    // backing buffer or panic.
    let Some(end) = abs.checked_add(bytes.len()) else {
        return;
    };
    let mut new_bytes = grow_or_fresh(body_data.get(&buffer_id), end);
    new_bytes[abs..end].copy_from_slice(bytes);
    body_data.insert(buffer_id, Arc::from(new_bytes));
}

/// Copy `len` bytes from `body_data[src_id][src_abs..]` to
/// `body_data[dst_id][dst_abs..]`, growing the destination buffer
/// with zero-fill as needed and installing a fresh `Arc<[u8]>`
/// afterwards.  Source bytes that fall past the source buffer's
/// length are read as zero (mirroring [`read_into`]'s partial-read
/// contract).
///
/// Replaces the per-element `read_element_raw` + `write_element_raw`
/// loop pattern (`slice()`, `copyWithin()`, same-`ElementKind`
/// `set(TypedArray)`) — one src snapshot + one dst clone-grow-install
/// instead of N of each.  The src snapshot is taken into a fresh
/// `Vec<u8>` *before* mutating the destination, so overlapping
/// source/destination ranges (`src_id == dst_id`) are correct
/// under any direction.
///
/// Callers retain responsibility for any view-relative bounds
/// check.  Zero-length, length overflow, and offset overflow are
/// all silent no-ops — the call sites pre-validate against their
/// own view's `[[ByteLength]]`, so reaching either branch
/// indicates a malformed receiver that must not corrupt the
/// backing buffer or panic.
pub(super) fn copy_bytes(
    body_data: &mut HashMap<ObjectId, Arc<[u8]>>,
    src_id: ObjectId,
    src_abs: usize,
    dst_id: ObjectId,
    dst_abs: usize,
    len: usize,
) {
    if len == 0 {
        return;
    }
    if src_abs.checked_add(len).is_none() {
        return;
    }
    let Some(dst_end) = dst_abs.checked_add(len) else {
        return;
    };
    // Snapshot the source slice into a fresh `Vec<u8>` *before*
    // touching the destination, so an in-place overlap (src == dst,
    // typical of `copyWithin`) is correct: the destination write
    // copies from the snapshot, never re-reading bytes that the
    // earlier write already overwrote.
    let src_snapshot: Vec<u8> = match body_data.get(&src_id) {
        Some(arc) => {
            let mut out = vec![0_u8; len];
            let buf_len = arc.len();
            if src_abs < buf_len {
                let avail = (buf_len - src_abs).min(len);
                out[..avail].copy_from_slice(&arc[src_abs..src_abs + avail]);
            }
            out
        }
        None => vec![0_u8; len],
    };
    let mut new_dst = grow_or_fresh(body_data.get(&dst_id), dst_end);
    new_dst[dst_abs..dst_end].copy_from_slice(&src_snapshot);
    body_data.insert(dst_id, Arc::from(new_dst));
}

/// Materialise a writable `Vec<u8>` of length `>= needed` from the
/// existing `body_data` entry, or allocate a fresh zero-filled
/// `Vec` of exactly `needed` bytes when no entry exists.
///
/// The fresh-buffer path skips the `&[]`-to-`Vec` clone that
/// `current.to_vec()` would otherwise perform on a missing entry,
/// going straight to a single `vec![0; needed]` allocation.
/// Pre-existing entries fall through to clone + grow as before.
fn grow_or_fresh(current: Option<&Arc<[u8]>>, needed: usize) -> Vec<u8> {
    match current {
        Some(arc) => {
            let mut new_bytes: Vec<u8> = arc.as_ref().to_vec();
            if new_bytes.len() < needed {
                new_bytes.resize(needed, 0);
            }
            new_bytes
        }
        None => vec![0_u8; needed],
    }
}

/// Write `pattern` `count` times consecutively into
/// `body_data[buffer_id]` starting at absolute byte offset `abs`,
/// growing the backing buffer with zero-fill as needed and
/// installing a single fresh `Arc<[u8]>` afterwards.
///
/// Replaces the per-element loop pattern (`fill()` etc.) where
/// each iteration would otherwise clone the entire buffer through
/// [`write_at`].  One clone-grow-install instead of N collapses
/// `%TypedArray%.prototype.fill` from O(N²) bytes touched to
/// O(N).  Single-byte patterns hit the `slice::fill` fast path;
/// wider patterns chunk in a tight inner loop.
///
/// Callers retain responsibility for any view-relative bounds
/// check.  Overflow on `pattern.len() * count` or
/// `abs + total_len` is treated as a no-op write — the call sites
/// pre-validate against their own view's `[[ByteLength]]`, so
/// reaching either branch indicates a malformed receiver that
/// must not corrupt the backing buffer or panic.
pub(super) fn fill_pattern(
    body_data: &mut HashMap<ObjectId, Arc<[u8]>>,
    buffer_id: ObjectId,
    abs: usize,
    pattern: &[u8],
    count: usize,
) {
    let Some(total_len) = pattern.len().checked_mul(count) else {
        return;
    };
    if total_len == 0 {
        // Zero-length writes (`count == 0`, `pattern == []`) are
        // pure no-ops — skip the clone/install so callers don't
        // accidentally materialise a `body_data` entry from a
        // zero-byte operation.
        return;
    }
    let Some(end) = abs.checked_add(total_len) else {
        return;
    };
    let mut new_bytes = grow_or_fresh(body_data.get(&buffer_id), end);
    // Post-`total_len == 0` early-return: `pattern.len() >= 1` and
    // `count >= 1`, so the empty-pattern arm is unreachable here.
    match pattern {
        [b] => new_bytes[abs..end].fill(*b),
        _ => {
            let plen = pattern.len();
            for i in 0..count {
                let dst_start = abs + i * plen;
                new_bytes[dst_start..dst_start + plen].copy_from_slice(pattern);
            }
        }
    }
    body_data.insert(buffer_id, Arc::from(new_bytes));
}
