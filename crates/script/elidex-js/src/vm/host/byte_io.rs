//! Byte-level read/write primitives over [`super::super::VmInner::body_data`].
//!
//! Shared by [`super::data_view`] (per-method getters / setters) and
//! [`super::typed_array`] (per-element raw read / write).  The
//! backing storage is `Vec<u8>` — owned, in-place mutable.  Reads
//! snapshot the requested span into a fixed-size array (so partial
//! reads near the buffer's end zero-pad cleanly), writes mutate the
//! `Vec<u8>` directly via `entry().or_default().resize()`
//! + `copy_from_slice`.  No clone-grow-install round-trip; repeated
//!   writes are O(N) total bytes touched, not O(N²).
//!
//! Cross-subsystem callers (`fetch` HTTP handoff,
//! `body_mixin::take_body_bytes`, `structured_clone`,
//! `array_buffer::array_buffer_view_bytes`) take an owned snapshot
//! *at the boundary* from the backing `Vec<u8>` — by `clone`,
//! `remove`, or sub-range `to_vec`, depending on whether the
//! consumer is non-destructive or one-shot.  Some boundaries keep
//! the snapshot as `Vec<u8>` (structured clone of `ArrayBuffer`,
//! body-mixin `.arrayBuffer()`); others convert it to `Arc<[u8]>`
//! only when the downstream API requires shared-immutable bytes
//! (`fetch` → `Bytes::from_owner` needs `Send + Sync`, `BlobData`
//! stores `Arc<[u8]>` per-spec immutability).  The snapshot
//! semantics that the previous immutable-`Arc` storage delivered
//! implicitly are now visible in those boundary APIs' types.

#![cfg(feature = "engine")]

use std::collections::HashMap;

use super::super::value::ObjectId;
use super::super::VmInner;

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
    if let [b] = pattern {
        dst[abs..end].fill(*b);
    } else {
        let plen = pattern.len();
        for i in 0..count {
            let dst_start = abs + i * plen;
            dst[dst_start..dst_start + plen].copy_from_slice(pattern);
        }
    }
}

// ---------------------------------------------------------------------------
// DR-11 wasm-backed routing wrappers (D-16 `#11-wasm-vm` plan §5
// Stage 4.1)
// ---------------------------------------------------------------------------
//
// These wrappers consult [`VmInner::wasm_backed_buffers`] before
// touching `body_data`.  When the buffer is wasm-backed, the access
// is dispatched through the live [`elidex_wasm_runtime::WasmMemoryView`]
// stashed at
// [`super::super::wasm_payload::WasmMemoryPayload::view`]; otherwise
// the standard `body_data` path runs unchanged.
//
// Coupling invariant (plan §5 Stage 4.1) — `wasm_backed_buffers` is
// `HashMap<ObjectId /* buf_id */, ObjectId /* mem_id */>` and the
// `Some(_)` notation below is `HashMap::get`'s `Option<&V>` return,
// not the value type:
//   `vm.wasm_backed_buffers.get(&buf_id) == Some(&mem_id)`
//   ⇔  `vm.wasm_memory_storage[&mem_id].view.is_some()`
// Both halves are written together at `.buffer` first-fire (entry
// insert + payload `view = Some(...)`) and cleared together at detach
// (entry remove + payload `view = None`).  Future code that touches
// either field MUST preserve the biconditional or refactor the
// `view.as_ref().unwrap()` sites to `.expect("coupling invariant")`
// with clear panic discipline.

/// Routing wrapper over [`read_into`] that consults
/// [`VmInner::wasm_backed_buffers`] for wasm-backed ArrayBuffers.
///
/// Wasm-backed reads dispatch through the stashed
/// [`elidex_wasm_runtime::WasmMemoryView::read`]; non-wasm-backed
/// reads fall through to [`read_into`] against `vm.body_data`.
/// Partial-read semantics (zero-pad past buffer end) match the
/// underlying primitive in both arms.
pub(super) fn read_into_with_routing<const N: usize>(
    vm: &VmInner,
    buffer_id: ObjectId,
    abs: usize,
) -> [u8; N] {
    if let Some(&mem_id) = vm.wasm_backed_buffers.get(&buffer_id) {
        let mut out = [0_u8; N];
        let payload = vm
            .wasm_memory_storage
            .get(&mem_id)
            .expect("wasm_backed_buffers → wasm_memory_storage coupling invariant");
        let view = payload
            .view
            .as_ref()
            .expect("wasm_backed_buffers → view Some coupling invariant");
        // Bound by the view's current byte size — out-of-range bytes
        // are zero-padded to match `read_into`'s contract.
        let byte_size = view.byte_size().unwrap_or(0) as usize;
        if abs < byte_size {
            let avail = byte_size.saturating_sub(abs).min(N);
            #[allow(clippy::cast_possible_truncation)]
            if let Ok(bytes) = view.read(abs as u32, avail as u32) {
                out[..avail].copy_from_slice(&bytes);
            }
        }
        out
    } else {
        read_into(&vm.body_data, buffer_id, abs)
    }
}

