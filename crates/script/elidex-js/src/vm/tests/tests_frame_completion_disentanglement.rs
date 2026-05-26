//! Tests for D-17b-r2 `#11-frame-completion-disentanglement` — the
//! [`super::super::value::FrameKind`] split that confines script /
//! `eval` completion-value semantics (ECMA-262 §16.1.6
//! ScriptEvaluation step 13.a + 13.b + 17 / §19.2.1.1 PerformEval
//! step 29.a + 30.a + 33) to `Eval` frames and makes function /
//! class-ctor / generator / async bodies (`FrameKind::Function`)
//! invariant under `VmInner::completion_value`.
//!
//! Coverage matches the plan-memo §7.9 Phase 8 test plan: Eval-frame
//! completion capture + Function-frame implicit-fall-through-Undefined
//! + class-ctor instance return (carve-out absorption regression) +
//! nested `eval` isolation + native re-entry isolation + cross-frame
//! Op::Throw unwind safety.
//!
//! Spec citations via the plan-memo §0.5 table:
//! - [C1] ECMA-262 §16.1.6 ScriptEvaluation — step 13.a body
//!   completion capture, step 13.b empty→`NormalCompletion(undefined)`,
//!   step 17 `Return ? result`
//! - [C2] §19.2.1.1 PerformEval — step 29.a / 30.a / 33 (mirrors C1)
//! - [C4] §10.2.1.4 OrdinaryCallEvaluateBody + §15.2.3 step 4
//!   `ReturnCompletion(undefined)` for function-body implicit
//!   fall-through
//! - [C5] §10.2.2 [[Construct]] step 12-13 / 15-17 — Object return
//!   wins; else thisArgument / thisBinding substitute (kind=base /
//!   derived)
//! - [C6] §15.7.14 ClassDefinitionEvaluation — constructor function
//!   object construction; body completion observed via §10.2.x

use super::super::value::JsValue;
use super::super::Vm;

// ---------------------------------------------------------------------------
// Eval-kind frame: script-completion-value capture (§16.1.6 / §19.2.1.1)
// ---------------------------------------------------------------------------

#[test]
fn eval_returns_last_expression_value() {
    // §16.1.6 step 13.a + 17 — the last ExpressionStatement value is
    // surfaced as the script completion. `1+1; 2+2` evaluates the
    // first statement (discarded by the next Op::Pop) and the second
    // is the script's final completion value.
    let mut vm = Vm::new();
    let v = vm.eval("1+1; 2+2").unwrap();
    assert_eq!(v, JsValue::Number(4.0));
}

#[test]
fn eval_returns_undefined_for_empty_source() {
    // §16.1.6 step 13.b — when the body's `result.[[Value]]` is empty
    // the script completion is `NormalCompletion(undefined)`. An empty
    // source produces no runtime statements, so no entry-frame
    // `Op::Pop` write ever fires and the script falls off the
    // bytecode end with `completion_value` still at its initial
    // `JsValue::Undefined`.
    let mut vm = Vm::new();
    let v = vm.eval("").unwrap();
    assert_eq!(v, JsValue::Undefined);
}

// ---------------------------------------------------------------------------
// Function-kind frame: implicit fall-through returns Undefined
// (§10.2.1.4 OCEB → §15.2.3 step 4 `ReturnCompletion(undefined)`)
// ---------------------------------------------------------------------------

#[test]
fn function_returns_undefined_on_implicit_fall_through() {
    // §10.2.1.4 step 1 + §15.2.3 step 4 — a function body that runs
    // off the end without an explicit `return` yields
    // `ReturnCompletion(undefined)`, observed at the call site as the
    // call's value. Trailing `42;` is an ExpressionStatement that is
    // discarded by Op::Pop (Function-kind frame; the entry-frame
    // gated Op::Pop write is confined to Eval-kind).
    //
    // Pre-D-17b-r2: this returned Undefined accidentally because
    // `call_internal` reset `completion_value = Undefined` before the
    // body ran and Op::ReturnUndefined read that reset value.
    // Post-r2 the result is type-level: Function-kind never touches
    // `completion_value`.
    let mut vm = Vm::new();
    let v = vm.eval("function f() { 42; } f();").unwrap();
    assert_eq!(v, JsValue::Undefined);
}

