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
//! `TemplateConcat`, the arithmetic / bitwise / relational operator groups, the
//! computed-key definitions, `SpreadObject`, `ArraySpread`, `IteratorRest`, and
//! `GetProp` / `SetProp`.
//!
//! ## The boundary, and why it kept moving
//!
//! It was drawn at "element access" first, then at "dispatch arms", and both
//! were too narrow: the exposed operand is often popped one or two layers deeper
//! (`ops.rs`'s operator helpers, `dispatch_objects.rs`'s definition bodies), and
//! sometimes it is not a compiler-placed operand at all — `ArraySpread`'s
//! iterator is created *by* the opcode and never touches the stack.
//!
//! The re-derived per-arm safety arguments were the other failure mode. Three of
//! them turned on "the operand is re-rooted as the receiver of the user code
//! that runs", which quietly stops being true for an **arrow or bound** callee:
//! its `this` comes from elsewhere. `SetProp`'s value had a second such
//! argument ("it is rooted as the setter's argument"), which fails for a
//! **zero-parameter** setter, since `call_internal` copies only
//! `args[..min(argc, param_count)]` onto the stack. Both are witnessed below.
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
use super::{gc_armed_number, gc_bool, gc_number, pool_setup};
use crate::bytecode::compiled::{CompiledFunction, CompiledScript, Constant};
use crate::bytecode::opcode::Op;
use crate::vm::value::VmErrorKind;

/// The user-code body every probe shares: one allocation, nothing else.
/// Under the armed one-shot a single allocation is a *guaranteed* collection,
/// so there is no allocation-volume heuristic to go stale.
const ALLOC: &str = "var t = {};";

/// The same body for windows the Rust-side one-shot cannot reach, because the
/// opcode's target object is allocated before the window opens. `__armGc()` is a
/// `cfg(test)` global (`vm/globals.rs`) that arms the identical one-shot from
/// script; the allocation that follows is what actually collects.
const ARM_ALLOC: &str = "__armGc(); var t = {};";

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

/// `Op::Sub` / `Op::Mul` / `Op::Div` / `Op::Mod` / `Op::Exp` read their operands
/// in place.
///
/// ECMA-262 §13.15.3 `ApplyStringOrNumericBinaryOperator` steps 3-4 run
/// `ToNumeric` (§7.1.3) on the left operand and only then on the right, so the
/// right operand spends the whole of the left one's user `valueOf` /
/// `@@toPrimitive` in a Rust local.
#[test]
fn numeric_binary_keeps_second_operand_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.lhs = {{ valueOf() {{ {ALLOC} return 6 }} }}; \
         pool.push({{ valueOf() {{ return 3 }} }});"
    ));
    // Pre-fix: NaN — the recycled slot is a bare `{}`, so ToNumber gives NaN.
    assert_eq!(gc_number(&setup, "lhs - mk()"), 3.0);
    assert_eq!(gc_number(&setup, "lhs * mk()"), 18.0);
    assert_eq!(gc_number(&setup, "lhs / mk()"), 2.0);
    assert_eq!(gc_number(&setup, "lhs % mk()"), 0.0);
    assert_eq!(gc_number(&setup, "lhs ** mk()"), 216.0);
}

/// `Op::BitAnd` / `BitOr` / `BitXor` / `Shl` / `Shr` / `UShr` read their operands
/// in place — the bitwise half of §13.15.3, where the same left-then-right
/// `ToNumeric` order exposes the right operand.
#[test]
fn bitwise_binary_keeps_second_operand_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.lhs = {{ valueOf() {{ {ALLOC} return 6 }} }}; \
         pool.push({{ valueOf() {{ return 3 }} }});"
    ));
    // Pre-fix: the recycled `{}` coerces to NaN → 0, so every case degenerates
    // to the identity (`6 & 0` = 0, `6 | 0` = 6, `6 << 0` = 6, …).
    assert_eq!(gc_number(&setup, "lhs & mk()"), 2.0);
    assert_eq!(gc_number(&setup, "lhs | mk()"), 7.0);
    assert_eq!(gc_number(&setup, "lhs ^ mk()"), 5.0);
    assert_eq!(gc_number(&setup, "lhs << mk()"), 48.0);
    assert_eq!(gc_number(&setup, "lhs >> mk()"), 0.0);
    assert_eq!(gc_number(&setup, "lhs >>> mk()"), 0.0);
}

