//! GC-window pins for the callee / `this` / args of a Rust-side call
//! (`#11-vm-call-internal-callee-this-rooting`), rooted by
//! `call_dispatch`'s entry scope.
//!
//! `ic_call` / `ic_call_method` (`vm/dispatch_ic.rs`) inline only
//! `ObjectKind::Function` callees.  Any other callee — a BoundFunction
//! among them — takes the fallback: the call args are copied into a Rust
//! `Vec`, `self.stack.truncate(..)` drops the callee (and receiver) from
//! the VM stack, and `self.call(..)` runs `call_dispatch`.  Its
//! BoundFunction arm unwraps to the target's Function arm, which enters
//! `call_internal` with the bound `this`, the target closure's
//! `upvalue_ids` and the combined args as Rust values.  `GetProp` pops the
//! receiver before calling a getter, so the getter and its holder are in
//! the same position.  Unless `call_dispatch` roots the callee, `this` and
//! `args`, a collection at an allocation inside `call_internal` before the
//! frame (or coroutine) holds them frees them:
//! - the rest-parameter array (`create_array_object`), for any target with
//!   a rest parameter;
//! - the Generator object, for a generator target.
//!
//! Placement: `setup` performs every allocation the case needs (the probed
//! object is handed out through `mk()`, which drops the pool's reference,
//! so the bound function is reachable only from the call's operand stack),
//! then `force_gc_before_next_alloc` places one collection at the first
//! `alloc_object` of `expr`, which is the allocation inside
//! `call_internal` named in each cell.  Each cell first evaluates the same
//! `expr` without a collection to pin the expected value.

use crate::vm::{JsValue, Vm, VmError};

const POOL: &str = "globalThis.pool = []; globalThis.mk = function () { return pool.pop() }; ";

/// Evaluate `setup` then `expr` in a fresh VM; when `gc`, place one
/// collection at the first allocation of `expr`.
fn eval_probe(setup: &str, expr: &str, gc: bool) -> Result<JsValue, VmError> {
    let mut vm = Vm::new();
    vm.eval(setup).expect("probe setup must succeed");
    if gc {
        vm.inner.force_gc_before_next_alloc = true;
    }
    let result = vm.eval(expr);
    if gc {
        assert!(
            !vm.inner.force_gc_before_next_alloc,
            "no collection was placed — `{expr}` never allocated"
        );
    }
    result
}

/// Assert `expr` yields `Number(expected)` both without and with the
/// collection at its first allocation.
fn assert_survives(setup: &str, expr: &str, expected: f64, what: &str) {
    for gc in [false, true] {
        match eval_probe(setup, expr, gc) {
            Ok(JsValue::Number(n)) if n == expected => {}
            other => panic!("{what} (gc = {gc}): expected Number({expected}), got {other:?}"),
        }
    }
}

/// (a) Non-coroutine target with a rest parameter: the target closure (and
/// through it the captured `o`) and the bound `this` are held only by the
/// unrooted BoundFunction while `call_internal` allocates the rest array.
#[test]
fn bound_rest_target_upvalue_and_this_survive_rest_array_allocation() {
    let setup = format!(
        "{POOL} pool.push((function () {{ var o = {{tag: 7}}; \
         return function (...r) {{ return o.tag + this.t + r.length }} }})().bind({{t: 10}}));"
    );
    assert_survives(
        &setup,
        "mk()(1, 2)",
        19.0,
        "captured upvalue + bound this + rest length",
    );
}

/// (b) Generator target: the target closure's captured `o` and the bound
/// `this` are held only by the unrooted BoundFunction while
/// `call_internal` allocates the Generator.  The generator has no
/// parameters and no object-valued locals at creation, so the initial
/// frame's drained `stack_slice` (the coroutine-creation defect,
/// `#11-vm-coroutine-creation-frame-rooting`) holds nothing this cell
/// reads: what is at risk is the callee / `this`, which that fix does not
/// root (it roots `actual_args` and the async wrapper only).
#[test]
fn bound_generator_target_upvalue_and_this_survive_generator_allocation() {
    let setup = format!(
        "{POOL} pool.push((function () {{ var o = {{tag: 7}}; \
         return function* () {{ yield o.tag + this.t }} }})().bind({{t: 10}}));"
    );
    assert_survives(
        &setup,
        "mk()().next().value",
        17.0,
        "captured upvalue + bound this after the first resume",
    );
}

/// (c) The bound `this` alone: the target is a global function (rooted
/// through the global object), so the only value at risk is the bound
/// receiver, held only by the unrooted BoundFunction while `call_internal`
/// allocates the (empty) rest array.
#[test]
fn bound_this_survives_rest_array_allocation() {
    let setup = format!(
        "{POOL} globalThis.f = function (...r) {{ return this.tag + r.length }}; \
         pool.push(f.bind({{tag: 7}}));"
    );
    assert_survives(&setup, "mk()()", 7.0, "bound this read in the body");
}

/// (d) Accessor getter: `GetProp` pops the
/// receiver before `ic_get_prop` calls a getter, so the holder (and the
/// getter reachable only through it) sits in Rust locals while
/// `call_internal` allocates the Generator.
#[test]
fn getter_generator_holder_upvalue_and_receiver_survive_generator_allocation() {
    let setup = format!(
        "{POOL} var o = {{t: 10}}; Object.defineProperty(o, 'x', {{ get: \
         (function () {{ var c = {{tag: 7}}; return function* () {{ yield c.tag + this.t }} }})() }}); \
         pool.push(o); o = undefined;"
    );
    assert_survives(
        &setup,
        "mk().x.next().value",
        17.0,
        "getter upvalue + receiver",
    );
}
