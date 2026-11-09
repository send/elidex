//! Tests for `VmInner::with_call_mode` panic safety and round-trip (moved from `vm/interpreter.rs`).

use std::panic::{catch_unwind, AssertUnwindSafe};

use super::super::value::CallMode;
use super::super::value::JsValue;
use super::super::Vm;

/// D-17b-r1 Phase 0b/Phase 7 positive assertion: a panic inside
/// `with_call_mode`'s closure must leave `VmInner.frames` /
/// `VmInner.stack` at exactly the entry depth before the panic
/// is re-raised. This replaces the pre-r1
/// `dispatch_with_construct_entry_pops_on_panic` test (the
/// `native_construct_stack` it pushed-and-popped no longer
/// exists) — and crucially, it also covers a pre-r1
/// frame-stack-stuck regression: `call_internal`'s
/// `result.is_err()` cleanup gate did not run on a Rust panic
/// mid-dispatch, leaving the pushed frame stuck (R2 CRIT-1
/// discovery).
#[test]
fn with_call_mode_truncates_frame_stack_on_panic() {
    let mut vm = Vm::new();
    let entry_frames = vm.inner.frames.len();

    // Suppress the intentional panic's stderr trace.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
            // Forge a pushed frame, then panic — the helper
            // must truncate the frames Vec back to
            // `entry_frames` before resume_unwind.
            use super::super::value::{CallFrame, FrameKind, FuncId};
            use std::sync::Arc;
            vm.frames.push(CallFrame {
                func_id: FuncId(0),
                ip: 0,
                base: vm.stack.len(),
                upvalue_ids: Arc::from([] as [super::super::value::UpvalueId; 0]),
                local_upvalue_ids: Vec::new(),
                this_value: JsValue::Undefined,
                exception_handlers: Vec::new(),
                tdz_bits: 0,
                tdz_overflow: Box::default(),
                actual_args: None,
                cleanup_base: vm.stack.len(),
                new_instance: None,
                mode: CallMode::Call,
                home_class: None,
                kind: FrameKind::Function,
                generator: None,
                pending_completion: None,
            });
            panic!("intentional panic for with_call_mode frame-truncate regression")
        });
    }));

    std::panic::set_hook(prev_hook);

    assert!(result.is_err(), "panic should propagate via resume_unwind");
    assert_eq!(
        vm.inner.frames.len(),
        entry_frames,
        "frames must be truncated back to entry depth despite panic; got len = {}",
        vm.inner.frames.len()
    );
}

/// Sibling assertion for the value stack: a panic mid-dispatch
/// must leave `VmInner.stack` at exactly the entry depth so any
/// outer dispatch the caller resumes (after the catch_unwind)
/// sees a clean stack.
#[test]
fn with_call_mode_truncates_value_stack_on_panic() {
    let mut vm = Vm::new();
    let entry_stack_depth = vm.inner.stack.len();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
            vm.stack.push(JsValue::Boolean(true));
            vm.stack.push(JsValue::Number(42.0));
            panic!("intentional panic for with_call_mode value-stack-truncate regression")
        });
    }));

    std::panic::set_hook(prev_hook);

    assert!(result.is_err(), "panic should propagate via resume_unwind");
    assert_eq!(
        vm.inner.stack.len(),
        entry_stack_depth,
        "value stack must be truncated back to entry depth despite panic; got len = {}",
        vm.inner.stack.len()
    );
}

/// D-17b-r1 panic-path upvalue-safety regression: a Rust panic
/// mid-dispatch that triggers `with_call_mode`'s frame truncate
/// must `close_upvalues` on every dropped frame's
/// `local_upvalue_ids` BEFORE the stack is truncated, so an
/// Upvalue still in `UpvalueState::Open { frame_base, slot }`
/// gets its slot value snapshotted as `Closed(value)`. Without
/// the close, the upvalue would point at a truncated-away stack
/// region and subsequent reads (via a closure that escaped the
/// panicked frame and survived the upstream `catch_unwind`)
/// would return stale slot values or OOB-panic.
///
/// Closed-via-upvalue invariant covers the F1 finding from the
/// D-17b-r1 code review.
#[test]
fn with_call_mode_closes_open_upvalues_on_panic() {
    use super::super::value::{CallFrame, FrameKind, FuncId, Upvalue, UpvalueId, UpvalueState};
    use std::sync::Arc;

    let mut vm = Vm::new();
    let entry_frames = vm.inner.frames.len();
    let entry_stack_depth = vm.inner.stack.len();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    // Stash an UpvalueId before the panic so we can read its
    // post-panic state after with_call_mode runs.
    let captured_uv_id: UpvalueId = {
        let id = UpvalueId(vm.inner.upvalues.len() as u32);
        vm.inner.upvalues.push(Upvalue {
            state: UpvalueState::Open {
                frame_base: entry_stack_depth,
                slot: 0,
            },
        });
        id
    };

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
            // Push the slot the upvalue references onto the value
            // stack BEFORE pushing the frame that owns it.
            let captured_value = JsValue::Number(123.0);
            vm.stack.push(captured_value);
            // Push a forged frame whose `local_upvalue_ids` lists
            // the upvalue captured above; with_call_mode must
            // close it before stack.truncate drops slot at
            // `entry_stack_depth`.
            vm.frames.push(CallFrame {
                func_id: FuncId(0),
                ip: 0,
                base: entry_stack_depth + 1,
                upvalue_ids: Arc::from([] as [UpvalueId; 0]),
                local_upvalue_ids: vec![captured_uv_id],
                this_value: JsValue::Undefined,
                exception_handlers: Vec::new(),
                tdz_bits: 0,
                tdz_overflow: Box::default(),
                actual_args: None,
                cleanup_base: entry_stack_depth + 1,
                new_instance: None,
                mode: CallMode::Call,
                home_class: None,
                kind: FrameKind::Function,
                generator: None,
                pending_completion: None,
            });
            panic!("intentional panic for close_upvalues regression")
        });
    }));

    std::panic::set_hook(prev_hook);

    assert!(result.is_err(), "panic should propagate via resume_unwind");
    assert_eq!(
        vm.inner.frames.len(),
        entry_frames,
        "frame stack must be truncated"
    );
    assert_eq!(
        vm.inner.stack.len(),
        entry_stack_depth,
        "value stack must be truncated"
    );

    // F1 invariant: the upvalue captured by the truncated frame
    // must be Closed(captured_value), NOT still Open into a
    // now-dropped stack region.
    match vm.inner.upvalues[captured_uv_id.0 as usize].state {
        UpvalueState::Closed(v) => assert_eq!(
            v,
            JsValue::Number(123.0),
            "upvalue must snapshot the pre-truncate slot value"
        ),
        UpvalueState::Open { frame_base, slot } => {
            panic!("upvalue still Open after panic-truncate: frame_base={frame_base}, slot={slot}")
        }
    }
}