/// `Op::Lt` / `LtEq` / `Gt` / `GtEq` read their operands in place.
///
/// All four productions of ECMA-262 §13.10.1 Runtime Semantics: Evaluation
/// reach §7.2.12 `IsLessThan` with `leftFirst` set so that the **left source
/// operand** is coerced first (`<` / `>=` pass it as `x` with `leftFirst` true;
/// `>` / `<=` pass it as `y` with `leftFirst` false). The right one is therefore
/// always the operand held across the other's user code.
#[test]
fn relational_keeps_second_operand_rooted() {
    let ascending = pool_setup(&format!(
        "globalThis.lhs = {{ valueOf() {{ {ALLOC} return 1 }} }}; \
         pool.push({{ valueOf() {{ return 2 }} }});"
    ));
    // Pre-fix: `false` — the recycled slot coerces to NaN, and every comparison
    // against NaN is false.
    assert!(gc_bool(&ascending, "lhs < mk()"));
    assert!(gc_bool(&ascending, "lhs <= mk()"));

    let descending = pool_setup(&format!(
        "globalThis.lhs = {{ valueOf() {{ {ALLOC} return 2 }} }}; \
         pool.push({{ valueOf() {{ return 1 }} }});"
    ));
    assert!(gc_bool(&descending, "lhs > mk()"));
    assert!(gc_bool(&descending, "lhs >= mk()"));
}

/// `Op::DefineComputedProperty` reads `[object key value]` in place.
///
/// §13.2.5.6 PropertyDefinitionEvaluation evaluates the key, then the value,
/// then converts the key with `ToPropertyKey` (§7.1.20) — so the already-
/// evaluated value is what sits unrooted across the key's user `toString`, and
/// it is then *stored*, making a dangling id durable rather than transient.
#[test]
fn computed_property_definition_keeps_value_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.k = {{ toString() {{ {ARM_ALLOC} return 'p' }} }}; pool.push({{ tag: 7 }});"
    ));
    // `.p` exists only if the key's `toString` ran, so the window is witnessed
    // by the assertion itself.  Pre-fix: the property holds the object that
    // recycled the collected slot, which has no `tag`.
    assert_eq!(gc_armed_number(&setup, "({ [k]: mk() }).p.tag"), 7.0);
}

/// `Op::DefineComputedMethod` — the same read for class computed methods
/// (§15.4.5 MethodDefinitionEvaluation), where the exposed value is the closure
/// `Op::Closure` just allocated and nothing else references.
#[test]
fn computed_method_definition_keeps_closure_rooted() {
    let setup = format!(
        "globalThis.k = {{ toString() {{ {ARM_ALLOC} return 'p' }} }};\
         globalThis.mkClass = function () {{ return class {{ [k]() {{ return 7 }} }} }};"
    );
    // Pre-fix: the installed method id points at whatever recycled the
    // collected closure's slot, so the call is not a function.
    assert_eq!(gc_armed_number(&setup, "mkClass().prototype.p()"), 7.0);
}

/// `Op::DefineComputedGetter` / `Op::DefineComputedSetter` — same again for
/// class computed accessors, with the accessor closure as the exposed operand.
#[test]
fn computed_accessor_definition_keeps_closure_rooted() {
    let setup = format!(
        "globalThis.k = {{ toString() {{ {ARM_ALLOC} return 'p' }} }};\
         globalThis.mkClass = function () {{ return class {{ get [k]() {{ return 7 }} }} }};"
    );
    assert_eq!(gc_armed_number(&setup, "mkClass().prototype.p"), 7.0);
}

