//! Operand rooting for the non-element opcodes, and the disposition of an
//! internal invariant violation.
//!
//! The element-access family (`GetElem` / `SetElem` / `DeleteElem` /
//! `IncElem` / `DecElem` / `GetElemRef`) was rooted first and is pinned by
//! `tests_member_compound_assign`. The concept it was rooted *for* is broader
//! than "element access": **any** opcode that pops an operand into a Rust local
//! and then runs user JS before reading or mutating through it is exposed, since
//! `gc/roots.rs` walks the VM stack but not Rust locals. This module covers the
//! rest of that set — `IncProp` / `DecProp`, `In`, `Instanceof`, `Add`,
//! `TemplateConcat` — plus the controls for the two arms deliberately left on
//! `pop()`.
//!
//! ## Why not `eval_gc_stressed`
//!
//! `eval_gc_stressed` sets `gc_threshold = 0` intending "collect at every
//! allocation", but `collect_garbage` ends with
//! `self.gc_threshold = (live_count * 128).max(32768)` — so the zero survives
//! exactly one allocation and the rest of the script runs at a normal threshold.
//! A rooting test written on it therefore places its collection at the script's
//! *first* allocation, not inside the user-code window, and passes whether or
//! not the operand is rooted.
//!
//! [`gc_in_window`] places the collection instead of provoking one: `setup`
//! performs every allocation the case needs, then the test-only
//! `force_gc_before_next_alloc` one-shot (checked in `VmInner::alloc_object`) is
//! armed and `expr` runs, whose *first* allocation is the `var t = {}` inside
//! the user code. The helper asserts the one-shot actually fired, so a case that
//! stops allocating in its window fails loudly instead of passing vacuously.

use super::super::value::JsValue;
use super::Vm;
use super::{gc_bool, gc_number, pool_setup};
use crate::bytecode::compiled::{CompiledFunction, CompiledScript, Constant};
use crate::bytecode::opcode::Op;
use crate::vm::value::VmErrorKind;

/// The user-code body every probe shares: one allocation, nothing else.
/// Under the armed one-shot a single allocation is a *guaranteed* collection,
/// so there is no allocation-volume heuristic to go stale.
const ALLOC: &str = "var t = {};";

// ── Rooted arms ──────────────────────────────────────────────────────

/// `Op::IncProp` / `Op::DecProp` read their base in place.
///
/// The worst member of the set: it crosses **two** user-code windows before it
/// stores — a user getter in `get_property_val` and a user `valueOf` in
/// `to_number` — and then writes back through the same base. Popped into a Rust
/// local, that base was collected by either window and the store went through a
/// dangling `ObjectId`; against a freed slot `get_object_mut` panicked outright
/// with "object already freed".
///
/// The witness is the setter: it is reachable from the base alone, so it cannot
/// run at all if the base was collected.
#[test]
fn property_update_keeps_temporary_base_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.seen = 0; globalThis.hot = {{ valueOf() {{ {ALLOC} return 1 }} }}; \
         pool.push({{ get p() {{ return hot }}, set p(v) {{ seen = v }} }});"
    ));
    // Postfix `++` yields the old value; `seen` proves the store landed on the
    // intended object.  Pre-fix: panic in `get_object_mut`.
    assert_eq!(gc_number(&setup, "mk().p++; seen"), 2.0);
    assert_eq!(gc_number(&setup, "mk().p--; seen"), 0.0);
    assert_eq!(gc_number(&setup, "mk().p++"), 1.0);
    assert_eq!(gc_number(&setup, "--mk().p"), 0.0);

    // Control: a base bound to a variable lives in the frame's stack slots, so
    // it was rooted even before the fix — reading the operand in place left the
    // ordinary path alone.
    let bound = format!(
        "globalThis.seen = 0; globalThis.hot = {{ valueOf() {{ {ALLOC} return 1 }} }}; \
         globalThis.o = {{ get p() {{ return hot }}, set p(v) {{ seen = v }} }};"
    );
    assert_eq!(gc_number(&bound, "o.p++; seen"), 2.0);
}

