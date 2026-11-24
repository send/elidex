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
//! - `keepalive` — the keepalive-predicate seam: marks each non-Node
//!   `EventTarget` whose spec keepalive rule holds (engine-only).
//! - [`mod@trace`] — work-list closure walker (`trace_work_list`).
//! - [`mod@sweep`] — post-mark slot reclamation + IC invalidation.
//! - [`mod@collect`] — `VmInner::collect_garbage` orchestrator.
//!
//! Bit-vector helpers, the single-object mark primitives and the
//! frame-level primitive ([`mark_frame`]) live in this file because both
//! halves of the mark phase consume them.
//!
//! ## Future evolution
//!
//! 1. Generational GC (nursery + semi-space scavenger)
//! 2. Incremental marking (write barriers via `PropertyStorage` access points)
//! 3. Lazy sweeping + selective compaction
//! 4. Concurrent marking (separate thread)

mod collect;
#[cfg(feature = "engine")]
mod keepalive;
mod roots;
mod sweep;
mod trace;

#[cfg(test)]
mod tests;

use super::coroutine_types::FrameCompletion;
use super::frame::{CallFrame, FrameKind, HandlerEntry, Upvalue, UpvalueState};
use super::value::{CallMode, FuncId, JsValue, ObjectId, UpvalueId};

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

/// Mark every GC handle a stored [`CallFrame`] holds.
///
/// Implements no abstract operation; it answers to ECMA-262 §9.9.2 Liveness
/// (with §9.9.1 Objectives): the spec does not require a field to be marked
/// (Note 5: presence of an object or a symbol in a field, an internal slot,
/// or a property does not imply that the value is live), and Note 6 says
/// engines use conservative approximations such as reachability.  Marking
/// every handle field of a frame is that approximation.  The hard requirement
/// is the engine's own soundness, separate from the spec: sweeping an object
/// a frame still holds frees its `ObjectId` for reuse while the frame still
/// reads it, a use-after-free; so every handle a frame holds stays marked.
/// This is the one enumeration for a frame in either storage residency: a
/// live frame on `VmInner::frames` ([`roots`]'s step (b)) and a suspended
/// frame inside a `Generator`'s `SuspendedFrame` ([`trace`]'s `Generator`
/// arm).
///
/// Completeness is compiler-checked: `CallFrame` and `HandlerEntry` are
/// destructured without `..`, `FrameKind`, `CallMode` and `FrameCompletion`
/// are matched without a wildcard, and every untraced field has its type
/// pinned (`let _: &T = field;`), so adding, removing or retyping a field
/// fails to compile here until it is classified.
pub(super) fn mark_frame(
    frame: &CallFrame,
    upvalues: &[Upvalue],
    uv_marks: &mut [u64],
    obj_marks: &mut [u64],
    work: &mut Vec<u32>,
) {
    let CallFrame {
        upvalue_ids,
        local_upvalue_ids,
        this_value,
        actual_args,
        new_instance,
        mode,
        home_class,
        generator,
        pending_completion,
        exception_handlers,
        kind,
        func_id,
        ip,
        base,
        tdz_bits,
        tdz_overflow,
        cleanup_base,
    } = frame;
    // Untraced: no GC handle in these types (permanent / plain data).  Each
    // type is pinned here.
    let _: &FuncId = func_id;
    let _: &usize = ip;
    let _: &usize = base;
    let _: &u64 = tdz_bits;
    #[allow(clippy::borrowed_box)] // pin the exact type; `&[u64]` would accept any slice deref
    let _: &Box<[u64]> = tdz_overflow;
    let _: &usize = cleanup_base;
    let _: &FrameKind = kind;
    match kind {
        FrameKind::Eval | FrameKind::Function => {}
    }
    for HandlerEntry {
        catch_ip,
        finally_ip,
        stack_depth,
    } in exception_handlers
    {
        let _: &Option<u32> = catch_ip;
        let _: &Option<u32> = finally_ip;
        let _: &usize = stack_depth;
    }

    mark_value(*this_value, obj_marks, work);
    for &uv_id in upvalue_ids.iter() {
        mark_upvalue(uv_id, upvalues, uv_marks, obj_marks, work);
    }
    for &uv_id in local_upvalue_ids {
        mark_upvalue(uv_id, upvalues, uv_marks, obj_marks, work);
    }
    if let Some(args) = actual_args {
        for &val in args {
            mark_value(val, obj_marks, work);
        }
    }
    if let Some(id) = new_instance {
        mark_object(*id, obj_marks, work);
    }
    match mode {
        CallMode::Call => {}
        CallMode::Construct { new_target } => mark_object(*new_target, obj_marks, work),
    }
    if let Some(id) = home_class {
        mark_object(*id, obj_marks, work);
    }
    if let Some(id) = generator {
        mark_object(*id, obj_marks, work);
    }
    // Pending completion value: held across a finally body, an independent
    // root during that window.  Every variant's value is marked (`Normal` is
    // never stored here today; marking it is the fail-safe direction).
    match pending_completion.as_deref() {
        Some(
            FrameCompletion::Normal(v) | FrameCompletion::Return(v) | FrameCompletion::Throw(v),
        ) => mark_value(*v, obj_marks, work),
        None => {}
    }
}
