//! GC-window pins for coroutine creation (`#11-vm-coroutine-creation-frame-rooting`).
//!
//! A generator call builds its initial `SuspendedFrame` from values that sat
//! on the VM stack (locals, the rest-param array, callee, receiver) and from
//! Rust locals (`this`, `actual_args`, the callee's `upvalue_ids`).
//! `VmInner::alloc_object` collects **before** inserting its object and
//! requires every `ObjectId` reachable only through that object to be rooted
//! already, so the container must exist before the values leave the stack.
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
