//! Tracing mark-and-sweep garbage collector for the elidex-js VM.
//!
//! Collects unreachable [`Object`]s and [`Upvalue`]s.  Strings, symbols,
//! shapes, and compiled functions are permanent and not collected.
//!
//! ## Design
//!
//! - **Stop-the-world**: GC pauses all JS execution.
//! - **Bit-vector marks**: `Vec<u64>` cached on `VmInner` to avoid re-allocating.
//! - **Explicit work list**: `Vec<u32>` avoids deep recursion on the object graph.
//! - **Free functions for mark phase**: split borrow — mark bits are `&mut` while
//!   objects/upvalues/stack/frames are `&` (immutable).
//! - **IC invalidation**: after sweep, stale IC entries referencing collected
//!   objects are cleared.
//!
//! ## Module layout
//!
//! Split across this directory to keep each file under the project's
//! 1000-line convention; the natural seams of the GC pipeline give a
//! one-phase-per-file mapping:
//!
//! - [`mod@roots`] — root-set walker (`GcRoots` snapshot + `mark_roots`).
//! - [`mod@trace`] — work-list closure walker (`trace_work_list`).
//! - [`mod@sweep`] — post-mark slot reclamation + IC invalidation.
//! - [`mod@collect`] — `VmInner::collect_garbage` orchestrator.
//!
//! Bit-vector helpers and the single-object mark primitives live in
//! this file because both halves of the mark phase consume them.
//!
//! ## Future evolution
//!
//! 1. Generational GC (nursery + semi-space scavenger)
//! 2. Incremental marking (write barriers via `PropertyStorage` access points)
//! 3. Lazy sweeping + selective compaction
//! 4. Concurrent marking (separate thread)

mod collect;
mod roots;
mod sweep;
mod trace;

#[cfg(test)]
mod tests;

use super::value::{JsValue, ObjectId, Upvalue, UpvalueId, UpvalueState};

// ---------------------------------------------------------------------------
// Bit-vector helpers
// ---------------------------------------------------------------------------

#[inline]
pub(super) fn bit_set(words: &mut [u64], idx: u32) {
    let (word, bit) = (idx as usize / 64, u64::from(idx) % 64);
    if word < words.len() {
        words[word] |= 1u64 << bit;
    }
}

#[inline]
pub(super) fn bit_get(words: &[u64], idx: u32) -> bool {
    let (word, bit) = (idx as usize / 64, u64::from(idx) % 64);
    word < words.len() && (words[word] & (1u64 << bit)) != 0
}

pub(super) fn resize_marks(marks: &mut Vec<u64>, capacity: usize) {
    let needed = capacity.div_ceil(64);
    if marks.len() < needed {
        marks.resize(needed, 0);
    }
}

pub(super) fn clear_marks(marks: &mut [u64]) {
    marks.fill(0);
}

// ---------------------------------------------------------------------------
// Mark phase (free functions for split borrow)
// ---------------------------------------------------------------------------

/// Mark a JsValue: if it's an Object, enqueue for tracing.
#[inline]
pub(super) fn mark_value(val: JsValue, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    if let JsValue::Object(id) = val {
        mark_object(id, obj_marks, work);
    }
    // BigInts, strings, symbols are permanent (pooled) — no tracing needed.
}

/// Mark an ObjectId as live and enqueue it for tracing (if not already marked).
#[inline]
pub(super) fn mark_object(id: ObjectId, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    let idx = id.0;
    if !bit_get(obj_marks, idx) {
        bit_set(obj_marks, idx);
        work.push(idx);
    }
}

/// Mark an Upvalue as live and trace its closed-over value.
#[inline]
pub(super) fn mark_upvalue(
    uv_id: UpvalueId,
    upvalues: &[Upvalue],
    uv_marks: &mut [u64],
    obj_marks: &mut [u64],
    work: &mut Vec<u32>,
) {
    let idx = uv_id.0;
    if !bit_get(uv_marks, idx) {
        bit_set(uv_marks, idx);
        // Open upvalues reference the stack (already a root).
        // Closed upvalues hold a JsValue that needs marking.
        if let UpvalueState::Closed(val) = upvalues[idx as usize].state {
            mark_value(val, obj_marks, work);
        }
    }
}
