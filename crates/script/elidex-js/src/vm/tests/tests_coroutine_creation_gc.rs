//! GC-window pins for coroutine creation (`#11-vm-coroutine-creation-frame-rooting`).
//!
//! A generator call builds its initial `SuspendedFrame` from values that sat
//! on the VM stack (locals, the rest-param array, callee, receiver) and from
//! Rust locals (`this`, `actual_args`, the callee's `upvalue_ids`).
//! `VmInner::alloc_object` collects **before** inserting its object and
//! requires every `ObjectId` reachable only through that object to be rooted
//! already, so the container must exist before the values leave the stack.
//! An async call adds two Rust-local objects to the same windows: the
//! wrapper Promise while its coroutine is allocated, and — during a drive —
//! the coroutine, its wrapper and the awaited value while the Promise
//! wrapping a non-Promise `await` operand is allocated.
//! Each cell places one collection inside a creation-time window and asserts
//! the value that window would otherwise lose.
//!
//! Placement: `setup` performs every allocation the case needs (the probed
//! values are handed out through `mk()`, which drops the pool's reference),
//! then the collection is armed at the k-th allocation of `expr`:
//! - k = 1 uses the `force_gc_before_next_alloc` one-shot;
//! - k >= 2 zeroes `gc_bytes_since_last` and sets `gc_threshold` to
//!   `k * OBJECT_ALLOC_ESTIMATE`, so the counter crosses it exactly at the
//!   k-th `alloc_object` (`collect_garbage` resets the threshold, so it fires
//!   once).  This is not the "low `gc_threshold` alone" that
//!   `tests_member_compound_assign_gc.rs` rules out: against a non-zero
//!   counter that fires at the script's first allocation, whereas zeroing the
//!   counter first is what places the k-th.
//!
//! The trailing "a collection happened" assertions are weaker witnesses than
//! the placement argument (they prove some allocation collected, not that it
//! was the one in the window); the placement itself is witnessed by these
//! cells failing on the pre-fix code.

use crate::vm::{JsValue, Vm, VmError, VmInner};

/// Arm one collection at the `k`-th `alloc_object` of the next evaluation.
fn arm_gc_at_alloc(vm: &mut Vm, k: usize) {
    assert!(k >= 1);
    if k == 1 {
        vm.inner.force_gc_before_next_alloc = true;
    } else {
        // alloc i (1-based) collects iff (i-1)·E + E >= k·E, i.e. i >= k.
        vm.inner.gc_bytes_since_last = 0;
        vm.inner.gc_threshold = k * VmInner::object_alloc_estimate();
    }
}

/// True once the armed collection has run.
fn armed_gc_fired(vm: &Vm, k: usize) -> bool {
    if k == 1 {
        !vm.inner.force_gc_before_next_alloc
    } else {
        // `collect_garbage` resets the threshold to at least the floor.
        vm.inner.gc_threshold >= VmInner::gc_threshold_floor()
    }
}

/// Evaluate `setup`, arm a collection at the `k`-th allocation, evaluate `expr`.
fn eval_with_gc_at(setup: &str, k: usize, expr: &str) -> (Vm, Result<JsValue, VmError>) {
    let mut vm = Vm::new();
    vm.eval(setup).expect("probe setup must succeed");
    arm_gc_at_alloc(&mut vm, k);
    let result = vm.eval(expr);
    assert!(
        armed_gc_fired(&vm, k),
        "no collection was placed — `{expr}` never reached allocation {k}"
    );
    (vm, result)
}

/// Like `eval_with_gc_at`, but the armed window is a host `Vm::call` of the
/// global function `name` (no `this`, no arguments).
fn call_with_gc_at(setup: &str, name: &str, k: usize) -> (Vm, Result<JsValue, VmError>) {
    let mut vm = Vm::new();
    vm.eval(setup).expect("probe setup must succeed");
    let Some(JsValue::Object(f)) = vm.get_global(name) else {
        panic!("`{name}` must be a function object");
    };
    arm_gc_at_alloc(&mut vm, k);
    let result = vm.call(f, JsValue::Undefined, &[]);
    assert!(
        armed_gc_fired(&vm, k),
        "no collection was placed — calling `{name}` never reached allocation {k}"
    );
    (vm, result)
}

/// Assert `result` is `Number(expected)`.
fn assert_number(result: Result<JsValue, VmError>, expected: f64, what: &str) {
    match result {
        Ok(JsValue::Number(n)) if n == expected => {}
        other => panic!("{what}: expected Number({expected}), got {other:?}"),
    }
}