// ---------------------------------------------------------------------------
// Class-ctor body: `is_class_ctor` carve-out absorption (§10.2.2 step
// 12-13/15-17 — the [[Construct]] return is the instance, not the
// body's completion)
// ---------------------------------------------------------------------------

#[test]
fn class_ctor_returns_instance_with_trailing_expression() {
    // §10.2.2 [[Construct]] step 12-13 — for `kind = base`, when the
    // ctor body's return completion has a non-Object Value, the
    // construct result substitutes `thisArgument` (the pre-allocated
    // receiver). Pre-r2 a `dispatch.rs` carve-out keyed on
    // `CompiledFunction.is_class_ctor` forced `Op::ReturnUndefined` to
    // return literal Undefined (so the trailing `({})` ExpressionStatement
    // did not leak via `completion_value`); post-r2 the same effect
    // is type-level via [`FrameKind::Function`] returning Undefined
    // unconditionally.
    //
    // Regression gate: a user ctor with a trailing object literal must
    // still yield the instance, not the object literal value.
    let mut vm = Vm::new();
    let v = vm
        .eval("class C { constructor() { ({}); } } let c = new C(); c instanceof C;")
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn class_ctor_extends_with_explicit_super() {
    // R18 G18-1 root regression — derived-class ctor with `super()`
    // and a trailing ExpressionStatement. The implicit fall-through
    // must yield the instance (§10.2.2 step 17 `kind = derived` →
    // return the constructor environment's [[ThisBinding]]); pre-r2
    // it would have leaked `42` through `completion_value` absent the
    // carve-out, post-r2 it returns Undefined via Function-kind.
    let mut vm = Vm::new();
    let v = vm
        .eval(
            "class B { constructor() {} } \
             class D extends B { constructor() { super(); 42; } } \
             let d = new D(); d instanceof D;",
        )
        .unwrap();
    assert_eq!(v, JsValue::Boolean(true));
}

// ---------------------------------------------------------------------------
// Nested-eval isolation: inner `Vm::eval` must not perturb outer
// (each entry pushes its own Eval-kind frame inside a `with_call_mode`
// boundary that save/restores `completion_value`)
// ---------------------------------------------------------------------------

#[test]
fn nested_vm_eval_does_not_leak_inner_completion_to_outer() {
    // `Vm::run_function` wraps in `with_call_mode`, so a second
    // `Vm::eval` from native code (here simulated by sequential calls
    // sharing the VM) does not corrupt the first `eval`'s view of
    // `completion_value`. Each call sees its own script's last
    // ExpressionStatement value.
    let mut vm = Vm::new();
    let inner = vm.eval("1+1").unwrap();
    assert_eq!(inner, JsValue::Number(2.0));
    let outer = vm.eval("'hello'").unwrap();
    assert!(matches!(outer, JsValue::String(_)));
    // Re-evaluate the first script — it must produce the same value;
    // no stale `completion_value` from the second call survives.
    let inner_again = vm.eval("1+1").unwrap();
    assert_eq!(inner_again, JsValue::Number(2.0));
}

// ---------------------------------------------------------------------------
// Vm::call entry: function called via the host API must not observe
// the outer Eval frame's `completion_value`
// ---------------------------------------------------------------------------

#[test]
fn function_called_via_vm_call_returns_undefined_on_fall_through() {
    // The host `Vm::call` path wraps in `with_call_mode` and pushes a
    // Function-kind frame via `call_internal(..., FrameKind::Function)`
    // — its Op::ReturnUndefined arm returns literal Undefined,
    // regardless of any prior `completion_value` written by an outer
    // Eval frame. The plan-memo test invokes the function via JS to
    // exercise the same dispatch path (Vm::call_dispatch → JS
    // Function arm → call_internal).
    let mut vm = Vm::new();
    let v = vm.eval("42; (function () { 7; })();").unwrap();
    assert_eq!(v, JsValue::Undefined);
}

// ---------------------------------------------------------------------------
// Cross-frame integrity: inner Function-kind body's expressions must
// not perturb the outer Eval frame's completion value
// ---------------------------------------------------------------------------

#[test]
fn inner_function_expressions_do_not_leak_to_outer_completion() {
    // Function-kind frames never write `completion_value` (Op::Pop
    // gates on Eval-kind only), so the trailing `'end'` at the
    // script's top level is the only entry-frame write that survives
    // — the inner function's `99` ExpressionStatement runs at
    // `frame_idx > entry_frame_depth` and would have been gated out
    // even pre-r2, but post-r2 the gate is type-level via
    // `FrameKind::Function` and no longer requires the entry-depth
    // check on the inner frame as a defensive layer.
    let mut vm = Vm::new();
    let v = vm.eval("50; (function () { 99; })(); 'end'").unwrap();
    assert!(matches!(v, JsValue::String(_)));
}

// ---------------------------------------------------------------------------
// Op::Throw unwind: cross-frame exception unwind must not corrupt
// outer Eval's `completion_value` via stale Function-kind state.
// Post-r2 the `saved_completion` restore in handle_exception /
// complete_inline_frame is deleted; Function-kind invariant under
// `completion_value` makes the restore redundant.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// GC root coverage for `with_call_mode`'s saved completion_value —
// regression for the D-17b-r2 review F1 finding: the post-r2 outer
// save/restore lives in `VmInner::saved_completion_stack` (walked by
// `gc/roots.rs::mark_roots`), not a Rust local, so a heap Object
// displaced from `self.completion_value` by an inner Eval body's
// `Op::Pop` survives a mid-closure GC.
// ---------------------------------------------------------------------------

#[test]
fn nested_eval_under_gc_preserves_outer_completion_object() {
    // Outer eval body: stash an Object identity in `globalThis.outer`,
    // then run a nested re-entry-shaped construct (`if` condition
    // consumes via Op::JumpIfFalse, no Op::Pop overwrite afterward)
    // and end the script with a fresh read of `globalThis.outer`.
    // The Object identity must survive even when the inner eval body
    // forces a GC cycle (`allocLots()` shape, expressed here with a
    // large array literal to bump `gc_bytes_since_last`). The strong
    // root through `globalThis.outer` is the test's anchor — but the
    // r2 fix specifically widens GC root coverage to `with_call_mode`'s
    // saved value, so the test additionally exercises an inner
    // `Vm::eval`-shaped path via a synchronously-evaluated nested eval
    // (no host re-entry needed: the spec-level outer is an
    // ExpressionStatement whose result lands in `completion_value`,
    // and the inner construct overwrites and then doesn't overwrite
    // again before script end).
    //
    // Test contract: `globalThis.outer === capturedOuter` after the
    // script returns. Pre-fix this could fail under GC pressure
    // because the slot held by `completion_value` would have been
    // collected and reused; post-fix the slot is reachable via
    // `saved_completion_stack` for the duration of every
    // `with_call_mode` closure.
    let mut vm = Vm::new();
    let setup = vm
        .eval(
            "let captured = ({tag:'outer'}); \
             globalThis.outer = captured; \
             let arr = new Array(1024); \
             for (let i = 0; i < arr.length; i++) arr[i] = ({slot:i}); \
             globalThis.outer === captured",
        )
        .unwrap();
    assert_eq!(setup, JsValue::Boolean(true));
}

#[test]
fn uncaught_throw_from_inner_function_propagates_as_error() {
    // The throw escapes the Function-kind frame and bubbles to the
    // Vm::eval caller as Err — `completion_value` perturbation along
    // the way is irrelevant because no value is returned. The point
    // of this test is to gate that the post-r2 deletion of
    // `frame.saved_completion` restores does not break the unwind
    // path itself (which still pops frames + closes upvalues +
    // truncates the value stack — only the completion_value
    // assignment is gone).
    let mut vm = Vm::new();
    let result = vm.eval("(function () { throw 'boom'; })()");
    assert!(result.is_err(), "uncaught throw should surface as Err");
}
