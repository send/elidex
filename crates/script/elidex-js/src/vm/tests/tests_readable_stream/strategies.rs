//! `CountQueuingStrategy` / `ByteLengthQueuingStrategy` (§6.1 /
//! §6.2) and the stream-level `highWaterMark` validation.
//!
//! Includes the PR-file-split-a Copilot R6 carry-over fixes for
//! `ByteLengthQueuingStrategy.prototype.size` `GetV` semantics
//! (throw on null/undefined; box other primitives for the lookup).

use crate::vm::Vm;

use super::eval_number;

#[test]
fn count_queuing_strategy_size_returns_one() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        "new CountQueuingStrategy({highWaterMark: 5}).size()",
    );
    assert_eq!(v, 1.0);
}

#[test]
fn count_queuing_strategy_high_water_mark_own_property() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        "new CountQueuingStrategy({highWaterMark: 7}).highWaterMark",
    );
    assert_eq!(v, 7.0);
}

#[test]
fn byte_length_queuing_strategy_size_reads_byte_length() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        "new ByteLengthQueuingStrategy({highWaterMark: 1024}).size(new Uint8Array(42))",
    );
    assert_eq!(v, 42.0);
}

/// PR-file-split-a Copilot R6 regression: §6.2.4
/// `ByteLengthQueuingStrategy.prototype.size(chunk)` evaluates as
/// `Return ? GetV(chunk, "byteLength")`.  GetV's `ToObject` step
/// must throw `TypeError` for `null` / `undefined` chunks instead
/// of silently returning `undefined` (which is what the pre-fix
/// non-Object early-return did).
#[test]
fn byte_length_queuing_strategy_size_throws_on_null() {
    let mut vm = Vm::new();
    let result = vm.eval("new ByteLengthQueuingStrategy({highWaterMark: 1}).size(null)");
    assert!(
        result.is_err(),
        "size(null) must throw via ToObject (got {result:?})"
    );
}

#[test]
fn byte_length_queuing_strategy_size_throws_on_undefined() {
    let mut vm = Vm::new();
    let result = vm.eval("new ByteLengthQueuingStrategy({highWaterMark: 1}).size()");
    assert!(
        result.is_err(),
        "size(undefined) must throw via ToObject (got {result:?})"
    );
}

/// Companion regression: a non-null primitive boxes through its
/// wrapper for the `byteLength` lookup.  No native `byteLength`
/// property exists on `Number.prototype`, so the spec-correct
/// result is `undefined` (not a throw, not 0).  Pre-fix the
/// non-Object early-return masked the boxing path entirely;
/// installing a `byteLength` accessor on `Number.prototype`
/// would not have fired.
#[test]
fn byte_length_queuing_strategy_size_boxes_number_primitive() {
    // Fresh VM per test — `Number.prototype` mutation lives only
    // for this VM's lifetime.  `configurable: true` is documented
    // for symmetry with future cleanup; not strictly required
    // because the VM is dropped at end of scope.
    let mut vm = Vm::new();
    let source = r#"
        Object.defineProperty(Number.prototype, "byteLength", {
            get() { return 7; },
            configurable: true,
        });
        new ByteLengthQueuingStrategy({highWaterMark: 1}).size(123);
    "#;
    let v = eval_number(&mut vm, source);
    // Confirm the accessor on `Number.prototype` was actually
    // consulted — pre-fix the strategy would have returned
    // `undefined` (NaN through `eval_number`).
    assert_eq!(
        v, 7.0,
        "primitive must be boxed so prototype accessor fires"
    );
}

#[test]
fn high_water_mark_negative_throws() {
    let mut vm = Vm::new();
    let result = vm.eval("new ReadableStream(undefined, {highWaterMark: -1})");
    assert!(result.is_err());
}

#[test]
fn high_water_mark_nan_throws() {
    let mut vm = Vm::new();
    let result = vm.eval("new ReadableStream(undefined, {highWaterMark: NaN})");
    assert!(result.is_err());
}