/// Assert the global `name` holds `Number(expected)`.
fn assert_global_number(vm: &Vm, name: &str, expected: f64, what: &str) {
    assert_number(
        vm.get_global(name)
            .ok_or_else(|| VmError::internal("global not set")),
        expected,
        what,
    );
}

const POOL: &str = "globalThis.pool = []; globalThis.mk = function () { return pool.pop() }; ";

/// `push_js_call_frame` generator branch: the receiver of a generator method
/// call is dropped from the stack (`truncate(cleanup_base)`) and is then held
/// only by the initial frame's `this_value`.
#[test]
fn generator_method_receiver_survives_generator_allocation() {
    let setup = format!("{POOL} pool.push({{tag: 7, *m() {{ return this.tag }} }});");
    let (_vm, r) = eval_with_gc_at(&setup, 1, "mk().m().next().value");
    assert_number(
        r,
        7.0,
        "receiver read through `this` after the first resume",
    );
}

/// `push_js_call_frame` generator branch: a generator closure that is
/// otherwise unreferenced reaches its captured variable only through the
/// initial frame's `upvalue_ids` once the callee leaves the stack.
#[test]
fn generator_closure_upvalue_survives_generator_allocation() {
    let setup = format!(
        "{POOL} pool.push((function () {{ var o = {{tag: 7}}; \
         return function* () {{ yield o.tag }} }})());"
    );
    let (_vm, r) = eval_with_gc_at(&setup, 1, "mk()().next().value");
    assert_number(r, 7.0, "captured upvalue read after the first resume");
}

/// `push_js_call_frame` generator branch: the rest-param array is a fresh
/// object held only by its stack slot, which `stack.drain(base..)` moves into
/// the initial frame's `stack_slice` before the Generator is allocated
/// (allocation 1 = the rest array, allocation 2 = the Generator).
#[test]
fn generator_rest_array_survives_generator_allocation() {
    let setup = format!(
        "{POOL} pool.push({{tag: 7}}); globalThis.g = function* (...r) {{ yield r[0].tag }};"
    );
    let (_vm, r) = eval_with_gc_at(&setup, 2, "g(mk()).next().value");
    assert_number(r, 7.0, "rest element read after the first resume");
}

/// `push_js_call_frame` generator branch: an excess argument (no rest
/// parameter) is cleared from its stack slot and is then held only by the
/// initial frame's `actual_args` (a Rust `Vec`) while the Generator
/// (allocation 1) is allocated.
#[test]
fn generator_excess_argument_survives_generator_allocation() {
    let setup = format!(
        "{POOL} pool.push({{x: 7}}); \
         globalThis.g = function* () {{ return arguments[0].x }};"
    );
    let (_vm, r) = eval_with_gc_at(&setup, 1, "g(mk()).next().value");
    assert_number(
        r,
        7.0,
        "excess argument read through `arguments` after the first resume",
    );
}

/// `call_internal` generator branch (host entry, `Vm::call`): the rest-param
/// array is the one value whose rooting is `call_internal`'s own business
/// (`this` / `args` / the callee are the caller's) — it is held only by its
/// stack slot, which is drained into the initial frame before the Generator is
/// allocated (allocation 1 = the rest array, allocation 2 = the Generator).
#[test]
fn host_call_generator_rest_array_survives_generator_allocation() {
    let setup = "globalThis.g = function* (...r) { yield r.length };";
    let (mut vm, r) = call_with_gc_at(setup, "g", 2);
    vm.set_global("G", r.expect("generator call"));
    assert_number(vm.eval("G.next().value"), 0.0, "rest array length");
}

/// `push_js_call_frame` async branch → `make_async_coroutine_and_drive`: the
/// receiver of an async method call is dropped from the stack before the
/// wrapper Promise (allocation 1) and the coroutine (allocation 2) are
/// allocated.
#[test]
fn async_method_receiver_survives_coroutine_allocation() {
    let setup = format!(
        "{POOL} globalThis.out = 0; \
         pool.push({{tag: 7, async m() {{ await 0; return this.tag }} }});"
    );
    let (vm, r) = eval_with_gc_at(&setup, 1, "mk().m().then(v => { globalThis.out = v })");
    r.expect("async call");
    assert_global_number(
        &vm,
        "out",
        7.0,
        "receiver read through `this` after the first await",
    );
}