/// `Op::SpreadObject` reads `[target source]` in place.
///
/// §7.3.25 `CopyDataProperties` step 4.c.ii.1 `Get(from, nextKey)` runs a user
/// getter once per key, and the loop dereferences the *source* again on every
/// following key. An arrow-function getter is the pointed case: the source is
/// not its receiver, so nothing else roots it while it runs.
#[test]
fn object_spread_keeps_source_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.mkSrc = function () {{ \
           var o = {{}}; \
           Object.defineProperty(o, 'a', \
             {{ get: () => {{ {ARM_ALLOC} return 1 }}, enumerable: true, configurable: true }}); \
           o.b = 7; \
           return o \
         }}; pool.push(mkSrc());"
    ));
    // `a` is copied first, so `b` is read after the getter's collection.
    // Pre-fix: the source id now names the object that recycled its slot.
    assert_eq!(gc_armed_number(&setup, "({ ...mk() }).b"), 7.0);
}

/// The iterator returned by `@@iterator` is a fresh object no root holds but
/// the operand stack.
///
/// §7.4.4 `GetIterator` calls the method with the *iterable* as receiver, so
/// what comes back is rooted nowhere; §7.4.10 `IteratorStepValue` then calls
/// `next()` once per element. When `next` is an arrow (or bound) function the
/// iterator is not its receiver either, so a Rust-local iterator is unrooted for
/// the whole of the first `next()` and dereferenced again on the second.
const ARROW_NEXT_ITERABLE: &str = "globalThis.src = { [Symbol.iterator]: function () { \
     var i = 0; \
     return { next: () => { \
        if (i === 0) { i = 1; __armGc(); var t = {}; return { value: 5, done: false } } \
        return { value: undefined, done: true } \
     } } \
   } };";

/// `Op::ArraySpread` keeps the iterator rooted for the iteration.
#[test]
fn array_spread_keeps_iterator_rooted() {
    // Pre-fix: the second `next()` lookup hits the recycled slot — "iterator.next
    // is not defined", or a panic in `get_object` if the slot is still free.
    assert_eq!(gc_armed_number(ARROW_NEXT_ITERABLE, "[...src][0]"), 5.0);
}

/// `Op::IteratorRest` (`var [...rest] = it`) keeps the iterator rooted too —
/// the sibling of `collect_iterator`, which already pushes it.
#[test]
fn iterator_rest_keeps_iterator_rooted() {
    assert_eq!(
        gc_armed_number(ARROW_NEXT_ITERABLE, "var [...rest] = src; rest[0]"),
        5.0
    );
}

/// `Op::SetProp` reads `[object value]` in place.
///
/// The stored value is handed to the setter as its argument, which roots it —
/// but only when the setter *declares a parameter*: `call_internal` copies
/// `args[..min(argc, param_count)]` into the frame's stack slots, so a
/// zero-parameter setter copies nothing. Combine that with an arrow setter
/// (receiver is lexical, so the base is unrooted too) and both operands spend
/// the setter's body in Rust locals — after which the arm pushes the value as
/// the assignment's result.
#[test]
fn static_property_store_keeps_value_rooted() {
    let setup = pool_setup(&format!(
        "globalThis.o = {{}}; \
         Object.defineProperty(o, 'p', {{ set: () => {{ {ARM_ALLOC} }}, configurable: true }}); \
         pool.push({{ tag: 9 }});"
    ));
    // Pre-fix: `r` names whatever recycled the collected slot — no `tag`.
    assert_eq!(gc_armed_number(&setup, "var r = (o.p = mk()); r.tag"), 9.0);
}

// ── Arms deliberately left on `pop()` ────────────────────────────────
//
// Only `Op::StrictEq` / `Op::StrictNotEq` remain, and not by a derived argument:
// `strict_eq` takes `&VmInner`, so the compiler — not a reviewer — proves they
// run no user code and open no window to root against.

/// The ordinary `Op::GetProp` / `Op::SetProp` accessor paths, which were rooted
/// even before the arms were.
///
/// A **method** accessor takes the base as its receiver and a **one-parameter**
/// setter puts the value in a callee stack slot, so these cases pass on both
/// sides of the change. They are kept as the control for
/// [`static_property_store_keeps_value_rooted`]: together they show the fix is
/// what closes the arrow / zero-parameter hole and not something that merely
/// perturbs the common path.
#[test]
fn static_property_access_via_method_accessors() {
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