/// `Op::In` reads both operands in place.
///
/// `op_in` converts the key with `ToPropertyKey` (ECMA-262 §7.1.20), which runs
/// user `toString` / `@@toPrimitive`, and only *then* dereferences the
/// right-hand object — through `get_object`, which is an `.unwrap()` on the
/// slot. So a collected base does not merely answer wrongly: if the slot is
/// still free it panics, and if something has recycled it the answer comes from
/// an unrelated object.
#[test]
fn in_operator_keeps_temporary_base_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.key = {{ toString() {{ {ALLOC} return 'p' }} }}; pool.push({{ p: 1 }});"
    ));
    // Pre-fix: `false` — the recycled slot had no `p`.
    assert!(gc_bool(&setup, "key in mk()"));

    let bound = format!(
        "globalThis.key = {{ toString() {{ {ALLOC} return 'p' }} }}; globalThis.o = {{ p: 1 }};"
    );
    assert!(gc_bool(&bound, "key in o"));
}

/// `Op::Instanceof` reads both operands in place.
///
/// `@@hasInstance` may be an **accessor**: `op_instanceof` resolves it (running
/// the getter) and only then calls the resulting function with the left operand
/// as its argument. Across that getter the left operand is rooted nowhere — it
/// is not the getter's receiver, and it has not been pushed as an argument yet.
#[test]
fn instanceof_keeps_operands_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.C = function () {{}}; \
         globalThis.check = function (x) {{ return x.tag === 7 }}; \
         Object.defineProperty(C, Symbol.hasInstance, \
             {{ get() {{ {ALLOC} return check }} }}); \
         pool.push({{ tag: 7 }});"
    ));
    // Pre-fix: `false` — `check` ran against a recycled slot with no `tag`.
    assert!(gc_bool(&setup, "mk() instanceof C"));
}

/// `Op::Add` reads both operands in place.
///
/// §13.8.1 converts the operands left-to-right, so `ToPrimitive` on the left one
/// runs user code while the right one is still unconverted — and popped, it is
/// held only in a Rust local for the whole of that window.
#[test]
fn addition_keeps_second_operand_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.lhs = {{ valueOf() {{ {ALLOC} return 1 }} }}; \
         pool.push({{ valueOf() {{ return 2 }} }});"
    ));
    // Pre-fix: TypeError — the recycled slot had no usable `valueOf`/`toString`.
    assert_eq!(gc_number(&setup, "lhs + mk()"), 3.0);
}

/// `Op::TemplateConcat` reads its parts in place.
///
/// The pre-fix body copied all `count` parts into a `Vec` and truncated the
/// stack *before* the conversion loop, so every part after the one being
/// converted was reachable only from that `Vec` — exactly the "no `truncate`
/// before the user code" failure, one opcode deeper than the pop-based arms.
#[test]
fn template_concat_keeps_later_parts_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.first = {{ toString() {{ {ALLOC} return 'A' }} }}; \
         pool.push({{ toString() {{ return 'B' }} }});"
    ));
    let mut vm = Vm::new();
    vm.eval(&setup).unwrap();
    vm.inner.force_gc_before_next_alloc = true;
    // Pre-fix: TypeError from the recycled second part.
    let value = vm.eval("`${first}${mk()}`").unwrap();
    assert!(!vm.inner.force_gc_before_next_alloc, "no collection placed");
    let JsValue::String(id) = value else {
        panic!("expected string, got {value:?}")
    };
    assert_eq!(vm.get_string(id), "AB");
}

// ── Arms deliberately left on `pop()` ────────────────────────────────

/// `Op::GetProp` / `Op::SetProp` need no rooting, verified rather than assumed.
///
/// Their base *is* handed to user code — as the accessor's receiver — but it is
/// re-rooted there as the callee frame's `this`, and neither `get_prop_slow` nor
/// `set_prop_slow` dereferences it again afterwards: both collect their IC info
/// *before* the accessor runs and touch only `compiled_functions` after it. The
/// stored value is likewise rooted for the duration, as the setter's argument.
///
/// These cases pass on both sides of the rooting change; they are here so the
/// verdict is a test rather than a claim, and so a future edit that starts
/// dereferencing the base after the accessor fails here.
#[test]
fn static_property_access_needs_no_rooting() {
    let own_get = pool_setup(&format!("pool.push({{ get p() {{ {ALLOC} return 5 }} }});"));
    assert_eq!(gc_number(&own_get, "mk().p"), 5.0);

    let proto_get = pool_setup(&format!(
        "globalThis.P = {{ get p() {{ {ALLOC} return 5 }} }}; pool.push(Object.create(P));"
    ));
    assert_eq!(gc_number(&proto_get, "mk().p"), 5.0);

    let own_set = pool_setup(&format!(
        "globalThis.seen = 0; pool.push({{ set p(v) {{ {ALLOC} seen = v }} }});"
    ));
    assert_eq!(gc_number(&own_set, "mk().p = 7; seen"), 7.0);

    // The assigned *value* survives the setter too — it is on the stack as the
    // setter's argument for the whole call.
    let set_value = pool_setup(&format!(
        "globalThis.o = {{ set p(v) {{ {ALLOC} }} }}; pool.push({{ tag: 9 }});"
    ));
    assert_eq!(gc_number(&set_value, "var r = (o.p = mk()); r.tag"), 9.0);
}