/// `make_async_coroutine_and_drive`: the wrapper Promise (allocation 1) is
/// held only by a Rust local while the coroutine (allocation 2) that will
/// own it is allocated.
#[test]
fn async_wrapper_promise_survives_coroutine_allocation() {
    let setup = "globalThis.f = async function () {};";
    let (_vm, r) = eval_with_gc_at(setup, 2, "f() instanceof Promise");
    match r {
        Ok(JsValue::Boolean(true)) => {}
        other => panic!("async call must return its wrapper Promise, got {other:?}"),
    }
}

/// Setup for the async excess-argument cells: `f` reads its one excess
/// argument (no rest parameter) through `arguments` after its first `await`.
fn async_excess_argument_setup() -> String {
    format!(
        "{POOL} globalThis.out = 0; pool.push({{x: 7}}); \
         globalThis.f = async function () {{ await 0; globalThis.out = arguments[0].x }};"
    )
}

/// `push_js_call_frame` async branch: an excess argument (no rest parameter)
/// is cleared from its stack slot and is then held only by the initial
/// frame's `actual_args` while the wrapper Promise (allocation 1) is
/// allocated.
#[test]
fn async_excess_argument_survives_wrapper_promise_allocation() {
    let (vm, r) = eval_with_gc_at(&async_excess_argument_setup(), 1, "f(mk()); 0");
    r.expect("call");
    assert_global_number(
        &vm,
        "out",
        7.0,
        "excess argument read through `arguments` after the first await",
    );
}

/// As above, with the collection at the coroutine allocation (allocation 2,
/// after the wrapper Promise).
#[test]
fn async_excess_argument_survives_coroutine_allocation() {
    let (vm, r) = eval_with_gc_at(&async_excess_argument_setup(), 2, "f(mk()); 0");
    r.expect("call");
    assert_global_number(
        &vm,
        "out",
        7.0,
        "excess argument read through `arguments` after the first await",
    );
}

/// `call_internal` async branch (host entry, `Vm::call`): rest array
/// (allocation 1), wrapper Promise (allocation 2), coroutine (allocation 3).
#[test]
fn host_call_async_rest_array_survives_coroutine_allocation() {
    let setup = "globalThis.out = -1; \
         globalThis.f = async function (...r) { await 0; globalThis.out = r.length };";
    let (mut vm, r) = call_with_gc_at(setup, "f", 3);
    r.expect("async call");
    vm.eval("0").expect("microtask checkpoint");
    assert_global_number(&vm, "out", 0.0, "rest array length after the first await");
}

/// Initial drive inside `make_async_coroutine_and_drive`: after the body
/// suspends at its first `await` of a non-Promise, `drive_async_coroutine`
/// allocates the awaited Promise (allocation 3) while the coroutine and its
/// wrapper are held only by Rust locals.
#[test]
fn async_initial_drive_coroutine_survives_await_promise_allocation() {
    let setup =
        "globalThis.out = 0; globalThis.f = async function () { await 0; globalThis.out = 7 };";
    let (vm, r) = eval_with_gc_at(setup, 3, "f() instanceof Promise");
    assert!(matches!(r, Ok(JsValue::Boolean(true))), "r={r:?}");
    assert_global_number(
        &vm,
        "out",
        7.0,
        "body ran to completion after the first await",
    );
}

/// `drive_async_coroutine`: an awaited non-Promise object is held only by a
/// Rust local while the Promise that wraps it is allocated.  Placed in a
/// continuation drive (the second `await`), where the coroutine is rooted
/// through the `AsyncDriverStep` in `current_microtask`, so the awaited
/// object is the only unrooted value: allocations 1–2 = wrapper Promise and
/// coroutine, 3 = the Promise wrapping `0` (initial drive), 4 = the Promise
/// wrapping `mk()`'s result (continuation drive).
#[test]
fn async_awaited_object_survives_await_promise_allocation() {
    let setup = format!(
        "{POOL} globalThis.out = 0; pool.push({{tag: 7}}); \
         globalThis.f = async function () {{ await 0; var v = await mk(); globalThis.out = v.tag }};"
    );
    let (vm, r) = eval_with_gc_at(&setup, 4, "f(); 0");
    r.expect("call");
    assert_global_number(
        &vm,
        "out",
        7.0,
        "awaited object read after the second await",
    );
}