/// Routing wrapper over [`write_at`].
pub(super) fn write_at_with_routing(
    vm: &mut VmInner,
    buffer_id: ObjectId,
    abs: usize,
    bytes: &[u8],
) {
    if let Some(&mem_id) = vm.wasm_backed_buffers.get(&buffer_id) {
        if bytes.is_empty() {
            return;
        }
        let payload = vm
            .wasm_memory_storage
            .get(&mem_id)
            .expect("wasm_backed_buffers → wasm_memory_storage coupling invariant");
        let view = payload
            .view
            .as_ref()
            .expect("wasm_backed_buffers → view Some coupling invariant");
        #[allow(clippy::cast_possible_truncation)]
        // Silent no-op on OOB matches `write_at`'s contract — call
        // sites pre-validate against the view's byte length.
        let _ = view.write(abs as u32, bytes);
    } else {
        write_at(&mut vm.body_data, buffer_id, abs, bytes);
    }
}

/// Routing wrapper over [`copy_bytes`].
///
/// Handles all 4 combinations of `(src wasm-backed, dst wasm-backed)`:
/// - both standard: delegate to [`copy_bytes`]
/// - src wasm-backed, dst standard: read via view, write via `write_at`
/// - src standard, dst wasm-backed: read via `body_data`, write via view
/// - both wasm-backed: read via src view, write via dst view (may be
///   the same view when `src_id == dst_id`)
///
/// The src snapshot is taken into a fresh `Vec<u8>` before the dst
/// write to handle overlapping ranges correctly (matches
/// [`copy_bytes`] semantics).
pub(super) fn copy_bytes_with_routing(
    vm: &mut VmInner,
    src_id: ObjectId,
    src_abs: usize,
    dst_id: ObjectId,
    dst_abs: usize,
    len: usize,
) {
    if len == 0 {
        return;
    }
    if src_abs.checked_add(len).is_none() || dst_abs.checked_add(len).is_none() {
        return;
    }
    let src_is_wasm = vm.wasm_backed_buffers.contains_key(&src_id);
    let dst_is_wasm = vm.wasm_backed_buffers.contains_key(&dst_id);
    if !src_is_wasm && !dst_is_wasm {
        copy_bytes(&mut vm.body_data, src_id, src_abs, dst_id, dst_abs, len);
        return;
    }
    // Snapshot from src (routing if wasm-backed) into a fresh Vec.
    let src_snapshot: Vec<u8> = if src_is_wasm {
        let mem_id = vm.wasm_backed_buffers[&src_id];
        let payload = &vm.wasm_memory_storage[&mem_id];
        let view = payload
            .view
            .as_ref()
            .expect("wasm_backed_buffers → view Some coupling invariant");
        let byte_size = view.byte_size().unwrap_or(0) as usize;
        let mut buf = vec![0_u8; len];
        if src_abs < byte_size {
            let avail = byte_size.saturating_sub(src_abs).min(len);
            #[allow(clippy::cast_possible_truncation)]
            if let Ok(bytes) = view.read(src_abs as u32, avail as u32) {
                buf[..avail].copy_from_slice(&bytes);
            }
        }
        buf
    } else {
        let mut buf = vec![0_u8; len];
        if let Some(bytes) = vm.body_data.get(&src_id) {
            let buf_len = bytes.len();
            if src_abs < buf_len {
                let avail = buf_len.saturating_sub(src_abs).min(len);
                buf[..avail].copy_from_slice(&bytes[src_abs..src_abs + avail]);
            }
        }
        buf
    };
    write_at_with_routing(vm, dst_id, dst_abs, &src_snapshot);
}

/// Routing wrapper over [`fill_pattern`].
pub(super) fn fill_pattern_with_routing(
    vm: &mut VmInner,
    buffer_id: ObjectId,
    abs: usize,
    pattern: &[u8],
    count: usize,
) {
    let Some(total_len) = pattern.len().checked_mul(count) else {
        return;
    };
    if total_len == 0 {
        return;
    }
    if vm.wasm_backed_buffers.contains_key(&buffer_id) {
        // Materialise the pattern × count into a fresh Vec and
        // dispatch via the view writer.  Allocates O(total_len)
        // memory (same as the routed write path's intermediate
        // buffer); the alternative would be N small view.write()
        // calls each of which incurs the store-borrow cost.
        let mut filled = Vec::with_capacity(total_len);
        for _ in 0..count {
            filled.extend_from_slice(pattern);
        }
        write_at_with_routing(vm, buffer_id, abs, &filled);
    } else {
        fill_pattern(&mut vm.body_data, buffer_id, abs, pattern, count);
    }
}
