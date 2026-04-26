//! Byte-level read/write primitives over [`super::super::VmInner::body_data`].
//!
//! Shared by [`super::data_view`] (per-method getters / setters) and
//! [`super::typed_array`] (per-element raw read / write).  The
//! backing storage is `Vec<u8>` — owned, in-place mutable.  Reads
//! snapshot the requested span into a fixed-size array (so partial
//! reads near the buffer's end zero-pad cleanly), writes mutate the
//! `Vec<u8>` directly via `entry().or_insert_with(Vec::new).resize()`
//! + `copy_from_slice`.  No clone-grow-install round-trip; repeated
//! writes are O(N) total bytes touched, not O(N²).
//!
//! Cross-subsystem callers (`fetch` HTTP handoff,
//! `body_mixin::read_body_bytes`, `structured_clone`) snapshot to an
//! `Arc<[u8]>` *at the boundary* via `Arc::<[u8]>::from(&vec[..])` —
//! the snapshot semantics that the previous immutable-`Arc` storage
//! delivered implicitly are now visible in the type signature.

#![cfg(feature = "engine")]

use std::collections::HashMap;

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
    body_data: &HashMap<ObjectId, Vec<u8>>,
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
/// needed.  Mutates the `Vec<u8>` in place — other views over the
/// same `buffer_id` observe the mutation through their next
/// `body_data.get(&buffer_id)` (the entry's identity is preserved).
///
/// Callers retain full responsibility for bounds-checking against
/// the *view's* own `[[ByteLength]]` — this helper only ensures
/// the underlying buffer is large enough to hold the write itself.
pub(super) fn write_at(
    body_data: &mut HashMap<ObjectId, Vec<u8>>,
    buffer_id: ObjectId,
    abs: usize,
    bytes: &[u8],
) {
    if bytes.is_empty() {
        // Zero-length writes are pure no-ops — short-circuit
        // before `entry().or_default()` so a caller passing an
        // empty slice doesn't accidentally materialise a
        // `body_data` entry, which would break the
        // `body_data.contains_key(&id)` "carries bytes?" signal
        // documented at `array_buffer::create_array_buffer_from_bytes`
        // and `fetch.rs` response-body installation.  Mirrors
        // `fill_pattern`'s `total_len == 0` early return.
        return;
    }
    // `abs + bytes.len()` can overflow on 32-bit targets when
    // callers pass an `abs` near `usize::MAX`.  Treat overflow as
    // a no-op write — the call sites pre-validate against their
    // own view's `[[ByteLength]]`, so reaching this branch
    // indicates a malformed receiver that must not corrupt the
    // backing buffer or panic.
    let Some(end) = abs.checked_add(bytes.len()) else {
        return;
    };
    let dst = body_data.entry(buffer_id).or_default();
    if dst.len() < end {
        dst.resize(end, 0);
    }
    dst[abs..end].copy_from_slice(bytes);
}

/// Copy `len` bytes from `body_data[src_id][src_abs..]` to
/// `body_data[dst_id][dst_abs..]`, growing the destination buffer
/// with zero-fill as needed.  Source bytes that fall past the
/// source buffer's length are read as zero (mirroring
/// [`read_into`]'s partial-read contract).
///
/// Replaces the per-element `read_element_raw` + `write_element_raw`
/// loop pattern (`slice()`, `copyWithin()`, same-`ElementKind`
/// `set(TypedArray)`) — one src snapshot + one dst in-place resize
/// instead of N decode/encode round-trips.
///
/// The src snapshot is taken into a fresh `Vec<u8>` *before*
/// borrowing the destination, both so overlapping source/destination
/// ranges (`src_id == dst_id`) are correct under any direction
/// (the destination write copies from the snapshot, never re-reading
/// bytes that the earlier write already overwrote) and so the
/// `body_data` HashMap is freed for the subsequent `entry().or_default()`
/// borrow.
///
/// Callers retain responsibility for any view-relative bounds
/// check.  Zero-length, length overflow, and offset overflow are
/// all silent no-ops — the call sites pre-validate against their
/// own view's `[[ByteLength]]`, so reaching either branch
/// indicates a malformed receiver that must not corrupt the
/// backing buffer or panic.
pub(super) fn copy_bytes(
    body_data: &mut HashMap<ObjectId, Vec<u8>>,
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
    let src_snapshot: Vec<u8> = match body_data.get(&src_id) {
        Some(bytes) => {
            let mut out = vec![0_u8; len];
            let buf_len = bytes.len();
            if src_abs < buf_len {
                let avail = (buf_len - src_abs).min(len);
                out[..avail].copy_from_slice(&bytes[src_abs..src_abs + avail]);
            }
            out
        }
        None => vec![0_u8; len],
    };
    let dst = body_data.entry(dst_id).or_default();
    if dst.len() < dst_end {
        dst.resize(dst_end, 0);
    }
    dst[dst_abs..dst_end].copy_from_slice(&src_snapshot);
}

/// Write `pattern` `count` times consecutively into
/// `body_data[buffer_id]` starting at absolute byte offset `abs`,
/// growing the backing buffer with zero-fill as needed.  Mutates
/// the existing `Vec<u8>` in place — one resize and one inner
/// fill loop replace the previous `clone-grow-install` per
/// iteration, collapsing `%TypedArray%.prototype.fill` from O(N²)
/// bytes touched to O(N).  Single-byte patterns hit the
/// `slice::fill` fast path; wider patterns chunk in a tight
/// inner loop.
///
/// Callers retain responsibility for any view-relative bounds
/// check.  Overflow on `pattern.len() * count` or
/// `abs + total_len` is treated as a no-op write — the call sites
/// pre-validate against their own view's `[[ByteLength]]`, so
/// reaching either branch indicates a malformed receiver that
/// must not corrupt the backing buffer or panic.
pub(super) fn fill_pattern(
    body_data: &mut HashMap<ObjectId, Vec<u8>>,
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
        // pure no-ops — skip the resize so callers don't
        // accidentally materialise a `body_data` entry from a
        // zero-byte operation.
        return;
    }
    let Some(end) = abs.checked_add(total_len) else {
        return;
    };
    let dst = body_data.entry(buffer_id).or_default();
    if dst.len() < end {
        dst.resize(end, 0);
    }
    // Post-`total_len == 0` early-return: `pattern.len() >= 1` and
    // `count >= 1`, so the empty-pattern arm is unreachable here.
    match pattern {
        [b] => dst[abs..end].fill(*b),
        _ => {
            let plen = pattern.len();
            for i in 0..count {
                let dst_start = abs + i * plen;
                dst[dst_start..dst_start + plen].copy_from_slice(pattern);
            }
        }
    }
}
