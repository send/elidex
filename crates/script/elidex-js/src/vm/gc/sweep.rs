//! Post-mark sweep + IC invalidation.
//!
//! Split from [`super::collect`] so each phase of the GC pipeline
//! sits in its own file under the project's 1000-line convention.
//! These helpers are mechanical: they observe the mark bit-vectors
//! produced by [`super::roots::mark_roots`] /
//! [`super::trace::trace_work_list`] and recycle / invalidate state
//! against them.

use super::super::ic;
use super::super::value::{JsValue, Object, Upvalue, UpvalueState};
use super::bit_get;
use crate::bytecode::compiled::CompiledFunction;

/// Sweep unreachable objects and rebuild the free list.
/// Returns the number of live objects (`objects.len() - free_list.len()`).
pub(super) fn sweep_objects(
    objects: &mut [Option<Object>],
    free_list: &mut Vec<u32>,
    marks: &[u64],
) -> usize {
    free_list.clear();
    for (i, slot) in objects.iter_mut().enumerate() {
        let idx = i as u32;
        if slot.is_some() && !bit_get(marks, idx) {
            *slot = None;
            free_list.push(idx);
        } else if slot.is_none() {
            free_list.push(idx);
        }
    }
    objects.len() - free_list.len()
}

pub(super) fn sweep_upvalues(upvalues: &mut [Upvalue], free_list: &mut Vec<u32>, marks: &[u64]) {
    free_list.clear();
    for (i, uv) in upvalues.iter_mut().enumerate() {
        let idx = i as u32;
        if !bit_get(marks, idx) {
            uv.state = UpvalueState::Closed(JsValue::Undefined);
            free_list.push(idx);
        }
    }
}

pub(super) fn invalidate_ics(compiled_functions: &mut [CompiledFunction], obj_marks: &[u64]) {
    for cf in compiled_functions {
        for slot in &mut cf.ic_slots {
            if let Some(ic) = slot {
                if let ic::ICHolder::Proto { proto_id, .. } = ic.holder {
                    if !bit_get(obj_marks, proto_id.0) {
                        *slot = None;
                    }
                }
            }
        }
        for slot in &mut cf.call_ic_slots {
            if let Some(ic) = slot {
                if !bit_get(obj_marks, ic.callee.0) {
                    *slot = None;
                }
            }
        }
    }
}