/// D-17b-r1 Copilot R2 + R3: `with_call_mode` saves
/// `gc_enabled`, `active_bound_key`, and `completion_value` at
/// entry and restores all three on every exit path (including
/// panic), so a panicked dispatch that skips the inner per-arm
/// restores (NativeFunction arm's `gc_enabled` / `bound_key`,
/// `call_internal`'s `completion_value`) does NOT leak
/// `gc_enabled = false` (which would silently disable GC for
/// the VM's remaining lifetime), a stale `active_bound_key`
/// (which would mis-attribute bound accessors), or a stale
/// `completion_value` (which would corrupt the caller's
/// evaluation state across a nested dispatch like
/// `construct_synchronous`'s JS branch).
#[test]
fn with_call_mode_restores_vm_flags_on_panic() {
    use super::super::value::StringId;

    let mut vm = Vm::new();
    let saved_gc = true;
    let saved_key: Option<StringId> = None;
    let saved_completion = JsValue::Number(42.0);
    vm.inner.gc_enabled = saved_gc;
    vm.inner.active_bound_key = saved_key;
    vm.inner.completion_value = saved_completion;

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
            // Simulate the mid-body state inner save/restore
            // patterns set up: gc_enabled flipped to false,
            // active_bound_key set to a forged sentinel, and
            // completion_value advanced to a different value —
            // then panic before any inner restore fires.
            vm.gc_enabled = false;
            vm.active_bound_key = Some(super::super::value::StringId(u32::MAX));
            vm.completion_value = JsValue::Number(-1.0);
            panic!("intentional panic for with_call_mode flag-restore regression")
        });
    }));

    std::panic::set_hook(prev_hook);

    assert!(result.is_err(), "panic should propagate via resume_unwind");
    assert_eq!(
        vm.inner.gc_enabled, saved_gc,
        "gc_enabled must be restored to entry value despite panic"
    );
    assert_eq!(
        vm.inner.active_bound_key, saved_key,
        "active_bound_key must be restored to entry value despite panic"
    );
    assert_eq!(
        vm.inner.completion_value, saved_completion,
        "completion_value must be restored to entry value despite panic"
    );
}

/// Round-trip: a successful (non-panicking) dispatch under the
/// helper leaves the frame + value stacks at exactly the entry
/// depth — the `Ok`-arm `debug_assert_eq` enforces push/pop
/// balance, and `truncate` is a no-op.
#[test]
fn with_call_mode_round_trip_ok() {
    let mut vm = Vm::new();
    let frames_before = vm.inner.frames.len();
    let stack_before = vm.inner.stack.len();

    let result = vm.inner.with_call_mode(CallMode::Call, |_vm, _mode| {
        // Closure does no work; helper bookkeeping should balance.
        Ok(JsValue::Undefined)
    });

    assert!(result.is_ok());
    assert_eq!(
        vm.inner.frames.len(),
        frames_before,
        "frames length must round-trip"
    );
    assert_eq!(
        vm.inner.stack.len(),
        stack_before,
        "value stack length must round-trip"
    );
}

/// D-17b R19 spirit preserved: when the closure panics, the
/// helper must surface the closure's original panic payload
/// rather than double-panic from a debug-assert. Pre-D-17b-r1
/// the assert was gated on `!std::thread::panicking()`, which
/// `catch_unwind` made always-false on the panic path; a
/// mismatched pop then aborted the thread with an "expected vs
/// got" message, clobbering the upstream-observable failure
/// shape. The r1 helper splits its frames-length assertion by
/// result arm (strict `==` on `Ok`, weaker `>=` on `Err`) and
/// truncates either way before `resume_unwind` — so even when a
/// closure leaves the frames Vec in an unbalanced state, the
/// original panic still surfaces.
#[test]
fn with_call_mode_panic_preserves_original_payload() {
    let mut vm = Vm::new();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |_vm, _mode| {
            panic!("R19_original_marker");
        });
    }));

    std::panic::set_hook(prev_hook);

    let payload = result.expect_err("closure panicked; helper must re-raise");
    let msg = payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string payload>");
    assert_eq!(
        msg, "R19_original_marker",
        "original panic must survive; got {msg:?}"
    );
}