// ── Internal invariant violations leave by the hard path ─────────────

/// Build a one-function script whose bytecode is `body`, wrapped in an
/// exception handler whose catch block returns `42`.
///
/// Layout: `PushExceptionHandler` is 5 bytes (opcode + catch u16 + finally
/// u16), so the catch target is `5 + body.len()`.
fn script_with_catch(body: &[u8], constants: Vec<Constant>) -> CompiledScript {
    let catch_ip = u16::try_from(5 + body.len()).unwrap();
    let mut bytecode = vec![
        Op::PushExceptionHandler as u8,
        (catch_ip & 0xFF) as u8,
        (catch_ip >> 8) as u8,
        0xFF, // no finally
        0xFF,
    ];
    bytecode.extend_from_slice(body);
    // Catch target: push 42 as the completion value and return.
    bytecode.extend_from_slice(&[
        Op::PushI8 as u8,
        42,
        Op::PopCompletion as u8,
        Op::ReturnUndefined as u8,
    ]);

    let mut top_level = CompiledFunction::new();
    top_level.bytecode = bytecode;
    top_level.constants = constants;
    top_level.is_strict = true;
    CompiledScript {
        top_level,
        source: String::new(),
        line_starts: vec![0],
    }
}

/// A broken VM invariant must not be catchable.
///
/// `VmError::internal` is documented in `vm/error.rs` as "should not occur in
/// correct programs" — the dispatch-path producers are stack-shape guards and
/// malformed-bytecode checks. Routed through `throw_error` it became a plain JS
/// `Error`, so a user `try`/`catch` could swallow the evidence and keep running
/// over a stack the VM had already decided was wrong. `Op::Swap` hard-returned
/// for the identical condition, which made the disposition depend on which arm
/// happened to notice; `raise` now decides by the error's kind instead.
#[test]
fn internal_invariant_violation_is_not_catchable() {
    // `Op::GetElem` on an empty stack is the witness: its guard predates this
    // change, and it is one of the arms whose `Err` went to `throw_error` — so
    // before `raise` the script's own `catch` swallowed it and this returned 42.
    let caught_before = script_with_catch(&[Op::GetElem as u8], Vec::new());
    let mut vm = Vm::new();
    let err = vm
        .run_script(caught_before)
        .expect_err("a stack underflow must not be caught by the script's own handler");
    assert!(
        matches!(err.kind, VmErrorKind::InternalError),
        "expected InternalError, got {:?}",
        err.kind
    );
    assert!(
        err.message.contains("stack underflow on GetElem"),
        "guard message lost: {}",
        err.message
    );

    // `Op::Add`'s guard is new with the rooting change (it replaced the two
    // `pop()?` calls, which propagated straight out of `run()`). It must keep
    // that disposition rather than inherit `throw_error` from the arm it now
    // routes through.
    let mut vm = Vm::new();
    let err = vm
        .run_script(script_with_catch(&[Op::Add as u8], Vec::new()))
        .expect_err("the new operand guard must not be catchable either");
    assert!(
        matches!(err.kind, VmErrorKind::InternalError),
        "expected InternalError, got {:?}",
        err.kind
    );
    assert!(
        err.message.contains("stack underflow on Add"),
        "guard message lost: {}",
        err.message
    );
}

/// `Op::ThrowUnsupported` is deliberately *not* in that category.
///
/// It reports an unimplemented language construct, not a broken invariant, so
/// its `TypeError` stays catchable — the failure is scoped to the statement that
/// runs it. This is the control that keeps `raise` keyed on the error's kind
/// rather than on "did an opcode body return `Err`".
#[test]
fn unsupported_construct_stays_catchable() {
    let script = script_with_catch(
        &[Op::ThrowUnsupported as u8, 0, 0],
        vec![Constant::Wtf16("nope".encode_utf16().collect())],
    );
    let mut vm = Vm::new();
    let value = vm
        .run_script(script)
        .expect("an unsupported-construct TypeError must remain catchable");
    assert!(
        matches!(value, JsValue::Number(n) if (n - 42.0).abs() < f64::EPSILON),
        "catch block did not run: {value:?}"
    );
}
