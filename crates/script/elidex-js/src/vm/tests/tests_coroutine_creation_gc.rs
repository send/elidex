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
//! - k = 2 pins the pressure trigger (`gc_bytes_since_last` /
//!   `gc_threshold`) so it fires exactly at the second `alloc_object`
//!   (`collect_garbage` resets the threshold, so it fires once).
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
        vm.inner.gc_threshold = k * VmInner::OBJECT_ALLOC_ESTIMATE;
    }
}

/// True once the armed collection has run.
fn armed_gc_fired(vm: &Vm, k: usize) -> bool {
    if k == 1 {
        !vm.inner.force_gc_before_next_alloc
    } else {
        // `collect_garbage` resets the threshold to at least 32768.
        vm.inner.gc_threshold >= 32768
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

fn assert_number(result: Result<JsValue, VmError>, expected: f64, what: &str) {
    match result {
        Ok(JsValue::Number(n)) if n == expected => {}
        other => panic!("{what}: expected Number({expected}), got {other:?}"),
    }
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

/// `call_internal` generator branch (host entry, `Vm::call`): the rest-param
/// array is the one value whose rooting is `call_internal`'s own business
/// (`this` / `args` / the callee are the caller's) — it is held only by its
/// stack slot, which is drained into the initial frame before the Generator is
/// allocated (allocation 1 = the rest array, allocation 2 = the Generator).
#[test]
fn host_call_generator_rest_array_survives_generator_allocation() {
    let mut vm = Vm::new();
    vm.eval("globalThis.g = function* (...r) { yield r.length };")
        .expect("setup");
    let Some(JsValue::Object(g)) = vm.get_global("g") else {
        panic!("g must be a function object");
    };
    arm_gc_at_alloc(&mut vm, 2);
    let gen = vm.call(g, JsValue::Undefined, &[]).expect("generator call");
    assert!(
        armed_gc_fired(&vm, 2),
        "no collection at the Generator allocation"
    );
    vm.set_global("G", gen);
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
    assert_number(
        vm.get_global("out")
            .ok_or_else(|| VmError::internal("no out")),
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

/// `call_internal` async branch (host entry, `Vm::call`): rest array
/// (allocation 1), wrapper Promise (allocation 2), coroutine (allocation 3).
#[test]
fn host_call_async_rest_array_survives_coroutine_allocation() {
    let mut vm = Vm::new();
    vm.eval(
        "globalThis.out = -1; \
         globalThis.f = async function (...r) { await 0; globalThis.out = r.length };",
    )
    .expect("setup");
    let Some(JsValue::Object(f)) = vm.get_global("f") else {
        panic!("f must be a function object");
    };
    arm_gc_at_alloc(&mut vm, 3);
    vm.call(f, JsValue::Undefined, &[]).expect("async call");
    assert!(
        armed_gc_fired(&vm, 3),
        "no collection at the coroutine allocation"
    );
    vm.eval("0").expect("microtask checkpoint");
    assert_number(
        vm.get_global("out")
            .ok_or_else(|| VmError::internal("no out")),
        0.0,
        "rest array length after the first await",
    );
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
    let ok = matches!(r, Ok(JsValue::Boolean(true)));
    let out = vm.get_global("out");
    assert!(
        ok && matches!(out, Some(JsValue::Number(n)) if n == 7.0),
        "r={r:?} out={out:?}"
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
    let out = vm.get_global("out");
    assert!(
        matches!(out, Some(JsValue::Number(n)) if n == 7.0),
        "out={out:?}"
    );
}
